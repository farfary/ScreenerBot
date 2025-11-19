//! Safety systems for trading

mod blacklist;
mod cooldown;
mod limits;
mod risk;

pub use blacklist::{check_blacklist_exit, is_blacklisted};
pub use cooldown::is_in_reentry_cooldown;
pub use limits::{check_position_limits, has_open_position};
pub use risk::check_risk_limits;

use crate::logger::{self, LogTag};

/// Initialize the safety system
pub async fn init_safety_system() -> Result<(), String> {
    logger::info(LogTag::Trader, "Initializing safety system...");
    logger::info(LogTag::Trader, "Safety system initialized");
    Ok(())
}
