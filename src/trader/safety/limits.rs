//! Position and trade limits enforcement

use crate::trader::config;

/// Check if we can open a new position based on limits
pub async fn check_position_limits() -> Result<bool, String> {
    // TODO: Integrate with positions module when ready
    // For now, always allow (up to max)
    let max_positions = config::get_max_open_positions();
    
    // Placeholder: assume 0 open positions for now
    Ok(true)
}

/// Check if a specific token already has an open position
pub async fn has_open_position(_mint: &str) -> Result<bool, String> {
    // TODO: Integrate with positions module when ready
    // For now, always return false (no open positions)
    Ok(false)
}

/// Check if a token is in re-entry cooldown
pub async fn is_in_reentry_cooldown(_mint: &str) -> Result<bool, String> {
    // TODO: Integrate with positions module when ready
    let cooldown_minutes = config::get_position_close_cooldown_minutes();
    if cooldown_minutes == 0 {
        return Ok(false);
    }

    // For now, always return false (no cooldown)
    Ok(false)
}
