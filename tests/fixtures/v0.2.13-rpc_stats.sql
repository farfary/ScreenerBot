-- Frozen schema of rpc_stats.db at release tag v0.2.13, plus one sentinel row
-- per table. The DDL is the exact text of src/rpc/stats/database.rs at that
-- tag; only the sentinel INSERTs below are added by the migration test.
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    total_calls INTEGER DEFAULT 0,
    total_errors INTEGER DEFAULT 0,
    is_current INTEGER DEFAULT 0
);
CREATE INDEX idx_sessions_current ON sessions(is_current);
CREATE INDEX idx_sessions_started ON sessions(started_at DESC);
CREATE TABLE providers (
    id TEXT PRIMARY KEY,
    url_masked TEXT NOT NULL,
    kind TEXT NOT NULL,
    priority INTEGER DEFAULT 100,
    enabled INTEGER DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_providers_kind ON providers(kind);
CREATE TABLE calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    method TEXT NOT NULL,
    success INTEGER NOT NULL,
    latency_ms INTEGER NOT NULL,
    error_code INTEGER,
    error_message TEXT,
    was_retried INTEGER DEFAULT 0,
    retry_count INTEGER DEFAULT 0,
    was_rate_limited INTEGER DEFAULT 0,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
CREATE INDEX idx_calls_session_time ON calls(session_id, timestamp DESC);
CREATE INDEX idx_calls_provider_time ON calls(provider_id, timestamp DESC);
CREATE INDEX idx_calls_method ON calls(method);
CREATE INDEX idx_calls_timestamp ON calls(timestamp DESC);
CREATE TABLE minute_buckets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    provider_id TEXT,
    minute_start TEXT NOT NULL,
    call_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    error_count INTEGER DEFAULT 0,
    rate_limit_count INTEGER DEFAULT 0,
    latency_sum_ms INTEGER DEFAULT 0,
    latency_min_ms INTEGER,
    latency_max_ms INTEGER,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    UNIQUE (session_id, provider_id, minute_start)
);
CREATE INDEX idx_minute_buckets_time ON minute_buckets(minute_start DESC);
CREATE TABLE provider_health (
    provider_id TEXT PRIMARY KEY,
    circuit_state TEXT NOT NULL DEFAULT 'closed',
    consecutive_failures INTEGER DEFAULT 0,
    consecutive_successes INTEGER DEFAULT 0,
    last_success TEXT,
    last_failure TEXT,
    last_error TEXT,
    avg_latency_ms REAL DEFAULT 0,
    current_rate_limit INTEGER,
    base_rate_limit INTEGER,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);
INSERT INTO sessions (id, started_at, is_current) VALUES ('sentinel-session', '2026-01-01T00:00:00+00:00', 0);
INSERT INTO providers (id, url_masked, kind, priority) VALUES ('sentinel-provider', 'https://rpc.example/__MASKED__', 'Helius', 100);
INSERT INTO calls (session_id, provider_id, method, success, latency_ms, timestamp)
VALUES ('sentinel-session', 'sentinel-provider', 'getAccountInfo', 1, 12, '2026-01-01T00:00:00+00:00');
INSERT INTO minute_buckets (session_id, provider_id, minute_start, call_count, success_count, error_count, latency_sum_ms)
VALUES ('sentinel-session', 'sentinel-provider', '2026-01-01T00:00:00+00:00', 3, 2, 1, 30);
INSERT INTO provider_health (provider_id) VALUES ('sentinel-provider');
