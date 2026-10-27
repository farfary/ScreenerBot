// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Position and trade limits enforcement.

use crate::positions;
use crate::trader::config;

/// The trader's position capacity: the open positions that consume it and the configured
/// maximum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionCapacity {
    pub used: usize,
    pub limit: usize,
}

impl PositionCapacity {
    /// Whether another position may open.
    pub const fn has_room(self) -> bool {
        self.used < self.limit
    }
}

/// The open positions counted against the limit, and the limit.
pub async fn position_capacity() -> PositionCapacity {
    // Wallet-derived rounds are the user's pre-existing holdings, not trades we placed,
    // so they must not eat the trader's capacity.
    let used = positions::state::get_capacity_consuming_positions()
        .await
        .len();
    PositionCapacity {
        used,
        limit: config::get_max_open_positions(),
    }
}

/// Check if we can open a new position based on limits
pub async fn check_position_limits() -> crate::trader::Result<bool> {
    Ok(position_capacity().await.has_room())
}

/// Whether a token is held: it has an open position, archived ones included, or an open
/// is pending, so the bot never enters a token it already holds.
pub async fn has_open_position(mint: &str) -> crate::trader::Result<bool> {
    Ok(positions::holds_open_round(mint).await)
}
