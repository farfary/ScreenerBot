//! Database module for ScreenerBot
//!
//! This module provides a well-organized database layer with separate concerns:
//! - Connection management and table initialization
//! - Token operations and queries
//! - Price data management
//! - Pool information handling
//! - Statistics tracking
//! - Blacklist management
//!
//! ## Usage
//!
//! ```rust
//! use screenerbot::database::{Database, DatabaseConfig};
//!
//! // Create database with default configuration
//! let db = Database::new("screener.db")?;
//!
//! // Or with custom configuration
//! let config = DatabaseConfig {
//!     path: "custom.db".to_string(),
//!     pool_size: Some(20),
//!     timeout_seconds: Some(60),
//! };
//! let db = Database::with_config(&config)?;
//! ```

pub mod models;
pub mod connection;
pub mod mints;
pub mod tokens;
pub mod prices;
pub mod pools;
pub mod stats;
pub mod blacklist;

// Re-export the main types for easier access
pub use connection::Database;
pub use models::{
    DatabaseConfig,
    DatabaseResult,
    DatabaseStats,
    QueryParams,
    TrackedToken,
    TokenPriority,
    BlacklistedToken,
};

// Re-export statistics types
pub use stats::{ DiscoveryStatsSummary, DiscoveryTrend };
pub use prices::PriceStatistics;
pub use pools::PoolStatistics;
pub use blacklist::BlacklistStatistics;

// Re-export the original TrackedToken struct for backward compatibility
pub use models::TrackedToken as TrackedTokenModel;

/// Database module version
pub const VERSION: &str = "1.0.0";

/// Default database configuration
pub fn default_config() -> DatabaseConfig {
    DatabaseConfig::default()
}

/// Initialize database with default settings
pub fn init_database(path: &str) -> DatabaseResult<Database> {
    Database::new(path)
}

/// Initialize database with custom configuration
pub fn init_database_with_config(config: &DatabaseConfig) -> DatabaseResult<Database> {
    Database::with_config(config)
}
