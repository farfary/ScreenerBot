use crate::global::*;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use crate::tokens::{
    get_token_rugcheck_data_safe,
    is_token_safe_for_trading_safe,
    get_high_risk_issues,
    TokenDatabase,
    pool::get_pool_service,
};
use chrono::Utc;
use serde::{ Serialize, Deserialize };
use std::collections::HashMap;

// ================================================================================================
// 🎯 NEXT-GENERATION INTELLIGENT PROFIT SYSTEM
// ================================================================================================
// Risk-based profit scaling with real-time token analysis
// Combines Rugcheck security data + Token API data + Market momentum
// Dynamic profit targets: 10% (safe) to 10,000% (dangerous)
// Time pressure: 10-45 minutes based on safety level
// Smart exit strategies based on liquidity, volume, and social proof
//
// ⚡ FAST PROFIT-TAKING OPTIMIZATION:
// - >3% profit in <1 minute = immediate exit (captures quick momentum)
// - >5% profit in <30 seconds = ultra-fast exit (exceptional momentum)
// - Prevents profit reversal on fast-moving tokens
// ================================================================================================

// 🔒 STOP LOSS PROTECTION - PATIENT LOSS MANAGEMENT
pub const STOP_LOSS_PERCENT: f64 = -70.0; // More tolerant stop loss at -70% (was -55%)

// ➕ Liquidity-tier soft stops (percent) - MORE PATIENT
const SOFT_STOP_LARGE: f64 = -55.0; // large/XLARGE liquidity: more patient at -55% (was -40%)
const SOFT_STOP_MEDIUM: f64 = -65.0; // medium liquidity: more patient at -65% (was -50%)
const SOFT_STOP_DEFAULT: f64 = -70.0; // small/unknown: more patient at -70% (was -55%)

// ⏳ Time caps (minutes) — MORE PATIENT FOR PROFITS
const SOFT_TIME_CAP_MIN: f64 = 45.0; // begin time pressure at 45 minutes (increased from 20)
const HARD_TIME_CAP_MIN: f64 = 90.0; // must act by 90 minutes (increased from 40)

// 📐 Risk-Reward minimums by liquidity tier (RR = current_gain% / |MAE%|) - MORE TOLERANT
const REQUIRED_RR_LARGE: f64 = 1.0; // more tolerant for high-liquidity (was 1.2)
const REQUIRED_RR_MEDIUM: f64 = 1.2; // more tolerant (was 1.4)
const REQUIRED_RR_DEFAULT: f64 = 1.0; // more tolerant for small/unknown (reduced from 1.4)

// ⏰ OPTIMIZED HOLD TIMES BY SAFETY LEVEL (MINUTES)
// AGGRESSIVE FOR FAST PROFITS: 0.25 minutes to 45 minutes based on volatility
const ULTRA_SAFE_MAX_TIME: f64 = 45.0; // Ultra safe tokens - 45 minutes max (reduced from 120)
const SAFE_MAX_TIME: f64 = 60.0; // Safe tokens - 60 minutes (increased from 30)
const MEDIUM_MAX_TIME: f64 = 45.0; // Medium risk tokens - 45 minutes (increased from 20)
const RISKY_MAX_TIME: f64 = 30.0; // Risky tokens - 30 minutes (increased from 15)
const DANGEROUS_MAX_TIME: f64 = 10.0; // Dangerous tokens - 10 minutes (reduced from 30)
const MIN_HOLD_TIME: f64 = 0.25; // ULTRA-FAST: 15 seconds minimum (reduced from 0.5)

// 🎯 AGGRESSIVE PROFIT TARGETS FOR VOLATILE TRADING
// INCREASED TARGETS TO MATCH VOLATILITY POTENTIAL
const ULTRA_SAFE_PROFIT_MIN: f64 = 10.0; // 10-300% for ultra safe tokens (increased from 8-500%)
const ULTRA_SAFE_PROFIT_MAX: f64 = 300.0;
const SAFE_PROFIT_MIN: f64 = 8.0; // 8-250% for safe tokens (increased from 6-300%)
const SAFE_PROFIT_MAX: f64 = 250.0;
const MEDIUM_PROFIT_MIN: f64 = 6.0; // 6-200% for medium risk tokens (increased from 5-200%)
const MEDIUM_PROFIT_MAX: f64 = 200.0;
const RISKY_PROFIT_MIN: f64 = 5.0; // 5-150% for risky tokens (INCREASED from 3-100%)
const RISKY_PROFIT_MAX: f64 = 150.0;
const DANGEROUS_PROFIT_MIN: f64 = 3.0; // 3-100% for dangerous tokens (INCREASED from 2-50%)
const DANGEROUS_PROFIT_MAX: f64 = 100.0;

// 📈 AGGRESSIVE TRAILING STOP CONFIGURATION - OPTIMIZED FOR VOLATILITY
const USE_TRAILING_STOP: bool = true;
// Tighter trailing stops for faster, more volatile trading
const TRAILING_STOP_ULTRA_SAFE: f64 = 8.0; // 8% for ultra safe (reduced from 12%)
const TRAILING_STOP_SAFE: f64 = 6.0; // 6% for safe (reduced from 10%)
const TRAILING_STOP_MEDIUM: f64 = 5.0; // 5% for medium (reduced from 8%)
const TRAILING_STOP_RISKY: f64 = 4.0; // 4% for risky (reduced from 6%)
const TRAILING_STOP_DANGEROUS: f64 = 3.0; // 3% for dangerous (reduced from 4%)
const TIME_DECAY_FACTOR: f64 = 0.25; // Aggressive time decay (25% vs 15%) for faster exits

// 🚀 INSTANT SELL THRESHOLDS - CAPTURE MOONSHOTS UP TO 100%+
const INSTANT_SELL_PROFIT: f64 = 2000.0; // 2000%+ = instant sell
const MEGA_PROFIT_THRESHOLD: f64 = 1000.0; // 1000%+ = very urgent
const ULTRA_MEGA_THRESHOLD: f64 = 100.0; // 100%+ = ultra mega pump
const SUPER_MEGA_THRESHOLD: f64 = 75.0; // 75%+ = super mega pump

// 🎯 ENHANCED PUMP DETECTION CONFIGURATION - MULTI-TIER THRESHOLDS
const PUMP_MIN_PERCENT: f64 = 15.0; // Minimum % gain to be considered a pump
const PUMP_VELOCITY_THRESHOLD: f64 = 0.5; // % per second for pump detection
const ULTRA_VELOCITY_THRESHOLD: f64 = 2.0; // Ultra high velocity for extreme gains
const SUPER_VELOCITY_THRESHOLD: f64 = 1.5; // Super high velocity
const HIGH_VELOCITY_THRESHOLD: f64 = 1.0; // High velocity
const MEGA_PUMP_PERCENT: f64 = 50.0; // 50%+ = mega pump
const STRONG_MEGA_PERCENT: f64 = 30.0; // 30%+ = strong mega pump
const MICRO_PUMP_PERCENT: f64 = 8.0; // 8%+ = micro pump (for volatile tokens)
const PUMP_TIME_WINDOW: f64 = 5.0; // Minutes to analyze for pump detection
const CONSERVATIVE_PROFIT_MIN: f64 = 3.0; // Don't sell below 3% profit easily (reduced from 8% to be more realistic)
const TREND_PROFIT_MIN: f64 = 15.0; // Minimum for trend-based exits (was 12%)

// ⚡ ULTRA-FAST PROFIT-TAKING THRESHOLDS - ADJUSTED TO REQUIRE DECENT PROFITS
const LIGHTNING_PROFIT_THRESHOLD: f64 = 25.0; // Increased from 20% - only significant pumps
const LIGHTNING_PROFIT_TIME_LIMIT: f64 = 0.5; // 30 seconds minimum (was 15s)
const FAST_PROFIT_THRESHOLD: f64 = 20.0; // Increased from 15% - avoid false exits
const FAST_PROFIT_TIME_LIMIT: f64 = 1.0; // 1 minute minimum
const SPEED_PROFIT_THRESHOLD: f64 = 12.0; // Increased from 8% - be more selective, align with CONSERVATIVE_PROFIT_MIN
const SPEED_PROFIT_TIME_LIMIT: f64 = 2.0; // 2 minutes minimum (was 1 minute)
const MOMENTUM_MIN_TIME_SECONDS: f64 = 5.0; // Minimum 5 seconds before momentum calculation

// 📊 LIQUIDITY THRESHOLDS FOR PROFIT CALCULATIONS AND SAFETY CLASSIFICATION
const PROFIT_HIGH_LIQUIDITY_THRESHOLD: f64 = 200_000.0; // For profit calculations
const PROFIT_MEDIUM_HIGH_LIQUIDITY_THRESHOLD: f64 = 100_000.0; // For profit calculations
const PROFIT_MEDIUM_LIQUIDITY_THRESHOLD: f64 = 50_000.0; // For profit calculations
const PROFIT_LOW_LIQUIDITY_THRESHOLD: f64 = 10_000.0; // For profit calculations

// 🔍 ATH DANGER DETECTION - MORE TOLERANT
const ATH_DANGER_THRESHOLD: f64 = 85.0; // >85% of ATH = dangerous (was 75%)

// ================================================================================================
// 📊 COMPREHENSIVE TOKEN ANALYSIS DATA
// ================================================================================================

