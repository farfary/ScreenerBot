-- Table and index DDL for schema_migrations, ohlcv_pools, ohlcv_candles, ohlcv_monitor_config and
-- ohlcv_data_versions from release tag v0.2.13, src/ohlcvs/database/{mod,migrations,data_version}.rs.
CREATE TABLE IF NOT EXISTS schema_migrations (
    migration_id TEXT PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE IF NOT EXISTS ohlcv_pools (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL,
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    dex TEXT NOT NULL,
    liquidity REAL NOT NULL DEFAULT 0.0,
    is_default INTEGER NOT NULL DEFAULT 0,
    is_sol_pair INTEGER NOT NULL DEFAULT 1,
    last_success TEXT,
    failure_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, mint, pool_address)
);
CREATE TABLE IF NOT EXISTS ohlcv_candles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL,
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    timeframe TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    open REAL NOT NULL,
    high REAL NOT NULL,
    low REAL NOT NULL,
    close REAL NOT NULL,
    volume REAL NOT NULL,
    source TEXT NOT NULL DEFAULT 'api',
    fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(chain_id, mint, pool_address, timeframe, timestamp)
);
CREATE TABLE IF NOT EXISTS ohlcv_monitor_config (
    chain_id TEXT NOT NULL,
    mint TEXT NOT NULL,
    priority TEXT NOT NULL,
    fetch_interval_seconds INTEGER NOT NULL DEFAULT 60,
    source TEXT NOT NULL DEFAULT 'manual',
    is_active INTEGER NOT NULL DEFAULT 1,
    backfill_1m_complete INTEGER NOT NULL DEFAULT 0,
    backfill_5m_complete INTEGER NOT NULL DEFAULT 0,
    backfill_15m_complete INTEGER NOT NULL DEFAULT 0,
    backfill_1h_complete INTEGER NOT NULL DEFAULT 0,
    backfill_4h_complete INTEGER NOT NULL DEFAULT 0,
    backfill_12h_complete INTEGER NOT NULL DEFAULT 0,
    backfill_1d_complete INTEGER NOT NULL DEFAULT 0,
    backfill_started_at TEXT,
    backfill_completed_at TEXT,
    last_fetch TEXT,
    last_activity TEXT NOT NULL,
    consecutive_empty_fetches INTEGER NOT NULL DEFAULT 0,
    last_pool_discovery_attempt INTEGER,
    consecutive_pool_failures INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (chain_id, mint)
);
CREATE TABLE IF NOT EXISTS ohlcv_data_versions (
    chain_id TEXT PRIMARY KEY,
    version INTEGER NOT NULL
);
INSERT INTO schema_migrations (migration_id) VALUES ('20260821_chain_scope');
CREATE INDEX IF NOT EXISTS idx_pools_mint ON ohlcv_pools(chain_id, mint);
CREATE INDEX IF NOT EXISTS idx_pools_default ON ohlcv_pools(chain_id, mint, is_default);
CREATE INDEX IF NOT EXISTS idx_candles_lookup ON ohlcv_candles(chain_id, mint, timeframe, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_candles_pool_lookup ON ohlcv_candles(chain_id, pool_address, timeframe, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_candles_cleanup ON ohlcv_candles(fetched_at);
CREATE INDEX IF NOT EXISTS idx_monitor_active ON ohlcv_monitor_config(chain_id, is_active, priority);
CREATE INDEX IF NOT EXISTS idx_monitor_backfill ON ohlcv_monitor_config(chain_id, is_active, backfill_1d_complete);
