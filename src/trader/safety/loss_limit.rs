//! Period-based loss limit protection
//!
//! Tracks cumulative realized losses over configurable time periods (1h, 6h, 24h).
//! When losses exceed the configured limit, entry monitor is paused while exit
//! monitor continues to manage open positions.

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use std::sync::RwLock;

use crate::logger::{self, LogTag};
use crate::trader::config;

/// Loss limit state tracking
#[derive(Debug, Clone, serde::Serialize)]
pub struct LossLimitState {
    /// Current period start time
    pub period_start: DateTime<Utc>,
    /// Cumulative realized loss in SOL (absolute value)
    pub cumulative_loss_sol: f64,
    /// Whether trading is paused due to loss limit
    pub is_limited: bool,
    /// Timestamp when limit was hit (if limited)
    pub limited_at: Option<DateTime<Utc>>,
    /// Remaining time in current period (seconds)
    pub period_remaining_secs: i64,
}

/// Global loss limit state
static LOSS_LIMIT_STATE: Lazy<RwLock<LossLimitStateInternal>> = Lazy::new(|| {
    RwLock::new(LossLimitStateInternal {
        period_start: Utc::now(),
        cumulative_loss_sol: 0.0,
        is_limited: false,
        limited_at: None,
    })
});

#[derive(Debug, Clone)]
struct LossLimitStateInternal {
    period_start: DateTime<Utc>,
    cumulative_loss_sol: f64,
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
            cumulative_loss_sol: state.cumulative_loss_sol,
            is_limited: state.is_limited,
            limited_at: state.limited_at,
            period_remaining_secs: remaining,
        }
    } else {
        LossLimitState {
            period_start: Utc::now(),
            cumulative_loss_sol: 0.0,
            is_limited: false,
            limited_at: None,
            period_remaining_secs: 0,
        }
    }
}

/// Record a realized loss from a closed position
/// Called when a position is closed with negative P&L
pub fn record_realized_loss(loss_sol: f64) {
    if !config::is_loss_limit_enabled() {
        return;
    }

    let loss_amount = loss_sol.abs();
    let limit = config::get_loss_limit_sol();

    if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
        state.cumulative_loss_sol += loss_amount;

        logger::debug(
            LogTag::Trader,
            &format!(
                "Loss recorded: -{:.4} SOL, cumulative: {:.4}/{:.4} SOL",
                loss_amount, state.cumulative_loss_sol, limit
            ),
        );

        // Check if limit exceeded
        if !state.is_limited && state.cumulative_loss_sol >= limit {
            state.is_limited = true;
            state.limited_at = Some(Utc::now());

            logger::warning(
                LogTag::Trader,
                &format!(
                    "LOSS LIMIT REACHED: {:.4}/{:.4} SOL - Entry monitor paused",
                    state.cumulative_loss_sol, limit
                ),
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
        state.cumulative_loss_sol = 0.0;
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
            state.cumulative_loss_sol = 0.0;

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

/// Initialize loss limit state on startup
/// Uses get_period_trading_stats to calculate losses since period start
pub async fn initialize_from_history() {
    if !config::is_loss_limit_enabled() {
        return;
    }

    let period_hours = config::get_loss_limit_period_hours();
    let period_start = Utc::now() - Duration::hours(period_hours as i64);

    match crate::positions::get_period_trading_stats(period_start, None).await {
        Ok(stats) => {
            let loss = stats.loss_sol; // Already absolute value
            let limit = config::get_loss_limit_sol();

            if let Ok(mut state) = LOSS_LIMIT_STATE.write() {
                state.period_start = period_start;
                state.cumulative_loss_sol = loss;

                if loss >= limit {
                    state.is_limited = true;
                    state.limited_at = Some(Utc::now());
                    logger::warning(
                        LogTag::Trader,
                        &format!(
                            "Loss limit active from startup: {:.4}/{:.4} SOL",
                            loss, limit
                        ),
                    );
                } else {
                    logger::info(
                        LogTag::Trader,
                        &format!(
                            "Loss limit initialized: {:.4}/{:.4} SOL in current period",
                            loss, limit
                        ),
                    );
                }
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Trader,
                &format!("Failed to initialize loss limit from history: {}", e),
            );
        }
    }
}
