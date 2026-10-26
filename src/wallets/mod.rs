// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Wallet records: the trading wallet, wallets kept from earlier versions,
//! watched wallets, the balance monitor and the empty-account cleanup.
//!
//! The trading wallet enters through setup (`config.wallet_encrypted`) and is
//! imported into the store on first start; key material stays encrypted at
//! rest with machine-derived keys. A listed wallet offers only its private-key
//! export.
//!
//! This module owns wallet records, roles and encrypted-at-rest key
//! material — never a decrypted `Keypair`/`Pubkey`. Signing, address
//! generation and key decryption are `crate::chains::solana::accounts`'
//! job; this module hands it a `wallet_id` or ciphertext/nonce pair and
//! gets back a String, never the reverse. See `tests/architecture_boundaries.rs`.

pub mod ata_cleanup;
pub mod balance_monitor;
pub(crate) mod database;
mod error;
mod manager;
mod types;
pub mod validation;
pub mod watch;

pub(crate) use error::balance_for_numeric_wire;
pub use error::{Error, Result};

// Re-export types
pub use types::{ExportWalletResponse, Wallet, WalletRole, WalletType, WalletsSummary};

// Re-export manager functions
pub use manager::{
    export_wallet,
    // Main wallet (fast path)
    get_main_address,
    get_main_wallet,
    // Access
    get_wallet,
    get_wallet_sol_balances,
    // Summary
    get_wallets_summary,
    has_main_wallet,
    // Initialization
    initialize,
    is_initialized,
    list_active_wallets,
    list_wallets,
};

// Chain-neutral encrypted-credential accessors. Crate-internal only — the
// one consumer is `crate::chains::solana::accounts::signing`, which decrypts
// them. No other module may hold a wallet's ciphertext/nonce pair.
pub(crate) use manager::{get_main_wallet_encrypted_key, get_wallet_encrypted_key};
