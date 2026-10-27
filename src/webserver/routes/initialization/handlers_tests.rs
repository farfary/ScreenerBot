// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Completing setup over a vault that already holds a trading wallet stages its replacement, the same key stages nothing, and a staged replacement restarts the process even when the setup settings cannot be saved.

use std::time::Duration;

use axum::body::to_bytes;
use axum::extract::Json;
use axum::response::Response;
use serde_json::Value;

use super::handlers::complete_initialization;
use super::types::CompleteInitializationRequest;
use super::validation::{store_setup_validation, RpcSource};
use crate::chains::solana::accounts::new_test_key;
use crate::chains::ChainId;

const RPC: &str = "https://rpc.example.com";

/// A data directory of its own with `config.toml` and a vault whose trading wallet is
/// `secret`, as a setup completed from Explore Mode finds it.
async fn vault_with(secret: &str) -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", directory.path());
    let _ = crate::config::utils::CONFIG
        .get_or_init(|| std::sync::RwLock::new(crate::config::schemas::Config::default()));
    crate::paths::ensure_all_directories().expect("create data directories");
    let encrypted = crate::secure_storage::encrypt_private_key(secret).expect("encrypt the key");
    crate::config::update_config_section(
        |cfg| {
            cfg.wallet_encrypted = encrypted.ciphertext;
            cfg.wallet_nonce = encrypted.nonce;
        },
        false,
    )
    .expect("configure the trading wallet");
    crate::config::save_config(None).expect("write config.toml");
    crate::wallets::initialize()
        .await
        .expect("initialize the wallet manager");
    directory
}

/// Complete setup with `secret` and one validated endpoint, as the setup dialog does
/// after its validation.
async fn complete_with(address: &str, secret: &str) -> Response {
    let rpc_urls = vec![RPC.to_owned()];
    let validation_id = store_setup_validation(
        secret,
        &rpc_urls,
        ChainId::Solana,
        address.to_owned(),
        RpcSource::Request,
        vec![0],
    );
    complete_initialization(Json(CompleteInitializationRequest {
        validation_id,
        wallet_private_key: secret.to_owned(),
        rpc_urls,
    }))
    .await
}

async fn body_of(response: Response) -> Value {
    serde_json::from_slice(
        &to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("read response body"),
    )
    .expect("decode response body")
}

fn marker_exists() -> bool {
    crate::paths::get_wallet_replacement_marker_path().exists()
}

#[tokio::test]
async fn setup_with_another_key_stages_the_replacement_of_the_vault_wallet() {
    let (current, current_secret) = new_test_key();
    let _directory = vault_with(&current_secret).await;
    let (address, secret) = new_test_key();

    let response = complete_with(&address, &secret).await;

    assert!(response.status().is_success(), "{}", response.status());
    let body = body_of(response).await;
    assert_eq!(body["wallet_address"], address.as_str());
    assert_eq!(body["restart_required"], true);
    assert!(marker_exists(), "the replacement of {current} is staged");
    let marker: Value = serde_json::from_slice(
        &std::fs::read(crate::paths::get_wallet_replacement_marker_path()).expect("read marker"),
    )
    .expect("decode marker");
    assert_eq!(marker["previous_address"], current.as_str());
    assert_eq!(marker["new_address"], address.as_str());
    assert_eq!(
        crate::config::with_config(|cfg| cfg.chains.rpc(ChainId::Solana).urls.clone()),
        vec![RPC.to_owned()]
    );
}

#[tokio::test]
async fn setup_with_the_vault_wallet_key_stages_nothing() {
    let (address, secret) = new_test_key();
    let _directory = vault_with(&secret).await;

    let response = complete_with(&address, &secret).await;

    assert!(response.status().is_success(), "{}", response.status());
    assert!(!marker_exists(), "the same wallet needs no replacement");
}

#[tokio::test]
async fn a_staged_replacement_restarts_even_when_the_setup_settings_cannot_be_saved() {
    let (_, current_secret) = new_test_key();
    let _directory = vault_with(&current_secret).await;
    // Without config.toml the stage leaves the settings alone; the in-memory config
    // then no longer validates, so saving the setup settings fails after the stage.
    std::fs::remove_file(crate::paths::get_config_path()).expect("remove config.toml");
    crate::config::utils::CONFIG
        .get()
        .expect("config loaded")
        .write()
        .expect("config lock")
        .trader
        .max_open_positions = 0;
    let (address, secret) = new_test_key();

    let response = complete_with(&address, &secret).await;

    let status = response.status();
    let body = body_of(response).await;
    assert!(!status.is_success(), "{status}");
    assert_eq!(body["error"]["code"], "CONFIG_ERROR");
    assert_eq!(
        body["error"]["text"]["id"],
        "errors-initialization-save-failed-replacement-staged"
    );
    assert!(marker_exists(), "the replacement stays staged");
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(
        crate::global::is_restart_requested(),
        "the restart that applies the replacement is scheduled"
    );
}
