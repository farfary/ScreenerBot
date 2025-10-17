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
    // DexScreener pools data (one row per pair)
    r#"
    CREATE TABLE IF NOT EXISTS data_dexscreener_pools (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mint TEXT NOT NULL,
        chain_id TEXT,
        dex_id TEXT,
        pair_address TEXT,
        base_token_address TEXT,
        base_token_name TEXT,
        base_token_symbol TEXT,
        quote_token_address TEXT,
        quote_token_name TEXT,
        quote_token_symbol TEXT,
        price_native REAL,
        price_usd TEXT,
        liquidity_usd REAL,
        liquidity_base REAL,
        liquidity_quote REAL,
        fdv REAL,
        market_cap REAL,
        price_change_m5 REAL,
        price_change_h1 REAL,
        price_change_h6 REAL,
        price_change_h24 REAL,
        volume_m5 REAL,
        volume_h1 REAL,
        volume_h6 REAL,
        volume_h24 REAL,
        txns_m5_buys INTEGER,
        txns_m5_sells INTEGER,
        txns_h1_buys INTEGER,
        txns_h1_sells INTEGER,
        txns_h6_buys INTEGER,
        txns_h6_sells INTEGER,
        txns_h24_buys INTEGER,
        txns_h24_sells INTEGER,
        pair_created_at INTEGER,
        labels TEXT,
        url TEXT,
        info_image_url TEXT,
        info_header TEXT,
        info_open_graph TEXT,
        info_websites TEXT,
        info_socials TEXT,
        boosts_active INTEGER,
        fetched_at INTEGER NOT NULL,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_dexscreener_mint ON data_dexscreener_pools(mint)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_dexscreener_pair ON data_dexscreener_pools(pair_address)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_dexscreener_fetched ON data_dexscreener_pools(fetched_at)
    "#,
    // GeckoTerminal pools data (one row per pool)
    r#"
    CREATE TABLE IF NOT EXISTS data_geckoterminal_pools (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mint TEXT NOT NULL,
        pool_address TEXT,
        pool_name TEXT,
        dex_id TEXT,
        base_token_id TEXT,
        quote_token_id TEXT,
        base_token_price_usd TEXT,
        base_token_price_native TEXT,
        base_token_price_quote TEXT,
        quote_token_price_usd TEXT,
        quote_token_price_native TEXT,
        quote_token_price_base TEXT,
        token_price_usd TEXT,
        fdv_usd REAL,
        market_cap_usd REAL,
        reserve_usd REAL,
        price_change_percentage_m5 TEXT,
        price_change_percentage_m15 TEXT,
        price_change_percentage_m30 TEXT,
        price_change_percentage_h1 TEXT,
        price_change_percentage_h6 TEXT,
        price_change_percentage_h24 TEXT,
        volume_usd_m5 TEXT,
        volume_usd_m15 TEXT,
        volume_usd_m30 TEXT,
        volume_usd_h1 TEXT,
        volume_usd_h6 TEXT,
        volume_usd_h24 TEXT,
        transactions_m5_buys INTEGER,
        transactions_m5_sells INTEGER,
        transactions_m15_buys INTEGER,
        transactions_m15_sells INTEGER,
        transactions_m30_buys INTEGER,
        transactions_m30_sells INTEGER,
        transactions_h1_buys INTEGER,
        transactions_h1_sells INTEGER,
        transactions_h6_buys INTEGER,
        transactions_h6_sells INTEGER,
        transactions_h24_buys INTEGER,
        transactions_h24_sells INTEGER,
        fdv_usd TEXT,
        market_cap_usd TEXT,
        pool_created_at TEXT,
        dex_id TEXT,
        fetched_at INTEGER NOT NULL,
        FOREIGN KEY (mint) REFERENCES tokens(mint) ON DELETE CASCADE
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_geckoterminal_mint ON data_geckoterminal_pools(mint)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_geckoterminal_pool ON data_geckoterminal_pools(pool_address)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_geckoterminal_fetched ON data_geckoterminal_pools(fetched_at)
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
    // API fetch log for tracking data freshness and debugging
    r#"
    CREATE TABLE IF NOT EXISTS api_fetch_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        mint TEXT NOT NULL,
        source TEXT NOT NULL,
        success INTEGER NOT NULL,
        error_message TEXT,
        records_fetched INTEGER,
        fetched_at INTEGER NOT NULL
    )
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_fetch_log_mint ON api_fetch_log(mint)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_fetch_log_source ON api_fetch_log(source)
    "#,
    r#"
    CREATE INDEX IF NOT EXISTS idx_fetch_log_time ON api_fetch_log(fetched_at)
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
