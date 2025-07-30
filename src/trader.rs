/// ScreenerBot Trading Engine
///
/// ======================================
/// STOP LOSS PROTECTION SYSTEM
/// ======================================
///
/// This trading bot implements a stop loss at -55% with multiple layers of protection:
///
/// 1. **Stop Loss Threshold**: Set at -55% for controlled risk management
/// 2. **Profit-First Exit Logic**: All sell decisions prioritize profit taking
/// 3. **Smart System Validation**: Advanced profit system enforces stop loss rule
/// 4. **Final Safeguard**: Last-minute validation before any sell execution
/// 5. **Controlled Loss Strategy**: Positions at loss below -55% are sold to limit damage
///
/// This ensures controlled risk management while allowing reasonable loss limits
/// for positions that don't recover within acceptable timeframes.
///
/// MULTI-STRATEGY DIP DETECTION SYSTEM
/// ===================================
///
/// This system implements 5 sophisticated strategies to detect profitable dip entry points:
///
/// 1. **Immediate Drop Detection** (-5% to -30%)
///    - Detects sudden price drops in the immediate timeframe
///    - Adapts thresholds based on token's historical volatility
///    - Higher urgency for larger drops within the range
///
/// 2. **Moving Average Deviation** (-8% to -25%)
///    - Analyzes deviation from 5, 10, and 20-period moving averages
///    - Longer period deviations get higher weight
///    - Identifies oversold conditions relative to recent trends
///
/// 3. **Support Level Bounce** (-10% to -30%)
///    - Detects when price approaches historical support levels
///    - Uses technical analysis to find key price levels
///    - Bonus urgency for proximity to strong support
///
/// 4. **Volume-Weighted Price Dip** (-7% to -20%)
///    - Combines price drops with volume analysis
///    - Higher volume during dips indicates buying interest
///    - Validates dips with market participation data
///
/// 5. **Multi-Timeframe Convergence** (-6% to -18%)
///    - Requires multiple timeframes showing dip signals
///    - Short, medium, and long-term trend analysis
///    - Higher confidence when timeframes converge
///
/// The system calculates urgency scores from 0.0 to 2.0, with multiple strategies
/// providing consensus-based scoring and increased confidence.

// =============================================================================
// TRADING SYSTEM CONFIGURATION CONSTANTS
// =============================================================================

// -----------------------------------------------------------------------------
// Core Trading Parameters
// -----------------------------------------------------------------------------

/// Maximum number of concurrent open positions
pub const MAX_OPEN_POSITIONS: usize = 3;

/// Trade size in SOL for each position
pub const TRADE_SIZE_SOL: f64 = 0.002;

/// Default transaction fee for buy/sell operations
pub const TRANSACTION_FEE_SOL: f64 = 0.000005;

/// Default swap fee (set to 0 for GMGN routing)
pub const SWAP_FEE_PERCENT: f64 = 0.0;

/// Default slippage tolerance for swaps
pub const SLIPPAGE_TOLERANCE_PERCENT: f64 = 5.0;

// -----------------------------------------------------------------------------
// Entry Signal Configuration (Dip Detection)
// -----------------------------------------------------------------------------

/// Minimum price drop percentage to trigger buy signal
pub const MIN_DIP_THRESHOLD_PERCENT: f64 = 5.0;

// -----------------------------------------------------------------------------
// Exit Signal Configuration (Profit Taking)
// -----------------------------------------------------------------------------

/// Profit target percentage for position exits - now duration-based
/// This is the base target for short-term positions (< 2 hours)
pub const PROFIT_TARGET_PERCENT: f64 = 5.0;

// -----------------------------------------------------------------------------
// Position Timing Configuration - Improved for longer holding
// -----------------------------------------------------------------------------

/// Minimum hold time before considering sell (seconds) - reduced for flexibility
pub const MIN_POSITION_HOLD_TIME_SECS: f64 = 30.0;

/// Maximum hold time extended for longer-term profit taking (48 hours)
pub const MAX_POSITION_HOLD_TIME_SECS: f64 = 1.0 * 60.0 * 60.0; // 48 hours

/// Time after which time decay pressure starts - now 2 hours for better patience
pub const TIME_DECAY_START_SECS: f64 = 7200.0; // 2 hours

// -----------------------------------------------------------------------------
// Trading Logic Configuration
// -----------------------------------------------------------------------------
// Monitoring & Display Configuration
// -----------------------------------------------------------------------------

/// Summary display refresh interval (seconds)
pub const SUMMARY_DISPLAY_INTERVAL_SECS: u64 = 5;

/// New entry signals check interval (seconds)
pub const ENTRY_MONITOR_INTERVAL_SECS: u64 = 5;

/// Open positions monitoring interval (seconds)
pub const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;

/// Price history tracking duration (hours)
pub const PRICE_HISTORY_DURATION_HOURS: i64 = 2;

// -----------------------------------------------------------------------------
// Task Timeout Configuration
// -----------------------------------------------------------------------------

/// Semaphore acquire timeout for token processing tasks (seconds)
pub const SEMAPHORE_ACQUIRE_TIMEOUT_SECS: u64 = 120;

/// Individual token check task timeout (seconds)
pub const TOKEN_CHECK_TASK_TIMEOUT_SECS: u64 = 60;

/// Price cache lock acquire timeout (milliseconds)
pub const PRICE_CACHE_LOCK_TIMEOUT_MS: u64 = 2000;

/// Task collection timeout for concurrent operations (seconds)
pub const TASK_COLLECTION_TIMEOUT_SECS: u64 = 120;

// -----------------------------------------------------------------------------
// Wallet Management Configuration
// -----------------------------------------------------------------------------

/// Automatically close Associated Token Accounts after selling tokens
pub const AUTO_CLOSE_ATA_AFTER_SELL: bool = true;

use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::is_debug_trader_enabled;
use crate::tokens::{
    Token,
    update_open_positions_safe,
    get_all_tokens_by_liquidity,
    discover_tokens_once,
    monitor_tokens_once,
    get_token_price_safe,
};
use crate::positions::{
    Position,
    calculate_position_pnl,
    update_position_tracking,
    get_open_positions_count,
    open_position,
    close_position,
    SAVED_POSITIONS,
};
use crate::summary::*;
use crate::utils::*;

use crate::filtering::{ should_buy_token, log_filtering_summary };
use crate::tokens::get_token_rugcheck_data_safe;

// =============================================================================
// IMPORTS AND DEPENDENCIES
// =============================================================================

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use chrono::{ Utc, Duration as ChronoDuration, DateTime };
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use colored::Colorize;

// =============================================================================
// GLOBAL STATE AND STATIC STORAGE
// =============================================================================

/// Static global: price history for each token (mint), stores Vec<(timestamp, price)>
pub static PRICE_HISTORY_24H: Lazy<
    StdArc<StdMutex<HashMap<String, Vec<(DateTime<Utc>, f64)>>>>
> = Lazy::new(|| StdArc::new(StdMutex::new(HashMap::new())));

/// Static global: last known prices for each token
pub static LAST_PRICES: Lazy<StdArc<StdMutex<HashMap<String, f64>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashMap::new()))
});

// =============================================================================
// DEBUG LOGGING CONFIGURATION
// =============================================================================

