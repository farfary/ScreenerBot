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

// FAST TRADING CONFIGURATION
const MAX_DATA_AGE_MINUTES: i64 = 2; // Reject any data older than 2 minutes
const FAST_ENTRY_MODE: bool = true; // Enable simplified scoring for speed

/// Helper function to get rugcheck score for a token
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

/// Pool-based entry decision function with -10% drop detection
/// Returns true if the token should be bought based on pool price movement
pub async fn should_buy(token: &Token) -> bool {
    // 0. ABSOLUTE FIRST: Check blacklist and exclusion status
    if is_token_excluded_from_trading(&token.mint) {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "BLACKLIST_REJECT",
                &format!("❌ {} rejected: Token is blacklisted or excluded", token.symbol)
            );
        }
        return false;
    }

    // Get pool service for real-time price data
    let pool_service = get_pool_service();

    // Check if pool price is available for this token
    if !pool_service.check_token_availability(&token.mint).await {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "POOL_UNAVAILABLE",
                &format!("❌ {} rejected: No pool price available", token.symbol)
            );
        }
        return false;
    }

    // Get current pool price with data age validation
    let (current_pool_price, pool_data_age) = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => {
                    // Check data freshness - pool data should be recent
                    let data_age_minutes = (Utc::now() - pool_result.calculated_at).num_minutes();
                    
                    // PRIORITY: Pool price data freshness validation
                    if data_age_minutes > MAX_DATA_AGE_MINUTES {
                        if is_debug_entry_enabled() {
                            log(
                                LogTag::Entry,
                                "STALE_POOL_DATA",
                                &format!("❌ {} rejected: Pool data too old ({} min > {} min limit)", 
                                    token.symbol, data_age_minutes, MAX_DATA_AGE_MINUTES)
                            );
                        }
                        return false;
                    }
                    
                    (price, data_age_minutes)
                },
                _ => {
                    if is_debug_entry_enabled() {
                        log(
                            LogTag::Entry,
                            "INVALID_PRICE",
                            &format!("❌ {} rejected: Invalid pool price", token.symbol)
                        );
                    }
                    return false;
                }
            }
        }
        None => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "PRICE_CALC_FAILED",
                    &format!("❌ {} rejected: Pool price calculation failed", token.symbol)
                );
            }
            return false;
        }
    };

    // Log pool price priority success
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "POOL_PRICE_OK",
            &format!("✅ {} pool price: {:.12} SOL (age: {}min)", 
                token.symbol, current_pool_price, pool_data_age)
        );
    }

    // RL Learning Integration - OPTIONAL for fast trading (not blocking)
    // Only use RL as advisory when model is ready, never block entries
    let rl_advisory_score = {
        use crate::rl_learning::get_trading_learner;
        let rl_learner = get_trading_learner();
        
        // Only use RL if model is ready AND we have 100+ trades
        if rl_learner.is_model_ready() && rl_learner.get_record_count() >= 100 {
            use crate::rl_learning::get_simple_entry_score;
            get_simple_entry_score(
                &token.mint,
                current_pool_price,
                token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(1000.0),
                token.volume
                    .as_ref()
                    .and_then(|v| v.h24)
                    .unwrap_or(50000.0),
                token.market_cap,
                get_rugcheck_score_for_token(&token.mint).await
            ).await
        } else {
            // No RL blocking - use neutral score for fast trading
            0.5  // Neutral advisory score
        }
    };

    // RL is ADVISORY ONLY - log but don't block entries for fast trading
    if is_debug_entry_enabled() {
        let rl_learner = get_trading_learner();
        if rl_learner.is_model_ready() && rl_learner.get_record_count() >= 100 {
            log(
                LogTag::Entry,
                "RL_ADVISORY",
                &format!(
                    "🤖 {} RL advisory: {:.1}% (not blocking - fast trading mode)",
                    token.symbol,
                    rl_advisory_score * 100.0
                )
            );
        } else {
            log(
                LogTag::Entry,
                "RL_LEARNING",
                &format!(
                    "📚 {} RL learning mode: {}/100 trades (not blocking entries)",
                    token.symbol,
                    rl_learner.get_record_count()
                )
            );
        }
    }

    // Get recent price history for advanced entry analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    // FAST TRADING MODE: Simplified entry decision for speed
    if FAST_ENTRY_MODE {
        let fast_entry_decision = analyze_fast_entry_signals(
            current_pool_price,
            &price_history,
            &token,
            pool_data_age
        ).await;

        if let Some(reason) = fast_entry_decision {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "FAST_ENTRY_ACCEPT",
                    &format!(
                        "⚡ {} FAST ENTRY: {} (price: {:.12} SOL, history: {} points)",
                        token.symbol,
                        reason,
                        current_pool_price,
                        price_history.len()
                    )
                );
            }
            return true;
        } else {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "FAST_NO_SIGNAL",
                    &format!(
                        "❌ {} fast entry rejected: No valid signal (price: {:.12} SOL, history: {} points)",
                        token.symbol,
                        current_pool_price,
                        price_history.len()
                    )
                );
            }
            return false;
        }
    }

    // Standard entry decision (fallback for detailed analysis)
    let entry_decision = analyze_entry_signals(
        current_pool_price,
        &price_history,
        &token
    ).await;

    if let Some(reason) = entry_decision {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "ENTRY_ACCEPT",
                &format!(
                    "✅ {} accepted: {} (price: {:.12} SOL, history: {} points)",
                    token.symbol,
                    reason,
                    current_pool_price,
                    price_history.len()
                )
            );
        }
        return true;
    } else {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "NO_SIGNAL",
                &format!(
                    "❌ {} rejected: No valid entry signal (price: {:.12} SOL, history: {} points)",
                    token.symbol,
                    current_pool_price,
                    price_history.len()
                )
            );
        }
        return false;
    }
}

