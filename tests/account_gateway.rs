// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The transaction-sending switch (`POST /api/account/gateway`) is written to
//! config.toml once a configuration exists, and stays in memory before setup.
//!
//! Each test runs in a process of its own (nextest isolates every test): the
//! config `OnceLock`, the data directory and the initialization flag are
//! process-wide.

use std::sync::atomic::Ordering;

use crate::common;

use axum::Json;
use screenerbot::webserver::routes::account::handlers::set_gateway_enabled;

fn data_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();
    dir
}

async fn switch_sending(enabled: bool) {
    let body = serde_json::from_value(serde_json::json!({ "enabled": enabled }))
        .expect("toggle body parses");
    let response = set_gateway_enabled(Json(body)).await;
    assert!(response.status().is_success(), "the switch is accepted");
}

#[tokio::test]
async fn sending_switch_is_saved_once_setup_is_complete() {
    let _dir = data_dir();
    screenerbot::global::INITIALIZATION_COMPLETE.store(true, Ordering::SeqCst);

    switch_sending(true).await;

    let saved = std::fs::read_to_string(screenerbot::paths::get_config_path())
        .expect("config.toml is written");
    let config: toml::Value = toml::from_str(&saved).expect("config.toml parses");
    assert_eq!(
        config["account"]["use_gateway_rpc"].as_bool(),
        Some(true),
        "a restart keeps the switch"
    );
}

#[tokio::test]
async fn sending_switch_creates_no_config_before_setup() {
    let _dir = data_dir();

    switch_sending(true).await;

    assert!(
        !screenerbot::paths::get_config_path().exists(),
        "setup decides when config.toml is first written"
    );
    assert!(screenerbot::config::with_config(|cfg| cfg
        .account
        .use_gateway_rpc));
}
