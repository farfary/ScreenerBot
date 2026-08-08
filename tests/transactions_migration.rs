//! The §7.1 primary-key migration: `raw_transactions`, `processed_transactions`,
//! `known_signatures`, `pending_transactions` and `deferred_retries` move from
//! `signature TEXT PRIMARY KEY` to a composite `(signature, wallet_address)` key.
//!
//! Before this migration, one signature could hold exactly one subject's
//! perspective -- the moment a second wallet (a copy target, an alert-only watch)
//! is recorded, a transaction that mentions both silently overwrites the earlier
//! row and rewrites its `wallet_address`. This test builds a pre-migration
//! ("v4-shaped") database by hand, runs `TransactionDatabase::new()` against it (the
//! same public entry point the app uses) twice, and proves the migration is correct,
//! lossless and idempotent.
//!
//! Deliberately a SINGLE test function rather than several: `paths::
//! get_data_directory()` memoises its base directory in a process-wide `LazyLock`,
//! and plain `cargo test` runs one file's tests concurrently on threads (unlike
//! `cargo nextest`'s one-process-per-test), so a second test calling
//! `isolated_env()` here would race the first over which temp directory wins for
//! the rest of the process.
//!
//! `#[tokio::test(flavor = "multi_thread")]` is required: database init resolves the
//! own wallet address via `crate::utils::get_wallet_address()`, which bridges into a
//! blocking call via `tokio::task::block_in_place` -- that panics on the default
//! current-thread test runtime.

mod common;

use rusqlite::{params, Connection};
use solana_sdk::signature::{Keypair, Signer};

use screenerbot::transactions::{
    database::TransactionListFilters, Subject, Transaction, TransactionDatabase, TransactionStatus,
};

/// Configure a throwaway wallet keypair as the "own wallet" for this process, so
/// `Subject::own()` / `crate::utils::get_wallet_address()` resolve instead of erroring
/// with "wallet not configured". Returns the wallet's base58 address.
fn configure_own_wallet() -> String {
    let keypair = Keypair::new();
    let address = keypair.pubkey().to_string();
    let private_key = bs58::encode(keypair.to_bytes()).into_string();
    let encrypted = screenerbot::secure_storage::encrypt_private_key(&private_key)
        .expect("encrypt throwaway test keypair");

    common::set_config(|cfg| {
        cfg.wallet_encrypted = encrypted.ciphertext.clone();
        cfg.wallet_nonce = encrypted.nonce.clone();
    });

    address
}