/// Get profit target range based on pool liquidity
/// Returns (min_profit%, max_profit%)
pub async fn get_profit_target(token: &Token) -> (f64, f64) {
    // Get pool service for real-time liquidity data
    let pool_service = get_pool_service();

    // Try to get current pool data for accurate liquidity
    let liquidity_usd = if
        let Some(pool_result) = pool_service.get_pool_price(&token.mint, None).await
    {
        pool_result.liquidity_usd
    } else {
        // Fallback to API liquidity data
        token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0)
    };

    // Simple tiered profit targets based on liquidity
    match liquidity_usd {
        x if x >= 1_000_000.0 => (5.0, 15.0), // Large tokens: 5-15%
        x if x >= 100_000.0 => (10.0, 30.0), // Medium tokens: 10-30%
        x if x >= 10_000.0 => (20.0, 50.0), // Small tokens: 20-50%
        _ => (30.0, 100.0), // Micro tokens: 30-100%
    }
}

/// Fast entry signal analysis optimized for sub-minute trading
/// Returns Some(reason) if entry is recommended, None if rejected
async fn analyze_fast_entry_signals(
    current_price: f64,
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    token: &Token,
    data_age_minutes: i64
) -> Option<String> {
    use chrono::Utc;

    // ULTRA-FAST: Immediate entry for strong signals (no complex analysis)
    
    // Strategy 1: Ultra-fresh data with momentum
    if data_age_minutes == 0 && price_history.is_empty() {
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        // Fast entry for tokens with decent liquidity
        if liquidity_usd >= 5_000.0 {
            return Some(format!("Ultra-fresh token with liquidity (${:.0})", liquidity_usd));
        }
        
        // Even micro-caps if volume indicates activity
        if let Some(volume) = &token.volume {
            if let Some(h24) = volume.h24 {
                if h24 >= liquidity_usd * 0.5 {  // Volume >= 50% of liquidity
                    return Some(format!("Micro-cap with high volume ratio ({:.0}%)", 
                        (h24 / liquidity_usd) * 100.0));
                }
            }
        }
    }

    // Strategy 2: Simple drop detection (faster calculation)
    if price_history.len() >= 2 {
        let recent_prices: Vec<f64> = price_history
            .iter()
            .rev()
            .take(3)  // Only look at last 3 points for speed
            .map(|(_, price)| *price)
            .collect();

        if !recent_prices.is_empty() {
            let recent_high = recent_prices.iter().fold(0.0f64, |a, &b| a.max(b));
            
            if recent_high > 0.0 && recent_high.is_finite() {
                let drop_from_high = ((recent_high - current_price) / recent_high) * 100.0;
                
                // Fast drop detection with lower thresholds for speed
                if drop_from_high >= 8.0 {  // 8% drop (was 10%)
                    return Some(format!("Fast drop: -{:.1}%", drop_from_high));
                }
                
                // Volume-supported smaller drops
                if drop_from_high >= 4.0 {
                    if let Some(volume) = &token.volume {
                        if let Some(h24) = volume.h24 {
                            if h24 >= 20_000.0 {  // Volume threshold
                                return Some(format!("Volume-supported drop: -{:.1}% (vol: ${:.0})", 
                                    drop_from_high, h24));
                            }
                        }
                    }
                }
            }
        }
    }

    // Strategy 3: New token with momentum indicators
    if price_history.len() == 1 {
        let age_minutes = (Utc::now() - price_history[0].0).num_minutes();
        
        // Very recent price action
        if age_minutes <= 2 {
            return Some("Fresh momentum entry".to_string());
        }
    }

    // Strategy 4: Volatility breakout (simplified)
    if price_history.len() >= 3 {
        let recent_prices: Vec<f64> = price_history
            .iter()
            .rev()
            .take(3)
            .map(|(_, price)| *price)
            .collect();

        if recent_prices.len() >= 3 {
            let price_range = recent_prices.iter().fold(0.0f64, |a, &b| a.max(b)) - 
                             recent_prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            if price_range > 0.0 && price_range.is_finite() {
                let volatility = (price_range / current_price) * 100.0;
                
                // High volatility = opportunity
                if volatility >= 5.0 {
                    return Some(format!("Volatility breakout: {:.1}%", volatility));
                }
            }
        }
    }

    // No fast signal found
    None
}

