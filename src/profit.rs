//! Profiting system.
//! Goal: Fast, probabilistic, trailing-stop driven exits capturing 10%-200% gains
//! within 0s–120m while enforcing sane loss protection.
//!
//! Rationale:
//! Previous version was overly complex (2k+ LOC) with heavy per-token analysis & pump logic.
//! This streamlined module keeps only battle‑tested primitives:
//! * Hard stop loss
//! * Time based max hold
//! * Dynamic quick profit capture (time + profit ladder)
//! * Adaptive trailing stop that widens with profit and tightens with time
//! * Odds model (simple EV proxy) to avoid over-holding late in lifecycle
//!
//! External dependency surface retained: public `should_sell(position, current_price)` only.
//! All removed advanced analysis functions intentionally deleted to avoid dead code confusion.

use chrono::Utc;
use crate::logger::{ log, LogTag };
use crate::positions::{ Position, calculate_position_pnl };
use crate::global::*;

// ============================= Core Tunable Parameters =============================

// Loss & risk
pub const STOP_LOSS_PERCENT: f64 = -40.0; // Hard kill
pub const EXTREME_LOSS_PERCENT: f64 = -55.0; // Emergency if reached very fast

// Time (minutes)
pub const MAX_HOLD_MINUTES: f64 = 120.0; // Absolute cap
pub const TRAILING_TIGHTEN_START_MIN: f64 = 45.0; // Start tightening trailing
pub const TRAILING_TIGHTEN_FULL_MIN: f64 = 90.0; // Fully tight by here

// Profit ladders (instant exits)
pub const BASE_MIN_PROFIT_PERCENT: f64 = 10.0; // Must reach before discretionary exits
pub const INSTANT_EXIT_LEVEL_1: f64 = 100.0; // Usually secure
pub const INSTANT_EXIT_LEVEL_2: f64 = 150.0; // Always secure

// Trailing stop dynamics
pub const TRAIL_MIN_GAP: f64 = 5.0; // Tightest trailing gap
pub const TRAIL_MAX_GAP: f64 = 35.0; // Widest trailing gap
pub const TRAIL_TIGHTEN_START: f64 = 45.0; // Minutes when to start tightening
pub const TRAIL_TIGHTEN_FULL: f64 = 90.0; // Minutes when fully tight

// Odds model
pub const EXIT_ODDS_THRESHOLD: f64 = 0.65; // Probability threshold for exit

// Quick capture windows (minutes, profit%)
const QUICK_WINDOWS: &[(f64, f64)] = &[
    (1.0, 30.0), // 30% profit in 1 minute = instant exit
    (5.0, 50.0), // 50% profit in 5 minutes = instant exit
    (15.0, 80.0), // 80% profit in 15 minutes = instant exit
];

// ============================= Core Logic Functions =============================

/// Calculate dynamic trailing stop gap based on profit level and time held
fn trailing_gap(peak_profit: f64, minutes_held: f64) -> f64 {
    if peak_profit <= 0.0 {
        return TRAIL_MIN_GAP;
    }

    // Base gap calculation - tighter for higher profits
    let mut gap = match peak_profit {
        p if p < 20.0 => p * 0.4, // 40% of profit for small gains
        p if p < 50.0 => p * 0.3, // 30% of profit for medium gains
        p if p < 100.0 => p * 0.25, // 25% of profit for good gains
        _ => peak_profit * 0.2, // 20% of profit for excellent gains
    };

    // Clamp to min/max bounds
    gap = gap.clamp(TRAIL_MIN_GAP, TRAIL_MAX_GAP);

    // Time-based tightening - reduce gap as position ages
    if minutes_held >= TRAIL_TIGHTEN_START {
        let progress = (
            (minutes_held - TRAIL_TIGHTEN_START) /
            (TRAIL_TIGHTEN_FULL - TRAIL_TIGHTEN_START)
        ).clamp(0.0, 1.0);
        gap *= 1.0 - 0.3 * progress; // Shrink up to 30% as time progresses
    }

    gap
}

