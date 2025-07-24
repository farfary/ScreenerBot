use crate::global::*;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };

// ========== IMPROVED PROFIT SYSTEM CONFIGURATION ==========
// Enhanced parameters for better profit-taking with duration-based scaling

/// Duration-based profit targets (hours -> required profit %)
const PROFIT_TARGET_SHORT: f64 = 5.0; // 5% profit for positions < 2 hours
const PROFIT_TARGET_MEDIUM: f64 = 8.0; // 8% profit for positions 2-6 hours
const PROFIT_TARGET_LONG: f64 = 12.0; // 12% profit for positions 6-24 hours
const PROFIT_TARGET_VERY_LONG: f64 = 20.0; // 20% profit for positions > 24 hours

/// High profit thresholds for quick exits
const HIGH_PROFIT_QUICK_EXIT: f64 = 50.0; // 50%+ profit = immediate high urgency
const VERY_HIGH_PROFIT_EXIT: f64 = 100.0; // 100%+ profit = maximum urgency
const EXTREME_PROFIT_EXIT: f64 = 500.0; // 500%+ profit = instant sell

/// Time duration thresholds (hours)
const SHORT_HOLD_HOURS: f64 = 2.0; // Short-term position
const MEDIUM_HOLD_HOURS: f64 = 6.0; // Medium-term position
const LONG_HOLD_HOURS: f64 = 24.0; // Long-term position
const VERY_LONG_HOLD_HOURS: f64 = 72.0; // Very long-term position

/// Momentum analysis - simplified and more effective
const MOMENTUM_STRONG_THRESHOLD: f64 = 10.0; // 10% gain in last hour = strong momentum
const MOMENTUM_WEAK_THRESHOLD: f64 = 2.0; // 2% gain in last hour = weak momentum
const MOMENTUM_FADING_HOURS: f64 = 0.5; // If no gains in 30 min = fading

/// Duration-based urgency scaling
const URGENCY_AGE_MULTIPLIER: f64 = 0.1; // Add 0.1 urgency per day held
const URGENCY_HIGH_PROFIT_BASE: f64 = 0.7; // Base urgency for high profits
const URGENCY_EXTREME_PROFIT: f64 = 0.95; // Urgency for extreme profits
const URGENCY_TIME_PRESSURE_MAX: f64 = 0.4; // Max urgency from time alone

/// Emergency stop loss - keep existing protection
const EMERGENCY_PRICE_THRESHOLD: f64 = -99.0; // Price decline % for emergency exit
const EMERGENCY_PNL_THRESHOLD: f64 = -99.9; // P&L decline % for emergency exit

// ================================================

/// Simplified momentum analysis for better decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMomentumAnalysis {
    pub current_profit_percent: f64,
    pub momentum_1h_percent: f64, // Price change in last hour
    pub momentum_30m_percent: f64, // Price change in last 30 minutes
    pub is_momentum_strong: bool, // Strong upward momentum
    pub is_momentum_weak: bool, // Weak momentum
    pub is_momentum_fading: bool, // Momentum declining
    pub time_held_hours: f64,
}

/// Calculate duration-based profit target
pub fn get_profit_target_for_duration(hours_held: f64) -> f64 {
    if hours_held < SHORT_HOLD_HOURS {
        PROFIT_TARGET_SHORT
    } else if hours_held < MEDIUM_HOLD_HOURS {
        PROFIT_TARGET_MEDIUM
    } else if hours_held < LONG_HOLD_HOURS {
        PROFIT_TARGET_LONG
    } else {
        PROFIT_TARGET_VERY_LONG
    }
}

/// Analyze momentum with simplified approach
pub fn analyze_simple_momentum(
    token: &Token,
    current_price: f64,
    position: &Position,
    time_held_hours: f64
) -> SimpleMomentumAnalysis {
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let current_profit_percent = ((current_price - entry_price) / entry_price) * 100.0;

    let mut momentum_1h_percent = 0.0;
    let mut momentum_30m_percent = 0.0;

    // Extract momentum from token price changes if available
    if let Some(price_changes) = &token.price_change {
        momentum_1h_percent = price_changes.h1.unwrap_or(0.0);
        // Estimate 30m from 5m data (approximate)
        momentum_30m_percent = price_changes.m5.unwrap_or(0.0) * 6.0; // 5m * 6 = 30m estimate
    }

    // Determine momentum strength
    let is_momentum_strong = momentum_1h_percent > MOMENTUM_STRONG_THRESHOLD;
    let is_momentum_weak =
        momentum_1h_percent < MOMENTUM_WEAK_THRESHOLD && momentum_1h_percent > 0.0;
    let is_momentum_fading =
        momentum_1h_percent <= 0.0 || momentum_30m_percent < momentum_1h_percent / 2.0;

    SimpleMomentumAnalysis {
        current_profit_percent,
        momentum_1h_percent,
        momentum_30m_percent,
        is_momentum_strong,
        is_momentum_weak,
        is_momentum_fading,
        time_held_hours,
    }
}

