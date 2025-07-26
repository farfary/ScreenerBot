use crate::global::*;
use crate::tokens::Token;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };

// ================================================================================================
// 🎯 NEXT-GENERATION PROFIT SYSTEM - COMPLETE REWRITE
// ================================================================================================
// Duration-based profit scaling: 1min to 1h trades
// Tracks highest/lowest prices after entry
// Zero-loss protection with -99% emergency exit
// Profits scale from 0% to 1000% based on speed and duration
// ================================================================================================

// 📊 DURATION-BASED PROFIT TARGETS (MINUTES)
const ULTRA_FAST_MINUTES: f64 = 1.0; // 1 minute - lightning fast
const VERY_FAST_MINUTES: f64 = 5.0; // 5 minutes - very fast
const FAST_MINUTES: f64 = 10.0; // 15 minutes - fast
const MEDIUM_MINUTES: f64 = 20.0; // 30 minutes - medium
const SLOW_MINUTES: f64 = 30.0; // 60 minutes - 1 hour max

// 🚀 PROFIT TARGETS BY SPEED (PERCENTAGE)
const ULTRA_FAST_PROFIT: f64 = 20.0; // 20% in 1 minute = ultra fast sell
const VERY_FAST_PROFIT: f64 = 50.0; // 50% in 5 minutes = very fast sell
const FAST_PROFIT: f64 = 100.0; // 100% in 15 minutes = fast sell
const MEDIUM_PROFIT: f64 = 200.0; // 200% in 30 minutes = medium sell
const SLOW_PROFIT: f64 = 500.0; // 500% in 60 minutes = slow sell
const EXTREME_PROFIT: f64 = 1000.0; // 1000% = instant sell regardless of time

// ⚡ SPEED BONUSES (MULTIPLIERS)
const SPEED_BONUS_ULTRA: f64 = 2.0; // 2x urgency for ultra fast profits
const SPEED_BONUS_VERY: f64 = 1.8; // 1.8x urgency for very fast profits
const SPEED_BONUS_FAST: f64 = 1.5; // 1.5x urgency for fast profits
const SPEED_BONUS_MEDIUM: f64 = 1.2; // 1.2x urgency for medium profits

// 🔒 ZERO-LOSS PROTECTION
const EMERGENCY_EXIT_THRESHOLD: f64 = -99.0; // Only sell at -99% for emergency
const BREAKEVEN_THRESHOLD: f64 = 0.0; // Never sell below breakeven
const MINIMUM_PROFIT_TO_CONSIDER: f64 = 0.1; // 0.1% minimum to consider selling

// 📈 PRICE TRACKING THRESHOLDS
const SIGNIFICANT_DIP_PERCENT: f64 = 3.0; // 10% dip from peak = warning
const MAJOR_DIP_PERCENT: f64 = 6.0; // 20% dip from peak = concern
const CRITICAL_DIP_PERCENT: f64 = 9.0; // 30% dip from peak = urgent

// 🕐 TIME PRESSURE SCALING
const TIME_PRESSURE_START: f64 = 45.0; // Start time pressure at 45 minutes
const MAX_TIME_PRESSURE: f64 = 0.6; // Maximum urgency from time alone

// ⏰ MANDATORY FORCE SELL RULES
const FORCE_SELL_TIME_MINUTES: f64 = 60.0; // Force sell after 1 hour (60 minutes)
const FORCE_SELL_MIN_PROFIT: f64 = 5.0; // Minimum 5% profit required for force sell

// ================================================================================================
// 📊 PRICE TRACKING SYSTEM
// ================================================================================================

/// Real-time price tracking for position analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTracker {
    pub entry_price: f64,
    pub current_price: f64,
    pub highest_price: f64, // Highest price since entry
    pub lowest_price: f64, // Lowest price since entry
    pub last_update: DateTime<Utc>,
    pub peak_reached_at: Option<DateTime<Utc>>, // When we hit our peak
    pub dip_from_peak_percent: f64, // Current dip from peak
}

impl PriceTracker {
    pub fn new(entry_price: f64) -> Self {
        Self {
            entry_price,
            current_price: entry_price,
            highest_price: entry_price,
            lowest_price: entry_price,
            last_update: Utc::now(),
            peak_reached_at: None,
            dip_from_peak_percent: 0.0,
        }
    }

