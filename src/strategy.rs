#![allow(warnings)]
use crate::prelude::*;

// PROFESSIONAL HIGH-FREQUENCY TRADING CONSTANTS
// ADVANCED MULTI-POSITION TRADING CONSTANTS
pub const TRADE_SIZE_SOL: f64 = 0.001; // Smaller size for more positions
pub const MAX_OPEN_POSITIONS: usize = 50; // Increased to 50 positions
pub const POSITION_SIZE_SCALPING: f64 = 0.0005; // Even smaller for scalping
pub const MIN_HOLD_TIME_SECONDS: i64 = 1800; // Minimum 30 minutes hold
pub const MAX_HOLD_TIME_SECONDS: i64 = 86400; // Maximum 24 hours hold
pub const SCALPING_HOLD_TIME_SECONDS: i64 = 300; // 5 minutes for scalping
pub const MAX_DCA_COUNT: u8 = 2; // Increased DCA allowance for recovery
pub const TRANSACTION_FEE_SOL: f64 = 0.0001; // More realistic fee estimate including slippage
pub const POSITIONS_CHECK_TIME: u64 = 2; // Check every 2 seconds
pub const POSITIONS_PRINT_TIME: u64 = 5; // Print every 5 seconds
pub const PRICE_HISTORY_CAP: usize = 120; // Larger history for better analysis
pub const SLIPPAGE_BPS: f64 = 0.5; // Slightly increased for better execution
pub const FEE_RATE: f64 = 0.000015; // More realistic total fee rate
pub const DCA_SIZE_FACTOR: f64 = 0.8; // Larger DCA when used

// SCALPING STRATEGY CONSTANTS
pub const MIN_VOLUME_USD: f64 = 5000.0; // Minimum daily volume
pub const MIN_LIQUIDITY_SOL: f64 = 10.0; // Minimum liquidity
pub const MAX_SPREAD_BPS: f64 = 50.0; // Maximum bid-ask spread
pub const MOMENTUM_PERIOD: usize = 5; // Periods for momentum calculation
pub const RSI_PERIOD: usize = 14; // RSI calculation period
pub const BB_PERIOD: usize = 20; // Bollinger Bands period
pub const BB_STD_DEV: f64 = 2.0; // Standard deviation multiplier
pub const VWAP_PERIOD: usize = 20; // VWAP calculation period
pub const QUICK_PROFIT_TARGET: f64 = 2.0; // Start taking profits at 1%
pub const MAX_PROFIT_TARGET: f64 = 500.0; // Maximum profit target 500%
pub const RAPID_STOP_LOSS: f64 = -25.0; // Slightly more reasonable stop loss, but still protective
pub const MAX_TRADE_DURATION_SEC: i64 = 86400; // Max 24 hours for longer holds
pub const MIN_PROFIT_BEFORE_EXIT_SOL: f64 = FEE_RATE * 0.3; // Even lower minimum profit requirement

// MULTI-TIMEFRAME ANALYSIS CONSTANTS
pub const TIMEFRAME_1M: usize = 1; // 1 minute
pub const TIMEFRAME_5M: usize = 5; // 5 minutes
pub const TIMEFRAME_15M: usize = 15; // 15 minutes
pub const TIMEFRAME_1H: usize = 60; // 1 hour
pub const TIMEFRAME_4H: usize = 240; // 4 hours
pub const TIMEFRAME_1D: usize = 1440; // 1 day

// LEARNING SYSTEM CONSTANTS
pub const LEARNING_HISTORY_SIZE: usize = 1000; // Remember last 1000 trades
pub const MIN_TRADES_FOR_LEARNING: usize = 50; // Minimum trades before learning kicks in
pub const SUCCESS_RATE_THRESHOLD: f64 = 0.6; // 60% success rate threshold

// PROFIT TAKING TIERS - More realistic targets
pub const PROFIT_TIER_1: f64 = 1.0; // 1% - quick scalp
pub const PROFIT_TIER_2: f64 = 2.5; // 2.5% - small profit
pub const PROFIT_TIER_3: f64 = 5.0; // 5% - medium profit
pub const PROFIT_TIER_4: f64 = 10.0; // 10% - good profit
pub const PROFIT_TIER_5: f64 = 20.0; // 20% - great profit
pub const PROFIT_TIER_6: f64 = 50.0; // 50% - excellent profit
pub const PROFIT_TIER_7: f64 = 100.0; // 100% - exceptional profit
pub const PROFIT_TIER_8: f64 = 200.0; // 200% - extraordinary profit
pub const PROFIT_TIER_9: f64 = 500.0; // 500% - maximum target

// MULTI-TIMEFRAME ANALYSIS FUNCTIONS

/// Analyze multiple timeframes to avoid buying at peaks
fn analyze_multi_timeframe(
    dataframe: &MarketDataFrame,
    token: &Token,
    current_price: f64
) -> (bool, String) {
    let mut reasons = Vec::new();

    // Check if we're at a peak across multiple timeframes
    if is_at_peak_across_timeframes(dataframe, token, current_price) {
        reasons.push("at_peak_multiple_timeframes");
        return (false, reasons.join(","));
    }

    // Check RSI across different periods
    let rsi_5m = calculate_rsi_for_timeframe(dataframe, 5);
    let rsi_15m = calculate_rsi_for_timeframe(dataframe, 15);
    let rsi_1h = calculate_rsi_for_timeframe(dataframe, 60);

    // All timeframes should be oversold or neutral
    if let (Some(rsi_5), Some(rsi_15), Some(rsi_1h)) = (rsi_5m, rsi_15m, rsi_1h) {
        if rsi_5 > 70.0 || rsi_15 > 65.0 || rsi_1h > 60.0 {
            reasons.push("overbought_timeframes");
            return (false, reasons.join(","));
        }

        // Prefer when short-term is oversold but longer-term is neutral
        if rsi_5 < 30.0 && rsi_15 < 40.0 && rsi_1h < 50.0 {
            reasons.push("oversold_alignment");
        }
    }

    // Check trend alignment across timeframes
    if !is_trend_aligned_for_entry(dataframe, token) {
        reasons.push("trend_misalignment");
        return (false, reasons.join(","));
    }

    // Check volume profile across timeframes
    if !is_volume_profile_healthy(dataframe, token) {
        reasons.push("unhealthy_volume_profile");
        return (false, reasons.join(","));
    }

    reasons.push("multi_timeframe_confirmed");
    (true, reasons.join(","))
}