/// Helper function for conditional debug trader logs
pub fn debug_trader_log(log_type: &str, message: &str) {
    if is_debug_trader_enabled() {
        log(LogTag::Trader, log_type, message);
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR TOKENS MODULE INTEGRATION
// =============================================================================

/// Get tokens using the safe tokens module system
pub async fn get_tokens_from_safe_system() -> Vec<Token> {
    match get_all_tokens_by_liquidity().await {
        Ok(api_tokens) => {
            // Convert ApiToken to Token for compatibility with existing code
            api_tokens
                .into_iter()
                .map(|api_token| api_token.into())
                .collect()
        }
        Err(e) => {
            log(LogTag::Trader, "WARN", &format!("Failed to get tokens from safe system: {}", e));
            Vec::new()
        }
    }
}

/// Update open positions tracking in price service
async fn update_position_tracking_in_service() {
    let open_mints = {
        if let Ok(positions) = SAVED_POSITIONS.lock() {
            positions
                .iter()
                .filter(|pos| pos.exit_price.is_none())
                .map(|pos| pos.mint.clone())
                .collect::<Vec<String>>()
        } else {
            Vec::new()
        }
    };

    if !open_mints.is_empty() {
        update_open_positions_safe(open_mints).await;
        log(LogTag::Trader, "TRACK", "Updated open positions in price service");
    }
}

/// Try to populate tokens database with discovery data if it's empty
async fn ensure_tokens_populated() {
    // Check if we have tokens in the database
    match get_all_tokens_by_liquidity().await {
        Ok(tokens) if tokens.is_empty() => {
            log(LogTag::Trader, "INFO", "Token database is empty, running discovery...");

            // Run manual discovery to populate the database
            if let Err(e) = discover_tokens_once().await {
                log(LogTag::Trader, "WARN", &format!("Failed to run token discovery: {}", e));
            }

            // Run manual monitoring to update prices
            if let Err(e) = monitor_tokens_once().await {
                log(LogTag::Trader, "WARN", &format!("Failed to run token monitoring: {}", e));
            }
        }
        Ok(tokens) => {
            log(LogTag::Trader, "INFO", &format!("Token database has {} tokens", tokens.len()));
        }
        Err(e) => {
            log(LogTag::Trader, "WARN", &format!("Failed to check token database: {}", e));
        }
    }
}

/// Helper function for regular trader logging (always visible)

/// Calculate dynamic liquidity thresholds based on current token watch list
/// Returns (high_threshold, medium_threshold, low_threshold) for liquidity factoring
fn calculate_dynamic_liquidity_thresholds() -> (f64, f64, f64) {
    // Note: Liquidity threshold calculation now uses safe price service
    // For now, use stable fallback values that work well in practice

    // Fallback thresholds based on market analysis
    log(
        LogTag::Trader,
        "WARN",
        "Could not access token database for liquidity threshold calculation, using fallback"
    );
    (100000.0, 50000.0, 10000.0)
}

/// Advanced Multi-Strategy Dip Detection System
/// Uses 5 different strategies to identify dips from -5% to -30%
/// Returns urgency score from 0.0 (don't buy) to 2.0 (buy immediately)
pub fn should_buy(token: &Token, current_price: f64, prev_price: f64) -> f64 {
    debug_trader_log(
        "SHOULD_BUY_START",
        &format!(
            "Checking buy signal for {} ({}): current={:.10}, prev={:.10}",
            token.symbol,
            token.mint,
            current_price,
            prev_price
        )
    );

    // Use centralized filtering system
    if !should_buy_token(token) {
        debug_trader_log(
            "SHOULD_BUY_FILTER_REJECT",
            &format!("Token {} ({}) rejected by filtering system", token.symbol, token.mint)
        );
        return 0.0;
    }

    debug_trader_log(
        "SHOULD_BUY_FILTER_OK",
        &format!("Token {} ({}) passed filtering", token.symbol, token.mint)
    );

    // Additional price validation for dip detection
    if current_price <= 0.0 || prev_price <= 0.0 {
        debug_trader_log(
            "SHOULD_BUY_PRICE_INVALID",
            &format!(
                "Token {} ({}) has invalid prices: current={:.10}, prev={:.10}",
                token.symbol,
                token.mint,
                current_price,
                prev_price
            )
        );
        return 0.0;
    }

    // Calculate price change percentage
    let price_change_percent = ((current_price - prev_price) / prev_price) * 100.0;
    debug_trader_log(
        "SHOULD_BUY_PRICE_CHANGE",
        &format!(
            "Token {} ({}) price change: {:.2}%",
            token.symbol,
            token.mint,
            price_change_percent
        )
    );

    // Run all 5 dip detection strategies
    let dip_signals = run_multi_strategy_dip_detection(token, current_price, prev_price);

    debug_trader_log(
        "SHOULD_BUY_STRATEGIES",
        &format!(
            "Token {} ({}) triggered {} strategies",
            token.symbol,
            token.mint,
            dip_signals.len()
        )
    );

    // If no strategies triggered, no buy signal
    if dip_signals.is_empty() {
        debug_trader_log(
            "SHOULD_BUY_NO_SIGNALS",
            &format!("Token {} ({}) no dip strategies triggered", token.symbol, token.mint)
        );
        return 0.0;
    }

    // Calculate final urgency based on multiple strategy consensus
    let final_urgency = calculate_multi_strategy_urgency(&dip_signals, token, Some(current_price));

    debug_trader_log(
        "SHOULD_BUY_FINAL_URGENCY",
        &format!("Token {} ({}) final urgency: {:.3}", token.symbol, token.mint, final_urgency)
    );

    if final_urgency > 0.0 {
        log(
            LogTag::Trader,
            "MULTI_DIP_SIGNAL",
            &format!(
                "Multi-strategy dip signal for {} ({}): {} strategies triggered, final urgency: {:.2}",
                token.symbol,
                token.mint,
                dip_signals.len(),
                final_urgency
            )
        );
    }

    final_urgency
}

/// Dip detection strategy result
#[derive(Debug, Clone)]
struct DipSignal {
    strategy_name: String,
    urgency: f64,
    drop_percent: f64,
    confidence: f64,
    details: String,
}

/// Runs all 5 dip detection strategies and returns triggered signals
fn run_multi_strategy_dip_detection(
    token: &Token,
    current_price: f64,
    prev_price: f64
) -> Vec<DipSignal> {
    let mut signals = Vec::new();

    // Strategy 1: Immediate Drop Detection (-5% to -30%)
    if let Some(signal) = strategy_immediate_drop_detection(token, current_price, prev_price) {
        signals.push(signal);
    }

    // Strategy 2: Moving Average Deviation (-8% to -25%)
    if let Some(signal) = strategy_moving_average_deviation(token, current_price) {
        signals.push(signal);
    }

    // Strategy 3: Support Level Bounce (-10% to -30%)
    if let Some(signal) = strategy_support_level_bounce(token, current_price) {
        signals.push(signal);
    }

    // Strategy 4: Volume-Weighted Price Dip (-7% to -20%)
    if let Some(signal) = strategy_volume_weighted_dip(token, current_price) {
        signals.push(signal);
    }

    // Strategy 5: Multi-Timeframe Convergence (-6% to -18%)
    if let Some(signal) = strategy_multi_timeframe_convergence(token, current_price) {
        signals.push(signal);
    }

    signals
}

/// Strategy 1: Immediate Drop Detection
/// Detects sudden drops from -5% to -30% with volatility scaling
fn strategy_immediate_drop_detection(
    token: &Token,
    current_price: f64,
    prev_price: f64
) -> Option<DipSignal> {
    let change = (current_price - prev_price) / prev_price;
    let percent_change = change * 100.0;

    debug_trader_log(
        "STRATEGY_IMMEDIATE_DROP",
        &format!(
            "Token {} ({}): price change {:.2}% (current={:.10}, prev={:.10})",
            token.symbol,
            token.mint,
            percent_change,
            current_price,
            prev_price
        )
    );

    // Base threshold: -5% to -30% (inclusive bounds)
    if percent_change > -5.0 || percent_change < -30.0 {
        debug_trader_log(
            "STRATEGY_IMMEDIATE_DROP_RANGE",
            &format!(
                "Token {} ({}): {:.2}% change outside -5% to -30% range",
                token.symbol,
                token.mint,
                percent_change
            )
        );
        return None;
    }

    debug_trader_log(
        "STRATEGY_IMMEDIATE_DROP_IN_RANGE",
        &format!(
            "Token {} ({}): {:.2}% change in valid range, analyzing volatility",
            token.symbol,
            token.mint,
            percent_change
        )
    );

    // Analyze token volatility to adjust thresholds
    let volatility_analysis = analyze_token_volatility_patterns(&token.mint, current_price);

    // Scale thresholds based on token's typical volatility
    let volatility_scale = f64::max(volatility_analysis.volatility_scale, 5.0);
    let adjusted_min_drop = f64::min(-5.0, -volatility_scale * 0.8);
    let adjusted_max_drop = f64::max(-30.0, -volatility_scale * 4.0); // Increased multiplier

    debug_trader_log(
        "STRATEGY_IMMEDIATE_DROP_VOLATILITY",
        &format!(
            "Token {} ({}): volatility_scale={:.1}, adjusted_range=[{:.2}%, {:.2}%], is_in_dip={}",
            token.symbol,
            token.mint,
            volatility_scale,
            adjusted_min_drop,
            adjusted_max_drop,
            volatility_analysis.is_in_dip
        )
    );

    if percent_change <= adjusted_min_drop && percent_change >= adjusted_max_drop {
        // Calculate urgency based on drop magnitude and volatility
        let drop_magnitude = percent_change.abs();
        let base_urgency = (drop_magnitude - 5.0) / 25.0; // 0.0 at -5%, 1.0 at -30%

        // Volatility adjustment - less volatile tokens get higher urgency for same drop
        let volatility_multiplier = 1.0 / (1.0 + volatility_analysis.volatility_score * 0.3); // Reduced penalty

        let urgency = base_urgency * volatility_multiplier;
        let confidence = if volatility_analysis.is_in_dip { 0.9 } else { 0.7 };

        debug_trader_log(
            "STRATEGY_IMMEDIATE_DROP_TRIGGERED",
            &format!(
                "Token {} ({}): SIGNAL TRIGGERED! urgency={:.3}, confidence={:.2}, drop_magnitude={:.2}",
                token.symbol,
                token.mint,
                urgency,
                confidence,
                drop_magnitude
            )
        );

        return Some(DipSignal {
            strategy_name: "ImmediateDrop".to_string(),
            urgency,
            drop_percent: percent_change,
            confidence,
            details: format!("Drop: {:.1}%, Vol Scale: {:.1}%", percent_change, volatility_scale),
        });
    } else {
        debug_trader_log(
            "STRATEGY_IMMEDIATE_DROP_VOLATILITY_FILTER",
            &format!(
                "Token {} ({}): {:.2}% change filtered out by volatility adjustment",
                token.symbol,
                token.mint,
                percent_change
            )
        );
    }

    None
}

/// Strategy 2: Moving Average Deviation
/// Detects when price drops -8% to -25% below various moving averages
fn strategy_moving_average_deviation(token: &Token, current_price: f64) -> Option<DipSignal> {
    if let Ok(price_history) = PRICE_HISTORY_24H.try_lock() {
        if let Some(history) = price_history.get(&token.mint) {
            if history.len() < 10 {
                return None;
            }

            let prices: Vec<f64> = history
                .iter()
                .map(|(_, price)| *price)
                .collect();

            // Calculate multiple moving averages
            let ma_5 = calculate_moving_average(&prices, 5);
            let ma_10 = calculate_moving_average(&prices, 10);
            let ma_20 = calculate_moving_average(&prices, 20);

            let mut best_signal: Option<DipSignal> = None;
            let mut best_urgency = 0.0;

            // Check deviation from each MA
            for (period, ma_price) in [
                (5, ma_5),
                (10, ma_10),
                (20, ma_20),
            ] {
                if let Some(ma) = ma_price {
                    let deviation = ((current_price - ma) / ma) * 100.0;

                    // Look for -8% to -25% deviation
                    if deviation >= -8.0 || deviation <= -25.0 {
                        continue;
                    }

                    // Calculate urgency based on deviation and MA period
                    let deviation_magnitude = deviation.abs();
                    let base_urgency = (deviation_magnitude - 8.0) / 17.0; // 0.0 at -8%, 1.0 at -25%

                    // Longer MA deviations are more significant
                    let period_multiplier = match period {
                        5 => 0.8,
                        10 => 1.0,
                        20 => 1.2,
                        _ => 1.0,
                    };

                    let urgency = base_urgency * period_multiplier;

                    if urgency > best_urgency {
                        best_urgency = urgency;
                        best_signal = Some(DipSignal {
                            strategy_name: "MovingAverage".to_string(),
                            urgency,
                            drop_percent: deviation,
                            confidence: 0.8,
                            details: format!("MA{} deviation: {:.1}%", period, deviation),
                        });
                    }
                }
            }

            return best_signal;
        }
    }

    None
}

/// Strategy 3: Support Level Bounce
/// Detects when price approaches or bounces from support levels with -10% to -30% drops
fn strategy_support_level_bounce(token: &Token, current_price: f64) -> Option<DipSignal> {
    if let Ok(price_history) = PRICE_HISTORY_24H.try_lock() {
        if let Some(history) = price_history.get(&token.mint) {
            if history.len() < 20 {
                return None;
            }

            let (support_level, _) = find_support_resistance_levels(history);

            if let Some(support) = support_level {
                // Calculate distance from support
                let support_distance = ((current_price - support) / support) * 100.0;

                // Look for price within 5% of support level
                if support_distance >= -5.0 && support_distance <= 10.0 {
                    // Calculate recent drop magnitude
                    let recent_prices: Vec<f64> = history
                        .iter()
                        .rev()
                        .take(10)
                        .map(|(_, p)| *p)
                        .collect();
                    if
                        let Some(recent_high) = recent_prices
                            .iter()
                            .max_by(|a, b| a.partial_cmp(b).unwrap())
                    {
                        let drop_from_high = ((current_price - recent_high) / recent_high) * 100.0;

                        // Look for -10% to -30% drop from recent high
                        if drop_from_high >= -10.0 || drop_from_high <= -30.0 {
                            return None;
                        }

                        // Calculate urgency based on proximity to support and drop magnitude
                        let drop_magnitude = drop_from_high.abs();
                        let base_urgency = (drop_magnitude - 10.0) / 20.0; // 0.0 at -10%, 1.0 at -30%

                        // Bonus for being very close to support
                        let support_proximity_bonus = if support_distance.abs() < 2.0 {
                            1.3
                        } else if support_distance.abs() < 5.0 {
                            1.1
                        } else {
                            1.0
                        };

                        let urgency = base_urgency * support_proximity_bonus;

                        return Some(DipSignal {
                            strategy_name: "SupportBounce".to_string(),
                            urgency,
                            drop_percent: drop_from_high,
                            confidence: 0.85,
                            details: format!(
                                "Drop: {:.1}%, Support dist: {:.1}%",
                                drop_from_high,
                                support_distance
                            ),
                        });
                    }
                }
            }
        }
    }

    None
}

/// Strategy 4: Volume-Weighted Price Dip
/// Detects dips of -7% to -20% with volume confirmation
fn strategy_volume_weighted_dip(token: &Token, current_price: f64) -> Option<DipSignal> {
    // Get volume data from token
    let current_volume = token.volume
        .as_ref()
        .and_then(|v| v.h24)
        .unwrap_or(0.0);

    if current_volume <= 0.0 {
        return None;
    }

    if let Ok(price_history) = PRICE_HISTORY_24H.try_lock() {
        if let Some(history) = price_history.get(&token.mint) {
            if history.len() < 10 {
                return None;
            }

            // Calculate VWAP-like metric using available data
            let recent_prices: Vec<f64> = history
                .iter()
                .rev()
                .take(10)
                .map(|(_, p)| *p)
                .collect();
            let avg_price = recent_prices.iter().sum::<f64>() / (recent_prices.len() as f64);

            let price_deviation = ((current_price - avg_price) / avg_price) * 100.0;

            // Look for -7% to -20% deviation
            if price_deviation >= -7.0 || price_deviation <= -20.0 {
                return None;
            }

            // Check if volume is above average (indicating interest during dip)
            let volume_score = if current_volume > 100000.0 {
                1.2 // High volume
            } else if current_volume > 50000.0 {
                1.0 // Medium volume
            } else {
                0.8 // Low volume
            };

            let deviation_magnitude = price_deviation.abs();
            let base_urgency = (deviation_magnitude - 7.0) / 13.0; // 0.0 at -7%, 1.0 at -20%
            let urgency = base_urgency * volume_score;

            return Some(DipSignal {
                strategy_name: "VolumeWeighted".to_string(),
                urgency,
                drop_percent: price_deviation,
                confidence: 0.75,
                details: format!(
                    "VWAP deviation: {:.1}%, Vol: ${:.0}",
                    price_deviation,
                    current_volume
                ),
            });
        }
    }

    None
}

/// Strategy 5: Multi-Timeframe Convergence
/// Detects when multiple timeframes show dip signals converging (-6% to -18%)
fn strategy_multi_timeframe_convergence(token: &Token, current_price: f64) -> Option<DipSignal> {
    if let Ok(price_history) = PRICE_HISTORY_24H.try_lock() {
        if let Some(history) = price_history.get(&token.mint) {
            if history.len() < 15 {
                return None;
            }

            let prices: Vec<f64> = history
                .iter()
                .map(|(_, price)| *price)
                .collect();

            // Analyze multiple timeframes
            let short_term = analyze_timeframe_trend(&prices, 5); // Last 5 periods
            let medium_term = analyze_timeframe_trend(&prices, 10); // Last 10 periods
            let long_term = analyze_timeframe_trend(&prices, 15); // Last 15 periods

            let mut convergence_signals = 0;
            let mut total_drop = 0.0;

            // Check for dip signals in each timeframe
            if
                short_term.is_dipping &&
                short_term.drop_percent >= 6.0 &&
                short_term.drop_percent <= 18.0
            {
                convergence_signals += 1;
                total_drop += short_term.drop_percent;
            }

            if
                medium_term.is_dipping &&
                medium_term.drop_percent >= 6.0 &&
                medium_term.drop_percent <= 18.0
            {
                convergence_signals += 1;
                total_drop += medium_term.drop_percent;
            }

            if
                long_term.is_dipping &&
                long_term.drop_percent >= 6.0 &&
                long_term.drop_percent <= 18.0
            {
                convergence_signals += 1;
                total_drop += long_term.drop_percent;
            }

            // Require at least 2 timeframes showing dip signals
            if convergence_signals >= 2 {
                let avg_drop = total_drop / (convergence_signals as f64);
                let base_urgency = (avg_drop - 6.0) / 12.0; // 0.0 at -6%, 1.0 at -18%

                // Bonus for more timeframes converging
                let convergence_multiplier = match convergence_signals {
                    2 => 1.0,
                    3 => 1.3,
                    _ => 1.0,
                };

                let urgency = base_urgency * convergence_multiplier;

                return Some(DipSignal {
                    strategy_name: "MultiTimeframe".to_string(),
                    urgency,
                    drop_percent: -avg_drop,
                    confidence: 0.9,
                    details: format!(
                        "Convergence: {} timeframes, avg drop: {:.1}%",
                        convergence_signals,
                        avg_drop
                    ),
                });
            }
        }
    }

    None
}

/// Calculate final urgency based on multiple strategy consensus
fn calculate_multi_strategy_urgency(
    signals: &[DipSignal],
    token: &Token,
    current_price: Option<f64>
) -> f64 {
    if signals.is_empty() {
        return 0.0;
    }

    // Weight strategies by confidence and combine signals
    let mut weighted_urgency = 0.0;
    let mut total_weight = 0.0;

    for signal in signals {
        let weight = signal.confidence;
        weighted_urgency += signal.urgency * weight;
        total_weight += weight;
    }

    let base_urgency = if total_weight > 0.0 { weighted_urgency / total_weight } else { 0.0 };

    // Apply liquidity and quality multipliers
    let liquidity_usd = token.liquidity
        .as_ref()
        .and_then(|l| l.usd)
        .unwrap_or(0.0);

    let (high_threshold, medium_threshold, low_threshold) =
        calculate_dynamic_liquidity_thresholds();

    let liquidity_factor = if liquidity_usd >= high_threshold {
        1.2 // Top 25% liquidity
    } else if liquidity_usd >= medium_threshold {
        1.0 // Top 50% liquidity
    } else if liquidity_usd >= low_threshold {
        0.8 // Top 75% liquidity
    } else {
        0.6 // Bottom 25% liquidity
    };

    // Multi-strategy bonus - more strategies = higher confidence
    let strategy_consensus_bonus = match signals.len() {
        1 => 1.0,
        2 => 1.2,
        3 => 1.4,
        4 => 1.6,
        5 => 1.8,
        _ => 2.0,
    };

    // Check historical data - only if we have a valid price
    let historical_factor = if let Some(price) = current_price {
        if price > 0.0 && price.is_finite() {
            let historical_allowed = is_entry_allowed_by_historical_data(&token.mint, price);
            if historical_allowed {
                1.0
            } else {
                0.5
            }
        } else {
            // Invalid price - don't allow historical analysis
            0.0
        }
    } else {
        // No price loaded - don't allow historical analysis
        0.0
    };

    let final_urgency =
        base_urgency * liquidity_factor * strategy_consensus_bonus * historical_factor;

    f64::min(final_urgency, 2.0) // Cap at 2.0 for multi-strategy signals
}

/// Volatility analysis structure for smart buying decisions
#[derive(Debug, Clone)]
struct VolatilityAnalysis {
    is_in_dip: bool,
    volatility_score: f64,
    average_move_size: f64,
    volatility_scale: f64,
    recent_moves: Vec<f64>,
    support_level: Option<f64>,
    resistance_level: Option<f64>,
}

/// Analyzes token volatility patterns to determine if current price represents a genuine dip
fn analyze_token_volatility_patterns(mint: &str, current_price: f64) -> VolatilityAnalysis {
    if let Ok(price_history) = PRICE_HISTORY_24H.try_lock() {
        if let Some(history) = price_history.get(mint) {
            if history.len() < 3 {
                // Reduced from 10 to 3
                // Not enough data for analysis - be more permissive
                return VolatilityAnalysis {
                    is_in_dip: true, // Changed from false to true - allow trades with limited data
                    volatility_score: 0.5, // Moderate volatility score
                    average_move_size: 5.0, // Reasonable default
                    volatility_scale: 5.0, // Reasonable default
                    recent_moves: Vec::new(),
                    support_level: None,
                    resistance_level: None,
                };
            }

            // Calculate price movements between consecutive points
            let mut price_moves = Vec::new();
            for i in 1..history.len() {
                let prev_price = history[i - 1].1;
                let curr_price = history[i].1;
                if prev_price > 0.0 {
                    let change_percent = ((curr_price - prev_price) / prev_price) * 100.0;
                    price_moves.push(change_percent);
                }
            }

            if price_moves.is_empty() {
                return VolatilityAnalysis {
                    is_in_dip: false,
                    volatility_score: 0.0,
                    average_move_size: 0.0,
                    volatility_scale: 1.0,
                    recent_moves: Vec::new(),
                    support_level: None,
                    resistance_level: None,
                };
            }

            // Calculate volatility metrics
            let average_move =
                price_moves
                    .iter()
                    .map(|m| m.abs())
                    .sum::<f64>() / (price_moves.len() as f64);
            let volatility_score = calculate_volatility_score(&price_moves);

            // Determine volatility scale (how big moves typically are for this token)
            let volatility_scale = determine_volatility_scale(&price_moves);

            // Find recent support and resistance levels
            let (support_level, resistance_level) = find_support_resistance_levels(history);

            // Determine if we're in a genuine dip
            let is_in_dip = is_genuine_dip(
                current_price,
                history,
                &price_moves,
                support_level,
                volatility_scale
            );

            // Debug logging to understand why dip detection might fail
            log(
                LogTag::Trader,
                "DEBUG_DIP",
                &format!(
                    "Dip analysis for {}: current={:.8}, history_len={}, moves_len={}, support={:?}, scale={:.2}, result={}",
                    mint,
                    current_price,
                    history.len(),
                    price_moves.len(),
                    support_level,
                    volatility_scale,
                    is_in_dip
                )
            );

            // Get recent moves (last 5 moves) for pattern analysis
            let recent_moves: Vec<f64> = price_moves.iter().rev().take(5).cloned().collect();

            return VolatilityAnalysis {
                is_in_dip,
                volatility_score,
                average_move_size: average_move,
                volatility_scale,
                recent_moves,
                support_level,
                resistance_level,
            };
        }
    }

    // Fallback if no history available - be permissive for new tokens
    VolatilityAnalysis {
        is_in_dip: true, // Changed from false to true - allow trades for new tokens
        volatility_score: 0.5, // Moderate volatility score
        average_move_size: 10.0, // Assume reasonable volatility
        volatility_scale: 10.0, // Assume reasonable scale
        recent_moves: Vec::new(),
        support_level: None,
        resistance_level: None,
    }
}

/// Calculates volatility score based on price movement patterns
fn calculate_volatility_score(price_moves: &[f64]) -> f64 {
    if price_moves.len() < 3 {
        return 0.0;
    }

    // Calculate standard deviation of price moves
    let mean = price_moves.iter().sum::<f64>() / (price_moves.len() as f64);
    let variance =
        price_moves
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (price_moves.len() as f64);
    let std_dev = variance.sqrt();

    // Normalize volatility score to 0-1 range
    f64::min(std_dev / 20.0, 1.0)
}

/// Determines the typical scale of price movements for this token
fn determine_volatility_scale(price_moves: &[f64]) -> f64 {
    if price_moves.is_empty() {
        return 1.0;
    }

    // Calculate 75th percentile of absolute price moves
    let mut abs_moves: Vec<f64> = price_moves
        .iter()
        .map(|m| m.abs())
        .collect();
    abs_moves.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let percentile_75_index = ((abs_moves.len() as f64) * 0.75) as usize;
    let volatility_scale = if percentile_75_index < abs_moves.len() {
        abs_moves[percentile_75_index]
    } else {
        abs_moves[abs_moves.len() - 1]
    };

    f64::max(volatility_scale, 1.0) // Minimum scale of 1%
}

/// Finds support and resistance levels based on price history
fn find_support_resistance_levels(history: &[(DateTime<Utc>, f64)]) -> (Option<f64>, Option<f64>) {
    if history.len() < 20 {
        return (None, None);
    }

    let prices: Vec<f64> = history
        .iter()
        .map(|(_, price)| *price)
        .collect();

    // Find local minima (support) and maxima (resistance)
    let mut local_minima = Vec::new();
    let mut local_maxima = Vec::new();

    for i in 1..prices.len() - 1 {
        if prices[i] < prices[i - 1] && prices[i] < prices[i + 1] {
            local_minima.push(prices[i]);
        }
        if prices[i] > prices[i - 1] && prices[i] > prices[i + 1] {
            local_maxima.push(prices[i]);
        }
    }

    // Calculate average support and resistance
    let support_level = if !local_minima.is_empty() {
        Some(local_minima.iter().sum::<f64>() / (local_minima.len() as f64))
    } else {
        None
    };

    let resistance_level = if !local_maxima.is_empty() {
        Some(local_maxima.iter().sum::<f64>() / (local_maxima.len() as f64))
    } else {
        None
    };

    (support_level, resistance_level)
}

/// Determines if current price represents a genuine dip based on multiple factors
fn is_genuine_dip(
    current_price: f64,
    history: &[(DateTime<Utc>, f64)],
    price_moves: &[f64],
    support_level: Option<f64>,
    volatility_scale: f64
) -> bool {
    // If we don't have enough data, be permissive
    if history.len() < 5 || price_moves.len() < 3 {
        return true; // Allow trades when we have limited data
    }

    // Check 1: Must be below recent average price (very lenient)
    let recent_prices: Vec<f64> = history
        .iter()
        .rev()
        .take(10)
        .map(|(_, price)| *price)
        .collect();
    let recent_avg = recent_prices.iter().sum::<f64>() / (recent_prices.len() as f64);

    // Very lenient - allow if current price is within 15% above recent average
    let check1 = current_price <= recent_avg * 1.15;
    if !check1 {
        log(
            LogTag::Trader,
            "DEBUG_DIP_FAIL",
            &format!(
                "Check 1 failed: price {:.8} > {:.8} (115% of avg {:.8})",
                current_price,
                recent_avg * 1.15,
                recent_avg
            )
        );
        return false;
    }

    // Check 2: Support level check (very lenient)
    if let Some(support) = support_level {
        // Very lenient - allow if within 50% above support level
        let check2 = current_price <= support * 1.5;
        if !check2 {
            log(
                LogTag::Trader,
                "DEBUG_DIP_FAIL",
                &format!(
                    "Check 2 failed: price {:.8} > {:.8} (150% of support {:.8})",
                    current_price,
                    support * 1.5,
                    support
                )
            );
            return false;
        }
    }

    // Check 3: Recent moves - very flexible
    let recent_moves: Vec<f64> = price_moves.iter().rev().take(5).cloned().collect();
    let downward_moves = recent_moves
        .iter()
        .filter(|&m| *m < -0.5)
        .count(); // Very small threshold

    // Only block if absolutely no downward moves at all
    let check3 = !(recent_moves.len() >= 5 && downward_moves == 0);
    if !check3 {
        log(
            LogTag::Trader,
            "DEBUG_DIP_FAIL",
            &format!(
                "Check 3 failed: no downward moves in recent {} moves (down: {})",
                recent_moves.len(),
                downward_moves
            )
        );
        return false;
    }

    // Check 4: Drop significance (very lenient)
    if let Some(last_price) = recent_prices.get(1) {
        let current_drop = (((current_price - last_price) / last_price) * 100.0).abs();
        // Very small requirement - just 10% of typical move size
        let check4 = !(volatility_scale > 0.0 && current_drop < volatility_scale * 0.1);
        if !check4 {
            log(
                LogTag::Trader,
                "DEBUG_DIP_FAIL",
                &format!(
                    "Check 4 failed: drop {:.2}% < {:.2}% (10% of scale {:.2}%)",
                    current_drop,
                    volatility_scale * 0.1,
                    volatility_scale
                )
            );
            return false;
        }
    }

    log(
        LogTag::Trader,
        "DEBUG_DIP_PASS",
        &format!(
            "All checks passed for price {:.8} (avg: {:.8}, support: {:?})",
            current_price,
            recent_avg,
            support_level
        )
    );
    true
}

/// Helper function to calculate moving average
fn calculate_moving_average(prices: &[f64], period: usize) -> Option<f64> {
    if prices.len() < period {
        return None;
    }

    let sum: f64 = prices.iter().rev().take(period).sum();
    Some(sum / (period as f64))
}

/// Timeframe trend analysis result
#[derive(Debug, Clone)]
struct TimeframeTrend {
    is_dipping: bool,
    drop_percent: f64,
    momentum: f64,
}

/// Analyze trend for a specific timeframe
fn analyze_timeframe_trend(prices: &[f64], period: usize) -> TimeframeTrend {
    if prices.len() < period + 2 {
        return TimeframeTrend {
            is_dipping: false,
            drop_percent: 0.0,
            momentum: 0.0,
        };
    }

    let recent_prices: Vec<f64> = prices.iter().rev().take(period).cloned().collect();
    let current_price = recent_prices[0];
    let period_high = recent_prices
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&current_price);
    let period_start = recent_prices[period - 1];

    // Calculate drop from period high
    let drop_from_high = ((current_price - period_high) / period_high) * 100.0;

    // Calculate momentum (trend direction)
    let momentum = ((current_price - period_start) / period_start) * 100.0;

    // Determine if we're in a dip
    let is_dipping = drop_from_high < -3.0 && momentum < 0.0; // At least 3% drop from high and negative momentum

    TimeframeTrend {
        is_dipping,
        drop_percent: drop_from_high.abs(),
        momentum,
    }
}

/// Checks if entry is allowed based on historical position data for this token
/// Returns true only if current price is below both:
/// 1. Average entry price from past closed positions
/// 2. Maximum price this token has ever reached
pub fn is_entry_allowed_by_historical_data(mint: &str, current_price: f64) -> bool {
    if let Ok(positions) = SAVED_POSITIONS.lock() {
        // Find all closed positions for this token
        let token_positions: Vec<&Position> = positions
            .iter()
            .filter(|p| p.mint == mint && p.exit_price.is_some())
            .collect();

        // If no historical positions, allow entry (first time seeing this token)
        if token_positions.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!(
                    "No historical positions found for token {}, allowing entry at {:.12}",
                    mint,
                    current_price
                )
            );
            return true;
        }

        // Calculate average entry price from past positions
        let total_entry_prices: f64 = token_positions
            .iter()
            .map(|p| p.effective_entry_price.unwrap_or(p.entry_price))
            .sum();
        let average_entry_price = total_entry_prices / (token_positions.len() as f64);

        // Find maximum price this token has ever reached
        let max_historical_price = token_positions
            .iter()
            .map(|p| p.price_highest)
            .fold(0.0, f64::max);

        // Log the analysis
        log(
            LogTag::Trader,
            "ANALYSIS",
            &format!(
                "Historical analysis for {}: Current: {:.12}, Avg Entry: {:.12}, Max Ever: {:.12}, Positions: {}",
                mint,
                current_price,
                average_entry_price,
                max_historical_price,
                token_positions.len()
            )
        );

        // Allow entry only if current price is below both thresholds
        let below_avg_entry = current_price < average_entry_price;
        let below_max_price = current_price < max_historical_price;

        if !below_avg_entry {
            log(
                LogTag::Trader,
                "BLOCK",
                &format!(
                    "Entry blocked: Current price {:.12} >= average entry price {:.12}",
                    current_price,
                    average_entry_price
                )
            );
        }

        if !below_max_price {
            log(
                LogTag::Trader,
                "BLOCK",
                &format!(
                    "Entry blocked: Current price {:.12} >= maximum historical price {:.12}",
                    current_price,
                    max_historical_price
                )
            );
        }

        if below_avg_entry && below_max_price {
            log(
                LogTag::Trader,
                "ALLOW",
                &format!(
                    "Entry allowed: Current price {:.12} < avg entry {:.12} and < max price {:.12}",
                    current_price,
                    average_entry_price,
                    max_historical_price
                )
            );
        }

        return below_avg_entry && below_max_price;
    } else {
        log(
            LogTag::Trader,
            "ERROR",
            "Could not acquire lock on SAVED_POSITIONS for historical analysis"
        );
        return false; // Conservative: don't allow entry if we can't analyze
    }
}

