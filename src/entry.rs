/// Pool-based entry logic for ScreenerBot
///
/// This module provides pool price-based entry decisions with -10% drop detection.
/// Uses real-time blockchain pool data for trading decisions while API data is used only for validation.
/// Enhanced with 2-minute data age filtering and RL learning advisory (non-blocking).
/// OPTIMIZED FOR FAST TRADING: Sub-minute decisions with pool price priority.

use crate::tokens::Token;
use crate::tokens::pool::get_pool_service;
use crate::tokens::is_token_excluded_from_trading;
use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_entry_enabled };
use crate::rl_learning::{ get_trading_learner, collect_market_features };
use crate::tokens::cache::TokenDatabase;
use chrono::Utc;

// DEEP DROP ENTRY CONFIGURATION
const MAX_DATA_AGE_MINUTES: i64 = 2; // Reject any data older than 2 minutes
const DEEP_DROP_MIN_PERCENT: f64 = 10.0; // Minimum drop % for entry
const DEEP_DROP_MAX_PERCENT: f64 = 50.0; // Maximum drop % for entry  
const DEEP_DROP_TIME_WINDOW_SEC: i64 = 60; // Must happen in last 60 seconds
const TARGET_DROP_RATIO: f64 = 0.33; // Target 1/3 of recent high

/// Deep drop entry decision with volatility-based scaling
/// Returns true if token shows deep drop pattern for immediate entry
pub async fn should_buy(token: &Token) -> bool {
    // Check blacklist first
    if is_token_excluded_from_trading(&token.mint) {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "BLACKLIST_REJECT", &format!("❌ {} blacklisted", token.symbol));
        }
        return false;
    }

    let pool_service = get_pool_service();
    
    if !pool_service.check_token_availability(&token.mint).await {
        return false;
    }

    // Get current pool price with age validation
    let (current_pool_price, pool_data_age) = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => {
                    let data_age_minutes = (Utc::now() - pool_result.calculated_at).num_minutes();
                    
                    if data_age_minutes > MAX_DATA_AGE_MINUTES {
                        return false;
                    }
                    
                    (price, data_age_minutes)
                },
                _ => return false,
            }
        }
        None => return false,
    };

    // Get recent price history for deep drop analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;
    
    // CORE LOGIC: Deep drop detection
    let deep_drop_result = analyze_deep_drop_entry(
        current_pool_price,
        &price_history,
        pool_data_age
    ).await;

    if let Some((drop_percent, entry_reason)) = deep_drop_result {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "DEEP_DROP_ENTRY",
                &format!(
                    "🎯 {} DEEP ENTRY: -{:.1}% {} (price: {:.12} SOL)",
                    token.symbol, drop_percent, entry_reason, current_pool_price
                )
            );
        }
        return true;
    }

    false
}

/// Get profit target range based on pool liquidity
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    let pool_service = get_pool_service();
    
    let liquidity_usd = if let Some(pool_result) = pool_service.get_pool_price(&token.mint, None).await {
        pool_result.liquidity_usd
    } else {
        token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0)
    };

    // Base targets adjusted by liquidity only
    let (min_target, max_target) = match liquidity_usd {
        x if x >= 1_000_000.0 => (5.0, 15.0),
        x if x >= 100_000.0 => (10.0, 30.0), 
        x if x >= 10_000.0 => (20.0, 50.0),
        _ => (30.0, 100.0),
    };
    
    (min_target, max_target)
}

/// Get dynamic entry threshold (simplified)
pub fn get_entry_threshold(token: &Token) -> f64 {
    DEEP_DROP_MIN_PERCENT
}

/// Helper function to get rugcheck score for a token (simplified)
pub async fn get_rugcheck_score_for_token(mint: &str) -> Option<f64> {
    match TokenDatabase::new() {
        Ok(database) => {
            match database.get_rugcheck_data(mint) {
                Ok(Some(rugcheck_data)) => rugcheck_data.score.map(|s| s as f64),
                _ => None,
            }
        }
        Err(_) => None,
    }
}