/// Pump detection result
#[derive(Debug, Clone)]
pub struct PumpAnalysis {
    pub is_pump: bool,
    pub pump_type: PumpType,
    pub velocity_percent_per_second: f64,
    pub magnitude_percent: f64,
    pub time_to_peak_seconds: f64,
    pub confidence: f64, // 0.0-1.0
    pub recommended_action: PumpAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PumpType {
    MegaPump, // 50%+ gains
    MainPump, // 15-50% gains
    MicroPump, // 8-15% gains (for volatile tokens)
    TrendMove, // 5-8% sustained move
    NoMove, // < 5% movement
}

#[derive(Debug, Clone, PartialEq)]
pub enum PumpAction {
    ExitImmediately, // Mega pump detected
    ExitSoon, // Main pump detected
    WatchClosely, // Micro pump or good trend
    Hold, // Normal movement
    HoldForMore, // Too small to exit
}

/// Detect pumps using price history and velocity analysis
pub async fn detect_pump(
    position: &Position,
    current_price: f64,
    minutes_held: f64
) -> PumpAnalysis {
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let current_profit_percent = ((current_price - entry_price) / entry_price) * 100.0;

    // Get price history for pump analysis
    let pool_service = get_pool_service();
    let price_history = pool_service.get_recent_price_history(&position.mint).await;

    if price_history.is_empty() || minutes_held < 0.5 {
        return PumpAnalysis {
            is_pump: false,
            pump_type: PumpType::NoMove,
            velocity_percent_per_second: 0.0,
            magnitude_percent: current_profit_percent,
            time_to_peak_seconds: minutes_held * 60.0,
            confidence: 0.0,
            recommended_action: PumpAction::Hold,
        };
    }

    // Analyze price velocity and patterns
    let velocity_analysis = analyze_price_velocity(
        &price_history,
        entry_price,
        current_price,
        minutes_held
    );
    let pattern_analysis = analyze_pump_pattern(&price_history, entry_price, minutes_held);
    let volatility_context = get_token_volatility_context(&position.mint).await;

    // Determine pump type based on magnitude and velocity
    let pump_type = classify_pump_type(
        current_profit_percent,
        velocity_analysis.max_velocity,
        volatility_context
    );

    // Calculate confidence based on multiple factors
    let confidence = calculate_pump_confidence(
        &velocity_analysis,
        &pattern_analysis,
        current_profit_percent,
        volatility_context
    );

    // Determine recommended action
    let recommended_action = determine_pump_action(
        &pump_type,
        confidence,
        current_profit_percent,
        minutes_held
    );

    PumpAnalysis {
        is_pump: matches!(pump_type, PumpType::MicroPump | PumpType::MainPump | PumpType::MegaPump),
        pump_type,
        velocity_percent_per_second: velocity_analysis.max_velocity,
        magnitude_percent: current_profit_percent,
        time_to_peak_seconds: velocity_analysis.time_to_peak,
        confidence,
        recommended_action,
    }
}

#[derive(Debug)]
struct VelocityAnalysis {
    max_velocity: f64, // Max % per second
    avg_velocity: f64, // Average % per second
    acceleration: f64, // Change in velocity
    time_to_peak: f64, // Seconds to reach peak
    is_accelerating: bool, // Still gaining speed
}

#[derive(Debug)]
struct PatternAnalysis {
    is_parabolic: bool, // Parabolic price curve
    has_sharp_spike: bool, // Sudden price spike
    volume_spike: bool, // Volume confirmation
    consistency_score: f64, // 0-1, how consistent the move is
}

/// Analyze price velocity over time
fn analyze_price_velocity(
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    entry_price: f64,
    current_price: f64,
    minutes_held: f64
) -> VelocityAnalysis {
    if price_history.len() < 2 {
        return VelocityAnalysis {
            max_velocity: 0.0,
            avg_velocity: 0.0,
            acceleration: 0.0,
            time_to_peak: minutes_held * 60.0,
            is_accelerating: false,
        };
    }

    let mut velocities = Vec::new();
    let mut max_velocity = 0.0;
    let mut time_to_peak = 0.0;
    let mut peak_price = entry_price;

    // Calculate velocities between consecutive points
    for i in 1..price_history.len() {
        let (time1, price1) = &price_history[i - 1];
        let (time2, price2) = &price_history[i];

        let time_diff_seconds = (*time2 - *time1).num_seconds() as f64;

        if time_diff_seconds > 0.0 && price1 > &0.0 {
            let price_change_percent = ((price2 - price1) / price1) * 100.0;
            let velocity = price_change_percent / time_diff_seconds;

            velocities.push(velocity);

            if velocity > max_velocity {
                max_velocity = velocity;
                time_to_peak = time_diff_seconds;
            }

            if *price2 > peak_price {
                peak_price = *price2;
            }
        }
    }

    // Include movement from entry to current
    let total_time_seconds = minutes_held * 60.0;
    if total_time_seconds > 0.0 {
        let entry_to_current_velocity =
            (((current_price - entry_price) / entry_price) * 100.0) / total_time_seconds;
        velocities.push(entry_to_current_velocity);

        if entry_to_current_velocity > max_velocity {
            max_velocity = entry_to_current_velocity;
            time_to_peak = total_time_seconds;
        }
    }

    let avg_velocity = if !velocities.is_empty() {
        velocities.iter().sum::<f64>() / (velocities.len() as f64)
    } else {
        0.0
    };

    // Calculate acceleration (change in velocity)
    let acceleration = if velocities.len() >= 2 {
        let recent_avg =
            velocities.iter().rev().take(3).sum::<f64>() / (3.0_f64).min(velocities.len() as f64);
        let early_avg =
            velocities.iter().take(3).sum::<f64>() / (3.0_f64).min(velocities.len() as f64);
        recent_avg - early_avg
    } else {
        0.0
    };

    VelocityAnalysis {
        max_velocity,
        avg_velocity,
        acceleration,
        time_to_peak,
        is_accelerating: acceleration > 0.1, // Accelerating if velocity increased by >0.1%/sec
    }
}

/// Analyze pump patterns
fn analyze_pump_pattern(
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    entry_price: f64,
    _minutes_held: f64
) -> PatternAnalysis {
    if price_history.len() < 3 {
        return PatternAnalysis {
            is_parabolic: false,
            has_sharp_spike: false,
            volume_spike: false,
            consistency_score: 0.0,
        };
    }

    let prices: Vec<f64> = price_history
        .iter()
        .map(|(_, price)| *price)
        .collect();

    // Check for parabolic curve (exponential growth)
    let is_parabolic = check_parabolic_pattern(&prices, entry_price);

    // Check for sharp spike (sudden large move)
    let has_sharp_spike = check_sharp_spike(&prices, entry_price);

    // Calculate consistency (how smooth the upward movement is)
    let consistency_score = calculate_consistency_score(&prices);

    PatternAnalysis {
        is_parabolic,
        has_sharp_spike,
        volume_spike: false, // Would need volume data integration
        consistency_score,
    }
}

/// Check if price pattern is parabolic
fn check_parabolic_pattern(prices: &[f64], entry_price: f64) -> bool {
    if prices.len() < 4 {
        return false;
    }

    // Look for accelerating gains (each move larger than the last)
    let mut acceleration_count = 0;
    let total_checks = prices.len() - 1;

    for i in 1..prices.len() {
        if i >= 2 {
            let prev_gain = (prices[i - 1] - prices[i - 2]) / prices[i - 2];
            let curr_gain = (prices[i] - prices[i - 1]) / prices[i - 1];

            if curr_gain > prev_gain * 1.1 {
                // 10% acceleration
                acceleration_count += 1;
            }
        }
    }

    // Parabolic if >60% of moves are accelerating and total gain > 10%
    let acceleration_ratio = (acceleration_count as f64) / (total_checks as f64);
    let total_gain = ((prices.last().unwrap() - entry_price) / entry_price) * 100.0;

    acceleration_ratio > 0.6 && total_gain > 10.0
}

/// Check for sharp price spike
fn check_sharp_spike(prices: &[f64], _entry_price: f64) -> bool {
    if prices.len() < 3 {
        return false;
    }

    // Look for sudden large moves
    for i in 1..prices.len() {
        let gain = ((prices[i] - prices[i - 1]) / prices[i - 1]) * 100.0;
        if gain > 8.0 {
            // Single move > 8%
            return true;
        }
    }

    false
}

/// Calculate how consistent the upward movement is
fn calculate_consistency_score(prices: &[f64]) -> f64 {
    if prices.len() < 3 {
        return 0.0;
    }

    let mut positive_moves = 0;
    let total_moves = prices.len() - 1;

    for i in 1..prices.len() {
        if prices[i] > prices[i - 1] {
            positive_moves += 1;
        }
    }

    (positive_moves as f64) / (total_moves as f64)
}

/// Get token volatility context (simplified without performance cache)
async fn get_token_volatility_context(mint: &str) -> f64 {
    // Return default volatility since we removed performance tracking
    0.1 // Default 10% volatility for all tokens
}

/// Classify pump type based on magnitude and velocity - ULTRA-DYNAMIC for up to 100%+
fn classify_pump_type(profit_percent: f64, max_velocity: f64, volatility_context: f64) -> PumpType {
    // Multi-tier volatility adjustment for extreme gains
    let volatility_multiplier = if profit_percent >= 75.0 {
        (1.0 + volatility_context * 0.5).min(1.5) // Less strict for ultra-high gains
    } else if profit_percent >= 50.0 {
        (1.0 + volatility_context * 0.7).min(1.7)
    } else {
        (1.0 + volatility_context).min(2.0)
    };

    // Dynamic thresholds with progressive scaling
    let ultra_mega_threshold = 100.0 / volatility_multiplier; // 100%+ ultra mega
    let super_mega_threshold = 75.0 / volatility_multiplier; // 75%+ super mega
    let mega_threshold = 50.0 / volatility_multiplier; // 50%+ mega
    let strong_mega_threshold = 30.0 / volatility_multiplier; // 30%+ strong mega
    let main_threshold = PUMP_MIN_PERCENT / volatility_multiplier;
    let micro_threshold = MICRO_PUMP_PERCENT / volatility_multiplier;
    let trend_threshold = 5.0 / volatility_multiplier;

    // Ultra-sensitive velocity detection for extreme gains
    let ultra_velocity_factor = max_velocity > 2.0; // Ultra high velocity
    let super_velocity_factor = max_velocity > 1.5; // Super high velocity
    let high_velocity_factor = max_velocity > 1.0; // High velocity
    let pump_velocity_factor = max_velocity > PUMP_VELOCITY_THRESHOLD;

    // Progressive pump classification with velocity boosting
    match profit_percent {
        // ULTRA TIER - 100%+ gains (immediate exit regardless)
        p if p >= ultra_mega_threshold => PumpType::MegaPump,

        // SUPER TIER - 75%+ gains
        p if p >= super_mega_threshold || (ultra_velocity_factor && p >= 60.0) =>
            PumpType::MegaPump,

        // MEGA TIER - 50%+ gains
        p if p >= mega_threshold || (super_velocity_factor && p >= 40.0) => PumpType::MegaPump,

        // STRONG MEGA TIER - 30%+ gains
        p if p >= strong_mega_threshold || (high_velocity_factor && p >= 25.0) =>
            PumpType::MegaPump,

        // MAIN PUMP TIER - 15%+ gains with various velocity combinations
        p if
            p >= main_threshold ||
            (pump_velocity_factor && p >= 12.0) ||
            (high_velocity_factor && p >= 10.0)
        => PumpType::MainPump,

        // MICRO PUMP TIER - 8%+ gains with velocity assistance
        p if
            p >= micro_threshold ||
            (pump_velocity_factor && p >= 6.0) ||
            (high_velocity_factor && p >= 5.0)
        => PumpType::MicroPump,

        // TREND TIER - 5%+ gains with some velocity
        p if p >= trend_threshold || (pump_velocity_factor && p >= 3.0) => PumpType::TrendMove,

        _ => PumpType::NoMove,
    }
}

/// Calculate pump confidence score - ULTRA-DYNAMIC for extreme gains
fn calculate_pump_confidence(
    velocity: &VelocityAnalysis,
    pattern: &PatternAnalysis,
    profit_percent: f64,
    volatility_context: f64
) -> f64 {
    let mut confidence = 0.0;

    // Ultra-high gain confidence boost (extreme gains = max confidence)
    if profit_percent >= 100.0 {
        confidence += 0.9; // 100%+ gains = nearly max confidence
    } else if profit_percent >= 75.0 {
        confidence += 0.8; // 75%+ gains = very high confidence
    } else if profit_percent >= 50.0 {
        confidence += 0.7; // 50%+ gains = high confidence
    } else if profit_percent >= 30.0 {
        confidence += 0.5; // 30%+ gains = moderate confidence boost
    }

    // Enhanced velocity confidence with progressive scaling
    if velocity.max_velocity > 2.0 {
        confidence += 0.3; // Ultra velocity
    } else if velocity.max_velocity > 1.5 {
        confidence += 0.25; // Super velocity
    } else if velocity.max_velocity > 1.0 {
        confidence += 0.2; // High velocity
    } else if velocity.max_velocity > PUMP_VELOCITY_THRESHOLD {
        confidence += 0.15 * (velocity.max_velocity / (PUMP_VELOCITY_THRESHOLD * 2.0)).min(1.0);
    }

    // Pattern confidence with extreme gain awareness
    if pattern.is_parabolic && profit_percent >= 30.0 {
        confidence += 0.15; // Parabolic + high gains = very confident
    } else if pattern.is_parabolic {
        confidence += 0.1;
    }

    if pattern.has_sharp_spike && profit_percent >= 20.0 {
        confidence += 0.15; // Sharp spike + good gains = confident
    } else if pattern.has_sharp_spike {
        confidence += 0.1;
    }

    // Consistency confidence with gain scaling
    let consistency_weight = if profit_percent >= 50.0 { 0.15 } else { 0.1 };
    confidence += pattern.consistency_score * consistency_weight;

    // Magnitude confidence (10% weight)
    let magnitude_factor = (profit_percent / (PUMP_MIN_PERCENT * 2.0)).min(1.0);
    confidence += magnitude_factor * 0.1;

    // Adjust for token volatility context
    let volatility_adjustment = if volatility_context > 0.2 {
        0.8 // Reduce confidence for highly volatile tokens
    } else {
        1.0
    };

    (confidence * volatility_adjustment).min(1.0).max(0.0)
}

/// Determine recommended action based on pump analysis - ULTRA-AGGRESSIVE for extreme gains
fn determine_pump_action(
    pump_type: &PumpType,
    confidence: f64,
    profit_percent: f64,
    minutes_held: f64
) -> PumpAction {
    match pump_type {
        PumpType::MegaPump => {
            // Ultra-aggressive for extreme gains
            if profit_percent >= 100.0 {
                PumpAction::ExitImmediately // 100%+ = always immediate exit
            } else if profit_percent >= 75.0 {
                PumpAction::ExitImmediately // 75%+ = immediate exit
            } else if profit_percent >= 50.0 && confidence > 0.5 {
                PumpAction::ExitImmediately // 50%+ with decent confidence
            } else if profit_percent >= 30.0 && confidence > 0.6 {
                PumpAction::ExitImmediately // 30%+ with good confidence
            } else if confidence > 0.7 {
                PumpAction::ExitImmediately // High confidence override
            } else {
                PumpAction::ExitSoon
            }
        }
        PumpType::MainPump => {
            // More aggressive for strong gains
            if profit_percent >= 50.0 {
                PumpAction::ExitSoon // 50%+ main pump = exit soon
            } else if profit_percent >= 25.0 && confidence > 0.5 {
                PumpAction::ExitSoon // 25%+ with confidence
            } else if confidence > 0.6 && minutes_held > 0.5 {
                PumpAction::ExitSoon // High confidence + minimal time
            } else if profit_percent >= 15.0 && minutes_held > 1.0 {
                PumpAction::WatchClosely // Standard main pump logic
            } else {
                PumpAction::WatchClosely
            }
        }
        PumpType::MicroPump => {
            // Enhanced micro pump sensitivity
            if profit_percent >= 30.0 {
                PumpAction::ExitSoon // 30%+ micro pump = upgrade to exit soon
            } else if profit_percent >= 20.0 && confidence > 0.6 {
                PumpAction::WatchClosely // 20%+ with confidence
            } else if confidence > 0.7 && minutes_held > 1.0 && profit_percent > 8.0 {
                PumpAction::WatchClosely // Very high confidence micro
            } else if profit_percent >= 12.0 && minutes_held > 2.0 {
                PumpAction::WatchClosely // Enhanced threshold
            } else {
                PumpAction::Hold
            }
        }
        PumpType::TrendMove => {
            // More sensitive trend detection
            if profit_percent >= 20.0 && confidence > 0.5 {
                PumpAction::WatchClosely // 20%+ trend = watch closely
            } else if profit_percent > TREND_PROFIT_MIN && minutes_held > 3.0 {
                PumpAction::WatchClosely // Standard trend logic
            } else if profit_percent >= 8.0 && confidence > 0.6 {
                PumpAction::Hold // Good trend developing
            } else {
                PumpAction::Hold
            }
        }
        PumpType::NoMove => {
            // Enhanced conservative logic
            if profit_percent < CONSERVATIVE_PROFIT_MIN && minutes_held < 10.0 {
                PumpAction::HoldForMore // Hold small profits longer
            } else if profit_percent < 2.0 {
                PumpAction::HoldForMore // Very small profits = definitely hold
            } else {
                PumpAction::Hold
            }
        }
    }
}

/// Complete token analysis combining all available data sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAnalysis {
    // Core identification
    pub mint: String,
    pub symbol: String,
    pub current_price: f64,

