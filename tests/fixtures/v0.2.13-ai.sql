-- Frozen schema of ai.db at release tag v0.2.13, plus one sentinel row. The
-- DDL is the exact text of src/llm_analysis/database.rs at that tag; only the
-- sentinel INSERT below is added by the migration test.
CREATE TABLE ai_instructions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'general',
    priority INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE TABLE ai_decision_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mint TEXT NOT NULL,
    symbol TEXT,
    decision TEXT NOT NULL,
    confidence INTEGER NOT NULL,
    reasoning TEXT,
    risk_level TEXT,
    provider TEXT NOT NULL,
    model TEXT,
    tokens_used INTEGER NOT NULL DEFAULT 0,
    latency_ms REAL NOT NULL DEFAULT 0,
    cached INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_decisions_mint ON ai_decision_history(mint);
CREATE INDEX idx_decisions_created ON ai_decision_history(created_at DESC);
INSERT INTO ai_decision_history (mint, symbol, decision, confidence, provider, created_at)
VALUES ('SentinelMint1111111111111111111111111111111', 'SENT', 'buy', 80, 'sentinel-provider', '2026-01-01T00:00:00+00:00');
