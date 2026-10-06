-- Table and index DDL for copy_tasks, copy_spend and copy_paper_positions from release tag v0.2.13, src/trader/copy/database/schema.rs.
CREATE TABLE IF NOT EXISTS copy_tasks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL,
    target_address TEXT NOT NULL,
    label TEXT,
    enabled INTEGER NOT NULL,
    mode_json TEXT NOT NULL,
    sizing_json TEXT NOT NULL,
    exit_mode_json TEXT NOT NULL,
    exit_policy_json TEXT NOT NULL DEFAULT '{}',
    max_sol_per_trade REAL NOT NULL,
    max_sol_per_token REAL NOT NULL,
    total_budget_sol REAL NOT NULL,
    min_target_trade_sol REAL,
    max_target_trade_sol REAL,
    buy_once_per_token INTEGER NOT NULL,
    slippage_pct REAL NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    require_filter_pass INTEGER,
    pause_reason_json TEXT,
    paused_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_copy_tasks_target_enabled
    ON copy_tasks(target_address, enabled);
CREATE TABLE IF NOT EXISTS copy_spend (
    task_id INTEGER NOT NULL,
    mode TEXT NOT NULL,
    mint TEXT NOT NULL,
    spent_sol REAL NOT NULL DEFAULT 0,
    buy_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (task_id, mode, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS copy_paper_positions (
    task_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    token_amount REAL NOT NULL DEFAULT 0,
    cost_basis_sol REAL NOT NULL DEFAULT 0,
    invested_sol REAL NOT NULL DEFAULT 0,
    realized_proceeds_sol REAL NOT NULL DEFAULT 0,
    realized_cost_sol REAL NOT NULL DEFAULT 0,
    buys INTEGER NOT NULL DEFAULT 0,
    sells INTEGER NOT NULL DEFAULT 0,
    last_price_sol REAL,
    last_price_at TEXT,
    opened_at TEXT NOT NULL,
    closed_at TEXT,
    updated_at TEXT NOT NULL,
    peak_price_sol REAL,
    PRIMARY KEY (task_id, mint),
    FOREIGN KEY (task_id) REFERENCES copy_tasks(id) ON DELETE CASCADE
);