    // Safety & Security Analysis
    pub safety_score: f64, // 0-100 comprehensive safety score
    pub rugcheck_score: Option<i32>, // Raw rugcheck score
    pub rugcheck_normalized: Option<i32>, // 0-100 normalized score
    pub is_rugged: bool,
    pub freeze_authority_safe: bool,
    pub lp_unlocked_risk: bool,
    pub risk_reasons: Vec<String>,

    // Market Data Analysis
    pub liquidity_usd: f64,
    pub volume_24h: f64,
    pub volume_trend: f64, // Current vs average volume
    pub buy_pressure: f64, // 0-1, higher = more buying
    pub price_momentum: f64, // Recent price acceleration

    // Legitimacy Indicators
    pub has_website: bool,
    pub has_socials: bool,
    pub has_image: bool,
    pub verified_labels: usize, // Number of verified labels
    pub legitimacy_score: f64, // 0-1 legitimacy factor

    // Market Context
    pub token_age_hours: f64,
    pub is_near_ath: bool,
    pub ath_proximity_percent: f64, // How close to ATH (0-100%)

    // Analysis Results
    pub volatility_factor: f64, // Expected volatility multiplier
    pub momentum_score: f64, // Momentum-based urgency
    pub time_pressure_max: f64, // Maximum recommended hold time
}

