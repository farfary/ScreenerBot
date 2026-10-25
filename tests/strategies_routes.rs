// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The strategy enabled-state route: switching a strategy on applies the validation a save applies, switching one off is always allowed, and neither touches the rest of the stored strategy.

use crate::common;

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use common::condition;
use screenerbot::strategies::db::{get_strategy, insert_strategy};
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