/// Check if we're at a peak across multiple timeframes
fn is_at_peak_across_timeframes(
    dataframe: &MarketDataFrame,
    token: &Token,
    current_price: f64
) -> bool {
    // Check recent price action
    if dataframe.prices.len() < 60 {
        return false;
    }

    // Check if price is near recent highs in multiple timeframes
    let prices_5m = get_timeframe_prices(dataframe, 5);
    let prices_15m = get_timeframe_prices(dataframe, 15);
    let prices_1h = get_timeframe_prices(dataframe, 60);

    let peak_5m = prices_5m
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&0.0);
    let peak_15m = prices_15m
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&0.0);
    let peak_1h = prices_1h
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&0.0);

    // If current price is within 5% of peaks in multiple timeframes, avoid buying
    let near_peak_5m = current_price >= peak_5m * 0.95;
    let near_peak_15m = current_price >= peak_15m * 0.95;
    let near_peak_1h = current_price >= peak_1h * 0.95;

    // If near peak in 2 or more timeframes, avoid
    let peak_count = [near_peak_5m, near_peak_15m, near_peak_1h]
        .iter()
        .filter(|&&x| x)
        .count();
    peak_count >= 2
}

/// Calculate RSI for specific timeframe
fn calculate_rsi_for_timeframe(
    dataframe: &MarketDataFrame,
    timeframe_minutes: usize
) -> Option<f64> {
    if dataframe.prices.len() < timeframe_minutes * 2 {
        return None;
    }

    let step = timeframe_minutes.max(1);
    let prices: VecDeque<f64> = dataframe.prices.iter().step_by(step).cloned().collect();

    calculate_rsi(&prices, RSI_PERIOD)
}

/// Get prices for specific timeframe
fn get_timeframe_prices(dataframe: &MarketDataFrame, timeframe_minutes: usize) -> Vec<f64> {
    if dataframe.prices.len() < timeframe_minutes {
        return Vec::new();
    }

    let step = timeframe_minutes.max(1);
    dataframe.prices.iter().step_by(step).cloned().collect()
}

/// Check if trend is aligned across timeframes for entry
fn is_trend_aligned_for_entry(dataframe: &MarketDataFrame, token: &Token) -> bool {
    // Short-term trend should be neutral to slightly down
    // Medium-term trend should be neutral to slightly up
    // Long-term trend should be neutral to up

    if dataframe.prices.len() < 240 {
        return false;
    }

    let short_trend = calculate_trend_for_timeframe(dataframe, 5);
    let medium_trend = calculate_trend_for_timeframe(dataframe, 15);
    let long_trend = calculate_trend_for_timeframe(dataframe, 60);

    // Avoid strong downtrends in any timeframe
    if short_trend < -0.05 || medium_trend < -0.03 || long_trend < -0.02 {
        return false;
    }

    // Prefer when short-term is down but medium/long-term are stable
    short_trend <= 0.02 && medium_trend >= -0.01 && long_trend >= -0.01
}

/// Calculate trend for specific timeframe
fn calculate_trend_for_timeframe(dataframe: &MarketDataFrame, timeframe_minutes: usize) -> f64 {
    let prices = get_timeframe_prices(dataframe, timeframe_minutes);
    if prices.len() < 10 {
        return 0.0;
    }

    calculate_price_trend(&prices)
}

/// Check if volume profile is healthy across timeframes
fn is_volume_profile_healthy(dataframe: &MarketDataFrame, token: &Token) -> bool {
    if dataframe.volumes.len() < 60 {
        return false;
    }

    // Check volume consistency across timeframes
    let vol_5m = get_average_volume_for_timeframe(dataframe, 5);
    let vol_15m = get_average_volume_for_timeframe(dataframe, 15);
    let vol_1h = get_average_volume_for_timeframe(dataframe, 60);

    // Volume should be reasonably consistent, not dropping off
    if vol_5m < vol_15m * 0.5 || vol_15m < vol_1h * 0.5 {
        return false;
    }

    // Check for volume spikes that might indicate manipulation
    let current_vol = *dataframe.volumes.back().unwrap_or(&0.0);
    let avg_vol = dataframe.volumes.iter().rev().take(20).sum::<f64>() / 20.0;

    // Avoid extreme volume spikes
    if current_vol > avg_vol * 10.0 {
        return false;
    }

    true
}

/// Get average volume for specific timeframe
fn get_average_volume_for_timeframe(dataframe: &MarketDataFrame, timeframe_minutes: usize) -> f64 {
    if dataframe.volumes.len() < timeframe_minutes {
        return 0.0;
    }

    let step = timeframe_minutes.max(1);
    let volumes: Vec<f64> = dataframe.volumes.iter().step_by(step).cloned().collect();

    if volumes.is_empty() {
        return 0.0;
    }

    volumes.iter().sum::<f64>() / (volumes.len() as f64)
}

/// Determine if this is a scalping opportunity (quick in-out)
fn is_scalping_opportunity(dataframe: &MarketDataFrame, token: &Token, current_price: f64) -> bool {
    // Check for quick scalping setups
    if dataframe.prices.len() < 20 {
        return false;
    }

    // Look for bounces off strong support/resistance
    if
        let Some((upper, middle, lower)) = calculate_bollinger_bands(
            &dataframe.prices,
            BB_PERIOD,
            BB_STD_DEV
        )
    {
        let near_lower = current_price <= lower * 1.01;
        let near_upper = current_price >= upper * 0.99;

        if near_lower || near_upper {
            // Check for reversal signals
            if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
                if (near_lower && rsi < 25.0) || (near_upper && rsi > 75.0) {
                    return true;
                }
            }
        }
    }

    // Check for quick momentum reversals
    if let Some(momentum) = calculate_momentum(&dataframe.prices, MOMENTUM_PERIOD) {
        let momentum_reversal = momentum.abs() > 5.0;
        if momentum_reversal {
            // Check if volume supports the reversal
            if dataframe.volumes.len() >= 3 {
                let current_vol = *dataframe.volumes.back().unwrap();
                let avg_vol = dataframe.volumes.iter().rev().take(5).sum::<f64>() / 5.0;

                if current_vol > avg_vol * 1.5 {
                    return true;
                }
            }
        }
    }

    false
}

// EXISTING FUNCTIONS CONTINUE...

