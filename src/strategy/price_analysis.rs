use crate::prelude::*;
use crate::price_validation::{
    get_trading_price,
    get_realtime_price_change,
    has_sufficient_price_history,
    update_price_history,
};
use super::config::*;

/// Calculate dynamic SOL amount based on liquidity, market cap, and safety thresholds
pub fn calculate_trade_size_sol(liquidity_sol: f64) -> f64 {
    // Calculate base size using liquidity scaling
    let base_size = if liquidity_sol <= MIN_LIQUIDITY_FOR_MIN_SIZE {
        MIN_TRADE_SIZE_SOL
    } else if liquidity_sol >= MAX_LIQUIDITY_FOR_MAX_SIZE {
        MAX_TRADE_SIZE_SOL
    } else {
        // Linear interpolation between min and max
        let liquidity_ratio =
            (liquidity_sol - MIN_LIQUIDITY_FOR_MIN_SIZE) /
            (MAX_LIQUIDITY_FOR_MAX_SIZE - MIN_LIQUIDITY_FOR_MIN_SIZE);
        let size_range = MAX_TRADE_SIZE_SOL - MIN_TRADE_SIZE_SOL;
        MIN_TRADE_SIZE_SOL + liquidity_ratio * size_range
    };

    // Apply safety constraints to prevent whale anger
    let max_safe_size = liquidity_sol * (MAX_TRADE_PCT_OF_LIQUIDITY / 100.0);
    let final_size = base_size.min(max_safe_size);

    // Ensure we don't go below minimum
    final_size.max(MIN_TRADE_SIZE_SOL)
}

/// Enhanced trade size calculation with market cap consideration
pub fn calculate_trade_size_with_market_cap(liquidity_sol: f64, market_cap_usd: f64) -> f64 {
    // Get base size from liquidity
    let liquidity_based_size = calculate_trade_size_sol(liquidity_sol);

    // Calculate market cap adjustment factor
    let market_cap_ratio = if market_cap_usd <= MIN_MARKET_CAP_USD {
        0.0
    } else if market_cap_usd >= MAX_MARKET_CAP_USD {
        1.0
    } else {
        (market_cap_usd - MIN_MARKET_CAP_USD) / (MAX_MARKET_CAP_USD - MIN_MARKET_CAP_USD)
    };

    // Apply market cap scaling (weighted)
    let market_cap_adjustment = market_cap_ratio * MARKET_CAP_SCALING_FACTOR;
    let size_range = MAX_TRADE_SIZE_SOL - MIN_TRADE_SIZE_SOL;
    let market_cap_bonus = size_range * market_cap_adjustment;

    // Combine liquidity and market cap factors
    let combined_size = liquidity_based_size + market_cap_bonus;

    // Apply safety constraints
    let max_safe_size = liquidity_sol * (MAX_TRADE_PCT_OF_LIQUIDITY / 100.0);
    let final_size = combined_size.min(max_safe_size);

    // Ensure bounds
    final_size.max(MIN_TRADE_SIZE_SOL).min(MAX_TRADE_SIZE_SOL)
}

// ═══════════════════════════════════════════════════════════════════════════════
// REAL-TIME PRICE CHANGE ANALYSIS SYSTEM
// ═══════════════════════════════════════════════════════════════════════════════

/// Get real-time price change with fallback to dexscreener data
/// Returns (price_change_pct, is_realtime)
pub fn get_price_change_with_fallback(token: &Token, minutes: u64) -> (f64, bool) {
    // First try to get real-time price change from pool prices
    if has_sufficient_price_history(&token.mint, minutes) {
        if let Some(realtime_change) = get_realtime_price_change(&token.mint, minutes) {
            return (realtime_change, true);
        }
    }

    // Fallback to dexscreener data based on requested timeframe
    let fallback_change = match minutes {
        5 => token.price_change.m5,
        15 => token.price_change.m5, // fallback to 5m since m15 doesn't exist
        60 => token.price_change.h1,
        _ => token.price_change.m5, // default to 5m for other timeframes
    };

    (fallback_change, false)
}

/// Get comprehensive real-time price analysis
pub fn get_realtime_price_analysis(token: &Token) -> PriceAnalysis {
    let (change_5m, is_5m_realtime) = get_price_change_with_fallback(token, 5);
    let (change_15m, is_15m_realtime) = get_price_change_with_fallback(token, 15);
    let (change_1h, is_1h_realtime) = get_price_change_with_fallback(token, 60);

    // Update price history if we have current price
    if let Some(current_price) = get_trading_price(&token.mint) {
        update_price_history(&token.mint, current_price);
    }

    PriceAnalysis {
        change_5m,
        change_15m,
        change_1h,
        is_5m_realtime,
        is_15m_realtime,
        is_1h_realtime,
        has_sufficient_history: has_sufficient_price_history(&token.mint, 5),
    }
}

#[derive(Debug, Clone)]
pub struct PriceAnalysis {
    pub change_5m: f64,
    pub change_15m: f64,
    pub change_1h: f64,
    pub is_5m_realtime: bool,
    pub is_15m_realtime: bool,
    pub is_1h_realtime: bool,
    pub has_sufficient_history: bool,
}

