//! profit.rs
//!
//! Streamlined profit-taking & exit decision module.
//!
//! Goals & changes:
//! * Keep surface area small: single public async `should_sell(position, current_price)`.
//! * Add "time pressure" behaviour: as a position nears MAX_HOLD_MINUTES the system
//!   becomes more aggressive about exiting (tightens trailing, lowers odds threshold).
//! * Improve numeric stability and logging clarity.
//! * Keep primitives simple and well-documented.
//!
//! External expectations:
//! * `Position` type and `calculate_position_pnl(position, Some(price)).await` exist in `crate::positions`.
//! * `crate::logger::log` and `crate::global::is_debug_profit_enabled()` are available.

use crate::global::*;
use crate::logger::{log, LogTag};
use crate::positions_lib::calculate_position_pnl;
use crate::positions_types::Position;
use chrono::Utc;

/// ============================= Tunables =============================

// Loss & risk (percent)
pub const STOP_LOSS_PERCENT: f64 = -40.0; // Hard kill after initial grace
pub const EXTREME_LOSS_PERCENT: f64 = -55.0; // Emergency immediate kill

// Time (minutes)
pub const MAX_HOLD_MINUTES: f64 = 120.0; // Absolute maximum hold duration

// Profit ladders (percent)
pub const BASE_MIN_PROFIT_PERCENT: f64 = 10.0; // Minimum gate to consider discretionary sells
pub const INSTANT_EXIT_LEVEL_1: f64 = 100.0; // Strong immediate take
pub const INSTANT_EXIT_LEVEL_2: f64 = 150.0; // Very strong immediate take

// Trailing stop dynamics (gaps are in percentage points of profit)
pub const TRAIL_MIN_GAP: f64 = 5.0; // Tightest trailing gap (% of profit)
pub const TRAIL_MAX_GAP: f64 = 35.0; // Widest trailing gap

// Trailing tighten schedule (minutes)
pub const TRAIL_TIGHTEN_START: f64 = 45.0;
pub const TRAIL_TIGHTEN_FULL: f64 = 90.0;

// Odds model threshold
pub const EXIT_ODDS_THRESHOLD: f64 = 0.65; // below this, favor exiting when EV not positive

// Quick capture windows: (minutes, required profit %)
const QUICK_WINDOWS: &[(f64, f64)] = &[(1.0, 30.0), (5.0, 50.0), (15.0, 80.0)];

/// ============================= Helpers =============================

#[inline]
fn clamp01(v: f64) -> f64 {
    if v.is_finite() {
        v.max(0.0).min(1.0)
    } else {
        0.0
    }
}

/// Compute a dynamic trailing gap (in percentage points) from peak profit and time held.
///
/// - For non-positive peak profit we return TRAIL_MIN_GAP to keep protection tight.
/// - Gap grows with profit but is clamped to TRAIL_MIN_GAP..TRAIL_MAX_GAP.
/// - As time passes beyond TRAIL_TIGHTEN_START the gap reduces up to 30% by TRAIL_TIGHTEN_FULL.
/// - Returned value rounded to 2 decimals to reduce log churn.
pub fn trailing_gap(peak_profit: f64, minutes_held: f64) -> f64 {
    // Sanitize inputs
    let minutes = if minutes_held.is_finite() && minutes_held > 0.0 {
        minutes_held
    } else {
        0.0
    };

    if !peak_profit.is_finite() || peak_profit <= 0.0 {
        return TRAIL_MIN_GAP;
    }

    // Base: higher profits allow wider absolute gaps (but proportionally smaller).
    // We use piecewise factors to avoid wild swings in the small profit region.
    let mut gap = if peak_profit < 20.0 {
        (peak_profit * 0.40)
    } else if peak_profit < 50.0 {
        (peak_profit * 0.30)
    } else if peak_profit < 100.0 {
        (peak_profit * 0.25)
    } else {
        (peak_profit * 0.20)
    };

    // Clamp
    gap = gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP);

    // Time tightening: from TRAIL_TIGHTEN_START -> TRAIL_TIGHTEN_FULL reduce gap by up to 30%
    if minutes >= TRAIL_TIGHTEN_START && TRAIL_TIGHTEN_FULL > TRAIL_TIGHTEN_START {
        let progress = ((minutes - TRAIL_TIGHTEN_START)
            / (TRAIL_TIGHTEN_FULL - TRAIL_TIGHTEN_START))
            .clamp(0.0, 1.0);
        let shrink = 0.30 * progress;
        gap *= 1.0 - shrink;
    }

    // Safety clamp + rounding to 2 decimals
    ((gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP)) * 100.0).round() / 100.0
}

