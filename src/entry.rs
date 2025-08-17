/// Pool-based entry logic for ScreenerBot
///
/// This module provides pool price-based entry decisions with -10% drop detection.
/// Uses real-time blockchain pool data for trading decisions while API data is used only for validation.
/// Enhanced with 2-minute data age filtering and RL learning advisory (non-blocking).
/// OPTIMIZED FOR FAST TRADING: Sub-minute decisions with pool price priority.

use crate::tokens::Token;
use crate::tokens::get_pool_service;
use crate::tokens::is_token_excluded_from_trading;
use crate::logger::{ log, LogTag };
use crate::global::{ is_debug_entry_enabled };
use crate::rl_learning::{ get_trading_learner, collect_market_features };
use crate::tokens::cache::TokenDatabase;
use chrono::Utc;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

// ============================================================================
// 🎯 TRADING PARAMETERS - HARDCODED CONFIGURATION
// ============================================================================

// DATA AGE LIMITS
const MAX_DATA_AGE_MINUTES: i64 = 10; // Extended from 5 to 10 minutes for maximum opportunities

// LIQUIDITY TARGETING RANGES (Expanded: $5k to $1M)
const TARGET_LIQUIDITY_MIN: f64 = 5000.0;    // Reduced from 10k to 5k - catch smaller tokens
const TARGET_LIQUIDITY_MAX: f64 = 1_000_000.0; // Increased from 500k to 1M - catch bigger opportunities

// DROP PERCENTAGE RANGES (Ultra-aggressive - catch micro drops)
const DROP_PERCENT_MIN: f64 = 1.0;   // Reduced from 3% to 1% - catch micro drops
const DROP_PERCENT_MAX: f64 = 15.0;  // Reduced from 20% to 15% - focus on smaller moves
const DROP_PERCENT_ULTRA_MAX: f64 = 30.0; // Reduced from 40% - avoid extreme crashes

// TIME WINDOWS FOR ANALYSIS (Multiple ultra-aggressive timeframes)
const INSTANT_DROP_TIME_WINDOW_SEC: i64 = 5;   // NEW: Ultra-fast detection (5 seconds)
const FAST_DROP_TIME_WINDOW_SEC: i64 = 10;     // Fast drop detection
const DEEP_DROP_TIME_WINDOW_SEC: i64 = 60;     // Standard time window
const MEDIUM_DROP_TIME_WINDOW_SEC: i64 = 120;  // Medium-term analysis (2 minutes)
const LONG_DROP_TIME_WINDOW_SEC: i64 = 300;    // Long-term analysis (5 minutes)
const EXTENDED_DROP_TIME_WINDOW_SEC: i64 = 600; // NEW: Extended analysis (10 minutes)
const NEAR_TOP_ANALYSIS_WINDOW_SEC: i64 = 900; // Near-top analysis (15 minutes)
// Dynamic near-top floor/ceiling based on liquidity and volatility
const NEAR_TOP_THRESHOLD_MIN: f64 = 8.0;   // never below 8%
const NEAR_TOP_THRESHOLD_MAX: f64 = 20.0;  // never above 20%

// DYNAMIC TARGET RATIOS (Ultra-aggressive)
const TARGET_DROP_RATIO_MIN: f64 = 0.05; // Reduced from 0.08 (5% instead of 8%) 
const TARGET_DROP_RATIO_MAX: f64 = 0.12; // Reduced from 0.15 (12% instead of 15%)

// STRATEGY-SPECIFIC PARAMETERS
const ULTRA_FRESH_MIN_LIQUIDITY: f64 = 500.0; // Minimum liquidity for ultra-fresh entries
const SMALL_TOKEN_MIN_DROP: f64 = 10.0;          // Reduced from 15% to 10% - more small token entries
const LARGE_TOKEN_MIN_DROP: f64 = 2.0;           // Reduced from 5% to 2% - catch tiny moves in large tokens  
const LONG_TERM_MIN_LIQUIDITY: f64 = 10_000.0;   // Reduced from 50k to 10k - more long-term entries
const VOLUME_MULTIPLIER_HIGH: f64 = 1.5;         // Reduced from 2.0x to 1.5x - easier volume requirements
const VOLUME_MULTIPLIER_LARGE: f64 = 0.3;        // Reduced from 0.5x to 0.3x - easier large token requirements
const MIN_VOLUME_DROP: f64 = 0.2;                // Reduced from 0.5% to 0.2% - catch micro volume moves
const MICRO_DROP_THRESHOLD: f64 = 0.5;           // NEW: Micro drops for mega liquidity tokens
const VOLUME_SPIKE_MULTIPLIER: f64 = 3.0;        // NEW: Volume spike detection (3x normal volume)

// NEAR-TOP FILTER PARAMETERS (Prevent buying at recent peaks)
const NEAR_TOP_THRESHOLD_PERCENT: f64 = 10.0;    // Must be MORE than 10% below 15-min high to enter
// Multi-window near-top minimums (stricter near fresh highs)
const NEAR_TOP_1M_MIN: f64 = 3.0;  // at least 3% below 1m high
const NEAR_TOP_5M_MIN: f64 = 6.0;  // at least 6% below 5m high
// Cooldown after making a new window high to avoid buying the spike
const COOLDOWN_AFTER_NEW_HIGH_SEC: i64 = 45;
// ATH proximity guard across all available history
const ATH_PROXIMITY_PERCENT: f64 = 3.5; // avoid entries within ~3.5% of observed ATH
// Toggle risky ultra-fresh entries (disabled by default)
const ULTRA_FRESH_ENTRY_ENABLED: bool = false;