/// Check if this is good timing for entry based on multiple factors
fn is_good_entry_timing(dataframe: &MarketDataFrame, token: &Token, current_price: f64) -> bool {
    // 1. Price should be near or below lower Bollinger Band
    if
        let Some((upper, middle, lower)) = calculate_bollinger_bands(
            &dataframe.prices,
            BB_PERIOD,
            BB_STD_DEV
        )
    {
        if current_price > lower * 1.02 {
            return false; // Only buy near lower BB
        }
    }

    // 2. RSI should be oversold but not falling rapidly
    if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
        if rsi > 35.0 {
            return false; // Must be oversold
        }

        // Check if RSI is stabilizing or starting to recover
        if dataframe.prices.len() >= RSI_PERIOD + 5 {
            let prev_prices: VecDeque<f64> = dataframe.prices
                .iter()
                .rev()
                .skip(2)
                .cloned()
                .collect();
            if let Some(prev_rsi) = calculate_rsi(&prev_prices, RSI_PERIOD) {
                if rsi < prev_rsi - 5.0 {
                    return false; // RSI still falling rapidly
                }
            }
        }
    }

    // 3. Volume should be present but not excessive (avoid pump/dump)
    if dataframe.volumes.len() >= 3 {
        let current_vol = *dataframe.volumes.back().unwrap();
        let avg_vol = dataframe.volumes.iter().rev().take(10).sum::<f64>() / 10.0;

        if current_vol < avg_vol * 0.5 || current_vol > avg_vol * 5.0 {
            return false; // Volume too low or too high
        }
    }

    // 4. Price should be stabilizing near the bottom
    if dataframe.prices.len() >= 6 {
        let recent_prices: Vec<f64> = dataframe.prices.iter().rev().take(6).cloned().collect();
        let price_volatility =
            recent_prices
                .iter()
                .zip(recent_prices.iter().skip(1))
                .map(|(a, b)| (a - b).abs() / b)
                .sum::<f64>() / 5.0;

        if price_volatility > 0.15 {
            return false; // Too much volatility
        }
    }

    // 5. Check for recent buying interest
    let recent_buy_pressure =
        (token.txns.h1.buys as f64) / ((token.txns.h1.buys + token.txns.h1.sells) as f64);
    if recent_buy_pressure < 0.6 {
        return false; // Need majority buying interest
    }

    true
}

/// Detect potential market manipulation patterns
fn is_potential_manipulation(
    dataframe: &MarketDataFrame,
    token: &Token,
    current_price: f64
) -> bool {
    // Check for excessive selling pressure
    if token.txns.h1.sells > token.txns.h1.buys * 2 {
        return true; // Too much selling activity
    }

    // Check for sudden volume spikes followed by price drops
    if dataframe.volumes.len() >= 5 && dataframe.prices.len() >= 5 {
        let recent_volumes: Vec<f64> = dataframe.volumes.iter().rev().take(5).cloned().collect();
        let recent_prices: Vec<f64> = dataframe.prices.iter().rev().take(5).cloned().collect();

        let current_vol = recent_volumes[0];
        let avg_vol = recent_volumes.iter().skip(1).sum::<f64>() / 4.0;

        // Large volume spike with price drop suggests dumping
        if current_vol > avg_vol * 3.0 {
            let price_change = (recent_prices[0] - recent_prices[4]) / recent_prices[4];
            if price_change < -0.05 {
                return true; // Volume spike with >5% drop
            }
        }
    }

    // Check for suspicious buy/sell patterns
    let total_buys = token.txns.h1.buys + token.txns.h6.buys + token.txns.h24.buys;
    let total_sells = token.txns.h1.sells + token.txns.h6.sells + token.txns.h24.sells;

    if (total_sells as f64) > (total_buys as f64) * 1.5 {
        return true; // Consistent selling pressure
    }

    // Check for price volatility that suggests manipulation
    if dataframe.prices.len() >= 10 {
        let prices: Vec<f64> = dataframe.prices.iter().rev().take(10).cloned().collect();
        let volatility = calculate_volatility(&prices.into_iter().collect(), 10).unwrap_or(0.0);

        if volatility > 0.25 {
            return true; // Extreme volatility suggests manipulation
        }
    }

    false
}

/// Calculate price trend from recent price data
fn calculate_price_trend(prices: &[f64]) -> f64 {
    if prices.len() < 3 {
        return 0.0;
    }

    let first_price = prices[0];
    let last_price = prices[prices.len() - 1];

    // Calculate percentage change
    (last_price - first_price) / first_price
}

/// Calculate Simple Moving Average
fn calculate_sma(prices: &VecDeque<f64>, period: usize) -> Option<f64> {
    if prices.len() < period {
        return None;
    }

    let sum: f64 = prices.iter().rev().take(period).sum();
    Some(sum / (period as f64))
}

/// Calculate Exponential Moving Average
fn calculate_ema(prices: &VecDeque<f64>, period: usize) -> Option<f64> {
    if prices.len() < period {
        return None;
    }

    let multiplier = 2.0 / ((period as f64) + 1.0);
    let mut ema = prices[0];

    for &price in prices.iter().skip(1) {
        ema = price * multiplier + ema * (1.0 - multiplier);
    }

    Some(ema)
}

/// Calculate RSI (Relative Strength Index)
fn calculate_rsi(prices: &VecDeque<f64>, period: usize) -> Option<f64> {
    if prices.len() < period + 1 {
        return None;
    }

    let mut gains = 0.0;
    let mut losses = 0.0;

    for i in 1..=period {
        let change = prices[i] - prices[i - 1];
        if change > 0.0 {
            gains += change;
        } else {
            losses -= change;
        }
    }

    let avg_gain = gains / (period as f64);
    let avg_loss = losses / (period as f64);

    if avg_loss == 0.0 {
        return Some(100.0);
    }

    let rs = avg_gain / avg_loss;
    Some(100.0 - 100.0 / (1.0 + rs))
}

/// Calculate Bollinger Bands (returns (upper, middle, lower))
fn calculate_bollinger_bands(
    prices: &VecDeque<f64>,
    period: usize,
    std_dev: f64
) -> Option<(f64, f64, f64)> {
    if prices.len() < period {
        return None;
    }

    let sma = calculate_sma(prices, period)?;
    let recent_prices: Vec<f64> = prices.iter().rev().take(period).cloned().collect();

    let variance =
        recent_prices
            .iter()
            .map(|&price| (price - sma).powi(2))
            .sum::<f64>() / (period as f64);

    let std = variance.sqrt();
    let upper = sma + std_dev * std;
    let lower = sma - std_dev * std;

    Some((upper, sma, lower))
}

/// Calculate VWAP (Volume Weighted Average Price)
fn calculate_vwap(prices: &VecDeque<f64>, volumes: &VecDeque<f64>, period: usize) -> Option<f64> {
    if prices.len() < period || volumes.len() < period {
        return None;
    }

    let mut price_volume_sum = 0.0;
    let mut volume_sum = 0.0;

    for i in 0..period {
        let price = prices[prices.len() - period + i];
        let volume = volumes[volumes.len() - period + i];
        price_volume_sum += price * volume;
        volume_sum += volume;
    }

    if volume_sum == 0.0 {
        return None;
    }

    Some(price_volume_sum / volume_sum)
}

/// Calculate momentum
fn calculate_momentum(prices: &VecDeque<f64>, period: usize) -> Option<f64> {
    if prices.len() < period + 1 {
        return None;
    }

    let current = *prices.back()?;
    let past = prices[prices.len() - period - 1];

    Some(((current - past) / past) * 100.0)
}

/// Calculate price volatility
fn calculate_volatility(prices: &VecDeque<f64>, period: usize) -> Option<f64> {
    if prices.len() < period {
        return None;
    }

    let recent_prices: Vec<f64> = prices.iter().rev().take(period).cloned().collect();
    let mean = recent_prices.iter().sum::<f64>() / (period as f64);

    let variance =
        recent_prices
            .iter()
            .map(|&price| (price - mean).powi(2))
            .sum::<f64>() / (period as f64);

    Some(variance.sqrt())
}

