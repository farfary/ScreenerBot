// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The strategy routes: the overview reads each strategy from the book of its mode with its last evaluation and signal times from the evaluation totals and answers stopped and in Explore Mode, a new strategy starts on paper and arms live only with the confirmation phrase while the Auto Trader can trade live, an update never changes the mode, a priority outside 1-100 is refused on create, update, import and inline validation, an update and a validation of a stored strategy answer whether the save changes its rules, the roster order is the entry evaluation order, readiness reads the paper rounds since the last rules change in the current run, switching a strategy on applies the validation a save applies, switching one off is always allowed, neither touches the rest of the stored strategy, an exit strategy keeps the entry strategies it manages, and a test reports every rule's verdict, a rule with no data as unknown with the reason, without waiting for candles the OHLCV service cannot read through.

use crate::common;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use common::condition;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::strategies::db::{get_strategy, insert_strategy, update_strategy};
use screenerbot::strategies::engine::EngineConfig;
use screenerbot::strategies::types::{Condition, RuleTree, Strategy, StrategyType};
use screenerbot::trader::paper::TradingMode;
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
        mode: TradingMode::Live,
        rules_changed_at: chrono::Utc::now(),
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

#[tokio::test]
async fn a_new_strategy_starts_on_paper_and_arms_live_only_with_the_phrase() {
    let _dir = strategy_system().await;
    let request = strategy_request("Paper Entry", "ENTRY", liquidity_at_least(), &[]);
    let (status, body) = send("POST", "/api/strategies", request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reload("paper-entry").mode, TradingMode::Paper);
    let (_, detail) = send("GET", "/api/strategies/paper-entry", Value::Null).await;
    assert_eq!(detail["mode"], "paper", "{detail}");

    for confirmation in [Value::Null, json!(""), json!("arm live strategy")] {
        let (status, body) = send(
            "PATCH",
            "/api/strategies/paper-entry/mode",
            json!({ "mode": "live", "confirmation": confirmation }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert_eq!(
            body["error"]["text"]["id"], "errors-strategies-live-confirmation-required",
            "{body}"
        );
        assert_eq!(reload("paper-entry").mode, TradingMode::Paper);
    }

    screenerbot::global::INITIALIZATION_COMPLETE.store(true, Ordering::SeqCst);
    let (status, body) = send(
        "PATCH",
        "/api/strategies/paper-entry/mode",
        json!({
            "mode": "live",
            "confirmation": screenerbot::trader::paper::STRATEGY_LIVE_CONFIRMATION,
        }),
    )
    .await;
    screenerbot::global::INITIALIZATION_COMPLETE.store(false, Ordering::SeqCst);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["mode"], "live", "{body}");
    let armed = reload("paper-entry");
    assert_eq!(armed.mode, TradingMode::Live);
    assert_eq!(armed.version, 1, "the mode is not a rule edit");

    let (status, body) = send(
        "PATCH",
        "/api/strategies/paper-entry/mode",
        json!({ "mode": "paper" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reload("paper-entry").mode, TradingMode::Paper);

    let (status, _) = send(
        "PATCH",
        "/api/strategies/missing/mode",
        json!({ "mode": "paper" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// Arming live goes through the live trading gate: an unfinished setup or the emergency
/// stop refuses it with the reason, and returning to paper stays allowed.
#[tokio::test]
async fn a_strategy_arms_live_only_while_the_auto_trader_can_trade_live() {
    let _dir = strategy_system().await;
    let request = strategy_request("Gated Entry", "ENTRY", liquidity_at_least(), &[]);
    let (status, body) = send("POST", "/api/strategies", request).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let arm = json!({
        "mode": "live",
        "confirmation": screenerbot::trader::paper::STRATEGY_LIVE_CONFIRMATION,
    });

    screenerbot::global::INITIALIZATION_COMPLETE.store(false, Ordering::SeqCst);
    let (status, body) = send("PATCH", "/api/strategies/gated-entry/mode", arm.clone()).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(
        body["error"]["text"]["id"], "paper-live-block-setup-incomplete",
        "{body}"
    );
    assert_eq!(reload("gated-entry").mode, TradingMode::Paper);

    screenerbot::global::INITIALIZATION_COMPLETE.store(true, Ordering::SeqCst);
    screenerbot::global::set_force_stopped(true, Some("test"));
    let (status, body) = send("PATCH", "/api/strategies/gated-entry/mode", arm).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(
        body["error"]["text"]["id"], "paper-live-block-force-stop",
        "{body}"
    );
    assert_eq!(reload("gated-entry").mode, TradingMode::Paper);

    let (status, body) = send(
        "PATCH",
        "/api/strategies/gated-entry/mode",
        json!({ "mode": "paper" }),
    )
    .await;
    screenerbot::global::set_force_stopped(false, None);
    screenerbot::global::INITIALIZATION_COMPLETE.store(false, Ordering::SeqCst);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reload("gated-entry").mode, TradingMode::Paper);
}

#[tokio::test]
async fn an_update_never_changes_the_mode() {
    let _dir = strategy_system().await;
    stored(
        "armed-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        false,
    );
    let update = strategy_request("Armed Entry", "ENTRY", liquidity_at_least(), &[]);
    let (status, body) = send("PUT", "/api/strategies/armed-entry", update).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reload("armed-entry").mode, TradingMode::Live);
}

/// A strategy request carrying `priority`.
fn with_priority(mut request: Value, priority: i64) -> Value {
    request["priority"] = json!(priority);
    request
}

/// A refused priority answers 400 with the range message.
fn assert_priority_refused(status: StatusCode, body: &Value) {
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], "INVALID_INPUT", "{body}");
    assert_eq!(
        body["error"]["text"]["id"], "errors-strategies-priority-out-of-range",
        "{body}"
    );
}

#[tokio::test]
async fn a_priority_outside_one_to_one_hundred_is_refused_on_create() {
    let _dir = strategy_system().await;
    for priority in [0, 101, -5, i64::from(i32::MAX)] {
        let request = with_priority(
            strategy_request("Ranked Entry", "ENTRY", liquidity_at_least(), &[]),
            priority,
        );
        let (status, body) = send("POST", "/api/strategies", request).await;
        assert_priority_refused(status, &body);
        assert!(
            get_strategy("ranked-entry")
                .expect("read strategy")
                .is_none(),
            "priority {priority} was stored"
        );
    }
    for priority in [1, 100] {
        let name = format!("Ranked {priority}");
        let request = with_priority(
            strategy_request(&name, "ENTRY", liquidity_at_least(), &[]),
            priority,
        );
        let (status, body) = send("POST", "/api/strategies", request).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(
            reload(&format!("ranked-{priority}")).priority,
            priority as i32
        );
    }
}

#[tokio::test]
async fn a_priority_outside_one_to_one_hundred_is_refused_on_update() {
    let _dir = strategy_system().await;
    stored("ranked", StrategyType::Entry, liquidity_at_least(), false);
    for priority in [0, 101] {
        let update = with_priority(
            strategy_request("Ranked", "ENTRY", liquidity_at_least(), &[]),
            priority,
        );
        let (status, body) = send("PUT", "/api/strategies/ranked", update).await;
        assert_priority_refused(status, &body);
        let kept = reload("ranked");
        assert_eq!(kept.priority, 7, "the stored priority is kept");
        assert_eq!(kept.version, 1, "a refused update writes nothing");
    }
    for priority in [1, 100] {
        let update = with_priority(
            strategy_request("Ranked", "ENTRY", liquidity_at_least(), &[]),
            priority,
        );
        let (status, body) = send("PUT", "/api/strategies/ranked", update).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(reload("ranked").priority, priority as i32);
    }
}

/// A strategy file is read in the dashboard and saved through the create route as a
/// switched-off strategy, so an imported priority meets the same range.
#[tokio::test]
async fn an_imported_priority_outside_one_to_one_hundred_is_refused() {
    let _dir = strategy_system().await;
    let imported = |name: &str, priority: i64| {
        json!({
            "name": name,
            "description": null,
            "strategy_type": "ENTRY",
            "enabled": false,
            "priority": priority,
            "timeframe": "5m",
            "rules": serde_json::to_value(RuleTree::leaf(liquidity_at_least()))
                .expect("serialize rules"),
            "parameters": {},
            "entry_strategy_ids": [],
        })
    };
    for priority in [0, 101] {
        let (status, body) = send("POST", "/api/strategies", imported("Imported", priority)).await;
        assert_priority_refused(status, &body);
        assert!(get_strategy("imported").expect("read strategy").is_none());
    }
    for priority in [1, 100] {
        let name = format!("Imported {priority}");
        let (status, body) = send("POST", "/api/strategies", imported(&name, priority)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let saved = reload(&format!("imported-{priority}"));
        assert_eq!(saved.priority, priority as i32);
        assert!(!saved.enabled);
    }
}

/// The inline validation of an unsaved strategy applies the range a save applies, so the
/// editor never reports a strategy valid that the save then refuses.
#[tokio::test]
async fn an_inline_validation_refuses_a_priority_outside_one_to_one_hundred() {
    let _dir = strategy_system().await;
    for priority in [0, 101] {
        let request = with_priority(
            strategy_request("Ranked Entry", "ENTRY", liquidity_at_least(), &[]),
            priority,
        );
        let (status, body) = send("POST", "/api/strategies/validate", request).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["valid"], false, "priority {priority}: {body}");
        assert_eq!(
            body["errors"][0]["id"], "errors-strategies-priority-out-of-range",
            "priority {priority}: {body}"
        );
    }
    for priority in [1, 100] {
        let request = with_priority(
            strategy_request("Ranked Entry", "ENTRY", liquidity_at_least(), &[]),
            priority,
        );
        let (status, body) = send("POST", "/api/strategies/validate", request).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body["valid"], true, "priority {priority}: {body}");
    }
}

/// The save request of the stored entry strategy `id` as it is: its own name,
/// timeframe, priority and rules, so a test changes one field at a time.
fn unchanged_entry_request(id: &str) -> Value {
    json!({
        "name": id,
        "strategy_type": "ENTRY",
        "enabled": false,
        "priority": 7,
        "timeframe": "1h",
        "rules": serde_json::to_value(RuleTree::leaf(liquidity_at_least())).expect("serialize rules"),
    })
}

/// The edits a save may carry, each with whether it changes the rules the strategy
/// trades on: a rule or the timeframe does; a name, a priority or the On switch does not.
fn entry_edits() -> Vec<(&'static str, &'static str, Value, bool)> {
    let changed_rule = serde_json::to_value(RuleTree::leaf(condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(25.0)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    )))
    .expect("serialize rules");
    vec![
        ("rule", "rules", changed_rule, true),
        ("timeframe", "timeframe", json!("15m"), true),
        ("name", "name", json!("Renamed Entry"), false),
        ("priority", "priority", json!(9), false),
        ("on switch", "enabled", json!(true), false),
    ]
}

/// The update answers whether it changed the rules, by the store's fingerprint.
#[tokio::test]
async fn an_update_answers_whether_it_changed_the_rules() {
    let _dir = strategy_system().await;
    for (edit, field, value, changes_rules) in entry_edits() {
        let id = format!("edited-{}", edit.replace(' ', "-"));
        stored(&id, StrategyType::Entry, liquidity_at_least(), false);
        let mut request = unchanged_entry_request(&id);
        request[field] = value;
        let (status, body) = send("PUT", &format!("/api/strategies/{id}"), request).await;
        assert_eq!(status, StatusCode::OK, "{edit}: {body}");
        assert_eq!(body["rules_changed"], changes_rules, "{edit}: {body}");
    }
}

/// A draft of a stored strategy validates with the answer its save would give, and
/// reads only: the stored strategy is unchanged afterwards. A draft without an id answers
/// no rules change, and an id no strategy holds is not found.
#[tokio::test]
async fn a_validation_of_a_stored_strategy_answers_whether_the_save_changes_its_rules() {
    let _dir = strategy_system().await;
    for (edit, field, value, changes_rules) in entry_edits() {
        let id = format!("drafted-{}", edit.replace(' ', "-"));
        stored(&id, StrategyType::Entry, liquidity_at_least(), false);
        let before = reload(&id);
        let mut request = unchanged_entry_request(&id);
        request[field] = value;
        let (status, body) = send(
            "POST",
            &format!("/api/strategies/validate?id={id}"),
            request,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{edit}: {body}");
        assert_eq!(body["valid"], true, "{edit}: {body}");
        assert_eq!(body["rules_changed"], changes_rules, "{edit}: {body}");
        let after = reload(&id);
        assert_eq!(after.version, before.version, "{edit}: validation wrote");
        assert_eq!(after.rules_changed_at, before.rules_changed_at, "{edit}");
    }

    let (status, body) = send(
        "POST",
        "/api/strategies/validate",
        unchanged_entry_request("unsaved"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["valid"], true, "{body}");
    assert!(body.get("rules_changed").is_none(), "{body}");

    let (status, body) = send(
        "POST",
        "/api/strategies/validate?id=missing",
        unchanged_entry_request("missing"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
}

#[tokio::test]
async fn a_strategy_without_paper_rounds_reads_as_not_ready() {
    let _dir = strategy_system().await;
    stored(
        "fresh-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        false,
    );
    stored("fresh-exit", StrategyType::Exit, position_profit(), false);

    let (status, body) = send("GET", "/api/strategies/fresh-entry/readiness", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["ready"], false, "{body}");
    let ids: Vec<&str> = body["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .map(|check| check["id"].as_str().expect("check id"))
        .collect();
    assert_eq!(
        ids,
        ["history", "profit", "priced", "exits", "runtime"],
        "{body}"
    );
    assert_eq!(body["checks"][0]["passed"], false, "{body}");
    assert_eq!(
        body["checks"][3]["passed"], true,
        "a strategy without failed paper exits reads as filling: {body}"
    );
    assert_eq!(
        body["checks"][4]["passed"], false,
        "an unfinished setup blocks live trading: {body}"
    );
    assert_eq!(
        body["live_confirmation"],
        screenerbot::trader::paper::STRATEGY_LIVE_CONFIRMATION,
        "{body}"
    );

    let (status, body) = send("GET", "/api/strategies/fresh-exit/readiness", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let ids: Vec<&str> = body["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .map(|check| check["id"].as_str().expect("check id"))
        .collect();
    assert_eq!(ids, ["history", "priced", "exits", "runtime"], "{body}");

    let (status, _) = send("GET", "/api/strategies/missing/readiness", Value::Null).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The history check of the readiness of strategy `id`, as the route answers it.
async fn history_check(id: &str) -> Value {
    let (status, body) = send(
        "GET",
        &format!("/api/strategies/{id}/readiness"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .find(|check| check["id"] == "history")
        .cloned()
        .unwrap_or_else(|| panic!("no history check: {body}"))
}

/// A rules change inside the current paper run restarts the strategy's readiness: the
/// history counts the rounds since the change and names it. A change of anything but the
/// rules leaves the readiness as it was.
#[tokio::test]
async fn a_rules_change_restarts_the_readiness_and_a_priority_change_does_not() {
    let _dir = strategy_system().await;
    let wallet = common::configure_trading_wallet();
    let strategy = stored(
        "paper-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        false,
    );
    let book =
        screenerbot::positions::paper::initialize(screenerbot::chains::ChainId::Solana, &wallet)
            .await
            .expect("load the paper book");
    let run = book.account(&wallet, 1.0).await.expect("start a paper run");
    assert!(run.started_at >= strategy.rules_changed_at);

    let whole_run = history_check("paper-entry").await;
    assert_eq!(
        whole_run["detail"]["id"], "paper-readiness-history-short",
        "{whole_run}"
    );
    assert!(
        whole_run["detail"]["args"].get("changed").is_none(),
        "{whole_run}"
    );

    let reprioritized = Strategy {
        priority: 9,
        ..reload("paper-entry")
    };
    assert!(!update_strategy(&reprioritized).expect("update the priority"));
    assert_eq!(
        history_check("paper-entry").await,
        whole_run,
        "a priority change keeps the readiness"
    );

    let mut rules_changed = reload("paper-entry");
    rules_changed.rules = RuleTree::leaf(condition(
        "LiquidityLevel",
        &[
            ("threshold", json!(25.0)),
            ("comparison", json!("GREATER_EQUAL")),
        ],
    ));
    assert!(update_strategy(&rules_changed).expect("update the rules"));
    let changed_at = reload("paper-entry").rules_changed_at;
    assert!(changed_at > run.started_at);

    let since_change = history_check("paper-entry").await;
    assert_eq!(since_change["passed"], false, "{since_change}");
    assert_eq!(
        since_change["detail"]["id"], "strategies-readiness-history-since-short",
        "{since_change}"
    );
    let args = &since_change["detail"]["args"];
    assert_eq!(args["count"]["value"], 0, "{since_change}");
    assert_eq!(
        args["changed"],
        json!({ "type": "time", "value": changed_at.timestamp_millis() }),
        "{since_change}"
    );
}

/// The overview row of strategy `id`.
fn overview_row<'a>(overview: &'a Value, id: &str) -> &'a Value {
    overview["strategies"]
        .as_array()
        .expect("strategies")
        .iter()
        .find(|row| row["item"]["id"] == id)
        .unwrap_or_else(|| panic!("no overview row for {id}: {overview}"))
}

/// A live round of `mint` that entry strategy `strategy_id` opened, in memory.
async fn live_round(mint: &str, strategy_id: &str, pnl: Option<f64>) {
    let mut position = common::test_position(0.001, 0.1);
    position.id = None;
    position.mint = mint.to_owned();
    position.entry_transaction_signature = Some(format!("entry-{mint}"));
    position.origin = screenerbot::positions::PositionOrigin::Auto {
        strategy_id: Some(strategy_id.to_owned()),
    };
    match pnl {
        Some(pnl) => {
            position.exit_time = Some(chrono::Utc::now());
            position.exit_transaction_signature = Some(format!("exit-{mint}"));
            position.transaction_exit_verified = true;
            position.pnl = Some(pnl);
            position.pnl_percent = Some(pnl / 0.1 * 100.0);
        }
        None => position.unrealized_pnl = Some(0.004),
    }
    screenerbot::positions::state::add_position(position).await;
}

/// A closed live round of `mint` that entry strategy `strategy_id` opened and the live
/// performance does not count: archived, or with its exit not yet verified on chain.
async fn uncounted_live_round(mint: &str, strategy_id: &str, archived: bool) {
    let mut position = common::test_position(0.001, 0.1);
    position.id = None;
    position.mint = mint.to_owned();
    position.entry_transaction_signature = Some(format!("entry-{mint}"));
    position.origin = screenerbot::positions::PositionOrigin::Auto {
        strategy_id: Some(strategy_id.to_owned()),
    };
    position.exit_time = Some(chrono::Utc::now());
    position.exit_transaction_signature = Some(format!("exit-{mint}"));
    position.transaction_exit_verified = archived;
    position.archived = archived;
    position.pnl = Some(0.5);
    position.pnl_percent = Some(500.0);
    screenerbot::positions::state::add_position(position).await;
}

#[tokio::test]
async fn the_overview_without_strategies_states_the_trader_once() {
    let _dir = strategy_system().await;
    let (status, body) = send("GET", "/api/strategies/overview", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["strategies"], json!([]), "{body}");
    assert_eq!(
        body["counts"],
        json!({ "entry": { "enabled": 0, "total": 0 }, "exit": { "enabled": 0, "total": 0 } }),
        "{body}"
    );
    let kinds: Vec<&str> = body["rule_exits"]
        .as_array()
        .expect("rule exits")
        .iter()
        .map(|rule| rule["kind"].as_str().expect("kind"))
        .collect();
    assert_eq!(
        kinds,
        ["stop_loss", "trailing_stop", "take_profit", "time_exit"],
        "{body}"
    );
    assert!(body["run_state"].is_string(), "{body}");
}

/// Each row reads the book of its mode: a live entry its live rounds, a paper entry its
/// paper run, with the open positions of the other book counted apart and never summed in.
#[tokio::test]
async fn the_overview_reads_each_strategy_from_the_book_of_its_mode() {
    let _dir = strategy_system().await;
    stored(
        "live-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );
    stored(
        "paper-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );
    screenerbot::strategies::db::set_strategy_mode("paper-entry", TradingMode::Paper)
        .expect("return to paper");
    stored("live-exit", StrategyType::Exit, position_profit(), true);
    stored(
        "idle-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        false,
    );

    live_round("LiveOpenMint", "live-entry", None).await;
    live_round("LiveWonMint", "live-entry", Some(0.02)).await;
    live_round("LiveLostMint", "live-entry", Some(-0.01)).await;
    live_round("PaperEntryLiveMint", "paper-entry", None).await;
    uncounted_live_round("LiveArchivedMint", "live-entry", true).await;
    uncounted_live_round("LiveUnverifiedMint", "live-entry", false).await;

    let (status, body) = send("GET", "/api/strategies/overview", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body["counts"]["entry"],
        json!({ "enabled": 2, "total": 3 }),
        "{body}"
    );
    assert_eq!(
        body["counts"]["exit"],
        json!({ "enabled": 1, "total": 1 }),
        "{body}"
    );

    let live = overview_row(&body, "live-entry");
    assert_eq!(live["book"], "live", "{live}");
    assert_eq!(live["order"], 1, "{live}");
    assert_eq!(live["open"], 1, "{live}");
    assert_eq!(live["open_other_book"], 0, "{live}");
    assert_eq!(live["unpriced"], 0, "{live}");
    assert_eq!(
        live["rounds"]["closed"], 2,
        "archived rounds and rounds with an unverified exit are not counted: {live}"
    );
    assert_eq!(live["rounds"]["winners"], 1, "{live}");
    assert_eq!(live["rounds"]["win_rate_pct"], 50.0, "{live}");
    assert!(live["readiness"].is_null(), "{live}");

    let paper = overview_row(&body, "paper-entry");
    assert_eq!(paper["book"], "paper", "{paper}");
    assert_eq!(paper["order"], 2, "{paper}");
    assert_eq!(paper["open"], 0, "{paper}");
    assert_eq!(
        paper["open_other_book"], 1,
        "the live position it opened before returning to paper: {paper}"
    );
    assert!(
        paper["unrealized_native"].is_null(),
        "the live book's P&L is not the paper figure: {paper}"
    );
    assert_eq!(paper["rounds"]["closed"], 0, "{paper}");
    assert!(paper["rounds"]["win_rate_pct"].is_null(), "{paper}");
    assert_eq!(paper["readiness"]["closed"], 0, "{paper}");
    assert_eq!(paper["readiness"]["ready"], false, "{paper}");
    assert_eq!(paper["readiness"]["failing"], "history", "{paper}");

    let exit = overview_row(&body, "live-exit");
    assert!(exit["order"].is_null(), "{exit}");
    assert!(exit["open"].is_null(), "{exit}");
    assert_eq!(
        exit["watching"], 2,
        "a live exit judges every open position it manages: {exit}"
    );
    assert!(
        exit["rounds"].is_null(),
        "live closes are not attributed to exit strategies: {exit}"
    );

    let idle = overview_row(&body, "idle-entry");
    assert_eq!(idle["status"], "off", "{idle}");
    assert!(idle["order"].is_null(), "{idle}");
}

/// A time the overview serialized, `None` for null.
fn overview_time(value: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    value.as_str().map(|text| {
        chrono::DateTime::parse_from_rfc3339(text)
            .unwrap_or_else(|e| panic!("{text}: {e}"))
            .with_timezone(&chrono::Utc)
    })
}

/// Each row's last evaluation and last signal are its evaluation totals' last times; a
/// strategy never evaluated has neither.
#[tokio::test]
async fn the_overview_reads_the_last_times_from_the_evaluation_totals() {
    use screenerbot::strategies::db::{record_evaluations, EvaluationRecord};
    use screenerbot::strategies::types::EvaluationResult;

    let _dir = strategy_system().await;
    stored("evaluated", StrategyType::Entry, liquidity_at_least(), true);
    stored(
        "never-evaluated",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );
    let signalled_at = chrono::Utc::now() - chrono::Duration::minutes(5);
    let evaluated_at = signalled_at + chrono::Duration::minutes(1);
    let evaluation = |signalled: bool, at| EvaluationRecord {
        result: EvaluationResult {
            strategy_id: "evaluated".to_owned(),
            result: signalled,
            confidence: 1.0,
            execution_time_ms: 1,
            details: std::collections::HashMap::new(),
        },
        evaluated_at: at,
    };
    record_evaluations(&[
        evaluation(true, signalled_at),
        evaluation(false, evaluated_at),
    ])
    .expect("record the evaluations");

    let (status, body) = send("GET", "/api/strategies/overview", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let evaluated = overview_row(&body, "evaluated");
    let millis = |time: chrono::DateTime<chrono::Utc>| time.timestamp_millis();
    assert_eq!(
        overview_time(&evaluated["last_evaluated_at"]).map(millis),
        Some(millis(evaluated_at)),
        "{evaluated}"
    );
    assert_eq!(
        overview_time(&evaluated["last_signal_at"]).map(millis),
        Some(millis(signalled_at)),
        "{evaluated}"
    );
    let never = overview_row(&body, "never-evaluated");
    assert!(never["last_evaluated_at"].is_null(), "{never}");
    assert!(never["last_signal_at"].is_null(), "{never}");
}

/// The roster ranks enabled entry strategies in the order the entry monitor evaluates
/// them, whatever their mode, with ties on priority.
#[tokio::test]
async fn the_overview_order_is_the_entry_evaluation_order() {
    let _dir = strategy_system().await;
    for (id, priority, enabled, mode) in [
        ("zeta", 5, true, TradingMode::Live),
        ("alpha", 5, true, TradingMode::Paper),
        ("late", 50, true, TradingMode::Live),
        ("early", 1, true, TradingMode::Paper),
        ("switched-off", 1, false, TradingMode::Live),
    ] {
        let mut strategy = stored(id, StrategyType::Entry, liquidity_at_least(), enabled);
        strategy.priority = priority;
        update_strategy(&strategy).expect("set priority");
        screenerbot::strategies::db::set_strategy_mode(id, mode).expect("set mode");
    }
    stored("exit", StrategyType::Exit, position_profit(), true);

    let evaluated: Vec<String> =
        screenerbot::strategies::db::get_enabled_strategies(StrategyType::Entry)
            .expect("read the evaluation order")
            .into_iter()
            .map(|strategy| strategy.id)
            .collect();
    let (status, body) = send("GET", "/api/strategies/overview", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let mut ranked: Vec<(u64, String)> = body["strategies"]
        .as_array()
        .expect("strategies")
        .iter()
        .filter_map(|row| {
            row["order"]
                .as_u64()
                .map(|order| (order, row["item"]["id"].as_str().expect("id").to_owned()))
        })
        .collect();
    ranked.sort();
    let ranks: Vec<u64> = ranked.iter().map(|(order, _)| *order).collect();
    let roster: Vec<String> = ranked.into_iter().map(|(_, id)| id).collect();
    assert_eq!(roster, evaluated, "{body}");
    assert_eq!(ranks, [1, 2, 3, 4], "{body}");
}

#[tokio::test]
async fn a_stopped_trader_reads_not_running_on_every_enabled_strategy() {
    let _dir = strategy_system().await;
    assert!(
        !screenerbot::config::with_config(|cfg| cfg.trader.enabled),
        "the Auto Trader starts switched off"
    );
    stored("on-entry", StrategyType::Entry, liquidity_at_least(), true);
    stored("on-exit", StrategyType::Exit, position_profit(), true);
    stored(
        "off-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        false,
    );

    let (status, body) = send("GET", "/api/strategies/overview", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["run_state"], "stopped", "{body}");
    assert_eq!(body["notice"], "trader_off", "{body}");
    assert_eq!(overview_row(&body, "on-entry")["status"], "not_running");
    assert_eq!(overview_row(&body, "on-exit")["status"], "not_running");
    assert_eq!(overview_row(&body, "off-entry")["status"], "off");
}

/// Explore Mode has no trader and no paper book: every row reads setup required, with no
/// figure, and the overview still answers.
#[tokio::test]
async fn the_overview_answers_in_explore_mode_without_figures() {
    let _dir = strategy_system().await;
    stored(
        "explore-entry",
        StrategyType::Entry,
        liquidity_at_least(),
        true,
    );
    stored("explore-exit", StrategyType::Exit, position_profit(), false);
    live_round("ExploreMint", "explore-entry", None).await;

    screenerbot::global::set_explore_mode(true);
    let (status, body) = send("GET", "/api/strategies/overview", Value::Null).await;
    screenerbot::global::set_explore_mode(false);
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["run_state"], "explore", "{body}");
    assert_eq!(body["notice"], "setup_required", "{body}");
    for id in ["explore-entry", "explore-exit"] {
        let row = overview_row(&body, id);
        assert_eq!(row["status"], "setup_required", "{row}");
        for figure in ["open", "open_other_book", "watching", "rounds", "readiness"] {
            assert!(row[figure].is_null(), "{figure}: {row}");
        }
    }
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
        mode: TradingMode::Live,
        rules_changed_at: chrono::Utc::now(),
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
        json!({ "token_mint": Pubkey::new_unique().to_string() }),
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
async fn a_live_test_of_a_token_no_enabled_chain_accepts_is_refused() {
    let _dir = strategy_system().await;
    stored("live-exit", StrategyType::Exit, position_profit(), true);

    // Without a supplied price the test reads the token's live context on its own chain,
    // so a token that is no address on any enabled chain is refused before any read.
    let (status, body) = send(
        "POST",
        "/api/strategies/live-exit/test",
        json!({ "token_mint": "not-an-address" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], json!("INVALID_INPUT"), "{body}");
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
        json!({ "token_mint": Pubkey::new_unique().to_string() }),
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