/// Background task to monitor new tokens for entry opportunities
pub async fn monitor_new_entries(shutdown: Arc<Notify>) {
    'outer: loop {
        // Add a maximum processing time for the entire token checking cycle
        let cycle_start = std::time::Instant::now();

        // Update position tracking in price service
        update_position_tracking_in_service().await;

        // Ensure we have tokens to work with
        ensure_tokens_populated().await;

        let mut tokens: Vec<_> = {
            // Get tokens from safe system
            let tokens_from_module = get_tokens_from_safe_system().await;

            // Log total tokens available
            log(
                LogTag::Trader,
                "DEBUG",
                &format!("Total tokens from safe system: {}", tokens_from_module.len())
                    .dimmed()
                    .to_string()
            );

            // Add debug info about token prices and update times
            debug_trader_log(
                "TOKEN_PRICE_DEBUG",
                &format!(
                    "DEBUG: First 3 tokens price info - {} tokens total",
                    tokens_from_module.len()
                )
            );

            for (i, token) in tokens_from_module.iter().take(3).enumerate() {
                let price_from_service = get_token_price_safe(&token.mint).await;
                debug_trader_log(
                    "TOKEN_PRICE_SAMPLE",
                    &format!(
                        "Token {}: {} ({}) - price_service={:?}",
                        i + 1,
                        token.symbol,
                        token.mint,
                        price_from_service
                    )
                );
            }

            // Include all tokens - we want to trade on existing tokens with updated info
            // The discovery system ensures tokens are updated with fresh data before trading
            log(
                LogTag::Trader,
                "DEBUG",
                &format!(
                    "Using all {} tokens for trading (startup filter removed)",
                    tokens_from_module.len()
                )
                    .dimmed()
                    .to_string()
            );

            // Count tokens with liquidity data
            let with_liquidity = tokens_from_module
                .iter()
                .filter(|token| {
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
                })
                .count();

            if with_liquidity > 0 {
                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Processing {} tokens with liquidity", with_liquidity)
                );
            }

            tokens_from_module
        };

        // Sort tokens by liquidity in descending order (highest liquidity first)
        tokens.sort_by(|a, b| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Safety check - if processing is taking too long, log it
        if cycle_start.elapsed() > Duration::from_secs(5) {
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token sorting took too long: {:?}", cycle_start.elapsed())
            );
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!(
                "Checking {} tokens for entry opportunities (sorted by liquidity)",
                tokens.len()
            )
                .dimmed()
                .to_string()
        );

        // Use centralized filtering system to get eligible tokens
        use crate::filtering::{ filter_tokens_with_reasons, get_filtering_stats };

        let (eligible_tokens, rejected_tokens) = filter_tokens_with_reasons(&tokens);

        // Log filtering statistics
        let (total, passed, pass_rate) = get_filtering_stats(&tokens);
        log(
            LogTag::Trader,
            "FILTER_STATS",
            &format!("Token filtering: {}/{} passed ({:.1}% pass rate)", passed, total, pass_rate)
        );

        // Use eligible tokens for trading
        tokens = eligible_tokens;

        // Early return if no tokens to process
        if tokens.is_empty() {
            log(LogTag::Trader, "INFO", "No tokens to process, skipping token checking cycle");

            // Calculate how long we've spent in this cycle
            let cycle_duration = cycle_start.elapsed();
            let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

            if check_shutdown_or_delay(&shutdown, wait_time).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break;
            }
            continue;
        }

        // Use a semaphore to limit the number of concurrent token checks
        // This balances between parallelism and not overwhelming external APIs
        use tokio::sync::Semaphore;
        let semaphore = Arc::new(Semaphore::new(5)); // Reduced to 5 concurrent checks to avoid overwhelming

        // Log filtering summary
        log_filtering_summary(&tokens);

        // Process all tokens in parallel with concurrent tasks
        let mut handles = Vec::new();

        // Get the total token count before starting the loop
        let total_tokens = tokens.len();

        // Note: tokens are still sorted by liquidity from highest to lowest
        for (index, token) in tokens.iter().enumerate() {
            // Check for shutdown before spawning tasks
            if check_shutdown_or_delay(&shutdown, Duration::from_millis(10)).await {
                log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
                break 'outer;
            }

            // Get permit from semaphore to limit concurrency with timeout
            let permit = match
                tokio::time::timeout(
                    Duration::from_secs(SEMAPHORE_ACQUIRE_TIMEOUT_SECS),
                    semaphore.clone().acquire_owned()
                ).await
            {
                Ok(Ok(permit)) => permit,
                Ok(Err(e)) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to acquire semaphore permit: {}", e)
                    );
                    continue;
                }
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!("Semaphore acquire timed out after {} seconds", SEMAPHORE_ACQUIRE_TIMEOUT_SECS)
                    );
                    continue;
                }
            };

            // Clone necessary variables for the task
            let token = token.clone();
            let shutdown_clone = shutdown.clone();

            // Spawn a new task for this token with overall timeout
            let handle = tokio::spawn(async move {
                // Keep the permit alive for the duration of this task
                let _permit = permit; // This will be automatically dropped when the task completes

                // Check for shutdown before starting task
                if check_shutdown_or_delay(&shutdown_clone, Duration::from_millis(1)).await {
                    return None;
                }

                // Clone the symbol for error logging before moving token into timeout
                let token_symbol = token.symbol.clone();

                // Wrap the entire task logic in a timeout to prevent hanging
                match
                    tokio::time::timeout(Duration::from_secs(TOKEN_CHECK_TASK_TIMEOUT_SECS), async {
                        let task_start = std::time::Instant::now();

                        // Use centralized price service instead of direct token field access
                        debug_trader_log(
                            "TASK_TIMING",
                            &format!("Token {} starting price fetch", token.symbol)
                        );
                        let price_start = std::time::Instant::now();

                        if let Some(current_price) = get_token_price_safe(&token.mint).await {
                            debug_trader_log(
                                "TASK_TIMING",
                                &format!(
                                    "Token {} price fetch took {:.2}ms",
                                    token.symbol,
                                    price_start.elapsed().as_millis()
                                )
                            );
                            // CRITICAL: Validate price is actually loaded and valid before any trading
                            if current_price <= 0.0 || !current_price.is_finite() {
                                debug_trader_log(
                                    "PRICE_INVALID",
                                    &format!(
                                        "Token {} ({}): INVALID PRICE DETECTED - not trading. Price = {:.10}",
                                        token.symbol,
                                        token.mint,
                                        current_price
                                    )
                                );
                                return None;
                            }

                            debug_trader_log(
                                "PRICE_SOURCE",
                                &format!(
                                    "Token {} ({}): current price from price service = {:.10}",
                                    token.symbol,
                                    token.mint,
                                    current_price
                                )
                            );

                            // Use centralized filtering system
                            debug_trader_log(
                                "TASK_TIMING",
                                &format!("Token {} starting filtering", token.symbol)
                            );
                            let filter_start = std::time::Instant::now();

                            if !should_buy_token(&token) {
                                // Token was filtered out, skip processing
                                return None;
                            }

                            // Check for shutdown after filtering
                            if
                                check_shutdown_or_delay(
                                    &shutdown_clone,
                                    Duration::from_millis(1)
                                ).await
                            {
                                return None;
                            }

                            debug_trader_log(
                                "TASK_TIMING",
                                &format!(
                                    "Token {} filtering took {:.2}ms",
                                    token.symbol,
                                    filter_start.elapsed().as_millis()
                                )
                            );

                            let liquidity_usd = token.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0);

                            // Update price history with proper error handling and timeout
                            let now = Utc::now();
                            match
                                tokio::time::timeout(
                                    Duration::from_millis(PRICE_CACHE_LOCK_TIMEOUT_MS),
                                    async {
                                        PRICE_HISTORY_24H.try_lock()
                                    }
                                ).await
                            {
                                Ok(Ok(mut hist)) => {
                                    let entry = hist
                                        .entry(token.mint.clone())
                                        .or_insert_with(Vec::new);
                                    entry.push((now, current_price));

                                    // Retain only last 24h
                                    let cutoff =
                                        now - ChronoDuration::hours(PRICE_HISTORY_DURATION_HOURS);
                                    entry.retain(|(ts, _)| *ts >= cutoff);
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire price history lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Check for shutdown after price history update
                            if
                                check_shutdown_or_delay(
                                    &shutdown_clone,
                                    Duration::from_millis(1)
                                ).await
                            {
                                return None;
                            }

                            // Check for entry opportunity using should_buy function
                            let mut buy_urgency = 0.0;

                            // First, get the previous price and release the lock
                            let prev_price = {
                                match
                                    tokio::time::timeout(
                                        Duration::from_millis(PRICE_CACHE_LOCK_TIMEOUT_MS),
                                        async {
                                            LAST_PRICES.try_lock()
                                        }
                                    ).await
                                {
                                    Ok(Ok(last_prices)) => { last_prices.get(&token.mint).copied() }
                                    _ => None,
                                }
                            };

                            if let Some(prev_price) = prev_price {
                                if prev_price > 0.0 {
                                    debug_trader_log(
                                        "TOKEN_CHECK_PRICES",
                                        &format!(
                                            "Token {} ({}): checking buy signal with current={:.10}, prev={:.10}",
                                            token.symbol,
                                            token.mint,
                                            current_price,
                                            prev_price
                                        )
                                    );

                                    // Check if prices are identical (indicating no price update)
                                    if (current_price - prev_price).abs() < 0.000000000001 {
                                        debug_trader_log(
                                            "PRICE_UPDATE_ISSUE",
                                            &format!(
                                                "Token {} ({}): IDENTICAL PRICES detected! Current={:.10} == Prev={:.10} - No price movement detected, this indicates price cache not updating",
                                                token.symbol,
                                                token.mint,
                                                current_price,
                                                prev_price
                                            )
                                        );
                                    }

                                    // Rugcheck safety validation - block rugged or high-risk tokens
                                    debug_trader_log(
                                        "TASK_TIMING",
                                        &format!("Token {} starting rugcheck", token.symbol)
                                    );
                                    let rugcheck_start = std::time::Instant::now();

                                    match get_token_rugcheck_data_safe(&token.mint).await {
                                        Ok(Some(rugcheck_data)) => {
                                            // Block tokens that are detected as rugged
                                            if rugcheck_data.rugged.unwrap_or(false) {
                                                debug_trader_log(
                                                    "RUGCHECK_REJECT_RUGGED",
                                                    &format!(
                                                        "Token {} ({}) rejected - detected as rugged",
                                                        token.symbol,
                                                        token.mint
                                                    )
                                                );
                                                buy_urgency = 0.0;
                                            } else {
                                                // Block tokens with very low rugcheck scores
                                                let rugcheck_score = rugcheck_data.score_normalised
                                                    .or(rugcheck_data.score)
                                                    .unwrap_or(50);

                                                if rugcheck_score < 20 {
                                                    debug_trader_log(
                                                        "RUGCHECK_REJECT_LOW_SCORE",
                                                        &format!(
                                                            "Token {} ({}) rejected - low rugcheck score: {}",
                                                            token.symbol,
                                                            token.mint,
                                                            rugcheck_score
                                                        )
                                                    );
                                                    buy_urgency = 0.0;
                                                } else if
                                                    // Block tokens with critical risks
                                                    let Some(risks) = &rugcheck_data.risks
                                                {
                                                    let critical_risks = risks
                                                        .iter()
                                                        .filter(|r| {
                                                            r.level
                                                                .as_ref()
                                                                .map(
                                                                    |l|
                                                                        l.to_lowercase() ==
                                                                        "critical"
                                                                )
                                                                .unwrap_or(false)
                                                        })
                                                        .count();

                                                    if critical_risks > 0 {
                                                        debug_trader_log(
                                                            "RUGCHECK_REJECT_CRITICAL",
                                                            &format!(
                                                                "Token {} ({}) rejected - {} critical risks detected",
                                                                token.symbol,
                                                                token.mint,
                                                                critical_risks
                                                            )
                                                        );
                                                        buy_urgency = 0.0;
                                                    } else {
                                                        // Use the new should_buy function
                                                        buy_urgency = should_buy(
                                                            &token,
                                                            current_price,
                                                            prev_price
                                                        );

                                                        debug_trader_log(
                                                            "RUGCHECK_OK",
                                                            &format!(
                                                                "Token {} ({}) passed rugcheck validation (score: {})",
                                                                token.symbol,
                                                                token.mint,
                                                                rugcheck_score
                                                            )
                                                        );
                                                    }
                                                } else {
                                                    // Use the new should_buy function
                                                    buy_urgency = should_buy(
                                                        &token,
                                                        current_price,
                                                        prev_price
                                                    );

                                                    debug_trader_log(
                                                        "RUGCHECK_OK",
                                                        &format!(
                                                            "Token {} ({}) passed rugcheck validation (score: {})",
                                                            token.symbol,
                                                            token.mint,
                                                            rugcheck_score
                                                        )
                                                    );
                                                }
                                            }
                                        }
                                        Ok(None) => {
                                            debug_trader_log(
                                                "RUGCHECK_MISSING",
                                                &format!(
                                                    "Token {} ({}) has no rugcheck data - proceeding with caution",
                                                    token.symbol,
                                                    token.mint
                                                )
                                            );

                                            // Use the new should_buy function
                                            buy_urgency = should_buy(
                                                &token,
                                                current_price,
                                                prev_price
                                            );
                                        }
                                        Err(e) => {
                                            debug_trader_log(
                                                "RUGCHECK_ERROR",
                                                &format!(
                                                    "Token {} ({}) rugcheck error: {} - proceeding with caution",
                                                    token.symbol,
                                                    token.mint,
                                                    e
                                                )
                                            );

                                            // Use the new should_buy function
                                            buy_urgency = should_buy(
                                                &token,
                                                current_price,
                                                prev_price
                                            );
                                        }
                                    }

                                    debug_trader_log(
                                        "TASK_TIMING",
                                        &format!(
                                            "Token {} rugcheck took {:.2}ms",
                                            token.symbol,
                                            rugcheck_start.elapsed().as_millis()
                                        )
                                    );

                                    // Check for shutdown after rugcheck processing
                                    if
                                        check_shutdown_or_delay(
                                            &shutdown_clone,
                                            Duration::from_millis(1)
                                        ).await
                                    {
                                        return None;
                                    }

                                    debug_trader_log(
                                        "TOKEN_CHECK_RESULT",
                                        &format!(
                                            "Token {} ({}): buy urgency result: {:.3}",
                                            token.symbol,
                                            token.mint,
                                            buy_urgency
                                        )
                                    );
                                } else {
                                    debug_trader_log(
                                        "TOKEN_CHECK_INVALID_PREV",
                                        &format!(
                                            "Token {} ({}): invalid prev_price: {:.10}",
                                            token.symbol,
                                            token.mint,
                                            prev_price
                                        )
                                    );
                                }
                            } else {
                                debug_trader_log(
                                    "TOKEN_CHECK_NO_PREV",
                                    &format!(
                                        "Token {} ({}): no previous price available",
                                        token.symbol,
                                        token.mint
                                    )
                                );
                            }

                            // Update price in cache
                            match
                                tokio::time::timeout(
                                    Duration::from_millis(PRICE_CACHE_LOCK_TIMEOUT_MS),
                                    async {
                                        LAST_PRICES.try_lock()
                                    }
                                ).await
                            {
                                Ok(Ok(mut last_prices)) => {
                                    // Add debug info before updating price
                                    debug_trader_log(
                                        "PRICE_UPDATE_BEFORE",
                                        &format!(
                                            "Token {} ({}): BEFORE UPDATE - old_price_in_cache={:.10}, new_current_price={:.10}",
                                            token.symbol,
                                            token.mint,
                                            last_prices.get(&token.mint).copied().unwrap_or(0.0),
                                            current_price
                                        )
                                    );

                                    last_prices.insert(token.mint.clone(), current_price);

                                    debug_trader_log(
                                        "PRICE_UPDATE_AFTER",
                                        &format!(
                                            "Token {} ({}): AFTER UPDATE - cache now contains: {:.10}",
                                            token.symbol,
                                            token.mint,
                                            current_price
                                        )
                                    );
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // If we can't get the lock within 500ms, just log and continue
                                    log(
                                        LogTag::Trader,
                                        "WARN",
                                        &format!(
                                            "Could not acquire last_prices lock for {} within timeout",
                                            token.symbol
                                        )
                                    );
                                }
                            }

                            // Return the token and price if buy signal detected
                            if buy_urgency > 0.0 {
                                // Check for shutdown before attempting to buy
                                if
                                    check_shutdown_or_delay(
                                        &shutdown_clone,
                                        Duration::from_millis(1)
                                    ).await
                                {
                                    return None;
                                }

                                let change = if let Ok(last_prices) = LAST_PRICES.try_lock() {
                                    if let Some(&prev_price) = last_prices.get(&token.mint) {
                                        if prev_price > 0.0 {
                                            ((current_price - prev_price) / prev_price) * 100.0
                                        } else {
                                            0.0
                                        }
                                    } else {
                                        0.0
                                    }
                                } else {
                                    0.0
                                };

                                return Some((token, current_price, change));
                            }
                        } else {
                            // Price is not loaded - do not attempt any trading
                            debug_trader_log(
                                "PRICE_NOT_LOADED",
                                &format!(
                                    "Token {} ({}): PRICE NOT LOADED - skipping trading. Price service returned None",
                                    token.symbol,
                                    token.mint
                                )
                            );
                        }

                        debug_trader_log(
                            "TASK_TIMING",
                            &format!(
                                "Token {} task completed in {:.2}ms",
                                token.symbol,
                                task_start.elapsed().as_millis()
                            )
                        );

                        None
                    }).await
                {
                    Ok(result) => result,
                    Err(_) => {
                        // Task timed out
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!(
                                "Token check task for {} timed out after {} seconds",
                                token_symbol,
                                TOKEN_CHECK_TASK_TIMEOUT_SECS
                            )
                        );
                        None
                    }
                }
            });

            handles.push(handle);
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Successfully spawned {} token checking tasks", handles.len())
                .dimmed()
                .to_string()
        );

        // Process the results of all tasks with overall timeout
        let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
            // This maintains the priority of processing high-liquidity tokens first
            log(
                LogTag::Trader,
                "INFO",
                &format!("Waiting for {} token checks to complete", handles.len())
                    .dimmed()
                    .to_string()
            );

            let mut opportunities = Vec::new();

            for handle in handles {
                // Skip any tasks that failed or if shutdown signal received
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "new entries monitor shutting down during result collection..."
                    );
                    return opportunities; // Return what we have so far
                }

                // Add timeout for each handle to prevent getting stuck on a single task
                match tokio::time::timeout(Duration::from_secs(120), handle).await {
                    Ok(task_result) => {
                        match task_result {
                            Ok(Some((token, price, percent_change))) => {
                                opportunities.push((token, price, percent_change));
                            }
                            Ok(None) => {
                                // No opportunity found for this token, continue
                            }
                            Err(e) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Token check task failed: {}", e)
                                );
                            }
                        }
                    }
                    Err(_) => {
                        // Task timed out after 5 seconds
                        log(LogTag::Trader, "WARN", "Token check task timed out after 5 seconds");
                    }
                }
            }

            opportunities
        }).await;

        let mut opportunities = match collection_result {
            Ok(opportunities) => opportunities,
            Err(_) => {
                log(LogTag::Trader, "ERROR", "Token check collection timed out after 60 seconds");
                Vec::new() // Return empty if timeout
            }
        };

        // Sort opportunities by liquidity again to ensure priority
        opportunities.sort_by(|(a, _, _), (b, _, _)| {
            let liquidity_a = a.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);
            let liquidity_b = b.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0);

            liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
        });

        log(
            LogTag::Trader,
            "INFO",
            &format!("Found {} potential entry opportunities", opportunities.len())
        );

        // Log the total time taken for the token checking cycle
        log(
            LogTag::Trader,
            "INFO",
            &format!("Token checking cycle completed in {:?}", cycle_start.elapsed())
                .dimmed()
                .to_string()
        );

        // Process opportunities concurrently while respecting position limits
        if !opportunities.is_empty() {
            let current_open_count = get_open_positions_count();
            let available_slots = MAX_OPEN_POSITIONS.saturating_sub(current_open_count);

            if available_slots == 0 {
                log(
                    LogTag::Trader,
                    "LIMIT",
                    &format!(
                        "Maximum open positions already reached ({}/{}). Skipping all opportunities.",
                        current_open_count,
                        MAX_OPEN_POSITIONS
                    )
                );
            } else {
                // Limit opportunities to available slots
                let opportunities_to_process = opportunities
                    .into_iter()
                    .take(available_slots)
                    .collect::<Vec<_>>();

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Processing {} opportunities concurrently (available slots: {}, current open: {})",
                        opportunities_to_process.len(),
                        available_slots,
                        current_open_count
                    )
                );

                // Use a semaphore to limit concurrent buy transactions
                use tokio::sync::Semaphore;
                let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent buys

                let mut handles = Vec::new();

                // Process all buy orders concurrently
                for (token, price, percent_change) in opportunities_to_process {
                    // Check for shutdown before spawning tasks
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "new entries monitor shutting down during buy processing..."
                        );
                        break;
                    }

                    // Get permit from semaphore to limit concurrency with timeout
                    let permit = match
                        tokio::time::timeout(
                            Duration::from_secs(120),
                            semaphore.clone().acquire_owned()
                        ).await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to acquire semaphore permit for buy: {}", e)
                            );
                            continue;
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                "Semaphore acquire timed out for buy operation"
                            );
                            continue;
                        }
                    };

                    let handle = tokio::spawn(async move {
                        let _permit = permit; // Keep permit alive for duration of task

                        let token_symbol = token.symbol.clone();

                        // Wrap the buy operation in a timeout
                        match
                            tokio::time::timeout(Duration::from_secs(120), async {
                                open_position(&token, price, percent_change).await
                            }).await
                        {
                            Ok(_) => {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Completed buy operation for {} in concurrent task", token_symbol)
                                );
                                true
                            }
                            Err(_) => {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Buy operation for {} timed out after 20 seconds", token_symbol)
                                );
                                false
                            }
                        }
                    });

                    handles.push(handle);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!("Spawned {} concurrent buy tasks", handles.len()).dimmed().to_string()
                );

                // Collect results from all concurrent buy operations with overall timeout
                let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                    let mut completed = 0;
                    let mut successful = 0;
                    let total_handles = handles.len();

                    for handle in handles {
                        // Skip if shutdown signal received
                        if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                            log(
                                LogTag::Trader,
                                "INFO",
                                "new entries monitor shutting down during buy result collection..."
                            );
                            break;
                        }

                        // Add timeout for each handle to prevent getting stuck
                        match tokio::time::timeout(Duration::from_secs(120), handle).await {
                            Ok(task_result) => {
                                match task_result {
                                    Ok(success) => {
                                        if success {
                                            successful += 1;
                                        }
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::Trader,
                                            "ERROR",
                                            &format!("Buy task failed: {}", e)
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                log(LogTag::Trader, "WARN", "Buy task timed out after 5 seconds");
                            }
                        }

                        completed += 1;
                        if completed % 2 == 0 || completed == total_handles {
                            log(
                                LogTag::Trader,
                                "INFO",
                                &format!("Completed {}/{} buy operations", completed, total_handles)
                                    .dimmed()
                                    .to_string()
                            );
                        }
                    }

                    (completed, successful)
                }).await;

                match collection_result {
                    Ok((completed, successful)) => {
                        let new_open_count = get_open_positions_count();
                        log(
                            LogTag::Trader,
                            "INFO",
                            &format!(
                                "Concurrent buy operations completed: {}/{} successful, new open positions: {}",
                                successful,
                                completed,
                                new_open_count
                            )
                        );
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            "Buy operations collection timed out after 30 seconds"
                        );
                    }
                }
            }
        }

        // Calculate how long we've spent in this cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
            // If we've already spent more time than the interval, just wait a short time
            log(
                LogTag::Trader,
                "WARN",
                &format!("Token checking cycle took longer than interval: {:?}", cycle_duration)
            );
            Duration::from_millis(100)
        } else {
            // Otherwise wait for the remaining interval time
            Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        if check_shutdown_or_delay(&shutdown, wait_time).await {
            log(LogTag::Trader, "INFO", "new entries monitor shutting down...");
            break;
        }
    }
}

