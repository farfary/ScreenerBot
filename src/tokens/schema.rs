/// Database schema for tokens system
/// Clean slate implementation (no migrations/ALTER fallbacks)
///
/// TIMESTAMP NAMING CONVENTION: {what}_{when}_{action}_at
/// - {what}: Specific data type (market_data, security_data, metadata, pool_price, etc.)
/// - {when}: last / first / blockchain
/// - {action}: fetched / calculated / updated / created / discovered
/// - _at: Suffix for all timestamps (consistent)
use rusqlite::Connection;
use std::time::Duration;

pub const SCHEMA_VERSION: i32 = 2;

/// All CREATE TABLE statements
pub const CREATE_TABLES: &[&str] = &[
    // Core token metadata
    r#"
    CREATE TABLE IF NOT EXISTS tokens (
        mint TEXT PRIMARY KEY,
        symbol TEXT,
        name TEXT,
        decimals INTEGER,
        first_discovered_at INTEGER NOT NULL,
        blockchain_created_at INTEGER,
        metadata_last_fetched_at INTEGER NOT NULL,
        decimals_last_fetched_at INTEGER NOT NULL
    )
    "#,
    // DexScreener market data (per token, per source)
    r#"
    CREATE TABLE IF NOT EXISTS market_dexscreener (
        mint TEXT PRIMARY KEY,
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
        chain_id TEXT,
        dex_id TEXT,
        url TEXT,
        pair_blockchain_created_at INTEGER,
        image_url TEXT,
        header_image_url TEXT,
        market_data_last_fetched_at INTEGER NOT NULL,
        market_data_first_fetched_at INTEGER NOT NULL,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE RESTRICT
    )
    "#,
    // GeckoTerminal market data (per token, per source)
    r#"
    CREATE TABLE IF NOT EXISTS market_geckoterminal (
        mint TEXT PRIMARY KEY,
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
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE RESTRICT
    )
    "#,
    // Aggregated token pool data (multi-source, per pool)
    r#"
    CREATE TABLE IF NOT EXISTS token_pools (
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
        PRIMARY KEY (mint, pool_address),
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE CASCADE
    )
    "#,
    // Rugcheck security data (per token)
    r#"
    CREATE TABLE IF NOT EXISTS security_rugcheck (
        mint TEXT PRIMARY KEY,
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
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE RESTRICT
    )
    "#,
    // Blacklist
    r#"
    CREATE TABLE IF NOT EXISTS blacklist (
        mint TEXT PRIMARY KEY,
        reason TEXT NOT NULL,
        source TEXT,
        added_at INTEGER NOT NULL
    )
    "#,
    // Update tracking and priority
    r#"
    CREATE TABLE IF NOT EXISTS update_tracking (
        mint TEXT PRIMARY KEY,
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
        last_rejection_reason TEXT,
        last_rejection_source TEXT,
        last_rejection_at INTEGER,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE RESTRICT
    )
    "#,
    // Token favorites (user-saved tokens)
    r#"
    CREATE TABLE IF NOT EXISTS token_favorites (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mint TEXT NOT NULL UNIQUE,
        name TEXT,
        symbol TEXT,
        logo_url TEXT,
        notes TEXT,
        created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    )
    "#,
    // Rejection history table for time-range analytics
    r#"
    CREATE TABLE IF NOT EXISTS rejection_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mint TEXT NOT NULL,
        reason TEXT NOT NULL,
        source TEXT NOT NULL,
        rejected_at INTEGER NOT NULL
    )
    "#,
    // Aggregated rejection stats table (hourly buckets) - replaces per-event logging
    r#"
    CREATE TABLE IF NOT EXISTS rejection_stats (
        bucket_hour INTEGER NOT NULL,
        reason TEXT NOT NULL,
        source TEXT NOT NULL,
        rejection_count INTEGER NOT NULL DEFAULT 0,
        unique_tokens INTEGER NOT NULL DEFAULT 0,
        first_seen INTEGER NOT NULL,
        last_seen INTEGER NOT NULL,
        PRIMARY KEY (bucket_hour, reason, source)
    )
    "#,
];