/// Calculate odds of profitable continuation based on current state
fn continuation_odds(profit_percent: f64, minutes_held: f64) -> f64 {
    if profit_percent <= 0.0 {
        return 0.55; // Slight edge when breaking even
    }

    // Time decay - longer held = lower odds of further gains
    let time_decay = (-minutes_held / 50.0).exp();

    // Profit decay - higher profits = lower odds of further significant gains
    let profit_decay = (-(profit_percent / 120.0).powf(1.1)).exp();

    // Early boost for very quick high profits
    let early_boost = if minutes_held < 5.0 && profit_percent > 40.0 { 0.1 } else { 0.0 };

    (time_decay * profit_decay + early_boost).clamp(0.0, 1.0)
}

// ============================= Main Decision Function =============================

/// Main profit system decision function
/// Returns true if position should be sold, false if should continue holding
pub async fn should_sell(position: &Position, current_price: f64) -> bool {
    // Validate inputs
    if current_price <= 0.0 || !current_price.is_finite() {
        return false;
    }

    let entry = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry <= 0.0 || !entry.is_finite() {
        return false;
    }

    // Calculate key metrics
    let (pnl_sol, pnl_percent) = if position.symbol == "SIM" {
        // Mock calculation for simulation
        let pnl_percent = ((current_price - entry) / entry) * 100.0;
        let pnl_sol = ((current_price - entry) * position.entry_size_sol) / entry;
        (pnl_sol, pnl_percent)
    } else {
        calculate_position_pnl(position, Some(current_price)).await
    };
    let minutes_held = ((Utc::now() - position.entry_time).num_seconds() as f64) / 60.0;
    let peak_price = if position.price_highest > 0.0 {
        position.price_highest
    } else {
        current_price
    };
    let peak_profit = ((peak_price - entry) / entry) * 100.0;
    let drawdown = peak_profit - pnl_percent;

    // 1. LOSS PROTECTION - Hard stops
    if pnl_percent <= EXTREME_LOSS_PERCENT {
        return true;
    }
    if pnl_percent <= STOP_LOSS_PERCENT && minutes_held >= 1.0 {
        return true;
    }

    // 2. TIME CAP - Maximum hold time
    if minutes_held >= MAX_HOLD_MINUTES {
        return true;
    }

    // 3. QUICK CAPTURE - Fast profit taking on quick moves
    for (window_minutes, required_profit) in QUICK_WINDOWS {
        if minutes_held <= *window_minutes && pnl_percent >= *required_profit {
            return true;
        }
    }

    // 4. INSTANT LARGE PROFITS - Take massive gains immediately
    if pnl_percent >= INSTANT_EXIT_LEVEL_2 {
        return true;
    }
    if pnl_percent >= INSTANT_EXIT_LEVEL_1 && (drawdown >= 10.0 || minutes_held > 10.0) {
        return true;
    }

    // 5. MINIMUM PROFIT GATE - Don't exit below minimum profit threshold
    if pnl_percent < BASE_MIN_PROFIT_PERCENT {
        return false;
    }

    // 6. TRAILING STOP - Dynamic trailing based on peak profits
    if peak_profit >= BASE_MIN_PROFIT_PERCENT {
        let mut gap = trailing_gap(peak_profit, minutes_held);

        // Adaptive tightening for significant drawdowns after 30 minutes
        if minutes_held > 30.0 && drawdown > gap * 0.6 {
            gap *= 0.85; // Tighten gap by 15%
        }

        if drawdown >= gap {
            return true;
        }
    }

    // 7. ODDS-BASED EXIT - Expected value calculation
    let odds = continuation_odds(pnl_percent, minutes_held);
    let potential_gain = (200.0 - pnl_percent).max(0.0).min(100.0);
    let future_gap = trailing_gap(pnl_percent.max(peak_profit), minutes_held);
    let expected_edge = odds * potential_gain - (1.0 - odds) * future_gap;

    if odds < EXIT_ODDS_THRESHOLD && expected_edge <= 0.0 {
        return true;
    }

    // Debug logging for decision transparency
    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "DECISION",
            &format!(
                "{} pnl={:.2}% ({:.6} SOL) peak={:.2}% dd={:.2}% t={:.1}m odds={:.2} edge={:.2}",
                position.symbol,
                pnl_percent,
                pnl_sol,
                peak_profit,
                drawdown,
                minutes_held,
                odds,
                expected_edge
            )
        );
    }

    false // Default: continue holding
}
