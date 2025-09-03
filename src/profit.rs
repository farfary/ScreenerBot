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
use crate::logger::{ log, LogTag };
use crate::positions_lib::calculate_position_pnl;
use crate::positions_types::Position;
use chrono::Utc;

/// ============================= Tunables =============================

// Loss & risk (percent) - Tighter for scalping

pub const STOP_LOSS_PERCENT: f64 = -30.0; // Tighter stop loss for scalping (was -40.0)
pub const EXTREME_LOSS_PERCENT: f64 = -45.0; // Emergency exit (was -55.0)

// Time (minutes) - Shorter hold times for scalping
pub const MAX_HOLD_MINUTES: f64 = 90.0; // Reduced from 120.0 for faster scalping

// Profit ladders (percent) - Lower thresholds for scalping
pub const BASE_MIN_PROFIT_PERCENT: f64 = 5.0; // Reduced from 10.0 for scalping
pub const INSTANT_EXIT_LEVEL_1: f64 = 20.0; // Reduced from 100.0 for faster exits
pub const INSTANT_EXIT_LEVEL_2: f64 = 35.0; // Reduced from 150.0 for scalping

// Trailing stop dynamics (tighter for scalping)
pub const TRAIL_MIN_GAP: f64 = 3.0; // Reduced from 5.0 for tighter trailing
pub const TRAIL_MAX_GAP: f64 = 20.0; // Reduced from 35.0 for scalping

// Trailing tighten schedule (faster for scalping)
pub const TRAIL_TIGHTEN_START: f64 = 30.0; // Reduced from 45.0
pub const TRAIL_TIGHTEN_FULL: f64 = 60.0; // Reduced from 90.0

// Odds model threshold (higher for scalping quality)
pub const EXIT_ODDS_THRESHOLD: f64 = 0.7; // Increased from 0.65

// Early-hold protection (seconds)
// Prevents non-emergency exits in the first moments after entry to avoid churn on noise.
// Quick-capture profit exits and extreme/stop-loss still apply.
pub const EARLY_HOLD_GRACE_SECS: f64 = 20.0; // ~45s grace

// Quick capture windows for scalping: (minutes, required profit %)
// More aggressive early capture to handle monitoring delays
const QUICK_WINDOWS: &[(f64, f64)] = &[
    (0.33, 12.0), // 20 seconds: 12% profit (catches spikes between 5s intervals)
    (0.5, 15.0), // 30 seconds: 15% profit
    (1.0, 10.0), // 1 minute: 10% profit (reduced from 12%)
    (2.0, 15.0), // 2 minutes: 15% profit (reduced from 18%)
    (3.0, 20.0), // 3 minutes: 20% profit (reduced from 22%)
    (5.0, 25.0), // 5 minutes: 25% profit
    (8.0, 30.0), // 8 minutes: 30% profit
    (12.0, 35.0), // 12 minutes: 35% profit
];

/// ============================= Helpers =============================

#[inline]
fn clamp01(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0).min(1.0) } else { 0.0 }
}

/// Compute a dynamic trailing gap (in percentage points) from peak profit and time held.
///
/// - For non-positive peak profit we return TRAIL_MIN_GAP to keep protection tight.
/// - Gap grows with profit but is clamped to TRAIL_MIN_GAP..TRAIL_MAX_GAP.
/// - As time passes beyond TRAIL_TIGHTEN_START the gap reduces up to 30% by TRAIL_TIGHTEN_FULL.
/// - Returned value rounded to 2 decimals to reduce log churn.
pub fn trailing_gap(peak_profit: f64, minutes_held: f64) -> f64 {
    // Sanitize inputs
    let minutes = if minutes_held.is_finite() && minutes_held > 0.0 { minutes_held } else { 0.0 };

    if !peak_profit.is_finite() || peak_profit <= 0.0 {
        return TRAIL_MIN_GAP;
    }

    // Base: higher profits allow wider absolute gaps (but proportionally smaller).
    // We use piecewise factors to avoid wild swings in the small profit region.
    let mut gap = if peak_profit < 20.0 {
        // Slightly tighter for small-to-moderate peaks to reduce giveback
        peak_profit * 0.35
    } else if peak_profit < 50.0 {
        peak_profit * 0.25
    } else if peak_profit < 100.0 {
        peak_profit * 0.2
    } else {
        peak_profit * 0.16
    };

    // Clamp
    gap = gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP);

    // Time tightening: from TRAIL_TIGHTEN_START -> TRAIL_TIGHTEN_FULL reduce gap by up to 30%
    if minutes >= TRAIL_TIGHTEN_START && TRAIL_TIGHTEN_FULL > TRAIL_TIGHTEN_START {
        let progress = (
            (minutes - TRAIL_TIGHTEN_START) /
            (TRAIL_TIGHTEN_FULL - TRAIL_TIGHTEN_START)
        ).clamp(0.0, 1.0);
        let shrink = 0.3 * progress;
        gap *= 1.0 - shrink;
    }

    // Safety clamp + rounding to 2 decimals
    (gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP) * 100.0).round() / 100.0
}

