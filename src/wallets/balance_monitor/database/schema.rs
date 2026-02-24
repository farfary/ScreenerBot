// Database schema version
pub(super) const WALLET_SCHEMA_VERSION: u32 = 3;

// =============================================================================
// DATABASE SCHEMA DEFINITIONS
// =============================================================================

pub(super) const SCHEMA_WALLET_SNAPSHOTS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    wallet_address TEXT NOT NULL,
    snapshot_time TEXT NOT NULL,
    sol_balance REAL NOT NULL,
    sol_balance_lamports INTEGER NOT NULL,
    total_tokens_count INTEGER NOT NULL DEFAULT 0,
    total_nfts_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub(super) const SCHEMA_TOKEN_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS token_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    balance_ui REAL NOT NULL,
    decimals INTEGER NOT NULL DEFAULT 0,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

pub(super) const SCHEMA_NFT_BALANCES: &str = r#"
CREATE TABLE IF NOT EXISTS nft_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    account_address TEXT NOT NULL,
    name TEXT,
    symbol TEXT,
    image_url TEXT,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
"#;

pub(super) const SCHEMA_WALLET_METADATA: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Cache table for pre-aggregated SOL flows (one row per processed transaction)
pub(super) const SCHEMA_SOL_FLOW_CACHE: &str = r#"
CREATE TABLE IF NOT EXISTS sol_flow_cache (
    signature TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    sol_delta REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

pub(super) const SCHEMA_WALLET_DASHBOARD_METRICS: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_dashboard_metrics (
    window_key TEXT PRIMARY KEY,
    window_hours INTEGER NOT NULL,
    snapshot_limit INTEGER NOT NULL,
    token_limit INTEGER NOT NULL,
    payload_blob BLOB NOT NULL,
    payload_format TEXT NOT NULL DEFAULT 'json-gzip',
    computed_at TEXT NOT NULL,
    valid_until TEXT NOT NULL,
    computation_duration_ms INTEGER,
    snapshot_count INTEGER NOT NULL DEFAULT 0,
    flow_cache_rows INTEGER NOT NULL DEFAULT 0,
    last_processed_timestamp TEXT,
    last_processed_signature TEXT,
    window_start TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
"#;

// Indexes for fast range aggregation on cache
pub(super) const FLOW_CACHE_INDEXES: &[&str] =
    &["CREATE INDEX IF NOT EXISTS idx_flow_cache_timestamp ON sol_flow_cache(timestamp DESC);"];

pub(super) const DASHBOARD_METRICS_INDEXES: &[&str] = &["CREATE INDEX IF NOT EXISTS idx_dashboard_metrics_valid_until ON wallet_dashboard_metrics(valid_until DESC);"];

// Performance indexes
pub(super) const WALLET_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_address ON wallet_snapshots(wallet_address);",
    "CREATE INDEX IF NOT EXISTS idx_wallet_snapshots_time ON wallet_snapshots(snapshot_time DESC);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_id ON token_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON token_balances(mint);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_snapshot_mint ON token_balances(snapshot_id, mint);",
    "CREATE INDEX IF NOT EXISTS idx_nft_balances_snapshot_id ON nft_balances(snapshot_id);",
    "CREATE INDEX IF NOT EXISTS idx_nft_balances_mint ON nft_balances(mint);",
];
