// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The live-app bridge as a paired connection reaches it: reading a submitted
//! trade's outcome through `get_trade_status`.
//!
//! Its own file (own test binary, own process) because it loads the global
//! configuration, which the bridge reads for `agent_control.enabled`, and points
//! the agent-control store at a temp database before the pool is first built.

use std::sync::{LazyLock, Mutex, MutexGuard, PoisonError};

use screenerbot::agent_control::bridge::{self, CallOutcome, TRADE_STATUS_TOOL};
use screenerbot::agent_control::submissions;
use screenerbot::agent_control::{pairing, store, PermissionLevel, ToolPermissions, ToolResult};
use screenerbot::config::schemas::Config;
use screenerbot::config::utils::{load_config_from_path, save_config_to_file};
use serde_json::json;

static SETUP: LazyLock<()> = LazyLock::new(|| {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::env::set_var(
        "SCREENERBOT_AGENT_CONTROL_DB",
        dir.path().join("agent_control.db"),
    );
    let data = dir.path().join("data");
    std::fs::create_dir_all(&data).expect("data dir");
    let path = data.join("config.toml");
    let path = path.to_str().expect("utf-8 temp path").to_owned();
    save_config_to_file(&Config::default(), &path, false).expect("write default config");
    load_config_from_path(&path).expect("load config into global");
    store::init().expect("store init");
    std::mem::forget(dir);
});

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn setup() -> MutexGuard<'static, ()> {
    LazyLock::force(&SETUP);
    TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A paired connection that may trade, with `portfolio` as given.
fn trading_connection(label: &str, portfolio: PermissionLevel) -> (String, String) {
    let created = pairing::create(
        label,
        "bridge-test",
        Some(ToolPermissions {
            portfolio,
            ..ToolPermissions::full_access()
        }),
    )
    .expect("pair");
    (created.client_id, created.pairing_secret)
}

async fn read_status(client_id: &str, secret: &str, trade_id: &str) -> ToolResult {
    match bridge::call_tool(
        client_id,
        secret,
        TRADE_STATUS_TOOL,
        json!({ "trade_id": trade_id }),
        "status-poll",
    )
    .await
    .expect("bridge answers")
    {
        CallOutcome::Executed { result } => result,
        other => panic!("a status read answered {other:?} instead of executing"),
    }
}

fn approvals_of(client_id: &str) -> i64 {
    let path = std::env::var("SCREENERBOT_AGENT_CONTROL_DB").expect("db path set by setup()");
    rusqlite::Connection::open(path)
        .expect("open temp agent_control.db")
        .query_row(
            "SELECT COUNT(*) FROM approvals WHERE client_id = ?1",
            rusqlite::params![client_id],
            |row| row.get(0),
        )
        .expect("count approvals")
}

/// Whatever the connection's portfolio policy, its own trade's outcome is read
/// directly and live: never parked on approval, never a frozen first answer.
#[tokio::test]
async fn a_trade_outcome_is_read_live_whatever_the_portfolio_policy() {
    let _guard = setup();
    for portfolio in [PermissionLevel::AskUser, PermissionLevel::Deny] {
        let (client_id, secret) = trading_connection("status-reader", portfolio);
        let trade = submissions::submit_or_reuse(
            &client_id,
            "buy_token",
            &json!({ "mint_address": "LIVE" }),
            "c1",
        )
        .expect("submit");

        let running = read_status(&client_id, &secret, &trade.trade_id).await;
        assert!(running.success, "{portfolio:?}: {:?}", running.error);
        assert_eq!(running.data.as_ref().unwrap()["state"], "submitted");

        assert!(submissions::finish(&trade.trade_id, true, &json!({ "success": true })).unwrap());
        let finished = read_status(&client_id, &secret, &trade.trade_id).await;
        assert_eq!(
            finished.data.as_ref().unwrap()["state"],
            "done",
            "{portfolio:?}: a later poll did not read the live state"
        );
        assert_eq!(
            approvals_of(&client_id),
            0,
            "{portfolio:?}: a poll was parked"
        );

        let listed = bridge::list_tools(&client_id, &secret).expect("list");
        assert!(listed.iter().any(|def| def.name == TRADE_STATUS_TOOL));
    }
}

/// A connection cannot read another connection's trade, and the refusal is
/// indistinguishable from an id that does not exist.
#[tokio::test]
async fn a_trade_of_another_connection_reads_as_not_found() {
    let _guard = setup();
    let (owner, _) = trading_connection("status-owner", PermissionLevel::Allow);
    let (stranger, secret) = trading_connection("status-stranger", PermissionLevel::Allow);
    let trade = submissions::submit_or_reuse(
        &owner,
        "buy_token",
        &json!({ "mint_address": "MINE" }),
        "c1",
    )
    .expect("submit");
    let missing = "00000000-0000-4000-8000-000000000000";

    let foreign = read_status(&stranger, &secret, &trade.trade_id).await;
    let absent = read_status(&stranger, &secret, missing).await;
    assert!(!foreign.success && foreign.data.is_none());
    assert_eq!(
        foreign
            .error
            .as_deref()
            .map(|e| e.replace(&trade.trade_id, "<id>")),
        absent.error.as_deref().map(|e| e.replace(missing, "<id>")),
    );
}

/// The status read is listed only where the connection may trade.
#[test]
fn the_status_read_is_listed_only_for_a_connection_that_may_trade() {
    let _guard = setup();
    let created = pairing::create(
        "status-read-only",
        "bridge-test",
        Some(ToolPermissions {
            trading: PermissionLevel::Deny,
            ..ToolPermissions::full_access()
        }),
    )
    .expect("pair");
    let listed = bridge::list_tools(&created.client_id, &created.pairing_secret).expect("list");
    assert!(!listed.iter().any(|def| def.name == TRADE_STATUS_TOOL));
}
