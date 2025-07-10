use crate::prelude::*;
use crate::price_validation::{
    get_trading_price,
    get_realtime_price_change,
    has_sufficient_price_history,
    update_price_history,
};
use super::config::*;

/// Calculate dynamic SOL amount based on liquidity
pub fn calculate_trade_size_sol(liquidity_sol: f64) -> f64 {
    if liquidity_sol <= MIN_LIQUIDITY_FOR_MIN_SIZE {
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
    }
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