// TRADING SIGNAL ANALYSIS

/// Analyze market microstructure for entry signals
fn analyze_microstructure(
    dataframe: &MarketDataFrame,
    token: &Token,
    current_price: f64
) -> (f64, String) {
    let mut signal_strength = 0.0;
    let mut signals = Vec::new();

    if dataframe.prices.len() < 10 {
        return (0.0, "insufficient_data".to_string());
    }

    // Use real-time current_price for trading decisions, dataframe prices for technical analysis

    // 1. RSI Analysis
    if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
        if rsi < 30.0 {
            signal_strength += 0.25;
            signals.push("rsi_oversold");
        } else if rsi > 70.0 {
            signal_strength -= 0.25;
            signals.push("rsi_overbought");
        }
    }

    // 2. Bollinger Bands Analysis
    if
        let Some((upper, middle, lower)) = calculate_bollinger_bands(
            &dataframe.prices,
            BB_PERIOD,
            BB_STD_DEV
        )
    {
        if current_price <= lower {
            signal_strength += 0.3;
            signals.push("bb_lower");
        } else if current_price >= upper {
            signal_strength -= 0.3;
            signals.push("bb_upper");
        }

        // Bollinger Band squeeze detection
        let band_width = (upper - lower) / middle;
        if band_width < 0.05 {
            // Very tight bands
            signal_strength += 0.15;
            signals.push("bb_squeeze");
        }
    }

    // 3. VWAP Analysis
    if let Some(vwap) = calculate_vwap(&dataframe.prices, &dataframe.volumes, VWAP_PERIOD) {
        if current_price < vwap * 0.995 {
            // Price below VWAP with margin
            signal_strength += 0.2;
            signals.push("below_vwap");
        }
    }

    // 4. Momentum Analysis
    if let Some(momentum) = calculate_momentum(&dataframe.prices, MOMENTUM_PERIOD) {
        if momentum > 2.0 && momentum < 5.0 {
            // Positive but not excessive momentum
            signal_strength += 0.2;
            signals.push("positive_momentum");
        } else if momentum < -3.0 {
            signal_strength += 0.1; // Potential reversal
            signals.push("negative_momentum");
        }
    }

    // 5. Volume Analysis
    if dataframe.volumes.len() >= 3 {
        let current_vol = *dataframe.volumes.back().unwrap();
        let avg_vol = dataframe.volumes.iter().rev().take(10).sum::<f64>() / 10.0;

        if current_vol > avg_vol * 1.5 {
            signal_strength += 0.15;
            signals.push("volume_spike");
        }
    }

    // 6. Spread Analysis
    let spread_bps = (token.price_change.h1.abs() / current_price) * 10000.0;
    if spread_bps < MAX_SPREAD_BPS {
        signal_strength += 0.1;
        signals.push("tight_spread");
    }

    (signal_strength, signals.join(","))
}

/// Advanced multi-timeframe buy logic with position sizing
pub fn should_buy(
    dataframe: &MarketDataFrame,
    token: &Token,
    can_buy: bool,
    current_price: f64
) -> bool {
    if !can_buy {
        return false;
    }

    // ─── RUG CHECK SAFETY (CRITICAL FIRST CHECK) ───
    if !crate::dexscreener::is_safe_to_trade(token, false) {
        return false;
    }

    // Pre-filters for professional trading
    let volume_24h = token.volume.h24;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let price_change_1h = token.price_change.h1;
    let price_change_5m = token.price_change.m5;
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;

    // Basic quality filters (relaxed for more opportunities)
    if volume_24h < MIN_VOLUME_USD {
        return false;
    }

    if liquidity_sol < MIN_LIQUIDITY_SOL {
        return false;
    }

    // Less restrictive decline limits for more opportunities
    if price_change_1h < -15.0 || price_change_5m < -10.0 {
        return false;
    }

    // Basic activity requirements
    if buys_1h < 3 || sells_1h < 2 {
        return false;
    }

    // MULTI-TIMEFRAME ANALYSIS (CRITICAL)
    let (timeframe_ok, timeframe_analysis) = analyze_multi_timeframe(
        dataframe,
        token,
        current_price
    );
    if !timeframe_ok {
        return false;
    }

    // Determine position type and size
    let is_scalping = is_scalping_opportunity(dataframe, token, current_price);
    let position_type = if is_scalping { "SCALP" } else { "HOLD" };

    // Check for potential manipulation patterns
    if is_potential_manipulation(dataframe, token, current_price) {
        return false;
    }

    // Technical analysis (more flexible for more opportunities)
    let (signal_strength, signal_types) = analyze_microstructure(dataframe, token, current_price);

    // Lower signal strength requirement for more opportunities
    if signal_strength >= 0.4 {
        // Additional confirmation for non-scalping trades
        if !is_scalping {
            // Additional confirmation: RSI should be oversold or neutral
            if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
                if rsi > 50.0 {
                    return false; // Only buy when RSI is reasonable
                }
            }

            // Additional confirmation: Check if we're buying at good timing
            if !is_good_entry_timing(dataframe, token, current_price) {
                return false;
            }
        }

        let buy_sell_ratio = (buys_1h as f64) / ((sells_1h as f64) + 1.0);

        println!(
            "🎯 {} BUY SIGNAL {} | Strength: {:.2} | Signals: {} | Timeframe: {} | Vol24h: ${:.0} | Liq: {:.1}SOL | Buy/Sell: {:.2} | 1h: {:.1}% | 5m: {:.1}%",
            position_type,
            token.symbol,
            signal_strength,
            signal_types,
            timeframe_analysis,
            volume_24h,
            liquidity_sol,
            buy_sell_ratio,
            price_change_1h,
            price_change_5m
        );
        return true;
    }

    false
}

/// Professional DCA strategy with ultra-strict conditions
pub fn should_dca(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> bool {
    // For scalping strategy, we limit DCA usage severely
    if pos.dca_count >= MAX_DCA_COUNT {
        return false;
    }

    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

    // Much stricter DCA conditions - only when we're confident in recovery
    if drop_pct <= -12.0 && elapsed.num_minutes() >= 10 {
        // Require VERY strong technical confirmation
        let (signal_strength, _) = analyze_microstructure(dataframe, token, current_price);
        if signal_strength < 0.9 {
            return false; // Need extremely strong signal
        }

        // Check that selling pressure is decreasing
        let buy_sell_ratio = (token.txns.h1.buys as f64) / ((token.txns.h1.sells as f64) + 1.0);
        if buy_sell_ratio < 1.5 {
            return false; // Need strong buying pressure
        }

        // Ensure we're not in a continuous downtrend
        if dataframe.prices.len() >= 10 {
            let recent_prices: Vec<f64> = dataframe.prices.iter().rev().take(10).cloned().collect();
            let price_trend = calculate_price_trend(&recent_prices);

            if price_trend < -0.01 {
                return false; // Still in downtrend
            }
        }

        // Volume should be stable or increasing (not declining)
        if dataframe.volumes.len() >= 3 {
            let current_vol = *dataframe.volumes.back().unwrap();
            let prev_vol = dataframe.volumes.get(dataframe.volumes.len() - 2).unwrap_or(&0.0);

            if current_vol < prev_vol * 0.8 {
                return false; // Volume declining
            }
        }

        // Check RSI for extreme oversold with potential reversal
        if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
            if rsi > 20.0 {
                return false; // Need to be extremely oversold
            }
        }

        // Only DCA if price is significantly below our last entry
        if current_price >= pos.last_dca_price * 0.9 {
            return false; // Need at least 10% drop from last entry
        }

        println!(
            "🔄 ULTRA-CONSERVATIVE DCA {} | Drop: {:.1}% | Signal: {:.2} | Buy/Sell: {:.2}",
            token.symbol,
            drop_pct,
            signal_strength,
            buy_sell_ratio
        );
        return true;
    }

    false
}

