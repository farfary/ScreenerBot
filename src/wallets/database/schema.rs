//! Wallets DB schema (tables + indexes)
//!
//! Kept in a dedicated module to keep `database.rs` focused on behavior.

pub(super) const WALLETS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS wallets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    chain_id TEXT NOT NULL DEFAULT 'solana',
    name TEXT NOT NULL,
    address TEXT NOT NULL,
    encrypted_key TEXT NOT NULL,
    nonce TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'secondary',
    wallet_type TEXT NOT NULL DEFAULT 'generated',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    notes TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    UNIQUE (chain_id, address)
);
"#;

pub(super) const TOKEN_BALANCES_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS wallet_token_balances (
    wallet_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    ui_amount REAL NOT NULL,
    decimals INTEGER NOT NULL,
    symbol TEXT,
    name TEXT,
    is_token_2022 INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (wallet_id, mint),
    FOREIGN KEY (wallet_id) REFERENCES wallets(id) ON DELETE CASCADE
);
"#;

pub(super) const WALLETS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallets_chain_address ON wallets(chain_id, address);",
    "CREATE INDEX IF NOT EXISTS idx_wallets_role ON wallets(role);",
    "CREATE INDEX IF NOT EXISTS idx_wallets_active ON wallets(is_active, role);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_wallet ON wallet_token_balances(wallet_id);",
    "CREATE INDEX IF NOT EXISTS idx_token_balances_mint ON wallet_token_balances(mint);",
];