// PROFIT TARGET CALCULATION PARAMETERS
const PROFIT_BASE_MIN: f64 = 50.0;               // Base minimum profit target %
const PROFIT_BASE_MAX: f64 = 150.0;              // Base maximum profit target %
const PROFIT_LIQUIDITY_ADJUSTMENT_MIN: f64 = 40.0; // Liquidity adjustment for min target
const PROFIT_LIQUIDITY_ADJUSTMENT_MAX: f64 = 100.0; // Liquidity adjustment for max target
const PROFIT_TARGET_MIN_FLOOR: f64 = 8.0;        // Never go below 8% profit target
const PROFIT_TARGET_MIN_RANGE: f64 = 10.0;       // Always at least 10% range

// FAST DROP MULTIPLIER
const FAST_DROP_THRESHOLD_MULTIPLIER: f64 = 1.2; // Fast drop threshold = 1.2x the min threshold (was 1.5x)

// CONFIDENCE SCORING PARAMETERS (for should_buy_with_confidence)
const CONFIDENCE_BELOW_RANGE: f64 = 45.0;        // Confidence for tokens below target liquidity
const CONFIDENCE_ABOVE_RANGE: f64 = 60.0;        // Confidence for tokens above target liquidity
const CONFIDENCE_CENTER_MAX: f64 = 85.0;         // Maximum confidence at center of range
const CONFIDENCE_EDGE_MIN: f64 = 70.0;           // Minimum confidence at edges of range
const CONFIDENCE_CENTER_ADJUSTMENT: f64 = 15.0;  // Adjustment factor for distance from center

// MATHEMATICAL CONSTANTS
const PERCENTAGE_MULTIPLIER: f64 = 100.0;        // Convert ratio to percentage
const THOUSAND_DIVISOR: f64 = 1000.0;            // Convert to thousands for display
const MILLION_DIVISOR: f64 = 1_000_000.0;        // Convert to millions for display
const MINUTES_PER_SECOND: i64 = 60;              // Time conversion

// ============================================================================

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

/// Check if current price is near recent top (15-minute high)
/// Returns true if price is too close to recent peak (should NOT enter)
fn is_near_recent_top(
    current_price: f64,
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    _liquidity_usd: f64  // Not used anymore, kept for compatibility
) -> bool {
    use chrono::Utc;
    
    if price_history.is_empty() {
        // No history = can't determine if near top, allow entry
        return false;
    }
    
    // Get prices from last 15 minutes
    let now = Utc::now();
    let mut recent_prices: Vec<f64> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= NEAR_TOP_ANALYSIS_WINDOW_SEC)
        .map(|(_, price)| *price)
        .collect();
    
    if recent_prices.len() < 3 {
        // Not enough data points, allow entry
        return false;
    }
    
    // Find the highest/lowest price in the 15-minute window and when it occurred
    let mut recent_high = 0.0f64;
    let mut recent_low = f64::INFINITY;
    let mut recent_high_ts = None;
    for (ts, price) in price_history.iter() {
        if (now - *ts).num_seconds() <= NEAR_TOP_ANALYSIS_WINDOW_SEC {
            if *price > recent_high { recent_high = *price; recent_high_ts = Some(*ts); }
            if *price < recent_low { recent_low = *price; }
        }
    }
    
    if recent_high <= 0.0 || !recent_high.is_finite() || current_price <= 0.0 || !current_price.is_finite() {
        return false;
    }
    
    // Calculate how much BELOW the recent high we are
    let drop_from_high_percent = ((recent_high - current_price) / recent_high) * PERCENTAGE_MULTIPLIER;

    // Dynamic near-top threshold: tighter when volatility and liquidity are low, looser when high
    let range_pct = if recent_low.is_finite() && recent_low > 0.0 {
        ((recent_high - recent_low) / recent_high).max(0.0) * 100.0
    } else { 0.0 };
    // Map range 0..30% -> threshold 12..8 (more conservative near tops when calm), beyond 30% -> 15%
    let mut dynamic_threshold = if range_pct < 30.0 {
        // linear from 12 down to 8
        12.0 - (range_pct / 30.0) * 4.0
    } else if range_pct < 80.0 {
        // moderate volatility -> increase threshold to avoid top entries
        12.0 + ((range_pct - 30.0) / 50.0) * 3.0 // up to 15%
    } else {
        15.0
    };
    // Clamp to global min/max bounds
    dynamic_threshold = dynamic_threshold.max(NEAR_TOP_THRESHOLD_MIN).min(NEAR_TOP_THRESHOLD_MAX);

    // STRICT RULE: Must be MORE than dynamic_threshold below 15-min high to allow entry
    let mut is_too_close_to_top = drop_from_high_percent < dynamic_threshold;

    // Additional 5m and 1m window checks (prevent buys near shorter-term highs)
    let window_check = |secs: i64| -> Option<f64> {
        let high = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= secs)
            .map(|(_, p)| *p)
            .fold(0.0f64, |a, b| a.max(b));
        if high > 0.0 && high.is_finite() {
            Some(((high - current_price) / high) * 100.0)
        } else { None }
    };
    if let Some(drop_5m) = window_check(300) {
        if drop_5m < NEAR_TOP_5M_MIN { is_too_close_to_top = true; }
    }
    if let Some(drop_1m) = window_check(60) {
        if drop_1m < NEAR_TOP_1M_MIN { is_too_close_to_top = true; }
    }

    // Cooldown after printing a new 15m high
    if let Some(high_ts) = recent_high_ts {
        let secs_since_high = (now - high_ts).num_seconds();
        if secs_since_high >= 0 && secs_since_high <= COOLDOWN_AFTER_NEW_HIGH_SEC {
            is_too_close_to_top = true;
        }
    }

    // ATH guard across all available history (observed within the provided history span)
    let observed_ath = price_history.iter().map(|(_, p)| *p).fold(0.0f64, |a, b| a.max(b));
    if observed_ath > 0.0 && observed_ath.is_finite() {
        let drop_from_ath = ((observed_ath - current_price) / observed_ath) * 100.0;
        if drop_from_ath < ATH_PROXIMITY_PERCENT { is_too_close_to_top = true; }
    }
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "NEAR_TOP_CHECK", &format!(
            "🔝 High proximity: current={:.12} | 15m drop={:.2}% req>{:.1}% (range={:.1}%) | 5m req>{:.1}% | 1m req>{:.1}% | cooldown={} | ath_guard={} -> too_close={}",
            current_price, drop_from_high_percent, dynamic_threshold, range_pct, NEAR_TOP_5M_MIN, NEAR_TOP_1M_MIN,
            COOLDOWN_AFTER_NEW_HIGH_SEC, ATH_PROXIMITY_PERCENT, is_too_close_to_top
        ));
    }
    
    is_too_close_to_top
}

