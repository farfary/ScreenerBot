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
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// DEEP DROP ENTRY CONFIGURATION
const MAX_DATA_AGE_MINUTES: i64 = 2; // Reject any data older than 2 minutes
const DEEP_DROP_MIN_PERCENT: f64 = 10.0; // Minimum drop % for entry
const DEEP_DROP_MAX_PERCENT: f64 = 50.0; // Maximum drop % for entry  
const DEEP_DROP_TIME_WINDOW_SEC: i64 = 60; // Must happen in last 60 seconds
const TARGET_DROP_RATIO: f64 = 0.33; // Target 1/3 of recent high
const PERFORMANCE_CACHE_PATH: &str = "data/entry_performance.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPerformance {
    pub mint: String,
    pub volatility_24h: f64,
    pub avg_drop_magnitude: f64,
    pub best_entry_drops: Vec<f64>,
    pub price_changes_recorded: u32,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCache {
    pub tokens: HashMap<String, TokenPerformance>,
    pub last_cleanup: chrono::DateTime<chrono::Utc>,
}

/// Load performance cache from disk
pub fn load_performance_cache() -> PerformanceCache {
    match std::fs::read_to_string(PERFORMANCE_CACHE_PATH) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| PerformanceCache {
            tokens: HashMap::new(),
            last_cleanup: Utc::now(),
        }),
        Err(_) => PerformanceCache {
            tokens: HashMap::new(),
            last_cleanup: Utc::now(),
        },
    }
}

/// Save performance cache to disk
fn save_performance_cache(cache: &PerformanceCache) {
    if let Ok(content) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(PERFORMANCE_CACHE_PATH, content);
    }
}

/// Update token performance data with new price change
pub fn record_token_price_change(mint: &str, drop_percent: f64, volatility: f64) {
    let mut cache = load_performance_cache();
    
    let performance = cache.tokens.entry(mint.to_string()).or_insert(TokenPerformance {
        mint: mint.to_string(),
        volatility_24h: volatility,
        avg_drop_magnitude: 0.0,
        best_entry_drops: Vec::new(),
        price_changes_recorded: 0,
        last_updated: Utc::now(),
    });
    
    performance.volatility_24h = (performance.volatility_24h + volatility) / 2.0;
    performance.price_changes_recorded += 1;
    performance.last_updated = Utc::now();
    
    if drop_percent >= DEEP_DROP_MIN_PERCENT {
        performance.best_entry_drops.push(drop_percent);
        performance.best_entry_drops.sort_by(|a, b| b.partial_cmp(a).unwrap());
        performance.best_entry_drops.truncate(10); // Keep top 10
        
        let total_drops: f64 = performance.best_entry_drops.iter().sum();
        performance.avg_drop_magnitude = total_drops / (performance.best_entry_drops.len() as f64);
    }
    
    save_performance_cache(&cache);
}

/// Get volatility-based drop threshold for token
fn get_volatility_based_threshold(mint: &str, base_drop: f64) -> f64 {
    let cache = load_performance_cache();
    
    if let Some(performance) = cache.tokens.get(mint) {
        let volatility_multiplier = (performance.volatility_24h / 100.0).min(2.0).max(0.5);
        (base_drop * volatility_multiplier).max(DEEP_DROP_MIN_PERCENT).min(DEEP_DROP_MAX_PERCENT)
    } else {
        base_drop
    }
}

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
    
    // CORE LOGIC: Deep drop detection with volatility scaling
    let deep_drop_result = analyze_deep_drop_entry(
        current_pool_price,
        &price_history,
        &token.mint,
        pool_data_age
    ).await;

    if let Some((drop_percent, entry_reason)) = deep_drop_result {
        // Record this price change for performance tracking
        let volatility = calculate_price_volatility(&price_history, current_pool_price);
        record_token_price_change(&token.mint, drop_percent, volatility);
        
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

/// Get profit target range based on pool liquidity and volatility
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    let pool_service = get_pool_service();
    
    let liquidity_usd = if let Some(pool_result) = pool_service.get_pool_price(&token.mint, None).await {
        pool_result.liquidity_usd
    } else {
        token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0)
    };

    // Get volatility-adjusted targets
    let cache = load_performance_cache();
    let volatility_multiplier = if let Some(performance) = cache.tokens.get(&token.mint) {
        (performance.volatility_24h / 100.0).min(3.0).max(0.5)
    } else {
        1.0
    };

    // Base targets adjusted by volatility and liquidity
    let (base_min, base_max) = match liquidity_usd {
        x if x >= 1_000_000.0 => (5.0, 15.0),
        x if x >= 100_000.0 => (10.0, 30.0), 
        x if x >= 10_000.0 => (20.0, 50.0),
        _ => (30.0, 100.0),
    };

    let min_target = (base_min * volatility_multiplier).max(5.0);
    let max_target = (base_max * volatility_multiplier).max(min_target + 5.0);
    
    (min_target, max_target)
}

/// Get dynamic entry threshold based on token volatility performance
pub fn get_entry_threshold(token: &Token) -> f64 {
    let base_threshold = DEEP_DROP_MIN_PERCENT;
    get_volatility_based_threshold(&token.mint, base_threshold)
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
    mint: &str,
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
    
    // Get volatility-adjusted threshold
    let base_threshold = DEEP_DROP_MIN_PERCENT;
    let adjusted_threshold = get_volatility_based_threshold(mint, base_threshold);
    
    // Strategy 2: Deep drop detection (main entry condition)
    if drop_percent >= adjusted_threshold && drop_percent <= DEEP_DROP_MAX_PERCENT {
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
        // Calculate confidence based on volatility and performance data
        let cache = load_performance_cache();
        let confidence = if let Some(performance) = cache.tokens.get(&token.mint) {
            // Higher confidence for tokens with good drop history
            let base_confidence = 70.0;
            let volatility_bonus = (performance.volatility_24h / 100.0 * 20.0).min(20.0);
            let experience_bonus = (performance.price_changes_recorded as f64 / 10.0 * 10.0).min(10.0);
            
            (base_confidence + volatility_bonus + experience_bonus).min(100.0)
        } else {
            65.0 // Default confidence for new tokens
        };
        
        (true, confidence, "Deep drop detected".to_string())
    } else {
        (false, 0.0, "No deep drop signal".to_string())
    }
}
