/// Database schema for tokens system
/// No migrations - clean slate implementation
use rusqlite::Connection;

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

/// Performance PRAGMAs
pub const PERFORMANCE_PRAGMAS: &[&str] = &[
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
    "PRAGMA cache_size = 10000",
    "PRAGMA temp_store = memory",
    "PRAGMA mmap_size = 30000000000",
    "PRAGMA page_size = 4096",
    "PRAGMA busy_timeout = 30000",
];

/// Initialize database schema
pub fn initialize_schema(conn: &Connection) -> Result<(), String> {
    // Apply PRAGMAs
    for pragma in PERFORMANCE_PRAGMAS {
        conn.execute(pragma, [])
            .map_err(|e| format!("Failed to apply PRAGMA: {}", e))?;
    }

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
