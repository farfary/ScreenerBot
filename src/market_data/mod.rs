//! Market Data Module
//!
//! This module provides comprehensive market data functionality for cryptocurrency tokens,
//! including price tracking, caching, and various data sources integration.
//!
//! ## Structure
//!
//! - `models/` - Data structures and types
//! - `sources/` - Price data sources (GeckoTerminal, Pool calculations)
//! - `pricing/` - Pricing logic, caching, and management
//! - `decoders/` - Pool data decoders for different DEX protocols
//!
//! ## Usage
//!
//! ```rust
//! use crate::market_data::{PricingManager, PricingConfig};
//!
//! let config = PricingConfig::default();
//! let manager = PricingManager::new(database, logger, config);
//! manager.start().await;
//!
//! // Get token price
//! let price = manager.get_token_price("token_address").await;
//! ```

pub mod models;
pub mod sources;
pub mod pricing;
pub mod decoders;
pub mod pool_decoders;

// Re-export commonly used types and functions
pub use models::*;
pub use sources::*;
pub use pricing::*;
pub use decoders::{ PoolDecoder, PoolDecoderError, DecodedPoolData };
pub use pool_decoders::PoolDecoderManager;

// Legacy compatibility - these will be deprecated
pub use models::{ TokenPrice, TokenInfo, PoolInfo, PoolType, PriceSource };
pub use pricing::{ PricingManager, PriceCache };
pub use sources::{ GeckoTerminalClient, PoolPriceCalculator };

/// Market data module version
pub const VERSION: &str = "2.0.0";

/// Supported DEX protocols
pub const SUPPORTED_DEXES: &[&str] = &["Raydium", "PumpFun", "Meteora", "Orca", "Serum"];

/// Default configuration for market data operations
pub fn default_config() -> PricingConfig {
    PricingConfig::default()
}

/// Create a new pricing manager with default configuration
pub fn create_pricing_manager(
    database: std::sync::Arc<crate::database::Database>,
    logger: std::sync::Arc<crate::logger::Logger>
) -> PricingManager {
    PricingManager::new(database, logger, default_config())
}

/// Utility function to validate token address format
pub fn is_valid_token_address(address: &str) -> bool {
    address.len() >= 32 && address.len() <= 44 && address.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Utility function to format price for display
pub fn format_price(price: f64) -> String {
    if price >= 1000.0 {
        format!("${:.2}", price)
    } else if price >= 1.0 {
        format!("${:.4}", price)
    } else if price >= 0.01 {
        format!("${:.6}", price)
    } else {
        format!("${:.8}", price)
    }
}

/// Utility function to format volume for display
pub fn format_volume(volume: f64) -> String {
    if volume >= 1_000_000.0 {
        format!("${:.1}M", volume / 1_000_000.0)
    } else if volume >= 1_000.0 {
        format!("${:.1}K", volume / 1_000.0)
    } else {
        format!("${:.0}", volume)
    }
}

/// Utility function to format liquidity for display
pub fn format_liquidity(liquidity: f64) -> String {
    if liquidity >= 1_000_000.0 {
        format!("${:.1}M", liquidity / 1_000_000.0)
    } else if liquidity >= 1_000.0 {
        format!("${:.1}K", liquidity / 1_000.0)
    } else {
        format!("${:.0}", liquidity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_token_address() {
        assert!(is_valid_token_address("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));
        assert!(is_valid_token_address("So11111111111111111111111111111111111111112"));
        assert!(!is_valid_token_address("invalid"));
        assert!(!is_valid_token_address(""));
    }

    #[test]
    fn test_format_price() {
        assert_eq!(format_price(1234.56), "$1234.56");
        assert_eq!(format_price(12.3456), "$12.3456");
        assert_eq!(format_price(0.123456), "$0.123456");
        assert_eq!(format_price(0.00123456), "$0.00123456");
    }

    #[test]
    fn test_format_volume() {
        assert_eq!(format_volume(1_234_567.0), "$1.2M");
        assert_eq!(format_volume(12_345.0), "$12.3K");
        assert_eq!(format_volume(123.0), "$123");
    }

    #[test]
    fn test_format_liquidity() {
        assert_eq!(format_liquidity(1_234_567.0), "$1.2M");
        assert_eq!(format_liquidity(12_345.0), "$12.3K");
        assert_eq!(format_liquidity(123.0), "$123");
    }

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert_eq!(config.update_interval_secs, 60);
        assert_eq!(config.top_tokens_count, 1000);
        assert_eq!(config.cache_ttl_secs, 300);
        assert_eq!(config.max_cache_size, 10000);
        assert!(!config.enable_dynamic_pricing);
    }
}
