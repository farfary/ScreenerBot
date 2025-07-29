use crate::global::*;
use crate::tokens::Token;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };

// ================================================================================================
// 🎯 ENHANCED TIME-BASED PROFIT SYSTEM - CURVE-OPTIMIZED VERSION
// ================================================================================================
// Follows the bell curve pattern more precisely
// Peak profit zone: 15-25 minutes  
// Declining efficiency after 30 minutes
// Enhanced momentum and volatility adaptation
// ================================================================================================

// 📊 CURVE-OPTIMIZED PROFIT TARGETS (MINUTES -> PROFIT%)
// Based on the bell curve analysis: rapid rise, peak at ~20-25min, gradual decline
const CURVE_TARGETS: [(f64, f64); 12] = [
    (0.5, 12.0),   // 30 seconds: 12% (ultra-aggressive)
    (1.0, 18.0),   // 1 minute: 18% (very fast)
    (2.0, 28.0),   // 2 minutes: 28% (rapid rise)
    (3.0, 40.0),   // 3 minutes: 40% (momentum building)
    (5.0, 55.0),   // 5 minutes: 55% (strong momentum)
    (8.0, 75.0),   // 8 minutes: 75% (approaching peak)
    (12.0, 95.0),  // 12 minutes: 95% (pre-peak)
    (18.0, 130.0), // 18 minutes: 130% (peak zone start)
    (22.0, 160.0), // 22 minutes: 160% (peak zone)
    (25.0, 180.0), // 25 minutes: 180% (absolute peak)
    (30.0, 150.0), // 30 minutes: 150% (post-peak decline)
    (45.0, 100.0), // 45 minutes: 100% (significant decline)
];

// 🚀 MOMENTUM-BASED MULTIPLIERS
const MOMENTUM_STRONG_MULTIPLIER: f64 = 0.75;  // Reduce targets by 25% for strong momentum
const MOMENTUM_FADING_MULTIPLIER: f64 = 0.65;  // Reduce targets by 35% for fading momentum
const CRITICAL_DIP_MULTIPLIER: f64 = 0.40;     // Reduce targets by 60% for critical dips

// 📈 VOLATILITY-BASED MULTIPLIERS  
const HIGH_VOLATILITY_MULTIPLIER: f64 = 0.70;  // Take profits faster in volatile markets
const LOW_VOLATILITY_MULTIPLIER: f64 = 1.15;   // Wait longer in stable markets

// 💰 POSITION SIZE MULTIPLIERS
const LARGE_POSITION_MULTIPLIER: f64 = 0.80;   // Exit larger positions faster
const MEDIUM_POSITION_MULTIPLIER: f64 = 0.90;  // Slight reduction for medium positions

// 🔒 ENHANCED STOP LOSS & SAFETY
pub const ENHANCED_STOP_LOSS_PERCENT: f64 = -55.0;  // Keep existing stop loss
const MINIMUM_HOLD_TIME_SECONDS: f64 = 20.0;        // Minimum 20 seconds before any sell
const EXTREME_PROFIT_THRESHOLD: f64 = 800.0;        // Instant sell at 800%+
const FORCE_SELL_TIME_MINUTES: f64 = 75.0;          // Force sell after 75 minutes
const FORCE_SELL_MIN_PROFIT: f64 = 3.0;             // Minimum 3% for force sell

// ================================================================================================
// 🎯 ENHANCED SHOULD_SELL FUNCTION - CURVE-OPTIMIZED
// ================================================================================================

