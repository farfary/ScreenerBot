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

// DYNAMIC DROP ENTRY CONFIGURATION - LIQUIDITY-BASED TARGETING
const MAX_DATA_AGE_MINUTES: i64 = 2; // Reject any data older than 2 minutes

// LIQUIDITY TARGETING RANGES (Our focus: 10k$ to 500k$)
const TARGET_LIQUIDITY_MIN: f64 = 10_000.0;   // Minimum liquidity we target
const TARGET_LIQUIDITY_MAX: f64 = 500_000.0;  // Maximum liquidity we target

// DYNAMIC DROP PERCENTAGE RANGES (not fixed - calculated by liquidity)
const DROP_PERCENT_MIN: f64 = 5.0;   // Minimum drop % for high liquidity tokens
const DROP_PERCENT_MAX: f64 = 30.0;  // Maximum drop % for low liquidity tokens
const DROP_PERCENT_ULTRA_MAX: f64 = 50.0; // Absolute maximum (safety limit)

// TIME WINDOWS FOR ANALYSIS
const DEEP_DROP_TIME_WINDOW_SEC: i64 = 60; // Standard time window for drop analysis
const FAST_DROP_TIME_WINDOW_SEC: i64 = 10; // Fast drop detection window

// DYNAMIC TARGET RATIOS
const TARGET_DROP_RATIO_MIN: f64 = 0.10; // 15% drop for high liquidity (conservative)
const TARGET_DROP_RATIO_MAX: f64 = 0.20; // 40% drop for low liquidity (aggressive)

/// Calculate dynamic drop thresholds based on token liquidity
/// Returns (min_drop_percent, max_drop_percent, target_ratio) based on liquidity
fn get_liquidity_based_thresholds(liquidity_usd: f64) -> (f64, f64, f64) {
    // Clamp liquidity to our target range
    let clamped_liquidity = liquidity_usd.max(TARGET_LIQUIDITY_MIN).min(TARGET_LIQUIDITY_MAX);
    
    // Calculate liquidity ratio (0.0 = min liquidity, 1.0 = max liquidity)
    let liquidity_ratio = (clamped_liquidity - TARGET_LIQUIDITY_MIN) / (TARGET_LIQUIDITY_MAX - TARGET_LIQUIDITY_MIN);
    
    // INVERSE RELATIONSHIP: Higher liquidity = smaller drops needed, Lower liquidity = larger drops needed
    let min_drop = DROP_PERCENT_MAX - (liquidity_ratio * (DROP_PERCENT_MAX - DROP_PERCENT_MIN));
    let max_drop = DROP_PERCENT_ULTRA_MAX;
    let target_ratio = TARGET_DROP_RATIO_MAX - (liquidity_ratio * (TARGET_DROP_RATIO_MAX - TARGET_DROP_RATIO_MIN));
    
    (min_drop, max_drop, target_ratio)
}

/// Deep drop entry decision with dynamic liquidity-based scaling
/// Returns true if token shows deep drop pattern for immediate entry
pub async fn should_buy(token: &Token) -> bool {
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "ENTRY_CHECK_START", &format!("🔍 Analyzing {} ({})", token.symbol, &token.mint[..8]));
    }
    
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

    // Get current pool price with age validation AND liquidity data
    let (current_pool_price, pool_data_age, liquidity_usd) = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => {
                    let data_age_minutes = (Utc::now() - pool_result.calculated_at).num_minutes();
                    
                    if data_age_minutes > MAX_DATA_AGE_MINUTES {
                        if is_debug_entry_enabled() {
                            log(LogTag::Entry, "DATA_AGE_REJECT", &format!("❌ {} data too old: {}min > {}min", 
                                token.symbol, data_age_minutes, MAX_DATA_AGE_MINUTES));
                        }
                        return false;
                    }
                    
                    // Get liquidity or fallback to token data
                    let liquidity = pool_result.liquidity_usd.max(
                        token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0)
                    );
                    
                    if is_debug_entry_enabled() {
                        log(LogTag::Entry, "POOL_DATA", &format!("📊 {} price: {:.12} SOL, liquidity: ${:.0}, age: {}min", 
                            token.symbol, price, liquidity, data_age_minutes));
                    }
                    
                    (price, data_age_minutes, liquidity)
                },
                _ => {
                    if is_debug_entry_enabled() {
                        log(LogTag::Entry, "PRICE_INVALID", &format!("❌ {} invalid pool price", token.symbol));
                    }
                    return false;
                }
            }
        }
        None => {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "NO_POOL_DATA", &format!("❌ {} no pool data available", token.symbol));
            }
            return false;
        }
    };

    // Skip tokens outside our liquidity target range
    if liquidity_usd < TARGET_LIQUIDITY_MIN || liquidity_usd > TARGET_LIQUIDITY_MAX {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry, 
                "LIQUIDITY_FILTER", 
                &format!("❌ {} liquidity ${:.0} outside target range ${:.0}-${:.0}", 
                    token.symbol, liquidity_usd, TARGET_LIQUIDITY_MIN, TARGET_LIQUIDITY_MAX
                )
            );
        }
        return false;
    }

    // Get recent price history for deep drop analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "PRICE_HISTORY", &format!("📈 {} has {} price points for analysis", 
            token.symbol, price_history.len()));
    }
    
    // CORE LOGIC: Dynamic drop detection based on liquidity
    let deep_drop_result = analyze_deep_drop_entry(
        current_pool_price,
        &price_history,
        pool_data_age,
        liquidity_usd
    ).await;

    if let Some((drop_percent, entry_reason)) = deep_drop_result {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "DYNAMIC_DROP_ENTRY",
                &format!(
                    "🎯 {} DYNAMIC ENTRY: -{:.1}% {} (liquidity: ${:.0}, price: {:.12} SOL)",
                    token.symbol, drop_percent, entry_reason, liquidity_usd, current_pool_price
                )
            );
        }
        return true;
    }

    if is_debug_entry_enabled() {
        log(LogTag::Entry, "NO_ENTRY_SIGNAL", &format!("❌ {} no dynamic drop signal detected", token.symbol));
    }

    false
}

