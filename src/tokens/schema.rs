//! Database schema for tokens system
//! Versioned, transactional SQLite schema for the chain-scoped token store.
//!
//! TIMESTAMP NAMING CONVENTION: {what}_{when}_{action}_at
//! - {what}: Specific data type (market_data, security_data, metadata, pool_price, etc.)
//! - {when}: last / first / blockchain
//! - {action}: fetched / calculated / updated / created / discovered
//! - _at: Suffix for all timestamps (consistent)

use super::migrations;
use crate::database;
use rusqlite::{Connection, OptionalExtension};

/// Current tokens.db schema version recorded in `PRAGMA user_version`.
/// Additive column repair inspects on-disk columns and does not trust this
/// number alone: a database already stamped current still receives a required
/// column that is missing from the live table.
pub const SCHEMA_VERSION: i64 = 2;

/// All CREATE TABLE statements
pub const CREATE_TABLES: &[&str] = &[
    // Core token metadata
    r#"
    CREATE TABLE IF NOT EXISTS tokens (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        symbol TEXT,
        name TEXT,
        decimals INTEGER,
        first_discovered_at INTEGER NOT NULL,
        blockchain_created_at INTEGER,
        metadata_last_fetched_at INTEGER NOT NULL,
        decimals_last_fetched_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, mint)
    )
    "#,
    // DexScreener market data (per token, per source)
    r#"
    CREATE TABLE IF NOT EXISTS market_dexscreener (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        price_usd REAL,
        price_sol REAL,
        price_native TEXT,
        price_change_5m REAL,
        price_change_1h REAL,
        price_change_6h REAL,
        price_change_24h REAL,
        market_cap REAL,
        fdv REAL,
        liquidity_usd REAL,
        volume_5m REAL,
        volume_1h REAL,
        volume_6h REAL,
        volume_24h REAL,
        txns_5m_buys INTEGER,
        txns_5m_sells INTEGER,
        txns_1h_buys INTEGER,
        txns_1h_sells INTEGER,
        txns_6h_buys INTEGER,
        txns_6h_sells INTEGER,
        txns_24h_buys INTEGER,
        txns_24h_sells INTEGER,
        pair_address TEXT,
        provider_chain_id TEXT,
        dex_id TEXT,
        url TEXT,
        pair_blockchain_created_at INTEGER,
        image_url TEXT,
        header_image_url TEXT,
        market_data_last_fetched_at INTEGER NOT NULL,
        market_data_first_fetched_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, mint),
        FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint) ON DELETE RESTRICT
    )
    "#,
    // GeckoTerminal market data (per token, per source)
    r#"
    CREATE TABLE IF NOT EXISTS market_geckoterminal (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        price_usd REAL,
        price_sol REAL,
        price_native TEXT,
        price_change_5m REAL,
        price_change_1h REAL,
        price_change_6h REAL,
        price_change_24h REAL,
        market_cap REAL,
        fdv REAL,
        liquidity_usd REAL,
        volume_5m REAL,
        volume_1h REAL,
        volume_6h REAL,
        volume_24h REAL,
        pool_count INTEGER,
        top_pool_address TEXT,
        reserve_in_usd REAL,
        image_url TEXT,
        market_data_last_fetched_at INTEGER NOT NULL,
        market_data_first_fetched_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, mint),
        FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint) ON DELETE RESTRICT
    )
    "#,
    // Aggregated token pool data (multi-source, per pool)
    r#"
    CREATE TABLE IF NOT EXISTS token_pools (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        pool_address TEXT NOT NULL,
        dex TEXT,
        base_mint TEXT NOT NULL,
        quote_mint TEXT NOT NULL,
        is_sol_pair INTEGER NOT NULL,
        liquidity_usd REAL,
        liquidity_token REAL,
        liquidity_sol REAL,
        volume_h24 REAL,
        price_usd REAL,
        price_sol REAL,
        price_native TEXT,
        sources_json TEXT,
        pool_data_last_fetched_at INTEGER NOT NULL,
        pool_data_first_seen_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, mint, pool_address),
        FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint) ON DELETE CASCADE
    )
    "#,
    // Rugcheck security data (per token)
    r#"
    CREATE TABLE IF NOT EXISTS security_rugcheck (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        token_type TEXT,
        token_decimals INTEGER,
        score INTEGER,
        score_normalised INTEGER,
        score_description TEXT,
        mint_authority TEXT,
        freeze_authority TEXT,
        update_authority TEXT,
        is_mutable INTEGER,
        top_10_holders_pct REAL,
        total_supply TEXT,
        total_holders INTEGER,
        total_lp_providers INTEGER,
        graph_insiders_detected INTEGER,
        total_market_liquidity REAL,
        total_stable_liquidity REAL,
        creator_balance_pct REAL,
        transfer_fee_pct REAL,
        transfer_fee_max_amount INTEGER,
        transfer_fee_authority TEXT,
        rugged INTEGER,
        risks TEXT,
        top_holders TEXT,
        markets TEXT,
        security_data_last_fetched_at INTEGER NOT NULL,
        security_data_first_fetched_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, mint),
        FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint) ON DELETE RESTRICT
    )
    "#,
    // Blacklist
    r#"
    CREATE TABLE IF NOT EXISTS blacklist (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        reason TEXT NOT NULL,
        source TEXT,
        added_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, mint)
    )
    "#,
    // Update tracking and priority
    r#"
    CREATE TABLE IF NOT EXISTS update_tracking (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        priority INTEGER NOT NULL DEFAULT 10,
        market_data_last_updated_at INTEGER,
        market_data_update_count INTEGER DEFAULT 0,
        security_data_last_updated_at INTEGER,
        security_data_update_count INTEGER DEFAULT 0,
        metadata_last_updated_at INTEGER,
        decimals_last_updated_at INTEGER,
        pool_price_last_calculated_at INTEGER,
        pool_price_last_used_pool_address TEXT,
        last_error TEXT,
        last_error_at INTEGER,
        market_error_count INTEGER DEFAULT 0,
        security_error_count INTEGER DEFAULT 0,
        last_security_error TEXT,
        last_security_error_at INTEGER,
        security_error_type TEXT,
        market_error_type TEXT,
        last_rejection_reason TEXT,
        last_rejection_source TEXT,
        last_rejection_at INTEGER,
        PRIMARY KEY (chain_id, mint),
        FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint) ON DELETE RESTRICT
    )
    "#,
    // Token favorites (user-saved tokens)
    r#"
    CREATE TABLE IF NOT EXISTS token_favorites (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        name TEXT,
        symbol TEXT,
        logo_url TEXT,
        notes TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (chain_id, mint)
    )
    "#,
    // Rejection history table for time-range analytics
    r#"
    CREATE TABLE IF NOT EXISTS rejection_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        chain_id TEXT NOT NULL DEFAULT 'solana',
        mint TEXT NOT NULL,
        reason TEXT NOT NULL,
        source TEXT NOT NULL,
        rejected_at INTEGER NOT NULL
    )
    "#,
    // Aggregated rejection stats table (hourly buckets) - replaces per-event logging
    r#"
    CREATE TABLE IF NOT EXISTS rejection_stats (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        bucket_hour INTEGER NOT NULL,
        reason TEXT NOT NULL,
        source TEXT NOT NULL,
        rejection_count INTEGER NOT NULL DEFAULT 0,
        unique_tokens INTEGER NOT NULL DEFAULT 0,
        first_seen INTEGER NOT NULL,
        last_seen INTEGER NOT NULL,
        PRIMARY KEY (chain_id, bucket_hour, reason, source)
    )
    "#,
    // Authority reputation — auto-growing scam authority detection
    // Populated by background discovery task from filtering results
    r#"
    CREATE TABLE IF NOT EXISTS authority_reputation (
        chain_id TEXT NOT NULL DEFAULT 'solana',
        address TEXT NOT NULL,
        authority_type TEXT NOT NULL DEFAULT 'unknown',
        total_token_count INTEGER NOT NULL DEFAULT 0,
        flagged_token_count INTEGER NOT NULL DEFAULT 0,
        confidence REAL NOT NULL DEFAULT 0.0,
        is_blocked INTEGER NOT NULL DEFAULT 0,
        source TEXT NOT NULL DEFAULT 'auto',
        first_seen_at INTEGER NOT NULL,
        last_updated_at INTEGER NOT NULL,
        PRIMARY KEY (chain_id, address)
    )
    "#,
];

