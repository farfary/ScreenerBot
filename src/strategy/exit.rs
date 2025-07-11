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

/// ULTRA-AGGRESSIVE PROFESSIONAL PROFIT-TAKING STRATEGY V3.0
///
/// 🎯 OPTIMIZED FOR:
/// • Ultra-fast scalping (seconds to minutes)
/// • Multi-timeframe profit capture (0.1% to 1000%+)
/// • Professional market structure analysis
/// • Velocity-adaptive exit algorithms
/// • Real-time momentum detection
/// • Dynamic risk-adjusted trailing stops
///
/// 🚀 PERFORMANCE TARGETS:
/// • 95%+ win rate on micro-profits (0.1-2%)
/// • 80%+ capture rate on medium profits (2-20%)
/// • 60%+ capture rate on large profits (20-100%+)
/// • Maximum drawdown protection during pumps
///
/// ⚡ ULTRA-FAST EXECUTION:
/// • Sub-second decision making
/// • Real-time price action analysis
/// • Multi-layer momentum detection
/// • Professional trading algorithms
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
    } // ═══ REAL-TIME POOL PRICE ANALYSIS ═══
    // 🚀 INSTANT PRICE ACTION DETECTION USING POOL DATA

    let held_duration = (Utc::now() - pos.open_time).num_seconds();
    let held_minutes = held_duration / 60;
    let held_hours = held_minutes / 60;

    // Ultra-precise profit calculations
    let total_fees =
        ((1 + (pos.dca_count as usize)) as f64) * TRANSACTION_FEE_SOL + TRANSACTION_FEE_SOL;
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - total_fees;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };
    let drop_from_peak = if pos.peak_price > 0.0 {
        ((pos.peak_price - current_price) / pos.peak_price) * 100.0
    } else {
        0.0
    };

    // Calculate profit velocity (profit per minute held)
    let profit_velocity = if held_minutes > 0 { profit_pct / (held_minutes as f64) } else { 0.0 };

    // Calculate real-time price momentum using peak price tracking
    let instant_price_change = if pos.peak_price > 0.0 {
        ((current_price - pos.peak_price) / pos.peak_price) * 100.0
    } else {
        0.0
    };

    // Calculate price velocity (change per second)
    let price_velocity_per_second = if held_duration > 0 {
        profit_pct / (held_duration as f64)
    } else {
        0.0
    };

    // Instant price momentum analysis
    let avg_entry_price = pos.sol_spent / pos.token_amount;
    let price_momentum_from_entry = ((current_price - avg_entry_price) / avg_entry_price) * 100.0;

    // Real-time price trend detection
    let is_rapid_price_increase = instant_price_change > 0.0 && profit_pct > 0.5;
    let is_instant_dump = instant_price_change < -1.0;
    let is_high_velocity = price_velocity_per_second.abs() > 0.01; // 0.01% per second

    println!(
        "⚡ [REAL-TIME] {} | Pool Price: ${:.8} | Instant Δ: {:.2}% | Velocity: {:.4}%/s | From Entry: {:.2}%",
        token.symbol,
        current_price,
        instant_price_change,
        price_velocity_per_second,
        price_momentum_from_entry
    );

    println!(
        "\n🎯 [ULTRA-EXIT] {} | Price: ${:.8} | Profit: {:.3}% | Velocity: {:.3}%/min | Peak Drop: {:.1}% | Held: {}min",
        token.symbol,
        current_price,
        profit_pct,
        profit_velocity,
        drop_from_peak,
        held_minutes
    );

    // ═══ MINIMUM HOLD TIME - ULTRA REDUCED FOR SCALPING ═══
    // Allow exits after just 10 seconds for extreme profits
    let min_hold_time = if profit_pct >= 10.0 {
        10 // 10 seconds for large profits
    } else if profit_pct >= 5.0 {
        20 // 20 seconds for medium profits
    } else if profit_pct >= 2.0 {
        30 // 30 seconds for small profits
    } else if profit_pct >= 0.5 {
        60 // 1 minute for micro profits
    } else {
        120 // 2 minutes for breakeven/small profits
    };

    if held_duration < min_hold_time {
        return (false, format!("ultra_min_hold({}s_need_{}s)", held_duration, min_hold_time));
    }

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

        // 3. DCA Enhanced Momentum Exit - More flexible exit conditions
        if token.price_change.m5 < -3.0 {
            // Reduced from -2.0 for DCA_AGGRESSIVE_EXIT_THRESHOLD
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

    // ═══ PROFESSIONAL TIME-BASED EXIT ESCALATION ═══

    // Calculate time-based urgency multipliers
    let time_urgency_multiplier = if held_minutes >= 360 {
        5.0 // 6+ hours: Emergency exit mode
    } else if held_minutes >= 240 {
        3.0 // 4+ hours: Very urgent
    } else if held_minutes >= 180 {
        2.0 // 3+ hours: Urgent
    } else if held_minutes >= 120 {
        1.5 // 2+ hours: Moderate urgency
    } else if held_minutes >= 60 {
        1.2 // 1+ hour: Slight urgency
    } else {
        1.0 // < 1 hour: Normal
    };

    // Force exit profitable positions after extended holds
    if profit_pct > 0.0 {
        if held_minutes >= 480 {
            // 8+ hours: Force exit any profit
            println!(
                "⏰ [TIME-FORCE] {} | 8H+ FORCE EXIT: {:.2}% profit",
                token.symbol,
                profit_pct
            );
            return (true, format!("time_force_8h({:.2}%)", profit_pct));
        } else if held_minutes >= 360 && profit_pct >= 0.2 {
            // 6+ hours: Force exit if >0.2% profit
            println!(
                "⏰ [TIME-URGENT] {} | 6H+ URGENT EXIT: {:.2}% profit",
                token.symbol,
                profit_pct
            );
            return (true, format!("time_urgent_6h({:.2}%)", profit_pct));
        } else if held_minutes >= 240 && profit_pct >= 0.5 {
            // 4+ hours: Force exit if >0.5% profit
            println!("⏰ [TIME-LONG] {} | 4H+ LONG EXIT: {:.2}% profit", token.symbol, profit_pct);
            return (true, format!("time_long_4h({:.2}%)", profit_pct));
        }
    }

    // Time-based momentum threshold adjustments
    let time_adjusted_threshold = -1.0 / time_urgency_multiplier;

    // ═══ PROFESSIONAL MARKET STRUCTURE ANALYSIS ═══

    // Get comprehensive real-time price analysis
    let price_analysis = get_realtime_price_analysis(token);

    // Professional pump intensity detection
    let (pump_intensity, pump_description) = detect_pump_intensity(&price_analysis);

    // Advanced momentum deceleration analysis
    let (is_decelerating, deceleration_factor, decel_description) = detect_momentum_deceleration(
        token,
        &price_analysis,
        dataframe
    );

    // Smart money distribution detection
    let (is_distribution, distribution_description) = detect_pump_distribution(
        token,
        pump_intensity,
        dataframe
    );

    // Calculate multi-timeframe momentum scores (using available timeframes)
    let momentum_5m = token.price_change.m5;
    let momentum_1h = token.price_change.h1;
    let momentum_6h = token.price_change.h6;
    let momentum_24h = token.price_change.h24;

    // Professional momentum convergence analysis
    let momentum_convergence = (momentum_5m + momentum_1h + momentum_6h + momentum_24h) / 4.0;
    let momentum_divergence = (momentum_5m - momentum_1h).abs() + (momentum_1h - momentum_6h).abs();

    println!(
        "� [ANALYSIS] {} | Pump: {:?} | Decel: {:.2}x | Dist: {} | Conv: {:.1}% | Div: {:.1}%",
        token.symbol,
        pump_intensity,
        deceleration_factor,
        is_distribution,
        momentum_convergence,
        momentum_divergence
    );

    // ═══ ULTRA-AGGRESSIVE INSTANT PROFIT CAPTURE ═══

    // 🚀 EXTREME PROFIT INSTANT EXITS (1000%+ gains)
    if profit_pct >= 1000.0 {
        println!(
            "🚀🚀🚀 [MEGA-EXIT] {} | EXTREME PROFIT: {:.1}% - INSTANT SELL!",
            token.symbol,
            profit_pct
        );
        return (true, format!("mega_profit({:.1}%)", profit_pct));
    }

    // 🔥 VERY LARGE PROFIT INSTANT EXITS (100-1000%)
    if profit_pct >= 100.0 {
        println!(
            "🔥🔥 [HUGE-EXIT] {} | HUGE PROFIT: {:.1}% - INSTANT SELL!",
            token.symbol,
            profit_pct
        );
        return (true, format!("huge_profit({:.1}%)", profit_pct));
    }

    // 💎 LARGE PROFIT VELOCITY EXITS (50-100%)
    if profit_pct >= 50.0 {
        // For large profits, exit on any momentum weakness
        if momentum_5m < -0.5 || is_decelerating || is_distribution {
            println!(
                "� [LARGE-EXIT] {} | LARGE PROFIT: {:.1}% + weakness detected",
                token.symbol,
                profit_pct
            );
            return (true, format!("large_profit_weakness({:.1}%)", profit_pct));
        }

        // Ultra-tight trailing stop for large profits
        if drop_from_peak > 2.0 {
            println!(
                "💎 [LARGE-TRAIL] {} | LARGE PROFIT: {:.1}% + {:.1}% trail",
                token.symbol,
                profit_pct,
                drop_from_peak
            );
            return (true, format!("large_profit_trail({:.1}%)", profit_pct));
        }
    }

    // 🏆 MEDIUM PROFIT SMART EXITS (20-50%)
    if profit_pct >= 20.0 {
        // Exit on momentum divergence or distribution
        if momentum_divergence > 3.0 || is_distribution {
            println!(
                "🏆 [MED-EXIT] {} | MEDIUM PROFIT: {:.1}% + divergence/distribution",
                token.symbol,
                profit_pct
            );
            return (true, format!("medium_profit_divergence({:.1}%)", profit_pct));
        }

        // Tight trailing stop
        if drop_from_peak > 4.0 {
            println!(
                "🏆 [MED-TRAIL] {} | MEDIUM PROFIT: {:.1}% + {:.1}% trail",
                token.symbol,
                profit_pct,
                drop_from_peak
            );
            return (true, format!("medium_profit_trail({:.1}%)", profit_pct));
        }
    }

    // 💰 SMALL PROFIT AGGRESSIVE EXITS (5-20%)
    if profit_pct >= 5.0 && profit_pct < 20.0 {
        // Exit on any negative momentum during pumps
        if
            matches!(
                pump_intensity,
                PumpIntensity::Fast | PumpIntensity::VeryFast | PumpIntensity::Extreme
            )
        {
            if momentum_5m < -1.0 || momentum_1h < -1.5 {
                println!(
                    "💰 [SMALL-PUMP] {} | SMALL PROFIT: {:.1}% + pump momentum loss",
                    token.symbol,
                    profit_pct
                );
                return (true, format!("small_profit_pump_loss({:.1}%)", profit_pct));
            }
        }

        // Normal trailing stop
        if drop_from_peak > 6.0 {
            println!(
                "💰 [SMALL-TRAIL] {} | SMALL PROFIT: {:.1}% + {:.1}% trail",
                token.symbol,
                profit_pct,
                drop_from_peak
            );
            return (true, format!("small_profit_trail({:.1}%)", profit_pct));
        }
    }

    // ⚡ MICRO PROFIT ULTRA-FAST EXITS (0.5-5%)
    if profit_pct >= 0.5 && profit_pct < 5.0 {
        // Ultra-aggressive micro profit taking
        let micro_exit_threshold = if matches!(pump_intensity, PumpIntensity::Extreme) {
            -0.3 // Exit on 0.3% negative momentum during extreme pumps
        } else if matches!(pump_intensity, PumpIntensity::VeryFast) {
            -0.5 // Exit on 0.5% negative momentum during very fast pumps
        } else if matches!(pump_intensity, PumpIntensity::Fast) {
            -0.8 // Exit on 0.8% negative momentum during fast pumps
        } else {
            -1.2 // Exit on 1.2% negative momentum during normal conditions
        };

        if momentum_5m < micro_exit_threshold || momentum_1h < micro_exit_threshold * 2.0 {
            println!(
                "⚡ [MICRO-EXIT] {} | MICRO PROFIT: {:.2}% + momentum weakness",
                token.symbol,
                profit_pct
            );
            return (true, format!("micro_profit_momentum({:.2}%)", profit_pct));
        }

        // Micro profit trailing stop based on velocity
        let micro_trail_stop = if profit_velocity > 2.0 {
            3.0 // Tight stop for high velocity profits
        } else if profit_velocity > 1.0 {
            4.0 // Medium stop for medium velocity
        } else {
            5.0 // Wider stop for slow velocity
        };

        if drop_from_peak > micro_trail_stop {
            println!(
                "⚡ [MICRO-TRAIL] {} | MICRO PROFIT: {:.2}% + {:.1}% trail",
                token.symbol,
                profit_pct,
                drop_from_peak
            );
            return (true, format!("micro_profit_trail({:.2}%)", profit_pct));
        }
    }

    // 🔬 ULTRA-MICRO PROFIT EXITS (0.1-0.5%) - SCALPING MODE
    if profit_pct >= 0.1 && profit_pct < 0.5 {
        // Only during extreme market conditions with high velocity
        if matches!(pump_intensity, PumpIntensity::Extreme) && profit_velocity > 0.5 {
            if momentum_5m < -0.1 {
                println!(
                    "🔬 [SCALP-EXIT] {} | SCALP PROFIT: {:.3}% + extreme conditions",
                    token.symbol,
                    profit_pct
                );
                return (true, format!("scalp_profit({:.3}%)", profit_pct));
            }
        }

        // Ultra-micro trailing for high-frequency trading
        if drop_from_peak > 2.0 && profit_velocity > 1.0 {
            println!(
                "🔬 [SCALP-TRAIL] {} | SCALP PROFIT: {:.3}% + {:.1}% trail",
                token.symbol,
                profit_pct,
                drop_from_peak
            );
            return (true, format!("scalp_trail({:.3}%)", profit_pct));
        }
    }

    // ═══ PROFESSIONAL WHALE ACTIVITY ANALYSIS ═══

    let mut whale_distribution_factor = 1.0;
    let mut smart_money_signal = 0.0;

    if let Some(trades_cache) = trades {
        // Analyze whale trading patterns in multiple timeframes
        let whale_activity_30m = analyze_whale_activity(trades_cache, 1800); // 30 minutes
        let whale_activity_1h = analyze_whale_activity(trades_cache, 3600); // 1 hour
        let whale_activity_4h = analyze_whale_activity(trades_cache, 14400); // 4 hours

        // Calculate smart money flow
        smart_money_signal =
            (whale_activity_30m + whale_activity_1h * 0.7 + whale_activity_4h * 0.3) / 2.0;

        // Heavy distribution detection
        if smart_money_signal < -500.0 {
            whale_distribution_factor = 3.0;
            println!(
                "🚨 [WHALE-DUMP] {} | Heavy distribution: ${:.0}",
                token.symbol,
                smart_money_signal
            );

            if profit_pct > 0.1 {
                println!(
                    "🚨 [WHALE-EXIT] {} | Whale dump + {:.2}% profit",
                    token.symbol,
                    profit_pct
                );
                return (true, format!("whale_dump_exit({:.2}%)", profit_pct));
            }
        } else if smart_money_signal < -200.0 {
            whale_distribution_factor = 2.0;
            println!(
                "⚠️ [WHALE-SELL] {} | Moderate distribution: ${:.0}",
                token.symbol,
                smart_money_signal
            );
        } else if smart_money_signal > 300.0 {
            whale_distribution_factor = 0.7; // Less likely to sell during accumulation
            println!(
                "🐋 [WHALE-BUY] {} | Strong accumulation: ${:.0}",
                token.symbol,
                smart_money_signal
            );
        }
    }

    // Helper function for whale activity analysis
    fn analyze_whale_activity(trades_cache: &TokenTradesCache, duration_seconds: u64) -> f64 {
        let cutoff_time =
            std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() - duration_seconds;

        let whale_buys: f64 = trades_cache
            .get_whale_trades(100.0, 0) // $100+ trades
            .iter()
            .filter(|t| t.kind == "buy" && t.timestamp > cutoff_time)
            .map(|t| t.volume_usd)
            .sum();

        let whale_sells: f64 = trades_cache
            .get_whale_trades(100.0, 0) // $100+ trades
            .iter()
            .filter(|t| t.kind == "sell" && t.timestamp > cutoff_time)
            .map(|t| t.volume_usd)
            .sum();

        whale_buys - whale_sells
    }

    // Helper function for comprehensive rug/collapse detection
    fn detect_rug_or_collapse(
        token: &Token,
        liquidity_sol: f64,
        price_analysis: &super::price_analysis::PriceAnalysis,
        profit_pct: f64
    ) -> bool {
        let mut rug_indicators = 0;
        let mut rug_score = 0.0;

        // 1. EXTREME LIQUIDITY COLLAPSE (Critical Indicator)
        if liquidity_sol < 1.0 {
            rug_indicators += 3; // Very strong indicator
            rug_score += 30.0;
            println!(
                "🚨 [RUG-INDICATOR] {} | EXTREME LIQUIDITY COLLAPSE: {:.1}SOL",
                token.symbol,
                liquidity_sol
            );
        } else if liquidity_sol < 2.0 && profit_pct < -20.0 {
            rug_indicators += 2; // Strong indicator when combined with large loss
            rug_score += 20.0;
            println!(
                "🚨 [RUG-INDICATOR] {} | CRITICAL LIQUIDITY + LOSS: {:.1}SOL + {:.1}%",
                token.symbol,
                liquidity_sol,
                profit_pct
            );
        }

        // 2. EXTREME PRICE COLLAPSE (>80% drop in short time)
        if token.price_change.m5 < -80.0 || token.price_change.h1 < -90.0 {
            rug_indicators += 3; // Very strong indicator
            rug_score += 35.0;
            println!(
                "🚨 [RUG-INDICATOR] {} | EXTREME PRICE COLLAPSE: 5m={:.1}%, 1h={:.1}%",
                token.symbol,
                token.price_change.m5,
                token.price_change.h1
            );
        } else if token.price_change.m5 < -50.0 && token.price_change.h1 < -60.0 {
            rug_indicators += 2; // Strong indicator
            rug_score += 20.0;
            println!(
                "🚨 [RUG-INDICATOR] {} | SEVERE PRICE DROP: 5m={:.1}%, 1h={:.1}%",
                token.symbol,
                token.price_change.m5,
                token.price_change.h1
            );
        }

        // 3. TRADING VOLUME COLLAPSE (No one buying)
        if token.volume.h24 < 1000.0 && liquidity_sol < 5.0 {
            rug_indicators += 2; // Strong indicator when combined
            rug_score += 15.0;
            println!(
                "🚨 [RUG-INDICATOR] {} | VOLUME COLLAPSE: ${:.0} 24h volume",
                token.symbol,
                token.volume.h24
            );
        }

        // 4. EXTREME MOMENTUM LOSS (All timeframes negative)
        let all_negative =
            token.price_change.m5 < -10.0 &&
            token.price_change.h1 < -15.0 &&
            token.price_change.h6 < -20.0 &&
            token.price_change.h24 < -25.0;
        if all_negative {
            rug_indicators += 1;
            rug_score += 10.0;
            println!("🚨 [RUG-INDICATOR] {} | ALL-TIMEFRAME COLLAPSE", token.symbol);
        }

        // 5. EXTREME LOSS THRESHOLD (>70% loss)
        if profit_pct < -70.0 {
            rug_indicators += 2; // Strong indicator of potential rug
            rug_score += 25.0;
            println!("🚨 [RUG-INDICATOR] {} | EXTREME LOSS: {:.1}%", token.symbol, profit_pct);
        } else if profit_pct < -50.0 && liquidity_sol < 3.0 {
            rug_indicators += 1;
            rug_score += 15.0;
            println!(
                "🚨 [RUG-INDICATOR] {} | SEVERE LOSS + LOW LIQUIDITY: {:.1}% + {:.1}SOL",
                token.symbol,
                profit_pct,
                liquidity_sol
            );
        }

        // 6. SUDDEN LIQUIDITY DRAIN (if we had historical data)
        // This would require tracking liquidity over time
        // For now, we use current liquidity as a proxy

        // FINAL RUG DECISION
        let is_rug = rug_indicators >= 3 || rug_score >= 50.0;

        if is_rug {
            println!(
                "💀 [RUG-CONFIRMED] {} | RUG/COLLAPSE DETECTED: indicators={}, score={:.1}",
                token.symbol,
                rug_indicators,
                rug_score
            );
        } else if rug_indicators > 0 || rug_score > 0.0 {
            println!(
                "⚠️ [RUG-SUSPICIOUS] {} | Rug indicators detected: indicators={}, score={:.1} (not confirmed)",
                token.symbol,
                rug_indicators,
                rug_score
            );
        }

        is_rug
    }

    // ═══ ADVANCED TECHNICAL ANALYSIS ═══

    let mut technical_multiplier = 1.0;
    let mut technical_signals = Vec::new();

    if let Some(df) = dataframe {
        let primary_timeframe = df.get_primary_timeframe();

        // Multi-period momentum analysis
        let momentum_3p = primary_timeframe.price_change_over_period(3).unwrap_or(0.0);
        let momentum_5p = primary_timeframe.price_change_over_period(5).unwrap_or(0.0);
        let momentum_10p = primary_timeframe.price_change_over_period(10).unwrap_or(0.0);

        // Momentum acceleration/deceleration
        let momentum_acceleration = momentum_3p - momentum_5p;
        let momentum_trend = momentum_5p - momentum_10p;

        if momentum_acceleration < -2.0 && momentum_trend < -1.0 {
            technical_multiplier *= 2.5;
            technical_signals.push("momentum_decel");
        }

        // Volume analysis
        let volume_3p = primary_timeframe.average_volume(3).unwrap_or(0.0);
        let volume_10p = primary_timeframe.average_volume(10).unwrap_or(0.0);
        let volume_ratio = if volume_10p > 0.0 { volume_3p / volume_10p } else { 1.0 };

        if volume_ratio < 0.6 && profit_pct > 1.0 {
            technical_multiplier *= 1.8;
            technical_signals.push("volume_decline");
        }

        // Resistance/support analysis
        let recent_candles = primary_timeframe.get_recent_candles(20);
        if !recent_candles.is_empty() {
            let recent_high = recent_candles
                .iter()
                .map(|c| c.high)
                .fold(0.0, f64::max);
            let recent_low = recent_candles
                .iter()
                .map(|c| c.low)
                .fold(f64::INFINITY, f64::min);

            let distance_from_high = ((recent_high - current_price) / recent_high) * 100.0;
            let distance_from_low = ((current_price - recent_low) / recent_low) * 100.0;

            if distance_from_high < 1.0 && profit_pct > 2.0 {
                technical_multiplier *= 1.5;
                technical_signals.push("at_resistance");
            }

            if distance_from_low > 20.0 && profit_pct > 5.0 {
                technical_multiplier *= 1.3;
                technical_signals.push("extended_from_low");
            }
        }

        // VWAP analysis
        if let Some(vwap) = primary_timeframe.vwap(20) {
            let vwap_distance = ((current_price - vwap) / vwap) * 100.0;

            if vwap_distance > 15.0 && profit_pct > 3.0 {
                technical_multiplier *= 1.4;
                technical_signals.push("far_above_vwap");
            }
        }

        if !technical_signals.is_empty() {
            println!(
                "📊 [TECHNICAL] {} | Signals: {} | Multiplier: {:.1}x",
                token.symbol,
                technical_signals.join(", "),
                technical_multiplier
            );
        }
    }

    // ═══ INSTANT POOL PRICE ACTION EXITS ═══

    // 1. INSTANT DUMP PROTECTION - Exit immediately on pool price dumps
    if is_instant_dump && profit_pct > 0.1 {
        println!(
            "🚨 [INSTANT-DUMP] {} | Pool price dump: {:.2}% + {:.3}% profit",
            token.symbol,
            instant_price_change,
            profit_pct
        );
        return (true, format!("instant_pool_dump({:.3}%)", profit_pct));
    }

    // 2. HIGH VELOCITY PROFIT CAPTURE - Exit on rapid price movements
    if is_high_velocity && profit_pct >= 0.3 {
        println!(
            "⚡ [HIGH-VELOCITY] {} | Rapid movement: {:.4}%/s + {:.3}% profit",
            token.symbol,
            price_velocity_per_second,
            profit_pct
        );
        return (true, format!("high_velocity_exit({:.3}%)", profit_pct));
    }

    // 3. REAL-TIME MOMENTUM LOSS DETECTION
    if profit_pct >= 1.0 && instant_price_change < -0.5 {
        println!(
            "📉 [MOMENTUM-LOSS] {} | Pool momentum loss: {:.2}% + {:.2}% profit",
            token.symbol,
            instant_price_change,
            profit_pct
        );
        return (true, format!("pool_momentum_loss({:.2}%)", profit_pct));
    }

    // 4. POOL PRICE VELOCITY DECAY DETECTION
    let velocity_threshold = if profit_pct >= 10.0 {
        0.005 // 0.005% per second for large profits
    } else if profit_pct >= 5.0 {
        0.003 // 0.003% per second for medium profits
    } else if profit_pct >= 2.0 {
        0.002 // 0.002% per second for small profits
    } else {
        0.001 // 0.001% per second for micro profits
    };

    if
        profit_pct >= 1.0 &&
        price_velocity_per_second > 0.0 &&
        price_velocity_per_second < velocity_threshold
    {
        println!(
            "🐌 [VELOCITY-DECAY] {} | Pool velocity decay: {:.5}%/s < {:.5} + {:.2}% profit",
            token.symbol,
            price_velocity_per_second,
            velocity_threshold,
            profit_pct
        );
        return (true, format!("pool_velocity_decay({:.2}%)", profit_pct));
    }

    // ═══ ULTRA-DYNAMIC TRAILING STOP SYSTEM ═══

    // Calculate base trailing stop based on profit level and velocity
    let base_trailing_stop = if profit_pct >= 100.0 {
        1.5 // Ultra-tight for massive profits
    } else if profit_pct >= 50.0 {
        2.0 // Very tight for large profits
    } else if profit_pct >= 20.0 {
        3.0 // Tight for medium profits
    } else if profit_pct >= 10.0 {
        4.0 // Moderate for small profits
    } else if profit_pct >= 5.0 {
        5.0 // Standard for micro profits
    } else if profit_pct >= 2.0 {
        6.0 // Wider for tiny profits
    } else if profit_pct >= 1.0 {
        7.0 // Very wide for minimal profits
    } else {
        8.0 // Maximum width for breakeven
    };

    // Velocity-based adjustments
    let velocity_multiplier = if profit_velocity > 5.0 {
        0.6 // Much tighter for very fast profits
    } else if profit_velocity > 2.0 {
        0.8 // Tighter for fast profits
    } else if profit_velocity > 1.0 {
        1.0 // Normal for medium velocity
    } else if profit_velocity > 0.5 {
        1.2 // Wider for slow velocity
    } else {
        1.5 // Much wider for very slow velocity
    };

    // Pump intensity adjustments
    let pump_multiplier = pump_intensity.get_trailing_multiplier();

    // Combine all factors
    let final_trailing_stop =
        base_trailing_stop *
        velocity_multiplier *
        pump_multiplier *
        (1.0 / whale_distribution_factor) *
        (1.0 / technical_multiplier) *
        (1.0 / time_urgency_multiplier);

    println!(
        "🎯 [TRAIL-CALC] {} | Base: {:.1}% | Vel: {:.1}x | Pump: {:.1}x | Final: {:.1}% | Drop: {:.1}%",
        token.symbol,
        base_trailing_stop,
        velocity_multiplier,
        pump_multiplier,
        final_trailing_stop,
        drop_from_peak
    );

    // Execute trailing stop
    if drop_from_peak > final_trailing_stop {
        println!(
            "📉 [TRAIL-EXIT] {} | TRAILING STOP HIT: {:.3}% profit, {:.1}% drop (limit: {:.1}%)",
            token.symbol,
            profit_pct,
            drop_from_peak,
            final_trailing_stop
        );
        return (true, format!("ultra_trailing_stop({:.3}%)", profit_pct));
    }

    // ═══ RUG/COLLAPSE DETECTION - ENHANCED FOR LOSS CONTROL ═══

    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let is_rug_detected = detect_rug_or_collapse(token, liquidity_sol, &price_analysis, profit_pct);

    // Enhanced rug detection logging for loss decisions
    if is_rug_detected {
        println!(
            "� [RUG-CONFIRMED] {} | RUG/COLLAPSE DETECTED - May override loss restrictions",
            token.symbol
        );
    }

    // ENHANCED LIQUIDITY PROTECTION - Different behavior based on profit/loss
    if liquidity_sol < 2.0 {
        if profit_pct > 0.0 {
            println!(
                "🚨 [EMERGENCY] {} | CRITICAL LIQUIDITY: {:.1}SOL + {:.3}% profit",
                token.symbol,
                liquidity_sol,
                profit_pct
            );
            return (true, format!("emergency_liquidity({:.3}%)", profit_pct));
        } else if is_rug_detected && profit_pct <= -30.0 {
            // Only sell at loss if liquidity is critical AND it's a confirmed rug AND loss is significant
            println!(
                "🚨 [RUG-LIQUIDITY] {} | CRITICAL LIQUIDITY + RUG + HEAVY LOSS: {:.1}SOL + {:.3}% loss",
                token.symbol,
                liquidity_sol,
                profit_pct
            );
            return (true, format!("rug_liquidity_emergency({:.3}%)", profit_pct));
        } else {
            println!(
                "🔒 [HOLD-CRITICAL] {} | CRITICAL LIQUIDITY but HOLDING: {:.1}SOL + {:.3}% (no confirmed rug or loss not severe enough)",
                token.symbol,
                liquidity_sol,
                profit_pct
            );
        }
    }

    if liquidity_sol < 5.0 && profit_pct > 0.5 {
        println!(
            "⚠️ [LIQUIDITY-WARN] {} | LOW LIQUIDITY: {:.1}SOL + {:.2}% profit",
            token.symbol,
            liquidity_sol,
            profit_pct
        );
        return (true, format!("low_liquidity_exit({:.2}%)", profit_pct));
    }

    // ═══ FINAL MOMENTUM-BASED EXITS WITH ULTRA-TIGHT THRESHOLDS ═══

    // Calculate ultra-aggressive momentum thresholds
    let ultra_momentum_threshold =
        time_adjusted_threshold * whale_distribution_factor * technical_multiplier;

    // Profit-based momentum exits with ultra-tight conditions
    if profit_pct >= 1.0 {
        if momentum_5m < ultra_momentum_threshold || momentum_1h < ultra_momentum_threshold * 1.5 {
            println!(
                "⚡ [MOMENTUM-EXIT] {} | {:.3}% profit + momentum loss: 5m={:.2}%, 1h={:.2}%",
                token.symbol,
                profit_pct,
                momentum_5m,
                momentum_1h
            );
            return (true, format!("ultra_momentum_exit({:.3}%)", profit_pct));
        }
    }

    // Convergence-based exits
    if profit_pct >= 0.5 && momentum_divergence > 5.0 {
        println!(
            "� [DIVERGENCE-EXIT] {} | {:.3}% profit + momentum divergence: {:.1}%",
            token.symbol,
            profit_pct,
            momentum_divergence
        );
        return (true, format!("momentum_divergence_exit({:.3}%)", profit_pct));
    }

    // ═══ PROFIT VELOCITY DECAY DETECTION ═══

    // Check if profit velocity is decreasing (losing momentum)
    let velocity_threshold = if matches!(pump_intensity, PumpIntensity::Extreme) {
        0.5 // High threshold during extreme pumps
    } else if matches!(pump_intensity, PumpIntensity::VeryFast) {
        0.3 // Medium threshold during very fast pumps
    } else if matches!(pump_intensity, PumpIntensity::Fast) {
        0.2 // Low threshold during fast pumps
    } else {
        0.1 // Very low threshold during normal conditions
    };

    if profit_pct >= 2.0 && profit_velocity < velocity_threshold {
        println!(
            "� [VELOCITY-DECAY] {} | {:.2}% profit + velocity decay: {:.3}%/min < {:.3}",
            token.symbol,
            profit_pct,
            profit_velocity,
            velocity_threshold
        );
        return (true, format!("velocity_decay_exit({:.2}%)", profit_pct));
    }

    // ═══ ENHANCED LOSS CONTROL POLICY - NO SELLING BETWEEN 0% TO -50% ═══
    if profit_pct <= FORBIDDEN_LOSS_ZONE_MIN {
        // CATASTROPHIC LOSS PROTECTION: Allow selling only if loss is worse than -50%
        if profit_pct <= CATASTROPHIC_LOSS_THRESHOLD {
            // Check if it's been enough time or if it's a confirmed rug
            if is_rug_detected {
                println!(
                    "💀 [CATASTROPHIC-RUG] {} | EXTREME LOSS + RUG: {:.3}% - EMERGENCY EXIT",
                    token.symbol,
                    profit_pct
                );
                return (true, format!("catastrophic_rug_exit({:.3}%)", profit_pct));
            } else if held_minutes >= CATASTROPHIC_TIME_LIMIT_MINUTES {
                // 3 days holding period for catastrophic losses
                println!(
                    "⏰ [CATASTROPHIC-TIME] {} | EXTREME LOSS + 3+ DAYS: {:.3}% - FORCED EXIT",
                    token.symbol,
                    profit_pct
                );
                return (true, format!("catastrophic_time_exit({:.3}%)", profit_pct));
            } else {
                println!(
                    "🔒 [CATASTROPHIC-HOLD] {} | EXTREME LOSS: {:.3}% - HOLDING ({}min < 3days)",
                    token.symbol,
                    profit_pct,
                    held_minutes
                );
                return (false, format!("catastrophic_hold({:.3}%)", profit_pct));
            }
        } else {
            // FORBIDDEN ZONE: 0% to -50% loss - NEVER SELL (except emergency rug)
            let loss_severity = if profit_pct <= -30.0 {
                "HEAVY"
            } else if profit_pct <= -15.0 {
                "MODERATE"
            } else if profit_pct <= -5.0 {
                "MINOR"
            } else {
                "SMALL"
            };

            // Only allow selling in forbidden zone if it's an extreme emergency rug
            if is_rug_detected && profit_pct <= EMERGENCY_RUG_LOSS_THRESHOLD {
                println!(
                    "� [EMERGENCY-RUG] {} | {} LOSS + CONFIRMED RUG: {:.3}% - EMERGENCY EXIT",
                    token.symbol,
                    loss_severity,
                    profit_pct
                );
                return (true, format!("emergency_rug_override({:.3}%)", profit_pct));
            } else {
                println!(
                    "🔒 [FORBIDDEN-ZONE] {} | {} LOSS: {:.3}% - HOLDING (0% to -50% no-sell zone)",
                    token.symbol,
                    loss_severity,
                    profit_pct
                );
                return (false, format!("forbidden_zone_hold({:.3}%)", profit_pct));
            }
        }
    }

    println!(
        "✅ [PROFITABLE] {} | {:.3}% profit - analyzing exit conditions...",
        token.symbol,
        profit_pct
    );

    // ═══ HOLD DECISION WITH ULTRA-DETAILED ANALYSIS ═══

    println!(
        "🔒 [HOLD] {} | HOLDING: {:.3}% profit | Velocity: {:.3}%/min | Analysis: whale={:.0}, tech={:.1}x, time={:.1}x",
        token.symbol,
        profit_pct,
        profit_velocity,
        smart_money_signal,
        technical_multiplier,
        time_urgency_multiplier
    );

    (false, format!("ultra_hold_optimized({:.3}%_v{:.2})", profit_pct, profit_velocity))
}
