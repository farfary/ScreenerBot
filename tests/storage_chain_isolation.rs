//! Falsifies chain-scoping (and, for the wallet monitor, wallet-scoping) across the
//! SQLite stores that gained a `chain_id` column in the multi-chain-preparation
//! refactor. `ChainId` has exactly one variant (`Solana`), so no Rust-level
//! comparison can prove the filters work — each case here writes a real row through
//! the store's public API, then inserts a conceptually-foreign row directly by SQL
//! with `chain_id = 'ethereum'` (same primary-key shape, different chain), then
//! exercises the public read surface and asserts the foreign row never comes back.
//!
//! Several stores resolve their database file from `crate::paths::get_*_db_path()`,
//! which memoises its base directory in a process-wide `LazyLock` fed by the
//! `SCREENERBOT_DATA_DIR` env var. Plain `cargo test` runs one binary's tests
//! concurrently on threads, so every case that needs that path resolution goes
//! through `shared_data_dir()`, a `OnceLock` that runs `common::isolated_env()`
//! exactly once no matter how many threads race it — every store after that shares
//! one temp base directory but writes to its own distinctly-named db file, so there
//! is no cross-store collision.

mod common;

use rusqlite::Connection;
use screenerbot::chains::ChainId;

/// Run `common::isolated_env()` exactly once for the whole process, however many
/// threads race to call this first. Every paths-module-dependent store test calls
/// this before touching its database.
fn shared_data_dir() -> &'static tempfile::TempDir {
    static DIR: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let dir = common::isolated_env();
        screenerbot::paths::ensure_all_directories().expect("create data directories");
        dir
    })
}

mod pools_store {
    use super::*;
    use screenerbot::pools::database::PoolsDatabase;

    /// A real blacklist row written through the public API must be the only one
    /// `list_blacklisted_pools` ever returns, even when a raw row for the SAME
    /// `pool_id` exists under `chain_id = 'ethereum'` — and the real write itself
    /// must have landed as `chain_id = 'solana'` in the underlying table.
    #[tokio::test]
    async fn ignores_raw_rows_from_another_chain() {
        shared_data_dir();
        let mut db = PoolsDatabase::new(ChainId::Solana);
        db.initialize().await.expect("initialize pools database");

        db.add_pool_to_blacklist("PoolReal", "reason-real", Some("MintReal"), None)
            .await
            .expect("add real pool to blacklist");

        // Conceptually-foreign row: same pool_id, different chain.
        let path = screenerbot::paths::get_pools_db_path();
        let conn = Connection::open(&path).expect("open pools db directly");
        conn.execute(
            "INSERT INTO blacklist_pools (chain_id, pool_id, reason, token_mint, error_count, first_failed_at, last_failed_at, added_at)
             VALUES ('ethereum', 'PoolReal', 'foreign', 'MintForeign', 1, 1, 1, 1)",
            [],
        )
        .expect("insert conceptual foreign-chain blacklist row");

        let pools = db
            .list_blacklisted_pools(None)
            .await
            .expect("list solana-scoped blacklist");
        assert_eq!(
            pools.len(),
            1,
            "the ethereum row for the same pool_id must not appear in the Solana-scoped list"
        );
        assert_eq!(pools[0].reason, "reason-real");
        assert!(pools.iter().all(|p| p.chain_id == ChainId::Solana));

        let solana_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM blacklist_pools WHERE chain_id = 'solana' AND pool_id = 'PoolReal'",
                [],
                |row| row.get(0),
            )
            .expect("count solana-scoped rows");
        assert_eq!(
            solana_rows, 1,
            "the public write must have landed as chain_id = 'solana'"
        );
    }
}

mod tokens_store {
    use super::*;
    use screenerbot::tokens::database::TokenDatabase;

