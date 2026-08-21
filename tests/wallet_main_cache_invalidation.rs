//! The main-wallet keypair cache (`crate::chains::solana::accounts::signing`)
//! must track whichever wallet the wallets database currently calls "main" —
//! never keep signing with a stale key after `create_wallet` (set_as_main),
//! `import_wallet` (set_as_main), or `set_main_wallet` changes it.
//!
//! Own test file (own process): `crate::wallets::manager` and the main-
//! keypair cache in `crate::chains::solana::accounts::signing` are both
//! process-wide statics, so isolating them from any other DB-touching test
//! requires a dedicated binary, same as `tests/wallet_readiness.rs`.

mod common;

use screenerbot::chains::solana::accounts::main_keypair;
use screenerbot::chains::solana::solana_sdk::signature::Signer;
use screenerbot::wallets::{create_wallet, set_main_wallet, CreateWalletRequest};

#[tokio::test]
async fn main_keypair_cache_follows_set_main_wallet() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();

    screenerbot::wallets::initialize()
        .await
        .expect("wallet manager initialization must succeed");

    let wallet_a = create_wallet(CreateWalletRequest {
        name: "A".to_owned(),
        set_as_main: true,
        notes: None,
    })
    .await
    .expect("create wallet A as main");

    let key_a = main_keypair().await.expect("main keypair resolves to A");
    assert_eq!(
        key_a.pubkey().to_string(),
        wallet_a.address,
        "cache must resolve to the wallet just created as main"
    );

    let wallet_b = create_wallet(CreateWalletRequest {
        name: "B".to_owned(),
        set_as_main: false,
        notes: None,
    })
    .await
    .expect("create wallet B as secondary");

    // Still A: creating a non-main wallet must not disturb the cache.
    let still_a = main_keypair().await.expect("main keypair still resolves");
    assert_eq!(still_a.pubkey().to_string(), wallet_a.address);

    set_main_wallet(wallet_b.id)
        .await
        .expect("set wallet B as main");

    let key_b = main_keypair()
        .await
        .expect("main keypair resolves to B after set_main_wallet");
    assert_eq!(
        key_b.pubkey().to_string(),
        wallet_b.address,
        "cache must invalidate and re-resolve to the new main wallet, not keep signing as A"
    );
    assert_ne!(
        key_b.pubkey(),
        key_a.pubkey(),
        "the two wallets must be genuinely distinct keys"
    );
}
