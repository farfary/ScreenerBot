use crate::prelude::*;
use crate::price_validation::{ is_price_valid, get_trading_price };
use super::config::*;
use super::price_analysis::get_realtime_price_analysis;
use super::pump_analysis::{
    detect_pump_intensity,
    detect_momentum_deceleration,
    detect_pump_distribution,
    PumpIntensity,
};

/// AGGRESSIVE PROFIT-TAKING SELL STRATEGY
/// Take profits quickly to offset rug losses
pub fn should_sell(
    token: &Token,
    pos: &Position,
    current_price: f64,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> (bool, String) {
    // ✅ CRITICAL: Validate price before any selling decision
    if !is_price_valid(current_price) {
        println!(
            "🚫 [SELL] {} | Invalid price: {:.12} - SELLING BLOCKED",
            token.symbol,
            current_price
        );
        return (false, format!("invalid_price({:.12})", current_price));
    }

    // Double-check with cached price validation
    if let Some(trading_price) = get_trading_price(&token.mint) {
        let price_diff = (((current_price - trading_price) / trading_price) * 100.0).abs();
        if price_diff > 10.0 {
            println!(
                "⚠️ [SELL] {} | Price mismatch: current={:.12}, cached={:.12} ({:.1}% diff) - using cached",
                token.symbol,
                current_price,
                trading_price,
                price_diff
            );
        }
    } else {
        println!("🚫 [SELL] {} | No valid cached price available - SELLING BLOCKED", token.symbol);
        return (false, "no_valid_price".to_string());
    }

    let total_fees =
        ((1 + (pos.dca_count as usize)) as f64) * TRANSACTION_FEE_SOL + TRANSACTION_FEE_SOL;
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - total_fees;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };
    let drop_from_peak = ((pos.peak_price - current_price) / pos.peak_price) * 100.0;
    let held_duration = (Utc::now() - pos.open_time).num_seconds();
    let held_minutes = held_duration / 60;

    println!(
        "\n💰 [SELL] {} | Current: ${:.8} | Profit: {:.2}% | Peak Drop: {:.1}% | Held: {}min",
        token.symbol,
        current_price,
        profit_pct,
        drop_from_peak,
        held_minutes
    );

    // ═══ DCA-SPECIFIC ENHANCED EXIT LOGIC (ADDRESSING 42% EFFICIENCY ISSUE) ═══
    let is_dca_position = pos.dca_count > 0;
    let mut dca_sell_multiplier = 1.0;

    if is_dca_position {
        println!("🔄 [SELL] {} | DCA POSITION - Applying enhanced exit criteria", token.symbol);

        // 1. DCA Profit Target - Take profits quickly at 3%
        if profit_pct >= DCA_PROFIT_TARGET {
            println!("💰 [SELL] {} | DCA PROFIT TARGET HIT: {:.2}%", token.symbol, profit_pct);
            return (true, format!("dca_profit_target({:.2}%)", profit_pct));
        }

        // 2. DCA Time Limit - Force exit after 2 hours
        if held_minutes >= DCA_MAX_HOLD_TIME_MINUTES {
            println!("⏰ [SELL] {} | DCA TIME LIMIT: {}min", token.symbol, held_minutes);
            return (true, format!("dca_time_limit({:.2}%)", profit_pct));
        }

        // 3. DCA Aggressive Momentum Exit - Exit on any 2% negative momentum
        if token.price_change.m5 < DCA_AGGRESSIVE_EXIT_THRESHOLD {
            println!(
                "📉 [SELL] {} | DCA MOMENTUM EXIT: {:.1}%",
                token.symbol,
                token.price_change.m5
            );
            return (true, format!("dca_momentum_exit({:.2}%)", profit_pct));
        }

        // 4. Apply DCA-specific multipliers
        dca_sell_multiplier = DCA_SELL_MULTIPLIER;
        println!(
            "🔥 [SELL] {} | DCA sell pressure multiplier: {:.1}x",
            token.symbol,
            dca_sell_multiplier
        );
    }

    // ═══ TIME-BASED EXIT URGENCY ═══
    let time_multiplier = if held_minutes >= 180 {
        TIME_BASED_SELL_MULTIPLIER_3H
    } else if held_minutes >= 120 {
        TIME_BASED_SELL_MULTIPLIER_2H
    } else if held_minutes >= 60 {
        TIME_BASED_SELL_MULTIPLIER_1H
    } else {
        1.0
    };

    if time_multiplier > 1.0 {
        println!("⏰ [SELL] {} | Time urgency multiplier: {:.1}x", token.symbol, time_multiplier);
    }

    // 1. Minimum hold time
    if held_duration < MIN_HOLD_TIME_SECONDS {
        return (false, format!("min_hold_time({}s)", held_duration));
    }

    // 2. NEVER sell at loss
    if profit_pct <= 0.0 {
        println!("📉 [SELL] {} | HOLD: Never sell at loss: {:.2}%", token.symbol, profit_pct);
        return (false, format!("no_loss_selling({:.2}%)", profit_pct));
    }

    // 3. Force exit after maximum hold time for profitable positions
    if held_minutes >= PROFITABLE_MAX_HOLD_MINUTES && profit_pct > 0.0 {
        println!("⏰ [SELL] {} | FORCED EXIT: {}min limit reached", token.symbol, held_minutes);
        return (true, format!("forced_time_exit({:.2}%)", profit_pct));
    }

    println!(
        "✅ [SELL] {} | Profitable: {:.2}% - checking exit conditions...",
        token.symbol,
        profit_pct
    );

    // ═══ ENHANCED FAST PUMP DETECTION & VELOCITY-BASED EXITS ═══

    // Get comprehensive price analysis
    let price_analysis = get_realtime_price_analysis(token);

    // Detect pump intensity and momentum
    let (pump_intensity, pump_description) = detect_pump_intensity(&price_analysis);

    // Detect momentum deceleration
    let (is_decelerating, deceleration_factor, decel_description) = detect_momentum_deceleration(
        token,
        &price_analysis,
        dataframe
    );

    // Detect pump distribution (volume declining during pump)
    let (is_distribution, distribution_description) = detect_pump_distribution(
        token,
        pump_intensity,
        dataframe
    );

    println!(
        "🚀 [PUMP ANALYSIS] {} | Intensity: {:?} ({}) | Decel: {} ({:.2}x) | Distribution: {}",
        token.symbol,
        pump_intensity,
        pump_description,
        is_decelerating,
        deceleration_factor,
        is_distribution
    );

    // ═══ FAST PUMP IMMEDIATE EXITS ═══

    // 1. EXTREME PUMP + DISTRIBUTION = IMMEDIATE EXIT
    if
        matches!(pump_intensity, PumpIntensity::Extreme) &&
        is_distribution &&
        profit_pct > VELOCITY_BASED_MIN_PROFIT
    {
        println!(
            "🚨 [SELL] {} | EXTREME PUMP + DISTRIBUTION: {:.2}% profit | Intensity: {:?}",
            token.symbol,
            profit_pct,
            pump_intensity
        );
        return (true, format!("extreme_pump_distribution({:.2}%)", profit_pct));
    }

    // 2. VERY FAST PUMP + MOMENTUM DECELERATION = QUICK EXIT
    if
        matches!(pump_intensity, PumpIntensity::VeryFast | PumpIntensity::Extreme) &&
        is_decelerating &&
        profit_pct > FAST_PUMP_QUICK_EXIT_PCT
    {
        println!(
            "⚡ [SELL] {} | FAST PUMP DECELERATION: {:.2}% profit | Decel: {:.2}x",
            token.symbol,
            profit_pct,
            deceleration_factor
        );
        return (true, format!("fast_pump_decel({:.2}%)", profit_pct));
    }

    // 3. ANY PUMP + STRONG DISTRIBUTION = EXIT
    if
        !matches!(pump_intensity, PumpIntensity::Normal) &&
        is_distribution &&
        profit_pct > VELOCITY_BASED_MIN_PROFIT
    {
        println!(
            "📊 [SELL] {} | PUMP + DISTRIBUTION: {:.2}% profit | {}",
            token.symbol,
            profit_pct,
            distribution_description
        );
        return (true, format!("pump_distribution({:.2}%)", profit_pct));
    }

    // ─── TRADES DATA ANALYSIS FOR SELLING ───
    let mut whale_distribution_detected = false;
    let mut sell_pressure_multiplier = 1.0;

    if let Some(trades_cache) = trades {
        // Check for whale distribution (large sells)
        let recent_whale_sells = trades_cache
            .get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 0) // Last 30 min
            .iter()
            .filter(
                |t|
                    t.kind == "sell" &&
                    t.timestamp >
                        std::time::SystemTime
                            ::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() - 1800
            ) // Last 30 minutes
            .map(|t| t.volume_usd)
            .sum::<f64>();

        let recent_whale_buys = trades_cache
            .get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 0) // Last 30 min
            .iter()
            .filter(
                |t|
                    t.kind == "buy" &&
                    t.timestamp >
                        std::time::SystemTime
                            ::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() - 1800
            ) // Last 30 minutes
            .map(|t| t.volume_usd)
            .sum::<f64>();

        let whale_net_flow = recent_whale_buys - recent_whale_sells;

        if whale_net_flow < WHALE_DISTRIBUTION_THRESHOLD {
            // Heavy whale distribution
            whale_distribution_detected = true;
            sell_pressure_multiplier = WHALE_DISTRIBUTION_MULTIPLIER;
            println!(
                "🚨 [SELL] {} | Whale distribution detected: ${:.0} net outflow",
                token.symbol,
                whale_net_flow.abs()
            );
        } else if whale_net_flow < MODERATE_SELLING_THRESHOLD {
            // Moderate distribution
            sell_pressure_multiplier = MODERATE_SELLING_MULTIPLIER;
            println!(
                "⚠️ [SELL] {} | Moderate selling pressure: ${:.0} net outflow",
                token.symbol,
                whale_net_flow.abs()
            );
        } else {
            println!(
                "🐋 [SELL] {} | Whale activity: ${:.0} net flow",
                token.symbol,
                whale_net_flow
            );
        }
    }

    // ─── OHLCV TECHNICAL ANALYSIS FOR SELLING ───
    let mut momentum_multiplier = 1.0;

    if let Some(df) = dataframe {
        let primary_timeframe = df.get_primary_timeframe();

        // Check for bearish momentum
        if let Some(price_change_recent) = primary_timeframe.price_change_over_period(3) {
            if price_change_recent < RECENT_MOMENTUM_THRESHOLD {
                momentum_multiplier = MOMENTUM_MULTIPLIER;
                println!(
                    "📉 [SELL] {} | Bearish momentum: {:.1}% over 3 periods",
                    token.symbol,
                    price_change_recent
                );
            }
        }

        // Check if price is at resistance (recent highs)
        let recent_candles = primary_timeframe.get_recent_candles(30); // Last 30 periods
        if !recent_candles.is_empty() {
            let recent_high = recent_candles
                .iter()
                .map(|c| c.high)
                .fold(0.0, f64::max);
            let distance_from_high = ((recent_high - current_price) / recent_high) * 100.0;

            if distance_from_high < RESISTANCE_DISTANCE_THRESHOLD {
                momentum_multiplier *= RESISTANCE_MULTIPLIER;
                println!(
                    "📊 [SELL] {} | Near resistance: current={:.8} vs high={:.8} (-{:.1}%)",
                    token.symbol,
                    current_price,
                    recent_high,
                    distance_from_high
                );
            }
        }

        // Check volume trends (decreasing volume on pump = distribution)
        let recent_avg_volume = primary_timeframe.average_volume(3).unwrap_or(0.0);
        let older_avg_volume = primary_timeframe.average_volume(10).unwrap_or(0.0);

        if
            recent_avg_volume < older_avg_volume * VOLUME_DECLINE_MULTIPLIER &&
            profit_pct > MIN_PROFIT_FOR_VWAP_SELL
        {
            println!(
                "📉 [SELL] {} | Volume declining on pump: recent={:.0} vs avg={:.0}",
                token.symbol,
                recent_avg_volume,
                older_avg_volume
            );
        }

        // VWAP check - if price significantly above VWAP and profitable, consider selling
        if let Some(vwap) = primary_timeframe.vwap(20) {
            if
                current_price > vwap * PROFITABLE_VWAP_THRESHOLD &&
                profit_pct > MIN_PROFIT_FOR_VWAP_SELL
            {
                momentum_multiplier *= VWAP_EXTENDED_MULTIPLIER;
                println!(
                    "📊 [SELL] {} | Price extended above VWAP: {:.8} vs {:.8} (+{:.1}%)",
                    token.symbol,
                    current_price,
                    vwap,
                    ((current_price - vwap) / vwap) * 100.0
                );
            }
        }
    }

    // Combine all multipliers including DCA, time-based urgency, and pump intensity
    let pump_multiplier = pump_intensity.get_momentum_multiplier();
    sell_pressure_multiplier *=
        momentum_multiplier * dca_sell_multiplier * time_multiplier * pump_multiplier;

    println!(
        "🎛️ [SELL] {} | Sell pressure: {:.1}x (momentum:{:.1}x, dca:{:.1}x, time:{:.1}x, pump:{:.1}x)",
        token.symbol,
        sell_pressure_multiplier,
        momentum_multiplier,
        dca_sell_multiplier,
        time_multiplier,
        pump_multiplier
    );

    // ═══ ENHANCED PUMP-AWARE TRAILING STOPS ═══

    // Determine appropriate trailing stop based on profit level
    let base_trailing_stop = if profit_pct >= 25.0 {
        LARGE_PROFIT_TRAILING_STOP
    } else if profit_pct >= 10.0 {
        MEDIUM_PROFIT_TRAILING_STOP
    } else if profit_pct >= 3.0 {
        SMALL_PROFIT_TRAILING_STOP
    } else {
        QUICK_PROFIT_TRAILING_STOP
    };

    // Apply pump intensity multiplier to tighten stops during fast pumps
    let pump_trailing_multiplier = pump_intensity.get_trailing_multiplier();
    let pump_adjusted_trailing = base_trailing_stop * pump_trailing_multiplier;

    // Apply sell pressure adjustment
    let final_trailing_stop = pump_adjusted_trailing / sell_pressure_multiplier;

    println!(
        "🎯 [TRAILING] {} | Base: {:.1}% | Pump adj: {:.1}% (x{:.2}) | Final: {:.1}% | Drop: {:.1}%",
        token.symbol,
        base_trailing_stop,
        pump_adjusted_trailing,
        pump_trailing_multiplier,
        final_trailing_stop,
        drop_from_peak
    );

    if drop_from_peak > final_trailing_stop {
        println!(
            "📉 [SELL] {} | PUMP-AWARE TRAILING STOP: {:.2}% profit, {:.1}% drop (limit: {:.1}%) | Pump: {:?}",
            token.symbol,
            profit_pct,
            drop_from_peak,
            final_trailing_stop,
            pump_intensity
        );
        return (true, format!("pump_aware_trailing({:.2}%)", profit_pct));
    }

    // Apply tightened momentum thresholds to different profit ranges with pump-aware adjustments
    let weak_threshold = WEAK_SELL_THRESHOLD * sell_pressure_multiplier;
    let medium_threshold = MEDIUM_SELL_THRESHOLD * sell_pressure_multiplier;
    let strong_threshold = STRONG_SELL_THRESHOLD * sell_pressure_multiplier;

    // Emergency exit on whale distribution
    if whale_distribution_detected && profit_pct > EMERGENCY_EXIT_MIN_PROFIT {
        println!(
            "🚨 [SELL] {} | WHALE DUMP: {:.2}% profit + distribution",
            token.symbol,
            profit_pct
        );
        return (true, format!("whale_distribution({:.2}%)", profit_pct));
    }

    // ═══ ENHANCED PUMP-AWARE PROFIT-TAKING WITH MOMENTUM ═══

    // Enhanced profit-taking with pump-specific thresholds
    if profit_pct >= 0.5 && profit_pct < 3.0 {
        // For fast pumps, be extra aggressive on quick profits
        let adjusted_threshold = if !matches!(pump_intensity, PumpIntensity::Normal) {
            weak_threshold * VELOCITY_EXIT_MULTIPLIER
        } else {
            weak_threshold
        };

        if token.price_change.m5 < adjusted_threshold {
            println!(
                "💸 [SELL] {} | QUICK PROFIT: {:.2}% + momentum weakness | Pump: {:?}",
                token.symbol,
                profit_pct,
                pump_intensity
            );
            return (true, format!("quick_profit_momentum({:.2}%)", profit_pct));
        }
    }

    if profit_pct >= 3.0 && profit_pct < 10.0 {
        // For pumps, use velocity-based exits
        let adjusted_threshold = if
            matches!(
                pump_intensity,
                PumpIntensity::Fast | PumpIntensity::VeryFast | PumpIntensity::Extreme
            )
        {
            medium_threshold * 1.5 // More aggressive during pumps
        } else {
            medium_threshold
        };

        if token.price_change.m5 < adjusted_threshold {
            println!(
                "💸 [SELL] {} | SMALL PROFIT: {:.2}% + momentum | Pump: {:?}",
                token.symbol,
                profit_pct,
                pump_intensity
            );
            return (true, format!("small_profit_momentum({:.2}%)", profit_pct));
        }
    }

    if profit_pct >= 10.0 && profit_pct < 25.0 {
        // During very fast pumps, take profits more aggressively
        let adjusted_threshold = if
            matches!(pump_intensity, PumpIntensity::VeryFast | PumpIntensity::Extreme)
        {
            strong_threshold * 1.8
        } else if matches!(pump_intensity, PumpIntensity::Fast) {
            strong_threshold * 1.3
        } else {
            strong_threshold
        };

        if token.price_change.m5 < adjusted_threshold {
            println!(
                "💸 [SELL] {} | MEDIUM PROFIT: {:.2}% + strong momentum | Pump: {:?}",
                token.symbol,
                profit_pct,
                pump_intensity
            );
            return (true, format!("medium_profit_momentum({:.2}%)", profit_pct));
        }
    }

    if profit_pct >= 25.0 {
        // For extreme pumps, be very aggressive on large profits
        let adjusted_threshold = if matches!(pump_intensity, PumpIntensity::Extreme) {
            strong_threshold * 2.5
        } else if matches!(pump_intensity, PumpIntensity::VeryFast) {
            strong_threshold * 2.0
        } else if matches!(pump_intensity, PumpIntensity::Fast) {
            strong_threshold * 1.5
        } else {
            strong_threshold * 1.2
        };

        if token.price_change.m5 < adjusted_threshold {
            println!(
                "💸 [SELL] {} | LARGE PROFIT: {:.2}% + very strong momentum | Pump: {:?}",
                token.symbol,
                profit_pct,
                pump_intensity
            );
            return (true, format!("large_profit_momentum({:.2}%)", profit_pct));
        }
    }

    // 4. Emergency exits
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    if liquidity_sol < MIN_LIQUIDITY_SOL * 0.3 {
        println!("🚨 [SELL] {} | LIQUIDITY CRISIS: {:.1}SOL", token.symbol, liquidity_sol);
        return (true, format!("liquidity_crisis({:.1}SOL)", liquidity_sol));
    }

    // Default: Hold
    println!("🔒 [SELL] {} | HOLDING: {:.2}% profit", token.symbol, profit_pct);
    (false, format!("holding_optimized({:.2}%)", profit_pct))
}
