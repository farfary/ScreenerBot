// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position and trade limits enforcement.

use crate::positions;
use crate::trader::config;

/// Check if we can open a new position based on limits
pub async fn check_position_limits() -> crate::trader::Result<bool> {
    let max_positions = config::get_max_open_positions();
    // Wallet-derived rounds are the user's pre-existing holdings, not trades we placed,
    // so they must not eat the trader's capacity.
    let open_positions = positions::state::get_capacity_consuming_positions().await;

    // Check if we're under the limit
    Ok(open_positions.len() < max_positions)
}

/// Whether a token is held: it has an open position, archived ones included, or an open
/// is pending, so the bot never enters a token it already holds.
pub async fn has_open_position(mint: &str) -> crate::trader::Result<bool> {
    Ok(positions::holds_open_round(mint).await)
}
