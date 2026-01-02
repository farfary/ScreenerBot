//! Multi-wallet management module
//!
//! Provides secure multi-wallet storage, generation, import/export,
//! and management with machine-bound encryption.
//!
//! ## Features
//! - Secure wallet generation using Solana SDK
//! - AES-256-GCM encryption with machine-derived keys
//! - Main wallet designation for trading
//! - Secondary wallets for tools/volume aggregator
//! - Import/export functionality
//! - Bulk import from CSV/Excel files
//!
//! ## Usage
//! ```rust,ignore
//! use screenerbot::wallets;
//!
//! // Initialize (call once at startup)
//! wallets::initialize().await?;
//!
//! // Get main wallet keypair (cached for performance)
//! let keypair = wallets::get_main_keypair().await?;
//!
//! // Create a new wallet
//! let wallet = wallets::create_wallet(CreateWalletRequest {
//!     name: "Trading Wallet".to_string(),
//!     notes: None,
//!     set_as_main: true,
//! }).await?;
//! ```

pub mod bulk;
mod crypto;
mod database;
mod manager;
mod types;

// Re-export types
pub use types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, SimpleTokenBalance,
    TokenBalance, UpdateWalletRequest, Wallet, WalletBalanceSummary, WalletRole, WalletType,
    WalletWithKey, WalletWithTokenBalance, WalletsSummary,
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
    get_all_wallet_balances,
    get_existing_wallet_addresses,
    // Main wallet (fast path)
    get_main_address,
    get_main_keypair,
    get_main_wallet,
    get_token_balances,
    get_wallet,
    get_wallet_by_address,
    get_wallet_keypair,
    // Summary
    get_wallets_summary,
    // Tools integration
    get_wallets_with_keys,
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
    update_last_used,
    update_wallet,
    update_wallet_balances,
    upsert_token_balance,
};

// Re-export crypto utilities
pub use crypto::{generate_keypair, parse_private_key, validate_address};
