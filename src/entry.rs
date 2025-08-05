/// Pool-based entry logic for ScreenerBot
///
/// This module provides pool price-based entry decisions with -10% drop detection.
/// Uses real-time blockchain pool data for trading decisions while API data is used only for validation.

use crate::tokens::Token;
use crate::tokens::pool::get_pool_service;
use crate::tokens::is_token_excluded_from_trading;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_trader_enabled;
use chrono::{ DateTime, Utc };

/// Pool-based entry decision function with -10% drop detection
/// Returns true if the token should be bought based on pool price movement
pub async fn should_buy(token: &Token) -> bool {
    // 0. ABSOLUTE FIRST: Check blacklist and exclusion status
    if is_token_excluded_from_trading(&token.mint) {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: Token is blacklisted or excluded", token.symbol)
            );
        }
        return false;
    }

    // Get pool service for real-time price data
    let pool_service = get_pool_service();

    // Check if pool price is available for this token
    if !pool_service.check_token_availability(&token.mint).await {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!("❌ {} rejected: No pool price available", token.symbol)
            );
        }
        return false;
    }

    // Get current pool price
    let current_pool_price = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => price,
                _ => {
                    if is_debug_trader_enabled() {
                        log(
                            LogTag::Trader,
                            "ENTRY_REJECT",
                            &format!("❌ {} rejected: Invalid pool price", token.symbol)
                        );
                    }
                    return false;
                }
            }
        }
        None => {
            if is_debug_trader_enabled() {
                log(
                    LogTag::Trader,
                    "ENTRY_REJECT",
                    &format!("❌ {} rejected: Pool price calculation failed", token.symbol)
                );
            }
            return false;
        }
    };

    // Get recent price history for advanced entry analysis
    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    // Enhanced entry decision with multiple strategies
    let entry_decision = analyze_entry_signals(
        &token.symbol,
        current_pool_price,
        &price_history,
        &token
    ).await;

    if let Some(reason) = entry_decision {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_ACCEPT",
                &format!(
                    "✅ {} accepted: {} (current: {:.12} SOL)",
                    token.symbol,
                    reason,
                    current_pool_price
                )
            );
        }
        return true;
    } else {
        if is_debug_trader_enabled() {
            log(
                LogTag::Trader,
                "ENTRY_REJECT",
                &format!(
                    "❌ {} rejected: No valid entry signal (current: {:.12} SOL, history: {} points)",
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

/// Advanced entry signal analysis with multiple strategies
/// Returns Some(reason) if entry is recommended, None if rejected
async fn analyze_entry_signals(
    symbol: &str,
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
    let recent_low = recent_prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));

    if recent_high <= 0.0 || !recent_high.is_finite() {
        return None;
    }

    // Calculate key metrics
    let drop_from_high = ((recent_high - current_price) / recent_high) * 100.0;
    let rise_from_low = if recent_low > 0.0 && recent_low.is_finite() {
        ((current_price - recent_low) / recent_low) * 100.0
    } else {
        0.0
    };

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
            let price_deviation = ((current_price - avg_price).abs() / avg_price) * 100.0;

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

    // Calculate moving averages
    let short_ma = prices.iter().rev().take(3).sum::<f64>() / 3.0;
    let long_ma = prices.iter().sum::<f64>() / (prices.len() as f64);

    let current_price = prices[prices.len() - 1];
    let max_price = prices.iter().fold(0.0f64, |a, &b| a.max(b));
    let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));

    // Oversold condition: price near bottom of range with upward short MA
    let range = max_price - min_price;
    let position_in_range = if range > 0.0 { (current_price - min_price) / range } else { 0.5 };

    if position_in_range <= 0.3 && short_ma > long_ma {
        return TrendSignal::Oversold;
    }

    // Support bounce: price above recent low with momentum
    if current_price > min_price * 1.02 && short_ma > current_price {
        return TrendSignal::SupportBounce;
    }

    // Volatility breakout: significant price movement with volume
    let volatility = range / long_ma;
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

/// Enhanced entry decision with confidence scoring
/// Returns (should_enter, confidence_score, reason)
pub async fn should_buy_with_confidence(token: &Token) -> (bool, f64, String) {
    // Check blacklist first
    if is_token_excluded_from_trading(&token.mint) {
        return (false, 0.0, "Token blacklisted or excluded".to_string());
    }

    let pool_service = get_pool_service();

    // Check pool availability
    if !pool_service.check_token_availability(&token.mint).await {
        return (false, 0.0, "No pool price available".to_string());
    }

    // Get current pool price
    let current_pool_price = match pool_service.get_pool_price(&token.mint, None).await {
        Some(pool_result) => {
            match pool_result.price_sol {
                Some(price) if price > 0.0 && price.is_finite() => price,
                _ => {
                    return (false, 0.0, "Invalid pool price".to_string());
                }
            }
        }
        None => {
            return (false, 0.0, "Pool price calculation failed".to_string());
        }
    };

    let price_history = pool_service.get_recent_price_history(&token.mint).await;

    // Calculate confidence score based on multiple factors
    let mut confidence: f64 = 0.0;
    let mut reasons = Vec::new();

    // Factor 1: Price history analysis (0-40 points)
    if
        let Some(reason) = analyze_entry_signals(
            &token.symbol,
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

    // Factor 2: Liquidity score (0-25 points)
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);
    let liquidity_score = match liquidity_usd {
        x if x >= 1_000_000.0 => 25.0,
        x if x >= 500_000.0 => 20.0,
        x if x >= 100_000.0 => 15.0,
        x if x >= 50_000.0 => 10.0,
        _ => 0.0,
    };
    confidence += liquidity_score;
    if liquidity_score > 0.0 {
        reasons.push(format!("Liquidity: ${:.0}", liquidity_usd));
    }

    // Factor 3: Token age factor (0-15 points)
    if let Some(created_at) = token.created_at {
        let age_hours = (Utc::now() - created_at).num_hours();
        let age_score = match age_hours {
            1..=24 => 15.0, // Sweet spot for new tokens
            25..=168 => 10.0, // Still good
            169..=720 => 5.0, // Older but stable
            _ => 0.0,
        };
        confidence += age_score;
        if age_score > 0.0 {
            reasons.push(format!("Age: {}h", age_hours));
        }
    }

    // Factor 4: Price data quality (0-10 points)
    let data_quality = match price_history.len() {
        0 => 2.0, // New token, limited data
        1 => 4.0, // Minimal data
        2..=3 => 6.0, // Some data
        4..=5 => 8.0, // Good data
        _ => 10.0, // Excellent data
    };
    confidence += data_quality;
    reasons.push(format!("Data points: {}", price_history.len()));

    // Factor 5: Volume/Activity bonus (0-10 points)
    if let Some(volume) = &token.volume {
        if let Some(h24) = volume.h24 {
            let volume_score = if h24 >= 100_000.0 {
                10.0
            } else if h24 >= 50_000.0 {
                7.0
            } else if h24 >= 10_000.0 {
                4.0
            } else {
                0.0
            };
            confidence += volume_score;
            if volume_score > 0.0 {
                reasons.push(format!("24h volume: ${:.0}", h24));
            }
        }
    }

    // Normalize confidence to 0-100 scale
    confidence = confidence.min(100.0);

    // Entry threshold: require at least 60% confidence
    let should_enter = confidence >= 60.0;
    let reason_str = reasons.join(", ");

    (should_enter, confidence, reason_str)
}