/// Advanced long-term holding strategy with quick profit capture
pub fn should_sell(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> (bool, String) {
    // Use consistent profit calculation method
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - TRANSACTION_FEE_SOL;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };

    let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
    let held_duration = (Utc::now() - pos.open_time).num_seconds();
    let has_min_profit = profit_sol >= MIN_PROFIT_BEFORE_EXIT_SOL;

    // Determine if this was a scalping position
    let is_scalping_position = pos.sol_spent <= POSITION_SIZE_SCALPING * 2.0;

    // SCALPING POSITIONS - Quick exits
    if is_scalping_position {
        // Quick profit taking for scalping
        if profit_pct >= 0.5 && has_min_profit {
            return (true, format!("scalping_quick_profit({:.2}%)", profit_pct));
        }

        // Quick stop loss for scalping
        if profit_pct <= -3.0 || held_duration >= SCALPING_HOLD_TIME_SECONDS {
            return (
                true,
                format!("scalping_exit(profit:{:.2}%, held:{}s)", profit_pct, held_duration),
            );
        }

        return (false, "scalping_hold".to_string());
    }

    // LONG-TERM POSITIONS - Patient holding strategy

    // 1. MINIMUM HOLD TIME - Must hold for at least 30 minutes
    if held_duration < MIN_HOLD_TIME_SECONDS {
        return (
            false,
            format!("min_hold_time(held:{}s, min:{}s)", held_duration, MIN_HOLD_TIME_SECONDS),
        );
    }

    // 2. QUICK PROFIT CAPTURE - Take profits on spikes
    if profit_pct >= 5.0 && has_min_profit {
        // Check if we're overbought and should take quick profits
        if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
            if rsi >= 75.0 {
                return (
                    true,
                    format!("quick_profit_spike(profit:{:.2}%, rsi:{:.1})", profit_pct, rsi),
                );
            }
        }

        // Check for volume spike (possible pump)
        if dataframe.volumes.len() >= 3 {
            let current_vol = *dataframe.volumes.back().unwrap();
            let avg_vol = dataframe.volumes.iter().rev().take(10).sum::<f64>() / 10.0;

            if current_vol > avg_vol * 3.0 {
                return (
                    true,
                    format!(
                        "volume_spike_profit(profit:{:.2}%, vol_ratio:{:.1})",
                        profit_pct,
                        current_vol / avg_vol
                    ),
                );
            }
        }
    }

    // 3. PROGRESSIVE PROFIT TAKING - Scale out on big wins
    if profit_pct >= 20.0 && has_min_profit {
        // Take profits on exceptional gains
        let (signal_strength, _) = analyze_microstructure(dataframe, token, current_price);

        if signal_strength <= -0.3 {
            return (
                true,
                format!(
                    "progressive_profit_taking(profit:{:.2}%, signal:{:.2})",
                    profit_pct,
                    signal_strength
                ),
            );
        }
    }

    // 4. MAXIMUM HOLD TIME - Eventually exit even without profit
    if held_duration >= MAX_HOLD_TIME_SECONDS {
        return (
            true,
            format!("max_hold_time_exit(profit:{:.2}%, held:{}s)", profit_pct, held_duration),
        );
    }

    // 5. STOP LOSS - More generous than before
    let stop_loss_pct = if held_duration < 3600 {
        -40.0 // Very generous first hour
    } else if held_duration < 7200 {
        -30.0 // Generous first 2 hours
    } else if held_duration < 21600 {
        -25.0 // Standard after 2 hours
    } else {
        -20.0 // Tighter after 6 hours
    };

    if profit_pct <= stop_loss_pct {
        return (
            true,
            format!(
                "progressive_stop_loss(profit:{:.2}%, threshold:{:.1}%)",
                profit_pct,
                stop_loss_pct
            ),
        );
    }

    // 6. MARKET DETERIORATION - Exit if fundamentals break down
    let liquidity_total = token.liquidity.base + token.liquidity.quote;
    if liquidity_total < MIN_LIQUIDITY_SOL * 0.3 {
        return (true, format!("liquidity_collapse(liq:{:.1}SOL)", liquidity_total));
    }

    // 7. SEVERE TOKEN COLLAPSE
    if token.price_change.h24 <= -60.0 {
        return (true, format!("token_collapse_24h({:.1}%)", token.price_change.h24));
    }

    // 8. MULTI-TIMEFRAME EXIT SIGNALS
    let (should_exit, exit_reason) = check_multi_timeframe_exit(
        dataframe,
        token,
        current_price,
        profit_pct
    );
    if should_exit {
        return (true, format!("multi_timeframe_exit({})", exit_reason));
    }

    // 9. TRAILING STOP for large profits
    if profit_pct >= 10.0 && has_min_profit {
        let trailing_stop_pct = if profit_pct >= 100.0 {
            -15.0 // 15% trailing stop for 100%+ profits
        } else if profit_pct >= 50.0 {
            -10.0 // 10% trailing stop for 50%+ profits
        } else if profit_pct >= 20.0 {
            -8.0 // 8% trailing stop for 20%+ profits
        } else {
            -5.0 // 5% trailing stop for 10%+ profits
        };

        if drop_from_peak <= trailing_stop_pct {
            return (
                true,
                format!(
                    "trailing_stop(profit:{:.2}%, drop:{:.2}%, trail:{:.1}%)",
                    profit_pct,
                    drop_from_peak,
                    trailing_stop_pct
                ),
            );
        }
    }

    // Default: Hold the position
    (false, format!("holding(profit:{:.2}%, held:{}s)", profit_pct, held_duration))
}

