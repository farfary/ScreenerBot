//! profit_new.rs - Synchronized Fast Scalping Profit System
//!
//! Rewritten to align with the optimized entry_new.rs system:
//! - 5-10% minimum profit targets optimized for Jupiter realization
//! - Aggressive profit-taking for fast scalping opportunities
//! - Synchronized timeframes and logic with entry system
//! - Enhanced trailing stops optimized for scalping volatility
//! - Time pressure system for quick profit realization
//!
//! Key Features:
//! - Minimum 5% profit gate (cost coverage + fees)
//! - Quick capture windows (1-15 minutes for fast profit taking)
//! - Activity-weighted profit targets (high activity = higher targets)
//! - Scalping-optimized trailing stops (tighter gaps, faster adjustments)
//! - Perfect scalp detection for maximum profit extraction

use crate::global::*;
use crate::logger::{ log, LogTag };
use crate::positions_lib::calculate_position_pnl;
use crate::positions_types::Position;
use crate::tokens::get_price;
use crate::tokens::types::PriceOptions;
use chrono::Utc;

// ================================ SCALPING PROFIT PARAMETERS ================================

/// Minimum profit thresholds (aligned with entry system)
pub const SCALP_MIN_PROFIT: f64 = 5.0; // Absolute minimum for cost coverage
pub const SCALP_GATE_PROFIT: f64 = 8.0; // Gate for discretionary sells
pub const SCALP_TARGET_PROFIT: f64 = 12.0; // Target profit for scalping

/// Quick profit capture levels
pub const INSTANT_SCALP_L1: f64 = 15.0; // Fast capture level
pub const INSTANT_SCALP_L2: f64 = 25.0; // Aggressive capture level
pub const INSTANT_SCALP_L3: f64 = 40.0; // Maximum capture level

/// Loss management (tighter for scalping)
pub const SCALP_STOP_LOSS: f64 = -25.0; // Tighter stop loss
pub const SCALP_EMERGENCY_LOSS: f64 = -40.0; // Emergency exit

/// Time management (optimized for scalping speed)
pub const SCALP_MAX_HOLD: f64 = 90.0; // Maximum hold duration (minutes)
pub const SCALP_QUICK_EXIT: f64 = 30.0; // Quick exit pressure start (minutes)

/// Trailing stop parameters for scalping
pub const SCALP_TRAIL_MIN: f64 = 2.0; // Minimum trail gap (tighter)
pub const SCALP_TRAIL_MAX: f64 = 15.0; // Maximum trail gap (tighter)
pub const SCALP_TRAIL_TIGHTEN: f64 = 20.0; // Start tightening at 20min
pub const SCALP_TRAIL_FULL: f64 = 60.0; // Full tightening at 60min

/// Quick capture windows for scalping (minutes, required profit %)
const SCALP_CAPTURE_WINDOWS: &[(f64, f64)] = &[
    (0.5, 20.0), // 30 seconds: 20% profit
    (1.0, 18.0), // 1 minute: 18% profit
    (2.0, 15.0), // 2 minutes: 15% profit
    (3.0, 20.0), // 3 minutes: 20% profit
    (5.0, 25.0), // 5 minutes: 25% profit
    (8.0, 30.0), // 8 minutes: 30% profit
    (12.0, 35.0), // 12 minutes: 35% profit
    (15.0, 40.0), // 15 minutes: 40% profit
];

/// Profit odds threshold for scalping
pub const SCALP_ODDS_THRESHOLD: f64 = 0.7; // Higher threshold for quality exits

// ================================ SCALPING PROFIT ANALYSIS ================================

#[derive(Debug, Clone)]
pub struct ScalpProfitAnalysis {
    pub current_profit: f64,
    pub peak_profit: f64,
    pub minutes_held: f64,
    pub trail_gap: f64,
    pub quick_capture_triggered: bool,
    pub time_pressure_factor: f64,
    pub activity_multiplier: f64,
    pub liquidity_factor: f64,
}

// ================================ MAIN PROFIT DECISION FUNCTION ================================