/// A simple continuation odds estimator (0.0..1.0).
///
/// - Odds decay with holding time and current profit (diminishing returns).
/// - Very quick, large moves get a temporary early_boost.
pub fn continuation_odds(profit_percent: f64, minutes_held: f64) -> f64 {
    let minutes = if minutes_held.is_finite() && minutes_held >= 0.0 {
        minutes_held
    } else {
        0.0
    };

    // Minimal floor - a slight edge to holding small profiting positions
    if profit_percent <= 0.0 {
        return 0.55;
    }

    // Time decay: longer held -> exponentially smaller chance of another big leg
    let time_decay = (-minutes / 50.0).exp();

    // Profit decay: higher profit reduces odds of another big leg (non-linear)
    let profit_decay = (-(profit_percent / 120.0).powf(1.1)).exp();

    // Early boost for very fast moves
    let early_boost = if minutes < 5.0 && profit_percent > 40.0 {
        0.10
    } else {
        0.0
    };

    (time_decay * profit_decay + early_boost).clamp(0.0, 1.0)
}

/// ============================= Main Decision Function =============================

/// Decide whether to sell (`true`) or continue holding (`false`).
///
/// The function is intentionally conservative about selling below `BASE_MIN_PROFIT_PERCENT`.
/// Time pressure makes the system more aggressive as we approach `MAX_HOLD_MINUTES`.
pub async fn should_sell(position: &Position, current_price: f64) -> bool {
    // Basic validation
    if !current_price.is_finite() || current_price <= 0.0 {
        return false;
    }

    let entry = position
        .effective_entry_price
        .unwrap_or(position.entry_price);
    if !entry.is_finite() || entry <= 0.0 {
        return false;
    }

    // PnL calculation (support SIM/test tokens)
    let (pnl_sol, pnl_percent) = if position.symbol == "SIM" || position.mint == "SIM" {
        let pnl_percent = ((current_price - entry) / entry) * 100.0;
        let pnl_sol = if entry != 0.0 {
            ((current_price - entry) * position.entry_size_sol) / entry
        } else {
            0.0
        };
        (pnl_sol, pnl_percent)
    } else {
        // Note: calculate_position_pnl is expected to be async and return (sol_value, percent)
        calculate_position_pnl(position, Some(current_price)).await
    };

    if !pnl_percent.is_finite() || !pnl_sol.is_finite() {
        return false;
    }

    // Time held in minutes (clamp >= 0)
    let minutes_held = {
        let secs = (Utc::now() - position.entry_time).num_seconds();
        let m = (secs as f64) / 60.0;
        if m.is_sign_negative() {
            0.0
        } else {
            m
        }
    };

    // Peak profit (percentage)
    let highest = position.price_highest.max(current_price);
    let peak_profit = if entry > 0.0 {
        ((highest - entry) / entry) * 100.0
    } else {
        0.0
    };

    // Drawdown from peak (how far we've come off the peak)
    let drawdown = peak_profit - pnl_percent;

    // Time pressure factor [0..1], increases as we approach the absolute MAX_HOLD_MINUTES
    let time_pressure = clamp01(minutes_held / MAX_HOLD_MINUTES);

    // If nearing the end, increase aggressiveness: reduce allowed gaps and reduce odds threshold.
    // We compute an adaptive odds threshold and a time-pressure multiplier for trailing gaps.
    let adaptive_odds_threshold = {
        // start reducing threshold moderately when time_pressure > 0.6
        if time_pressure <= 0.6 {
            EXIT_ODDS_THRESHOLD
        } else {
            // Move linearly down to 0.50 at full pressure (forces more exits)
            let factor = 1.0 - ((time_pressure - 0.6) / (1.0 - 0.6)); // 1 -> 0 as pressure increases
            let lowered = 0.50 + 0.15 * factor; // ranges roughly 0.50 .. 0.65
            lowered.clamp(0.45, EXIT_ODDS_THRESHOLD)
        }
    };

    let trailing_time_pressure_multiplier = 1.0 - (time_pressure * 0.35); // up to 35% tighter trailing gaps

    // 1) Extreme loss immediate kill (no questions)
    if pnl_percent <= EXTREME_LOSS_PERCENT {
        return true;
    }

    // 2) Regular stop loss after short grace (avoid spurious micro-exits right at open)
    if pnl_percent <= STOP_LOSS_PERCENT && minutes_held >= 1.0 {
        return true;
    }

    // 3) Absolute time cap: must close
    if minutes_held >= MAX_HOLD_MINUTES {
        return true;
    }

    // 4) Quick capture windows (fast large moves)
    for (window_minutes, required_profit) in QUICK_WINDOWS {
        if minutes_held <= *window_minutes && pnl_percent >= *required_profit {
            return true;
        }
    }

    // 5) Instant large profits - take them immediately
    if pnl_percent >= INSTANT_EXIT_LEVEL_2 {
        return true;
    }
    if pnl_percent >= INSTANT_EXIT_LEVEL_1 {
        // If we've had a meaningful drawdown off the peak or held a while, lock it in
        if drawdown >= 10.0 || minutes_held > 10.0 {
            return true;
        }
    }

    // 6) Gate: do not consider discretionary exits below minimum profit
    if pnl_percent < BASE_MIN_PROFIT_PERCENT {
        // However, if time pressure is very high and we're above a small profit, close to avoid forced cap
        if time_pressure > 0.95 && pnl_percent > 2.0 {
            return true;
        }
        return false;
    }

    // 7) Trailing stop logic (dynamic)
    if peak_profit >= BASE_MIN_PROFIT_PERCENT {
        // baseline gap derived from peak profit and age
        let mut gap = trailing_gap(peak_profit, minutes_held);

        // Apply time-pressure multiplier (tighten gap as approaching max hold)
        gap *= trailing_time_pressure_multiplier;

        // Additional adaptive tightening if we've been showing large drawdown relative to gap
        if minutes_held > 30.0 && drawdown > gap * 0.6 {
            gap = (gap * 0.85).max(TRAIL_MIN_GAP);
        }

        // If drawdown exceeds dynamic gap -> sell
        if drawdown >= gap {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "TRAIL_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% drawdown={:.2}% gap={:.2}% t={:.1}m tp={:.2}",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        drawdown,
                        gap,
                        minutes_held,
                        time_pressure
                    ),
                );
            }
            return true;
        }
    }

    // 8) Odds / expected-value based exit
    let odds = continuation_odds(pnl_percent, minutes_held);

    // Heuristic future upside cap (conservative)
    let potential_gain_ceiling = 200.0;
    let potential_gain = (potential_gain_ceiling - pnl_percent).max(0.0).min(100.0);

    // Future gap is an estimate of how much downside we'd accept as the trailing gap if we continued.
    // Use max(pnl_percent, peak_profit) to avoid underestimating gap in certain edge cases.
    let future_gap = trailing_gap(pnl_percent.max(peak_profit), minutes_held)
        * trailing_time_pressure_multiplier;

    // Expected edge: simplified EV proxy
    let expected_edge = odds * potential_gain - (1.0 - odds) * future_gap;

    // If odds are poor relative to adaptive threshold and EV is non-positive -> exit.
    if odds < adaptive_odds_threshold && expected_edge <= 0.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "ODDS_EXIT",
                &format!(
                    "{} pnl={:.2}% odds={:.2} thr={:.2} edge={:.2} t={:.1}m",
                    position.symbol,
                    pnl_percent,
                    odds,
                    adaptive_odds_threshold,
                    expected_edge,
                    minutes_held
                ),
            );
        }
        return true;
    }

    // 9) Time pressure final nudge — if we are very close to MAX_HOLD_MINUTES and have any decent profit,
    // prefer to exit rather than risk forced closure on the cap.
    if time_pressure > 0.92 && pnl_percent > BASE_MIN_PROFIT_PERCENT {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "TIME_PRESSURE_EXIT",
                &format!(
                    "{} pnl={:.2}% t={:.1}m pressure={:.2}",
                    position.symbol, pnl_percent, minutes_held, time_pressure
                ),
            );
        }
        return true;
    }

    // Final debug logging for transparency
    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "DECISION",
            &format!(
                "{} pnl={:.2}% ({:.6} SOL) peak={:.2}% dd={:.2}% t={:.1}m tp={:.2} odds={:.2} edge={:.2} thr={:.2}",
                position.symbol,
                pnl_percent,
                pnl_sol,
                peak_profit,
                drawdown,
                minutes_held,
                time_pressure,
                odds,
                expected_edge,
                adaptive_odds_threshold
            ),
        );
    }

    // Default: continue holding
    false
}