/// All CREATE INDEX statements
pub const CREATE_INDEXES: &[&str] = &[
    // Core token metadata indexes
    "CREATE INDEX IF NOT EXISTS idx_tokens_discovered ON tokens(first_discovered_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_blockchain_created ON tokens(blockchain_created_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_metadata_fetched ON tokens(metadata_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol)",

    // DexScreener market data indexes
    "CREATE INDEX IF NOT EXISTS idx_market_dex_last_fetch ON market_dexscreener(market_data_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_dex_first_fetch ON market_dexscreener(market_data_first_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_dex_liquidity ON market_dexscreener(liquidity_usd DESC)",

    // GeckoTerminal market data indexes
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_last_fetch ON market_geckoterminal(market_data_last_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_first_fetch ON market_geckoterminal(market_data_first_fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_liquidity ON market_geckoterminal(liquidity_usd DESC)",

    // Rejection stats index (hourly buckets)
    "CREATE INDEX IF NOT EXISTS idx_rejection_stats_hour ON rejection_stats(bucket_hour DESC)",

    // Token pools indexes
    "CREATE INDEX IF NOT EXISTS idx_token_pools_mint ON token_pools(mint)",
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
    "CREATE INDEX IF NOT EXISTS idx_tracking_market_update ON update_tracking(market_data_last_updated_at ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_security_update ON update_tracking(security_data_last_updated_at ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_pool_calc ON update_tracking(pool_price_last_calculated_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_priority_market ON update_tracking(priority DESC, market_data_last_updated_at ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_priority_calc ON update_tracking(priority DESC, pool_price_last_calculated_at DESC)",

    // Composite indexes for common sorting patterns
    "CREATE INDEX IF NOT EXISTS idx_tokens_discovery_mint ON tokens(first_discovered_at DESC, mint)",

    // Token favorites indexes
    "CREATE INDEX IF NOT EXISTS idx_favorites_mint ON token_favorites(mint)",
    "CREATE INDEX IF NOT EXISTS idx_favorites_created ON token_favorites(created_at DESC)",

    // Rejection history indexes (for time-range queries)
    "CREATE INDEX IF NOT EXISTS idx_rejection_history_time ON rejection_history(rejected_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_rejection_history_reason_time ON rejection_history(reason, rejected_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_rejection_history_mint ON rejection_history(mint)",
];

/// ALTER TABLE statements for schema migrations (existing databases)
pub const ALTER_STATEMENTS: &[&str] = &[
    // Add score_normalised column to security_rugcheck (0-100, HIGHER = MORE RISKY)
    "ALTER TABLE security_rugcheck ADD COLUMN score_normalised INTEGER",
    // Add rejection tracking columns to update_tracking
    "ALTER TABLE update_tracking ADD COLUMN last_rejection_reason TEXT",
    "ALTER TABLE update_tracking ADD COLUMN last_rejection_source TEXT",
    "ALTER TABLE update_tracking ADD COLUMN last_rejection_at INTEGER",
    // Add mutable metadata tracking to security_rugcheck
    "ALTER TABLE security_rugcheck ADD COLUMN update_authority TEXT",
    "ALTER TABLE security_rugcheck ADD COLUMN is_mutable INTEGER",
];

/// Performance PRAGMAs
// Kept for reference; we now set PRAGMAs via rusqlite APIs to avoid "Execute returned results" errors
pub const PERFORMANCE_PRAGMAS: &[&str] = &[];

/// Initialize database schema
pub fn initialize_schema(conn: &Connection) -> Result<(), String> {
    // Apply PRAGMAs using proper APIs (some PRAGMAs return rows and must not be executed directly)
    conn.pragma_update(None, "journal_mode", &"WAL")
        .map_err(|e| format!("Failed to set journal_mode: {}", e))?;
    conn.pragma_update(None, "synchronous", &"NORMAL")
        .map_err(|e| format!("Failed to set synchronous: {}", e))?;
    conn.pragma_update(None, "cache_size", &10000i64)
        .map_err(|e| format!("Failed to set cache_size: {}", e))?;
    conn.pragma_update(None, "temp_store", &"MEMORY")
        .map_err(|e| format!("Failed to set temp_store: {}", e))?;
    conn.pragma_update(None, "mmap_size", &30000000000i64)
        .map_err(|e| format!("Failed to set mmap_size: {}", e))?;
    // page_size must be set before any tables are created
    conn.pragma_update(None, "page_size", &4096i64)
        .map_err(|e| format!("Failed to set page_size: {}", e))?;
    // Prefer busy_timeout API over PRAGMA busy_timeout
    conn.busy_timeout(Duration::from_millis(30000))
        .map_err(|e| format!("Failed to set busy_timeout: {}", e))?;

    // Create tables
    for statement in CREATE_TABLES {
        conn.execute(statement, [])
            .map_err(|e| format!("Failed to create table: {}", e))?;
    }

    // Create indexes
    for statement in CREATE_INDEXES {
        conn.execute(statement, [])
            .map_err(|e| format!("Failed to create index: {}", e))?;
    }

    // Apply ALTER statements (for existing databases - ignore errors if column already exists)
    for statement in ALTER_STATEMENTS {
        let _ = conn.execute(statement, []);
    }

    Ok(())
}

/// Get current schema version from database
pub fn get_schema_version(conn: &Connection) -> Result<i32, String> {
    // Try to get version from a metadata table (if we add one in future)
    // For now, just return current version
    Ok(SCHEMA_VERSION)
}

/// Check if database is initialized
pub fn is_initialized(conn: &Connection) -> bool {
    conn.prepare("SELECT 1 FROM tokens LIMIT 1").is_ok()
}
