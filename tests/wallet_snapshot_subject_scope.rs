// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The wallet-monitor database must scope every snapshot/metrics read and
//! mutation to its stable `subject` (`WalletDatabase::subject` in
//! `src/wallets/balance_monitor/database.rs`), never merely to
//! `ChainId::Solana` -- two wallet addresses sharing a chain must never mix
//! in a snapshot list, a monitor-stats row, or a worth-at-time lookup.
//!
//! It runs in a process of its own (nextest isolates every test): the wallets manager, the main-keypair cache
//! and the wallet-monitor database are all process-wide statics, same
//! reasoning as `tests/wallet_readiness.rs`.

use crate::common;

use chrono::Utc;
use rusqlite::{params, Connection};

use screenerbot::wallet::{
    get_recent_wallet_snapshots, get_wallet_monitor_stats, get_worth_at_time,
    initialize_wallet_database,
};

/// Insert a synthetic `wallet_snapshots` row directly against the real
/// wallet-monitor database file, bypassing the (RPC-backed) public
/// collection API entirely -- the point is to prove the READ side scopes
/// correctly even when a row for another subject exists on the same chain,
/// not to exercise collection itself.
fn seed_snapshot(wallet_address: &str, sol_balance: f64) {
    let conn = Connection::open(screenerbot::paths::get_wallet_db_path())
        .expect("open wallet-monitor db file directly for seeding");
    conn.execute(
        "INSERT INTO wallet_snapshots (
            chain_id, wallet_address, snapshot_time, native_balance, native_balance_raw,
            total_equity_native, total_tokens_count, total_nfts_count
        ) VALUES ('solana', ?1, ?2, ?3, ?4, ?3, 0, 0)",
        params![
            wallet_address,
            Utc::now().to_rfc3339(),
            sol_balance,
            (sol_balance * 1_000_000_000.0) as i64
        ],
    )
    .expect("insert synthetic wallet snapshot");
}

#[tokio::test]
async fn wallet_monitor_reads_stay_scoped_to_the_active_subject() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();
    let wallet_a = common::configure_trading_wallet();

    screenerbot::wallets::initialize()
        .await
        .expect("wallet manager initializes with the configured wallet A as main");

    initialize_wallet_database()
        .await
        .expect("wallet-monitor database initializes bound to A");
    let db_path = screenerbot::paths::get_wallet_db_path();
    assert!(db_path.exists(), "wallet-monitor database file must exist");

    // Two subjects on the same chain: A (the active one) and an unrelated
    // address that was never set main. Only A's row may ever surface.
    let other_subject = "Other1111111111111111111111111111111111111";
    seed_snapshot(&wallet_a, 1.0);
    seed_snapshot(other_subject, 99.0);

    let recent = get_recent_wallet_snapshots(10)
        .await
        .expect("recent snapshots query succeeds");
    assert_eq!(
        recent.len(),
        1,
        "must see only the active subject's snapshot, not OTHER's"
    );
    assert_eq!(recent[0].wallet_address, wallet_a);
    assert_eq!(recent[0].native_balance, 1.0);

    let stats = get_wallet_monitor_stats()
        .await
        .expect("monitor stats query succeeds");
    assert_eq!(
        stats.total_snapshots, 1,
        "monitor stats must count only the active subject's rows"
    );
    assert_eq!(stats.wallet_address, wallet_a);

    let balance = get_worth_at_time(Utc::now())
        .await
        .expect("worth-at-time query succeeds")
        .map(|sample| sample.equity_native);
    assert_eq!(
        balance,
        Some(1.0),
        "worth-at-time must read A's row, never OTHER's 99.0"
    );
}
