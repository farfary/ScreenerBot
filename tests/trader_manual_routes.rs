// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The manual buy and add routes: a size the caller sent but the trader cannot use is refused with the trade-size error, never replaced by the configured size.

// The config guard is a `std::sync::Mutex` on purpose; see `tests/trader_exits.rs`.
#![allow(clippy::await_holding_lock)]

use crate::common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use screenerbot::global::{
    CONNECTIVITY_SYSTEM_READY, POOL_SERVICE_READY, POSITIONS_SYSTEM_READY, TOKENS_SYSTEM_READY,
    TRANSACTIONS_SYSTEM_READY,
};
use screenerbot::webserver::routes;
use screenerbot::webserver::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;

const MINT: &str = "So11111111111111111111111111111111111111112";

/// Longest a refused request may take: a refusal happens before any network call, a
/// request that proceeds to trade does not.
const REFUSAL_DEADLINE: Duration = Duration::from_secs(20);

fn core_service_flags() -> [&'static AtomicBool; 5] {
    [
        &CONNECTIVITY_SYSTEM_READY,
        &TOKENS_SYSTEM_READY,
        &POSITIONS_SYSTEM_READY,
        &POOL_SERVICE_READY,
        &TRANSACTIONS_SYSTEM_READY,
    ]
}

/// The readiness flags are process-wide statics; a test marks the core services ready so
/// the preflight passes, and clears them when it ends.
struct CoreServicesReady {
    _guard: MutexGuard<'static, ()>,
}

fn core_services_ready() -> CoreServicesReady {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for flag in core_service_flags() {
        flag.store(true, Ordering::SeqCst);
    }
    CoreServicesReady { _guard: guard }
}

impl Drop for CoreServicesReady {
    fn drop(&mut self) {
        for flag in core_service_flags() {
            flag.store(false, Ordering::SeqCst);
        }
    }
}

async fn post(path: &str, body: Value) -> (StatusCode, Value) {
    let router = routes::create_router(Arc::new(AppState::new()));
    let request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    let response = tokio::time::timeout(REFUSAL_DEADLINE, router.oneshot(request))
        .await
        .expect("a request with an unusable size must be refused before any trade work")
        .expect("router responded");
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

fn assert_trade_size_refused(label: &str, outcome: &(StatusCode, Value)) {
    let (status, body) = outcome;
    assert_eq!(*status, StatusCode::BAD_REQUEST, "{label}: {body}");
    assert_eq!(
        body["error"]["text"]["id"], "errors-trade-size-invalid",
        "{label}: {body}"
    );
}

#[tokio::test]
async fn a_buy_with_a_non_positive_size_is_refused_not_traded_at_the_configured_size() {
    let _cfg = common::config_guard();
    let _ready = core_services_ready();

    for size in [-1.0, 0.0, -0.0001] {
        let outcome = post(
            "/api/trader/manual/buy",
            json!({ "mint": MINT, "size_sol": size }),
        )
        .await;
        assert_trade_size_refused(&format!("buy size_sol={size}"), &outcome);
    }
}

#[tokio::test]
async fn an_add_with_a_non_positive_size_is_refused_not_traded_at_the_configured_size() {
    let _cfg = common::config_guard();
    let _ready = core_services_ready();

    let mut held = common::test_position(0.01, 1.0);
    held.mint = MINT.to_owned();
    *screenerbot::positions::state::POSITIONS.write().await = vec![held];

    for size in [-1.0, 0.0, -0.0001] {
        let outcome = post(
            "/api/trader/manual/add",
            json!({ "mint": MINT, "size_sol": size }),
        )
        .await;
        assert_trade_size_refused(&format!("add size_sol={size}"), &outcome);
    }
    screenerbot::positions::state::POSITIONS
        .write()
        .await
        .clear();
}