/// IMPROVED SMART PROFIT SYSTEM - Duration-aware profit taking
/// Scales profit requirements based on how long position has been held
/// Focuses on taking profits with appropriate urgency based on time and momentum
pub fn should_sell_smart_system(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    // BULLETPROOF PROTECTION: Check simple price relationship FIRST
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let simple_price_change_percent = ((current_price - entry_price) / entry_price) * 100.0;

    // ABSOLUTE RULE: If simple price check shows loss, NEVER SELL
    if simple_price_change_percent < 0.0 {
        log(
            LogTag::Trader,
            "HOLD",
            &format!(
                "HOLDING {} - Simple price check: {:.2}% loss - NEVER SELL AT LOSS",
                position.symbol,
                simple_price_change_percent
            )
        );
        return (
            0.0,
            format!(
                "HOLD: Price {:.1}% below entry - NEVER SELL AT LOSS",
                simple_price_change_percent
            ),
        );
    }

    // Secondary check: Calculate P&L for additional validation
    let (_, current_pnl_percent) = calculate_position_pnl(position, Some(current_price));

    // Only allow emergency exit if BOTH price AND P&L show extreme loss
    if
        current_pnl_percent <= EMERGENCY_PNL_THRESHOLD &&
        simple_price_change_percent <= EMERGENCY_PRICE_THRESHOLD
    {
        log(
            LogTag::Trader,
            "EMERGENCY",
            &format!(
                "EXTREME EMERGENCY: {} - Price: {:.2}%, P&L: {:.2}% - emergency exit",
                position.symbol,
                simple_price_change_percent,
                current_pnl_percent
            )
        );
        return (1.0, "EMERGENCY: Confirmed extreme loss on both price and P&L".to_string());
    }

    // Additional P&L based protection (backup)
    if current_pnl_percent < 0.0 {
        log(
            LogTag::Trader,
            "HOLD",
            &format!(
                "HOLDING {} - P&L shows {:.2}% loss - NEVER SELL AT LOSS",
                position.symbol,
                current_pnl_percent
            )
        );
        return (0.0, format!("HOLD: P&L {:.1}% loss - NEVER SELL AT LOSS", current_pnl_percent));
    }

    // === PROFIT-ONLY LOGIC BELOW ===

    let time_held_hours = time_held_seconds / 3600.0;

    // === EXTREME PROFIT PROTECTION ===
    // Immediate high urgency for extreme profits to secure gains
    if current_pnl_percent >= EXTREME_PROFIT_EXIT {
        return (
            URGENCY_EXTREME_PROFIT,
            format!("EXTREME PROFIT: {:.1}% - SECURE IMMEDIATELY", current_pnl_percent),
        );
    }

    if current_pnl_percent >= VERY_HIGH_PROFIT_EXIT {
        return (
            URGENCY_EXTREME_PROFIT,
            format!("VERY HIGH PROFIT: {:.1}% - SECURE GAINS", current_pnl_percent),
        );
    }

    if current_pnl_percent >= HIGH_PROFIT_QUICK_EXIT {
        return (
            URGENCY_HIGH_PROFIT_BASE,
            format!("HIGH PROFIT: {:.1}% - TAKE PROFITS", current_pnl_percent),
        );
    }

    // === DURATION-BASED PROFIT TARGETS ===
    let required_profit = get_profit_target_for_duration(time_held_hours);

    // If we've reached the duration-based profit target, calculate sell urgency
    if current_pnl_percent >= required_profit {
        // Analyze momentum for better timing
        let momentum = analyze_simple_momentum(token, current_price, position, time_held_hours);

        // Base urgency starts at 0.5 when profit target is reached
        let mut urgency: f64 = 0.5;

        // Adjust urgency based on momentum
        if momentum.is_momentum_fading {
            urgency += 0.3; // Higher urgency if momentum is fading
            return (
                urgency.min(0.95),
                format!(
                    "MOMENTUM FADING: {:.1}% profit (target: {:.1}%) after {:.1}h",
                    current_pnl_percent,
                    required_profit,
                    time_held_hours
                ),
            );
        } else if momentum.is_momentum_weak {
            urgency += 0.2; // Medium urgency for weak momentum
        } else if momentum.is_momentum_strong {
            urgency -= 0.1; // Lower urgency if momentum is still strong
        }

        // Add time pressure - longer positions get higher urgency
        let age_pressure = (time_held_hours / 24.0) * URGENCY_AGE_MULTIPLIER;
        urgency += age_pressure;

        // Add profit magnitude bonus
        let profit_bonus = ((current_pnl_percent - required_profit) / required_profit) * 0.1;
        urgency += profit_bonus.min(0.2);

        return (
            urgency.min(0.95),
            format!(
                "PROFIT TARGET: {:.1}% profit (target: {:.1}%) after {:.1}h",
                current_pnl_percent,
                required_profit,
                time_held_hours
            ),
        );
    }

    // === VERY LONG DURATION PRESSURE ===
    // For positions held too long, apply time pressure even below profit target
    if time_held_hours > VERY_LONG_HOLD_HOURS && current_pnl_percent > 0.0 {
        let time_pressure = ((time_held_hours - VERY_LONG_HOLD_HOURS) / 24.0) * 0.3;
        let urgency = time_pressure.min(URGENCY_TIME_PRESSURE_MAX);

        if urgency > 0.3 {
            return (
                urgency,
                format!(
                    "TIME PRESSURE: {:.1}% profit after {:.1}h - very long duration",
                    current_pnl_percent,
                    time_held_hours
                ),
            );
        }
    }

    // === DEFAULT: HOLD FOR MORE PROFIT ===
    // Position has profit but hasn't reached target yet
    if current_pnl_percent > 0.0 {
        // Small gradual urgency build-up for very long positions
        let gradual_urgency = (time_held_hours / 48.0) * 0.1; // Very gradual over 48 hours

        return (
            gradual_urgency.min(0.2),
            format!(
                "HOLD: {:.1}% profit (need {:.1}%) after {:.1}h",
                current_pnl_percent,
                required_profit,
                time_held_hours
            ),
        );
    }

    // Breakeven or tiny gains - hold indefinitely
    (0.0, format!("HOLD: Near breakeven at {:.1}%", current_pnl_percent))
}

/// Legacy compatibility functions - these wrap the new smart system

/// Legacy function - wraps the new smart system
pub fn should_sell_dynamic(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    should_sell_smart_system(position, token, current_price, time_held_seconds)
}
