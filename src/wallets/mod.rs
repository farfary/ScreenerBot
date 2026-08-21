//! Multi-wallet management module
//!
//! Provides secure multi-wallet storage, generation, import/export,
//! and management with machine-bound encryption.
//!
//! ## Features
//! - Secure wallet generation using Solana SDK
//! - AES-256-GCM encryption with machine-derived keys
//! - Main wallet designation for trading
//! - Secondary wallets for tools
//! - Import/export functionality
//! - Bulk import from CSV/Excel files
//! - Token balance tracking (cached + real-time RPC)
//! - Balance monitoring with historical snapshots
//!
//! ## Usage
//! ```rust,ignore
//! use screenerbot::wallets;
//!
//! // Initialize (call once at startup)
//! wallets::initialize().await?;
//!
//! // Actual signing goes through the chain-owned adapter, never this module:
//! // crate::chains::solana::accounts::main_keypair().await?;
//!
//! // Create a new wallet
//! let wallet = wallets::create_wallet(CreateWalletRequest {
//!     name: "Trading Wallet".to_owned(),
//!     notes: None,
//!     set_as_main: true,
//! }).await?;
//! ```
//!
//! This module owns wallet records, roles and encrypted-at-rest key
//! material — never a decrypted `Keypair`/`Pubkey`. Signing, address
//! generation and key decryption are `crate::chains::solana::accounts`'
//! job; this module hands it a `wallet_id` or ciphertext/nonce pair and
//! gets back a String, never the reverse. See `tests/architecture_boundaries.rs`.

pub mod balance_monitor;
pub mod bulk;
/// Solana keypair generation/parsing/signing mechanics live at
/// `crate::chains::solana::accounts::keypair`; this alias keeps every
/// internal `super::super::crypto::*` call site working without churning
/// the wallet management submodules. Only chain-neutral helpers from that
/// module (address/material derivation, never a `Keypair`-returning one)
/// are used through it — see the doc comment above.
use crate::chains::solana::accounts::keypair as crypto;
mod database;
mod manager;
pub mod recovery;
mod types;
pub mod validation;
pub mod watch;

// Re-export types
pub use types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, SimpleTokenBalance,
    TokenBalance, UpdateWalletRequest, Wallet, WalletBalanceSummary, WalletRole, WalletType,
    WalletWithTokenBalance, WalletsSummary,
};

// Re-export manager functions
pub use manager::{
    // CRUD
    archive_wallet,
    // Bulk operations
    bulk_import_wallets,
    // Token balance operations
    clear_token_balances,
    create_wallet,
    create_wallets_batch,
    delete_wallet,
    export_wallet,
    export_wallets,
    get_all_token_balances,
    // Token balance queries (RPC)
    get_all_wallet_balances,
    get_existing_wallet_addresses,
    // Main wallet (fast path)
    get_main_address,
    get_main_wallet,
    get_token_balances,
    // Access
    get_wallet,
    get_wallet_by_address,
    // Tools & summary
    get_wallets_summary,
    get_wallets_with_token,
    has_main_wallet,
    import_wallet,
    // Initialization
    initialize,
    is_initialized,
    list_active_wallets,
    list_wallets,
    restore_wallet,
    set_main_wallet,
    update_all_wallet_balances,
    update_wallet,
    update_wallet_balances,
    upsert_token_balance,
};

// Chain-neutral encrypted-credential accessors. Crate-internal only — the
// one consumer is `crate::chains::solana::accounts::signing`, which decrypts
// them. No other module may hold a wallet's ciphertext/nonce pair.
pub(crate) use manager::{get_main_wallet_encrypted_key, get_wallet_encrypted_key};

// Re-export crypto utilities — chain-neutral surface only (never a
// concrete-Solana-typed return; those stay under `crate::chains::solana`).
pub use crypto::validate_address;
