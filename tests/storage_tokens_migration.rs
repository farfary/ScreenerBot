//! Tokens schema forward-migration coverage, modeled on
//! `storage_transactions_migration.rs`.
//!
//! A single test function: `paths::get_data_directory()` memoises its base
//! directory in a process-wide `LazyLock`, and plain `cargo test` runs one
//! file's tests concurrently.

mod common;

use rusqlite::{Connection, OptionalExtension};
use screenerbot::chains::ChainId;
use screenerbot::tokens::schema::{CREATE_TABLES, SCHEMA_VERSION};
use screenerbot::tokens::TokenDatabase;

/// Every `CREATE_TABLES` statement with `token_favorites.notes` stripped — a
/// chain-scoped database written before that currently-declared nullable column.
fn schema_missing_favorites_notes_column() -> Vec<String> {
    CREATE_TABLES
        .iter()
        .map(|statement| statement.replace("notes TEXT,\n", ""))
        .collect()
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .expect("inspect table schema");
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("read table schema")
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    columns.iter().any(|name| name == column)
}

fn foreign_key_check_is_clean(conn: &Connection) -> bool {
    conn.query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
        .optional()
        .expect("run foreign_key_check")
        .is_none()
}

fn table_count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0))
        .expect("count seeded rows")
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("read user_version")
}

fn seed_all_token_tables(conn: &Connection) {
    conn.execute(
        "INSERT INTO tokens (chain_id, mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at) \
         VALUES ('solana', 'TOKEN_MINT', 'TOK', 'Token', 9, 1, 1, 1)",
        [],
    )
    .expect("seed tokens");
    conn.execute(
        "INSERT INTO market_dexscreener (chain_id, mint, market_data_last_fetched_at, market_data_first_fetched_at) \
         VALUES ('solana', 'TOKEN_MINT', 1, 1)",
        [],
    )
    .expect("seed market_dexscreener");
    conn.execute(
        "INSERT INTO market_geckoterminal (chain_id, mint, market_data_last_fetched_at, market_data_first_fetched_at) \
         VALUES ('solana', 'TOKEN_MINT', 1, 1)",
        [],
    )
    .expect("seed market_geckoterminal");
    conn.execute(
        "INSERT INTO token_pools (chain_id, mint, pool_address, base_mint, quote_mint, is_sol_pair, pool_data_last_fetched_at, pool_data_first_seen_at) \
         VALUES ('solana', 'TOKEN_MINT', 'POOL', 'TOKEN_MINT', 'So11111111111111111111111111111111111111112', 1, 1, 1)",
        [],
    )
    .expect("seed token_pools");
    conn.execute(
        "INSERT INTO security_rugcheck (chain_id, mint, security_data_last_fetched_at, security_data_first_fetched_at) \
         VALUES ('solana', 'TOKEN_MINT', 1, 1)",
        [],
    )
    .expect("seed security_rugcheck");
    conn.execute(
        "INSERT INTO blacklist (chain_id, mint, reason, added_at) VALUES ('solana', 'TOKEN_MINT', 'test', 1)",
        [],
    )
    .expect("seed blacklist");
    conn.execute(
        "INSERT INTO update_tracking (chain_id, mint, priority) VALUES ('solana', 'TOKEN_MINT', 10)",
        [],
    )
    .expect("seed update_tracking");
    conn.execute(
        "INSERT INTO token_favorites (chain_id, mint, name, symbol, logo_url) \
         VALUES ('solana', 'FAV_MINT', 'Favorite Token', 'FAV', 'https://example.test/logo.png')",
        [],
    )
    .expect("seed token_favorites");
    conn.execute(
        "INSERT INTO rejection_history (chain_id, mint, reason, source, rejected_at) \
         VALUES ('solana', 'TOKEN_MINT', 'reason', 'filter', 1)",
        [],
    )
    .expect("seed rejection_history");
    conn.execute(
        "INSERT INTO rejection_stats (chain_id, bucket_hour, reason, source, rejection_count, unique_tokens, first_seen, last_seen) \
         VALUES ('solana', 0, 'reason', 'filter', 1, 1, 1, 1)",
        [],
    )
    .expect("seed rejection_stats");
    conn.execute(
        "INSERT INTO authority_reputation (chain_id, address, first_seen_at, last_updated_at) \
         VALUES ('solana', 'AUTH', 1, 1)",
        [],
    )
    .expect("seed authority_reputation");
}

fn assert_seeded_rows_survived(conn: &Connection) {
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM tokens WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM market_dexscreener WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM market_geckoterminal WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM token_pools WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM security_rugcheck WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM blacklist WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM update_tracking WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    let seeded_favorite: (String, String) = conn
        .query_row(
            "SELECT chain_id, mint FROM token_favorites WHERE mint = 'FAV_MINT'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("seeded favorite survived");
    assert_eq!(
        seeded_favorite,
        ("solana".to_owned(), "FAV_MINT".to_owned())
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM rejection_history WHERE mint = 'TOKEN_MINT'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM rejection_stats WHERE reason = 'reason'"
        ),
        1
    );
    assert_eq!(
        table_count(
            conn,
            "SELECT COUNT(*) FROM authority_reputation WHERE address = 'AUTH'"
        ),
        1
    );
}

#[test]
fn already_versioned_database_missing_an_additive_column_self_heals() {
    let _guard = common::isolated_env();
    let dir = tempfile::tempdir().expect("create fixture directory");
    let db_path = dir.path().join("tokens.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    {
        let conn = Connection::open(&db_path).expect("create already-versioned fixture");
        for statement in schema_missing_favorites_notes_column() {
            conn.execute(&statement, [])
                .expect("create chain-scoped table without the notes column");
        }
        assert!(
            !has_column(&conn, "token_favorites", "notes"),
            "fixture must start without the additive column"
        );
        seed_all_token_tables(&conn);
        // Already versioned at the current schema version, with a required
        // additive column absent — the case user_version alone cannot heal.
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .expect("stamp fixture at the current schema version");
    }

    let db = TokenDatabase::new(&db_path_str, ChainId::Solana).expect("open + migrate fixture");
    drop(db);

    let conn = Connection::open(&db_path).expect("reopen migrated database");
    assert!(
        has_column(&conn, "token_favorites", "notes"),
        "additive column must be restored"
    );
    assert_seeded_rows_survived(&conn);
    assert_eq!(user_version(&conn), SCHEMA_VERSION);
    assert!(
        foreign_key_check_is_clean(&conn),
        "foreign_key_check must be clean after the additive-column migration"
    );
    drop(conn);

    let db_again =
        TokenDatabase::new(&db_path_str, ChainId::Solana).expect("second open is a clean no-op");
    drop(db_again);

    let conn = Connection::open(&db_path).expect("reopen after second migration pass");
    assert!(has_column(&conn, "token_favorites", "notes"));
    assert_seeded_rows_survived(&conn);
    assert_eq!(user_version(&conn), SCHEMA_VERSION);
    assert!(foreign_key_check_is_clean(&conn));
}
