-- Frozen schema of actions.db at release tag v0.2.13, plus one sentinel row.
-- The DDL is the exact text of src/actions/database/mod.rs at that tag; only
-- the sentinel INSERT below is added by the migration test.
CREATE TABLE actions (
    id TEXT PRIMARY KEY,
    action_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    state TEXT NOT NULL,
    state_data TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT,
    duration_ms INTEGER,
    read_at TEXT,
    dismissed_at TEXT,
    metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX idx_actions_action_type ON actions(action_type);
CREATE INDEX idx_actions_entity_id ON actions(entity_id);
CREATE INDEX idx_actions_state ON actions(state);
CREATE INDEX idx_actions_started_at ON actions(started_at DESC);
CREATE INDEX idx_actions_wallet_address ON actions(wallet_address);
CREATE INDEX idx_actions_completed_at ON actions(completed_at DESC) WHERE completed_at IS NOT NULL;
INSERT INTO actions (id, action_type, entity_id, wallet_address, state, started_at, metadata, created_at, updated_at)
VALUES ('sentinel-action', 'SENTINEL_TYPE', 'sentinel-entity', 'SentinelWallet111111111111111111111111111',
        'completed', '2026-01-01T00:00:00+00:00', '{"sentinel":true}',
        '2026-01-01T00:00:00+00:00', '2026-01-01T00:00:00+00:00');
