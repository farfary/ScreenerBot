//! Safety systems for trading

mod blacklist;
mod limits;
mod risk;

pub use blacklist::{check_blacklist_exit, is_blacklisted};
pub use limits::{check_position_limits, has_open_position, is_in_reentry_cooldown};
pub use risk::check_risk_limits;

use crate::logger::{log, LogTag};

/// Initialize the safety system
pub async fn init_safety_system() -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Initializing safety system...");

    // Initialize blacklist cache
    blacklist::init_blacklist().await?;

    log(LogTag::Trader, "INFO", "Safety system initialized");
    Ok(())
}
