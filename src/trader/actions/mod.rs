//! Action tracking for manual and automated trading operations
//!
//! Provides helper functions to create and manage actions for buy/sell/add operations.
//! Actions are tracked through the global actions system and broadcast to the dashboard.

mod auto;
mod manual;

pub use auto::*;
pub use manual::*;

/// Step indices (shared across manual and auto actions)
pub const STEP_VALIDATE: usize = 0;
pub const STEP_QUOTE: usize = 1;
pub const STEP_SWAP: usize = 2;
pub const STEP_VERIFY: usize = 3;
