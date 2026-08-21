//! The wallet-monitor database must scope every snapshot/metrics read and
//! mutation to its stable `subject` (`WalletDatabase::subject` in
//! `src/wallets/balance_monitor/database.rs`), never merely to
//! `ChainId::Solana` -- two wallet addresses sharing a chain must never mix
//! in a snapshot list, a monitor-stats row, or a balance-at-time lookup. And
//! switching the main wallet must rebind that subject in place, without
//! reopening a separate wallet-monitor database.
//!
//! Own test file (own process): the wallets manager, the main-keypair cache
//! and the wallet-monitor database are all process-wide statics, same
//! reasoning as `tests/wallet_readiness.rs`.

mod common;

use chrono::Utc;
use rusqlite::{params, Connection};

use screenerbot::wallet::{
    get_balance_at_time, get_recent_wallet_snapshots, get_wallet_monitor_stats,
    initialize_wallet_database,
};
use screenerbot::wallets::{create_wallet, CreateWalletRequest};

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
            chain_id, wallet_address, snapshot_time, sol_balance, sol_balance_lamports,
            total_equity_sol, total_tokens_count, total_nfts_count
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
async fn wallet_monitor_reads_stay_scoped_to_the_active_subject_and_rebind_on_main_wallet_switch() {
    let dir = tempfile::tempdir().expect("temp dir");
    std::env::set_var("SCREENERBOT_DATA_DIR", dir.path());
    std::fs::create_dir_all(dir.path().join("data")).expect("create data dir");
    common::ensure_config();

    screenerbot::wallets::initialize()
        .await
        .expect("wallet manager initializes");
    let wallet_a = create_wallet(CreateWalletRequest {
        name: "A".to_owned(),
        set_as_main: true,
        notes: None,
    })
    .await
    .expect("create wallet A as main");

    initialize_wallet_database()
        .await
        .expect("wallet-monitor database initializes bound to A");
    let db_path = screenerbot::paths::get_wallet_db_path();
    assert!(db_path.exists(), "wallet-monitor database file must exist");

    // Two subjects on the same chain: A (the active one) and an unrelated
    // address that was never set main. Only A's row may ever surface.
    let other_subject = "Other1111111111111111111111111111111111111";
    seed_snapshot(&wallet_a.address, 1.0);
    seed_snapshot(other_subject, 99.0);

    let recent = get_recent_wallet_snapshots(10)
        .await
        .expect("recent snapshots query succeeds");
    assert_eq!(
        recent.len(),
        1,
        "must see only the active subject's snapshot, not OTHER's"
    );
    assert_eq!(recent[0].wallet_address, wallet_a.address);
    assert_eq!(recent[0].sol_balance, 1.0);

    let stats = get_wallet_monitor_stats()
        .await
        .expect("monitor stats query succeeds");
    assert_eq!(
        stats.total_snapshots, 1,
        "monitor stats must count only the active subject's rows"
    );
    assert_eq!(stats.wallet_address, wallet_a.address);

    let balance = get_balance_at_time(Utc::now())
        .await
        .expect("balance-at-time query succeeds");
    assert_eq!(
        balance,
        Some(1.0),
        "balance-at-time must read A's row, never OTHER's 99.0"
    );

    // Switching the main wallet must rebind the monitor's subject to B --
    // still the SAME database file, never a second one.
    let wallet_b = create_wallet(CreateWalletRequest {
        name: "B".to_owned(),
        set_as_main: true,
        notes: None,
    })
    .await
    .expect("create wallet B as main");
    assert_eq!(
        screenerbot::paths::get_wallet_db_path(),
        db_path,
        "the wallet-monitor database path must never change on a main-wallet switch"
    );

    seed_snapshot(&wallet_b.address, 2.0);

    let recent_after_switch = get_recent_wallet_snapshots(10)
        .await
        .expect("recent snapshots query succeeds after switch");
    assert_eq!(
        recent_after_switch.len(),
        1,
        "after the subject rebinds, only B's snapshot may surface -- not A's or OTHER's"
    );
    assert_eq!(recent_after_switch[0].wallet_address, wallet_b.address);
    assert_eq!(recent_after_switch[0].sol_balance, 2.0);

    let stats_after_switch = get_wallet_monitor_stats()
        .await
        .expect("monitor stats query succeeds after switch");
    assert_eq!(stats_after_switch.wallet_address, wallet_b.address);
    assert_eq!(stats_after_switch.total_snapshots, 1);
}
