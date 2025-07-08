#![allow(warnings)]
use crate::prelude::*;

// PROFESSIONAL HIGH-FREQUENCY TRADING CONSTANTS
pub const TRADE_SIZE_SOL: f64 = 0.003; // Increased size for better fee coverage
pub const MAX_OPEN_POSITIONS: usize = 3; // Fewer positions for better risk management
pub const MAX_DCA_COUNT: u8 = 2; // Increased DCA allowance for recovery
pub const TRANSACTION_FEE_SOL: f64 = 0.0001; // More realistic fee estimate including slippage
pub const POSITIONS_CHECK_TIME: u64 = 2; // Check every 2 seconds
pub const POSITIONS_PRINT_TIME: u64 = 5; // Print every 5 seconds
pub const PRICE_HISTORY_CAP: usize = 120; // Larger history for better analysis
pub const SLIPPAGE_BPS: f64 = 0.5; // Slightly increased for better execution
pub const FEE_RATE: f64 = 0.0001; // More realistic total fee rate
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
pub const QUICK_PROFIT_TARGET: f64 = 1.0; // Start taking profits at 1%
pub const MAX_PROFIT_TARGET: f64 = 500.0; // Maximum profit target 500%
pub const RAPID_STOP_LOSS: f64 = -3.0; // Relaxed 3% stop loss for crypto volatility
pub const MAX_TRADE_DURATION_SEC: i64 = 3600; // Max 60 minutes for better opportunities
pub const MIN_PROFIT_BEFORE_EXIT_SOL: f64 = FEE_RATE * 1.5; // Reduced minimum profit requirement

// PROFIT TAKING TIERS FOR 1% TO 500% RANGE
pub const PROFIT_TIER_1: f64 = 1.0; // 1% - quick scalp
pub const PROFIT_TIER_2: f64 = 3.0; // 3% - small profit
pub const PROFIT_TIER_3: f64 = 5.0; // 5% - medium profit
pub const PROFIT_TIER_4: f64 = 10.0; // 10% - good profit
pub const PROFIT_TIER_5: f64 = 20.0; // 20% - great profit
pub const PROFIT_TIER_6: f64 = 50.0; // 50% - excellent profit
pub const PROFIT_TIER_7: f64 = 100.0; // 100% - exceptional profit
pub const PROFIT_TIER_8: f64 = 200.0; // 200% - extraordinary profit
pub const PROFIT_TIER_9: f64 = 500.0; // 500% - maximum target

// TECHNICAL ANALYSIS FUNCTIONS

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

/// Professional high-frequency buy logic with multiple confirmations
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
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;

    // Strict quality filters
    if volume_24h < MIN_VOLUME_USD {
        return false;
    }

    if liquidity_sol < MIN_LIQUIDITY_SOL {
        return false;
    }

    // Avoid tokens in severe decline
    if price_change_1h < -15.0 {
        return false;
    }

    // Require reasonable trading activity
    if buys_1h < 5 || sells_1h < 3 {
        return false;
    }

    // Buy/sell ratio analysis (avoid dump scenarios)
    let buy_sell_ratio = (buys_1h as f64) / ((sells_1h as f64) + 1.0);
    if buy_sell_ratio < 0.7 {
        return false;
    }

    // Technical analysis
    let (signal_strength, signal_types) = analyze_microstructure(dataframe, token, current_price);

    // Require strong signal for entry
    if signal_strength >= 0.6 {
        println!(
            "� PROFESSIONAL BUY SIGNAL {} | Strength: {:.2} | Signals: {} | Vol24h: ${:.0} | Liq: {:.1}SOL | Buy/Sell: {:.2}",
            token.symbol,
            signal_strength,
            signal_types,
            volume_24h,
            liquidity_sol,
            buy_sell_ratio
        );
        return true;
    }

    false
}

