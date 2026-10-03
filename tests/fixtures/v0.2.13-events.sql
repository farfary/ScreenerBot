-- Frozen schema of events.db at release tag v0.2.13, plus one sentinel row.
-- The DDL is the exact text of src/events/database/mod.rs at that tag; only
-- the sentinel INSERT below is added by the migration test.
CREATE TABLE events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    event_time      TEXT    NOT NULL,
    category        TEXT    NOT NULL,
    subtype         TEXT,
    severity        TEXT    NOT NULL,
    mint            TEXT,
    reference_id    TEXT,
    message_short   TEXT,
    json_payload    TEXT    NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_events_category_time ON events(category, event_time DESC);
CREATE INDEX idx_events_reference_id ON events(reference_id);
CREATE INDEX idx_events_mint ON events(mint);
CREATE INDEX idx_events_severity_time ON events(severity, event_time DESC);
CREATE INDEX idx_events_created_at ON events(created_at);
CREATE INDEX idx_events_id_desc ON events(id DESC);
CREATE INDEX idx_events_category_severity_id ON events(category, severity, id DESC);
CREATE INDEX idx_events_mint_id ON events(mint, id DESC);
INSERT INTO events (event_time, category, subtype, severity, mint, reference_id, message_short, json_payload)
VALUES ('2026-01-01T00:00:00+00:00', 'System', 'sentinel', 'Info',
        'SentinelMint1111111111111111111111111111111', 'sentinel-reference',
        'sentinel message', '{"message":"sentinel"}');
