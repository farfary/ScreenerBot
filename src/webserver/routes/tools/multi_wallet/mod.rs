//! Multi-wallet operation handlers
//!
//! This module provides handlers for multi-wallet operations including:
//! - Multi-buy: Coordinated token purchases across multiple wallets
//! - Multi-sell: Coordinated token sales across multiple wallets
//! - Wallet management: Summary, consolidation, and ATA cleanup
//! - Session management: Tracking and controlling multi-wallet operations

// Module declarations
mod multi_buy;
mod multi_sell;
mod session;
mod wallet_ops;

// Re-export all public handler functions
pub use multi_buy::{abort_multi_buy, get_multi_buy_status, preview_multi_buy, start_multi_buy};
pub use multi_sell::{
    abort_multi_sell, get_multi_sell_status, preview_multi_sell, start_multi_sell,
};
pub use session::{cleanup_old_sessions, get_multi_wallet_sessions, has_active_multi_wallet_session};
pub use wallet_ops::{cleanup_subwallet_atas, consolidate_wallets, get_wallets_summary};
