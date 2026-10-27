// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The strategy routes: switching a strategy on applies the validation a save applies, switching one off is always allowed, neither touches the rest of the stored strategy, an exit strategy keeps the entry strategies it manages, and a test reports every rule's verdict, a rule with no data as unknown with the reason, without waiting for candles the OHLCV service cannot read through.

use crate::common;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use common::condition;
use screenerbot::strategies::db::{get_strategy, insert_strategy, update_strategy};
use screenerbot::strategies::engine::EngineConfig;
use screenerbot::strategies::types::{Condition, RuleTree, Strategy, StrategyType};
use screenerbot::webserver::routes;
use screenerbot::webserver::state::AppState;
use serde_json::{json, Value};
use tower::ServiceExt;

/// An isolated data directory with the strategy engine and store initialised.
async fn strategy_system() -> tempfile::TempDir {
    let dir = common::isolated_env();
    screenerbot::strategies::init_strategy_system(EngineConfig {
        evaluation_timeout_ms: 5_000,
        cache_ttl_seconds: 0,
        max_concurrent_evaluations: 10,
    })
    .await
    .expect("initialise the strategy system");
    dir
}

fn stored(id: &str, strategy_type: StrategyType, rule: Condition, enabled: bool) -> Strategy {
    let strategy = Strategy {
        id: id.to_owned(),
        name: id.to_owned(),
        description: Some("Stored description".to_owned()),
        strategy_type,
        enabled,
        priority: 7,
        timeframe: "1h".to_owned(),
        rules: RuleTree::leaf(rule),
        parameters: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
    };
    insert_strategy(&strategy).expect("insert strategy");
    strategy
}

/// Exit-only: a strategy of the entry type that holds it does not pass validation.
fn position_profit() -> Condition {
    condition(
        "PositionProfit",
        &[
            ("comparison", json!("ABOVE")),
            ("threshold_pct", json!(20.0)),
        ],
    )
}

fn liquidity_at_least() -> Condition {
    condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(10.0)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    )
}

async fn set_enabled(id: &str, enabled: bool) -> (StatusCode, Value) {
    let router = routes::create_router(Arc::new(AppState::new()));
    let request = Request::builder()
        .method("PATCH")
        .uri(format!("/api/strategies/{id}/enabled"))
        .header("content-type", "application/json")
        .body(Body::from(json!({ "enabled": enabled }).to_string()))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router responded");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read response body");
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

fn reload(id: &str) -> Strategy {
    get_strategy(id)
        .expect("read strategy")
        .expect("strategy is stored")
}

#[tokio::test]
async fn a_strategy_that_fails_validation_is_not_switched_on() {
    let _dir = strategy_system().await;
    stored(
        "entry-with-exit-rule",
        StrategyType::Entry,
        position_profit(),
        false,
    );

    let (status, body) = set_enabled("entry-with-exit-rule", true).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(
        body["error"]["text"]["id"], "errors-strategies-validation-failed",
        "{body}"
    );
    let after = reload("entry-with-exit-rule");
    assert!(!after.enabled);
    assert_eq!(after.version, 1);
}

#[tokio::test]
async fn a_strategy_that_fails_validation_can_still_be_switched_off() {
    let _dir = strategy_system().await;
    stored(
        "enabled-before-the-rule",
        StrategyType::Entry,
        position_profit(),
        true,
    );

    let (status, body) = set_enabled("enabled-before-the-rule", false).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!reload("enabled-before-the-rule").enabled);
}