    pub fn update(&mut self, new_price: f64) {
        self.current_price = new_price;
        self.last_update = Utc::now();

        // Track new highs
        if new_price > self.highest_price {
            self.highest_price = new_price;
            self.peak_reached_at = Some(Utc::now());
            self.dip_from_peak_percent = 0.0;
        } else {
            // Calculate dip from peak
            self.dip_from_peak_percent =
                ((self.highest_price - new_price) / self.highest_price) * 100.0;
        }

        // Track new lows
        if new_price < self.lowest_price {
            self.lowest_price = new_price;
        }
    }

    pub fn get_profit_percent(&self) -> f64 {
        ((self.current_price - self.entry_price) / self.entry_price) * 100.0
    }

    pub fn get_peak_profit_percent(&self) -> f64 {
        ((self.highest_price - self.entry_price) / self.entry_price) * 100.0
    }

    pub fn get_lowest_percent(&self) -> f64 {
        ((self.lowest_price - self.entry_price) / self.entry_price) * 100.0
    }
}

// ================================================================================================
// 🧠 INTELLIGENT MOMENTUM ANALYZER
// ================================================================================================

/// Advanced momentum analysis for smart decision making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentumAnalysis {
    pub profit_percent: f64,
    pub peak_profit_percent: f64,
    pub dip_from_peak_percent: f64,
    pub minutes_held: f64,
    pub is_momentum_strong: bool,
    pub is_momentum_fading: bool,
    pub is_critical_dip: bool,
    pub speed_category: SpeedCategory,
    pub urgency_modifier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpeedCategory {
    UltraFast, // < 1 minute
    VeryFast, // 1-5 minutes
    Fast, // 5-15 minutes
    Medium, // 15-30 minutes
    Slow, // 30-60 minutes
    TooSlow, // > 60 minutes
}

/// Analyze token momentum and price action
pub fn analyze_momentum(
    tracker: &PriceTracker,
    token: &Token,
    minutes_held: f64
) -> MomentumAnalysis {
    let profit_percent = tracker.get_profit_percent();
    let peak_profit_percent = tracker.get_peak_profit_percent();
    let dip_from_peak_percent = tracker.dip_from_peak_percent;

    // Determine speed category
    let speed_category = match minutes_held {
        x if x <= ULTRA_FAST_MINUTES => SpeedCategory::UltraFast,
        x if x <= VERY_FAST_MINUTES => SpeedCategory::VeryFast,
        x if x <= FAST_MINUTES => SpeedCategory::Fast,
        x if x <= MEDIUM_MINUTES => SpeedCategory::Medium,
        x if x <= SLOW_MINUTES => SpeedCategory::Slow,
        _ => SpeedCategory::TooSlow,
    };

    // Analyze momentum from token data
    let mut is_momentum_strong = false;
    let mut is_momentum_fading = false;

    if let Some(price_changes) = &token.price_change {
        let m5_change = price_changes.m5.unwrap_or(0.0);
        let h1_change = price_changes.h1.unwrap_or(0.0);

        is_momentum_strong = m5_change > 5.0 && h1_change > 10.0;
        is_momentum_fading = m5_change < 1.0 && profit_percent > 10.0;

        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "Momentum Data: 5m: {:.1}% | 1h: {:.1}% | Strong: {} (5m>5% && 1h>10%) | Fading: {} (5m<1% && profit>10%)",
                    m5_change,
                    h1_change,
                    is_momentum_strong,
                    is_momentum_fading
                )
            );
        }
    } else if is_debug_profit_enabled() {
        log(
            LogTag::Trader,
            "🔍 PROFIT-DEBUG",
            "No price change data available for momentum analysis"
        );
    }

    // Check for critical dip
    let is_critical_dip = dip_from_peak_percent > CRITICAL_DIP_PERCENT;

    // Calculate urgency modifier based on speed
    let urgency_modifier = match speed_category {
        SpeedCategory::UltraFast => SPEED_BONUS_ULTRA,
        SpeedCategory::VeryFast => SPEED_BONUS_VERY,
        SpeedCategory::Fast => SPEED_BONUS_FAST,
        SpeedCategory::Medium => SPEED_BONUS_MEDIUM,
        _ => 1.0,
    };

    if is_debug_profit_enabled() {
        log(
            LogTag::Trader,
            "🔍 PROFIT-DEBUG",
            &format!(
                "Speed Category: {:?} | Modifier: {:.1}x | Critical Dip: {} (>{:.0}%)",
                speed_category,
                urgency_modifier,
                is_critical_dip,
                CRITICAL_DIP_PERCENT
            )
        );
    }

    MomentumAnalysis {
        profit_percent,
        peak_profit_percent,
        dip_from_peak_percent,
        minutes_held,
        is_momentum_strong,
        is_momentum_fading,
        is_critical_dip,
        speed_category,
        urgency_modifier,
    }
}

