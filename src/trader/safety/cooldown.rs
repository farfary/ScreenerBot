// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Re-entry cooldown management
//!
//! Prevents immediate re-entry after exiting a position.
//! Uses positions module to check when a position was last exited.

use crate::positions::{self, Book};
use crate::trader::config;
use chrono::Utc;

/// Check if a token is in re-entry cooldown on `book`
///
/// Returns true if the token was recently exited on that book and is still within the
/// cooldown period; each book cools down on its own exits.
/// Cooldown period is configured via `position_close_cooldown_minutes`.
///
/// Logic:
/// - If cooldown is 0, no cooldown is enforced (returns false)
/// - Otherwise, checks if token's last exit was within cooldown period
pub async fn is_in_reentry_cooldown(book: Book, mint: &str) -> crate::trader::Result<bool> {
    let cooldown_minutes = config::get_position_close_cooldown_minutes();
    if cooldown_minutes == 0 {
        return Ok(false); // Cooldown disabled
    }

    // Check if there's a closed position of the book within cooldown period
    if let Ok(Some(position)) = positions::books::latest_by_mint(book, mint).await {
        // Only check cooldown if position was closed (has exit time)
        if let Some(exit_time) = position.exit_time {
            let elapsed = Utc::now().signed_duration_since(exit_time).num_minutes();
            if elapsed < cooldown_minutes as i64 {
                return Ok(true); // Still in cooldown
            }
        }
    }

    Ok(false) // Not in cooldown
}
