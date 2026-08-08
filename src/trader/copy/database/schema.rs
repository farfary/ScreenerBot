pub(super) const SCHEMA_VERSION: i64 = 1;

pub(super) const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS copy_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS copy_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    target_address TEXT NOT NULL,
    label TEXT,
    enabled INTEGER NOT NULL,
    mode_json TEXT NOT NULL,
    sizing_json TEXT NOT NULL,
    exit_mode_json TEXT NOT NULL,
    max_sol_per_trade REAL NOT NULL,
    max_sol_per_token REAL NOT NULL,
    total_budget_sol REAL NOT NULL,
    min_target_trade_sol REAL,
    max_target_trade_sol REAL,
    buy_once_per_token INTEGER NOT NULL,
    slippage_pct REAL NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_copy_tasks_target_enabled
    ON copy_tasks(target_address, enabled);
CREATE TABLE IF NOT EXISTS copy_spend (
    task_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    spent_sol REAL NOT NULL DEFAULT 0,
    buy_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    signature TEXT NOT NULL,
    mint TEXT,
    outcome_json TEXT NOT NULL,
    decided_at TEXT NOT NULL,
    UNIQUE (task_id, signature),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_live_claims (
    task_id INTEGER NOT NULL,
    signature TEXT NOT NULL,
    claimed_at TEXT NOT NULL,
    PRIMARY KEY (task_id, signature),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_copy_decisions_task_time
    ON copy_decisions(task_id, decided_at DESC);
CREATE TABLE IF NOT EXISTS copy_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    details_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_copy_activity_task_time
    ON copy_activity(task_id, created_at DESC);
CREATE TABLE IF NOT EXISTS copy_position_links (
    task_id INTEGER NOT NULL,
    position_id TEXT NOT NULL UNIQUE,
    mint TEXT NOT NULL,
    created_at TEXT NOT NULL,
    PRIMARY KEY (task_id, position_id),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
"#;
