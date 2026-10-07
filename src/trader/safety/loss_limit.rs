// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Period-based loss limit protection
//!
//! Tracks cumulative realized losses over configurable time periods (1h, 6h, 24h).
//! When losses exceed the configured limit, entry monitor is paused while exit
//! monitor continues to manage open positions.

use chrono::{DateTime, Duration, Utc};
use std::sync::LazyLock;
use std::sync::RwLock;

use crate::logger::{self, LogTag};
use crate::trader::config;

/// Loss limit state tracking
#[derive(Debug, Clone, serde::Serialize)]
pub struct LossLimitState {
    /// Current period start time
    pub period_start: DateTime<Utc>,
    /// Cumulative realized loss in SOL (absolute value)
    pub cumulative_loss_native: f64,
    /// Whether trading is paused due to loss limit
    pub is_limited: bool,
    /// Timestamp when limit was hit (if limited)
    pub limited_at: Option<DateTime<Utc>>,
    /// Remaining time in current period (seconds)
    pub period_remaining_secs: i64,
}

/// Global loss limit state
static LOSS_LIMIT_STATE: LazyLock<RwLock<LossLimitStateInternal>> = LazyLock::new(|| {
    RwLock::new(LossLimitStateInternal {
        period_start: Utc::now(),
        counted_since: Utc::now(),
        cumulative_loss_native: 0.0,
        is_limited: false,
        limited_at: None,
    })
});

#[derive(Debug, Clone)]
struct LossLimitStateInternal {
    period_start: DateTime<Utc>,
    /// Where the window the figure counts begins: the period start, or for the period that
    /// begins at startup, one period earlier, so losses booked before a restart still count.
    counted_since: DateTime<Utc>,
    cumulative_loss_native: f64,
    is_limited: bool,
    limited_at: Option<DateTime<Utc>>,
}

/// Check if entry is blocked due to loss limit
/// Called by entry evaluator before evaluating any token
pub fn is_entry_blocked_by_loss_limit() -> bool {
    // If loss limit not enabled, never block
    if !config::is_loss_limit_enabled() {
        return false;
    }

    // Check if period has reset, and if so, reset state
    check_and_reset_period_if_needed();

    // Check current state
    if let Ok(state) = LOSS_LIMIT_STATE.read() {
        state.is_limited
    } else {
        false
    }
}

/// Get current loss limit status for dashboard display
pub fn get_loss_limit_status() -> LossLimitState {
    check_and_reset_period_if_needed();

    let period_hours = config::get_loss_limit_period_hours();

    if let Ok(state) = LOSS_LIMIT_STATE.read() {
        let period_end = state.period_start + Duration::hours(period_hours as i64);
        let remaining = (period_end - Utc::now()).num_seconds().max(0);

        LossLimitState {
            period_start: state.period_start,
            cumulative_loss_native: state.cumulative_loss_native,
            is_limited: state.is_limited,
            limited_at: state.limited_at,
            period_remaining_secs: remaining,
        }
    } else {
        LossLimitState {
            period_start: Utc::now(),
            cumulative_loss_native: 0.0,
            is_limited: false,
            limited_at: None,
            period_remaining_secs: 0,
        }
    }
}