    /// A real `tokens` row and a real `blacklist` row, each written through the
    /// public API, must be the only ones the reads return, even when raw rows for
    /// the SAME mint exist under `chain_id = 'ethereum'` — and the real writes
    /// themselves must have landed as `chain_id = 'solana'`.
    #[test]
    fn ignores_raw_rows_from_another_chain() {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path().join("tokens.db");
        let db = TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana)
            .expect("open tokens database");

        db.upsert_token("Mint1", Some("REAL"), Some("Real Token"), Some(9))
            .expect("upsert real token");
        db.add_to_blacklist("Mint1", "reason-real", "source-real")
            .expect("add real blacklist entry");

        // Conceptually-foreign rows: same mint, different chain.
        let conn = Connection::open(&path).expect("open tokens db directly");
        conn.execute(
            "INSERT INTO tokens (chain_id, mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at)
             VALUES ('ethereum', 'Mint1', 'FAKE', 'Fake Token', 18, 1, 1, 1)",
            [],
        )
        .expect("insert conceptual foreign-chain token row");
        conn.execute(
            "INSERT INTO blacklist (chain_id, mint, reason, source, added_at)
             VALUES ('ethereum', 'Mint1', 'reason-foreign', 'source-foreign', 1)",
            [],
        )
        .expect("insert conceptual foreign-chain blacklist row");

        let token = db
            .get_token("Mint1")
            .expect("read solana-scoped token")
            .expect("solana row must exist");
        assert_eq!(token.symbol.as_deref(), Some("REAL"));

        let tokens = db.list_tokens(10).expect("list solana-scoped tokens");
        assert_eq!(
            tokens.len(),
            1,
            "the ethereum row for the same mint must not appear in the Solana-scoped list"
        );

        let blacklist = db
            .list_blacklisted_tokens()
            .expect("list solana-scoped blacklist");
        assert_eq!(
            blacklist.len(),
            1,
            "the ethereum blacklist row for the same mint must not appear in the Solana-scoped list"
        );
        assert_eq!(blacklist[0].reason, "reason-real");

        let solana_token_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens WHERE chain_id = 'solana' AND mint = 'Mint1'",
                [],
                |row| row.get(0),
            )
            .expect("count solana-scoped token rows");
        assert_eq!(
            solana_token_rows, 1,
            "the public write must have landed as chain_id = 'solana'"
        );
    }
}

mod positions_store {
    use super::*;
    use screenerbot::positions::database::PositionsDatabase;

    /// A real position written through the public API must be the only one
    /// `get_open_positions` ever returns, even when a raw row for the SAME wallet
    /// and mint exists under `chain_id = 'ethereum'` — and the real write itself
    /// must have landed as `chain_id = 'solana'`.
    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_raw_rows_from_another_chain() {
        shared_data_dir();
        // Serialises against every other test in this binary that mutates the global
        // CONFIG wallet fields (`configure_own_wallet`) — without this guard, two
        // stores' tests running concurrently on separate threads race over which
        // random keypair `subject.address()` resolves to at write time.
        let _cfg = common::config_guard();
        let wallet_address = common::configure_own_wallet();
        let db = PositionsDatabase::new(ChainId::Solana)
            .await
            .expect("open positions database");

        let position = common::test_position(0.001, 1.0);
        db.insert_position(&position)
            .await
            .expect("insert real position");

        // Conceptually-foreign row: same wallet + mint, different chain.
        let path = screenerbot::paths::get_positions_db_path();
        let conn = Connection::open(&path).expect("open positions db directly");
        conn.execute(
            "INSERT INTO positions (chain_id, wallet_address, mint, symbol, name, entry_price, entry_time, position_type, entry_size_sol, total_size_sol, price_highest, price_lowest)
             VALUES ('ethereum', ?1, ?2, 'FAKE', 'Fake', 0.002, '2026-01-01T00:00:00Z', 'buy', 2.0, 2.0, 0.002, 0.002)",
            rusqlite::params![wallet_address, common::TEST_MINT],
        )
        .expect("insert conceptual foreign-chain position row");

        let open = db
            .get_open_positions()
            .await
            .expect("read solana-scoped open positions");
        assert_eq!(
            open.len(),
            1,
            "the ethereum row for the same wallet+mint must not appear in the Solana-scoped list"
        );
        assert_eq!(open[0].total_size_sol, 1.0);

        let solana_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM positions WHERE chain_id = 'solana' AND wallet_address = ?1 AND mint = ?2",
                rusqlite::params![wallet_address, common::TEST_MINT],
                |row| row.get(0),
            )
            .expect("count solana-scoped rows");
        assert_eq!(
            solana_rows, 1,
            "the public write must have landed as chain_id = 'solana'"
        );
    }
}