#[tokio::test]
async fn switching_a_valid_strategy_on_keeps_the_rest_of_it() {
    let _dir = strategy_system().await;
    let before = stored(
        "liquidity-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        false,
    );

    let (status, body) = set_enabled("liquidity-entry", true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let after = reload("liquidity-entry");
    assert!(after.enabled);
    assert_eq!(after.version, 2);
    assert_eq!(after.timeframe, before.timeframe);
    assert_eq!(after.priority, before.priority);
    assert_eq!(after.description, before.description);
    assert_eq!(
        serde_json::to_value(&after.rules).expect("serialize rules"),
        serde_json::to_value(&before.rules).expect("serialize rules")
    );
}

/// Send one JSON request to the strategies API and read the JSON answer.
async fn send(method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let router = routes::create_router(Arc::new(AppState::new()));
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("build request");
    let response = router.oneshot(request).await.expect("router responded");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read response body");
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn strategy_request(name: &str, strategy_type: &str, rule: Condition, bound: &[&str]) -> Value {
    json!({
        "name": name,
        "strategy_type": strategy_type,
        "rules": serde_json::to_value(RuleTree::leaf(rule)).expect("serialize rules"),
        "entry_strategy_ids": bound,
    })
}

#[tokio::test]
async fn an_exit_strategy_keeps_the_entry_strategies_it_manages() {
    let _dir = strategy_system().await;
    let request = strategy_request(
        "Bound Exit",
        "EXIT",
        position_profit(),
        &["dip-buyer", " dip-buyer ", "", "breakout"],
    );
    let (status, body) = send("POST", "/api/strategies", request).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    // Each id once, in the order given; blanks are not ids.
    assert_eq!(
        reload("bound-exit").entry_strategy_ids,
        ["dip-buyer", "breakout"]
    );
    let (status, detail) = send("GET", "/api/strategies/bound-exit", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(
        detail["entry_strategy_ids"],
        json!(["dip-buyer", "breakout"]),
        "{detail}"
    );

    // An update replaces the binding; an empty one manages every position again.
    let update = strategy_request("Bound Exit", "EXIT", position_profit(), &[]);
    let (status, body) = send("PUT", "/api/strategies/bound-exit", update).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(reload("bound-exit").entry_strategy_ids.is_empty());
}

#[tokio::test]
async fn an_update_that_omits_the_binding_keeps_the_stored_one() {
    let _dir = strategy_system().await;
    let mut strategy = stored("kept-binding", StrategyType::Exit, position_profit(), false);
    strategy.entry_strategy_ids = vec!["dip-buyer".to_owned()];
    update_strategy(&strategy).expect("store the binding");

    let mut update = strategy_request("Kept Binding", "EXIT", position_profit(), &[]);
    update
        .as_object_mut()
        .expect("request object")
        .remove("entry_strategy_ids");
    let (status, body) = send("PUT", "/api/strategies/kept-binding", update).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reload("kept-binding").entry_strategy_ids, ["dip-buyer"]);
}

#[tokio::test]
async fn an_entry_strategy_manages_no_positions_whatever_it_is_sent() {
    let _dir = strategy_system().await;
    let request = strategy_request("Bound Entry", "ENTRY", liquidity_at_least(), &["other"]);
    let (status, body) = send("POST", "/api/strategies", request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(reload("bound-entry").entry_strategy_ids.is_empty());
}

#[tokio::test]
async fn switching_an_exit_strategy_on_keeps_its_binding() {
    let _dir = strategy_system().await;
    let mut strategy = stored("bound-off", StrategyType::Exit, position_profit(), false);
    strategy.entry_strategy_ids = vec!["dip-buyer".to_owned()];
    update_strategy(&strategy).expect("store the binding");

    let (status, body) = set_enabled("bound-off", true).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reload("bound-off").entry_strategy_ids, ["dip-buyer"]);
}

#[test]
fn an_exit_strategy_manages_its_bound_entry_strategies_or_every_position() {
    let mut exit = Strategy {
        id: "exit".to_owned(),
        name: "exit".to_owned(),
        description: None,
        strategy_type: StrategyType::Exit,
        enabled: true,
        priority: 1,
        timeframe: "5m".to_owned(),
        rules: RuleTree::leaf(position_profit()),
        parameters: std::collections::HashMap::new(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
    };
    assert!(exit.manages(Some("dip-buyer")));
    assert!(
        exit.manages(None),
        "unbound: every position, manual and copied ones too"
    );

    exit.entry_strategy_ids = vec!["dip-buyer".to_owned()];
    assert!(exit.manages(Some("dip-buyer")));
    assert!(!exit.manages(Some("breakout")));
    assert!(
        !exit.manages(None),
        "bound: a position no entry strategy opened is not its"
    );
}

#[tokio::test]
async fn a_test_whose_rule_has_no_data_answers_unknown_with_the_reason() {
    let _dir = strategy_system().await;
    stored(
        "liquidity-test",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );

    let (status, body) = send(
        "POST",
        "/api/strategies/liquidity-test/test",
        json!({ "token_mint": "mint", "current_price": 1.0 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        body["result"].is_null(),
        "no liquidity is neither a pass nor a fail: {body}"
    );
    assert_eq!(
        body["unknown_reason"]["id"],
        json!("strategies-error-missing-data"),
        "{body}"
    );

    let (status, body) = send(
        "POST",
        "/api/strategies/liquidity-test/test",
        json!({
            "token_mint": "mint",
            "current_price": 1.0,
            "market_data": { "liquidity_native": 25.0 },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], json!(true), "{body}");
    assert!(body["unknown_reason"].is_null(), "{body}");
    assert_eq!(
        body["rules"],
        json!({ "verdict": "pass", "condition_type": "LiquidityLevel" }),
        "the rule reports its own verdict: {body}"
    );
}

#[tokio::test]
async fn a_test_reports_the_verdict_of_every_rule() {
    let _dir = strategy_system().await;
    let mut strategy = stored(
        "rule-by-rule",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );
    strategy.rules = RuleTree::branch(
        screenerbot::strategies::types::LogicalOperator::Or,
        vec![
            RuleTree::leaf(condition(
                "LiquidityLevel",
                &[
                    ("threshold", json!(100.0)),
                    ("comparison", json!("GREATER_EQUAL")),
                ],
            )),
            RuleTree::leaf(liquidity_at_least()),
        ],
    );
    update_strategy(&strategy).expect("update strategy");

    let (status, body) = send(
        "POST",
        "/api/strategies/rule-by-rule/test",
        json!({
            "token_mint": "mint",
            "current_price": 1.0,
            "market_data": { "liquidity_native": 25.0 },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["result"], json!(true), "{body}");
    assert_eq!(
        body["rules"],
        json!({
            "verdict": "pass",
            "operator": "OR",
            "children": [
                { "verdict": "fail", "condition_type": "LiquidityLevel" },
                { "verdict": "pass", "condition_type": "LiquidityLevel" },
            ],
        }),
        "{body}"
    );
}

#[tokio::test]
async fn a_test_without_a_price_reads_the_tokens_live_context() {
    let _dir = strategy_system().await;
    stored("live-exit", StrategyType::Exit, position_profit(), true);

    // No open position holds the mint, so the position rule has nothing to read.
    let (status, body) = send(
        "POST",
        "/api/strategies/live-exit/test",
        json!({ "token_mint": "mint-without-position" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["result"].is_null(), "{body}");
    assert_eq!(body["rules"]["verdict"], json!("unknown"), "{body}");
    assert_eq!(
        body["rules"]["reason"]["id"],
        json!("strategies-error-missing-data"),
        "{body}"
    );
}

#[tokio::test]
async fn a_live_test_of_a_candle_rule_without_a_running_ohlcv_service_answers_unknown_at_once() {
    let _dir = strategy_system().await;
    stored(
        "live-candles",
        StrategyType::Entry,
        condition(
            "Rsi",
            &[
                ("period", json!(14)),
                ("comparison", json!("BELOW")),
                ("threshold", json!(30)),
                ("timeframe", json!("1h")),
            ],
        ),
        true,
    );

    // No service manager runs the OHLCV service, so no candles can be read through: the
    // test does not wait for them and the candle rule stays unknown, never answered.
    let started = std::time::Instant::now();
    let (status, body) = send(
        "POST",
        "/api/strategies/live-candles/test",
        json!({ "token_mint": "mint-without-candles" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(5),
        "the candle wait applies only while the OHLCV service runs: {:?}",
        started.elapsed()
    );
    assert!(body["result"].is_null(), "{body}");
    assert_eq!(body["rules"]["verdict"], json!("unknown"), "{body}");
    assert_eq!(
        body["rules"]["reason"]["id"],
        json!("strategies-error-missing-data"),
        "{body}"
    );
}

#[tokio::test]
async fn a_test_refuses_supplied_data_without_a_price() {
    let _dir = strategy_system().await;
    stored(
        "liquidity-test",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );
    let (status, body) = send(
        "POST",
        "/api/strategies/liquidity-test/test",
        json!({
            "token_mint": "mint",
            "market_data": { "liquidity_native": 25.0 },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}