/// Check multi-timeframe signals for exit
fn check_multi_timeframe_exit(
    dataframe: &MarketDataFrame,
    token: &Token,
    current_price: f64,
    profit_pct: f64
) -> (bool, String) {
    // Only use multi-timeframe exit for positions in profit
    if profit_pct < 0.0 {
        return (false, "negative_profit".to_string());
    }

    // Check if we're overbought across multiple timeframes
    let rsi_5m = calculate_rsi_for_timeframe(dataframe, 5);
    let rsi_15m = calculate_rsi_for_timeframe(dataframe, 15);
    let rsi_1h = calculate_rsi_for_timeframe(dataframe, 60);

    if let (Some(rsi_5), Some(rsi_15), Some(rsi_1h)) = (rsi_5m, rsi_15m, rsi_1h) {
        // All timeframes overbought and we have profit
        if rsi_5 > 75.0 && rsi_15 > 70.0 && rsi_1h > 65.0 && profit_pct >= 2.0 {
            return (
                true,
                format!(
                    "overbought_all_timeframes(5m:{:.1},15m:{:.1},1h:{:.1})",
                    rsi_5,
                    rsi_15,
                    rsi_1h
                ),
            );
        }
    }

    // Check for trend reversal across timeframes
    let trend_5m = calculate_trend_for_timeframe(dataframe, 5);
    let trend_15m = calculate_trend_for_timeframe(dataframe, 15);
    let trend_1h = calculate_trend_for_timeframe(dataframe, 60);

    // All timeframes turning negative
    if trend_5m < -0.03 && trend_15m < -0.02 && trend_1h < -0.01 && profit_pct >= 1.0 {
        return (true, "trend_reversal_all_timeframes".to_string());
    }

    (false, "no_exit_signal".to_string())
}

/// Get position size based on opportunity type
pub fn get_position_size(dataframe: &MarketDataFrame, token: &Token, current_price: f64) -> f64 {
    let is_scalping = is_scalping_opportunity(dataframe, token, current_price);

    if is_scalping {
        POSITION_SIZE_SCALPING
    } else {
        TRADE_SIZE_SOL
    }
}

/// Calculate position size based on volatility and risk
pub fn calculate_position_size(dataframe: &MarketDataFrame, token: &Token, base_size: f64) -> f64 {
    if let Some(volatility) = calculate_volatility(&dataframe.prices, 10) {
        let vol_adjustment = if volatility > 0.1 {
            0.7 // Reduce size for high volatility
        } else if volatility > 0.05 {
            0.85 // Slightly reduce for medium volatility
        } else {
            1.0 // Full size for low volatility
        };

        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        let liquidity_adjustment = if liquidity_sol > 50.0 {
            1.0
        } else if liquidity_sol > 20.0 {
            0.8
        } else {
            0.6 // Reduce size for low liquidity
        };

        return base_size * vol_adjustment * liquidity_adjustment;
    }

    base_size * 0.8 // Conservative default
}

// LEARNING SYSTEM STRUCTURES AND FUNCTIONS