/// Get profit target range based on pool liquidity (DYNAMIC TARGETING)
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    let pool_service = get_pool_service();
    
    let liquidity_usd = if let Some(pool_result) = pool_service.get_pool_price(&token.mint, None).await {
        pool_result.liquidity_usd.max(
            token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0)
        )
    } else {
        token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0)
    };

    // DYNAMIC targets based on liquidity (INVERSE relationship like entry thresholds)
    // Higher liquidity = lower targets (safer), Lower liquidity = higher targets (more risk/reward)
    
    // Clamp to our target range
    let clamped_liquidity = liquidity_usd.max(TARGET_LIQUIDITY_MIN).min(TARGET_LIQUIDITY_MAX);
    let liquidity_ratio = (clamped_liquidity - TARGET_LIQUIDITY_MIN) / (TARGET_LIQUIDITY_MAX - TARGET_LIQUIDITY_MIN);
    
    // INVERSE: High liquidity = conservative targets, Low liquidity = aggressive targets  
    let base_min = 50.0 - (liquidity_ratio * 40.0); // 50% down to 10%
    let base_max = 150.0 - (liquidity_ratio * 100.0); // 150% down to 50%
    
    let min_target = base_min.max(8.0);  // Never below 8%
    let max_target = base_max.max(min_target + 10.0); // Always at least 10% range
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "PROFIT_TARGET", &format!("🎯 {} targets: {:.1}%-{:.1}% (liquidity: ${:.0})", 
            token.symbol, min_target, max_target, liquidity_usd));
    }
    
    (min_target, max_target)
}