/// Build a pre-migration ("v4-shaped") `transactions.db` at `path`: every
/// subject-scoped table keyed by `signature TEXT PRIMARY KEY` alone, exactly as
/// `schema.rs` declared them before the §7.1 migration, with `deferred_retries`
/// missing `wallet_address` entirely. Seeds one row per table for `own_wallet`, plus
/// one `raw_transactions` row with an EMPTY `wallet_address` to exercise the
/// backfill rule.
fn seed_v4_database(path: &std::path::Path, own_wallet: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create data directory for v4 fixture");
    }
    let conn = Connection::open(path).expect("open v4 fixture database");

    conn.execute_batch(
        "
        CREATE TABLE db_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE raw_transactions (
            signature TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            slot INTEGER,
            block_time INTEGER,
            timestamp TEXT NOT NULL,
            status TEXT NOT NULL,
            success BOOLEAN NOT NULL DEFAULT false,
            error_message TEXT,
            fee_lamports INTEGER,
            compute_units_consumed INTEGER,
            instructions_count INTEGER NOT NULL DEFAULT 0,
            accounts_count INTEGER NOT NULL DEFAULT 0,
            raw_transaction_data TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE processed_transactions (
            signature TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            transaction_type TEXT NOT NULL,
            direction TEXT NOT NULL,
            sol_balance_change TEXT,
            token_balance_changes TEXT,
            token_swap_info TEXT,
            swap_pnl_info TEXT,
            ata_operations TEXT,
            token_transfers TEXT,
            instruction_info TEXT,
            analysis_duration_ms INTEGER,
            cached_analysis TEXT,
            analysis_version INTEGER NOT NULL DEFAULT 2,
            fee_sol REAL NOT NULL DEFAULT 0,
            sol_delta REAL,
            processed_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (signature) REFERENCES raw_transactions(signature) ON DELETE CASCADE
        );

        CREATE TABLE known_signatures (
            signature TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'known',
            added_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE deferred_retries (
            signature TEXT PRIMARY KEY,
            next_retry_at TEXT NOT NULL,
            remaining_attempts INTEGER NOT NULL DEFAULT 3,
            current_delay_secs INTEGER NOT NULL DEFAULT 60,
            last_error TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE pending_transactions (
            signature TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            added_at TEXT NOT NULL DEFAULT (datetime('now')),
            last_checked_at TEXT,
            check_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE bootstrap_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            backfill_before_cursor TEXT,
            full_history_completed INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )
    .expect("create v4 fixture schema");

    conn.execute(
        "INSERT INTO db_metadata (key, value) VALUES ('schema_version', '4')",
        [],
    )
    .expect("seed v4 schema_version");

    // A normal own-wallet row, in every table.
    conn.execute(
        "INSERT INTO raw_transactions
            (signature, wallet_address, slot, timestamp, status, success, raw_transaction_data)
         VALUES ('SIG_OWN', ?1, 100, '2026-01-01T00:00:00Z', 'Finalized', 1, '{\"n\":1}')",
        params![own_wallet],
    )
    .expect("seed raw_transactions own row");
    conn.execute(
        "INSERT INTO processed_transactions
            (signature, wallet_address, transaction_type, direction, fee_sol, sol_delta)
         VALUES ('SIG_OWN', ?1, 'Transfer', 'Outgoing', 0.000005, -0.25)",
        params![own_wallet],
    )
    .expect("seed processed_transactions own row");
    conn.execute(
        "INSERT INTO known_signatures (signature, wallet_address) VALUES ('SIG_OWN', ?1)",
        params![own_wallet],
    )
    .expect("seed known_signatures own row");
    conn.execute(
        "INSERT INTO pending_transactions (signature, wallet_address) VALUES ('SIG_PENDING', ?1)",
        params![own_wallet],
    )
    .expect("seed pending_transactions own row");
    conn.execute(
        "INSERT INTO deferred_retries (signature, next_retry_at, last_error)
         VALUES ('SIG_RETRY', '2026-01-01T00:00:00Z', 'indexing delay')",
        [],
    )
    .expect("seed deferred_retries row (no wallet_address column pre-v5)");

    // A row with an EMPTY wallet_address -- the backfill rule must fill this with
    // the own wallet address rather than dropping the row.
    conn.execute(
        "INSERT INTO raw_transactions
            (signature, wallet_address, slot, timestamp, status, success)
         VALUES ('SIG_BLANK_WALLET', '', 101, '2026-01-01T00:01:00Z', 'Finalized', 1)",
        [],
    )
    .expect("seed raw_transactions row with blank wallet_address");
}

/// The primary key on `table` spans more than one column -- the same check the
/// migration itself uses (`PRAGMA table_info`'s `pk` field, 1-based position).
fn has_composite_key(conn: &Connection, table: &str) -> bool {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("inspect table schema");
    let pk_columns = stmt
        .query_map([], |row| row.get::<_, i64>(5))
        .expect("read table schema")
        .filter_map(|r| r.ok())
        .filter(|&pk| pk > 0)
        .count();
    pk_columns >= 2
}

#[tokio::test(flavor = "multi_thread")]
async fn migration_bumps_version_rebuilds_tables_and_is_idempotent() {
    let _cfg = common::config_guard();
    let _guard = common::isolated_env();
    let own_wallet = configure_own_wallet();

    let db_path = screenerbot::paths::get_transactions_db_path();
    seed_v4_database(&db_path, &own_wallet);

    // The public entry point the whole app uses -- exercises the exact migration
    // path a real upgrade takes, not a test-only shortcut.
    let db = TransactionDatabase::new()
        .await
        .expect("open + migrate v4 database");

    // 1. Version is 5.
    let conn = Connection::open(&db_path).expect("reopen migrated database");
    let stored_version: String = conn
        .query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .expect("read migrated schema_version");
    assert_eq!(stored_version, "5");

    // 2. The composite key exists on every migrated table.
    for table in [
        "raw_transactions",
        "processed_transactions",
        "known_signatures",
        "pending_transactions",
        "deferred_retries",
    ] {
        assert!(
            has_composite_key(&conn, table),
            "{table} was not rebuilt onto a composite (signature, wallet_address) key"
        );
    }

    // 3. Every original row survived with its wallet_address intact.
    let raw_wallet: String = conn
        .query_row(
            "SELECT wallet_address FROM raw_transactions WHERE signature = 'SIG_OWN'",
            [],
            |row| row.get(0),
        )
        .expect("SIG_OWN survived the migration in raw_transactions");
    assert_eq!(raw_wallet, own_wallet);

    let processed_wallet: String = conn
        .query_row(
            "SELECT wallet_address FROM processed_transactions WHERE signature = 'SIG_OWN'",
            [],
            |row| row.get(0),
        )
        .expect("SIG_OWN survived the migration in processed_transactions");
    assert_eq!(processed_wallet, own_wallet);

    let known_wallet: String = conn
        .query_row(
            "SELECT wallet_address FROM known_signatures WHERE signature = 'SIG_OWN'",
            [],
            |row| row.get(0),
        )
        .expect("SIG_OWN survived the migration in known_signatures");
    assert_eq!(known_wallet, own_wallet);

    let pending_wallet: String = conn
        .query_row(
            "SELECT wallet_address FROM pending_transactions WHERE signature = 'SIG_PENDING'",
            [],
            |row| row.get(0),
        )
        .expect("SIG_PENDING survived the migration in pending_transactions");
    assert_eq!(pending_wallet, own_wallet);

    // deferred_retries had no wallet_address at all pre-v5: every existing row must
    // be attributed to the own wallet, not dropped.
    let retry_wallet: String = conn
        .query_row(
            "SELECT wallet_address FROM deferred_retries WHERE signature = 'SIG_RETRY'",
            [],
            |row| row.get(0),
        )
        .expect("SIG_RETRY survived the migration in deferred_retries");
    assert_eq!(retry_wallet, own_wallet);

    // 4. A NULL/empty wallet_address is backfilled with the own wallet address, not
    // dropped.
    let blank_wallet: String = conn
        .query_row(
            "SELECT wallet_address FROM raw_transactions WHERE signature = 'SIG_BLANK_WALLET'",
            [],
            |row| row.get(0),
        )
        .expect("SIG_BLANK_WALLET survived the migration");
    assert_eq!(blank_wallet, own_wallet);

    // 5. Two subjects can now both hold the same signature -- the exact collision the
    // migration exists to fix. Exercised through the public API, not raw SQL, since
    // that is what a real caller (the watch service) will do.
    let target = Subject::from(solana_sdk::pubkey::Pubkey::new_unique());
    let own_subject = Subject::own().expect("own subject after wallet configured");

    db.add_known_signature(own_subject, "SIG_SHARED")
        .await
        .expect("own subject records the shared signature");
    db.add_known_signature(target, "SIG_SHARED")
        .await
        .expect("target subject records the SAME shared signature");

    assert!(
        db.is_signature_known(own_subject, "SIG_SHARED")
            .await
            .expect("check own subject"),
        "own subject's row must survive the second subject's insert"
    );
    assert!(
        db.is_signature_known(target, "SIG_SHARED")
            .await
            .expect("check target subject"),
        "target subject's row must exist independently of the own subject's"
    );

    // 6. Reporting and detail reads stay subject-scoped too. The same signature is
    // valid for both perspectives and must never leak one subject into the other.
    let mut own_view = Transaction::new("SIG_SUBJECT_READ".to_owned());
    own_view.status = TransactionStatus::Finalized;
    own_view.success = true;
    own_view.sol_balance_change = -1.0;
    let mut target_view = own_view.clone();
    target_view.sol_balance_change = 2.0;
    db.upsert_full_transaction(own_subject, &own_view)
        .await
        .expect("store own perspective");
    db.upsert_full_transaction(target, &target_view)
        .await
        .expect("store target perspective");

    let filters = TransactionListFilters {
        signature: Some("SIG_SUBJECT_READ".to_owned()),
        ..Default::default()
    };
    let target_rows = db
        .list_transactions_for_subject(target, &filters, None, 10)
        .await
        .expect("list target subject");
    assert_eq!(target_rows.items.len(), 1);
    assert_eq!(target_rows.items[0].sol_delta, 2.0);
    let own_detail = db
        .get_transaction_for_subject(own_subject, "SIG_SUBJECT_READ")
        .await
        .expect("read own detail")
        .expect("own detail exists");
    let target_detail = db
        .get_transaction_for_subject(target, "SIG_SUBJECT_READ")
        .await
        .expect("read target detail")
        .expect("target detail exists");
    assert_eq!(own_detail.sol_balance_change, -1.0);
    assert_eq!(target_detail.sol_balance_change, 2.0);

    // 7. Idempotent: opening the now-migrated database a second time is a clean
    // no-op -- no error, version stays 5, and nothing seeded above is lost (a second
    // destructive rebuild would have dropped it). Deliberately reuses this test's
    // single `isolated_env()` rather than a second test function: `paths::
    // get_data_directory()` memoises its base directory in a process-wide
    // `LazyLock`, so two tests in this file each calling `isolated_env()` would race
    // over which one's temp directory wins for the whole process under plain
    // `cargo test` (which runs a file's tests concurrently on threads, unlike
    // `cargo nextest`'s one-process-per-test).
    let db_again = TransactionDatabase::new()
        .await
        .expect("second open against an already-migrated database");

    let stored_version_again: String = conn
        .query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .expect("read schema_version after second open");
    assert_eq!(stored_version_again, "5");

    assert!(
        db_again
            .is_signature_known(own_subject, "SIG_OWN")
            .await
            .expect("check SIG_OWN after second open"),
        "a second migration pass must not lose data from the first"
    );
}

/// Exercise the same migration against a copy-on-write clone of the owner's real
/// database. Kept ignored because the fixture is machine-local and can be much
/// larger than the deterministic v4 fixture above; the helper copies the DB and WAL
/// sidecars before this test opens anything, so the live database is never locked or
/// modified.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a local transactions.db; runs only against a throwaway clone"]
async fn migration_preserves_the_real_transactions_database_shape_and_rows() {
    let _cfg = common::config_guard();
    let Some(_clone) = common::real_db_env() else {
        return;
    };
    let own_wallet = configure_own_wallet();
    let db_path = screenerbot::paths::get_transactions_db_path();

    let before = Connection::open(&db_path).expect("open cloned real transactions database");
    let counts_before: Vec<i64> = [
        "raw_transactions",
        "processed_transactions",
        "known_signatures",
        "pending_transactions",
        "deferred_retries",
    ]
    .iter()
    .map(|table| {
        before
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {table} before migration: {error}"))
    })
    .collect();
    drop(before);

    TransactionDatabase::new()
        .await
        .expect("migrate cloned real transactions database");

    let after = Connection::open(&db_path).expect("reopen migrated real clone");
    for (index, table) in [
        "raw_transactions",
        "processed_transactions",
        "known_signatures",
        "pending_transactions",
        "deferred_retries",
    ]
    .iter()
    .enumerate()
    {
        assert!(
            has_composite_key(&after, table),
            "{table} in the real clone did not receive the composite key"
        );
        let count_after: i64 = after
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {table} after migration: {error}"));
        assert_eq!(
            count_after, counts_before[index],
            "migration changed the real clone's {table} row count"
        );
    }

    let missing_wallets: i64 = after
        .query_row(
            "SELECT COUNT(*) FROM deferred_retries WHERE wallet_address IS NULL OR wallet_address = ''",
            [],
            |row| row.get(0),
        )
        .expect("count blank deferred-retry subjects");
    assert_eq!(missing_wallets, 0);

    let stored_version: String = after
        .query_row(
            "SELECT value FROM db_metadata WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .expect("read schema version from real clone");
    assert_eq!(stored_version, "5");

    // Keep the configured subject used for backfills observable in failure output.
    assert!(!own_wallet.is_empty());
}