impl PriceAnalysis {
    /// Check if price changes show excessive upward momentum
    pub fn has_excessive_momentum(&self) -> (bool, String) {
        if self.change_5m > 2.0 {
            // MAX_UPWARD_MOMENTUM_5M value
            return (
                true,
                format!("high_5m_momentum({:.1}%{})", self.change_5m, if self.is_5m_realtime {
                    "_RT"
                } else {
                    "_DX"
                }),
            );
        }

        if self.change_1h > 8.0 {
            // MAX_UPWARD_MOMENTUM_1H value
            return (
                true,
                format!("high_1h_momentum({:.1}%{})", self.change_1h, if self.is_1h_realtime {
                    "_RT"
                } else {
                    "_DX"
                }),
            );
        }

        (false, "momentum_acceptable".to_string())
    }

    /// Check if token is in a major dump
    pub fn is_major_dump(&self) -> bool {
        self.change_5m <= -25.0 // BIG_DUMP_THRESHOLD value
    }

    /// Check if price is in accumulation range (controlled movement)
    pub fn is_accumulation_range(&self) -> bool {
        self.change_5m >= -10.0 && self.change_5m <= 3.0 // ACCUMULATION_PATIENCE_THRESHOLD value
    }

    /// Get a display string showing data sources
    pub fn get_data_source_info(&self) -> String {
        format!(
            "5m:{} 15m:{} 1h:{}",
            if self.is_5m_realtime {
                "RT"
            } else {
                "DX"
            },
            if self.is_15m_realtime {
                "RT"
            } else {
                "DX"
            },
            if self.is_1h_realtime {
                "RT"
            } else {
                "DX"
            }
        )
    }
}

/// Enhanced trend detection with uptrend/downtrend/consolidation classification
pub fn classify_market_trend(price_analysis: &PriceAnalysis) -> (String, f64, bool) {
    let change_5m = price_analysis.change_5m;
    let change_1h = price_analysis.change_1h;

    // Determine trend strength and direction
    let trend_strength = (change_5m.abs() + change_1h.abs()) / 2.0;

    if change_5m > UPTREND_MOMENTUM_THRESHOLD && change_1h > 0.0 {
        // Strong uptrend - good for momentum entries
        ("strong_uptrend".to_string(), trend_strength, true)
    } else if change_5m > 0.0 && change_1h > UPTREND_MOMENTUM_THRESHOLD {
        // Building uptrend - early entry opportunity
        ("building_uptrend".to_string(), trend_strength, true)
    } else if change_5m < DOWNTREND_DIP_OPPORTUNITY && change_1h < -2.0 {
        // Strong downtrend - dip buying opportunity
        ("strong_downtrend_dip".to_string(), trend_strength, true)
    } else if change_5m.abs() <= CONSOLIDATION_RANGE && change_1h.abs() <= CONSOLIDATION_RANGE {
        // Consolidation - safe entry zone
        ("consolidation".to_string(), trend_strength, true)
    } else if change_5m < -2.0 && change_1h > 2.0 {
        // Potential reversal - be cautious
        ("potential_reversal".to_string(), trend_strength, false)
    } else {
        // Mixed signals - neutral
        ("mixed_signals".to_string(), trend_strength, false)
    }
}

/// Calculate market condition bonus for entry scoring
pub fn get_market_condition_bonus(price_analysis: &PriceAnalysis, volume_ratio: f64) -> f64 {
    let (trend_type, trend_strength, is_favorable) = classify_market_trend(price_analysis);

    let mut bonus = 0.0;

    if is_favorable {
        match trend_type.as_str() {
            "strong_uptrend" => {
                // Strong uptrend with volume confirmation
                if volume_ratio > UPTREND_VOLUME_CONFIRMATION {
                    bonus += 0.3; // Strong bonus for confirmed uptrend
                } else {
                    bonus += 0.15; // Moderate bonus without volume
                }
            }
            "building_uptrend" => {
                bonus += 0.2; // Good early entry bonus
            }
            "strong_downtrend_dip" => {
                bonus += 0.25; // Excellent dip buying opportunity
            }
            "consolidation" => {
                bonus += 0.1; // Safe entry zone
            }
            _ => {}
        }
    }

    // Additional bonus for real-time data
    if price_analysis.is_5m_realtime {
        bonus += REAL_TIME_PRICE_BONUS;
    }

    // Volume surge bonus
    if volume_ratio > 2.0 {
        bonus += HIGH_VOLUME_BONUS;
    }

    bonus
}

/// Prioritize real-time pool prices over API data
pub fn get_most_reliable_price(token: &Token) -> (f64, bool, String) {
    // First priority: Real-time pool price
    if let Some(pool_price) = get_trading_price(&token.mint) {
        return (pool_price, true, "real_time_pool".to_string());
    }

    // Fallback to dexscreener price
    let price_usd = token.price_usd.parse::<f64>().unwrap_or(0.0);
    (price_usd, false, "dexscreener_api".to_string())
}