/// Fast scalping profit decision optimized for 5-10% minimum targets
/// Returns true if should sell, false if should continue holding
pub async fn should_sell_scalp(position: &Position, current_price: f64) -> bool {
    // Basic validation
    if !current_price.is_finite() || current_price <= 0.0 {
        return false;
    }

    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if !entry_price.is_finite() || entry_price <= 0.0 {
        return false;
    }

    // Calculate current profit/loss
    let current_pnl = match calculate_position_pnl(position, Some(current_price)).await {
        Some(pnl) => pnl.profit_loss_percent,
        None => {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "PNL_CALC_FAILED",
                    &format!("❌ {} PnL calculation failed", position.token_symbol)
                );
            }
            return false;
        }
    };

    let minutes_held = position.minutes_held();

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "SCALP_PROFIT_START",
            &format!(
                "🎯 {} scalp profit analysis: {:.1}% after {:.1}min",
                position.token_symbol,
                current_pnl,
                minutes_held
            )
        );
    }

    // Emergency loss protection
    if current_pnl <= SCALP_EMERGENCY_LOSS {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "EMERGENCY_EXIT",
                &format!(
                    "🚨 {} emergency exit: {:.1}% <= {:.1}%",
                    position.token_symbol,
                    current_pnl,
                    SCALP_EMERGENCY_LOSS
                )
            );
        }
        return true;
    }

    // Stop loss protection (tighter for scalping)
    if current_pnl <= SCALP_STOP_LOSS {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "SCALP_STOP_LOSS",
                &format!(
                    "🛑 {} stop loss: {:.1}% <= {:.1}%",
                    position.token_symbol,
                    current_pnl,
                    SCALP_STOP_LOSS
                )
            );
        }
        return true;
    }

    // Below minimum profit: very conservative (only exit on severe loss)
    if current_pnl < SCALP_MIN_PROFIT {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "BELOW_MIN_SCALP_PROFIT",
                &format!(
                    "⏳ {} below min profit {:.1}% < {:.1}% - hold",
                    position.token_symbol,
                    current_pnl,
                    SCALP_MIN_PROFIT
                )
            );
        }
        return false;
    }

    // Get comprehensive scalping analysis
    let analysis = analyze_scalp_profit(position, current_pnl, minutes_held).await;

    // Quick capture windows (aggressive profit taking)
    if analysis.quick_capture_triggered {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "QUICK_SCALP_CAPTURE",
                &format!(
                    "⚡ {} quick capture: {:.1}% in {:.1}min",
                    position.token_symbol,
                    current_pnl,
                    minutes_held
                )
            );
        }
        return true;
    }

    // Instant exit levels
    if current_pnl >= INSTANT_SCALP_L3 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "INSTANT_SCALP_L3",
                &format!(
                    "💰 {} instant exit L3: {:.1}% >= {:.1}%",
                    position.token_symbol,
                    current_pnl,
                    INSTANT_SCALP_L3
                )
            );
        }
        return true;
    }

    if current_pnl >= INSTANT_SCALP_L2 && minutes_held <= 10.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "INSTANT_SCALP_L2",
                &format!(
                    "💰 {} instant exit L2: {:.1}% >= {:.1}% in {:.1}min",
                    position.token_symbol,
                    current_pnl,
                    INSTANT_SCALP_L2,
                    minutes_held
                )
            );
        }
        return true;
    }

    if current_pnl >= INSTANT_SCALP_L1 && minutes_held <= 5.0 {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "INSTANT_SCALP_L1",
                &format!(
                    "💰 {} instant exit L1: {:.1}% >= {:.1}% in {:.1}min",
                    position.token_symbol,
                    current_pnl,
                    INSTANT_SCALP_L1,
                    minutes_held
                )
            );
        }
        return true;
    }

    // Maximum hold time pressure
    if minutes_held >= SCALP_MAX_HOLD {
        if current_pnl >= SCALP_MIN_PROFIT {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "MAX_HOLD_EXIT",
                    &format!(
                        "⏰ {} max hold exit: {:.1}% after {:.1}min >= {:.1}min",
                        position.token_symbol,
                        current_pnl,
                        minutes_held,
                        SCALP_MAX_HOLD
                    )
                );
            }
            return true;
        }
    }

    // Trailing stop analysis
    let should_trail_exit = analyze_scalp_trailing_stop(position, current_pnl, &analysis);
    if should_trail_exit {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "SCALP_TRAIL_EXIT",
                &format!(
                    "📉 {} trail stop: {:.1}% (gap: {:.1}%, peak: {:.1}%)",
                    position.token_symbol,
                    current_pnl,
                    analysis.trail_gap,
                    analysis.peak_profit
                )
            );
        }
        return true;
    }

    // Time pressure discretionary selling
    if minutes_held >= SCALP_QUICK_EXIT {
        let time_pressure_exit = analyze_time_pressure_exit(current_pnl, &analysis);
        if time_pressure_exit {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "TIME_PRESSURE_EXIT",
                    &format!(
                        "⏳ {} time pressure exit: {:.1}% after {:.1}min (pressure: {:.2})",
                        position.token_symbol,
                        current_pnl,
                        minutes_held,
                        analysis.time_pressure_factor
                    )
                );
            }
            return true;
        }
    }

    // Odds-based exit analysis
    let continuation_odds = scalp_continuation_odds(current_pnl, minutes_held, &analysis);
    if continuation_odds < SCALP_ODDS_THRESHOLD && current_pnl >= SCALP_GATE_PROFIT {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "ODDS_EXIT",
                &format!(
                    "🎲 {} odds exit: {:.1}% (odds: {:.2} < {:.2})",
                    position.token_symbol,
                    current_pnl,
                    continuation_odds,
                    SCALP_ODDS_THRESHOLD
                )
            );
        }
        return true;
    }

    // Default: continue holding
    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "SCALP_HOLD_CONTINUE",
            &format!(
                "⏳ {} continue hold: {:.1}% (odds: {:.2}, trail_gap: {:.1}%)",
                position.token_symbol,
                current_pnl,
                continuation_odds,
                analysis.trail_gap
            )
        );
    }

    false
}

