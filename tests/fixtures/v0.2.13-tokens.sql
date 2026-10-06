-- Table and index DDL for tokens and token_pools from release tag v0.2.13, src/tokens/schema.rs.
CREATE TABLE IF NOT EXISTS tokens (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    mint TEXT NOT NULL,
    symbol TEXT,
    name TEXT,
    decimals INTEGER,
    first_discovered_at INTEGER NOT NULL,
    blockchain_created_at INTEGER,
    metadata_last_fetched_at INTEGER NOT NULL,
    decimals_last_fetched_at INTEGER NOT NULL,
    PRIMARY KEY (chain_id, mint)
);
CREATE TABLE IF NOT EXISTS token_pools (
    chain_id TEXT NOT NULL DEFAULT 'solana',
    mint TEXT NOT NULL,
    pool_address TEXT NOT NULL,
    dex TEXT,
    base_mint TEXT NOT NULL,
    quote_mint TEXT NOT NULL,
    is_sol_pair INTEGER NOT NULL,
    liquidity_usd REAL,
    liquidity_token REAL,
    liquidity_sol REAL,
    volume_h24 REAL,
    price_usd REAL,
    price_sol REAL,
    price_native TEXT,
    sources_json TEXT,
    pool_data_last_fetched_at INTEGER NOT NULL,
    pool_data_first_seen_at INTEGER NOT NULL,
    PRIMARY KEY (chain_id, mint, pool_address),
    FOREIGN KEY (chain_id, mint) REFERENCES tokens(chain_id, mint) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_tokens_discovered ON tokens(chain_id, first_discovered_at DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_blockchain_created ON tokens(chain_id, blockchain_created_at DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_metadata_fetched ON tokens(chain_id, metadata_last_fetched_at DESC);
CREATE INDEX IF NOT EXISTS idx_tokens_symbol ON tokens(symbol);
CREATE INDEX IF NOT EXISTS idx_tokens_symbol_nocase ON tokens(symbol COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_tokens_name_nocase ON tokens(name COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_tokens_discovery_mint ON tokens(chain_id, first_discovered_at DESC, mint);
CREATE INDEX IF NOT EXISTS idx_token_pools_mint ON token_pools(chain_id, mint);
CREATE INDEX IF NOT EXISTS idx_token_pools_last_fetch ON token_pools(pool_data_last_fetched_at DESC);
CREATE INDEX IF NOT EXISTS idx_token_pools_first_seen ON token_pools(pool_data_first_seen_at DESC);