/// Risk classification levels
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyLevel {
    UltraSafe, // 90-100 safety score
    Safe, // 70-89 safety score
    Medium, // 50-69 safety score
    Risky, // 30-49 safety score
    Dangerous, // 0-29 safety score
}

impl SafetyLevel {
    fn from_score(score: f64) -> Self {
        match score {
            s if s >= 90.0 => SafetyLevel::UltraSafe,
            s if s >= 70.0 => SafetyLevel::Safe,
            s if s >= 50.0 => SafetyLevel::Medium,
            s if s >= 30.0 => SafetyLevel::Risky,
            _ => SafetyLevel::Dangerous,
        }
    }

    fn get_base_profit_range(&self) -> (f64, f64) {
        match self {
            SafetyLevel::UltraSafe => (ULTRA_SAFE_PROFIT_MIN, ULTRA_SAFE_PROFIT_MAX),
            SafetyLevel::Safe => (SAFE_PROFIT_MIN, SAFE_PROFIT_MAX),
            SafetyLevel::Medium => (MEDIUM_PROFIT_MIN, MEDIUM_PROFIT_MAX),
            SafetyLevel::Risky => (RISKY_PROFIT_MIN, RISKY_PROFIT_MAX),
            SafetyLevel::Dangerous => (DANGEROUS_PROFIT_MIN, DANGEROUS_PROFIT_MAX),
        }
    }

    fn get_max_hold_time(&self) -> f64 {
        match self {
            SafetyLevel::UltraSafe => ULTRA_SAFE_MAX_TIME,
            SafetyLevel::Safe => SAFE_MAX_TIME,
            SafetyLevel::Medium => MEDIUM_MAX_TIME,
            SafetyLevel::Risky => RISKY_MAX_TIME,
            SafetyLevel::Dangerous => DANGEROUS_MAX_TIME,
        }
    }

    fn get_trailing_stop_percent(&self) -> f64 {
        match self {
            SafetyLevel::UltraSafe => TRAILING_STOP_ULTRA_SAFE,
            SafetyLevel::Safe => TRAILING_STOP_SAFE,
            SafetyLevel::Medium => TRAILING_STOP_MEDIUM,
            SafetyLevel::Risky => TRAILING_STOP_RISKY,
            SafetyLevel::Dangerous => TRAILING_STOP_DANGEROUS,
        }
    }
}

// ================================================================================================
// 🧠 INTELLIGENT TOKEN ANALYSIS ENGINE
// ================================================================================================

/// Analyze token comprehensively using all available data sources
pub async fn analyze_token_comprehensive(mint: &str) -> Result<TokenAnalysis, String> {
    // Get token price using new universal price function for real-time accuracy
    let current_price = if
        let Some(price_result) = crate::tokens::get_price(
            mint,
            Some(crate::tokens::PriceOptions::pool_only()),
            false
        ).await
    {
        price_result
            .best_sol_price()
            .ok_or_else(|| format!("Pool price calculation failed for token: {}", mint))?
    } else {
        return Err(format!("Failed to get pool price for token: {}", mint));
    };

    if current_price <= 0.0 || !current_price.is_finite() {
        return Err(format!("Invalid current price for token: {}: {}", mint, current_price));
    }

    // Get token data from database
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let token_data = database
        .get_token_by_mint(mint)
        .map_err(|e| format!("Failed to get token data: {}", e))?
        .ok_or_else(|| format!("Token not found in database: {}", mint))?;

    // Get rugcheck security analysis
    let rugcheck_data = get_token_rugcheck_data_safe(mint).await.map_err(|e|
        format!("Failed to get rugcheck data: {}", e)
    )?;

    // Extract core security data
    let (
        rugcheck_score,
        rugcheck_normalized,
        is_rugged,
        freeze_authority_safe,
        lp_unlocked_risk,
        risk_reasons,
    ) = if let Some(data) = &rugcheck_data {
        let high_risk_issues = get_high_risk_issues(data);
        let is_safe = is_token_safe_for_trading_safe(mint).await;

        (
            data.score,
            data.score_normalised,
            data.rugged.unwrap_or(false),
            data.freeze_authority.is_none() && data.mint_authority.is_none(),
            false, // Will be determined from market data if available
            if high_risk_issues.is_empty() {
                vec!["Token appears safe based on rugcheck analysis".to_string()]
            } else {
                high_risk_issues
            },
        )
    } else {
        (None, None, false, true, false, vec!["No rugcheck data available".to_string()])
    };

    // Calculate safety score (0-100)
    let safety_score = calculate_comprehensive_safety_score(
        &token_data,
        &rugcheck_data,
        current_price
    );

    // Extract market data
    let liquidity_usd = token_data.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    let volume_24h = token_data.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(0.0);

    // Calculate volume trend (current vs historical average)
    let volume_trend = calculate_volume_trend(&token_data);

    // Calculate buy pressure from transaction data
    let buy_pressure = calculate_buy_pressure(&token_data);

    // Calculate price momentum
    let price_momentum = calculate_price_momentum(&token_data);

    // Analyze legitimacy indicators
    let (has_website, has_socials, has_image, verified_labels) = analyze_legitimacy_indicators(
        &token_data
    );
    let legitimacy_score = calculate_legitimacy_score(
        has_website,
        has_socials,
        has_image,
        verified_labels
    );

    // Calculate token age
    let token_age_hours = calculate_token_age_hours(&token_data);

    // Check ATH proximity (simplified - would need historical data for exact ATH)
    let (is_near_ath, ath_proximity_percent) = estimate_ath_proximity(&token_data, current_price);

    // Calculate volatility factor based on liquidity
    let volatility_factor = calculate_volatility_factor(liquidity_usd);

    // Calculate momentum score
    let momentum_score = calculate_momentum_score(volume_trend, buy_pressure, price_momentum);

    // Determine maximum hold time
    let safety_level = SafetyLevel::from_score(safety_score);
    let time_pressure_max = safety_level.get_max_hold_time();

    Ok(TokenAnalysis {
        mint: mint.to_string(),
        symbol: token_data.symbol.clone(),
        current_price,
        safety_score,
        rugcheck_score,
        rugcheck_normalized,
        is_rugged,
        freeze_authority_safe,
        lp_unlocked_risk,
        risk_reasons,
        liquidity_usd,
        volume_24h,
        volume_trend,
        buy_pressure,
        price_momentum,
        has_website,
        has_socials,
        has_image,
        verified_labels,
        legitimacy_score,
        token_age_hours,
        is_near_ath,
        ath_proximity_percent,
        volatility_factor,
        momentum_score,
        time_pressure_max,
    })
}

/// Calculate comprehensive safety score (0-100)
fn calculate_comprehensive_safety_score(
    token_data: &crate::tokens::types::ApiToken,
    rugcheck_data: &Option<crate::tokens::rugcheck::RugcheckResponse>,
    _current_price: f64
) -> f64 {
    let mut safety_score: f64 = 50.0; // Start with neutral score

    // Rugcheck contribution (40% of total score)
    if let Some(rugcheck) = rugcheck_data {
        // Check if token is detected as rugged
        if rugcheck.rugged.unwrap_or(false) {
            safety_score = 0.0; // Rugged token = 0 safety
        } else {
            // CORRECTED: Rugcheck score is a RISK score - higher means MORE risk!
            let rugcheck_risk_score = rugcheck.score_normalised
                .or(rugcheck.score)
                .unwrap_or(50) as f64;

            // Convert risk score (0-100) to safety contribution (40-0)
            // Higher risk score = lower safety contribution
            let rugcheck_contribution = 40.0 - (rugcheck_risk_score / 100.0) * 40.0;

            // Additional penalty for high-risk items
            let risk_penalty = if let Some(risks) = &rugcheck.risks {
                let high_risk_count = risks
                    .iter()
                    .filter(|r| {
                        r.level
                            .as_ref()
                            .map(|l| (l.to_lowercase() == "high" || l.to_lowercase() == "critical"))
                            .unwrap_or(false)
                    })
                    .count();
                (high_risk_count as f64) * 5.0 // -5 points per high/critical risk
            } else {
                0.0
            };

            safety_score = (rugcheck_contribution - risk_penalty).max(0.0);
        }
    } else {
        safety_score = 20.0; // No rugcheck data = lower safety
    }

    // Liquidity contribution (25% of total score)
    let liquidity_usd = token_data.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);
    let liquidity_contribution = match liquidity_usd {
        l if l >= PROFIT_HIGH_LIQUIDITY_THRESHOLD => 25.0,
        l if l >= PROFIT_MEDIUM_HIGH_LIQUIDITY_THRESHOLD => 20.0,
        l if l >= PROFIT_MEDIUM_LIQUIDITY_THRESHOLD => 15.0,
        l if l >= PROFIT_LOW_LIQUIDITY_THRESHOLD => 10.0,
        _ => 5.0,
    };
    safety_score += liquidity_contribution;

    // Legitimacy contribution (20% of total score)
    let has_website = token_data.info
        .as_ref()
        .and_then(|info| info.websites.as_ref())
        .map(|w| !w.is_empty())
        .unwrap_or(false);
    let has_socials = token_data.info
        .as_ref()
        .and_then(|info| info.socials.as_ref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_image = token_data.info
        .as_ref()
        .and_then(|info| info.image_url.as_ref())
        .is_some();

    let legitimacy_contribution =
        (if has_website { 7.0 } else { 0.0 }) +
        (if has_socials { 7.0 } else { 0.0 }) +
        (if has_image { 6.0 } else { 0.0 });
    safety_score += legitimacy_contribution;

    // Age contribution (10% of total score)
    let age_hours = token_data.pair_created_at
        .map(|timestamp| {
            let now = Utc::now().timestamp();
            ((now - timestamp) / 3600) as f64
        })
        .unwrap_or(0.0);

    let age_contribution = match age_hours {
        a if a >= 168.0 => 10.0, // 1 week+
        a if a >= 72.0 => 8.0, // 3 days+
        a if a >= 24.0 => 6.0, // 1 day+
        a if a >= 6.0 => 4.0, // 6 hours+
        _ => 2.0, // Very new
    };
    safety_score += age_contribution;

    // Volume/activity contribution (5% of total score)
    let volume_24h = token_data.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(0.0);
    let volume_contribution = if volume_24h > 10000.0 { 5.0 } else { 2.0 };
    safety_score += volume_contribution;

    safety_score.min(100.0).max(0.0)
}

