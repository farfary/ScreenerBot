//! Position and trade limits enforcement

use crate::positions;
use crate::trader::config;

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
