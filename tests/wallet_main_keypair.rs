// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The main-wallet keypair (`crate::chains::solana::accounts::signing`) is the
//! trading wallet setup configured: the wallet store imports the configured key
//! as its main wallet, and every signer resolves to that wallet.
//!
//! It runs in a process of its own (nextest isolates every test): `crate::wallets::manager` and the main-
//! keypair cache in `crate::chains::solana::accounts::signing` are both
//! process-wide statics, so isolating them from any other DB-touching test
//! requires a process of its own, same as `tests/wallet_readiness.rs`.

use crate::common;

use screenerbot::chains::solana::accounts::main_keypair;
use screenerbot::chains::solana::solana_sdk::signature::Signer;

#[tokio::test]
async fn main_keypair_is_the_configured_trading_wallet() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();
    let address = common::configure_trading_wallet();

    screenerbot::wallets::initialize()
        .await
        .expect("wallet manager initialization must succeed");

    let main = screenerbot::wallets::get_main_wallet()
        .await
        .expect("read the main wallet")
        .expect("the configured wallet is imported as main");
    assert_eq!(main.address, address);

    let key = main_keypair().await.expect("main keypair resolves");
    assert_eq!(
        key.pubkey().to_string(),
        address,
        "the signer is the configured trading wallet"
    );
    let again = main_keypair().await.expect("cached main keypair resolves");
    assert_eq!(
        again.pubkey(),
        key.pubkey(),
        "the cache keeps the same wallet"
    );
}