// ================================ ANALYSIS FUNCTIONS ================================

/// Comprehensive scalping profit analysis
async fn analyze_scalp_profit(
    position: &Position,
    current_pnl: f64,
    minutes_held: f64
) -> ScalpProfitAnalysis {
    // Get peak profit from position history
    let peak_profit = position.peak_pnl_percent.unwrap_or(current_pnl.max(0.0));

    // Calculate scalping trailing gap
    let trail_gap = calculate_scalp_trail_gap(peak_profit, minutes_held);

    // Check quick capture windows
    let quick_capture = check_quick_capture_windows(current_pnl, minutes_held);

    // Time pressure factor (increases as we approach max hold)
    let time_pressure = if minutes_held >= SCALP_QUICK_EXIT {
        ((minutes_held - SCALP_QUICK_EXIT) / (SCALP_MAX_HOLD - SCALP_QUICK_EXIT)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Get token for activity analysis
    let (activity_mult, liquidity_fact) = get_token_factors(&position.token_mint).await;

    ScalpProfitAnalysis {
        current_profit: current_pnl,
        peak_profit,
        minutes_held,
        trail_gap,
        quick_capture_triggered: quick_capture,
        time_pressure_factor: time_pressure,
        activity_multiplier: activity_mult,
        liquidity_factor: liquidity_fact,
    }
}

/// Calculate trailing gap optimized for scalping
fn calculate_scalp_trail_gap(peak_profit: f64, minutes_held: f64) -> f64 {
    if peak_profit <= 0.0 {
        return SCALP_TRAIL_MIN;
    }

    // Base gap calculation (tighter for scalping)
    let mut gap = if peak_profit < 15.0 {
        peak_profit * 0.25 // 25% of profit for small gains
    } else if peak_profit < 30.0 {
        peak_profit * 0.2 // 20% of profit for medium gains
    } else if peak_profit < 60.0 {
        peak_profit * 0.18 // 18% of profit for large gains
    } else {
        peak_profit * 0.15 // 15% of profit for huge gains
    };

    // Clamp to scalping range
    gap = gap.clamp(SCALP_TRAIL_MIN, SCALP_TRAIL_MAX);

    // Time-based tightening (more aggressive for scalping)
    if minutes_held >= SCALP_TRAIL_TIGHTEN {
        let tighten_progress = (
            (minutes_held - SCALP_TRAIL_TIGHTEN) /
            (SCALP_TRAIL_FULL - SCALP_TRAIL_TIGHTEN)
        ).clamp(0.0, 1.0);
        let tighten_factor = 0.4 * tighten_progress; // Up to 40% tightening
        gap *= 1.0 - tighten_factor;
    }

    (gap.clamp(SCALP_TRAIL_MIN, SCALP_TRAIL_MAX) * 100.0).round() / 100.0
}

/// Check quick capture windows for aggressive profit taking
fn check_quick_capture_windows(current_pnl: f64, minutes_held: f64) -> bool {
    for &(window_min, required_profit) in SCALP_CAPTURE_WINDOWS.iter() {
        if minutes_held <= window_min && current_pnl >= required_profit {
            return true;
        }
    }
    false
}

/// Analyze trailing stop for scalping
fn analyze_scalp_trailing_stop(
    position: &Position,
    current_pnl: f64,
    analysis: &ScalpProfitAnalysis
) -> bool {
    let peak_profit = analysis.peak_profit;

    // Only apply trailing if we've hit significant profit
    if peak_profit < SCALP_GATE_PROFIT {
        return false;
    }

    // Check if current profit has dropped below trail threshold
    let trail_threshold = peak_profit - analysis.trail_gap;
    current_pnl <= trail_threshold
}

/// Analyze time pressure exit for scalping
fn analyze_time_pressure_exit(current_pnl: f64, analysis: &ScalpProfitAnalysis) -> bool {
    if analysis.time_pressure_factor <= 0.3 {
        return false; // Not enough time pressure yet
    }

    // Time pressure thresholds decrease as pressure increases
    let pressure_adjusted_threshold =
        SCALP_GATE_PROFIT * (1.0 - analysis.time_pressure_factor * 0.5);

    current_pnl >= pressure_adjusted_threshold.max(SCALP_MIN_PROFIT)
}

/// Calculate continuation odds for scalping
fn scalp_continuation_odds(
    current_pnl: f64,
    minutes_held: f64,
    analysis: &ScalpProfitAnalysis
) -> f64 {
    // Base odds start higher for profitable positions
    let mut odds = if current_pnl >= SCALP_TARGET_PROFIT {
        0.75 // Good starting odds for target+ profits
    } else if current_pnl >= SCALP_GATE_PROFIT {
        0.7 // Decent odds for gate+ profits
    } else {
        0.65 // Lower odds below gate
    };

    // Time decay (faster for scalping)
    let time_decay = (-minutes_held / 30.0).exp(); // Faster decay than original
    odds *= time_decay;

    // Profit decay (non-linear)
    let profit_decay = (-(current_pnl / 80.0).powf(1.2)).exp();
    odds *= profit_decay;

    // Activity bonus
    if analysis.activity_multiplier > 0.8 {
        odds *= 1.1; // 10% bonus for high activity
    }

    // Liquidity bonus
    if analysis.liquidity_factor > 0.8 {
        odds *= 1.05; // 5% bonus for good liquidity
    }

    // Early scalp bonus (very fast profitable moves)
    if minutes_held < 3.0 && current_pnl > INSTANT_SCALP_L1 {
        odds *= 1.15; // 15% bonus for ultra-fast profits
    }

    odds.clamp(0.0, 1.0)
}

/// Get token-specific factors for profit analysis
async fn get_token_factors(token_mint: &str) -> (f64, f64) {
    // Try to get fresh token data for activity/liquidity analysis
    match get_price(token_mint, Some(PriceOptions::default()), false).await {
        Some(price_result) => {
            // Activity factor (based on transaction volume if available)
            let activity_mult = 1.0; // Default - could be enhanced with txn data

            // Liquidity factor (based on reserves)
            let liquidity_fact = price_result.reserve_sol
                .map(|reserves| {
                    if reserves >= 100.0 && reserves <= 500.0 {
                        1.0 // Optimal scalping liquidity
                    } else if reserves >= 50.0 && reserves <= 700.0 {
                        0.9 // Good liquidity
                    } else if reserves >= 25.0 {
                        0.7 // Adequate liquidity
                    } else {
                        0.5 // Limited liquidity
                    }
                })
                .unwrap_or(0.7);

            (activity_mult, liquidity_fact)
        }
        None => (0.8, 0.7), // Default values
    }
}

// ================================ PUBLIC API COMPATIBILITY ================================

/// Main profit decision function (maintains compatibility with existing interface)
/// Redirects to optimized scalping system
pub async fn should_sell(position: &Position, current_price: f64) -> bool {
    should_sell_scalp(position, current_price).await
}

// ================================ HELPER FUNCTIONS ================================

/// Utility function for clamping values to 0.0-1.0 range
#[inline]
fn clamp01(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0).min(1.0) } else { 0.0 }
}

/// Calculate dynamic trailing gap (legacy compatibility)
pub fn trailing_gap(peak_profit: f64, minutes_held: f64) -> f64 {
    calculate_scalp_trail_gap(peak_profit, minutes_held)
}

/// Continuation odds estimator (legacy compatibility)
pub fn continuation_odds(profit_percent: f64, minutes_held: f64) -> f64 {
    let analysis = ScalpProfitAnalysis {
        current_profit: profit_percent,
        peak_profit: profit_percent.max(0.0),
        minutes_held,
        trail_gap: calculate_scalp_trail_gap(profit_percent.max(0.0), minutes_held),
        quick_capture_triggered: false,
        time_pressure_factor: 0.0,
        activity_multiplier: 1.0,
        liquidity_factor: 1.0,
    };

    scalp_continuation_odds(profit_percent, minutes_held, &analysis)
}
