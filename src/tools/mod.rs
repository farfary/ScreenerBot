//! Tools module for ScreenerBot
//!
//! Contains utility tools like volume aggregator for token operations.
//!
//! ## Available Tools
//! - `volume_aggregator` - Generate trading volume using multiple wallets

mod types;
pub mod volume_aggregator;

// Re-export common types
pub use types::{ToolResult, ToolStatus};

// Re-export volume aggregator
pub use volume_aggregator::{VolumeAggregator, VolumeConfig, VolumeSession, VolumeTransaction};
