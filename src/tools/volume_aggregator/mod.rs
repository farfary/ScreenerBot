//! Volume Aggregator Tool
//!
//! Generates trading volume for tokens using multiple wallets.
//! Distributes transactions across secondary wallets to create
//! organic-looking trading activity.
//!
//! ## Usage
//! ```rust,ignore
//! use screenerbot::tools::{VolumeAggregator, VolumeConfig};
//! use solana_sdk::pubkey::Pubkey;
//!
//! let config = VolumeConfig {
//!     token_mint: token_pubkey,
//!     total_volume_sol: 10.0,
//!     num_wallets: 5,
//!     min_amount_sol: 0.1,
//!     max_amount_sol: 0.5,
//!     delay_between_ms: 2000,
//!     randomize_amounts: true,
//! };
//!
//! let mut aggregator = VolumeAggregator::new(config);
//! aggregator.prepare().await?;
//! let session = aggregator.execute().await?;
//! ```

mod executor;
mod types;

// Re-export types
pub use executor::VolumeAggregator;
pub use types::{VolumeConfig, VolumeSession, VolumeTransaction, TransactionStatus};