/// Enhanced should_sell function that follows the profit curve more precisely
/// 
/// Key improvements:
/// - Better curve adherence with more granular targets
/// - Dynamic momentum adaptation  
/// - Volatility-based adjustments
/// - Position size considerations
/// - Multi-timeframe analysis integration
pub fn should_sell_enhanced(position: &Position, current_price: f64, token: Option<&Token>) -> (f64, String) {
    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🔍 INPUT VALIDATION & SAFETY CHECKS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if current_price <= 0.0 || !current_price.is_finite() {
        log(
            LogTag::Profit,
            "ERROR",
            &format!(
                "INVALID PRICE for enhanced sell analysis: {} - Price = {:.10}",
                position.symbol,
                current_price
            )
        );
        return (0.0, format!("❌ INVALID PRICE: {:.10}", current_price));
    }

    // Calculate basic parameters
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let now = Utc::now();
    let duration = now - position.entry_time;
    let time_held_seconds = duration.num_seconds() as f64;
    let minutes_held = time_held_seconds / 60.0;
    let current_profit_percent = ((current_price - entry_price) / entry_price) * 100.0;

    // Minimum hold time protection
    if time_held_seconds < MINIMUM_HOLD_TIME_SECONDS {
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "⏱️ MINIMUM HOLD: {} held for {:.1}s < {:.0}s minimum",
                    position.symbol,
                    time_held_seconds,
                    MINIMUM_HOLD_TIME_SECONDS
                )
            );
        }
        return (0.0, format!("⏱️ HOLD: {:.1}s < {:.0}s min", time_held_seconds, MINIMUM_HOLD_TIME_SECONDS));
    }

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "🔍 PROFIT-DEBUG",
            &format!(
                "Enhanced Analysis: {} | Price: {:.8} → {:.8} | Profit: {:.2}% | Time: {:.1}m",
                position.symbol,
                entry_price,
                current_price,
                current_profit_percent,
                minutes_held
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ STOP LOSS PROTECTION - ENHANCED VERSION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if current_profit_percent <= ENHANCED_STOP_LOSS_PERCENT {
        log(
            LogTag::Profit,
            "🚨 STOP_LOSS",
            &format!(
                "ENHANCED STOP LOSS: {} at {:.2}% - MANDATORY EXIT",
                position.symbol,
                current_profit_percent
            )
        );
        return (1.0, format!("🚨 STOP LOSS: {:.1}% loss", current_profit_percent));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🚀 EXTREME PROFIT PROTECTION (INSTANT SELLS)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if current_profit_percent >= EXTREME_PROFIT_THRESHOLD {
        log(
            LogTag::Profit,
            "💎 EXTREME",
            &format!(
                "EXTREME PROFIT (ENHANCED): {} at {:.1}% in {:.1}m - INSTANT SELL",
                position.symbol,
                current_profit_percent,
                minutes_held
            )
        );
        return (0.99, format!("💎 EXTREME: {:.0}% - INSTANT SELL", current_profit_percent));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ ENHANCED FORCE SELL (EXTENDED TIME LIMIT)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if minutes_held >= FORCE_SELL_TIME_MINUTES && current_profit_percent >= FORCE_SELL_MIN_PROFIT {
        log(
            LogTag::Profit,
            "⏰ FORCE",
            &format!(
                "ENHANCED FORCE SELL: {} at {:.1}% after {:.0}m",
                position.symbol,
                current_profit_percent,
                minutes_held
            )
        );
        return (0.95, format!("⏰ FORCE: {:.1}% after {:.0}m", current_profit_percent, minutes_held));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 📊 CURVE-OPTIMIZED TARGET CALCULATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let base_target = get_curve_optimized_target(minutes_held);
    let mut adjusted_target = base_target;

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "🔍 PROFIT-DEBUG",
            &format!(
                "Curve Target: {} at {:.1}m needs {:.1}% (base curve target)",
                position.symbol,
                minutes_held,
                base_target
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🌊 DYNAMIC ADJUSTMENTS BASED ON MARKET CONDITIONS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let mut adjustment_factors = Vec::new();
    let mut adjustment_reasons = Vec::new();

    // Momentum-based adjustments
    if let Some(token_data) = token {
        let momentum_multiplier = calculate_momentum_multiplier(token_data, current_profit_percent);
        if momentum_multiplier != 1.0 {
            adjusted_target *= momentum_multiplier;
            adjustment_factors.push(momentum_multiplier);
            adjustment_reasons.push(format!("momentum({}x)", momentum_multiplier));
        }

        // Volatility-based adjustments
        let volatility_multiplier = calculate_volatility_multiplier(token_data);
        if volatility_multiplier != 1.0 {
            adjusted_target *= volatility_multiplier;
            adjustment_factors.push(volatility_multiplier);
            adjustment_reasons.push(format!("volatility({}x)", volatility_multiplier));
        }
    }

    // Position size adjustments
    let position_multiplier = calculate_position_size_multiplier(position);
    if position_multiplier != 1.0 {
        adjusted_target *= position_multiplier;
        adjustment_factors.push(position_multiplier);
        adjustment_reasons.push(format!("size({}x)", position_multiplier));
    }

    if is_debug_profit_enabled() && !adjustment_factors.is_empty() {
        log(
            LogTag::Profit,
            "🔍 PROFIT-DEBUG",
            &format!(
                "Adjustments: {} | {:.1}% → {:.1}% | Factors: {}",
                position.symbol,
                base_target,
                adjusted_target,
                adjustment_reasons.join(", ")
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🎯 ENHANCED TARGET EVALUATION
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if current_profit_percent >= adjusted_target {
        // Calculate urgency based on curve position and adjustments
        let curve_position = get_curve_position_factor(minutes_held);
        let base_urgency = calculate_base_urgency(current_profit_percent, adjusted_target, curve_position);
        
        // Apply momentum urgency boosts
        let final_urgency = if let Some(token_data) = token {
            apply_momentum_urgency_boost(base_urgency, token_data, current_profit_percent)
        } else {
            base_urgency
        };

        let speed_category = get_speed_category(minutes_held);
        
        if is_debug_profit_enabled() {
            log(
                LogTag::Profit,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "✅ TARGET HIT: {} | {:.1}% ≥ {:.1}% | Category: {:?} | Urgency: {:.2}",
                    position.symbol,
                    current_profit_percent,
                    adjusted_target,
                    speed_category,
                    final_urgency
                )
            );
        }

        return (
            final_urgency,
            format!(
                "🎯 {}: {:.0}% in {:.1}m (target: {:.0}%)",
                get_speed_category_emoji(&speed_category),
                current_profit_percent,
                minutes_held,
                adjusted_target
            ),
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏳ ENHANCED WAITING LOGIC
    // ═══════════════════════════════════════════════════════════════════════════════════════

    // Calculate progress toward target
    let progress = if adjusted_target > 0.0 {
        (current_profit_percent / adjusted_target).min(1.0).max(0.0)
    } else {
        0.0
    };

    // Time pressure calculation (enhanced curve-based)
    let time_pressure = calculate_curve_based_time_pressure(minutes_held, current_profit_percent);

    if is_debug_profit_enabled() {
        log(
            LogTag::Profit,
            "🔍 PROFIT-DEBUG",
            &format!(
                "⏳ WAITING: {} | {:.1}% / {:.1}% ({:.0}% progress) | Time pressure: {:.2}",
                position.symbol,
                current_profit_percent,
                adjusted_target,
                progress * 100.0,
                time_pressure
            )
        );
    }

    // Return appropriate waiting state
    if time_pressure > 0.3 {
        (time_pressure, format!("⏰ TIME PRESSURE: {:.1}% in {:.1}m", current_profit_percent, minutes_held))
    } else if progress > 0.5 {
        (0.1, format!("📈 PROGRESS: {:.1}%/{:.1}% ({:.0}%)", current_profit_percent, adjusted_target, progress * 100.0))
    } else {
        (0.0, format!("⏳ WAIT: {:.1}% (need {:.1}%) in {:.1}m", current_profit_percent, adjusted_target, minutes_held))
    }
}

// ================================================================================================
// 🧮 CURVE-OPTIMIZED HELPER FUNCTIONS
// ================================================================================================

/// Get curve-optimized profit target for a given duration
fn get_curve_optimized_target(minutes_held: f64) -> f64 {
    // Find the appropriate target using linear interpolation between curve points
    for window in CURVE_TARGETS.windows(2) {
        let (time1, target1) = window[0];
        let (time2, target2) = window[1];
        
        if minutes_held <= time1 {
            return target1;
        } else if minutes_held <= time2 {
            // Linear interpolation between points
            let ratio = (minutes_held - time1) / (time2 - time1);
            return target1 + (target2 - target1) * ratio;
        }
    }
    
    // Beyond the curve - use declining targets
    let last_target = CURVE_TARGETS.last().unwrap().1;
    let decline_rate = 0.98; // 2% decline per minute after curve end
    let minutes_beyond = minutes_held - CURVE_TARGETS.last().unwrap().0;
    last_target * decline_rate.powf(minutes_beyond)
}

/// Calculate momentum-based multiplier
fn calculate_momentum_multiplier(token: &Token, current_profit: f64) -> f64 {
    if let Some(price_changes) = &token.price_change {
        let m1_change = price_changes.m1.unwrap_or(0.0);
        let m5_change = price_changes.m5.unwrap_or(0.0);
        
        // Strong momentum indicators
        if m5_change > 10.0 && m1_change > 5.0 {
            return MOMENTUM_STRONG_MULTIPLIER;
        }
        
        // Fading momentum indicators
        if current_profit > 20.0 && m1_change < 1.0 {
            return MOMENTUM_FADING_MULTIPLIER;
        }
        
        // Critical decline indicators
        if m1_change < -3.0 && current_profit > 10.0 {
            return CRITICAL_DIP_MULTIPLIER;
        }
    }
    
    1.0 // No adjustment
}

/// Calculate volatility-based multiplier
fn calculate_volatility_multiplier(token: &Token) -> f64 {
    if let Some(price_changes) = &token.price_change {
        let h1_change = price_changes.h1.unwrap_or(0.0).abs();
        
        match h1_change {
            v if v > 50.0 => HIGH_VOLATILITY_MULTIPLIER,  // High volatility
            v if v < 5.0 => LOW_VOLATILITY_MULTIPLIER,    // Low volatility  
            _ => 1.0,                                      // Normal volatility
        }
    } else {
        1.0
    }
}

/// Calculate position size multiplier
fn calculate_position_size_multiplier(position: &Position) -> f64 {
    match position.entry_size_sol {
        size if size >= 1.0 => LARGE_POSITION_MULTIPLIER,
        size if size >= 0.5 => MEDIUM_POSITION_MULTIPLIER,
        _ => 1.0,
    }
}

/// Get curve position factor (closer to peak = higher factor)
fn get_curve_position_factor(minutes_held: f64) -> f64 {
    let peak_time = 22.0; // Peak at 22 minutes based on curve
    let distance_from_peak = (minutes_held - peak_time).abs();
    let max_distance = 30.0; // Normalize to 30 minute range
    
    1.0 - (distance_from_peak / max_distance).min(1.0)
}

/// Calculate base urgency score
fn calculate_base_urgency(current_profit: f64, target: f64, curve_position: f64) -> f64 {
    let base = 0.6; // Base urgency when target is hit
    let profit_bonus = (current_profit / target - 1.0).min(1.0) * 0.3; // Bonus for exceeding target
    let curve_bonus = curve_position * 0.1; // Bonus for being near peak
    
    (base + profit_bonus + curve_bonus).min(0.9)
}

/// Apply momentum urgency boost
fn apply_momentum_urgency_boost(base_urgency: f64, token: &Token, current_profit: f64) -> f64 {
    let mut urgency = base_urgency;
    
    if let Some(price_changes) = &token.price_change {
        let m1_change = price_changes.m1.unwrap_or(0.0);
        
        // Boost urgency for fading momentum
        if current_profit > 20.0 && m1_change < 0.0 {
            urgency += 0.2;
        }
        
        // Boost urgency for critical decline
        if m1_change < -5.0 {
            urgency += 0.3;
        }
    }
    
    urgency.min(0.99)
}

/// Calculate curve-based time pressure
fn calculate_curve_based_time_pressure(minutes_held: f64, current_profit: f64) -> f64 {
    // Time pressure starts building after 45 minutes
    if minutes_held < 45.0 {
        return 0.0;
    }
    
    // Calculate pressure based on time beyond optimal range
    let excess_time = minutes_held - 45.0;
    let base_pressure = (excess_time / 30.0).min(0.8); // Build over 30 minutes
    
    // Scale by current profit (more pressure with more profit)
    let profit_scaling = (current_profit / 50.0).min(1.0);
    
    base_pressure * profit_scaling
}

// ================================================================================================
// 🏷️ SPEED CATEGORY SYSTEM (ENHANCED)
// ================================================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpeedCategoryEnhanced {
    Lightning,    // < 0.5 minutes
    UltraFast,    // 0.5-1 minutes  
    VeryFast,     // 1-5 minutes
    Fast,         // 5-12 minutes
    Peak,         // 12-25 minutes (peak zone)
    PostPeak,     // 25-35 minutes
    Slow,         // 35-60 minutes
    TooSlow,      // > 60 minutes
}

fn get_speed_category(minutes_held: f64) -> SpeedCategoryEnhanced {
    match minutes_held {
        x if x < 0.5 => SpeedCategoryEnhanced::Lightning,
        x if x <= 1.0 => SpeedCategoryEnhanced::UltraFast,
        x if x <= 5.0 => SpeedCategoryEnhanced::VeryFast,
        x if x <= 12.0 => SpeedCategoryEnhanced::Fast,
        x if x <= 25.0 => SpeedCategoryEnhanced::Peak,
        x if x <= 35.0 => SpeedCategoryEnhanced::PostPeak,
        x if x <= 60.0 => SpeedCategoryEnhanced::Slow,
        _ => SpeedCategoryEnhanced::TooSlow,
    }
}

fn get_speed_category_emoji(category: &SpeedCategoryEnhanced) -> &'static str {
    match category {
        SpeedCategoryEnhanced::Lightning => "⚡",
        SpeedCategoryEnhanced::UltraFast => "🚀",
        SpeedCategoryEnhanced::VeryFast => "🔥",
        SpeedCategoryEnhanced::Fast => "📈",
        SpeedCategoryEnhanced::Peak => "💎",
        SpeedCategoryEnhanced::PostPeak => "📉",
        SpeedCategoryEnhanced::Slow => "🐌",
        SpeedCategoryEnhanced::TooSlow => "⏰",
    }
}

// ================================================================================================
// 🔄 BACKWARD COMPATIBILITY
// ================================================================================================

/// Wrapper function to maintain compatibility with existing code
/// Falls back to original should_sell if token data is not available
pub fn should_sell_with_fallback(position: &Position, current_price: f64, token: Option<&Token>) -> (f64, String) {
    if token.is_some() {
        should_sell_enhanced(position, current_price, token)
    } else {
        // Fall back to original function from profit.rs
        crate::profit::should_sell(position, current_price)
    }
}