/// Advanced entry signal analysis with multiple strategies
/// Returns Some(reason) if entry is recommended, None if rejected
async fn analyze_entry_signals(
    current_price: f64,
    price_history: &[(chrono::DateTime<chrono::Utc>, f64)],
    token: &Token
) -> Option<String> {
    use chrono::Utc;

    // Strategy 1: Immediate entry for new tokens (no history required)
    if price_history.is_empty() {
        // For new tokens, use liquidity-based entry criteria
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        if liquidity_usd >= 50_000.0 {
            return Some(format!("New token with strong liquidity (${:.0})", liquidity_usd));
        } else {
            return None; // Skip very low liquidity new tokens
        }
    }

    // Strategy 2: Single data point - momentum-based entry
    if price_history.len() == 1 {
        let age_minutes = (Utc::now() - price_history[0].0).num_minutes();

        // Recent price action within 5 minutes - early entry opportunity
        if age_minutes <= 5 {
            return Some("Early momentum entry (fresh price data)".to_string());
        } else {
            return None; // Old single data point, wait for more data
        }
    }

    // Strategy 3: Multi-point analysis (2+ data points)

    // Find recent high (last 5 points or all available if less)
    let analysis_window = std::cmp::min(5, price_history.len());
    let recent_prices: Vec<f64> = price_history
        .iter()
        .rev()
        .take(analysis_window)
        .map(|(_, price)| *price)
        .collect();

    let recent_high = recent_prices.iter().fold(0.0f64, |a, &b| a.max(b));
    let recent_low = if recent_prices.is_empty() {
        f64::INFINITY
    } else {
        recent_prices.iter().fold(f64::INFINITY, |a, &b| a.min(b))
    };

    // Validate fold results
    if recent_high <= 0.0 || !recent_high.is_finite() {
        return None;
    }
    
    // Check for empty data causing invalid low value
    if recent_low == f64::INFINITY || !recent_low.is_finite() {
        return None;
    }

    // Calculate key metrics
    let drop_from_high = ((recent_high - current_price) / recent_high) * 100.0;
    let rise_from_low = if recent_low > 0.0 && recent_low.is_finite() {
        let result = ((current_price - recent_low) / recent_low) * 100.0;
        if result.is_finite() { result } else { 0.0 }
    } else {
        0.0
    };

    // Validate that calculated percentages are finite
    if !drop_from_high.is_finite() {
        return None;
    }

    // Strategy 3a: Classic -10% drop detection (enhanced)
    if drop_from_high >= 10.0 {
        return Some(format!("-{:.1}% drop from recent high", drop_from_high));
    }

    // Strategy 3b: Moderate drop with volume consideration (5-10% drop)
    if drop_from_high >= 5.0 && drop_from_high < 10.0 {
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        // Lower threshold for high liquidity tokens
        if liquidity_usd >= 500_000.0 {
            return Some(
                format!("-{:.1}% drop (high liquidity: ${:.0})", drop_from_high, liquidity_usd)
            );
        }
    }

    // Strategy 3c: Bounce detection (recovering from recent low)
    if recent_prices.len() >= 3 && rise_from_low >= 5.0 && drop_from_high <= 5.0 {
        return Some(format!("+{:.1}% bounce from recent low", rise_from_low));
    }

    // Strategy 3d: Trend analysis for longer history
    if price_history.len() >= 5 {
        let trend_analysis = analyze_price_trend(&recent_prices);

        match trend_analysis {
            TrendSignal::Oversold => {
                return Some("Oversold reversal signal".to_string());
            }
            TrendSignal::SupportBounce => {
                return Some("Support level bounce".to_string());
            }
            TrendSignal::VolatilityBreakout => {
                return Some("Volatility breakout signal".to_string());
            }
            TrendSignal::NoSignal => {
                // Continue to next strategy
            }
        }
    }

    // Strategy 4: Time-based entry for trending tokens
    if price_history.len() >= 3 {
        let time_since_last = (Utc::now() - price_history.last().unwrap().0).num_minutes();

        // If we haven't seen price updates recently but token is still active
        if time_since_last >= 2 && time_since_last <= 10 {
            let avg_price = recent_prices.iter().sum::<f64>() / (recent_prices.len() as f64);
            
            // Validate average price calculation
            if !avg_price.is_finite() || avg_price <= 0.0 {
                return None;
            }
            
            let price_deviation = ((current_price - avg_price).abs() / avg_price) * 100.0;
            
            // Validate deviation calculation
            if !price_deviation.is_finite() {
                return None;
            }

            // Significant deviation from recent average
            if price_deviation >= 3.0 {
                return Some(format!("{:.1}% deviation from recent average", price_deviation));
            }
        }
    }

    // No entry signal found
    None
}

