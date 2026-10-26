// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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

/// Drops the per-wallet token balance cache, which nothing reads. Its indexes go
/// with the table.
pub(super) const DROP_RETIRED_TABLES: &str = "DROP TABLE IF EXISTS wallet_token_balances;";

pub(super) const WALLETS_INDEXES: &[&str] = &[
    "CREATE INDEX IF NOT EXISTS idx_wallets_chain_address ON wallets(chain_id, address);",
    "CREATE INDEX IF NOT EXISTS idx_wallets_role ON wallets(role);",
    "CREATE INDEX IF NOT EXISTS idx_wallets_active ON wallets(is_active, role);",
];