// ================================================================================================
// 🎯 MAIN PROFIT DECISION ENGINE
// ================================================================================================

/// NEW GENERATION PROFIT SYSTEM
/// Zero-loss protection with emergency exit only at -99%
/// Speed-based profit targets: faster = sell sooner
/// Duration scaling: 1min to 1h optimal trade window
/// Profit scaling: 0% to 1000% based on speed achieved
pub fn should_sell_next_gen(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64,
    price_tracker: &PriceTracker
) -> (f64, String) {
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let minutes_held = time_held_seconds / 60.0;

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🔍 DEBUG PROFIT LOGGING
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let current_profit_percent = ((current_price - entry_price) / entry_price) * 100.0;

    if is_debug_profit_enabled() {
        log(
            LogTag::Trader,
            "🔍 PROFIT-DEBUG",
            &format!(
                "Analyzing {} | Price: {:.8} → {:.8} | Profit: {:.2}% | Time: {:.1}m | Peak: {:.2}% | Dip: {:.1}%",
                position.symbol,
                entry_price,
                current_price,
                current_profit_percent,
                minutes_held,
                price_tracker.get_peak_profit_percent(),
                price_tracker.dip_from_peak_percent
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🛡️ ZERO-LOSS PROTECTION SYSTEM
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let current_profit_percent = ((current_price - entry_price) / entry_price) * 100.0;

    // ABSOLUTE RULE: NEVER SELL AT LOSS (except emergency -99%)
    if current_profit_percent <= BREAKEVEN_THRESHOLD {
        if current_profit_percent <= EMERGENCY_EXIT_THRESHOLD {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Trader,
                    "� PROFIT-DEBUG",
                    &format!(
                        "�🚨 EMERGENCY EXIT TRIGGERED: {} at {:.2}% loss (threshold: {:.0}%)",
                        position.symbol,
                        current_profit_percent,
                        EMERGENCY_EXIT_THRESHOLD
                    )
                );
            }
            log(
                LogTag::Trader,
                "🚨 EMERGENCY",
                &format!(
                    "EMERGENCY EXIT: {} at {:.2}% - EXTREME LOSS",
                    position.symbol,
                    current_profit_percent
                )
            );
            return (1.0, format!("🚨 EMERGENCY EXIT: {:.1}% loss", current_profit_percent));
        }

        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "🔒 ZERO-LOSS PROTECTION ACTIVE: {} at {:.2}% (breakeven: {:.0}%) - HOLDING",
                    position.symbol,
                    current_profit_percent,
                    BREAKEVEN_THRESHOLD
                )
            );
        }
        log(
            LogTag::Trader,
            "🔒 HOLD",
            &format!(
                "ZERO-LOSS PROTECTION: {} at {:.2}% - NEVER SELL AT LOSS",
                position.symbol,
                current_profit_percent
            )
        );
        return (0.0, format!("🔒 HOLD: {:.2}% - NO LOSS SALES", current_profit_percent));
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🚀 EXTREME PROFIT PROTECTION (INSTANT SELLS)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if current_profit_percent >= EXTREME_PROFIT {
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "💎 EXTREME PROFIT REACHED: {} at {:.1}% (threshold: {:.0}%) in {:.1}m - INSTANT SELL",
                    position.symbol,
                    current_profit_percent,
                    EXTREME_PROFIT,
                    minutes_held
                )
            );
        }
        log(
            LogTag::Trader,
            "💎 EXTREME",
            &format!(
                "EXTREME PROFIT: {} at {:.1}% in {:.1}m - INSTANT SELL",
                position.symbol,
                current_profit_percent,
                minutes_held
            )
        );
        return (
            0.99,
            format!(
                "💎 EXTREME: {:.0}% in {:.1}m - INSTANT SELL",
                current_profit_percent,
                minutes_held
            ),
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ FORCE SELL AFTER 1 HOUR (60+ MINUTES) WITH 5%+ PROFIT
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if minutes_held >= FORCE_SELL_TIME_MINUTES && current_profit_percent >= FORCE_SELL_MIN_PROFIT {
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "⏰ FORCE SELL TRIGGERED: {} held {:.0}m (limit: {:.0}m) with {:.1}% profit (min: {:.0}%)",
                    position.symbol,
                    minutes_held,
                    FORCE_SELL_TIME_MINUTES,
                    current_profit_percent,
                    FORCE_SELL_MIN_PROFIT
                )
            );
        }
        log(
            LogTag::Trader,
            "⏰ FORCE",
            &format!(
                "FORCE SELL: {} at {:.1}% after {:.0}m - MANDATORY EXIT",
                position.symbol,
                current_profit_percent,
                minutes_held
            )
        );
        return (
            0.95,
            format!(
                "⏰ FORCE SELL: {:.1}% after {:.0}m - NO MORE WAITING",
                current_profit_percent,
                minutes_held
            ),
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⚡ SPEED-BASED PROFIT TARGETS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    let momentum = analyze_momentum(price_tracker, token, minutes_held);
    let mut base_urgency = 0.0;
    let mut reason = String::new();

    if is_debug_profit_enabled() {
        log(
            LogTag::Trader,
            "🔍 PROFIT-DEBUG",
            &format!(
                "Speed Analysis: {} | {:.1}m | Ultra<{:.0}m ({:.0}%) | VFast<{:.0}m ({:.0}%) | Fast<{:.0}m ({:.0}%) | Med<{:.0}m ({:.0}%) | Slow<{:.0}m ({:.0}%)",
                position.symbol,
                minutes_held,
                ULTRA_FAST_MINUTES,
                ULTRA_FAST_PROFIT,
                VERY_FAST_MINUTES,
                VERY_FAST_PROFIT,
                FAST_MINUTES,
                FAST_PROFIT,
                MEDIUM_MINUTES,
                MEDIUM_PROFIT,
                SLOW_MINUTES,
                SLOW_PROFIT
            )
        );
    }

    // Ultra-fast profits (< 1 minute)
    if minutes_held <= ULTRA_FAST_MINUTES && current_profit_percent >= ULTRA_FAST_PROFIT {
        base_urgency = 0.9;
        reason = format!("⚡ ULTRA-FAST: {:.0}% in {:.1}m", current_profit_percent, minutes_held);
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "⚡ ULTRA-FAST TARGET HIT: {} urgency={:.1}",
                    position.symbol,
                    base_urgency
                )
            );
        }
    } else if
        // Very fast profits (1-5 minutes)
        minutes_held <= VERY_FAST_MINUTES &&
        current_profit_percent >= VERY_FAST_PROFIT
    {
        base_urgency = 0.8;
        reason = format!("🚀 VERY-FAST: {:.0}% in {:.1}m", current_profit_percent, minutes_held);
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!("🚀 VERY-FAST TARGET HIT: {} urgency={:.1}", position.symbol, base_urgency)
            );
        }
    } else if
        // Fast profits (5-15 minutes)
        minutes_held <= FAST_MINUTES &&
        current_profit_percent >= FAST_PROFIT
    {
        base_urgency = 0.7;
        reason = format!("🔥 FAST: {:.0}% in {:.1}m", current_profit_percent, minutes_held);
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!("🔥 FAST TARGET HIT: {} urgency={:.1}", position.symbol, base_urgency)
            );
        }
    } else if
        // Medium profits (15-30 minutes)
        minutes_held <= MEDIUM_MINUTES &&
        current_profit_percent >= MEDIUM_PROFIT
    {
        base_urgency = 0.6;
        reason = format!("📈 MEDIUM: {:.0}% in {:.1}m", current_profit_percent, minutes_held);
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!("📈 MEDIUM TARGET HIT: {} urgency={:.1}", position.symbol, base_urgency)
            );
        }
    } else if
        // Slow profits (30-60 minutes)
        minutes_held <= SLOW_MINUTES &&
        current_profit_percent >= SLOW_PROFIT
    {
        base_urgency = 0.5;
        reason = format!("🐌 SLOW: {:.0}% in {:.1}m", current_profit_percent, minutes_held);
        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!("🐌 SLOW TARGET HIT: {} urgency={:.1}", position.symbol, base_urgency)
            );
        }
    } else if is_debug_profit_enabled() {
        // Debug why no speed target was hit
        let target_profit = get_target_profit_for_duration(minutes_held);
        log(
            LogTag::Trader,
            "🔍 PROFIT-DEBUG",
            &format!(
                "❌ NO SPEED TARGET: {} has {:.1}% profit but needs {:.0}% for {:.1}m duration",
                position.symbol,
                current_profit_percent,
                target_profit,
                minutes_held
            )
        );
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 📊 MOMENTUM & DIP ANALYSIS
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if base_urgency > 0.0 {
        let mut final_urgency = base_urgency;

        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "Momentum Analysis: {} | Strong: {} | Fading: {} | Critical Dip: {} | Modifier: {:.1}x",
                    position.symbol,
                    momentum.is_momentum_strong,
                    momentum.is_momentum_fading,
                    momentum.is_critical_dip,
                    momentum.urgency_modifier
                )
            );
        }

        // Apply speed bonus
        final_urgency *= momentum.urgency_modifier;

        // Critical dip from peak = higher urgency
        if momentum.is_critical_dip {
            final_urgency += 0.2;
            reason = format!("{} + CRITICAL DIP {:.1}%", reason, momentum.dip_from_peak_percent);
            if is_debug_profit_enabled() {
                log(
                    LogTag::Trader,
                    "🔍 PROFIT-DEBUG",
                    &format!(
                        "🔴 CRITICAL DIP: {} +0.2 urgency (dip: {:.1}%)",
                        position.symbol,
                        momentum.dip_from_peak_percent
                    )
                );
            }
        } else if
            // Major dip = medium urgency boost
            momentum.dip_from_peak_percent > MAJOR_DIP_PERCENT
        {
            final_urgency += 0.15;
            reason = format!("{} + MAJOR DIP {:.1}%", reason, momentum.dip_from_peak_percent);
            if is_debug_profit_enabled() {
                log(
                    LogTag::Trader,
                    "🔍 PROFIT-DEBUG",
                    &format!(
                        "🟠 MAJOR DIP: {} +0.15 urgency (dip: {:.1}%)",
                        position.symbol,
                        momentum.dip_from_peak_percent
                    )
                );
            }
        } else if
            // Significant dip = small urgency boost
            momentum.dip_from_peak_percent > SIGNIFICANT_DIP_PERCENT
        {
            final_urgency += 0.1;
            reason = format!("{} + DIP {:.1}%", reason, momentum.dip_from_peak_percent);
            if is_debug_profit_enabled() {
                log(
                    LogTag::Trader,
                    "🔍 PROFIT-DEBUG",
                    &format!(
                        "🟡 SIGNIFICANT DIP: {} +0.1 urgency (dip: {:.1}%)",
                        position.symbol,
                        momentum.dip_from_peak_percent
                    )
                );
            }
        }

        // Fading momentum = urgency boost
        if momentum.is_momentum_fading {
            final_urgency += 0.15;
            reason = format!("{} + FADING", reason);
            if is_debug_profit_enabled() {
                log(
                    LogTag::Trader,
                    "🔍 PROFIT-DEBUG",
                    &format!("📉 FADING MOMENTUM: {} +0.15 urgency", position.symbol)
                );
            }
        }

        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "✅ SELL DECISION: {} | Base: {:.2} → Final: {:.2} | Reason: {}",
                    position.symbol,
                    base_urgency,
                    final_urgency.min(0.99),
                    reason
                )
            );
        }

        return (final_urgency.min(0.99), reason);
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // ⏰ TIME PRESSURE SYSTEM (45+ minutes)
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if minutes_held >= TIME_PRESSURE_START && current_profit_percent > MINIMUM_PROFIT_TO_CONSIDER {
        let time_pressure =
            ((minutes_held - TIME_PRESSURE_START) / (SLOW_MINUTES - TIME_PRESSURE_START)) *
            MAX_TIME_PRESSURE;
        let pressure_urgency = time_pressure.min(MAX_TIME_PRESSURE);

        // Add profit scaling to time pressure
        let profit_scaling = (current_profit_percent / 100.0).min(1.0) * 0.3;
        let final_urgency = (pressure_urgency + profit_scaling).min(0.8);

        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "⏰ TIME PRESSURE: {} | {:.1}m > {:.0}m | Pressure: {:.2} | Profit Scale: {:.2} | Final: {:.2}",
                    position.symbol,
                    minutes_held,
                    TIME_PRESSURE_START,
                    pressure_urgency,
                    profit_scaling,
                    final_urgency
                )
            );
        }

        if final_urgency > 0.3 {
            if is_debug_profit_enabled() {
                log(
                    LogTag::Trader,
                    "🔍 PROFIT-DEBUG",
                    &format!(
                        "⏰ TIME PRESSURE SELL: {} urgency {:.2}",
                        position.symbol,
                        final_urgency
                    )
                );
            }
            return (
                final_urgency,
                format!(
                    "⏰ TIME PRESSURE: {:.1}% profit in {:.1}m",
                    current_profit_percent,
                    minutes_held
                ),
            );
        } else if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "⏰ TIME PRESSURE TOO LOW: {} urgency {:.2} < 0.3",
                    position.symbol,
                    final_urgency
                )
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════════════════════════
    // 🔄 DEFAULT HOLDING PATTERN
    // ═══════════════════════════════════════════════════════════════════════════════════════

    if current_profit_percent > MINIMUM_PROFIT_TO_CONSIDER {
        // Calculate target profit for current duration
        let target_profit = get_target_profit_for_duration(minutes_held);

        if is_debug_profit_enabled() {
            log(
                LogTag::Trader,
                "🔍 PROFIT-DEBUG",
                &format!(
                    "📊 HOLDING: {} | Current: {:.2}% | Target: {:.0}% | Time: {:.1}m | Minimum: {:.1}%",
                    position.symbol,
                    current_profit_percent,
                    target_profit,
                    minutes_held,
                    MINIMUM_PROFIT_TO_CONSIDER
                )
            );
        }

        return (
            0.1,
            format!(
                "📊 HOLD: {:.1}% profit (target: {:.0}%) in {:.1}m",
                current_profit_percent,
                target_profit,
                minutes_held
            ),
        );
    }

    if is_debug_profit_enabled() {
        log(
            LogTag::Trader,
            "🔍 PROFIT-DEBUG",
            &format!(
                "⏳ WAITING: {} | Profit: {:.2}% < minimum {:.1}% | Time: {:.1}m",
                position.symbol,
                current_profit_percent,
                MINIMUM_PROFIT_TO_CONSIDER,
                minutes_held
            )
        );
    }

    (0.0, format!("⏳ WAIT: {:.2}% in {:.1}m", current_profit_percent, minutes_held))
}

