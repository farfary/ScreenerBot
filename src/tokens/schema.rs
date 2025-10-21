/// Database schema for tokens system
/// No migrations - clean slate implementation
use rusqlite::Connection;
use std::time::Duration;

pub const SCHEMA_VERSION: i32 = 1;

/// All CREATE TABLE statements
pub const CREATE_TABLES: &[&str] = &[
    // Core token metadata
    r#"
    CREATE TABLE IF NOT EXISTS tokens (
        mint TEXT PRIMARY KEY,
        symbol TEXT,
        name TEXT,
        decimals INTEGER,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL
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
        pair_created_at INTEGER,
        fetched_at INTEGER NOT NULL,
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
        fetched_at INTEGER NOT NULL,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE RESTRICT
    )
    "#,
    // Rugcheck security data (per token)
    r#"
    CREATE TABLE IF NOT EXISTS security_rugcheck (
        mint TEXT PRIMARY KEY,
        token_type TEXT,
        score INTEGER,
        score_description TEXT,
        mint_authority TEXT,
        freeze_authority TEXT,
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
        fetched_at INTEGER NOT NULL,
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
        last_market_update INTEGER,
        last_security_update INTEGER,
        last_decimals_update INTEGER,
        market_update_count INTEGER DEFAULT 0,
        security_update_count INTEGER DEFAULT 0,
        last_error TEXT,
        last_error_at INTEGER,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE RESTRICT
    )
    "#,
];

/// All CREATE INDEX statements
pub const CREATE_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_tokens_updated ON tokens(updated_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol)",

    "CREATE INDEX IF NOT EXISTS idx_market_dex_fetched ON market_dexscreener(fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_dex_liquidity ON market_dexscreener(liquidity_usd DESC)",

    "CREATE INDEX IF NOT EXISTS idx_market_gecko_fetched ON market_geckoterminal(fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_market_gecko_liquidity ON market_geckoterminal(liquidity_usd DESC)",

    "CREATE INDEX IF NOT EXISTS idx_security_rug_fetched ON security_rugcheck(fetched_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_security_rug_score ON security_rugcheck(score DESC)",

    "CREATE INDEX IF NOT EXISTS idx_blacklist_added ON blacklist(added_at DESC)",
    "CREATE INDEX IF NOT EXISTS idx_blacklist_source ON blacklist(source)",

    "CREATE INDEX IF NOT EXISTS idx_tracking_priority ON update_tracking(priority DESC, last_market_update ASC)",
    "CREATE INDEX IF NOT EXISTS idx_tracking_market_update ON update_tracking(last_market_update ASC)",
];

/// ALTER TABLE statements to backfill new columns when upgrading existing databases.
pub const SECURITY_RUGCHECK_ALTER_STATEMENTS: &[&str] = &[
    "ALTER TABLE security_rugcheck ADD COLUMN total_holders INTEGER",
    "ALTER TABLE security_rugcheck ADD COLUMN total_lp_providers INTEGER",
    "ALTER TABLE security_rugcheck ADD COLUMN graph_insiders_detected INTEGER",
    "ALTER TABLE security_rugcheck ADD COLUMN total_market_liquidity REAL",
    "ALTER TABLE security_rugcheck ADD COLUMN total_stable_liquidity REAL",
    "ALTER TABLE security_rugcheck ADD COLUMN creator_balance_pct REAL",
    "ALTER TABLE security_rugcheck ADD COLUMN transfer_fee_pct REAL",
    "ALTER TABLE security_rugcheck ADD COLUMN transfer_fee_max_amount INTEGER",
    "ALTER TABLE security_rugcheck ADD COLUMN transfer_fee_authority TEXT",
    "ALTER TABLE security_rugcheck ADD COLUMN rugged INTEGER",
];

pub const MARKET_DEXSCREENER_ALTER_STATEMENTS: &[&str] =
    &["ALTER TABLE market_dexscreener ADD COLUMN pair_created_at INTEGER"];

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

    for statement in SECURITY_RUGCHECK_ALTER_STATEMENTS {
        let _ = conn.execute(statement, []);
    }
    for statement in MARKET_DEXSCREENER_ALTER_STATEMENTS {
        let _ = conn.execute(statement, []);
    }

    // Create indexes
    for statement in CREATE_INDEXES {
        conn.execute(statement, [])
            .map_err(|e| format!("Failed to create index: {}", e))?;
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
