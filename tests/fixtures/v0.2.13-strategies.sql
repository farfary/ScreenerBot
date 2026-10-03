-- Frozen schema of strategies.db at release tag v0.2.13, plus sentinel rows.
-- The DDL is the exact text of src/strategies/database.rs at that tag; only
-- the sentinel INSERTs below are added by the migration test. The version
-- stamp records 1 -- the state a real v0.2.13 database carries -- which is
-- exactly why the chain-column migration must gate on the live schema and
-- not on this stamp.
CREATE TABLE strategies (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 10,
    timeframe TEXT NOT NULL DEFAULT '5m',
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT,
    version INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_strategies_type ON strategies(type);
CREATE INDEX idx_strategies_enabled ON strategies(enabled);
CREATE INDEX idx_strategies_priority ON strategies(priority);
CREATE TABLE strategy_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id TEXT NOT NULL,
    execution_time_ms INTEGER NOT NULL,
    result INTEGER NOT NULL,
    confidence REAL NOT NULL,
    details_json TEXT,
    token_mint TEXT,
    execution_timestamp TEXT NOT NULL,
    trade_id TEXT,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);
CREATE INDEX idx_performance_strategy ON strategy_performance(strategy_id);
CREATE INDEX idx_performance_timestamp ON strategy_performance(execution_timestamp);
CREATE INDEX idx_performance_token ON strategy_performance(token_mint);
CREATE TABLE strategy_assignments (
    position_id TEXT NOT NULL,
    strategy_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    PRIMARY KEY (position_id, strategy_id)
);
CREATE TABLE strategy_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    category TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    rules_json TEXT NOT NULL,
    parameters_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    author TEXT
);
CREATE TABLE strategy_backtests (
    id TEXT PRIMARY KEY,
    strategy_id TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    total_trades INTEGER NOT NULL,
    win_trades INTEGER NOT NULL,
    loss_trades INTEGER NOT NULL,
    total_profit_sol REAL NOT NULL,
    results_json TEXT NOT NULL,
    FOREIGN KEY (strategy_id) REFERENCES strategies(id) ON DELETE CASCADE
);
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL
);
INSERT INTO schema_version (version, applied_at) VALUES (1, '2026-01-01T00:00:00+00:00');
INSERT INTO strategies (id, name, type, rules_json, created_at, updated_at)
VALUES ('sentinel-strategy', 'Sentinel', 'ENTRY', '{"rules":[]}', '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00');
INSERT INTO strategy_performance (strategy_id, execution_time_ms, result, confidence, token_mint, execution_timestamp)
VALUES ('sentinel-strategy', 7, 1, 0.9, 'SentinelMint1111111111111111111111111111111', '2026-01-01T00:00:00+00:00');