/// Professional DCA strategy with strict conditions
pub fn should_dca(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> bool {
    // For scalping strategy, we limit DCA usage
    if pos.dca_count >= MAX_DCA_COUNT {
        return false;
    }

    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

    // Only DCA if we have strong technical confirmation
    let (signal_strength, _) = analyze_microstructure(dataframe, token, current_price);

    // Require oversold conditions and strong technical signal (more conservative)
    if drop_pct <= -8.0 && signal_strength >= 0.8 && elapsed.num_minutes() >= 5 {
        // Additional safety checks
        let volume_declining = if dataframe.volumes.len() >= 2 {
            let latest_vol = dataframe.volumes.back().unwrap_or(&0.0);
            let prev_vol = dataframe.volumes.get(dataframe.volumes.len() - 2).unwrap_or(&0.0);
            *latest_vol < *prev_vol * 1.5 // Allow some volume increase but not excessive
        } else {
            false
        };

        // Check if we're still in a downtrend (more conservative RSI threshold)
        if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
            return rsi < 25.0 && volume_declining && current_price < pos.last_dca_price * 0.95;
        }
    }

    false
}

/// Professional sell strategy with multiple exit conditions
pub fn should_sell(
    dataframe: &MarketDataFrame,
    token: &Token,
    pos: &Position,
    current_price: f64
) -> (bool, String) {
    // Use consistent profit calculation method (same as web server and helpers)
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };

    let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
    let held_duration = (Utc::now() - pos.open_time).num_seconds();

    let has_min_profit = profit_sol >= MIN_PROFIT_BEFORE_EXIT_SOL;

    // Get technical analysis for profit-taking decisions
    let (signal_strength, _) = analyze_microstructure(dataframe, token, current_price);

    // Calculate dynamic profit target based on market conditions
    let dynamic_target = calculate_dynamic_profit_target(dataframe, token, pos, current_price);

    // 1. TIERED PROFIT TAKING SYSTEM (1% to 500%)
    if has_min_profit && should_take_profit_at_tier(profit_pct, dynamic_target, signal_strength) {
        let tier_name = if profit_pct >= PROFIT_TIER_9 {
            "MAXIMUM_500PCT"
        } else if profit_pct >= PROFIT_TIER_8 {
            "EXTRAORDINARY_200PCT"
        } else if profit_pct >= PROFIT_TIER_7 {
            "EXCEPTIONAL_100PCT"
        } else if profit_pct >= PROFIT_TIER_6 {
            "EXCELLENT_50PCT"
        } else if profit_pct >= PROFIT_TIER_5 {
            "GREAT_20PCT"
        } else if profit_pct >= PROFIT_TIER_4 {
            "GOOD_10PCT"
        } else if profit_pct >= PROFIT_TIER_3 {
            "MEDIUM_5PCT"
        } else if profit_pct >= PROFIT_TIER_2 {
            "SMALL_3PCT"
        } else {
            "QUICK_1PCT"
        };

        return (
            true,
            format!(
                "tiered_profit_{}({:.2}%, {:.6}SOL, target:{:.1}%)",
                tier_name,
                profit_pct,
                profit_sol,
                dynamic_target
            ),
        );
    }

    // 2. DYNAMIC TIME-BASED EXIT - Longer holding for larger profits
    let max_duration = if profit_pct >= PROFIT_TIER_7 {
        7200 // 2 hours for 100%+ profits
    } else if profit_pct >= PROFIT_TIER_6 {
        5400 // 90 minutes for 50%+ profits
    } else if profit_pct >= PROFIT_TIER_5 {
        3600 // 60 minutes for 20%+ profits
    } else if profit_pct >= PROFIT_TIER_4 {
        2700 // 45 minutes for 10%+ profits
    } else if profit_pct >= PROFIT_TIER_2 {
        1800 // 30 minutes for 3%+ profits
    } else if profit_pct > 0.0 {
        1200 // 20 minutes for any positive profit
    } else {
        900 // 15 minutes for break-even/losses
    };

    if held_duration >= max_duration {
        return (
            true,
            format!("dynamic_time_exit({:.0}s, profit:{:.2}%)", held_duration, profit_pct),
        );
    }

    // 3. TIGHT STOP LOSS - Rapid exit on losses
    if profit_pct <= RAPID_STOP_LOSS {
        return (true, format!("rapid_stop_loss({:.2}%)", profit_pct));
    }

    // 4. TECHNICAL ANALYSIS BASED EXITS - Scaled by profit level
    if let Some(rsi) = calculate_rsi(&dataframe.prices, RSI_PERIOD) {
        // More conservative exits to avoid selling profitable positions too early
        let rsi_exit_threshold = if profit_pct >= PROFIT_TIER_6 {
            78.0 // Exit at RSI 78+ for 50%+ profits
        } else if profit_pct >= PROFIT_TIER_4 {
            82.0 // Exit at RSI 82+ for 10%+ profits
        } else if profit_pct >= PROFIT_TIER_2 {
            85.0 // Exit at RSI 85+ for 3%+ profits
        } else {
            88.0 // Exit at RSI 88+ for smaller profits (very overbought)
        };

        let min_profit_for_rsi_exit = if profit_pct >= PROFIT_TIER_3 {
            0.0 // Any profit above 5%
        } else if profit_pct >= PROFIT_TIER_1 {
            0.3 // Require 0.3% minimum for 1%+ profits
        } else {
            0.8 // Require 0.8% minimum for smaller profits
        };

        // Exit on overbought conditions with appropriate profit levels
        if rsi >= rsi_exit_threshold && profit_pct > min_profit_for_rsi_exit && has_min_profit {
            return (
                true,
                format!(
                    "rsi_overbought_exit(rsi:{:.1}, profit:{:.2}%, {:.6}SOL, threshold:{})",
                    rsi,
                    profit_pct,
                    profit_sol,
                    rsi_exit_threshold
                ),
            );
        }

        // Exit on oversold if in significant loss (more conservative)
        if rsi <= 15.0 && profit_pct <= -4.0 {
            return (true, format!("rsi_oversold_cutloss(rsi:{:.1}, loss:{:.2}%)", rsi, profit_pct));
        }
    }

    // 5. BOLLINGER BANDS EXIT
    if
        let Some((upper, _, lower)) = calculate_bollinger_bands(
            &dataframe.prices,
            BB_PERIOD,
            BB_STD_DEV
        )
    {
        // Exit if price hits upper band with profit (only if min profit met)
        if current_price >= upper && profit_pct > 0.8 && has_min_profit {
            return (
                true,
                format!("bb_upper_exit(profit:{:.2}%, {:.6}SOL)", profit_pct, profit_sol),
            );
        }

        // Exit if price breaks lower band significantly (more conservative)
        if current_price <= lower * 0.99 && profit_pct <= -3.5 {
            return (true, format!("bb_lower_breakdown(loss:{:.2}%)", profit_pct));
        }
    }

    // 6. VOLUME-BASED EXITS
    if dataframe.volumes.len() >= 3 {
        let current_vol = *dataframe.volumes.back().unwrap();
        let avg_vol = dataframe.volumes.iter().rev().take(5).sum::<f64>() / 5.0;

        // Exit on volume spike with profit (possible distribution) - only if min profit met
        if current_vol > avg_vol * 4.0 && profit_pct > 1.5 && has_min_profit {
            return (
                true,
                format!(
                    "volume_spike_exit(vol_ratio:{:.1}, profit:{:.2}%, {:.6}SOL)",
                    current_vol / avg_vol,
                    profit_pct,
                    profit_sol
                ),
            );
        }

        // Exit on volume death (liquidity drying up) - more conservative
        if current_vol < avg_vol * 0.2 && profit_pct <= -1.0 {
            return (true, format!("volume_death_exit(vol_ratio:{:.1})", current_vol / avg_vol));
        }
    }

    // 7. MOMENTUM REVERSAL EXIT
    if let Some(momentum) = calculate_momentum(&dataframe.prices, MOMENTUM_PERIOD) {
        // Exit on negative momentum in profit (only if min profit met) - more conservative
        if momentum <= -3.5 && profit_pct > 1.0 && has_min_profit {
            return (
                true,
                format!(
                    "momentum_reversal(momentum:{:.1}, profit:{:.2}%, {:.6}SOL)",
                    momentum,
                    profit_pct,
                    profit_sol
                ),
            );
        }
    }

    // 8. MARKET CONDITION EXITS
    let liquidity_total = token.liquidity.base + token.liquidity.quote;
    if liquidity_total < MIN_LIQUIDITY_SOL * 0.5 {
        return (true, format!("liquidity_drain(liq:{:.1}SOL)", liquidity_total));
    }

    // 9. SEVERE TOKEN FUNDAMENTALS DETERIORATION
    if token.price_change.h24 <= -40.0 && profit_pct <= 0.0 {
        return (true, format!("token_collapse_24h({:.1}%)", token.price_change.h24));
    }

    // 10. TIERED TRAILING STOP SYSTEM for 1% to 500% profits
    if profit_pct >= PROFIT_TIER_1 && has_min_profit {
        let trailing_stop = calculate_tiered_trailing_stop(profit_pct);

        if trailing_stop > f64::NEG_INFINITY && drop_from_peak <= trailing_stop {
            let tier_description = if profit_pct >= PROFIT_TIER_9 {
                "ULTRA_HIGH_500PCT+"
            } else if profit_pct >= PROFIT_TIER_8 {
                "VERY_HIGH_200PCT+"
            } else if profit_pct >= PROFIT_TIER_7 {
                "HIGH_100PCT+"
            } else if profit_pct >= PROFIT_TIER_6 {
                "MEDIUM_HIGH_50PCT+"
            } else if profit_pct >= PROFIT_TIER_5 {
                "MEDIUM_20PCT+"
            } else if profit_pct >= PROFIT_TIER_4 {
                "LOW_MEDIUM_10PCT+"
            } else if profit_pct >= PROFIT_TIER_3 {
                "LOW_5PCT+"
            } else if profit_pct >= PROFIT_TIER_2 {
                "VERY_LOW_3PCT+"
            } else {
                "MINIMAL_1PCT+"
            };

            return (
                true,
                format!(
                    "tiered_trailing_stop_{}(trail:{:.1}%, drop:{:.2}%, profit:{:.2}%, {:.6}SOL)",
                    tier_description,
                    trailing_stop,
                    drop_from_peak,
                    profit_pct,
                    profit_sol
                ),
            );
        }
    }

    // 11. MINIMUM PROFIT THRESHOLD - Scaled by time and profit level
    if profit_pct > 0.0 && !has_min_profit {
        let time_factor = if held_duration < max_duration / 4 {
            0.25 // Very early in trade
        } else if held_duration < max_duration / 2 {
            0.5 // Early-mid trade
        } else {
            1.0 // Late in trade, allow exit even without min profit
        };

        // Allow exit if we're late in trade OR have decent percentage profit
        if time_factor >= 1.0 || profit_pct >= PROFIT_TIER_1 * 1.5 {
            return (
                true,
                format!(
                    "time_override_exit(profit:{:.2}%, {:.6}SOL, time_factor:{:.1})",
                    profit_pct,
                    profit_sol,
                    time_factor
                ),
            );
        }

        return (
            false,
            format!(
                "profit_below_min_threshold({:.6}SOL < {:.6}SOL, time_factor:{:.1})",
                profit_sol,
                MIN_PROFIT_BEFORE_EXIT_SOL,
                time_factor
            ),
        );
    }

    (false, "".to_string())
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
    let profit_sol = current_value - pos.sol_spent;
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
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent;
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
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent;
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

// TRADING STRATEGY FUNCTIONS
//
// IMPORTANT: All trading functions (should_buy, should_sell, should_dca) must always
// receive current_price from batch_prices_from_pools for real-time trading decisions.
// Technical analysis functions use dataframe prices for historical indicators (RSI, BB, etc.)
// while using real-time current_price for current price comparisons and signal evaluation//