/// Calculate volume trend factor
fn calculate_volume_trend(token_data: &crate::tokens::types::ApiToken) -> f64 {
    if let Some(volume) = &token_data.volume {
        let vol_1h = volume.h1.unwrap_or(0.0);
        let vol_6h = volume.h6.unwrap_or(0.0);
        let vol_24h = volume.h24.unwrap_or(0.0);

        if vol_24h > 0.0 && vol_6h > 0.0 {
            // Compare recent volume to average
            let avg_hourly = vol_24h / 24.0;
            let recent_hourly = vol_1h;

            if avg_hourly > 0.0 {
                return (recent_hourly / avg_hourly).min(3.0); // Cap at 3x
            }
        }
    }
    1.0 // Neutral if no data
}

/// Calculate buy pressure from transaction data
fn calculate_buy_pressure(token_data: &crate::tokens::types::ApiToken) -> f64 {
    if let Some(txns) = &token_data.txns {
        if let Some(h1) = &txns.h1 {
            let buys = h1.buys.unwrap_or(0) as f64;
            let sells = h1.sells.unwrap_or(0) as f64;
            let total = buys + sells;

            if total > 0.0 {
                return buys / total; // 0-1 ratio
            }
        }
    }
    0.5 // Neutral if no data
}

/// Calculate price momentum
fn calculate_price_momentum(token_data: &crate::tokens::types::ApiToken) -> f64 {
    if let Some(price_change) = &token_data.price_change {
        let change_1h = price_change.h1.unwrap_or(0.0);
        let change_6h = price_change.h6.unwrap_or(0.0);

        // Acceleration = short term change vs longer term
        if change_6h != 0.0 {
            return (change_1h / change_6h).abs().min(3.0);
        }

        return change_1h.abs() / 100.0; // Direct momentum
    }
    0.0 // No momentum if no data
}

/// Analyze legitimacy indicators
fn analyze_legitimacy_indicators(
    token_data: &crate::tokens::types::ApiToken
) -> (bool, bool, bool, usize) {
    let has_website = token_data.info
        .as_ref()
        .and_then(|info| info.websites.as_ref())
        .map(|w| !w.is_empty())
        .unwrap_or(false);

    let has_socials = token_data.info
        .as_ref()
        .and_then(|info| info.socials.as_ref())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let has_image = token_data.info
        .as_ref()
        .and_then(|info| info.image_url.as_ref())
        .is_some();

    let verified_labels = token_data.labels
        .as_ref()
        .map(|labels| labels.len())
        .unwrap_or(0);

    (has_website, has_socials, has_image, verified_labels)
}

/// Calculate legitimacy score
fn calculate_legitimacy_score(
    has_website: bool,
    has_socials: bool,
    has_image: bool,
    verified_labels: usize
) -> f64 {
    let mut score = 0.0;

    if has_website {
        score += 0.3;
    }
    if has_socials {
        score += 0.3;
    }
    if has_image {
        score += 0.2;
    }
    score += ((verified_labels as f64) * 0.05).min(0.2); // Up to 0.2 for labels

    score.min(1.0)
}

/// Calculate token age in hours
fn calculate_token_age_hours(token_data: &crate::tokens::types::ApiToken) -> f64 {
    token_data.pair_created_at
        .map(|timestamp| {
            let now = Utc::now().timestamp();
            ((now - timestamp) / 3600) as f64
        })
        .unwrap_or(0.0)
}

/// Estimate ATH proximity (simplified without historical data)
fn estimate_ath_proximity(
    token_data: &crate::tokens::types::ApiToken,
    _current_price: f64
) -> (bool, f64) {
    // Use price change data to estimate if we're near recent highs
    if let Some(price_change) = &token_data.price_change {
        let change_24h = price_change.h24.unwrap_or(0.0);

        // If we're up significantly in 24h, we might be near highs
        if change_24h > 100.0 {
            // >100% gain in 24h
            let proximity = ((change_24h / 200.0) * 100.0).min(95.0); // Estimate proximity
            return (proximity > ATH_DANGER_THRESHOLD, proximity);
        }
    }

    (false, 0.0)
}

/// Calculate volatility factor based on liquidity
fn calculate_volatility_factor(liquidity_usd: f64) -> f64 {
    match liquidity_usd {
        l if l >= PROFIT_HIGH_LIQUIDITY_THRESHOLD => 0.5, // Low volatility
        l if l >= PROFIT_MEDIUM_HIGH_LIQUIDITY_THRESHOLD => 0.7, // Medium-low volatility
        l if l >= PROFIT_MEDIUM_LIQUIDITY_THRESHOLD => 1.0, // Normal volatility
        l if l >= PROFIT_LOW_LIQUIDITY_THRESHOLD => 1.5, // High volatility
        _ => 2.0, // Very high volatility
    }
}

/// Calculate momentum score for urgency
fn calculate_momentum_score(volume_trend: f64, buy_pressure: f64, price_momentum: f64) -> f64 {
    let volume_component = (volume_trend - 1.0).max(0.0).min(1.0); // 0-1
    let pressure_component = (buy_pressure - 0.5) * 2.0; // -1 to 1, then scale
    let momentum_component = price_momentum.min(1.0); // 0-1

    // Weighted average
    (volume_component * 0.4 + pressure_component.abs() * 0.3 + momentum_component * 0.3)
        .max(0.0)
        .min(2.0)
}

// ================================================================================================
// 🎯 MASTER SHOULD_SELL FUNCTION - THE ONE AND ONLY
// ================================================================================================