/// Calculate target profit based on time held
fn get_target_profit_for_duration(minutes_held: f64) -> f64 {
    match minutes_held {
        x if x <= ULTRA_FAST_MINUTES => ULTRA_FAST_PROFIT,
        x if x <= VERY_FAST_MINUTES => VERY_FAST_PROFIT,
        x if x <= FAST_MINUTES => FAST_PROFIT,
        x if x <= MEDIUM_MINUTES => MEDIUM_PROFIT,
        x if x <= SLOW_MINUTES => SLOW_PROFIT,
        _ => SLOW_PROFIT * 1.5, // Higher target for very slow trades
    }
}

// ================================================================================================
// 🔄 COMPATIBILITY LAYER
// ================================================================================================

/// Legacy compatibility function - wraps the new system
pub fn should_sell_dynamic(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    // Create basic price tracker for legacy compatibility
    let mut tracker = PriceTracker::new(
        position.effective_entry_price.unwrap_or(position.entry_price)
    );
    tracker.update(current_price);

    should_sell_next_gen(position, token, current_price, time_held_seconds, &tracker)
}

/// Legacy compatibility function
pub fn should_sell_smart_system(
    position: &Position,
    token: &Token,
    current_price: f64,
    time_held_seconds: f64
) -> (f64, String) {
    should_sell_dynamic(position, token, current_price, time_held_seconds)
}
