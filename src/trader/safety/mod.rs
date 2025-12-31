//! Safety systems for trading

mod blacklist;
mod cooldown;
mod limits;
pub mod loss_limit;
mod risk;

pub use blacklist::{check_blacklist_exit, is_blacklisted};
pub use cooldown::is_in_reentry_cooldown;
pub use limits::{check_position_limits, has_open_position};
pub use loss_limit::*;
pub use risk::check_risk_limits;

use crate::logger::{self, LogTag};

/// Initialize the safety system
pub async fn init_safety_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing safety system...");

    // Initialize loss limit state from historical data (survives restart)
    loss_limit::initialize_from_history().await;

    logger::info(LogTag::Trader, "Safety system initialized");
    Ok(())
}
