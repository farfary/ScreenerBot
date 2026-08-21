//! Wallet-monitor database lifecycle: readiness flips exactly once, from
//! false to true, and never earlier than a real initialized database.
//!
//! Own test file (own process) because `crate::paths` resolves
//! `SCREENERBOT_DATA_DIR` into a process-wide `LazyLock` on first access, and
//! `WALLET_DB_READY`/`GLOBAL_WALLET_DB` are process-wide statics that can only
//! go from not-ready to ready once per process — see
//! `src/wallets/balance_monitor/database.rs`.

mod common;

use screenerbot::wallet::{
    get_current_wallet_status, initialize_wallet_database, is_wallet_database_ready,
};

#[tokio::test]
async fn database_readiness_flips_once_and_initialization_is_idempotent() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();
    common::configure_own_wallet();

    assert!(
        !is_wallet_database_ready(),
        "must not report ready before initialize_wallet_database() has run"
    );

    // Pre-readiness queries surface the (expected) not-initialized error rather
    // than silently returning data — the dashboard collector is the one place
    // allowed to swallow this, by checking readiness first (see
    // `crate::webserver::snapshot::collectors::collect_wallet_snapshot`).
    let pre_init = get_current_wallet_status().await;
    assert!(
        pre_init.is_err(),
        "querying before initialization must error, not silently succeed"
    );

    initialize_wallet_database()
        .await
        .expect("first initialization must succeed");

    assert!(
        is_wallet_database_ready(),
        "must report ready once initialization has completed"
    );

    // A fresh database has no snapshot yet, but the query itself must now
    // succeed (Ok(None)), never error, now that readiness is true.
    let post_init = get_current_wallet_status().await;
    assert!(
        matches!(post_init, Ok(None)),
        "post-initialization query on an empty database must be Ok(None), got {post_init:?}"
    );

    // Idempotent: a second call must not fail, error, or flip readiness back off.
    initialize_wallet_database()
        .await
        .expect("second initialization call must be idempotent, not fail");
    assert!(is_wallet_database_ready());
}
