-- Complete table and index DDL from release tag v0.2.13, src/transactions/database/schema.rs.
CREATE TABLE IF NOT EXISTS raw_transactions (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    slot INTEGER,
    block_time INTEGER,
    timestamp TEXT NOT NULL,
    status TEXT NOT NULL, -- 'Pending', 'Confirmed', 'Finalized', 'Failed'
    success BOOLEAN NOT NULL DEFAULT false,
    error_message TEXT,
    fee_lamports INTEGER,
    compute_units_consumed INTEGER,
    instructions_count INTEGER NOT NULL DEFAULT 0,
    accounts_count INTEGER NOT NULL DEFAULT 0,
    raw_transaction_data TEXT, -- JSON blob of raw Solana transaction data
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, signature, wallet_address)
);

CREATE TABLE IF NOT EXISTS processed_transactions (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    transaction_type TEXT NOT NULL, -- serde JSON of the TransactionType enum (rich variants round-trip)
    type_kind TEXT NOT NULL DEFAULT 'unknown', -- TransactionType::kind(): the stable value the UI filters and groups on
    direction TEXT NOT NULL, -- 'Incoming', 'Outgoing', 'Internal', 'Unknown'

    -- Balance change data (calculated fresh, not cached)
    sol_balance_change TEXT, -- JSON blob of SolBalanceChange
    token_balance_changes TEXT, -- JSON array of TokenBalanceChange

    -- Swap analysis data (calculated fresh, not cached)
    token_swap_info TEXT, -- JSON blob of TokenSwapInfo
    swap_pnl_info TEXT, -- JSON blob of SwapPnLInfo

    -- ATA operations data (calculated fresh, not cached)
    ata_operations TEXT, -- JSON array of AtaOperation

    -- Token transfers data (calculated fresh, not cached)
    token_transfers TEXT, -- JSON array of TokenTransfer

    -- Instruction analysis data (calculated fresh, not cached)
    instruction_info TEXT, -- JSON array of InstructionInfo

    -- Analysis metadata
    analysis_duration_ms INTEGER,
    cached_analysis TEXT, -- JSON blob of CachedAnalysis
    analysis_version INTEGER NOT NULL DEFAULT 2,
    -- Commonly queried scalar fields
    fee_sol REAL NOT NULL DEFAULT 0,
    sol_delta REAL,

    -- Processing timestamps
    processed_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    PRIMARY KEY (chain_id, signature, wallet_address),
    FOREIGN KEY (chain_id, signature, wallet_address) REFERENCES raw_transactions(chain_id, signature, wallet_address) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS known_signatures (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'known',
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, signature, wallet_address)
);

CREATE TABLE IF NOT EXISTS deferred_retries (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    next_retry_at TEXT NOT NULL,
    remaining_attempts INTEGER NOT NULL DEFAULT 3,
    current_delay_secs INTEGER NOT NULL DEFAULT 60,
    last_error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, signature, wallet_address)
);

CREATE TABLE IF NOT EXISTS pending_transactions (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    signature TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    added_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_checked_at TEXT,
    check_count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (chain_id, signature, wallet_address)
);

CREATE TABLE IF NOT EXISTS db_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS bootstrap_state (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    id INTEGER NOT NULL CHECK (id = 1),
    backfill_before_cursor TEXT,
    full_history_completed INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chain_id, id)
);

CREATE TABLE IF NOT EXISTS subject_asset_deltas (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    wallet_address TEXT NOT NULL,
    signature TEXT NOT NULL,
    mint TEXT NOT NULL,          -- SPL mint, or the literal 'native' for native SOL
    slot INTEGER,
    block_time INTEGER,
    tx_index INTEGER NOT NULL DEFAULT 0,
    delta_raw INTEGER NOT NULL,  -- signed, raw base units
    before_raw INTEGER,          -- NULL when not knowable
    after_raw INTEGER,
    decimals INTEGER NOT NULL,
    kind TEXT NOT NULL,          -- 'trade' | 'transfer' | 'defi' | 'other'
    venue TEXT,                  -- router name when a known DEX program is present
    fee_lamports INTEGER,
    success BOOLEAN NOT NULL DEFAULT 1,
    PRIMARY KEY (chain_id, wallet_address, signature, mint)
);

CREATE INDEX IF NOT EXISTS idx_raw_transactions_chain_wallet ON raw_transactions(chain_id, wallet_address);
CREATE INDEX IF NOT EXISTS idx_raw_transactions_chain_timestamp ON raw_transactions(chain_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_raw_transactions_status ON raw_transactions(status);
CREATE INDEX IF NOT EXISTS idx_raw_transactions_slot ON raw_transactions(slot DESC);
CREATE INDEX IF NOT EXISTS idx_raw_transactions_success ON raw_transactions(success);
CREATE INDEX IF NOT EXISTS idx_processed_transactions_chain_wallet ON processed_transactions(chain_id, wallet_address);
CREATE INDEX IF NOT EXISTS idx_processed_transactions_type ON processed_transactions(transaction_type);
CREATE INDEX IF NOT EXISTS idx_processed_transactions_type_kind ON processed_transactions(chain_id, wallet_address, type_kind);
CREATE INDEX IF NOT EXISTS idx_processed_transactions_direction ON processed_transactions(direction);
CREATE INDEX IF NOT EXISTS idx_processed_transactions_analysis_version ON processed_transactions(analysis_version);
CREATE INDEX IF NOT EXISTS idx_deferred_retries_next_retry ON deferred_retries(next_retry_at);
CREATE INDEX IF NOT EXISTS idx_known_signatures_chain_wallet ON known_signatures(chain_id, wallet_address);
CREATE INDEX IF NOT EXISTS idx_known_signatures_added_at ON known_signatures(added_at DESC);
CREATE INDEX IF NOT EXISTS idx_pending_transactions_chain_wallet ON pending_transactions(chain_id, wallet_address);
CREATE INDEX IF NOT EXISTS idx_pending_transactions_added_at ON pending_transactions(added_at DESC);
CREATE INDEX IF NOT EXISTS idx_subject_deltas_chain_wallet_mint ON subject_asset_deltas(chain_id, wallet_address, mint, slot, tx_index);
CREATE INDEX IF NOT EXISTS idx_subject_deltas_chain_wallet_order ON subject_asset_deltas(chain_id, wallet_address, slot, tx_index, signature);
