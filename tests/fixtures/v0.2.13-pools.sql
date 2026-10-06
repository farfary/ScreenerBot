-- Chain-scoped table and index DDL from release tag v0.2.13, src/pools/database/migrations.rs
-- (the shape a v0.2.13 open leaves on disk, stamped user_version 1).
CREATE TABLE price_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL,
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    price_usd REAL NOT NULL,
    price_sol REAL NOT NULL,
    confidence REAL NOT NULL,
    slot INTEGER NOT NULL,
    timestamp_unix INTEGER NOT NULL,
    sol_reserves REAL NOT NULL,
    token_reserves REAL NOT NULL,
    source_pool TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(chain_id, mint, pool_address, timestamp_unix)
);
CREATE TABLE blacklist_accounts (
    chain_id TEXT NOT NULL,
    account_pubkey TEXT NOT NULL,
    reason TEXT NOT NULL,
    source TEXT,
    pool_id TEXT,
    token_mint TEXT,
    error_count INTEGER DEFAULT 1,
    first_failed_at INTEGER NOT NULL,
    last_failed_at INTEGER NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY(chain_id, account_pubkey)
);
CREATE TABLE blacklist_pools (
    chain_id TEXT NOT NULL,
    pool_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    token_mint TEXT,
    program_id TEXT,
    error_count INTEGER DEFAULT 1,
    first_failed_at INTEGER NOT NULL,
    last_failed_at INTEGER NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY(chain_id, pool_id)
);
CREATE INDEX IF NOT EXISTS idx_price_history_chain_mint_timestamp ON price_history(chain_id, mint, timestamp_unix DESC);
CREATE INDEX IF NOT EXISTS idx_price_history_chain_pool_timestamp ON price_history(chain_id, pool_address, timestamp_unix DESC);
CREATE INDEX IF NOT EXISTS idx_price_history_created_at ON price_history(created_at);
CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_chain_pool ON blacklist_accounts(chain_id, pool_id);
CREATE INDEX IF NOT EXISTS idx_blacklist_accounts_chain_token ON blacklist_accounts(chain_id, token_mint);
CREATE INDEX IF NOT EXISTS idx_blacklist_pools_chain_token ON blacklist_pools(chain_id, token_mint);
