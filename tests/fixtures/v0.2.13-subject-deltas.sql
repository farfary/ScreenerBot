-- v0.2.13 transaction metadata and subject-delta table and index definitions.
CREATE TABLE IF NOT EXISTS db_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
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

CREATE INDEX IF NOT EXISTS idx_subject_deltas_chain_wallet_mint ON subject_asset_deltas(chain_id, wallet_address, mint, slot, tx_index);
CREATE INDEX IF NOT EXISTS idx_subject_deltas_chain_wallet_order ON subject_asset_deltas(chain_id, wallet_address, slot, tx_index, signature);