/// Trend analysis for advanced entry signals
#[derive(Debug)]
enum TrendSignal {
    Oversold,
    SupportBounce,
    VolatilityBreakout,
    NoSignal,
}

/// Analyze price trend patterns
fn analyze_price_trend(prices: &[f64]) -> TrendSignal {
    if prices.len() < 5 {
        return TrendSignal::NoSignal;
    }

    // Calculate moving averages with validation
    let short_ma = prices.iter().rev().take(3).sum::<f64>() / 3.0;
    let long_ma = prices.iter().sum::<f64>() / (prices.len() as f64);

    // Validate moving averages
    if !short_ma.is_finite() || !long_ma.is_finite() {
        return TrendSignal::NoSignal;
    }

    let current_price = prices[prices.len() - 1];
    
    // Validate current price
    if !current_price.is_finite() || current_price <= 0.0 {
        return TrendSignal::NoSignal;
    }
    
    let max_price = if prices.is_empty() {
        0.0
    } else {
        prices.iter().fold(0.0f64, |a, &b| a.max(b))
    };
    let min_price = if prices.is_empty() {
        f64::INFINITY  
    } else {
        prices.iter().fold(f64::INFINITY, |a, &b| a.min(b))
    };

    // Validate min/max calculations
    if !max_price.is_finite() || max_price <= 0.0 || 
       !min_price.is_finite() || min_price == f64::INFINITY {
        return TrendSignal::NoSignal;
    }

    // Oversold condition: price near bottom of range with upward short MA
    let range = max_price - min_price;
    let position_in_range = if range > 0.0 && range.is_finite() { 
        let result = (current_price - min_price) / range;
        if result.is_finite() && result >= 0.0 && result <= 1.0 { result } else { 0.5 }
    } else { 
        0.5 
    };

    if position_in_range <= 0.3 && short_ma > long_ma {
        return TrendSignal::Oversold;
    }

    // Support bounce: price above recent low with momentum
    if current_price > min_price * 1.02 && short_ma > current_price {
        return TrendSignal::SupportBounce;
    }

    // Volatility breakout: significant price movement with volume
    let volatility = if long_ma > 0.0 && long_ma.is_finite() {
        range / long_ma
    } else {
        0.0 // Avoid division by zero or invalid long_ma
    };
    if volatility >= 0.05 && position_in_range >= 0.4 && position_in_range <= 0.6 {
        return TrendSignal::VolatilityBreakout;
    }

    TrendSignal::NoSignal
}