/// Get dynamic entry threshold based on liquidity (not fixed)
pub fn get_entry_threshold(token: &Token) -> f64 {
    let liquidity_usd = token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(TARGET_LIQUIDITY_MIN);
    let (min_drop, _max_drop, _target_ratio) = get_liquidity_based_thresholds(liquidity_usd);
    min_drop
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

/// Dynamic drop analysis with liquidity-based entry decisions
/// Returns Some((drop_percent, reason)) if dynamic drop detected, None otherwise
async fn analyze_deep_drop_entry(
    current_price: f64,
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    data_age_minutes: i64,
    liquidity_usd: f64
) -> Option<(f64, String)> {
    use chrono::Utc;
    
    // Get dynamic thresholds based on liquidity
    let (min_drop_threshold, max_drop_threshold, target_drop_ratio) = get_liquidity_based_thresholds(liquidity_usd);
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "DROP_THRESHOLDS", &format!("🎯 Dynamic thresholds for ${:.0}k: {:.1}%-{:.1}%, ratio: {:.1}%", 
            liquidity_usd / 1000.0, min_drop_threshold, max_drop_threshold, target_drop_ratio * 100.0));
    }
    
    // Strategy 1: Immediate entry for ultra-fresh data (only for good liquidity)
    if data_age_minutes == 0 && price_history.is_empty() && liquidity_usd >= 50_000.0 {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "ULTRA_FRESH_ENTRY", &format!("⚡ Ultra-fresh entry for ${:.0}k liquidity", liquidity_usd / 1000.0));
        }
        return Some((0.0, format!("ultra-fresh entry (${:.0}k liquidity)", liquidity_usd / 1000.0)));
    }
    
    // Need at least 2 data points for drop analysis
    if price_history.len() < 2 {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "INSUFFICIENT_DATA", "❌ Need at least 2 price points for drop analysis");
        }
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
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "INVALID_DROP", &format!("❌ Invalid drop calculation: {:.2}%", drop_percent));
        }
        return None;
    }
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "DROP_ANALYSIS", &format!("📉 Drop: {:.2}% (high: {:.12} → current: {:.12})", 
            drop_percent, recent_high, current_price));
    }
    
    // Strategy 2: Dynamic drop detection (main entry condition) - LIQUIDITY ADJUSTED
    if drop_percent >= min_drop_threshold && drop_percent <= max_drop_threshold {
        let time_span = recent_prices.len();
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "DYNAMIC_DROP_HIT", &format!("✅ Dynamic drop {:.1}% in range {:.1}%-{:.1}%", 
                drop_percent, min_drop_threshold, max_drop_threshold));
        }
        return Some((
            drop_percent, 
            format!("dynamic drop in {}pts (${:.0}k: {:.1}%-{:.1}%)", 
                time_span, liquidity_usd / 1000.0, min_drop_threshold, max_drop_threshold
            )
        ));
    }
    
    // Strategy 3: Dynamic target ratio drop detection - LIQUIDITY ADJUSTED
    let target_drop_absolute = recent_high * target_drop_ratio;
    let current_drop_absolute = recent_high - current_price;
    
    if current_drop_absolute >= target_drop_absolute {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "TARGET_RATIO_HIT", &format!("✅ Target ratio hit: {:.6} ≥ {:.6} SOL", 
                current_drop_absolute, target_drop_absolute));
        }
        return Some((
            drop_percent, 
            format!("dynamic target hit {:.1}% (${:.0}k ratio: {:.1}%)", 
                drop_percent, liquidity_usd / 1000.0, target_drop_ratio * 100.0
            )
        ));
    }
    
    // Strategy 4: Fast dynamic drop (higher threshold but faster timeframe) - LIQUIDITY ADJUSTED
    let ultra_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= FAST_DROP_TIME_WINDOW_SEC)
        .cloned()
        .collect();
    
    if ultra_recent.len() >= 2 {
        let ultra_high = ultra_recent.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
        
        if ultra_high > 0.0 && ultra_high.is_finite() {
            let ultra_drop = ((ultra_high - current_price) / ultra_high) * 100.0;
            
            // Fast drop threshold is 1.5x the minimum threshold for that liquidity level
            let fast_threshold = min_drop_threshold * 1.5;
            
            if ultra_drop >= fast_threshold && ultra_drop <= max_drop_threshold {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "FAST_DROP_HIT", &format!("⚡ Fast drop {:.1}% ≥ {:.1}% threshold", 
                        ultra_drop, fast_threshold));
                }
                return Some((
                    ultra_drop, 
                    format!("fast dynamic drop {:.1}% (${:.0}k: ≥{:.1}%)", 
                        ultra_drop, liquidity_usd / 1000.0, fast_threshold
                    )
                ));
            }
        }
    }
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "NO_DROP_SIGNAL", &format!("❌ No drop signals: {:.1}% (need {:.1}%-{:.1}%)", 
            drop_percent, min_drop_threshold, max_drop_threshold));
    }
    
    None
}

/// Enhanced entry decision with liquidity-based confidence scoring 
/// Returns (should_enter, confidence_score, reason)
pub async fn should_buy_with_confidence(token: &Token) -> (bool, f64, String) {
    // Check blacklist first
    if is_token_excluded_from_trading(&token.mint) {
        return (false, 0.0, "Token blacklisted or excluded".to_string());
    }

    // Use the main should_buy logic for entry decision
    let should_enter = should_buy(token).await;
    
    if should_enter {
        // Calculate confidence based on liquidity positioning within our target range
        let liquidity_usd = token.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
        
        let confidence = if liquidity_usd < TARGET_LIQUIDITY_MIN {
            45.0 // Below our target range = lower confidence
        } else if liquidity_usd > TARGET_LIQUIDITY_MAX {
            60.0 // Above our target range = moderate confidence  
        } else {
            // Within our target range - calculate position-based confidence
            let position_in_range = (liquidity_usd - TARGET_LIQUIDITY_MIN) / (TARGET_LIQUIDITY_MAX - TARGET_LIQUIDITY_MIN);
            
            // SWEET SPOT: Middle of our range gets highest confidence
            let distance_from_center = (position_in_range - 0.5).abs() * 2.0; // 0.0 = center, 1.0 = edges
            let base_confidence = 85.0 - (distance_from_center * 15.0); // 85% at center, 70% at edges
            
            base_confidence.max(70.0).min(85.0)
        };
        
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "CONFIDENCE_SCORE", &format!("🎯 Confidence: {:.1}% for ${:.0}k liquidity", 
                confidence, liquidity_usd / 1000.0));
        }
        
        (true, confidence, format!("Dynamic drop detected (${:.0}k liquidity)", liquidity_usd / 1000.0))
    } else {
        (false, 0.0, "No dynamic drop signal".to_string())
    }
}
