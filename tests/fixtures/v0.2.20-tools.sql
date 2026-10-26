-- Frozen schema of tools.db at release tag v0.2.20, the last release whose
-- tools area wrote it, plus sentinel rows. The DDL is the text SQLite stores for
-- src/tools/database/schema.rs and the store_schema ledger at that tag; only the
-- INSERTs below are added by the cleanup-store test. The ledger records version
-- 1, and every table holds one row, so the upgrade that retires the tools tables
-- must drop exactly those and keep the ATA cleanup rows.
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
CREATE TABLE ata_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    wallet_address TEXT NOT NULL,

    -- Target configuration
    target_count INTEGER,

    -- Status
    status TEXT NOT NULL DEFAULT 'ready',
    started_at TEXT,
    ended_at TEXT,
    error_message TEXT,

    -- Metrics
    total_atas_found INTEGER NOT NULL DEFAULT 0,
    successful_closures INTEGER NOT NULL DEFAULT 0,
    failed_closures INTEGER NOT NULL DEFAULT 0,
    sol_recovered REAL NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_ata_sessions_session_id ON ata_sessions(session_id);
CREATE INDEX idx_ata_sessions_wallet ON ata_sessions(wallet_address);
CREATE INDEX idx_ata_sessions_status ON ata_sessions(status);
CREATE TABLE ata_closures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    ata_address TEXT NOT NULL,
    token_mint TEXT NOT NULL,

    -- Transaction details
    signature TEXT,
    sol_recovered REAL NOT NULL DEFAULT 0,

    -- Status
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    executed_at TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (session_id) REFERENCES ata_sessions(session_id) ON DELETE CASCADE
);
CREATE INDEX idx_ata_closures_session_id ON ata_closures(session_id);
CREATE INDEX idx_ata_closures_ata_address ON ata_closures(ata_address);
CREATE INDEX idx_ata_closures_status ON ata_closures(status);
CREATE TABLE ata_failed_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ata_address TEXT NOT NULL UNIQUE,
    token_mint TEXT,
    wallet_address TEXT NOT NULL,

    -- Failure tracking
    failure_count INTEGER NOT NULL DEFAULT 1,
    last_error TEXT,
    first_failed_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_failed_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Retry tracking
    next_retry_at TEXT,
    is_permanent_failure INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_ata_failed_ata_address ON ata_failed_cache(ata_address);
