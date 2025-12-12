//! Volume Aggregator Tool
//!
//! Generates trading volume for tokens using multiple wallets.
//! Distributes transactions across secondary wallets to create
//! organic-looking trading activity.
//!
//! ## Usage
//! ```rust,ignore
//! use screenerbot::tools::{VolumeAggregator, VolumeConfig};
//! use screenerbot::tools::{DelayConfig, SizingConfig, DistributionStrategy};
//! use solana_sdk::pubkey::Pubkey;
//!
//! let config = VolumeConfig::new(token_pubkey, 10.0)
//!     .with_delay(DelayConfig::random(1000, 3000))
//!     .with_sizing(SizingConfig::random(0.1, 0.5))
//!     .with_strategy(DistributionStrategy::RoundRobin)
//!     .with_num_wallets(5);
//!
//! let mut aggregator = VolumeAggregator::new(config);
//! aggregator.prepare().await?;
//! let session = aggregator.execute().await?;
//! ```

mod executor;
pub mod strategies;
mod types;

// Re-export types
pub use executor::VolumeAggregator;
pub use strategies::{calculate_amount, calculate_amount_clamped, calculate_delay, StrategyExecutor};
pub use types::{SessionStatus, TransactionStatus, VolumeConfig, VolumeSession, VolumeTransaction};
