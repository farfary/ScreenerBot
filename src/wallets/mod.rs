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

mod crypto;
mod database;
mod manager;
mod types;

// Re-export types
pub use types::{
    CreateWalletRequest, ExportWalletResponse, ImportWalletRequest, UpdateWalletRequest, Wallet,
    WalletBalance, WalletRole, WalletType, WalletWithKey, WalletsSummary,
};

// Re-export manager functions
pub use manager::{
    // Initialization
    initialize,
    is_initialized,
    // Main wallet (fast path)
    get_main_address,
    get_main_keypair,
    get_main_wallet,
    has_main_wallet,
    // CRUD
    archive_wallet,
    create_wallet,
    delete_wallet,
    export_wallet,
    get_wallet,
    get_wallet_by_address,
    get_wallet_keypair,
    import_wallet,
    list_active_wallets,
    list_wallets,
    restore_wallet,
    set_main_wallet,
    update_last_used,
    update_wallet,
    // Tools integration
    get_wallets_with_keys,
    // Summary
    get_wallets_summary,
};

// Re-export crypto utilities
pub use crypto::{generate_keypair, parse_private_key, validate_address};
