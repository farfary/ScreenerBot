//! Multi-Wallet Trading Tools
//!
//! Tools for coordinating trades across multiple wallets:
//! - **Multi-Buy**: Distribute buys across multiple wallets for stealth accumulation
//! - **Multi-Sell**: Coordinate sells with automatic consolidation
//! - **Consolidation**: Manage and cleanup sub-wallets, recover SOL and tokens
//!
//! ## Usage
//! ```rust,ignore
//! use screenerbot::tools::multi_wallet::{MultiBuyConfig, MultiSellConfig, ConsolidateConfig};
//!
//! // Multi-buy across 5 wallets
//! let config = MultiBuyConfig {
//!     token_mint: "TokenMint...".to_string(),
//!     wallet_count: 5,
//!     min_amount_sol: 0.01,
//!     max_amount_sol: 0.05,
//!     ..Default::default()
//! };
//!
//! // Execute multi-buy
//! let result = execute_multi_buy(config).await?;
//! ```

mod buy;
mod consolidate;
mod sell;
mod transfer;
mod types;

// Re-export types
pub use types::{
    ConsolidateConfig, MultiBuyConfig, MultiSellConfig, SessionResult, SessionStatus, WalletOpResult,
    WalletPlan,
};

// Re-export transfer functions
pub use transfer::{close_ata, collect_sol, fund_wallets, transfer_sol, transfer_token};

// Re-export operations
pub use buy::execute_multi_buy;
pub use consolidate::execute_consolidation;
pub use sell::execute_multi_sell;