/// Get dynamic entry threshold based on market conditions and token characteristics
pub fn get_entry_threshold(token: &Token) -> f64 {
    let mut base_threshold: f64 = 10.0; // Base -10% drop requirement

    // Adjust based on token age
    if let Some(created_at) = token.created_at {
        let age_hours = (Utc::now() - created_at).num_hours();

        match age_hours {
            0..=6 => {
                base_threshold *= 0.5;
            } // Very new: -5% threshold
            7..=24 => {
                base_threshold *= 0.7;
            } // New: -7% threshold
            25..=168 => {
                base_threshold *= 0.8;
            } // Week old: -8% threshold
            _ => {} // Keep base -10% for older tokens
        }
    }

    // Adjust based on liquidity
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    match liquidity_usd {
        x if x >= 1_000_000.0 => {
            base_threshold *= 0.6;
        } // High liquidity: -6% threshold
        x if x >= 500_000.0 => {
            base_threshold *= 0.7;
        } // Good liquidity: -7% threshold
        x if x >= 100_000.0 => {
            base_threshold *= 0.8;
        } // Medium liquidity: -8% threshold
        x if x < 50_000.0 => {
            base_threshold *= 1.2;
        } // Low liquidity: -12% threshold
        _ => {} // Keep adjusted threshold for normal liquidity
    }

    // Cap the threshold between 3% and 15%
    base_threshold.max(3.0).min(15.0)
}