/// Calculate price volatility from recent history
fn calculate_price_volatility(price_history: &[(chrono::DateTime<chrono::Utc>, f64)], current_price: f64) -> f64 {
    if price_history.len() < 2 {
        return 10.0; // Default volatility for new tokens
    }
    
    let mut prices: Vec<f64> = price_history.iter().map(|(_, price)| *price).collect();
    prices.push(current_price);
    
    let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let max_price = prices.iter().fold(0.0f64, |a, &b| a.max(b));
    
    if min_price > 0.0 && min_price.is_finite() && max_price.is_finite() {
        ((max_price - min_price) / min_price) * 100.0
    } else {
        10.0
    }
}

/// Deep drop analysis for volatility-based entry decisions
/// Returns Some((drop_percent, reason)) if deep drop detected, None otherwise
async fn analyze_deep_drop_entry(
    current_price: f64,
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    data_age_minutes: i64
) -> Option<(f64, String)> {
    use chrono::Utc;
    
    // Strategy 1: Immediate entry for ultra-fresh data
    if data_age_minutes == 0 && price_history.is_empty() {
        return Some((0.0, "ultra-fresh entry".to_string()));
    }
    
    // Need at least 2 data points for drop analysis
    if price_history.len() < 2 {
        return None;
    }
    
    // Get recent prices within time window
    let now = Utc::now();
    let recent_prices: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= DEEP_DROP_TIME_WINDOW_SEC)
        .cloned()
        .collect();
    
    if recent_prices.is_empty() {
        return None;
    }
    
    // Find recent high and calculate drop
    let recent_high = recent_prices.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
    
    if recent_high <= 0.0 || !recent_high.is_finite() {
        return None;
    }
    
    let drop_percent = ((recent_high - current_price) / recent_high) * 100.0;
    
    if !drop_percent.is_finite() || drop_percent < 0.0 {
        return None;
    }
    
    // Get base threshold without volatility adjustment
    let base_threshold = DEEP_DROP_MIN_PERCENT;
    
    // Strategy 2: Deep drop detection (main entry condition)
    if drop_percent >= base_threshold && drop_percent <= DEEP_DROP_MAX_PERCENT {
        let time_span = recent_prices.len();
        return Some((drop_percent, format!("deep drop in {}pts", time_span)));
    }
    
    // Strategy 3: Target 1/3 drop detection
    let target_drop = recent_high * TARGET_DROP_RATIO;
    let current_drop_absolute = recent_high - current_price;
    
    if current_drop_absolute >= target_drop {
        let drop_ratio = current_drop_absolute / recent_high;
        return Some((drop_percent, format!("1/3 target hit ({:.1}%)", drop_ratio * 100.0)));
    }
    
    // Strategy 4: Fast deep drop (higher threshold but faster timeframe)
    let ultra_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= 10) // Last 10 seconds
        .cloned()
        .collect();
    
    if ultra_recent.len() >= 2 {
        let ultra_high = ultra_recent.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
        
        if ultra_high > 0.0 && ultra_high.is_finite() {
            let ultra_drop = ((ultra_high - current_price) / ultra_high) * 100.0;
            
            if ultra_drop >= 15.0 && ultra_drop <= DEEP_DROP_MAX_PERCENT {
                return Some((ultra_drop, "fast deep drop".to_string()));
            }
        }
    }
    
    None
}

/// Enhanced entry decision with confidence scoring 
/// Returns (should_enter, confidence_score, reason)
pub async fn should_buy_with_confidence(token: &Token) -> (bool, f64, String) {
    // Check blacklist first
    if is_token_excluded_from_trading(&token.mint) {
        return (false, 0.0, "Token blacklisted or excluded".to_string());
    }

    // Use the main should_buy logic for entry decision
    let should_enter = should_buy(token).await;
    
    if should_enter {
        // Calculate confidence based on liquidity and basic factors
        let liquidity_usd = token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
        
        let confidence = match liquidity_usd {
            x if x >= 1_000_000.0 => 85.0, // High liquidity = high confidence
            x if x >= 100_000.0 => 75.0,   // Good liquidity
            x if x >= 10_000.0 => 65.0,    // Moderate liquidity
            _ => 55.0,                     // Low liquidity = lower confidence
        };
        
        (true, confidence, "Deep drop detected".to_string())
    } else {
        (false, 0.0, "No deep drop signal".to_string())
    }
}