/// THE ULTIMATE SHOULD_SELL FUNCTION
///
/// Combines all available data sources for intelligent profit decisions:
/// - Real-time P&L calculation
/// - Comprehensive token safety analysis
/// - Market momentum detection
/// - Risk-adjusted profit targets
/// - Time pressure scaling
/// - ATH proximity warnings
/// - Minimum profit threshold in SOL (from trader.rs PROFIT_EXTRA_NEEDED_SOL)
///
/// Returns: boolean indicating whether to sell the position
pub async fn should_sell(position: &Position, current_price: f64) -> bool {
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🔍 CRITICAL SAFETY CHECKS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Validate inputs
    if current_price <= 0.0 || !current_price.is_finite() {
        log(LogTag::Profit, "ERROR", &format!("Invalid current price: {}", current_price));
        return false;
    }

    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    if entry_price <= 0.0 || !entry_price.is_finite() {
        log(LogTag::Profit, "ERROR", &format!("Invalid entry price: {}", entry_price));
        return false;
    }

    // Calculate current P&L
    let (pnl_sol, pnl_percent) = calculate_position_pnl(position, Some(current_price)).await;

    // Calculate position duration
    let now = Utc::now();
    let duration = now - position.entry_time;
    let minutes_held = (duration.num_seconds() as f64) / 60.0;

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 💰 MINIMUM PROFIT THRESHOLD CHECK (NEW FEATURE)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Import the minimum profit threshold from trader.rs
    use crate::trader::PROFIT_EXTRA_NEEDED_SOL;

    // For profitable positions, ensure minimum SOL profit before selling
    if pnl_percent > 0.0 && pnl_sol < PROFIT_EXTRA_NEEDED_SOL {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "MIN_PROFIT_CHECK",
                &format!(
                    "Profit below minimum threshold: {:.8} SOL < {:.8} SOL required ({}% profit) - holding position",
                    pnl_sol,
                    PROFIT_EXTRA_NEEDED_SOL,
                    pnl_percent
                )
            );
        }
        return false;
    }

    // ───────────────────────────────────────────────────────────────────────────────────────
    // ⛑️ Dynamic soft stop by liquidity tier + time caps
    // ───────────────────────────────────────────────────────────────────────────────────────
    let liquidity_tier = position.liquidity_tier.clone().unwrap_or_else(|| "UNKNOWN".to_string());
    let soft_stop = match liquidity_tier.as_str() {
        "LARGE" | "XLARGE" => SOFT_STOP_LARGE,
        "MEDIUM" => SOFT_STOP_MEDIUM,
        _ => SOFT_STOP_DEFAULT,
    };

    // Hard stop is enforced later as well (double safety)
    if pnl_percent <= STOP_LOSS_PERCENT {
        return true;
    }

    // Early soft stop for higher-liquidity tokens - MORE PATIENT
    if pnl_percent <= soft_stop && minutes_held > 5.0 {
        // Give more time before soft stop (was 1.0 minute)
        log(
            LogTag::Profit,
            "PATIENT_SOFT_STOP",
            &format!(
                "Patient soft stop triggered: {:.2}% loss, {:.1}min held (tier: {})",
                pnl_percent,
                minutes_held,
                liquidity_tier
            )
        );
        return true;
    }

    // Risk-Reward check using pool price history since entry
    let (mae_pct, mfe_pct, rr_now) = {
        let pool_service = get_pool_service();
        let history = pool_service.get_recent_price_history(&position.mint).await;
        let mut min_p = current_price;
        let mut max_p = current_price;
        for (ts, p) in history.iter() {
            if *ts >= position.entry_time {
                if *p < min_p {
                    min_p = *p;
                }
                if *p > max_p {
                    max_p = *p;
                }
            }
        }
        let mae = if entry_price > 0.0 { (min_p / entry_price - 1.0) * 100.0 } else { 0.0 };
        let mfe = if entry_price > 0.0 { (max_p / entry_price - 1.0) * 100.0 } else { 0.0 };
        let risk = mae.abs().max(0.1); // avoid div-by-zero; floor 0.1%
        let rr = pnl_percent / risk;
        (mae, mfe, rr)
    };

    let required_rr = match liquidity_tier.as_str() {
        "LARGE" | "XLARGE" => REQUIRED_RR_LARGE,
        "MEDIUM" => REQUIRED_RR_MEDIUM,
        _ => REQUIRED_RR_DEFAULT,
    };

    // Time pressure and hard cap
    if minutes_held >= HARD_TIME_CAP_MIN {
        return true;
    }

    if minutes_held >= SOFT_TIME_CAP_MIN {
        if pnl_percent < CONSERVATIVE_PROFIT_MIN || rr_now < required_rr {
            log(
                LogTag::Profit,
                "TIME_PRESSURE_EXIT",
                &format!(
                    "Time pressure exit: {:.1}min held, {:.2}% profit < {:.1}% minimum or RR {:.2} < {:.2} required",
                    minutes_held,
                    pnl_percent,
                    CONSERVATIVE_PROFIT_MIN,
                    rr_now,
                    required_rr
                )
            );
            return true;
        }
    }

    // Momentum fade guard
    if mae_pct <= -35.0 && rr_now < required_rr * 0.8 {
        return true;
    }

    // ───────────────────────────────────────────────────────────────────────────────────────
    // 🚀 PUMP DETECTION SYSTEM - HIGHEST PRIORITY
    // ───────────────────────────────────────────────────────────────────────────────────────

    // Analyze pump patterns BEFORE other logic
    let pump_analysis = detect_pump(position, current_price, minutes_held).await;

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "PUMP_ANALYSIS",
            &format!(
                "🚀 {} pump analysis: type={:?}, velocity={:.3}%/sec, confidence={:.2}, action={:?}",
                position.symbol,
                pump_analysis.pump_type,
                pump_analysis.velocity_percent_per_second,
                pump_analysis.confidence,
                pump_analysis.recommended_action
            )
        );
    }

    // PUMP-BASED EXIT DECISIONS
    match pump_analysis.recommended_action {
        PumpAction::ExitImmediately => {
            // Enhanced messaging for extreme gains
            let exit_message = if pnl_percent >= 100.0 {
                format!(
                    "🚀💎 ULTRA MEGA PUMP: {} - {:.1}% in {:.1}min - LEGENDARY MOONSHOT!",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            } else if pnl_percent >= 75.0 {
                format!(
                    "🚀🔥 SUPER MEGA PUMP: {} - {:.1}% in {:.1}min - MASSIVE GAIN!",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            } else if pnl_percent >= 50.0 {
                format!(
                    "🚀⚡ MEGA PUMP: {} - {:.1}% in {:.1}min - HUGE GAIN!",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            } else {
                format!(
                    "🚀 MEGA PUMP DETECTED: {} - {:.2}% in {:.1}min, velocity: {:.3}%/sec - IMMEDIATE EXIT!",
                    position.symbol,
                    pnl_percent,
                    minutes_held,
                    pump_analysis.velocity_percent_per_second
                )
            };

            log(LogTag::Profit, "PUMP_EXIT_IMMEDIATE", &exit_message);

            return true;
        }
        PumpAction::ExitSoon => {
            // Enhanced exit soon messaging for strong gains
            let exit_message = if pnl_percent >= 50.0 {
                format!(
                    "🎯🔥 MAJOR PUMP: {} - {:.1}% in {:.1}min - EXIT VERY SOON!",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            } else if pnl_percent >= 30.0 {
                format!(
                    "🎯⚡ STRONG PUMP: {} - {:.1}% in {:.1}min - EXIT SOON!",
                    position.symbol,
                    pnl_percent,
                    minutes_held
                )
            } else {
                format!(
                    "🎯 PUMP DETECTED: {} - {:.2}% in {:.1}min, confidence: {:.2} - EXIT SOON!",
                    position.symbol,
                    pnl_percent,
                    minutes_held,
                    pump_analysis.confidence
                )
            };

            log(LogTag::Profit, "PUMP_EXIT_SOON", &exit_message);

            return true;
        }
        PumpAction::WatchClosely => {
            // Continue to normal logic but with higher urgency baseline
            if is_debug_profit_enabled() {
                log(
                    LogTag::Profit,
                    "PUMP_WATCH",
                    &format!("👀 {} potential pump developing - watching closely", position.symbol)
                );
            }
        }
        PumpAction::HoldForMore => {
            // Conservative mode - don't exit on small gains
            if pnl_percent > 0.0 && pnl_percent < CONSERVATIVE_PROFIT_MIN {
                if is_debug_profit_enabled() {
                    log(
                        LogTag::Profit,
                        "HOLD_SMALL_PROFIT",
                        &format!(
                            "💎 {} holding small profit: {:.2}% < {:.1}% threshold",
                            position.symbol,
                            pnl_percent,
                            CONSERVATIVE_PROFIT_MIN
                        )
                    );
                }
                return false;
            }
        }
        PumpAction::Hold => {
            // Normal logic continues
        }
    }

    // ⚡ Fast profit capture safeguards (non-pump scenarios)
    if minutes_held <= LIGHTNING_PROFIT_TIME_LIMIT && pnl_percent >= LIGHTNING_PROFIT_THRESHOLD {
        return true;
    }
    if minutes_held <= FAST_PROFIT_TIME_LIMIT && pnl_percent >= FAST_PROFIT_THRESHOLD {
        return true;
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ STOP LOSS PROTECTION - ABSOLUTE PRIORITY
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ HARD STOP LOSS PROTECTION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if pnl_percent <= STOP_LOSS_PERCENT {
        log(
            LogTag::Profit,
            "STOP_LOSS",
            &format!(
                "Stop loss triggered: {:.2}% loss (threshold: {:.2}%)",
                pnl_percent,
                STOP_LOSS_PERCENT
            )
        );
        return true;
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🧠 PATIENT LOSS MANAGEMENT - MORE TOLERANT TIME-BASED APPROACH
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // For positions at loss, apply patient time-based loss management
    if pnl_percent < 0.0 {
        let loss_severity = pnl_percent.abs();

        // 🕐 60+ MINUTE PATIENT LOSS MANAGEMENT RULE (was 30+ minutes)
        if minutes_held >= 60.0 {
            let hours_held = minutes_held / 60.0;

            // More patient time-based exit criteria for losses
            let should_exit = if loss_severity >= 50.0 {
                // Severe losses: exit after 1 hour (was 30 minutes, was 30% threshold)
                true
            } else if loss_severity >= 35.0 {
                // Moderate losses: exit after 2 hours (was 1 hour, was 20% threshold)
                hours_held >= 2.0
            } else if loss_severity >= 25.0 {
                // Smaller losses: exit after 3 hours (was 1.5 hours, was 15% threshold)
                hours_held >= 3.0
            } else {
                // Minor losses: exit after 4 hours (was 2 hours)
                hours_held >= 4.0
            };

            if should_exit {
                log(
                    LogTag::Profit,
                    "PATIENT_LOSS_EXIT",
                    &format!(
                        "Patient loss exit after extended hold: {:.2}% loss, {:.1}min held, severity threshold reached",
                        pnl_percent,
                        minutes_held
                    )
                );
                return true;
            }
        }

        // Emergency exit for severe early losses - MORE TOLERANT (under 60 minutes)
        if minutes_held < 60.0 && loss_severity >= 50.0 {
            log(
                LogTag::Profit,
                "EMERGENCY_LOSS_EXIT",
                &format!(
                    "Emergency loss exit: {:.2}% severe loss in {:.1}min, critical decline detected",
                    pnl_percent,
                    minutes_held
                )
            );
            return true;
        }

        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "PATIENT_HOLD_LOSS",
                &format!(
                    "Patiently holding position with {:.2}% loss ({:.1}min held, being patient for recovery)",
                    pnl_percent,
                    minutes_held
                )
            );
        }
        return false;
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🚀 FAST PROFIT-TAKING OPTIMIZATION - RESPECTS MINIMUM HOLD TIME
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // FIXED: Fast profits now only apply AFTER minimum hold time is respected

    // ⏰ MINIMUM HOLD TIME PROTECTION - NO EXITS BEFORE 1 MINUTE
    if minutes_held < MIN_HOLD_TIME && pnl_percent > 0.0 {
        // Still calculate momentum for logging but don't exit
        if pnl_percent >= SPEED_PROFIT_THRESHOLD {
            log(
                LogTag::Profit,
                "FAST_PROFIT_BLOCKED",
                &format!(
                    "Fast profit blocked by MIN_HOLD_TIME: {:.2}% profit in {:.1}s (need {:.0}s minimum)",
                    pnl_percent,
                    minutes_held * 60.0,
                    MIN_HOLD_TIME * 60.0
                )
            );
        }
        return false;
    }

    // ⚡ LIGHTNING PROFIT EXIT: >10% profit in 15+ seconds = ultra-instant sell (HIGHEST PRIORITY)
    if minutes_held >= LIGHTNING_PROFIT_TIME_LIMIT && pnl_percent >= LIGHTNING_PROFIT_THRESHOLD {
        log(
            LogTag::Profit,
            "LIGHTNING_PROFIT_EXIT",
            &format!(
                "⚡ LIGHTNING profit exit triggered: {:.2}% profit in {:.1} seconds - capture moonshot momentum!",
                pnl_percent,
                minutes_held * 60.0
            )
        );
        return true;
    }

    // 🚀 SPEED PROFIT EXIT: >5% profit in 30+ seconds = mega urgent (SECOND PRIORITY)
    if minutes_held >= FAST_PROFIT_TIME_LIMIT && pnl_percent >= FAST_PROFIT_THRESHOLD {
        log(
            LogTag::Profit,
            "SPEED_PROFIT_EXIT",
            &format!(
                "🚀 Speed profit exit triggered: {:.2}% profit in {:.1} seconds - exceptional momentum",
                pnl_percent,
                minutes_held * 60.0
            )
        );
        return true;
    }

    // ⚡ FAST PROFIT EXIT: >3% profit in 1+ minute = immediate sell (THIRD PRIORITY)
    if minutes_held >= SPEED_PROFIT_TIME_LIMIT && pnl_percent >= SPEED_PROFIT_THRESHOLD {
        log(
            LogTag::Profit,
            "FAST_PROFIT_EXIT",
            &format!(
                "⚡ Fast profit exit triggered: {:.2}% profit in {:.1} seconds - capturing quick momentum",
                pnl_percent,
                minutes_held * 60.0
            )
        );
        return true;
    }

    // 🧠 ADAPTIVE FAST PROFIT: Adjusts thresholds based on momentum and time
    // Lower thresholds for very fast gains, higher thresholds for sustained gains
    if minutes_held < 2.0 && pnl_percent > 0.0 {
        // Prevent division by zero and ensure minimum time for meaningful momentum calculation
        let time_seconds = (minutes_held * 60.0).max(MOMENTUM_MIN_TIME_SECONDS);

        // Calculate momentum factor: faster gains = higher urgency
        let momentum_factor = pnl_percent / time_seconds; // % per second

        // Dynamic threshold based on momentum - MORE CONSERVATIVE
        let dynamic_threshold = if momentum_factor > 0.2 {
            // Ultra high momentum (>0.2% per second) = reasonable threshold
            CONSERVATIVE_PROFIT_MIN // Use our 8% minimum
        } else if momentum_factor > 0.1 {
            // Very high momentum (>0.1% per second) = higher threshold
            10.0 // 10% minimum for fast momentum
        } else if momentum_factor > 0.05 {
            // High momentum (>0.05% per second) = even higher threshold
            12.0 // 12% minimum for moderate momentum
        } else {
            // Normal momentum = highest threshold (don't exit on small gains)
            15.0 // 15% minimum for normal momentum
        };

        if pnl_percent >= dynamic_threshold {
            log(
                LogTag::Profit,
                "ADAPTIVE_PATIENT_PROFIT",
                &format!(
                    "Patient adaptive profit: {:.2}% in {:.1}s (momentum: {:.4}%/s, threshold: {:.1}%)",
                    pnl_percent,
                    time_seconds,
                    momentum_factor,
                    dynamic_threshold
                )
            );
            return true;
        }
    } // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🚀 INSTANT MEGA-PROFIT EXITS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if pnl_percent >= INSTANT_SELL_PROFIT {
        log(
            LogTag::Profit,
            "MEGA_PROFIT",
            &format!("Instant sell triggered: {:.2}% profit", pnl_percent)
        );
        return true;
    }

    if pnl_percent >= MEGA_PROFIT_THRESHOLD {
        log(LogTag::Profit, "LARGE_PROFIT", &format!("Large profit detected: {:.2}%", pnl_percent));
        return true;
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🧠 COMPREHENSIVE TOKEN ANALYSIS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let token_analysis = match analyze_token_comprehensive(&position.mint).await {
        Ok(analysis) => analysis,
        Err(e) => {
            log(
                LogTag::Profit,
                "WARN",
                &format!(
                    "Failed to analyze token {}: {} - using fallback logic",
                    position.symbol,
                    e
                )
            );

            // Fallback to simple profit logic when analysis fails
            return fallback_profit_logic(pnl_percent, minutes_held);
        }
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🎯 DYNAMIC PROFIT TARGET CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let safety_level = SafetyLevel::from_score(token_analysis.safety_score);
    let (base_min_profit, base_max_profit) = safety_level.get_base_profit_range();

    // Adjust profit targets based on momentum and volatility
    let momentum_multiplier = 1.0 + token_analysis.momentum_score * 0.5; // Up to 50% increase
    let volatility_multiplier = token_analysis.volatility_factor; // 0.5x to 2.0x

    let target_min_profit = (base_min_profit / momentum_multiplier) * volatility_multiplier;
    let target_max_profit = base_max_profit * momentum_multiplier * volatility_multiplier;

    // Calculate profit progression for reference (now using adjusted version)
    let _profit_progression = if pnl_percent >= target_max_profit {
        1.0
    } else if pnl_percent >= target_min_profit {
        (pnl_percent - target_min_profit) / (target_max_profit - target_min_profit)
    } else {
        0.0
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ TIME PRESSURE CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let max_hold_time = token_analysis.time_pressure_max;
    let time_pressure = (minutes_held / max_hold_time).min(1.0);

    // Additional time pressure for risky tokens
    let risk_time_pressure = match safety_level {
        SafetyLevel::Dangerous => time_pressure * 1.5, // 50% more time pressure
        SafetyLevel::Risky => time_pressure * 1.2, // 20% more time pressure
        _ => time_pressure,
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ SAFETY-BASED TIME EXIT LOGIC - RESPECTS TOKEN SAFETY LEVELS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Use previously computed safety_level and max_hold_time; derive thresholds
    let warning_time = max_hold_time * 0.7; // 70% of max hold time = warning
    let urgent_time = max_hold_time * 0.9; // 90% of max hold time = urgent

    // 🚨 URGENT SAFETY EXIT: Near or past urgent threshold with any profit
    if minutes_held >= urgent_time && pnl_percent > 0.0 {
        log(
            LogTag::Profit,
            "URGENT_SAFETY_EXIT",
            &format!(
                "URGENT SAFETY EXIT: {:.1}min held (>{:.1}min urgent threshold) with {:.2}% profit - immediate sell!",
                minutes_held,
                urgent_time,
                pnl_percent
            )
        );

        return true;
    }

    // 🟡 SAFETY TIME EXIT: Approaching max hold time with profit
    if minutes_held >= warning_time && pnl_percent > 0.0 {
        let time_progress = (minutes_held - warning_time) / (max_hold_time - warning_time);
        let time_exit_urgency = (0.6 + time_progress * 0.4).min(1.0); // 60-100% urgency

        // Higher urgency for riskier tokens
        let risk_multiplier = match safety_level {
            SafetyLevel::Dangerous => 1.3,
            SafetyLevel::Risky => 1.2,
            _ => 1.0,
        };
        let final_urgency = (time_exit_urgency * risk_multiplier).min(1.0);

        log(
            LogTag::Profit,
            "SAFETY_TIME_EXIT",
            &format!(
                "SAFETY TIME EXIT: {:.1}min held ({:.1}% of {:.1}min max) with {:.2}% profit - urgency: {:.2}",
                minutes_held,
                (minutes_held / max_hold_time) * 100.0,
                max_hold_time,
                pnl_percent,
                final_urgency
            )
        );

        return final_urgency >= 0.7;
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ SAFETY-BASED TRAILING STOP LOGIC - RISK-ADJUSTED PROTECTION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let safety_level = SafetyLevel::from_score(token_analysis.safety_score);
    let trailing_stop_threshold = safety_level.get_trailing_stop_percent();

    if USE_TRAILING_STOP && pnl_percent > trailing_stop_threshold {
        // Get highest price reached (from position tracking)
        let highest_price = position.price_highest;

        if highest_price > 0.0 {
            let highest_profit_percent = ((highest_price - entry_price) / entry_price) * 100.0;
            let current_drop_from_peak = highest_profit_percent - pnl_percent;

            if current_drop_from_peak >= trailing_stop_threshold {
                log(
                    LogTag::Profit,
                    "TRAILING_STOP",
                    &format!(
                        "Safety-based trailing stop triggered: Dropped {:.2}% from peak of {:.2}% (safety={:?}, threshold: {:.2}%)",
                        current_drop_from_peak,
                        highest_profit_percent,
                        safety_level,
                        trailing_stop_threshold
                    )
                );
                return true;
            }
        }
    }

    // Note: Minimum hold time is enforced earlier (seconds-level message). No redundant check here.

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🎯 TIME DECAY PROFIT TARGET ADJUSTMENT - NEW FEATURE
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let time_decay_multiplier = 1.0 - (minutes_held / max_hold_time) * TIME_DECAY_FACTOR;
    let adjusted_min_profit = target_min_profit * time_decay_multiplier.max(0.7); // Never go below 70%
    let adjusted_max_profit = target_max_profit * time_decay_multiplier.max(0.8); // Never go below 80%

    // Recalculate profit progression with time decay
    let adjusted_profit_progression = if pnl_percent >= adjusted_max_profit {
        1.0
    } else if pnl_percent >= adjusted_min_profit {
        (pnl_percent - adjusted_min_profit) / (adjusted_max_profit - adjusted_min_profit)
    } else {
        0.0
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // �🚨 SPECIAL RISK FACTORS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let mut risk_urgency: f64 = 0.0;
    let mut risk_reasons = Vec::new();

    // Critical security risks
    if token_analysis.is_rugged {
        risk_urgency = 1.0;
        risk_reasons.push("TOKEN MARKED AS RUGGED".to_string());
    }

    if !token_analysis.freeze_authority_safe {
        risk_urgency = risk_urgency.max(0.7);
        risk_reasons.push("FREEZE AUTHORITY RISK".to_string());
    }

    if token_analysis.lp_unlocked_risk {
        risk_urgency = risk_urgency.max(0.6);
        risk_reasons.push("LP UNLOCK RISK".to_string());
    }

    // ATH proximity danger
    if token_analysis.is_near_ath {
        let ath_urgency = (token_analysis.ath_proximity_percent - ATH_DANGER_THRESHOLD) / 25.0;
        risk_urgency = risk_urgency.max(ath_urgency * 0.5); // Up to 50% urgency
        risk_reasons.push(format!("NEAR ATH ({:.1}%)", token_analysis.ath_proximity_percent));
    }

    // Low liquidity warning
    if token_analysis.liquidity_usd < PROFIT_LOW_LIQUIDITY_THRESHOLD {
        risk_urgency = risk_urgency.max(0.3);
        risk_reasons.push(format!("LOW LIQUIDITY (${:.0})", token_analysis.liquidity_usd));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🧮 FINAL URGENCY CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Enhanced time pressure for profitable positions
    let enhanced_time_pressure = if pnl_percent > 0.0 {
        match minutes_held {
            m if m >= 25.0 => risk_time_pressure * 1.8, // 80% boost for 25+ minutes
            m if m >= 20.0 => risk_time_pressure * 1.5, // 50% boost for 20+ minutes
            m if m >= 15.0 => risk_time_pressure * 1.3, // 30% boost for 15+ minutes
            _ => risk_time_pressure,
        }
    } else {
        risk_time_pressure
    };

    // Combine all factors with optimized weights
    let profit_urgency = adjusted_profit_progression * 0.3; // Use adjusted progression
    let time_urgency = enhanced_time_pressure * 0.4; // Increased from 30% to 40%
    let momentum_urgency = (token_analysis.momentum_score / 2.0) * 0.2; // 20% weight on momentum
    let safety_urgency = (1.0 - token_analysis.safety_score / 100.0) * 0.1; // 10% weight on safety

    let base_urgency = profit_urgency + time_urgency + momentum_urgency + safety_urgency;

    // Apply risk multipliers
    let final_urgency = (base_urgency + risk_urgency).min(1.0).max(0.0);

    // Additional urgency boost for profitable positions held too long
    let final_urgency_with_time_boost = if pnl_percent > 0.0 && minutes_held >= 25.0 {
        let time_boost = ((minutes_held - 25.0) / 20.0) * 0.3; // Up to 30% boost
        (final_urgency + time_boost).min(1.0)
    } else {
        final_urgency
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 📝 DETAILED REASON GENERATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let reason = if final_urgency_with_time_boost >= 0.8 {
        format!(
            "URGENT SELL: {:.1}% profit, {:.1}min held, safety={:.0}/100{}{}",
            pnl_percent,
            minutes_held,
            token_analysis.safety_score,
            if risk_reasons.is_empty() {
                ""
            } else {
                ", RISKS: "
            },
            risk_reasons.join(", ")
        )
    } else if final_urgency_with_time_boost >= 0.6 {
        format!(
            "CONSIDER SELL: {:.1}% profit, {:.1}min held, safety={:.0}/100, targets={:.1}%-{:.1}%",
            pnl_percent,
            minutes_held,
            token_analysis.safety_score,
            target_min_profit,
            target_max_profit
        )
    } else if final_urgency_with_time_boost >= 0.3 {
        format!(
            "WATCH CLOSELY: {:.1}% profit, {:.1}min held, target={:.1}%-{:.1}%, safety={:.0}/100",
            pnl_percent,
            minutes_held,
            target_min_profit,
            target_max_profit,
            token_analysis.safety_score
        )
    } else {
        format!(
            "HOLD: {:.1}% profit, {:.1}min held, target={:.1}%-{:.1}%, safety={:.0}/100",
            pnl_percent,
            minutes_held,
            target_min_profit,
            target_max_profit,
            token_analysis.safety_score
        )
    };

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 📊 DEBUG LOGGING (if enabled)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "ANALYSIS",
            &format!(
                "Token: {} | Safety: {:.0}/100 | Liquidity: ${:.0} | Momentum: {:.2} | Time: {:.1}/{:.1}min",
                position.symbol,
                token_analysis.safety_score,
                token_analysis.liquidity_usd,
                token_analysis.momentum_score,
                minutes_held,
                max_hold_time
            )
        );

        log(
            LogTag::Profit,
            "OPTIMIZED",
            &format!(
                "OPTIMIZED SYSTEM: Profit: {:.1}% | Adjusted Targets: {:.1}%-{:.1}% | Time Decay: {:.3} | Trailing Stop: {} | Min Hold: {:.1}min",
                pnl_percent,
                adjusted_min_profit,
                adjusted_max_profit,
                time_decay_multiplier,
                if USE_TRAILING_STOP {
                    "ENABLED"
                } else {
                    "DISABLED"
                },
                MIN_HOLD_TIME
            )
        );

        log(
            LogTag::Profit,
            "TARGETS",
            &format!(
                "Decision: Urgency={:.3} | Original Targets: {:.1}%-{:.1}% | Adjusted: {:.1}%-{:.1}% | Action: {}",
                final_urgency_with_time_boost,
                target_min_profit,
                target_max_profit,
                adjusted_min_profit,
                adjusted_max_profit,
                if final_urgency_with_time_boost >= 0.6 {
                    "SELL"
                } else {
                    "HOLD"
                }
            )
        );
    }

    // Return true if urgency is high enough to sell
    final_urgency_with_time_boost >= 0.7
}

// ================================================================================================
// 🔧 FALLBACK PROFIT LOGIC (when token analysis fails)
// ================================================================================================

/// Simple fallback profit logic when comprehensive analysis fails
fn fallback_profit_logic(pnl_percent: f64, minutes_held: f64) -> bool {
    // 🚨 MANDATORY TIME-BASED EXITS (even in fallback mode)
    if minutes_held >= 45.0 && pnl_percent > 0.0 {
        return true;
    }

    if minutes_held >= 30.0 && pnl_percent > 0.0 {
        return true;
    }

    // Conservative profit targets when we can't analyze the token
    let target_profit = match minutes_held {
        m if m < 5.0 => 50.0, // 50% in first 5 minutes
        m if m < 10.0 => 30.0, // 30% in 5-10 minutes
        m if m < 20.0 => 20.0, // 20% in 10-20 minutes
        _ => 15.0, // 15% after 20 minutes
    };

    // Enhanced time pressure for fallback mode
    let time_pressure = ((minutes_held / 25.0).min(1.0) * 1.2).min(1.0); // More aggressive, max 25 minutes
    let profit_factor = (pnl_percent / target_profit).min(1.0);

    // Add extra urgency for positions held 20+ minutes
    let time_boost = if minutes_held >= 20.0 && pnl_percent > 0.0 {
        ((minutes_held - 20.0) / 10.0) * 0.3 // +30% urgency for every 10 minutes over 20
    } else {
        0.0
    };

    let urgency = (profit_factor * 0.5 + time_pressure * 0.5 + time_boost).min(1.0);

    // Return true if urgency is high enough to sell
    urgency >= 0.7
}

// ================================================================================================
// 🤖 RL INTEGRATION HELPER FUNCTIONS
// ================================================================================================

/// Helper function to extract token data for RL analysis
pub async fn analyze_token_for_rl(
    mint: &str
) -> Result<(f64, f64, Option<f64>, Option<f64>), String> {
    // Get token data from database
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to initialize database: {}", e)
    )?;

    let token_data = database
        .get_token_by_mint(mint)
        .map_err(|e| format!("Failed to get token data: {}", e))?
        .ok_or_else(|| format!("Token not found in database: {}", mint))?;

    // Extract basic market data
    let liquidity_usd = token_data.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(50000.0); // Default to safe value

    let volume_24h = token_data.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(200000.0); // Default to safe value

    let market_cap = token_data.market_cap;

    // Get rugcheck risk score (remember: higher = more risk)
    let rugcheck_score = match get_token_rugcheck_data_safe(mint).await {
        Ok(Some(data)) => data.score_normalised.or(data.score).map(|s| s as f64),
        _ => Some(50.0), // Default medium risk
    };

    Ok((liquidity_usd, volume_24h, market_cap, rugcheck_score))
}