/// A simple continuation odds estimator (0.0..1.0).
///
/// - Odds decay with holding time and current profit (diminishing returns).
/// - Very quick, large moves get a temporary early_boost.
pub fn continuation_odds(profit_percent: f64, minutes_held: f64) -> f64 {
    let minutes = if minutes_held.is_finite() && minutes_held >= 0.0 { minutes_held } else { 0.0 };

    // Minimal floor - a slight edge to holding small profiting positions
    if profit_percent <= 0.0 {
        return 0.55;
    }

    // Time decay: longer held -> exponentially smaller chance of another big leg
    let time_decay = (-minutes / 50.0).exp();

    // Profit decay: higher profit reduces odds of another big leg (non-linear)
    let profit_decay = (-(profit_percent / 120.0).powf(1.1)).exp();

    // Early boost for very fast moves
    let early_boost = if minutes < 5.0 && profit_percent > 40.0 { 0.1 } else { 0.0 };

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

    let entry = position.effective_entry_price.unwrap_or(position.entry_price);
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

    // Time held in minutes/seconds (clamp >= 0)
    let (minutes_held, seconds_held) = {
        let secs_i = (Utc::now() - position.entry_time).num_seconds();
        let secs = secs_i.max(0) as f64;
        (secs / 60.0, secs)
    };

    // Early-hold grace flag
    let early_hold_active = seconds_held < EARLY_HOLD_GRACE_SECS;

    // Peak profit (percentage)
    let highest = position.price_highest.max(current_price);
    let peak_profit = if entry > 0.0 { ((highest - entry) / entry) * 100.0 } else { 0.0 };

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
            let factor = 1.0 - (time_pressure - 0.6) / (1.0 - 0.6); // 1 -> 0 as pressure increases
            let lowered = 0.5 + 0.15 * factor; // ranges roughly 0.50 .. 0.65
            lowered.clamp(0.45, EXIT_ODDS_THRESHOLD)
        }
    };

    let trailing_time_pressure_multiplier = 1.0 - time_pressure * 0.35; // up to 35% tighter trailing gaps

    // ============================= Profit Targets Integration =============================
    // Use position-specific targets when available. Keep it simple and local.
    let target_min = position.profit_target_min
        .unwrap_or(BASE_MIN_PROFIT_PERCENT)
        .clamp(1.0, 300.0);
    let mut target_max = position.profit_target_max
        .unwrap_or(INSTANT_EXIT_LEVEL_1)
        .clamp(1.0, 500.0);
    if target_max < target_min {
        target_max = (target_min + 1.0).min(500.0);
    }

    // 1) Extreme loss immediate kill (no questions)
    if pnl_percent <= EXTREME_LOSS_PERCENT {
        return true;
    }

    // 2) Regular stop loss after short grace (avoid spurious micro-exits right at open)
    if pnl_percent <= STOP_LOSS_PERCENT && minutes_held >= 1.0 {
        return true;
    }

    // 2b) Time-scaled soft stop for positions that never showed a meaningful peak.
    // Rationale: If we haven't reached even a small positive peak, accept smaller losses sooner
    // to avoid being trapped for long periods waiting for the hard stop. This is deliberately
    // simple and avoids new functions/complexity.
    if peak_profit < 5.0 {
        let trigger =
            (minutes_held >= 6.0 && pnl_percent <= -18.0) ||
            (minutes_held >= 15.0 && pnl_percent <= -25.0) ||
            (minutes_held >= 30.0 && pnl_percent <= -32.0);
        if trigger {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "LOSS_SOFT_STOP",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% t={:.1}m (soft stop)",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 3) Absolute time cap: must close
    if minutes_held >= MAX_HOLD_MINUTES {
        return true;
    }

    // 3a) Ultra-fast profit capture for immediate spikes (handles monitoring delays)
    // Capture any profit >= 12% within the first minute, especially early spikes
    if minutes_held <= 1.0 && pnl_percent >= 12.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "ULTRA_FAST_CAPTURE",
                &format!(
                    "{} pnl={:.2}% t={:.1}m (immediate spike capture)",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 3b) Round-trip neutral exit: after a meaningful peak, if we round-trip back to ~flat,
    // prefer to exit instead of waiting for another full cycle.
    if peak_profit >= target_min && minutes_held >= 8.0 && pnl_percent <= 1.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "ROUND_TRIP_NEUTRAL_EXIT",
                &format!(
                    "{} pnl={:.2}% peak={:.2}% t={:.1}m",
                    position.symbol,
                    pnl_percent,
                    peak_profit,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 4)A Target-based take-profit: if we hit the configured maximum target, exit cleanly.
    if pnl_percent >= target_max {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "TARGET_MAX_HIT",
                &format!(
                    "{} pnl={:.2}% target_max={:.2}% t={:.1}m",
                    position.symbol,
                    pnl_percent,
                    target_max,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 4) Quick capture windows (fast large moves)
    for (window_minutes, required_profit) in QUICK_WINDOWS {
        if minutes_held <= *window_minutes && pnl_percent >= *required_profit {
            return true;
        }
    }

    // 4a) Early moderate peak retention (handles 15-40% spikes better):
    // For peaks that are meaningful but below the "strong peak" threshold,
    // still protect against major givebacks to prevent round-trips to losses
    {
        let moderate_peak = peak_profit >= 15.0 && peak_profit < 40.0;
        let very_early = minutes_held <= 5.0; // very early in position lifecycle
        let retain_frac = 0.4; // keep at least 40% of peak (less strict than strong peaks)
        let must_retain = peak_profit * retain_frac;
        let significant_giveback = pnl_percent < must_retain;
        let meaningful_dd = peak_profit - pnl_percent >= 6.0; // smaller threshold for moderate peaks

        if moderate_peak && very_early && significant_giveback && meaningful_dd {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "MODERATE_PEAK_RETAIN_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% retain_req={:.2}% t={:.1}m (moderate peak protection)",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        must_retain,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 4b) Big-peak retention rule (anti-giveback):
    // If we printed a strong early peak, require we retain a reasonable fraction of it.
    // Rationale: after big impulse moves, reversals are violent; we prefer to bank a
    // chunk rather than round-trip. Tunables chosen conservatively.
    {
        let strong_peak = peak_profit >= 40.0; // "big first leg" heuristic
        let early_enough = minutes_held <= 15.0; // focus on the opening impulse
        let retain_frac = if peak_profit >= 80.0 { 0.55 } else { 0.5 }; // keep at least 50-55%
        let must_retain = peak_profit * retain_frac;
        let big_giveback = pnl_percent < must_retain;
        let decent_abs_dd = peak_profit - pnl_percent >= 10.0; // avoid triggering on tiny wobbles

        if strong_peak && early_enough && big_giveback && decent_abs_dd {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "BIG_PEAK_RETAIN_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% retain_req={:.2}% t={:.1}m",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        must_retain,
                        minutes_held
                    )
                );
            }
            return true;
        }
    }

    // 4c) General peak retention rule (time-agnostic):
    // If we've printed a meaningful peak at any time, enforce retaining a reasonable fraction
    // to avoid multi-hour round trips. Less strict than the early rule above but always active.
    {
        let meaningful_peak = peak_profit >= 30.0;
        if meaningful_peak {
            let retain_frac = if minutes_held <= 20.0 {
                if peak_profit >= 80.0 { 0.6 } else { 0.5 }
            } else {
                if peak_profit >= 80.0 { 0.5 } else { 0.45 }
            };
            let must_retain = peak_profit * retain_frac;
            let big_giveback = pnl_percent < must_retain;
            let decent_abs_dd = peak_profit - pnl_percent >= 8.0;
            if big_giveback && decent_abs_dd {
                if is_debug_profit_enabled() {
                    log(
                        LogTag::Profit,
                        "PEAK_RETAIN_EXIT",
                        &format!(
                            "{} pnl={:.2}% peak={:.2}% retain_req={:.2}% t={:.1}m",
                            position.symbol,
                            pnl_percent,
                            peak_profit,
                            must_retain,
                            minutes_held
                        )
                    );
                }
                return true;
            }
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

    // 6) Adaptive minimum profit gate
    // We reduce the min threshold over time (so we can exit smaller wins later instead of round-tripping)
    // and also if volatility already produced a decent peak but we gave a lot back.
    let dynamic_min_profit = {
        // Base decays from target_min toward 0.5 * target_min over time pressure
        let decay_component = target_min * (1.0 - 0.5 * time_pressure);
        // After 25 minutes allow a further soft decay to encourage freeing capital
        let long_hold_bonus = if minutes_held > 25.0 { target_min * 0.2 } else { 0.0 };
        (decay_component - long_hold_bonus).max(3.0)
    };

    // Early round-trip protection (re-tuned):
    // Original logic was triggering too aggressively on modest early peaks (e.g. 12-18%) causing
    // exits at tiny retained profits after a quick spike + pullback. We now:
    // * Require a more meaningful peak (>= ~1.8x BASE) OR enough elapsed time before considering.
    // * Impose a warm-up (>= 2.5m) so very early noise doesn't force an exit.
    // * Demand a substantial absolute drawdown (>= 8%) AND large relative give-back (current < 25% of peak).
    // * Still cap window to first 10 minutes (after that trailing / odds logic govern).
    // * Ensure we aren't still sitting on a solid base profit (skip if pnl >= 0.8 * BASE_MIN_PROFIT_PERCENT).
    {
        let meaningful_peak = peak_profit >= target_min * 1.8; // scale with target
        let warmup_passed = minutes_held >= 2.5;
        let within_window = minutes_held <= 10.0; // tighter than prior 12m
        let large_absolute_drawdown = peak_profit - pnl_percent >= 8.0; // avoid micro noise
        let large_relative_giveback = pnl_percent < peak_profit * 0.25; // retain <25% of peak
        let not_still_ok_profit = pnl_percent < target_min * 0.8; // scale with target

        if
            meaningful_peak &&
            warmup_passed &&
            within_window &&
            large_absolute_drawdown &&
            large_relative_giveback &&
            not_still_ok_profit
        {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "EARLY_RETRACE_EXIT",
                    &format!(
                        "{} pnl={:.2}% peak={:.2}% t={:.1}m retrace={:.2}% cond=mp:{} wa:{} win:{} abs_dd:{} rel:{} ok:{}",
                        position.symbol,
                        pnl_percent,
                        peak_profit,
                        minutes_held,
                        peak_profit - pnl_percent,
                        meaningful_peak,
                        warmup_passed,
                        within_window,
                        large_absolute_drawdown,
                        large_relative_giveback,
                        not_still_ok_profit
                    )
                );
            }
            return true;
        }
    }

    if pnl_percent < dynamic_min_profit && peak_profit < target_min {
        // However, if time pressure is very high and we're above a small profit, close to avoid forced cap
        if time_pressure > 0.95 && pnl_percent > 2.0 {
            return true;
        }
        // Do not return here; allow trailing/retention/scoring to handle exits once we have a peak.
    }

    // 7) Trailing stop logic (dynamic)
    // Suppress trailing exits during the early-hold grace to avoid churn on tiny spikes.
    if peak_profit >= target_min && !early_hold_active {
        // baseline gap derived from peak profit and age
        let mut gap = trailing_gap(peak_profit, minutes_held);

        // Micro trailing for early profits:
        // - For modest peaks (10% - 25%): keep very tight to avoid full givebacks.
        // - For medium peaks (25% - 60%): still tighten a bit vs baseline.
        if peak_profit < 25.0 {
            let micro_gap = (peak_profit * 0.35).clamp(3.0, 8.0);
            if micro_gap < gap {
                gap = micro_gap;
            }
        } else if peak_profit < 60.0 {
            let micro_gap_mid = (peak_profit * 0.3).clamp(6.0, 12.0);
            if micro_gap_mid < gap {
                gap = micro_gap_mid;
            }
        }

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
                    )
                );
            }
            return true;
        }
    }

    // 8) Scoring-based exit decision (lightweight, reuses existing signals)
    // Suppress scoring exits during the early-hold grace; rely on quick-capture or hard stops.
    if early_hold_active {
        // Skip SCORE_EXIT during grace
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "GRACE_HOLD",
                &format!(
                    "{} pnl={:.2}% peak={:.2}% t={:.1}s — suppressing scoring/trailing exits",
                    position.symbol,
                    pnl_percent,
                    peak_profit,
                    seconds_held
                )
            );
        }
        return false;
    }
    // Score components: negative adds to exit_score; positive subtracts (hold bias).
    let odds = continuation_odds(pnl_percent, minutes_held);
    let future_gap =
        trailing_gap(pnl_percent.max(peak_profit), minutes_held) *
        trailing_time_pressure_multiplier;
    let potential_gain_ceiling = 200.0;
    let potential_gain = (potential_gain_ceiling - pnl_percent).max(0.0).min(100.0);
    let expected_edge = odds * potential_gain - (1.0 - odds) * future_gap;

    let mut exit_score = 0.0f64;

    // Drawdown vs dynamic gap: heavy weight
    if peak_profit >= target_min {
        let gap_now = trailing_gap(peak_profit, minutes_held) * trailing_time_pressure_multiplier;
        let dd_ratio = if gap_now > 0.0 { drawdown / gap_now } else { 0.0 };
        if dd_ratio >= 1.0 {
            exit_score += 2.0;
        } else if
            // would trigger trail
            dd_ratio >= 0.7
        {
            exit_score += 1.2;
        } else if dd_ratio >= 0.4 {
            exit_score += 0.6;
        }
    }

    // Odds and EV
    if odds < adaptive_odds_threshold {
        exit_score += (adaptive_odds_threshold - odds) * 1.5;
    }
    if expected_edge <= 0.0 {
        exit_score += (expected_edge.abs() / (future_gap + 1.0)).min(1.5);
    }

    // Time pressure
    exit_score += time_pressure * 0.8; // up to +0.8

    // Profit quality
    if pnl_percent < dynamic_min_profit {
        exit_score += 0.8;
    } // below our dynamic gate
    if pnl_percent >= INSTANT_EXIT_LEVEL_1 {
        exit_score += 1.0;
    }

    // Early retrace already handled above; add a small nudge if peak was meaningful and we gave back a lot
    if peak_profit >= 20.0 && drawdown >= peak_profit * 0.4 {
        exit_score += 0.7;
    }

    // Very small profits after long time -> nudge to close
    if minutes_held > 45.0 && pnl_percent < 5.0 {
        exit_score += 0.6;
    }

    // Flat after big peak for a while -> additional nudge
    if peak_profit >= target_min && minutes_held >= 12.0 && pnl_percent <= 1.0 {
        exit_score += 1.0;
    }

    // Holding bias when trend looks healthy (odds good and EV positive)
    if odds >= adaptive_odds_threshold && expected_edge > 0.0 {
        exit_score -= 0.6;
    }

    // Normalize and threshold
    let mut score_threshold = 1.2; // calibrated to act after 1-3 weak signals or one strong
    // Lower threshold under high time pressure to favor exits as cap approaches
    score_threshold -= 0.2 * time_pressure; // down to ~1.0 at full pressure
    if exit_score >= score_threshold {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "SCORE_EXIT",
                &format!(
                    "{} score={:.2} thr={:.2} pnl={:.2}% peak={:.2}% dd={:.2}% odds={:.2} edge={:.2} gapF={:.2} t={:.1}m",
                    position.symbol,
                    exit_score,
                    score_threshold,
                    pnl_percent,
                    peak_profit,
                    drawdown,
                    odds,
                    expected_edge,
                    future_gap,
                    minutes_held
                )
            );
        }
        return true;
    }

    // 9) Time pressure final nudge — if we are very close to MAX_HOLD_MINUTES and have any decent profit,
    // prefer to exit rather than risk forced closure on the cap.
    if time_pressure > 0.92 && pnl_percent > target_min {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "TIME_PRESSURE_EXIT",
                &format!(
                    "{} pnl={:.2}% t={:.1}m pressure={:.2}",
                    position.symbol,
                    pnl_percent,
                    minutes_held,
                    time_pressure
                )
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
                "{} pnl={:.2}% ({:.6} SOL) peak={:.2}% dd={:.2}% t={:.1}m tp={:.2} odds={:.2} edge={:.2} thr={:.2} dyn_min={:.2} tgt_min={:.2} tgt_max={:.2}",
                position.symbol,
                pnl_percent,
                pnl_sol,
                peak_profit,
                drawdown,
                minutes_held,
                time_pressure,
                odds,
                expected_edge,
                adaptive_odds_threshold,
                dynamic_min_profit,
                target_min,
                target_max
            )
        );
    }

    // Default: continue holding
    false
}