/// Serialises [`sync_from_books`], so a recompute that read the books earlier never writes
/// its figure after one that read them later.
static BOOKS_SYNC: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Sets the period's loss to what the books realized in the window it counts: the loss of
/// the bot's closed positions, derived the same way at startup by
/// [`initialize_from_history`], so a restart finds the figure the live limiter holds. A
/// figure that rises to the limit pauses entries; one that did not rise never undoes a
/// manual resume. A correction that lowers the figure never lifts a pause: resuming stays a
/// deliberate step, never a side effect of fixing the books. When the books cannot be read,
/// the figure and the pause stay as they were.
pub async fn sync_from_books() {
    if !config::is_loss_limit_enabled() {
        return;
    }
    let _serial = BOOKS_SYNC.lock().await;
    check_and_reset_period_if_needed();

    let Some(counted_since) = LOSS_LIMIT_STATE
        .read()
        .ok()
        .map(|state| state.counted_since)
    else {
        return;
    };
    let loss = match crate::positions::get_period_trading_stats(counted_since, None).await {
        Ok(stats) => stats.loss_native,
        Err(e) => {
            logger::warning(
                LogTag::Trader,
                &format!("Loss limit kept its figure: the books could not be read: {e}"),
            );
            return;
        }
    };
    let limit = config::get_loss_limit_native();

    if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
        // A period that rolled over while the books were read starts from its own figure.
        if state.counted_since != counted_since {
            return;
        }
        let rose = loss > state.cumulative_loss_native;
        state.cumulative_loss_native = loss;
        logger::debug(
            LogTag::Trader,
            &format!("Loss limit follows the books: {loss:.4}/{limit:.4} SOL"),
        );

        // Only a figure that rose can pause: books that did not move never undo a manual
        // resume.
        if !state.is_limited && rose && loss >= limit {
            state.is_limited = true;
            state.limited_at = Some(Utc::now());

            logger::warning(
                LogTag::Trader,
                &format!("LOSS LIMIT REACHED: {loss:.4}/{limit:.4} SOL - Entry monitor paused"),
            );
        }
    }
}

/// Manually resume trading after loss limit (for dashboard control)
pub fn resume_from_loss_limit() {
    if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
        if state.is_limited {
            state.is_limited = false;
            state.limited_at = None;
            logger::info(
                LogTag::Trader,
                "Loss limit manually resumed - entries enabled",
            );
        }
    }
}

/// Reset loss limit state (for new period or manual reset)
pub fn reset_loss_limit_state() {
    if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
        state.period_start = Utc::now();
        state.counted_since = state.period_start;
        state.cumulative_loss_native = 0.0;
        state.is_limited = false;
        state.limited_at = None;
        logger::info(
            LogTag::Trader,
            "Loss limit state reset - new period started",
        );
    }
}

/// Check if period has elapsed and reset if needed
fn check_and_reset_period_if_needed() {
    let period_hours = config::get_loss_limit_period_hours();
    let auto_resume = config::is_loss_limit_auto_resume();

    if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
        let period_end = state.period_start + Duration::hours(period_hours as i64);

        if Utc::now() >= period_end {
            let was_limited = state.is_limited;

            // Reset is_limited first to minimize race window with readers
            if auto_resume {
                state.is_limited = false;
                state.limited_at = None;
            }

            // Then reset period data
            state.period_start = Utc::now();
            state.counted_since = state.period_start;
            state.cumulative_loss_native = 0.0;

            // NOTE: Race window between write() release and next read() is negligible.
            // All state changes happen atomically within single write lock scope.

            if auto_resume && was_limited {
                logger::info(
                    LogTag::Trader,
                    "Loss limit period reset - auto-resumed entries",
                );
            } else if was_limited {
                logger::info(
                    LogTag::Trader,
                    "Loss limit period reset - manual resume required",
                );
            }
        }
    }
}

/// Initialize loss limit state on startup. A period starts now, and its figure counts the
/// losses of the period before it as well: a restart neither forgets them nor restarts the
/// budget. The figure comes from the books exactly as [`sync_from_books`] derives it.
pub async fn initialize_from_history() {
    if !config::is_loss_limit_enabled() {
        return;
    }

    let period_hours = config::get_loss_limit_period_hours();
    if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
        state.period_start = Utc::now();
        state.counted_since = state.period_start - Duration::hours(period_hours as i64);
    }
    sync_from_books().await;

    let status = get_loss_limit_status();
    let limit = config::get_loss_limit_native();
    if status.is_limited {
        logger::warning(
            LogTag::Trader,
            &format!(
                "Loss limit active from startup: {:.4}/{:.4} SOL",
                status.cumulative_loss_native, limit
            ),
        );
    } else {
        logger::info(
            LogTag::Trader,
            &format!(
                "Loss limit initialized: {:.4}/{:.4} SOL in current period",
                status.cumulative_loss_native, limit
            ),
        );
    }
}