/// Enhanced entry decision with confidence scoring and reinforcement learning
/// Returns (should_enter, confidence_score, reason)
pub async fn should_buy_with_confidence(token: &Token) -> (bool, f64, String) {
    // Check blacklist first
    if is_token_excluded_from_trading(&token.mint) {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "BLACKLIST_REJECT",
                &format!("❌ {} rejected: Token blacklisted or excluded", token.symbol)
            );
        }
        return (false, 0.0, "Token blacklisted or excluded".to_string());
    }

    let pool_service = get_pool_service();

    // Check pool availability
    if !pool_service.check_token_availability(&token.mint).await {
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "POOL_UNAVAILABLE",
                &format!("❌ {} rejected: No pool available", token.symbol)
            );
        }
        return (false, 0.0, "No pool price available".to_string());
    }

    // Get current pool price
    let current_pool_price = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => price,
                _ => {
                    if is_debug_entry_enabled() {
                        log(
                            LogTag::Entry,
                            "INVALID_PRICE",
                            &format!("❌ {} rejected: Invalid pool price", token.symbol)
                        );
                    }
                    return (false, 0.0, "Invalid pool price".to_string());
                }
            }
        }
        None => {
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "PRICE_CALC_FAILED",
                    &format!("❌ {} rejected: Pool price calculation failed", token.symbol)
                );
            }
            return (false, 0.0, "Pool price calculation failed".to_string());
        }
    };

    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "CONFIDENCE_START",
            &format!(
                "🎯 Starting confidence analysis for {} (price: {:.12} SOL, history: {} points)",
                token.symbol,
                current_pool_price,
                price_history.len()
            )
        );
    }

    // Calculate confidence score based on multiple factors
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();

    // Factor 1: Price history analysis (0-35 points)
    if
        let Some(reason) = analyze_entry_signals(
            current_pool_price,
            &price_history,
            token
        ).await
    {
        reasons.push(reason.clone());

        // Score based on signal strength
        if reason.contains("drop from recent high") {
            confidence += 35.0; // Strong signal
        } else if reason.contains("bounce") {
            confidence += 30.0; // Good signal
        } else if reason.contains("liquidity") {
            confidence += 25.0; // Moderate signal
        } else {
            confidence += 20.0; // Weak signal
        }
    }

    // Factor 2: Liquidity score (0-20 points)
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);
    let liquidity_score = match liquidity_usd {
        x if x >= 1_000_000.0 => 20.0,
        x if x >= 500_000.0 => 16.0,
        x if x >= 100_000.0 => 12.0,
        x if x >= 50_000.0 => 8.0,
        _ => 0.0,
    };
    confidence += liquidity_score;
    if liquidity_score > 0.0 {
        reasons.push(format!("Liquidity: ${:.0}", liquidity_usd));
        if is_debug_entry_enabled() {
            log(
                LogTag::Entry,
                "FACTOR_2",
                &format!(
                    "💧 {} Liquidity: ${:.0} (+{:.1} pts)",
                    token.symbol,
                    liquidity_usd,
                    liquidity_score
                )
            );
        }
    }

    // Factor 3: Token age factor (0-10 points)
    if let Some(created_at) = token.created_at {
        let age_hours = (Utc::now() - created_at).num_hours();
        let age_score = match age_hours {
            1..=24 => 10.0, // Sweet spot for new tokens
            25..=168 => 7.0, // Still good
            169..=720 => 4.0, // Older but stable
            _ => 0.0,
        };
        confidence += age_score;
        if age_score > 0.0 {
            reasons.push(format!("Age: {}h", age_hours));
            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "FACTOR_3",
                    &format!("⏰ {} Age: {}h (+{:.1} pts)", token.symbol, age_hours, age_score)
                );
            }
        }
    }

    // Factor 4: Price data quality (0-8 points)
    let data_quality = match price_history.len() {
        0 => 2.0, // New token, limited data
        1 => 3.0, // Minimal data
        2..=3 => 5.0, // Some data
        4..=5 => 7.0, // Good data
        _ => 8.0, // Excellent data
    };
    confidence += data_quality;
    reasons.push(format!("Data points: {}", price_history.len()));
    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "FACTOR_4",
            &format!(
                "📊 {} Data quality: {} points (+{:.1} pts)",
                token.symbol,
                price_history.len(),
                data_quality
            )
        );
    }

    // Factor 5: Volume/Activity bonus (0-7 points)
    if let Some(volume) = &token.volume {
        if let Some(h24) = volume.h24 {
            let volume_score = if h24 >= 100_000.0 {
                7.0
            } else if h24 >= 50_000.0 {
                5.0
            } else if h24 >= 10_000.0 {
                3.0
            } else {
                0.0
            };
            confidence += volume_score;
            if volume_score > 0.0 {
                reasons.push(format!("24h volume: ${:.0}", h24));
                if is_debug_entry_enabled() {
                    log(
                        LogTag::Entry,
                        "FACTOR_5",
                        &format!(
                            "📈 {} Volume: ${:.0} (+{:.1} pts)",
                            token.symbol,
                            h24,
                            volume_score
                        )
                    );
                }
            }
        }
    }

    // Factor 6: Reinforcement Learning Score (0-20 points) - NEW!
    let rl_learner = get_trading_learner();
    if rl_learner.is_model_ready() {
        // Collect market features for RL prediction
        if
            let Some(
                (
                    price_change_5min,
                    price_change_10min,
                    price_change_30min,
                    pool_price,
                    price_drop_detected,
                    _,
                ),
            ) = collect_market_features(
                &token.mint,
                &token.symbol,
                current_pool_price,
                liquidity_usd,
                token.volume
                    .as_ref()
                    .and_then(|v| v.h24)
                    .unwrap_or(0.0),
                token.market_cap.map(|mc| mc as f64),
                get_rugcheck_score_for_token(&token.mint).await
            ).await
        {
            // Get RL learning score (0.0 to 1.0)
            let rl_score = rl_learner.get_learning_score(
                &token.mint,
                current_pool_price,
                (price_change_5min, price_change_10min, price_change_30min),
                liquidity_usd,
                token.volume
                    .as_ref()
                    .and_then(|v| v.h24)
                    .unwrap_or(0.0),
                token.market_cap.map(|mc| mc as f64),
                get_rugcheck_score_for_token(&token.mint).await,
                pool_price,
                price_drop_detected,
                confidence / 100.0 // Pass current confidence as baseline
            ).await;

            // Convert RL score to points (0-20)
            let rl_points = rl_score * 20.0;
            confidence += rl_points;

            if rl_points > 10.0 {
                reasons.push(format!("RL-AI: {:.1}%", rl_score * 100.0));
            }

            if is_debug_entry_enabled() {
                log(
                    LogTag::Entry,
                    "FACTOR_6",
                    &format!(
                        "🤖 {} RL Score: {:.1}% (+{:.1} pts) - Features: price_changes({:.2}%, {:.2}%, {:.2}%), liquidity: ${:.0}",
                        token.symbol,
                        rl_score * 100.0,
                        rl_points,
                        price_change_5min,
                        price_change_10min,
                        price_change_30min,
                        liquidity_usd
                    )
                );
            }
        }
    } else {
        // Model not ready, log learning progress
        let record_count = rl_learner.get_record_count();
        if is_debug_entry_enabled() && record_count > 0 {
            log(
                LogTag::Entry,
                "RL_LEARNING",
                &format!("🤖 {} RL model training: {}/50 records", token.symbol, record_count)
            );
        }
    }

    // Normalize confidence to 0-100 scale and validate result
    confidence = confidence.min(100.0).max(0.0);
    
    // Validate confidence is finite and within bounds
    if !confidence.is_finite() {
        confidence = 0.0;
    }

    // Entry threshold: require at least 60% confidence
    let should_enter = confidence >= 60.0;
    let reason_str = reasons.join(", ");

    if is_debug_entry_enabled() {
        log(
            LogTag::Entry,
            "FINAL_DECISION",
            &format!(
                "🎯 {} Final confidence: {:.1}% -> {} (factors: {})",
                token.symbol,
                confidence,
                if should_enter {
                    "✅ ENTER"
                } else {
                    "❌ SKIP"
                },
                reason_str
            )
        );
    }

    (should_enter, confidence, reason_str)
}