CREATE INDEX idx_ata_failed_wallet ON ata_failed_cache(wallet_address);
CREATE INDEX idx_ata_failed_permanent ON ata_failed_cache(is_permanent_failure);
CREATE INDEX idx_ata_failed_next_retry ON ata_failed_cache(next_retry_at);
CREATE TABLE tool_favorites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Token identification
    mint TEXT NOT NULL,
    symbol TEXT,
    name TEXT,
    logo_url TEXT,

    -- Tool context
    tool_type TEXT NOT NULL,

    -- Custom configuration (JSON)
    config_json TEXT,

    -- User metadata
    label TEXT,
    notes TEXT,

    -- Usage tracking
    use_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    UNIQUE(mint, tool_type)
);
CREATE INDEX idx_tool_favorites_mint ON tool_favorites(mint);
CREATE INDEX idx_tool_favorites_tool_type ON tool_favorites(tool_type);
CREATE INDEX idx_tool_favorites_use_count ON tool_favorites(use_count DESC);
CREATE TABLE mw_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    session_type TEXT NOT NULL,
    token_mint TEXT,

    -- Configuration
    total_wallets INTEGER NOT NULL DEFAULT 0,
    target_amount_sol REAL,
    min_amount_sol REAL,
    max_amount_sol REAL,
    delay_ms INTEGER NOT NULL DEFAULT 1000,
    delay_max_ms INTEGER,
    concurrency INTEGER NOT NULL DEFAULT 1,
    sol_buffer REAL NOT NULL DEFAULT 0.015,

    -- Status tracking
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT,
    ended_at TEXT,
    error_message TEXT,

    -- Metrics
    wallets_funded INTEGER NOT NULL DEFAULT 0,
    successful_ops INTEGER NOT NULL DEFAULT 0,
    failed_ops INTEGER NOT NULL DEFAULT 0,
    total_sol_spent REAL NOT NULL DEFAULT 0,
    total_sol_recovered REAL NOT NULL DEFAULT 0,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Chain scope
    chain_id TEXT NOT NULL DEFAULT 'solana'
);
CREATE INDEX idx_mw_sessions_session_id ON mw_sessions(session_id);
CREATE INDEX idx_mw_sessions_type ON mw_sessions(session_type);
CREATE INDEX idx_mw_sessions_status ON mw_sessions(status);
CREATE TABLE mw_wallet_ops (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    wallet_id INTEGER NOT NULL,
    wallet_address TEXT NOT NULL,
    op_index INTEGER NOT NULL,

    -- Operation details
    op_type TEXT NOT NULL,
    amount_sol REAL,
    token_amount REAL,
    signature TEXT,

    -- Status
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    executed_at TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (session_id) REFERENCES mw_sessions(session_id) ON DELETE CASCADE
);
CREATE INDEX idx_mw_wallet_ops_session ON mw_wallet_ops(session_id);
CREATE INDEX idx_mw_wallet_ops_wallet ON mw_wallet_ops(wallet_id);
CREATE TABLE watched_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    symbol TEXT,
    pool_address TEXT NOT NULL,
    pool_source TEXT NOT NULL,
    pool_dex TEXT,
    pool_pair TEXT,
    pool_liquidity REAL,

    -- Watch configuration
    watch_type TEXT NOT NULL,
    trigger_amount_sol REAL,
    action_amount_sol REAL,
    slippage_bps INTEGER DEFAULT 500,
    is_active INTEGER NOT NULL DEFAULT 1,

    -- Tracking
    last_checked_at TEXT,
    last_trade_signature TEXT,
    trades_detected INTEGER DEFAULT 0,
    actions_triggered INTEGER DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Chain scope
    chain_id TEXT NOT NULL DEFAULT 'solana'
);
CREATE INDEX idx_watched_tokens_mint ON watched_tokens(mint);
CREATE INDEX idx_watched_tokens_active ON watched_tokens(is_active);
CREATE TABLE store_schema (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    version INTEGER NOT NULL,
    min_reader_version INTEGER NOT NULL,
    written_by TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
INSERT INTO schema_version (version, applied_at) VALUES (1, '2026-01-01T00:00:00+00:00');
INSERT INTO store_schema (id, version, min_reader_version, written_by, updated_at)
VALUES (1, 1, 1, '0.2.20', '2026-01-01T00:00:00+00:00');
INSERT INTO ata_sessions (session_id, wallet_address)
VALUES ('sentinel-ata-session', 'SentinelWallet111111111111111111111111111111');
INSERT INTO ata_closures (session_id, ata_address, token_mint)
VALUES ('sentinel-ata-session', 'SentinelAta1111111111111111111111111111111111', 'SentinelMint1111111111111111111111111111111');
INSERT INTO ata_failed_cache (ata_address, wallet_address)
VALUES ('SentinelFailedAta111111111111111111111111111', 'SentinelWallet111111111111111111111111111111');
INSERT INTO tool_favorites (mint, tool_type)
VALUES ('SentinelMint1111111111111111111111111111111', 'multi_buy');
INSERT INTO mw_sessions (session_id, session_type)
VALUES ('sentinel-mw-session', 'buy');
INSERT INTO mw_wallet_ops (session_id, wallet_id, wallet_address, op_index, op_type)
VALUES ('sentinel-mw-session', 1, 'SentinelWallet111111111111111111111111111111', 0, 'buy');
INSERT INTO watched_tokens (mint, pool_address, pool_source, watch_type)
VALUES ('SentinelMint1111111111111111111111111111111', 'SentinelPool1111111111111111111111111111111', 'geckoterminal', 'copy');
