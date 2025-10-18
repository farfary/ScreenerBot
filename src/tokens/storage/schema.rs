// Database schema definitions for the unified token data system
// All CREATE TABLE statements with proper indexes and constraints

/// SQL statements to initialize the database schema
pub const SCHEMA_STATEMENTS: &[&str] = &[
    // Unified token metadata table
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
    // Blacklist table
    r#"
    CREATE TABLE IF NOT EXISTS blacklist (
        mint TEXT PRIMARY KEY,
        reason TEXT,
        added_at INTEGER NOT NULL
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_blacklist_added ON blacklist(added_at)
    "#,
    // Rugcheck security data (one row per token)
    r#"
    CREATE TABLE IF NOT EXISTS data_rugcheck_info (
        mint TEXT PRIMARY KEY,
        token_type TEXT,
        symbol TEXT,
        name TEXT,
        decimals INTEGER,
        supply TEXT,
        rugcheck_score INTEGER,
        rugcheck_score_description TEXT,
        market_solscan_tags TEXT,
        market_top_holders_percentage REAL,
        risks TEXT,
        top_holders TEXT,
        fetched_at INTEGER NOT NULL,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_rugcheck_fetched ON data_rugcheck_info(fetched_at)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_rugcheck_score ON data_rugcheck_info(rugcheck_score)
    "#,
];

/// Pragmas for optimal SQLite performance
pub const PERFORMANCE_PRAGMAS: &[&str] = &[
    "PRAGMA journal_mode = WAL",
    "PRAGMA synchronous = NORMAL",
    "PRAGMA cache_size = 10000",
    "PRAGMA temp_store = memory",
    "PRAGMA mmap_size = 30000000000",
    "PRAGMA page_size = 4096",
    "PRAGMA busy_timeout = 30000",
];
