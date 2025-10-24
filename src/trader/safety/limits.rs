//! Position and trade limits enforcement

use crate::positions;
use crate::trader::config;
use chrono::Utc;

/// Check if we can open a new position based on limits
pub async fn check_position_limits() -> Result<bool, String> {
    let max_positions = config::get_max_open_positions();
    let open_positions = positions::get_open_positions().await;

    // Check if we're under the limit
    Ok(open_positions.len() < max_positions)
}

/// Check if a specific token already has an open position
/// This includes checking pending-open flags to prevent race conditions
pub async fn has_open_position(mint: &str) -> Result<bool, String> {
    // Use positions module's is_open_position which checks both actual positions
    // and pending-open flags to prevent concurrent duplicate entries
    Ok(positions::is_open_position(mint).await)
}

/// Check if a token is in re-entry cooldown
pub async fn is_in_reentry_cooldown(mint: &str) -> Result<bool, String> {
    let cooldown_minutes = config::get_position_close_cooldown_minutes();
    if cooldown_minutes == 0 {
        return Ok(false);
    }

    // Check if there's a closed position within cooldown period
    if let Ok(Some(position)) = positions::db::get_position_by_mint(mint).await {
        // Only check cooldown if position was closed (has exit time)
        if let Some(exit_time) = position.exit_time {
            let elapsed = Utc::now().signed_duration_since(exit_time).num_minutes();
            if elapsed < cooldown_minutes as i64 {
                return Ok(true); // Still in cooldown
            }
        }
    }

    Ok(false)
}
