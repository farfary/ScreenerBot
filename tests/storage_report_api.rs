// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The storage routes Settings reads and drives: the report answers every category against its limit, a requested cleanup starts once while another runs, its result reaches the report, and the key vault and live store files survive it.

mod common;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use rusqlite::Connection;
use screenerbot::chains::ChainId;
use screenerbot::database::catalog::{OHLCVS, TOKENS, WALLETS};
use screenerbot::webserver::routes;
use screenerbot::webserver::state::AppState;
use serde_json::Value;
use tower::ServiceExt;

/// How long a requested pass over an almost empty data directory may take.
const CLEANUP_DEADLINE: Duration = Duration::from_secs(30);

const CATEGORIES: [&str; 9] = [
    "wallet_keys",
    "trading_records",
    "wallet_history",
    "market_data",
    "settings",
    "recovery_copies",
    "updates",
    "logs",
    "leftovers",
];

async fn send(method: &str, path: &str) -> (StatusCode, Value) {
    let router = routes::create_router(Arc::new(AppState::new()));
    let request = Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("build request");
    let response = router.oneshot(request).await.expect("router responded");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read response body");
    let body = serde_json::from_slice(&bytes).unwrap_or_else(|error| {
        panic!(
            "{path} did not answer JSON ({error}): {}",
            String::from_utf8_lossy(&bytes)
        )
    });
    (status, body)
}

/// A store file holding one sentinel row, closed so only its main file remains.
fn sentinel_store(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("store directory")).expect("create data dir");
    let conn = Connection::open(path).expect("open store");
    conn.execute_batch(
        "CREATE TABLE sentinel (value TEXT NOT NULL);
         INSERT INTO sentinel (value) VALUES ('kept');",
    )
    .expect("write sentinel");
}

fn sentinel(path: &Path) -> String {
    Connection::open(path)
        .expect("reopen store")
        .query_row("SELECT value FROM sentinel", [], |row| row.get(0))
        .expect("read sentinel")
}

#[test]
fn the_storage_report_and_a_requested_cleanup_keep_the_vault_and_live_stores() {
    let _data = common::isolated_env();
    let stores = [
        WALLETS.path(ChainId::Solana),
        TOKENS.path(ChainId::Solana),
        OHLCVS.path(ChainId::Solana),
    ];
    for store in &stores {
        sentinel_store(store);
    }

    // One thread, so the requested pass cannot finish between the two requests.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime");
    runtime.block_on(async {
        let (status, report) = send("GET", "/api/system/storage").await;
        assert_eq!(status, StatusCode::OK, "{report}");
        let categories: Vec<&str> = report["categories"]
            .as_array()
            .expect("categories")
            .iter()
            .map(|entry| entry["category"].as_str().expect("category id"))
            .collect();
        assert_eq!(categories, CATEGORIES);
        assert!(
            report["categories"][0]["bytes"]
                .as_u64()
                .expect("vault bytes")
                > 0,
            "the key vault is counted: {report}"
        );
        assert_eq!(report["limit_mode"], "automatic");
        assert_eq!(report["limit_megabytes"], 0);
        assert!(report["limit_bytes"].as_u64().expect("limit") > 0);
        assert_eq!(report["running"], false);
        assert!(report["last_cleanup"].is_null());

        let (status, started) = send("POST", "/api/system/storage/cleanup").await;
        assert_eq!(status, StatusCode::ACCEPTED, "{started}");
        assert_eq!(started["started"], true);
        let (status, refused) = send("POST", "/api/system/storage/cleanup").await;
        assert_eq!(status, StatusCode::CONFLICT, "{refused}");
        assert_eq!(refused["error"]["code"], "CONFLICT", "{refused}");

        let deadline = Instant::now() + CLEANUP_DEADLINE;
        let report = loop {
            let (status, report) = send("GET", "/api/system/storage").await;
            assert_eq!(status, StatusCode::OK, "{report}");
            if !report["last_cleanup"].is_null() && report["running"] == false {
                break report;
            }
            assert!(
                Instant::now() < deadline,
                "the cleanup never finished: {report}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        };
        assert_eq!(report["last_cleanup"]["failures"], 0, "{report}");
        assert_eq!(report["last_cleanup"]["evicted_rows"], 0, "{report}");

        let (status, again) = send("POST", "/api/system/storage/cleanup").await;
        assert_eq!(
            status,
            StatusCode::ACCEPTED,
            "a finished pass frees the lock: {again}"
        );
        let deadline = Instant::now() + CLEANUP_DEADLINE;
        while send("GET", "/api/system/storage").await.1["running"] != false {
            assert!(
                Instant::now() < deadline,
                "the second cleanup never finished"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });

    for store in &stores {
        assert!(store.exists(), "{} was deleted", store.display());
        assert_eq!(sentinel(store), "kept", "{} lost its rows", store.display());
    }
}