/// All CREATE INDEX statements
pub const CREATE_INDEXES: &[&str] = &[
    // Core token metadata indexes
    "CREATE INDEX IF NOT EXISTS idx_tokens_discovered ON tokens(chain_id, first_discovered_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_blockchain_created ON tokens(chain_id, blockchain_created_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_metadata_fetched ON tokens(chain_id, metadata_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol)",
    // Case-insensitive search indexes for symbol and name (improves dashboard search performance)
    "CREATE INDEX IF NOT EXISTS idx_tokens_symbol_nocase ON tokens(symbol COLLATE NOCASE)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_name_nocase ON tokens(name COLLATE NOCASE)",

    // DexScreener market data indexes
    "CREATE INDEX IF NOT EXISTS idx_market_dex_last_fetch ON market_dexscreener(market_data_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_dex_first_fetch ON market_dexscreener(market_data_first_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_dex_liquidity ON market_dexscreener(liquidity_usd DESC)",

    // GeckoTerminal market data indexes
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_last_fetch ON market_geckoterminal(market_data_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_first_fetch ON market_geckoterminal(market_data_first_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_liquidity ON market_geckoterminal(liquidity_usd DESC)",

    // Rejection stats index (hourly buckets)
    "CREATE INDEX IF NOT EXISTS idx_rejection_stats_hour ON rejection_stats(chain_id, bucket_hour DESC)",

    // Token pools indexes
    "CREATE INDEX IF NOT EXISTS idx_token_pools_mint ON token_pools(chain_id, mint)",
    "CREATE INDEX IF NOT EXISTS idx_token_pools_last_fetch ON token_pools(pool_data_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_token_pools_first_seen ON token_pools(pool_data_first_seen_at DESC)",

    // Security data indexes
    "CREATE INDEX IF NOT EXISTS idx_security_rug_last_fetch ON security_rugcheck(security_data_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_security_rug_first_fetch ON security_rugcheck(security_data_first_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_security_rug_score ON security_rugcheck(score DESC)",

    // Blacklist indexes
    "CREATE INDEX IF NOT EXISTS idx_blacklist_added ON blacklist(added_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_blacklist_source ON blacklist(source)",

    // Update tracking indexes (for priority queries and sorting)
    "CREATE INDEX IF NOT EXISTS idx_tracking_market_update ON update_tracking(chain_id, market_data_last_updated_at ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_security_update ON update_tracking(chain_id, security_data_last_updated_at ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_pool_calc ON update_tracking(pool_price_last_calculated_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_priority_market ON update_tracking(chain_id, priority DESC, market_data_last_updated_at ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_priority_calc ON update_tracking(priority DESC, pool_price_last_calculated_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_market_error_type ON update_tracking(market_error_type)",

    // Rejection indexes — every filtering-tab surface reads `update_tracking` through the
    // rejection columns, and without these each read is a full scan plus a temp b-tree sort
    // of the whole table (453k rows on the owner's database). All three are PARTIAL on
    // `last_rejection_reason IS NOT NULL`, which is the predicate every one of those queries
    // already carries, so they never index the passing tokens.
    //   * `_reason` covers the stats GROUP BY (Status tab, polled every 5s): 366ms -> 42ms.
    //   * `_at` drives the unfiltered "most recently rejected" reads (Analytics' recent
    //     rejections, unfiltered Explore paging): 1104ms -> <1ms.
    //   * `_reason_at` drives Explore, which always pages within one reason: 41ms -> <1ms.
    "CREATE INDEX IF NOT EXISTS idx_tracking_rejection_reason ON update_tracking(chain_id, last_rejection_reason, last_rejection_source) WHERE last_rejection_reason IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_tracking_rejection_at ON update_tracking(chain_id, last_rejection_at DESC) WHERE last_rejection_reason IS NOT NULL",
    "CREATE INDEX IF NOT EXISTS idx_tracking_rejection_reason_at ON update_tracking(chain_id, last_rejection_reason, last_rejection_at DESC) WHERE last_rejection_reason IS NOT NULL",

    // Composite indexes for common sorting patterns
    "CREATE INDEX IF NOT EXISTS idx_tokens_discovery_mint ON tokens(chain_id, first_discovered_at DESC, mint)",

    // Token favorites indexes
    "CREATE INDEX IF NOT EXISTS idx_favorites_mint ON token_favorites(mint)",
    "CREATE INDEX IF NOT EXISTS idx_favorites_created ON token_favorites(created_at DESC)",

    // Rejection history indexes (for time-range queries)
    "CREATE INDEX IF NOT EXISTS idx_rejection_history_time ON rejection_history(rejected_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_rejection_history_reason_time ON rejection_history(reason, rejected_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_rejection_history_mint ON rejection_history(mint)",

    // Unique constraint to prevent duplicate rejection history entries
    "CREATE UNIQUE INDEX IF NOT EXISTS idx_rejection_history_unique ON rejection_history(chain_id, mint, source, rejected_at)",

    // Partial index for active (non-blacklisted) tokens by priority (common query pattern)
    "CREATE INDEX IF NOT EXISTS idx_tracking_active_priority ON update_tracking(priority DESC, market_data_last_updated_at ASC) WHERE last_rejection_at IS NULL",

    // Authority reputation indexes (for auto-discovery queries)
    "CREATE INDEX IF NOT EXISTS idx_authority_rep_blocked ON authority_reputation(is_blocked) WHERE is_blocked = 1",
    "CREATE INDEX IF NOT EXISTS idx_authority_rep_confidence ON authority_reputation(confidence DESC)",
];

/// Initialize database schema.
///
/// Fresh table/index creation is separate from upgrades. The chain-identity
/// rebuild and additive column steps each inspect on-disk shape before touching
/// anything: `user_version` is recorded after success, never used as the only
/// signal that a required structural change is already present.
pub fn initialize_schema(conn: &Connection) -> Result<(), String> {
    // Apply centralized PRAGMA configuration
    database::configure_connection(conn, database::TOKENS_DB)
        .map_err(|e| format!("Failed to configure connection: {e}"))?;

    if migrations::table_needs_chain_rebuild(conn)? {
        migrations::migrate_legacy_schema(conn)?;
    }

    for statement in CREATE_TABLES {
        conn.execute(statement, [])
            .map_err(|e| format!("Failed to create table: {e}"))?;
    }

    migrations::apply_additive_migrations(conn)?;

    for statement in CREATE_INDEXES {
        conn.execute(statement, [])
            .map_err(|e| format!("Failed to create index: {e}"))?;
    }

    if conn
        .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
        .optional()
        .map_err(|error| format!("Failed to validate tokens foreign keys: {error}"))?
        .is_some()
    {
        return Err("Tokens schema initialization failed foreign-key validation".to_owned());
    }

    conn.pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|error| format!("Failed to record tokens schema version: {error}"))
}

/// Check if database is initialized
pub fn is_initialized(conn: &Connection) -> bool {
    conn.prepare("SELECT 1 FROM tokens LIMIT 1").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn legacy_statement(statement: &str) -> String {
        statement
            .replace("chain_id TEXT NOT NULL DEFAULT 'solana',", "")
            .replace(
                "PRIMARY KEY (chain_id, mint, pool_address)",
                "PRIMARY KEY (mint, pool_address)",
            )
            .replace("PRIMARY KEY (chain_id, mint)", "PRIMARY KEY (mint)")
            .replace("UNIQUE (chain_id, mint)", "UNIQUE (mint)")
            .replace(
                "PRIMARY KEY (chain_id, bucket_hour, reason, source)",
                "PRIMARY KEY (bucket_hour, reason, source)",
            )
            .replace("PRIMARY KEY (chain_id, address)", "PRIMARY KEY (address)")
            .replace(
                "FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint)",
                "FOREIGN KEY (mint) REFERENCES tokens(mint)",
            )
    }

    #[test]
    fn legacy_tokens_schema_migrates_losslessly_and_idempotently() {
        let conn = Connection::open_in_memory().unwrap();
        for statement in CREATE_TABLES {
            conn.execute(&legacy_statement(statement), []).unwrap();
        }
        conn.execute(
            "INSERT INTO tokens (mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at) VALUES ('legacy-mint', 'OLD', 'Legacy', 9, 1, 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE INDEX \"legacy index \"\"quoted\"\"\" ON tokens(symbol)",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();

        initialize_schema(&conn).unwrap();
        initialize_schema(&conn).unwrap();

        let migrated: (String, String) = conn
            .query_row(
                "SELECT chain_id, mint FROM tokens WHERE mint = 'legacy-mint'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(migrated, ("solana".to_owned(), "legacy-mint".to_owned()));
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens WHERE mint = 'legacy-mint'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn raw_foreign_chain_rows_do_not_match_solana_queries() {
        let conn = Connection::open_in_memory().unwrap();
        initialize_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO tokens (chain_id, mint, symbol, name, decimals, first_discovered_at, metadata_last_fetched_at, decimals_last_fetched_at) VALUES ('foreign', 'shared-mint', 'X', 'Foreign', 9, 1, 1, 1)",
            [],
        )
        .unwrap();

        let solana_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM tokens WHERE chain_id = ?1 AND mint = ?2",
                ["solana", "shared-mint"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(solana_rows, 0);
    }
}