/// Deep drop entry decision with dynamic liquidity-based scaling
/// Returns true if token shows deep drop pattern for immediate entry
pub async fn should_buy(token: &Token) -> (bool, f64, String) {
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "ENTRY_CHECK_START", &format!("🔍 Analyzing {} ({})", token.symbol, &token.mint[..8]));
    }
    
    // Check blacklist first
    if is_token_excluded_from_trading(&token.mint) {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "BLACKLIST_REJECT", &format!("❌ {} blacklisted", token.symbol));
        }
    return (false, 0.0, "Token blacklisted or excluded".to_string());
    }

    let pool_service = get_pool_service();
    
    if !pool_service.check_token_availability(&token.mint).await {
        return (false, 0.0, "Pool not available".to_string());
    }

    // Get current pool price with age validation AND liquidity data
    let (current_pool_price, pool_data_age, liquidity_usd) = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => {
                    let data_age_minutes = (Utc::now() - pool_result.calculated_at).num_seconds() / MINUTES_PER_SECOND;
                    
                    if data_age_minutes > MAX_DATA_AGE_MINUTES {
                        if is_debug_entry_enabled() {
                            log(LogTag::Entry, "DATA_AGE_REJECT", &format!("❌ {} data too old: {}min > {}min", 
                                token.symbol, data_age_minutes, MAX_DATA_AGE_MINUTES));
                        }
                        return (false, 0.0, format!("Pool data too old: {}min > {}min", data_age_minutes, MAX_DATA_AGE_MINUTES));
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
                    return (false, 0.0, "Invalid pool price".to_string());
                }
            }
        }
        None => {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "NO_POOL_DATA", &format!("❌ {} no pool data available", token.symbol));
            }
            return (false, 0.0, "No pool data available".to_string());
        }
    };

    // Ultra-flexible liquidity filtering - allow almost any token with meaningful volume
    if liquidity_usd < TARGET_LIQUIDITY_MIN {
        // Allow micro tokens (even under $500) if they have volume or big drops
        if liquidity_usd < 100.0 {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "NANO_LIQUIDITY_REJECT", &format!("❌ {} liquidity ${:.0} too small (under $100)", 
                    token.symbol, liquidity_usd));
            }
            return (false, 0.0, format!("Liquidity ${:.0} too small (under $100)", liquidity_usd));
        }
    } else if liquidity_usd > TARGET_LIQUIDITY_MAX {
        // Allow mega tokens (even over $10M) - no upper limit rejection
        if liquidity_usd > 50_000_000.0 {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "GIGA_LIQUIDITY_NOTICE", &format!("📈 {} mega liquidity ${:.0}M detected", 
                    token.symbol, liquidity_usd / 1_000_000.0));
            }
            // Don't reject, just log for visibility
        }
    }
    
    if is_debug_entry_enabled() && (liquidity_usd < TARGET_LIQUIDITY_MIN || liquidity_usd > TARGET_LIQUIDITY_MAX) {
        log(LogTag::Entry, "EXTENDED_LIQUIDITY_ACCEPT", &format!("✅ {} liquidity ${:.0} outside target but allowed", 
            token.symbol, liquidity_usd));
    }

    // Get recent price history for deep drop analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "PRICE_HISTORY", &format!("📈 {} has {} price points for analysis", 
            token.symbol, price_history.len()));
    }
    
    // CRITICAL SAFETY CHECK: Reject entries if price is near recent top (multi-window + ATH guards)
    if is_near_recent_top(current_pool_price, &price_history, liquidity_usd) {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "NEAR_TOP_REJECT", &format!("🚫 {} rejected: price too close to 15-min high (safety filter)", token.symbol));
        }
    return (false, 0.0, "Price too close to highs (multi-window/ATH guard)".to_string());
    }
    
    // CORE LOGIC: Dynamic drop detection based on liquidity
    let volume_24h = token.volume.as_ref().and_then(|v| v.h24);
    let deep_drop_result = analyze_deep_drop_entry(
        &token.mint,
        current_pool_price,
        &price_history,
        pool_data_age,
        liquidity_usd,
        volume_24h
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
        // Confidence scoring (merged from should_buy_with_confidence)
        let confidence = if liquidity_usd < TARGET_LIQUIDITY_MIN {
            CONFIDENCE_BELOW_RANGE
        } else if liquidity_usd > TARGET_LIQUIDITY_MAX {
            CONFIDENCE_ABOVE_RANGE
        } else {
            let position_in_range = (liquidity_usd - TARGET_LIQUIDITY_MIN) / (TARGET_LIQUIDITY_MAX - TARGET_LIQUIDITY_MIN);
            let distance_from_center = (position_in_range - 0.5).abs() * 2.0; // 0.0 = center, 1.0 = edges
            let base_confidence = CONFIDENCE_CENTER_MAX - (distance_from_center * CONFIDENCE_CENTER_ADJUSTMENT);
            base_confidence.max(CONFIDENCE_EDGE_MIN).min(CONFIDENCE_CENTER_MAX)
        };

        if is_debug_entry_enabled() {
            log(LogTag::Entry, "CONFIDENCE_SCORE", &format!("🎯 Confidence: {:.1}% for ${:.0}k liquidity", 
                confidence, liquidity_usd / THOUSAND_DIVISOR));
        }

        let reason = format!("{} (${:.0}k liquidity)", entry_reason, liquidity_usd / THOUSAND_DIVISOR);
        return (true, confidence, reason);
    }

    // RL-ASSISTED FALLBACK: If no drop signal, consult learning system for promising timing
    if let Some(vol_24h) = token.volume.as_ref().and_then(|v| v.h24) {
        let learner = get_trading_learner();
        // Only attempt lightweight analysis; do not block trading paths
        if learner.is_model_ready() {
            let market_cap = token.market_cap;
            let rug = get_rugcheck_score_for_token(&token.mint).await;
            let analysis = learner.analyze_entry_opportunity(
                &token.mint,
                current_pool_price,
                liquidity_usd,
                vol_24h,
                market_cap,
                rug
            ).await;
            {
                    // Accept only when combined score high and timing favorable, and price not near top (already checked)
                    if analysis.should_enter && analysis.confidence >= 0.6 {
                        let mut conf = analysis.combined_score * 100.0;
                        if conf < 55.0 { conf = 55.0; }
                        if conf > 90.0 { conf = 90.0; }
                        if is_debug_entry_enabled() {
                            log(LogTag::Entry, "RL_FALLBACK_ENTRY", &format!(
                                "🤖 RL-approved entry: score {:.2}, conf {:.2}", analysis.combined_score, analysis.confidence
                            ));
                        }
                        return (true, conf, "RL timing opportunity".to_string());
                    }
            }
        }
    }

    if is_debug_entry_enabled() {
        log(LogTag::Entry, "NO_ENTRY_SIGNAL", &format!("❌ {} no dynamic drop signal detected", token.symbol));
    }

    (false, 0.0, "No dynamic drop signal".to_string())
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
    let base_min: f64 = PROFIT_BASE_MIN - (liquidity_ratio * PROFIT_LIQUIDITY_ADJUSTMENT_MIN); // 50% down to 10%
    let base_max: f64 = PROFIT_BASE_MAX - (liquidity_ratio * PROFIT_LIQUIDITY_ADJUSTMENT_MAX); // 150% down to 50%
    
    let min_target = base_min.max(PROFIT_TARGET_MIN_FLOOR);  // Never below 8%
    let max_target = base_max.max(min_target + PROFIT_TARGET_MIN_RANGE); // Always at least 10% range
    
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
        ((max_price - min_price) / min_price) * PERCENTAGE_MULTIPLIER
    } else {
        10.0
    }
}