#[derive(Debug, Clone)]
pub struct TradeOutcome {
    pub symbol: String,
    pub entry_price: f64,
    pub exit_price: f64,
    pub profit_pct: f64,
    pub hold_time_seconds: i64,
    pub entry_signals: String,
    pub exit_reason: String,
    pub rsi_at_entry: Option<f64>,
    pub rsi_at_exit: Option<f64>,
    pub volume_ratio_at_entry: f64,
    pub was_scalping: bool,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl TradeOutcome {
    pub fn new(
        symbol: String,
        entry_price: f64,
        exit_price: f64,
        profit_pct: f64,
        hold_time_seconds: i64,
        entry_signals: String,
        exit_reason: String,
        rsi_at_entry: Option<f64>,
        rsi_at_exit: Option<f64>,
        volume_ratio_at_entry: f64,
        was_scalping: bool
    ) -> Self {
        Self {
            symbol,
            entry_price,
            exit_price,
            profit_pct,
            hold_time_seconds,
            entry_signals,
            exit_reason,
            rsi_at_entry,
            rsi_at_exit,
            volume_ratio_at_entry,
            was_scalping,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Analyze historical trades to improve future decisions
pub fn analyze_trade_performance(trade_history: &[TradeOutcome]) -> (f64, String) {
    if trade_history.len() < MIN_TRADES_FOR_LEARNING {
        return (0.5, "insufficient_data".to_string());
    }

    let recent_trades = &trade_history[trade_history.len().saturating_sub(100)..];
    let total_trades = recent_trades.len() as f64;
    let winning_trades = recent_trades
        .iter()
        .filter(|t| t.profit_pct > 0.0)
        .count() as f64;
    let success_rate = winning_trades / total_trades;

    let mut insights = Vec::new();

    // Analyze RSI entry patterns
    let rsi_winners: Vec<f64> = recent_trades
        .iter()
        .filter(|t| t.profit_pct > 0.0 && t.rsi_at_entry.is_some())
        .map(|t| t.rsi_at_entry.unwrap())
        .collect();

    let rsi_losers: Vec<f64> = recent_trades
        .iter()
        .filter(|t| t.profit_pct <= 0.0 && t.rsi_at_entry.is_some())
        .map(|t| t.rsi_at_entry.unwrap())
        .collect();

    if !rsi_winners.is_empty() && !rsi_losers.is_empty() {
        let avg_rsi_winners = rsi_winners.iter().sum::<f64>() / (rsi_winners.len() as f64);
        let avg_rsi_losers = rsi_losers.iter().sum::<f64>() / (rsi_losers.len() as f64);

        insights.push(
            format!("rsi_winner_avg:{:.1}_loser_avg:{:.1}", avg_rsi_winners, avg_rsi_losers)
        );
    }

    // Analyze hold time patterns
    let avg_hold_time_winners =
        (
            recent_trades
                .iter()
                .filter(|t| t.profit_pct > 0.0)
                .map(|t| t.hold_time_seconds)
                .sum::<i64>() as f64
        ) / winning_trades;

    let avg_hold_time_losers =
        (
            recent_trades
                .iter()
                .filter(|t| t.profit_pct <= 0.0)
                .map(|t| t.hold_time_seconds)
                .sum::<i64>() as f64
        ) /
        (total_trades - winning_trades);

    insights.push(
        format!(
            "hold_time_winner_avg:{:.0}s_loser_avg:{:.0}s",
            avg_hold_time_winners,
            avg_hold_time_losers
        )
    );

    // Analyze scalping vs holding performance
    let scalping_trades: Vec<&TradeOutcome> = recent_trades
        .iter()
        .filter(|t| t.was_scalping)
        .collect();
    let holding_trades: Vec<&TradeOutcome> = recent_trades
        .iter()
        .filter(|t| !t.was_scalping)
        .collect();

    if !scalping_trades.is_empty() && !holding_trades.is_empty() {
        let scalping_success =
            (
                scalping_trades
                    .iter()
                    .filter(|t| t.profit_pct > 0.0)
                    .count() as f64
            ) / (scalping_trades.len() as f64);
        let holding_success =
            (
                holding_trades
                    .iter()
                    .filter(|t| t.profit_pct > 0.0)
                    .count() as f64
            ) / (holding_trades.len() as f64);

        insights.push(
            format!(
                "scalping_success:{:.2}_holding_success:{:.2}",
                scalping_success,
                holding_success
            )
        );
    }

    (success_rate, insights.join(","))
}

/// Adjust signal strength requirements based on learning
pub fn get_adaptive_signal_strength(trade_history: &[TradeOutcome]) -> f64 {
    let (success_rate, _) = analyze_trade_performance(trade_history);

    if success_rate < 0.3 {
        0.7 // Require higher signal strength if success rate is low
    } else if success_rate < 0.5 {
        0.5 // Standard requirement
    } else {
        0.4 // Lower requirement if doing well
    }
}

/// Get recommended RSI threshold based on learning
pub fn get_adaptive_rsi_threshold(trade_history: &[TradeOutcome]) -> f64 {
    if trade_history.len() < MIN_TRADES_FOR_LEARNING {
        return 35.0; // Default threshold
    }

    let recent_trades = &trade_history[trade_history.len().saturating_sub(100)..];

    // Find the RSI range that produces the best results
    let winning_rsi: Vec<f64> = recent_trades
        .iter()
        .filter(|t| t.profit_pct > 5.0 && t.rsi_at_entry.is_some())
        .map(|t| t.rsi_at_entry.unwrap())
        .collect();

    if winning_rsi.is_empty() {
        return 35.0;
    }

    let avg_winning_rsi = winning_rsi.iter().sum::<f64>() / (winning_rsi.len() as f64);

    // Adjust threshold based on what's been working
    if avg_winning_rsi < 25.0 {
        20.0 // Very oversold has been working
    } else if avg_winning_rsi < 35.0 {
        30.0 // Oversold has been working
    } else {
        40.0 // Less oversold has been working
    }
}

/// Get recommended hold time based on learning
pub fn get_adaptive_min_hold_time(trade_history: &[TradeOutcome]) -> i64 {
    if trade_history.len() < MIN_TRADES_FOR_LEARNING {
        return MIN_HOLD_TIME_SECONDS;
    }

    let recent_trades = &trade_history[trade_history.len().saturating_sub(100)..];

    // Find the hold time that produces the best results
    let profitable_hold_times: Vec<i64> = recent_trades
        .iter()
        .filter(|t| t.profit_pct > 3.0)
        .map(|t| t.hold_time_seconds)
        .collect();

    if profitable_hold_times.is_empty() {
        return MIN_HOLD_TIME_SECONDS;
    }

    let avg_profitable_hold_time =
        profitable_hold_times.iter().sum::<i64>() / (profitable_hold_times.len() as i64);

    // Adjust minimum hold time based on what's been working
    if avg_profitable_hold_time < 600 {
        600 // 10 minutes minimum
    } else if avg_profitable_hold_time < 1800 {
        1800 // 30 minutes minimum
    } else {
        3600 // 1 hour minimum
    }
}

/// Advanced risk assessment for the token
pub fn assess_token_risk(token: &Token, dataframe: &MarketDataFrame) -> (f64, String) {
    let mut risk_score: f64 = 0.0;
    let mut risk_factors = Vec::new();

    // Liquidity risk
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    if liquidity_sol < 5.0 {
        risk_score += 0.4;
        risk_factors.push("low_liquidity");
    } else if liquidity_sol < 15.0 {
        risk_score += 0.2;
        risk_factors.push("medium_liquidity");
    }

    // Volume consistency
    if token.volume.h1 < token.volume.h24 * 0.02 {
        // Less than 2% of daily volume in last hour
        risk_score += 0.3;
        risk_factors.push("volume_declining");
    }

    // Price stability
    if token.price_change.h1.abs() > 20.0 {
        risk_score += 0.3;
        risk_factors.push("high_volatility");
    }

    // Transaction pattern
    let buy_sell_ratio = (token.txns.h1.buys as f64) / ((token.txns.h1.sells as f64) + 1.0);
    if buy_sell_ratio < 0.5 {
        risk_score += 0.25;
        risk_factors.push("sell_pressure");
    }

    // Technical risk from price action
    if let Some(volatility) = calculate_volatility(&dataframe.prices, 10) {
        if volatility > 0.15 {
            risk_score += 0.2;
            risk_factors.push("price_volatility");
        }
    }

    (risk_score.min(1.0), risk_factors.join(","))
}

/// Get minimum profit requirement in SOL for a position to cover transaction fees
pub fn get_min_profit_requirement() -> f64 {
    MIN_PROFIT_BEFORE_EXIT_SOL
}

/// Check if a position has achieved minimum profitable exit threshold
pub fn has_achieved_min_profit(pos: &Position, current_price: f64) -> (bool, f64) {
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - TRANSACTION_FEE_SOL;
    let has_min_profit = profit_sol >= MIN_PROFIT_BEFORE_EXIT_SOL;
    (has_min_profit, profit_sol)
}

/// Calculate break-even price that covers all fees for a position
pub fn calculate_breakeven_price(pos: &Position) -> f64 {
    // Entry price + minimum profit requirement per token
    pos.entry_price + MIN_PROFIT_BEFORE_EXIT_SOL / pos.token_amount
}

/// Calculate dynamic profit target based on market conditions and position performance
fn calculate_dynamic_profit_target(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> f64 {
    // Use consistent profit calculation method
    // Account for sell transaction fee to make profit calculation more realistic
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - TRANSACTION_FEE_SOL;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };
    let held_duration_minutes = (Utc::now() - pos.open_time).num_minutes();

    // Base profit target depends on current profit level
    let base_target = if profit_pct < PROFIT_TIER_1 {
        PROFIT_TIER_1 // Target 1% for quick scalps
    } else if profit_pct < PROFIT_TIER_2 {
        PROFIT_TIER_2 // Target 3% for small profits
    } else if profit_pct < PROFIT_TIER_3 {
        PROFIT_TIER_3 // Target 5% for medium profits
    } else if profit_pct < PROFIT_TIER_4 {
        PROFIT_TIER_4 // Target 10% for good profits
    } else if profit_pct < PROFIT_TIER_5 {
        PROFIT_TIER_5 // Target 20% for great profits
    } else if profit_pct < PROFIT_TIER_6 {
        PROFIT_TIER_6 // Target 50% for excellent profits
    } else if profit_pct < PROFIT_TIER_7 {
        PROFIT_TIER_7 // Target 100% for exceptional profits
    } else if profit_pct < PROFIT_TIER_8 {
        PROFIT_TIER_8 // Target 200% for extraordinary profits
    } else {
        PROFIT_TIER_9 // Target 500% maximum
    };

    // Market condition adjustments
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let volume_24h = token.volume.h24;

    // Reduce target if liquidity is low (harder to exit at higher prices)
    let liquidity_multiplier = if liquidity_sol < 20.0 {
        0.7 // Take profits earlier with low liquidity
    } else if liquidity_sol < 50.0 {
        0.85
    } else {
        1.0
    };

    // Adjust based on volume strength
    let volume_multiplier = if volume_24h > 50000.0 {
        1.2 // Can hold for higher profits with strong volume
    } else if volume_24h > 20000.0 {
        1.0
    } else {
        0.8 // Take profits earlier with weak volume
    };

    // Time-based adjustment (take profits sooner if held too long)
    let time_multiplier = if held_duration_minutes > 20 {
        0.8 // Reduce target after 20 minutes
    } else if held_duration_minutes > 10 {
        0.9 // Slightly reduce after 10 minutes
    } else {
        1.0
    };

    base_target * liquidity_multiplier * volume_multiplier * time_multiplier
}

/// Calculate tiered trailing stop based on profit level
fn calculate_tiered_trailing_stop(profit_pct: f64) -> f64 {
    if profit_pct < PROFIT_TIER_1 {
        f64::NEG_INFINITY // No trailing stop below 1%
    } else if profit_pct <= PROFIT_TIER_2 {
        -0.6 // Relaxed 0.6% trailing stop for 1-3% profits
    } else if profit_pct <= PROFIT_TIER_3 {
        -0.8 // 0.8% trailing stop for 3-5% profits
    } else if profit_pct <= PROFIT_TIER_4 {
        -1.2 // 1.2% trailing stop for 5-10% profits
    } else if profit_pct <= PROFIT_TIER_5 {
        -1.8 // 1.8% trailing stop for 10-20% profits
    } else if profit_pct <= PROFIT_TIER_6 {
        -2.5 // 2.5% trailing stop for 20-50% profits
    } else if profit_pct <= PROFIT_TIER_7 {
        -3.5 // 3.5% trailing stop for 50-100% profits
    } else if profit_pct <= PROFIT_TIER_8 {
        -5.5 // 5.5% trailing stop for 100-200% profits
    } else {
        -8.0 // 8% trailing stop for 200%+ profits (unchanged for massive profits)
    }
}

/// Check if we should take profits at current level
fn should_take_profit_at_tier(profit_pct: f64, target_pct: f64, signal_strength: f64) -> bool {
    // Take profits if we've hit target and have some confirmation
    if profit_pct >= target_pct {
        // For small profits (1-5%), require strong technical confirmation
        if target_pct <= PROFIT_TIER_3 {
            return signal_strength <= -0.3; // Negative signal strength (overbought conditions)
        } else if
            // For medium profits (5-20%), require moderate confirmation
            target_pct <= PROFIT_TIER_5
        {
            return signal_strength <= -0.2;
        } else {
            // For large profits (20%+), take them with minimal confirmation
            return signal_strength <= 0.0;
        }
    }
    false
}

/// Get the current profit tier name for a position
pub fn get_profit_tier_name(profit_pct: f64) -> &'static str {
    if profit_pct >= PROFIT_TIER_9 {
        "TIER_9_MAXIMUM_500PCT+"
    } else if profit_pct >= PROFIT_TIER_8 {
        "TIER_8_EXTRAORDINARY_200PCT+"
    } else if profit_pct >= PROFIT_TIER_7 {
        "TIER_7_EXCEPTIONAL_100PCT+"
    } else if profit_pct >= PROFIT_TIER_6 {
        "TIER_6_EXCELLENT_50PCT+"
    } else if profit_pct >= PROFIT_TIER_5 {
        "TIER_5_GREAT_20PCT+"
    } else if profit_pct >= PROFIT_TIER_4 {
        "TIER_4_GOOD_10PCT+"
    } else if profit_pct >= PROFIT_TIER_3 {
        "TIER_3_MEDIUM_5PCT+"
    } else if profit_pct >= PROFIT_TIER_2 {
        "TIER_2_SMALL_3PCT+"
    } else if profit_pct >= PROFIT_TIER_1 {
        "TIER_1_QUICK_1PCT+"
    } else if profit_pct > 0.0 {
        "TIER_0_MINIMAL_PROFIT"
    } else {
        "NEGATIVE_LOSS"
    }
}

/// Calculate recommended action for current profit level
pub fn get_profit_recommendation(
    pos: &Position,
    current_price: f64,
    dataframe: &MarketDataFrame,
    token: &Token
) -> String {
    // Use consistent profit calculation method
    // Account for sell transaction fee to make profit calculation more realistic
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - TRANSACTION_FEE_SOL;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };

    let (has_min_profit, _) = has_achieved_min_profit(pos, current_price);
    let (signal_strength, _) = analyze_microstructure(dataframe, token, current_price);
    let tier_name = get_profit_tier_name(profit_pct);
    let dynamic_target = calculate_dynamic_profit_target(dataframe, token, pos, current_price);
    let trailing_stop = calculate_tiered_trailing_stop(profit_pct);

    format!(
        "{}(profit:{:.2}%, {:.6}SOL, target:{:.1}%, trail:{:.1}%, signal:{:.2}, min_profit:{})",
        tier_name,
        profit_pct,
        profit_sol,
        dynamic_target,
        trailing_stop,
        signal_strength,
        has_min_profit
    )
}

/// Check if we should avoid early exit when we bought on oversold conditions
fn should_avoid_early_exit(
    dataframe: &MarketDataFrame,
    held_duration: i64,
    profit_pct: f64,
    pos: &Position
) -> bool {
    // If we're making decent profit, don't avoid exit
    if profit_pct > 2.0 {
        return false;
    }

    // If we're in severe loss, don't avoid exit
    if profit_pct <= -20.0 {
        return false;
    }

    // For small losses, be more patient initially
    if held_duration < 600 && profit_pct > -15.0 {
        return true; // Wait at least 10 minutes for small losses
    }

    // Check if we're still in oversold territory and should wait for recovery
    if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
        // If RSI is still oversold and we haven't held too long, wait
        if rsi < 40.0 && profit_pct > -18.0 && held_duration < 1200 {
            return true; // Wait up to 20 minutes in oversold
        }
    }

    // Check if there's recent volume increase suggesting potential recovery
    if dataframe.volumes.len() >= 3 {
        let current_vol = *dataframe.volumes.back().unwrap();
        let avg_vol = dataframe.volumes.iter().rev().take(5).sum::<f64>() / 5.0;

        // If volume is increasing and we're not in severe loss, wait
        if current_vol > avg_vol * 1.3 && profit_pct > -12.0 && held_duration < 900 {
            return true; // Wait 15 minutes on volume increase
        }
    }

    // Check if we DCA'd and should give it more time
    if pos.dca_count > 0 && profit_pct > -25.0 && held_duration < 1800 {
        return true; // Wait longer after DCA
    }

    false
}

// TRADING STRATEGY FUNCTIONS
//
// IMPORTANT: All trading functions (should_buy, should_sell, should_dca) must always
// receive current_price from batch_prices_from_pools for real-time trading decisions.
// Technical analysis functions use dataframe prices for historical indicators (RSI, BB, etc.)
// while using real-time current_price for current price comparisons and signal evaluation//
