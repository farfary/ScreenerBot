//! Regression coverage for the wallet sync/async signing bridge
//! (`crate::chains::solana::accounts::signing`) and the wallet-monitor
//! read paths that must stay key-free.
//!
//! Two invariants under test:
//! - Reading an address, a dashboard snapshot, or monitor stats must NEVER
//!   decrypt the main wallet or populate `MAIN_KEYPAIR_CACHE` — only an
//!   actual signing call may do that.
//! - The synchronous `configured_address()`/`configured_keypair()` bridge
//!   must never hang: it either makes real progress via
//!   `tokio::task::block_in_place` on a multi-thread runtime, or returns a
//!   clear error on a current-thread runtime, where blocking synchronously
//!   could starve the only worker thread.
//!
//! Deliberately ONE test function: `crate::wallets::manager` and the main-
//! keypair cache in `crate::chains::solana::accounts::signing` are both
//! process-wide statics (same reasoning as
//! `tests/wallet_main_cache_invalidation.rs`), and `cargo test` runs a
//! file's tests concurrently on threads within one process, so a second
//! test here would race this one over that shared state. The current-thread
//! probe builds its OWN throwaway `current_thread` runtime on a plain
//! `std::thread` instead of relying on a second `#[tokio::test]` function,
//! so the whole file stays hermetic within a single async test.

mod common;

use std::time::Duration;

use screenerbot::chains::solana::accounts::{
    cached_main_wallet_id, configured_address, configured_address_async, configured_keypair,
};
use screenerbot::wallet::{
    get_current_wallet_status, get_recent_wallet_snapshots, get_wallet_monitor_stats,
    initialize_wallet_database,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wallet_monitor_reads_stay_key_free_and_the_sync_bridge_never_hangs() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();
    common::configure_own_wallet();

    assert!(
        cached_main_wallet_id().await.is_none(),
        "cache must start empty"
    );

    // Legacy-config fast path (before the multi-wallet DB initializes):
    // resolving the address must not populate MAIN_KEYPAIR_CACHE, which only
    // ever holds the multi-wallet database's main wallet.
    let address = configured_address_async()
        .await
        .expect("legacy address resolves");
    assert!(!address.is_empty());
    assert!(
        cached_main_wallet_id().await.is_none(),
        "an address-only read must not populate the keypair cache"
    );

    initialize_wallet_database()
        .await
        .expect("wallet-monitor database initializes");

    let _ = get_current_wallet_status()
        .await
        .expect("status query on an empty database succeeds");
    let _ = get_recent_wallet_snapshots(5)
        .await
        .expect("recent snapshots query succeeds");
    let _ = get_wallet_monitor_stats()
        .await
        .expect("monitor stats query succeeds");

    assert!(
        cached_main_wallet_id().await.is_none(),
        "dashboard/snapshot/stats reads must not populate the keypair cache"
    );

    // A current-thread runtime must make the sync bridge return a clear
    // error, never hang. Built on its own OS thread with its own throwaway
    // `current_thread` runtime so this probe stays independent of the outer
    // (multi-thread) test runtime.
    let current_thread_address = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build throwaway current-thread runtime");
        rt.block_on(async {
            tokio::task::spawn_blocking(configured_address)
                .await
                .expect("blocking task must not panic")
        })
    })
    .join()
    .expect("current-thread probe thread must not panic");
    assert!(
        current_thread_address.is_err(),
        "configured_address() must refuse to run synchronously on a current-thread \
         runtime rather than risk a deadlock"
    );

    let current_thread_keypair = std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build throwaway current-thread runtime");
        rt.block_on(async {
            tokio::task::spawn_blocking(configured_keypair)
                .await
                .expect("blocking task must not panic")
        })
    })
    .join()
    .expect("current-thread probe thread must not panic");
    assert!(
        current_thread_keypair.is_err(),
        "configured_keypair() must refuse to run synchronously on a current-thread \
         runtime rather than risk a deadlock"
    );

    // On a multi-thread runtime the bridge must make real progress even
    // under contention: more concurrent blocking calls than worker threads,
    // each bounded by a timeout that would fail the test on a real deadlock.
    let mut handles = Vec::new();
    for _ in 0..8 {
        handles.push(tokio::spawn(async {
            tokio::task::spawn_blocking(configured_address)
                .await
                .expect("blocking task must not panic")
        }));
    }
    for handle in handles {
        let result = tokio::time::timeout(Duration::from_secs(10), handle)
            .await
            .expect("sync bridge must make progress under contention, not deadlock")
            .expect("task must not panic");
        assert!(
            result.is_ok(),
            "sync bridge must resolve the address on a multi-thread runtime"
        );
    }
}
