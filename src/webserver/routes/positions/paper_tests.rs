// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The paper routes win over the live `/positions/{key}` routes and sit behind the paper
//! store's own gate, answer that the paper book is unavailable until it loads, a paper
//! close and a paper details read resolve their id in the paper book only, the stats and
//! account keep their response shape, and a paper round's details are the live details
//! shape with the paper provenance of each fill.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use chrono::{Duration, TimeZone, Utc};
use serde_json::Value;
use tower::ServiceExt;

use super::*;
use crate::chains::RawAmount;
use crate::database::catalog;
use crate::database::health::{self, StoreHealth};
use crate::database::recovery::RecoveryAction;
use crate::errors::DatabaseError;
use crate::positions::db::PaperLegRecord;
use crate::positions::{
    Book, EntryRecord, ExitRecord, Opening, Position, PositionManagement, PositionOrigin,
};

async fn answer(method: &str, uri: &str) -> (StatusCode, Value) {
    let router = crate::webserver::routes::create_router(Arc::new(AppState::new()));
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router responded");
    let status = response.status();
    let body = to_bytes(response.into_body(), 256 * 1024)
        .await
        .expect("read response body");
    (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
}

fn unavailable(store: &'static catalog::StoreSpec) {
    let too_new = DatabaseError::StoreTooNew {
        store: store.id.to_owned(),
        version: 9,
        min_reader_version: 9,
        supported_version: 1,
        written_by: "99.0.0".to_owned(),
    };
    for path in store.paths() {
        health::publish(
            store,
            &path,
            StoreHealth::unavailable(&too_new, RecoveryAction::Isolate),
        );
    }
}

#[tokio::test]
async fn paper_routes_win_over_the_live_position_routes_and_gate_on_the_paper_store() {
    unavailable(&catalog::PAPER_POSITIONS);
    for (method, uri) in [
        ("GET", "/api/positions/paper"),
        ("GET", "/api/positions/paper/stats"),
        ("GET", "/api/positions/paper/account"),
        ("POST", "/api/positions/paper/account"),
        ("GET", "/api/positions/paper/7/details"),
        ("POST", "/api/positions/paper/7/close"),
    ] {
        let (status, body) = answer(method, uri).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{method} {uri}");
        assert_eq!(
            body["error"]["text"]["args"]["store"]["value"],
            catalog::PAPER_POSITIONS.id,
            "{method} {uri} was not answered by a paper route"
        );
    }

    let (status, body) = answer("GET", "/api/positions/stats").await;
    assert_ne!(
        body["error"]["text"]["args"]["store"]["value"],
        catalog::PAPER_POSITIONS.id,
        "a live route is gated on the paper store ({status})"
    );
}

#[tokio::test]
async fn paper_routes_answer_paper_book_unavailable_until_the_paper_book_loads() {
    for (method, uri) in [
        ("POST", "/api/positions/paper/7/close"),
        ("GET", "/api/positions/paper/7/details"),
        ("GET", "/api/positions/paper"),
    ] {
        let (status, body) = answer(method, uri).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{method} {uri}");
        assert_eq!(
            body["error"]["text"]["id"], "errors-trade-paper-book-unavailable",
            "{method} {uri}: {body}"
        );
    }
}

/// A live round and a loaded paper book in an isolated data directory: closing or reading
/// the live round's id through the paper routes answers not found and leaves the live
/// round open.
#[tokio::test(flavor = "multi_thread")]
async fn a_paper_close_and_details_resolve_their_id_in_the_paper_book_only() {
    let directory = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", directory.path());
    let wallet = crate::chains::solana::accounts::configure_test_trading_wallet();
    crate::paths::ensure_all_directories().expect("create data directories");
    crate::positions::initialize_positions_database()
        .await
        .expect("open the live store");
    let mut live = round(1, true, 0.1, None);
    live.id = None;
    live.book = Book::Live;
    let live_id = crate::positions::db::save_position(&live)
        .await
        .expect("store the live round");
    let chain = crate::positions::db::get_store_chain()
        .await
        .expect("the live store's chain");
    let book = crate::positions::paper::initialize(chain, &wallet)
        .await
        .expect("load the paper book");
    assert!(book.position(live_id).await.is_none());

    let (status, body) = answer("GET", &format!("/api/positions/paper/{live_id}/details")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        body["error"]["text"]["id"], "errors-positions-not-found",
        "{body}"
    );

    let (status, body) = answer("POST", &format!("/api/positions/paper/{live_id}/close")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(
        body["error"]["text"]["id"], "errors-positions-not-found",
        "{body}"
    );
    let stored = crate::positions::db::get_position_by_id(live_id)
        .await
        .expect("read the live round")
        .expect("the live round is stored");
    assert!(stored.exit_time.is_none(), "the live round was closed");
    assert!(stored.exit_transaction_signature.is_none());
}

fn round(id: i64, open: bool, total_size_native: f64, pnl: Option<f64>) -> Position {
    let mut position = Position::opening(Opening {
        mint: format!("PaperRouteMint{id}"),
        symbol: format!("P{id}"),
        name: "Paper".to_owned(),
        entry_price: 0.001,
        entry_time: Utc::now() - Duration::hours(id),
        size_native: total_size_native,
        entry_signature: None,
        origin: PositionOrigin::Auto {
            strategy_id: Some("paper-entry".to_owned()),
        },
        management: PositionManagement::AutoTrader,
        entry_liquidity: None,
        book: Book::Paper,
    });
    position.id = Some(id);
    position.total_size_native = total_size_native;
    if !open {
        position.exit_time = Some(Utc::now());
        position.transaction_exit_verified = true;
        position.pnl = pnl;
    }
    position
}

#[test]
fn paper_stats_count_and_sum_the_paper_rounds_given() {
    let open = [round(1, true, 0.1, None), round(2, true, 0.2, None)];
    let closed = [
        round(3, false, 0.1, Some(0.03)),
        round(4, false, 0.1, Some(-0.01)),
    ];
    let stats = serde_json::to_value(positions_stats(&open, &closed)).unwrap();
    assert_eq!(stats["total"], 4);
    assert_eq!(stats["open"], 2);
    assert_eq!(stats["closed"], 2);
    assert!((stats["total_invested_native"].as_f64().unwrap() - 0.3).abs() < 1e-12);
    assert!((stats["total_pnl"].as_f64().unwrap() - 0.02).abs() < 1e-12);
}

#[test]
fn the_account_response_carries_the_run_and_its_cash() {
    let account = PaperAccount {
        epoch: 2,
        starting_balance_native: 1.0,
        started_at: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
        cash_native: 0.7,
    };
    let response = serde_json::to_value(paper_account_response(account)).unwrap();
    assert_eq!(response["epoch"], 2);
    assert_eq!(response["started_at"], 1_790_812_800);
    assert_eq!(response["starting_balance_native"], 1.0);
    assert_eq!(response["cash_native"], 0.7);
    // Open exposure is reported by the trader stats, never a second time here.
    for dropped in [
        "open_rounds",
        "open_cost_native",
        "open_unrealized_pnl_native",
    ] {
        assert!(
            response.get(dropped).is_none(),
            "{dropped} is still reported"
        );
    }
}

/// The booked history of paper round 7: an entry, a partial exit with no leg and a full
/// exit whose leg carries an unreadable quote, the exits stored newest first.
fn round_history() -> PaperRoundHistory {
    let at = |minute| Utc.with_ymd_and_hms(2026, 10, 1, 0, minute, 0).unwrap();
    PaperRoundHistory {
        entries: vec![EntryRecord {
            id: Some(1),
            position_id: 7,
            timestamp: at(0),
            amount: RawAmount::new(1_000),
            price: 0.001,
            native_spent: 0.1,
            transaction_signature: "paper:entry".to_owned(),
            is_dca: false,
            fees_raw: Some(5_000),
        }],
        exits: vec![
            ExitRecord {
                id: Some(3),
                position_id: 7,
                timestamp: at(9),
                amount: RawAmount::new(500),
                price: 0.002,
                native_received: 0.1,
                transaction_signature: "paper:exit".to_owned(),
                is_partial: false,
                percentage: 100.0,
                fees_raw: None,
            },
            ExitRecord {
                id: Some(2),
                position_id: 7,
                timestamp: at(5),
                amount: RawAmount::new(500),
                price: 0.0015,
                native_received: 0.075,
                transaction_signature: "paper:partial".to_owned(),
                is_partial: true,
                percentage: 50.0,
                fees_raw: None,
            },
        ],
        legs: vec![
            PaperLegRecord {
                kind: "entry".to_owned(),
                leg_ref: "paper:entry".to_owned(),
                strategy_id: Some("paper-entry".to_owned()),
                quote_json: r#"{"model":"quote"}"#.to_owned(),
                created_at: at(0).to_rfc3339(),
            },
            PaperLegRecord {
                kind: "exit".to_owned(),
                leg_ref: "paper:exit".to_owned(),
                strategy_id: Some("paper-exit".to_owned()),
                quote_json: "not json".to_owned(),
                created_at: at(9).to_rfc3339(),
            },
        ],
    }
}

#[test]
fn paper_records_are_ordered_and_joined_to_their_legs() {
    let (entries, exits) = paper_records(round_history(), ChainId::Solana);

    assert_eq!(entries.len(), 1);
    assert!(!entries[0].is_dca);
    assert_eq!(entries[0].strategy_id.as_deref(), Some("paper-entry"));
    assert_eq!(
        entries[0].quote,
        Some(serde_json::json!({"model": "quote"}))
    );
    assert!(entries[0].fees_native.is_some());

    let percentages: Vec<f64> = exits.iter().map(|exit| exit.percentage).collect();
    assert_eq!(percentages, [50.0, 100.0], "exits are oldest first");
    assert!(exits[0].is_partial);
    assert_eq!(
        exits[0].strategy_id, None,
        "a fill without a leg has no provenance"
    );
    assert_eq!(exits[1].strategy_id.as_deref(), Some("paper-exit"));
    assert_eq!(exits[1].quote, None, "an unreadable quote is left out");
}

/// The paper round and the live round of the same shape build the same response: the
/// book is the only difference, a paper round has no swap pending and takes no manual
/// add, and a record carries its provenance only when the paper leg gave it one.
#[tokio::test]
async fn a_paper_round_builds_the_live_details_shape_with_its_paper_provenance() {
    crate::config::utils::install_default_config();
    let (entries, exits) = paper_records(round_history(), ChainId::Solana);
    let paper =
        build_position_details(&round(7, true, 0.1, None), entries, exits, Vec::new()).await;
    let paper = serde_json::to_value(paper).unwrap();

    assert_eq!(paper["book"], "paper");
    assert_eq!(paper["pending_swaps"], serde_json::json!([]));
    assert_eq!(paper["position"]["id"], 7);
    assert_eq!(paper["position"]["mint"], "PaperRouteMint7");
    assert_eq!(paper["position"]["default_add_size_native"], Value::Null);
    assert_eq!(paper["entries"][0]["strategy_id"], "paper-entry");
    assert_eq!(paper["entries"][0]["quote"]["model"], "quote");
    assert_eq!(paper["entries"][0]["transaction_signature"], "paper:entry");
    assert!(paper["exits"][0].get("strategy_id").is_none());
    assert!(paper["exits"][0].get("quote").is_none());
    assert_eq!(paper["exits"][1]["strategy_id"], "paper-exit");
    assert!(paper["exits"][1].get("quote").is_none());

    let mut live_round = round(8, true, 0.1, None);
    live_round.book = Book::Live;
    let live = build_position_details(&live_round, Vec::new(), Vec::new(), Vec::new()).await;
    let live = serde_json::to_value(live).unwrap();
    assert_eq!(live["book"], "live");
    let keys =
        |value: &Value| -> Vec<String> { value.as_object().unwrap().keys().cloned().collect() };
    assert_eq!(keys(&paper), keys(&live), "both books answer one shape");
    assert_eq!(keys(&paper["position"]), keys(&live["position"]));
}
