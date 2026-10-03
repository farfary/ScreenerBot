-- Frozen schema of tools.db at release tag v0.2.13, plus sentinel rows. The
-- DDL is the exact text of src/tools/database/schema.rs at that tag; only the
-- sentinel INSERTs below are added by the migration test. The version stamp
-- records 1 -- the state a real v0.2.13 database carries -- which is exactly
-- why the chain-column migration must gate on the live schema and not on
-- this stamp.
CREATE TABLE mw_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL UNIQUE,
    session_type TEXT NOT NULL,
    token_mint TEXT,
    total_wallets INTEGER NOT NULL DEFAULT 0,
    target_amount_sol REAL,
    min_amount_sol REAL,
    max_amount_sol REAL,
    delay_ms INTEGER NOT NULL DEFAULT 1000,
    delay_max_ms INTEGER,
    concurrency INTEGER NOT NULL DEFAULT 1,
    sol_buffer REAL NOT NULL DEFAULT 0.015,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TEXT,
    ended_at TEXT,
    error_message TEXT,
    wallets_funded INTEGER NOT NULL DEFAULT 0,
    successful_ops INTEGER NOT NULL DEFAULT 0,
    failed_ops INTEGER NOT NULL DEFAULT 0,
    total_sol_spent REAL NOT NULL DEFAULT 0,
    total_sol_recovered REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_mw_sessions_session_id ON mw_sessions(session_id);
CREATE INDEX idx_mw_sessions_type ON mw_sessions(session_type);
CREATE INDEX idx_mw_sessions_status ON mw_sessions(status);
CREATE TABLE watched_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    symbol TEXT,
    pool_address TEXT NOT NULL,
    pool_source TEXT NOT NULL,
    pool_dex TEXT,
    pool_pair TEXT,
    pool_liquidity REAL,
    watch_type TEXT NOT NULL,
    trigger_amount_sol REAL,
    action_amount_sol REAL,
    slippage_bps INTEGER DEFAULT 500,
    is_active INTEGER NOT NULL DEFAULT 1,
    last_checked_at TEXT,
    last_trade_signature TEXT,
    trades_detected INTEGER DEFAULT 0,
    actions_triggered INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_watched_tokens_mint ON watched_tokens(mint);
CREATE INDEX idx_watched_tokens_active ON watched_tokens(is_active);
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
INSERT INTO schema_version (version, applied_at) VALUES (1, '2026-01-01T00:00:00+00:00');
INSERT INTO mw_sessions (session_id, session_type, token_mint)
VALUES ('sentinel-mw-session', 'buy', 'SentinelMint1111111111111111111111111111111');
INSERT INTO watched_tokens (mint, pool_address, pool_source, watch_type)
VALUES ('SentinelMint1111111111111111111111111111111', 'SentinelPool1111111111111111111111111111111', 'geckoterminal', 'copy');