/// Dynamic drop analysis with liquidity-based entry decisions
/// Returns Some((drop_percent, reason)) if dynamic drop detected, None otherwise
async fn analyze_deep_drop_entry(
    mint: &str,
    current_price: f64,
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    data_age_minutes: i64,
    liquidity_usd: f64,
    volume_24h: Option<f64>
) -> Option<(f64, String)> {
    use chrono::Utc;
    
    // --- Adaptive tuner (runtime auto-tuning of min drop) -----------------
    // Lightweight, in-memory EMA-based scaler per token
    struct TunerState {
        scale_ema: f64,        // threshold multiplier (0.6..1.4)
        ema_volatility: f64,   // smoothed volatility %
        ema_velocity: f64,     // smoothed pct/minute (+/-)
        last_update: Instant,
    }

    struct AdaptiveDropTuner {
        inner: RwLock<HashMap<String, TunerState>>,
    }

    static ADAPTIVE_TUNER: OnceLock<AdaptiveDropTuner> = OnceLock::new();
    fn get_adaptive_tuner() -> &'static AdaptiveDropTuner {
        ADAPTIVE_TUNER.get_or_init(|| AdaptiveDropTuner { inner: RwLock::new(HashMap::new()) })
    }
    
    // Get dynamic thresholds based on liquidity
    let (base_min_drop, max_drop_threshold, target_drop_ratio) = get_liquidity_based_thresholds(liquidity_usd);
    // Slight relaxation to increase opportunities; other safeties prevent top buys
    let min_drop_threshold = (base_min_drop * 0.9).max(DROP_PERCENT_MIN);

    // Volatility-aware adjustment: in higher volatility allow smaller drops to qualify
    let vol_percent = calculate_price_volatility(price_history, current_price);
    let volatility_factor = if vol_percent > 100.0 { 0.7 } else if vol_percent > 60.0 { 0.8 } else if vol_percent > 30.0 { 0.9 } else { 1.0 };

    // Short-term velocity (percent per minute) over last ~30s
    let velocity_per_minute = {
        let now = Utc::now();
        let window_sec: i64 = 30;
        let recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
            .iter()
            .filter(|(ts, _)| (now - *ts).num_seconds() <= window_sec)
            .cloned()
            .collect();
        if recent.len() >= 2 {
            let first = recent.first().unwrap();
            let last = recent.last().unwrap();
            if first.1 > 0.0 && first.1.is_finite() && last.1.is_finite() {
                let dt = (last.0 - first.0).num_seconds().max(1) as f64;
                let pct_change = ((last.1 - first.1) / first.1) * 100.0; // % over dt seconds
                pct_change * (60.0 / dt) // % per minute
            } else { 0.0 }
        } else { 0.0 }
    };

    // Adaptive tuner: compute a per-mint scale with EMA smoothing
    let adaptive_scale = {
        // Compute a bounded target scale from current observations
        let vol_scale = if vol_percent <= 20.0 { 0.90 } else if vol_percent <= 40.0 { 0.98 } else if vol_percent <= 80.0 { 1.05 } else { 1.20 };
        let vel_scale = if velocity_per_minute < -6.0 {
            // accelerating downtrend: lower threshold to catch
            0.92
        } else if velocity_per_minute > 6.0 {
            // accelerating uptrend: raise threshold to avoid chasing
            1.10
        } else { 1.0 };
        let liq_scale = if liquidity_usd >= 200_000.0 { 0.92 } else if liquidity_usd <= 5_000.0 { 1.08 } else { 1.0 };
    let mut target_scale = vol_scale * vel_scale * liq_scale;
    if target_scale < 0.6f64 { target_scale = 0.6f64; }
    if target_scale > 1.4f64 { target_scale = 1.4f64; }

        // Update EMA state
        let tuner = get_adaptive_tuner();
        if let Ok(mut map) = tuner.inner.try_write() {
            let st = map.entry(mint.to_string()).or_insert(TunerState {
                scale_ema: 1.0,
                ema_volatility: vol_percent,
                ema_velocity: velocity_per_minute,
                last_update: Instant::now(),
            });
            let old_scale = st.scale_ema;
            st.ema_volatility = st.ema_volatility * 0.8 + vol_percent * 0.2;
            st.ema_velocity = st.ema_velocity * 0.8 + velocity_per_minute * 0.2;
            st.scale_ema = st.scale_ema * 0.7 + target_scale * 0.3;
            if st.scale_ema < 0.6f64 { st.scale_ema = 0.6f64; }
            if st.scale_ema > 1.4f64 { st.scale_ema = 1.4f64; }
            st.last_update = Instant::now();
            let new_scale = st.scale_ema;
            if is_debug_entry_enabled() && (new_scale - old_scale).abs() >= 0.05 {
                log(LogTag::Entry, "ADAPT_TUNE", &format!(
                    "🛠️ Tuner {}: vol {:.1}% vel {:.1}%/min liq ${:.0} → scale {:.2} (target {:.2})",
                    &mint[..8], vol_percent, velocity_per_minute, liquidity_usd, new_scale, target_scale
                ));
            }
            new_scale
        } else if let Ok(map_ro) = tuner.inner.try_read() {
            if let Some(st) = map_ro.get(mint) { st.scale_ema } else { 1.0 }
        } else { 1.0 }
    };

    let effective_min_drop = (min_drop_threshold * volatility_factor * adaptive_scale).max(DROP_PERCENT_MIN * 0.5);
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "DROP_THRESHOLDS", &format!("🎯 Dynamic thresholds for ${:.0}k: min {:.1}% (eff {:.1}%, scale {:.2}) - max {:.1}%, ratio: {:.1}% | vol {:.1}% vel {:.1}%/min", 
            liquidity_usd / THOUSAND_DIVISOR, min_drop_threshold, effective_min_drop, adaptive_scale, max_drop_threshold, target_drop_ratio * PERCENTAGE_MULTIPLIER, vol_percent, velocity_per_minute));
    }
    
    // Strategy 1: (disabled by default) Ultra-fresh entry — risky near tops
    if ULTRA_FRESH_ENTRY_ENABLED {
        if data_age_minutes == 0 && price_history.is_empty() && liquidity_usd >= ULTRA_FRESH_MIN_LIQUIDITY {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "ULTRA_FRESH_ENTRY", &format!("⚡ Ultra-fresh entry for ${:.0}k liquidity", liquidity_usd / THOUSAND_DIVISOR));
            }
            return Some((0.0, format!("ultra-fresh entry (${:.0}k liquidity)", liquidity_usd / THOUSAND_DIVISOR)));
        }
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
    let recent_low  = recent_prices.iter().map(|(_, price)| *price).fold(f64::INFINITY, |a, b| a.min(b));
    
    if recent_high <= 0.0 || !recent_high.is_finite() {
        return None;
    }
    
    let drop_percent = ((recent_high - current_price) / recent_high) * PERCENTAGE_MULTIPLIER;
    
    if !drop_percent.is_finite() || drop_percent < 0.0 {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "INVALID_DROP", &format!("❌ Invalid drop calculation: {:.2}%", drop_percent));
        }
        return None;
    }
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "DROP_ANALYSIS", &format!("📉 Drop: {:.2}% (high: {:.12} → current: {:.12}, low: {:.12})", 
            drop_percent, recent_high, current_price, recent_low));
    }

    // Bounce suppression: if price has already retraced > 35% of the drop from low -> avoid chasing tops
    if recent_low.is_finite() && recent_low > 0.0 && recent_high > 0.0 && current_price > recent_low {
        let total_drop_from_high = recent_high - recent_low;
        if total_drop_from_high.is_finite() && total_drop_from_high > 0.0 {
            let retrace = (current_price - recent_low) / total_drop_from_high; // 0..1
            if retrace >= 0.35 {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "BOUNCE_SUPPRESS", &format!(
                        "🚫 Retrace {:.0}% of drop detected (>{:.0}%), skipping to avoid buying bounce",
                        retrace * 100.0, 35.0
                    ));
                }
                return None;
            }
        }
    }
    
    // Strategy 1.5: Capitulation wick recovery (very short-term flush then snapback)
    // If we saw a sharp low within last seconds and current recovered a bit, but still well below recent high
    {
        const CAPITULATION_WINDOW_SEC: i64 = 20;
        let now2 = Utc::now();
        let cap_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
            .iter()
            .filter(|(ts, _)| (now2 - *ts).num_seconds() <= CAPITULATION_WINDOW_SEC)
            .cloned()
            .collect();
        if cap_recent.len() >= 2 {
            let cap_high = cap_recent.iter().map(|(_, p)| *p).fold(0.0_f64, |a, b| a.max(b));
            let cap_low = cap_recent.iter().map(|(_, p)| *p).fold(f64::INFINITY, |a, b| a.min(b));
            if cap_high > 0.0 && cap_low.is_finite() && cap_high.is_finite() {
                let flush_drop = ((cap_high - cap_low) / cap_high) * 100.0;
                let recovered_from_low = if current_price > 0.0 { ((current_price - cap_low) / current_price) * 100.0 } else { 0.0 };
                // Require a meaningful flush and slight recovery, but still below high enough
                if flush_drop >= (effective_min_drop * 1.1).min(20.0) && recovered_from_low >= 1.0 && drop_percent >= (effective_min_drop * 0.5) {
                    if is_debug_entry_enabled() {
                        log(LogTag::Entry, "CAPITULATION_WICK", &format!(
                            "🕯️ Capitulation wick: flush {:.1}% | recovered {:.1}% | drop_from_high {:.1}%",
                            flush_drop, recovered_from_low, drop_percent
                        ));
                    }
                    return Some((drop_percent.max(flush_drop), format!("capitulation wick {:.1}% (eff≥{:.1}%)", flush_drop, effective_min_drop)));
                }
            }
        }
    }
    
    // Strategy 2: Dynamic drop detection (main entry condition) - LIQUIDITY ADJUSTED
    if drop_percent >= effective_min_drop && drop_percent <= max_drop_threshold {
        let time_span = recent_prices.len();
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "DYNAMIC_DROP_HIT", &format!("✅ Dynamic drop {:.1}% in range {:.1}%-{:.1}%", 
                drop_percent, effective_min_drop, max_drop_threshold));
        }
        return Some((
            drop_percent, 
            format!("dynamic drop in {}pts (${:.0}k: {:.1}%-{:.1}%)", 
                time_span, liquidity_usd / THOUSAND_DIVISOR, effective_min_drop, max_drop_threshold
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
                drop_percent, liquidity_usd / THOUSAND_DIVISOR, target_drop_ratio * PERCENTAGE_MULTIPLIER
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
            let ultra_drop = ((ultra_high - current_price) / ultra_high) * PERCENTAGE_MULTIPLIER;
            
            // Fast drop threshold is 1.5x the minimum threshold for that liquidity level
            let fast_threshold = effective_min_drop * FAST_DROP_THRESHOLD_MULTIPLIER;
            
            if ultra_drop >= fast_threshold && ultra_drop <= max_drop_threshold {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "FAST_DROP_HIT", &format!("⚡ Fast drop {:.1}% ≥ {:.1}% threshold", 
                        ultra_drop, fast_threshold));
                }
                return Some((
                    ultra_drop, 
                    format!("fast dynamic drop {:.1}% (${:.0}k: ≥{:.1}%)", 
                        ultra_drop, liquidity_usd / THOUSAND_DIVISOR, fast_threshold
                    )
                ));
            }
        }
    }
    
    // Strategy 5: Small drop detection for high liquidity tokens (OPTIMIZED - lower requirements)
    if liquidity_usd >= 50_000.0 && drop_percent >= 0.5 && drop_percent < min_drop_threshold {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "SMALL_DROP_HIT", &format!("💰 Small drop {:.1}% for high liquidity ${:.0}k", 
                drop_percent, liquidity_usd / THOUSAND_DIVISOR));
        }
        return Some((
            drop_percent, 
            format!("small drop high-liq {:.1}% (${:.0}k)", 
                drop_percent, liquidity_usd / THOUSAND_DIVISOR
            )
        ));
    }
    
    // Strategy 6: Medium-term drop analysis (NEW - 2 minutes)
    let medium_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= MEDIUM_DROP_TIME_WINDOW_SEC)
        .cloned()
        .collect();
    
    if medium_recent.len() >= 3 {
        let medium_high = medium_recent.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
        
        if medium_high > 0.0 && medium_high.is_finite() {
            let medium_drop = ((medium_high - current_price) / medium_high) * 100.0;
            
            // Medium-term threshold is 0.8x the minimum (catch sustained drops)
            let medium_threshold = effective_min_drop * 0.8;
            
            if medium_drop >= medium_threshold && medium_drop <= max_drop_threshold {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "MEDIUM_DROP_HIT", &format!("📊 Medium-term drop {:.1}% ≥ {:.1}% threshold", 
                        medium_drop, medium_threshold));
                }
                return Some((
                    medium_drop, 
                    format!("medium-term drop {:.1}% (${:.0}k: ≥{:.1}%)", 
                        medium_drop, liquidity_usd / THOUSAND_DIVISOR, medium_threshold
                    )
                ));
            }
        }
    }
    
    // Strategy 7: Long-term drop analysis (NEW - 5 minutes)
    let long_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= LONG_DROP_TIME_WINDOW_SEC)
        .cloned()
        .collect();
    
    if long_recent.len() >= 5 {
        let long_high = long_recent.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
        
        if long_high > 0.0 && long_high.is_finite() {
            let long_drop = ((long_high - current_price) / long_high) * 100.0;
            
            // Long-term threshold is 0.6x the minimum (catch extended downtrends)
            let long_threshold = effective_min_drop * 0.6;
            
            if long_drop >= long_threshold && long_drop <= max_drop_threshold && liquidity_usd >= LONG_TERM_MIN_LIQUIDITY {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "LONG_DROP_HIT", &format!("📈 Long-term drop {:.1}% ≥ {:.1}% threshold", 
                        long_drop, long_threshold));
                }
                return Some((
                    long_drop, 
                    format!("long-term drop {:.1}% (${:.0}k: ≥{:.1}%)", 
                        long_drop, liquidity_usd / THOUSAND_DIVISOR, long_threshold
                    )
                ));
            }
        }
    }
    
    // Strategy 8: Volume-based entry (NEW - any meaningful drop with high volume)
    if let Some(vol_24h) = volume_24h {
        if vol_24h >= liquidity_usd * VOLUME_MULTIPLIER_HIGH && drop_percent >= MIN_VOLUME_DROP {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "VOLUME_DROP_HIT", &format!("🔥 Volume spike drop {:.1}% with {:.1}x volume", 
                    drop_percent, vol_24h / liquidity_usd));
            }
            return Some((
                drop_percent, 
                format!("volume spike drop {:.1}% (vol: {:.1}x liq)", 
                    drop_percent, vol_24h / liquidity_usd
                )
            ));
        }
    }
    
    // Strategy 9: Extended range tokens with special requirements (NEW)
    if liquidity_usd < TARGET_LIQUIDITY_MIN || liquidity_usd > TARGET_LIQUIDITY_MAX {
        // For small tokens ($1k-$5k): require bigger drops (15%+)
        if liquidity_usd < TARGET_LIQUIDITY_MIN && drop_percent >= SMALL_TOKEN_MIN_DROP {
            if is_debug_entry_enabled() {
                log(LogTag::Entry, "SMALL_TOKEN_BIG_DROP", &format!("💎 Small token big drop {:.1}% for ${:.0}", 
                    drop_percent, liquidity_usd));
            }
            return Some((
                drop_percent, 
                format!("small token big drop {:.1}% (${:.0})", 
                    drop_percent, liquidity_usd
                )
            ));
        }
        
        // For large tokens ($1M-$10M): require moderate drops (5%+) with volume
        if liquidity_usd > TARGET_LIQUIDITY_MAX && drop_percent >= LARGE_TOKEN_MIN_DROP {
            if let Some(vol_24h) = volume_24h {
                if vol_24h >= liquidity_usd * VOLUME_MULTIPLIER_LARGE {
                    if is_debug_entry_enabled() {
                        log(LogTag::Entry, "LARGE_TOKEN_VOLUME_DROP", &format!("🚀 Large token drop {:.1}% with volume ${:.0}k", 
                            drop_percent, vol_24h / THOUSAND_DIVISOR));
                    }
                    return Some((
                        drop_percent, 
                        format!("large token drop {:.1}% (${:.0}M, vol: ${:.0}k)", 
                            drop_percent, liquidity_usd / MILLION_DIVISOR, vol_24h / THOUSAND_DIVISOR
                        )
                    ));
                }
            }
        }
    }
    
    // Strategy 10: Instant drop detection (5s) with extra near-top guard
    let instant_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= INSTANT_DROP_TIME_WINDOW_SEC)
        .cloned()
        .collect();
    
    if instant_recent.len() >= 1 {
        let instant_high = instant_recent.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
        
        if instant_high > 0.0 && instant_high.is_finite() {
            let instant_drop = ((instant_high - current_price) / instant_high) * 100.0;
            
            // Instant drop threshold uses 0.5x minimum to avoid tiny dips at peaks
            let instant_threshold = (effective_min_drop * 0.5).max(0.6);
            // Additional guard: must also be > 1m near-top minimum
            let one_min_high = instant_recent.iter().map(|(_, p)| *p).fold(0.0f64, |a, b| a.max(b));
            let one_min_drop_from_high = if one_min_high > 0.0 { ((one_min_high - current_price) / one_min_high) * 100.0 } else { 0.0 };
            
            if instant_drop >= instant_threshold && instant_drop <= max_drop_threshold && one_min_drop_from_high >= NEAR_TOP_1M_MIN {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "INSTANT_DROP_HIT", &format!("⚡⚡ Instant drop {:.1}% ≥ {:.1}% (1m drop_from_high {:.1}% ≥ {:.1}%)", 
                        instant_drop, instant_threshold, one_min_drop_from_high, NEAR_TOP_1M_MIN));
                }
                return Some((
                    instant_drop, 
                    format!("instant drop {:.1}% (${:.0}k: ≥{:.1}%)", 
                        instant_drop, liquidity_usd / THOUSAND_DIVISOR, instant_threshold
                    )
                ));
            }
        }
    }

    // Strategy 13: Moving-average deviation (current below short MA by enough margin)
    {
        const MA_WINDOW_SEC: i64 = 60; // last 60 seconds
        let now3 = Utc::now();
        let ma_recent: Vec<f64> = price_history
            .iter()
            .filter(|(ts, _)| (now3 - *ts).num_seconds() <= MA_WINDOW_SEC)
            .map(|(_, p)| *p)
            .collect();
        if ma_recent.len() >= 3 {
            let ma = ma_recent.iter().sum::<f64>() / (ma_recent.len() as f64);
            if ma > 0.0 && ma.is_finite() {
                let ma_dev = ((ma - current_price) / ma) * 100.0; // how far below MA
                // Liquidity-aware MA thresholds
                let ma_threshold = if liquidity_usd >= 200_000.0 { effective_min_drop * 0.5 } else { effective_min_drop * 0.7 };
                if ma_dev >= ma_threshold && ma_dev <= max_drop_threshold {
                    if is_debug_entry_enabled() {
                        log(LogTag::Entry, "MA_DEVIATION_HIT", &format!("📉 MA deviation {:.1}% ≥ {:.1}% (MA {:.12})", ma_dev, ma_threshold, ma));
                    }
                    return Some((ma_dev.max(drop_percent), format!("MA deviation {:.1}% (≥{:.1}%)", ma_dev, ma_threshold)));
                }
            }
        }
    }
    
    // Strategy 11: Micro drops for mega liquidity tokens (NEW)
    if liquidity_usd >= 5_000_000.0 && drop_percent >= MICRO_DROP_THRESHOLD {
        if is_debug_entry_enabled() {
            log(LogTag::Entry, "MICRO_DROP_MEGA_LIQ", &format!("💎 Micro drop {:.1}% for mega liquidity ${:.0}M", 
                drop_percent, liquidity_usd / MILLION_DIVISOR));
        }
        return Some((
            drop_percent, 
            format!("micro drop mega-liq {:.1}% (${:.0}M)", 
                drop_percent, liquidity_usd / MILLION_DIVISOR
            )
        ));
    }
    
    // Strategy 12: Extended time window analysis (NEW - 10 minutes)
    let extended_recent: Vec<(chrono::DateTime<chrono::Utc>, f64)> = price_history
        .iter()
        .filter(|(timestamp, _)| (now - *timestamp).num_seconds() <= EXTENDED_DROP_TIME_WINDOW_SEC)
        .cloned()
        .collect();
    
    if extended_recent.len() >= 8 {
        let extended_high = extended_recent.iter().map(|(_, price)| *price).fold(0.0f64, |a, b| a.max(b));
        
        if extended_high > 0.0 && extended_high.is_finite() {
            let extended_drop = ((extended_high - current_price) / extended_high) * 100.0;
            
            // Extended threshold is very low (0.4x minimum) to catch slow bleeds
            let extended_threshold = min_drop_threshold * 0.4;
            
            if extended_drop >= extended_threshold && extended_drop <= max_drop_threshold && liquidity_usd >= 5_000.0 {
                if is_debug_entry_enabled() {
                    log(LogTag::Entry, "EXTENDED_DROP_HIT", &format!("📉 Extended drop {:.1}% ≥ {:.1}% threshold", 
                        extended_drop, extended_threshold));
                }
                return Some((
                    extended_drop, 
                    format!("extended drop {:.1}% (${:.0}k: ≥{:.1}%)", 
                        extended_drop, liquidity_usd / THOUSAND_DIVISOR, extended_threshold
                    )
                ));
            }
        }
    }
    
    if is_debug_entry_enabled() {
        log(LogTag::Entry, "NO_DROP_SIGNAL", &format!("❌ No drop signals: {:.1}% (need {:.1}%-{:.1}%)", 
            drop_percent, effective_min_drop, max_drop_threshold));
        
        // Final debug: Entry criteria summary for expanded analysis
        let criteria_summary = format!("Liquidity: ${:.0} (target: ${:.0}k-${:.0}k), Drop: {:.1}% (min: {:.1}%), Age: {:.1}min (max: {:.1}min)", 
            liquidity_usd, TARGET_LIQUIDITY_MIN / THOUSAND_DIVISOR, TARGET_LIQUIDITY_MAX / THOUSAND_DIVISOR,
            drop_percent, DROP_PERCENT_MIN, data_age_minutes, MAX_DATA_AGE_MINUTES as f64);
        log(LogTag::Entry, "ENTRY_ANALYSIS", &criteria_summary);
    }
    
    None
}

// Enhanced entry decision with liquidity-based confidence scoring merged into should_buy