mod transactions_store {
    use super::*;
    use screenerbot::transactions::{Subject, TransactionDatabase};

    /// A real known-signature row written through the public API must be the only
    /// one counted for the Solana subject, even when a raw row for the SAME
    /// wallet + signature exists under `chain_id = 'ethereum'` — and the real write
    /// itself must have landed as `chain_id = 'solana'`.
    #[tokio::test(flavor = "multi_thread")]
    async fn ignores_raw_rows_from_another_chain() {
        shared_data_dir();
        // Serialises against every other test in this binary that mutates the global
        // CONFIG wallet fields (`configure_own_wallet`) — without this guard, two
        // stores' tests running concurrently on separate threads race over which
        // random keypair `subject.address()` resolves to at write time.
        let _cfg = common::config_guard();
        let wallet_address = common::configure_own_wallet();
        let db = TransactionDatabase::new(ChainId::Solana)
            .await
            .expect("open transactions database");
        let subject = Subject::own().expect("resolve own-wallet subject");

        db.add_known_signature(subject.clone(), "sig-real")
            .await
            .expect("add real known signature");

        // Conceptually-foreign row: same wallet + signature, different chain.
        let path = screenerbot::paths::get_transactions_db_path();
        let conn = Connection::open(&path).expect("open transactions db directly");
        conn.execute(
            "INSERT INTO known_signatures (chain_id, signature, wallet_address) VALUES ('ethereum', 'sig-real', ?1)",
            rusqlite::params![wallet_address],
        )
        .expect("insert conceptual foreign-chain known-signature row");

        let count = db
            .get_known_signatures_count(subject.clone())
            .await
            .expect("count solana-scoped known signatures");
        assert_eq!(
            count, 1,
            "the ethereum row for the same wallet+signature must not appear in the Solana-scoped count"
        );
        assert!(db
            .is_signature_known(subject, "sig-real")
            .await
            .expect("check solana-scoped known signature"));

        let solana_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM known_signatures WHERE chain_id = 'solana' AND wallet_address = ?1 AND signature = 'sig-real'",
                rusqlite::params![wallet_address],
                |row| row.get(0),
            )
            .expect("count solana-scoped rows");
        assert_eq!(
            solana_rows, 1,
            "the public write must have landed as chain_id = 'solana'"
        );
    }
}

// SKIPPED: ohlcvs — `OhlcvDatabase` (and its only real write path,
// `insert_candles_batch`) is never re-exported publicly; the truly public API
// (`screenerbot::ohlcvs::service_api`) only writes candles via the monitor's
// network-touching backfill/gap-fill path (`src/ohlcvs/monitor.rs`,
// `src/ohlcvs/gaps.rs`), so there is no offline public write surface within budget.

// SKIPPED: wallets balance monitor — `WalletDatabase` is a private module type
// (`mod database;` in `src/wallets/balance_monitor/mod.rs` re-exports only
// `get_wallet_service_metrics`/`is_wallet_database_ready`), and the only public
// write path, `force_wallet_snapshot`, calls live RPC balance/token-account
// fetches (`src/wallets/balance_monitor/service.rs` -> `collect_wallet_snapshot`),
// so there is no offline public write surface within budget.