/// Background task to monitor open positions for exit opportunities
pub async fn monitor_open_positions(shutdown: Arc<Notify>) {
    loop {
        // First, collect all open position mints to fetch pool prices in parallel
        let open_position_mints: Vec<String> = {
            if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions
                    .iter()
                    .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
                    .map(|p| p.mint.clone())
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Request immediate pool price checks for open positions (non-blocking)
        if !open_position_mints.is_empty() {
            for mint in &open_position_mints {
                let mint_clone = mint.clone();
                // Temporarily disabled tokio::spawn due to Send/Sync issues with database
                // TODO: Re-enable once database is made thread-safe
                // tokio::spawn(async move {
                //     // Use new pool price system for immediate price check
                //     let _price = crate::tokens::get_token_price(&mint_clone).await;
                // });
            }
        }

        let mut positions_to_close = Vec::new();

        // First, collect open positions data (without holding mutex across await)
        let open_positions_data: Vec<(usize, Position)> = {
            if let Ok(positions) = SAVED_POSITIONS.lock() {
                positions
                    .iter()
                    .enumerate()
                    .filter_map(|(index, position)| {
                        if position.position_type == "buy" && position.exit_price.is_none() {
                            Some((index, position.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            }
        };

        // Now process each position with async calls (mutex is released)
        for (index, mut position) in open_positions_data {
            // Get current price from safe price service
            if let Some(current_price) = get_token_price_safe(&position.mint).await {
                if current_price > 0.0 && current_price.is_finite() {
                    // Update position tracking (extremes) on the local copy
                    update_position_tracking(&mut position, current_price);

                    // Calculate P&L using unified function with pool price
                    let (pnl_sol, pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(current_price)
                    );

                    let now = Utc::now();

                    // Calculate sell urgency using the unified profit system
                    let (sell_urgency, sell_reason) = crate::profit::should_sell(
                        &position,
                        current_price
                    ).await;

                    // Use profit system decision only - no duplicate emergency checks
                    let should_exit = sell_urgency > 0.7;

                    if is_debug_trader_enabled() {
                        debug_trader_log(
                            "SELL_ANALYSIS",
                            &format!(
                                "{} | Urgency: {:.2} | Reason: {} | Exit: {}",
                                position.symbol,
                                sell_urgency,
                                sell_reason,
                                should_exit
                            )
                        );
                    }

                    if should_exit {
                        let minimal_token = Token {
                            mint: position.mint.clone(),
                            symbol: position.symbol.clone(),
                            name: position.symbol.clone(),
                            chain: "solana".to_string(),
                            // All other fields as None/defaults
                            logo_url: None,
                            coingecko_id: None,
                            website: None,
                            description: None,
                            tags: vec![],
                            is_verified: false,
                            created_at: None,
                            price_dexscreener_sol: None,
                            price_dexscreener_usd: None,
                            price_pool_sol: None,
                            price_pool_usd: None,
                            dex_id: None,
                            pair_address: None,
                            pair_url: None,
                            labels: vec![],
                            fdv: None,
                            market_cap: None,
                            txns: None,
                            volume: None,
                            price_change: None,
                            liquidity: None,
                            info: None,
                            boosts: None,
                        };

                        log(
                            LogTag::Trader,
                            "SELL",
                            &format!(
                                "Sell signal for {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Emergency: {}",
                                position.symbol,
                                position.mint,
                                sell_urgency,
                                pnl_percent,
                                sell_urgency >= 1.0 // Emergency is when urgency is 1.0 (from profit.rs)
                            )
                        );

                        positions_to_close.push((
                            index,
                            position.clone(), // Include the full position data
                            minimal_token,
                            current_price,
                            now,
                        ));
                    } else {
                        // log(
                        //     LogTag::Trader,
                        //     "HOLD",
                        //     &format!(
                        //         "Holding {} ({}) - Urgency: {:.2}, P&L: {:.2}%, Price: {:.12}",
                        //         position.symbol,
                        //         position.mint,
                        //         sell_urgency,
                        //         pnl_percent,
                        //         current_price
                        //     )
                        // );
                    }

                    // Update the position in the global list with tracking data
                    if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                        if let Some(saved_position) = positions.get_mut(index) {
                            saved_position.price_highest = position.price_highest;
                            saved_position.price_lowest = position.price_lowest;
                        }
                    }
                } else {
                    // Price found but invalid (0, negative, or NaN)
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!(
                            "Invalid price for position monitoring: {} ({}) - Price = {:.10}",
                            position.symbol,
                            position.mint,
                            current_price
                        )
                    );
                }
            } else {
                log(
                    LogTag::Trader,
                    "WARN",
                    &format!(
                        "No price found for open position: {} ({})",
                        position.symbol,
                        position.mint
                    )
                );
            }
        }

        // Close positions that need to be closed concurrently (outside of lock to avoid deadlock)
        if !positions_to_close.is_empty() {
            log(
                LogTag::Trader,
                "INFO",
                &format!("Processing {} positions for concurrent closing", positions_to_close.len())
            );

            // Use a semaphore to limit concurrent sell transactions to avoid overwhelming the network
            use tokio::sync::Semaphore;
            let semaphore = Arc::new(Semaphore::new(3)); // Allow up to 3 concurrent sells

            let mut handles = Vec::new();

            // Process all sell orders concurrently
            for (index, position, token, exit_price, exit_time) in positions_to_close {
                // Check for shutdown before spawning tasks
                if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                    log(
                        LogTag::Trader,
                        "INFO",
                        "open positions monitor shutting down during sell processing..."
                    );
                    break;
                }

                // Get permit from semaphore to limit concurrency with timeout
                let permit = match
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        semaphore.clone().acquire_owned()
                    ).await
                {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(e)) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to acquire semaphore permit for sell: {}", e)
                        );
                        continue;
                    }
                    Err(_) => {
                        log(
                            LogTag::Trader,
                            "WARN",
                            "Semaphore acquire timed out for sell operation"
                        );
                        continue;
                    }
                };

                // We already have the position from the analysis phase, no need to look it up
                let handle = tokio::spawn(async move {
                    let _permit = permit; // Keep permit alive for duration of task

                    let mut position = position;
                    let token_symbol = token.symbol.clone();

                    // FINAL SAFEGUARD: Only allow sales at profit (>0%) or significant loss (<=-55%)
                    let (_, final_pnl_percent) = calculate_position_pnl(
                        &position,
                        Some(exit_price)
                    );

                    // Block sales only if it's a small loss (between 0% and -55%)
                    if
                        final_pnl_percent <= 0.0 &&
                        final_pnl_percent > crate::profit::STOP_LOSS_PERCENT
                    {
                        log(
                            LogTag::Trader,
                            "SAFEGUARD",
                            &format!(
                                "Blocking sale of {} - P&L {:.2}% is a small loss (between 0% and {:.2}%)",
                                token_symbol,
                                final_pnl_percent,
                                crate::profit::STOP_LOSS_PERCENT
                            )
                        );
                        return None; // Abort the sale - small loss, hold until bigger loss or profit
                    }

                    // Wrap the sell operation in a timeout
                    match
                        tokio::time::timeout(Duration::from_secs(120), async {
                            close_position(&mut position, &token, exit_price, exit_time).await
                        }).await
                    {
                        Ok(success) => {
                            if success {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("Successfully closed position for {} in concurrent task", token_symbol)
                                );
                                Some((index, position))
                            } else {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("Failed to close position for {} in concurrent task", token_symbol)
                                );
                                None
                            }
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Sell operation for {} timed out after 15 seconds", token_symbol)
                            );
                            None
                        }
                    }
                });

                handles.push(handle);
            }

            log(
                LogTag::Trader,
                "INFO",
                &format!("Spawned {} concurrent sell tasks", handles.len()).dimmed().to_string()
            );

            // Collect results from all concurrent sell operations with overall timeout
            // Increased timeout to 60 seconds to accommodate multiple 15-second sell operations
            let collection_result = tokio::time::timeout(Duration::from_secs(120), async {
                let mut completed_positions = Vec::new();

                for handle in handles {
                    // Skip if shutdown signal received
                    if check_shutdown_or_delay(&shutdown, Duration::from_millis(1)).await {
                        log(
                            LogTag::Trader,
                            "INFO",
                            "open positions monitor shutting down during sell result collection..."
                        );
                        break;
                    }

                    // Add timeout for each handle to prevent getting stuck
                    // Increased timeout to 15 seconds to allow for transaction verification and ATA closing
                    match tokio::time::timeout(Duration::from_secs(120), handle).await {
                        Ok(task_result) => {
                            match task_result {
                                Ok(Some((index, updated_position))) => {
                                    completed_positions.push((index, updated_position));
                                }
                                Ok(None) => {
                                    // Position failed to close, continue
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Trader,
                                        "ERROR",
                                        &format!("Sell task failed: {}", e)
                                    );
                                }
                            }
                        }
                        Err(_) => {
                            log(LogTag::Trader, "WARN", "Sell task timed out after 60 seconds");
                        }
                    }
                }

                completed_positions
            }).await;

            let completed_positions = match collection_result {
                Ok(positions) => positions,
                Err(_) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        "Sell operations collection timed out after 60 seconds"
                    );
                    Vec::new()
                }
            };

            // Update all successfully closed positions in the saved positions
            if !completed_positions.is_empty() {
                if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                    for (index, updated_position) in &completed_positions {
                        if let Some(saved_position) = positions.get_mut(*index) {
                            *saved_position = updated_position.clone();
                        }
                    }
                    save_positions_to_file(&positions);
                }

                log(
                    LogTag::Trader,
                    "INFO",
                    &format!(
                        "Updated {} positions after concurrent sell operations",
                        completed_positions.len()
                    )
                );
            }
        }

        if
            check_shutdown_or_delay(
                &shutdown,
                Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)
            ).await
        {
            log(LogTag::Trader, "INFO", "open positions monitor shutting down...");
            break;
        }
    }
}

/// Main trader function that spawns both monitoring tasks
pub async fn trader(shutdown: Arc<Notify>) {
    log(LogTag::Trader, "INFO", "Starting trader with background tasks...");

    let shutdown_clone = shutdown.clone();
    let entries_task = tokio::spawn(async move {
        monitor_new_entries(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let positions_task = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    let shutdown_clone = shutdown.clone();
    let display_task = tokio::spawn(async move {
        monitor_positions_display(shutdown_clone).await;
    });

    // Wait for shutdown signal
    shutdown.notified().await;

    log(LogTag::Trader, "INFO", "Trader shutting down...");

    // Give tasks a chance to shutdown gracefully
    let graceful_timeout = tokio::time::timeout(Duration::from_secs(5), async {
        let _ = tokio::try_join!(entries_task, positions_task, display_task);
    });

    match graceful_timeout.await {
        Ok(_) => {
            log(LogTag::Trader, "INFO", "Trader tasks finished gracefully");
        }
        Err(_) => {
            log(LogTag::Trader, "WARN", "Trader tasks did not finish gracefully, aborting");
            // Force abort if graceful shutdown fails
            // entries_task.abort(); // These might already be finished
            // positions_task.abort();
            // display_task.abort();
        }
    }
}
