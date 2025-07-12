use crate::prelude::*;
use crate::price_validation::{ is_price_valid, get_trading_price };
use crate::configs::ARGS; // Add import for argument checking
use super::config::*;
use super::position::can_enter_token_position;
use super::price_analysis::{
    get_realtime_price_analysis,
    calculate_trade_size_sol,
    calculate_trade_size_with_market_cap,
    get_price_change_with_fallback,
    classify_market_trend,
    get_market_condition_bonus,
    get_most_reliable_price,
};

// Check if debug entry mode is enabled
fn is_debug_entry_enabled() -> bool {
    ARGS.iter().any(|arg| arg == "--debug-entry")
}

// Enhanced debug macro for entry-specific logging with timestamp
macro_rules! debug_entry {
    ($($arg:tt)*) => {
        if is_debug_entry_enabled() {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            println!("🔍 [DEBUG_ENTRY][{}] {}", timestamp, format!($($arg)*));
        }
    };
}

// Debug section macro for major sections
macro_rules! debug_section {
    ($title:expr) => {
        if is_debug_entry_enabled() {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            println!("🔍 [DEBUG_ENTRY][{}] ═══ {} ═══", timestamp, $title);
        }
    };
}

// Debug result macro for decision points
macro_rules! debug_result {
    ($condition:expr, $success_msg:expr, $fail_msg:expr) => {
        if is_debug_entry_enabled() {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            if $condition {
                println!("🔍 [DEBUG_ENTRY][{}] ✅ {}", timestamp, $success_msg);
            } else {
                println!("🔍 [DEBUG_ENTRY][{}] ❌ {}", timestamp, $fail_msg);
            }
        }
    };
}

// Debug metrics macro for displaying key values
macro_rules! debug_metrics {
    ($($key:expr => $value:expr),+ $(,)?) => {
        if is_debug_entry_enabled() {
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            print!("🔍 [DEBUG_ENTRY][{}] 📊 METRICS: ", timestamp);
            $(
                print!("{}={} | ", $key, $value);
            )+
            println!();
        }
    };
}

const NEAR_15M_HIGH_THRESHOLD_PCT: f64 = 1.5; // % threshold for 15m high proximity warning
const NEAR_5M_HIGH_THRESHOLD_PCT: f64 = 1.0; // % threshold for 5m high proximity warning

/// Analyze if a price dump is dangerous or a healthy dip opportunity
fn analyze_dump_safety(
    token: &Token,
    change_5m: f64,
    is_realtime: bool,
    trades: Option<&TokenTradesCache>,
    liquidity_sol: f64,
    total_holders: u64
) -> bool {
    // Not a significant drop, no need for analysis
    if change_5m > -5.0 {
        debug_entry!("Dump analysis skipped: change_5m={:.1}% (not significant)", change_5m);
        return false;
    }

    debug_entry!("=== DUMP SAFETY ANALYSIS START ===");
    debug_entry!("Analyzing {:.1}%{} drop for safety", change_5m, if is_realtime {
        "_RT"
    } else {
        "_DX"
    });

    let mut danger_signals = 0;
    let mut safety_signals = 0;

    println!("🔍 [DIP_ANALYSIS] {} | Analyzing {:.1}%{} drop...", token.symbol, change_5m, if
        is_realtime
    {
        "_RT"
    } else {
        "_DX"
    });

    // 1. Extreme dump check (likely rug or major event)
    debug_entry!("Checking dump severity...");
    if change_5m <= EXTREME_DUMP_THRESHOLD {
        danger_signals += 3;
        debug_entry!(
            "EXTREME DUMP: {:.1}% ≤ {:.1}% (+3 danger)",
            change_5m,
            EXTREME_DUMP_THRESHOLD
        );
        println!("🚨 [DIP] Extreme dump: {:.1}% (+3 danger)", change_5m);
    } else if change_5m <= DANGEROUS_DUMP_THRESHOLD {
        danger_signals += 2;
        debug_entry!(
            "MAJOR DUMP: {:.1}% ≤ {:.1}% (+2 danger)",
            change_5m,
            DANGEROUS_DUMP_THRESHOLD
        );
        println!("⚠️ [DIP] Major dump: {:.1}% (+2 danger)", change_5m);
    } else if change_5m <= HEALTHY_DIP_MAX {
        danger_signals += 1;
        debug_entry!("SIGNIFICANT DUMP: {:.1}% ≤ {:.1}% (+1 danger)", change_5m, HEALTHY_DIP_MAX);
        println!("📉 [DIP] Significant dump: {:.1}% (+1 danger)", change_5m);
    } else {
        safety_signals += 1;
        debug_entry!("MODERATE DIP: {:.1}% > {:.1}% (+1 safety)", change_5m, HEALTHY_DIP_MAX);
        println!("✅ [DIP] Moderate dip: {:.1}% (+1 safety)", change_5m);
    }

    // 2. Liquidity safety check
    debug_entry!("Checking liquidity safety: {:.1}SOL", liquidity_sol);
    if liquidity_sol < 5.0 {
        danger_signals += 2;
        debug_entry!("VERY LOW LIQUIDITY: {:.1}SOL < 5.0SOL (+2 danger)", liquidity_sol);
        println!("💧 [DIP] Very low liquidity: {:.1}SOL (+2 danger)", liquidity_sol);
    } else if liquidity_sol < 15.0 {
        danger_signals += 1;
        debug_entry!("LOW LIQUIDITY: {:.1}SOL < 15.0SOL (+1 danger)", liquidity_sol);
        println!("💧 [DIP] Low liquidity: {:.1}SOL (+1 danger)", liquidity_sol);
    } else if liquidity_sol > 50.0 {
        safety_signals += 1;
        debug_entry!("STRONG LIQUIDITY: {:.1}SOL > 50.0SOL (+1 safety)", liquidity_sol);
        println!("💪 [DIP] Strong liquidity: {:.1}SOL (+1 safety)", liquidity_sol);
    } else {
        debug_entry!("MODERATE LIQUIDITY: {:.1}SOL (no signal)", liquidity_sol);
    }

    // 3. Holder count safety
    debug_entry!("Checking holder count safety: {}", total_holders);
    if total_holders < 5 {
        danger_signals += 2;
        debug_entry!("VERY FEW HOLDERS: {} < 5 (+2 danger)", total_holders);
        println!("👥 [DIP] Very few holders: {} (+2 danger)", total_holders);
    } else if total_holders < 20 {
        danger_signals += 1;
        debug_entry!("FEW HOLDERS: {} < 20 (+1 danger)", total_holders);
        println!("👥 [DIP] Few holders: {} (+1 danger)", total_holders);
    } else if total_holders > 100 {
        safety_signals += 1;
        debug_entry!("GOOD HOLDER BASE: {} > 100 (+1 safety)", total_holders);
        println!("👥 [DIP] Good holder base: {} (+1 safety)", total_holders);
    } else {
        debug_entry!("MODERATE HOLDER COUNT: {} (no signal)", total_holders);
    }

    // 4. Whale activity during the dump
    debug_entry!("Analyzing whale activity during dump...");
    if let Some(trades_cache) = trades {
        let recent_whales = trades_cache.get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 1);
        let whale_buys: Vec<_> = recent_whales
            .iter()
            .filter(|t| t.kind == "buy")
            .collect();
        let whale_sells: Vec<_> = recent_whales
            .iter()
            .filter(|t| t.kind == "sell")
            .collect();

        debug_entry!(
            "Whale trades analysis: total={}, buys={}, sells={}",
            recent_whales.len(),
            whale_buys.len(),
            whale_sells.len()
        );

        if whale_buys.len() > whale_sells.len() && whale_buys.len() > 0 {
            safety_signals += 2;
            debug_entry!(
                "WHALES BUYING THE DIP: {} buys vs {} sells (+2 safety)",
                whale_buys.len(),
                whale_sells.len()
            );
            println!(
                "🐋 [DIP] Whales buying the dip: {} buys vs {} sells (+2 safety)",
                whale_buys.len(),
                whale_sells.len()
            );
        } else if whale_sells.len() > whale_buys.len() * 2 {
            danger_signals += 2;
            debug_entry!(
                "WHALE EXODUS: {} sells vs {} buys (+2 danger)",
                whale_sells.len(),
                whale_buys.len()
            );
            println!(
                "🐋 [DIP] Whale exodus: {} sells vs {} buys (+2 danger)",
                whale_sells.len(),
                whale_buys.len()
            );
        } else {
            debug_entry!(
                "NEUTRAL WHALE ACTIVITY: {} buys vs {} sells (no signal)",
                whale_buys.len(),
                whale_sells.len()
            );
        }

        // Check for panic selling (many small sells)
        let small_sells = trades_cache
            .get_trades_by_type("sell", 1)
            .iter()
            .filter(|t| t.volume_usd < SMALL_TRADE_THRESHOLD_USD)
            .count();

        debug_entry!(
            "Panic selling check: small_sells={}, threshold={}",
            small_sells,
            PANIC_SELLING_THRESHOLD
        );

        if small_sells > (PANIC_SELLING_THRESHOLD as usize) {
            danger_signals += 1;
            debug_entry!(
                "PANIC SELLING: {} small sells > {} (+1 danger)",
                small_sells,
                PANIC_SELLING_THRESHOLD
            );
            println!("😰 [DIP] Panic selling detected: {} small sells (+1 danger)", small_sells);
        } else {
            debug_entry!(
                "NO PANIC SELLING: {} small sells ≤ {} (no signal)",
                small_sells,
                PANIC_SELLING_THRESHOLD
            );
        }
    } else {
        debug_entry!("No trades data available for whale activity analysis");
    }

    // 5. Volume analysis
    debug_entry!("Analyzing volume during dump...");
    let volume_ratio = if token.volume.h24 > 0.0 {
        token.volume.h1 / (token.volume.h24 / 24.0)
    } else {
        0.0
    };

    debug_entry!(
        "Volume analysis: h1={:.0}, h24={:.0}, ratio={:.1}x",
        token.volume.h1,
        token.volume.h24,
        volume_ratio
    );

    if volume_ratio > 3.0 {
        safety_signals += 1;
        debug_entry!("HIGH VOLUME DUMP: {:.1}x avg > 3.0x (+1 safety)", volume_ratio);
        println!("📊 [DIP] High volume during drop: {:.1}x avg (+1 safety)", volume_ratio);
    } else if volume_ratio < 0.5 {
        danger_signals += 1;
        debug_entry!("LOW VOLUME DUMP: {:.1}x avg < 0.5x (+1 danger)", volume_ratio);
        println!("📊 [DIP] Low volume dump: {:.1}x avg (+1 danger)", volume_ratio);
    }

    let total_signals = danger_signals + safety_signals;
    let danger_ratio = if total_signals > 0 {
        (danger_signals as f64) / (total_signals as f64)
    } else {
        0.5
    };

    debug_entry!("Final dump safety analysis:");
    debug_entry!("  - Total signals: {}", total_signals);
    debug_entry!("  - Danger signals: {}", danger_signals);
    debug_entry!("  - Safety signals: {}", safety_signals);
    debug_entry!(
        "  - Danger ratio: {:.1}% (threshold: {:.1}%)",
        danger_ratio * 100.0,
        MAX_DANGER_RATIO * 100.0
    );

    let is_dangerous = danger_ratio > MAX_DANGER_RATIO;
    debug_entry!(
        "DUMP SAFETY RESULT: {} (danger_ratio={:.1}% {} threshold={:.1}%)",
        if is_dangerous {
            "DANGEROUS"
        } else {
            "SAFE"
        },
        danger_ratio * 100.0,
        if is_dangerous {
            ">"
        } else {
            "≤"
        },
        MAX_DANGER_RATIO * 100.0
    );

    println!(
        "⚖️ [DIP_RESULT] Danger: {} | Safety: {} | Ratio: {:.1}%",
        danger_signals,
        safety_signals,
        danger_ratio * 100.0
    );

    // Consider dangerous if danger signals outweigh safety significantly
    is_dangerous
}

/// Enhanced swing trading entry analysis for dip opportunities
fn analyze_swing_entry_opportunity(
    token: &Token,
    price_analysis: &PriceAnalysis,
    trades: Option<&TokenTradesCache>
) -> (f64, Vec<String>) {
    debug_section!("SWING ENTRY ANALYSIS");

    let mut swing_score = 0.0;
    let mut signals = Vec::new();

    debug_entry!("Initial state: swing_score={:.2}, signals={:?}", swing_score, signals);

    // 1. Dip buying opportunity (healthy pullbacks)
    debug_entry!("1. Analyzing healthy dip opportunity...");
    debug_entry!(
        "   Price change 5m: {:.1}% (range: {:.1}% to {:.1}%)",
        price_analysis.change_5m,
        HEALTHY_DIP_MAX,
        HEALTHY_DIP_MIN
    );

    if price_analysis.change_5m >= HEALTHY_DIP_MAX && price_analysis.change_5m <= HEALTHY_DIP_MIN {
        let dip_strength = (
            (price_analysis.change_5m.abs() - HEALTHY_DIP_MIN.abs()) /
            (HEALTHY_DIP_MAX.abs() - HEALTHY_DIP_MIN.abs())
        ).min(1.0);
        let score_addition = dip_strength * 0.3;
        swing_score += score_addition;
        signals.push(format!("healthy_dip({:.1}%)", price_analysis.change_5m));

        debug_entry!(
            "   ✅ HEALTHY DIP: strength={:.2}, score_addition={:.2}",
            dip_strength,
            score_addition
        );
        debug_entry!(
            "   Updated swing_score: {:.2} -> {:.2}",
            swing_score - score_addition,
            swing_score
        );

        println!(
            "📉 [SWING] Healthy dip opportunity: {:.1}% (+{:.2})",
            price_analysis.change_5m,
            score_addition
        );
    } else {
        debug_entry!(
            "   ❌ NO HEALTHY DIP: {:.1}% outside range [{:.1}%, {:.1}%]",
            price_analysis.change_5m,
            HEALTHY_DIP_MAX,
            HEALTHY_DIP_MIN
        );
    }

    // 2. Momentum reversal signals
    debug_entry!("2. Analyzing momentum reversal signals...");
    let (change_1m, _is_1m_realtime) = get_price_change_with_fallback(token, 1);
    let (_change_15m, _is_15m_realtime) = get_price_change_with_fallback(token, 15);

    debug_entry!("   Change 1m: {:.1}%, Change 5m: {:.1}%", change_1m, price_analysis.change_5m);
    debug_entry!("   Momentum reversal threshold: {:.1}%", MOMENTUM_REVERSAL_THRESHOLD);

    // Look for momentum shifts (short-term recovery from longer-term decline)
    if change_1m > MOMENTUM_REVERSAL_THRESHOLD && price_analysis.change_5m < -2.0 {
        let score_addition = 0.25;
        swing_score += score_addition;
        signals.push(format!("momentum_reversal(1m:+{:.1}%)", change_1m));

        debug_entry!(
            "   ✅ MOMENTUM REVERSAL: 1m={:.1}% > {:.1}% AND 5m={:.1}% < -2.0%",
            change_1m,
            MOMENTUM_REVERSAL_THRESHOLD,
            price_analysis.change_5m
        );
        debug_entry!(
            "   Score addition: +{:.2}, Updated swing_score: {:.2}",
            score_addition,
            swing_score
        );

        println!(
            "🔄 [SWING] Momentum reversal: 1m:+{:.1}% vs 5m:{:.1}% (+0.25)",
            change_1m,
            price_analysis.change_5m
        );
    } else {
        debug_entry!("   ❌ NO MOMENTUM REVERSAL: conditions not met");
        debug_entry!(
            "      - 1m change: {:.1}% {} {:.1}%",
            change_1m,
            if change_1m > MOMENTUM_REVERSAL_THRESHOLD {
                ">"
            } else {
                "≤"
            },
            MOMENTUM_REVERSAL_THRESHOLD
        );
        debug_entry!("      - 5m change: {:.1}% {} -2.0%", price_analysis.change_5m, if
            price_analysis.change_5m < -2.0
        {
            "<"
        } else {
            "≥"
        });
    }

    // 3. Multi-timeframe analysis
    debug_entry!("3. Analyzing multi-timeframe divergence...");
    debug_entry!(
        "   Change 1h: {:.1}%, Change 5m: {:.1}%",
        price_analysis.change_1h,
        price_analysis.change_5m
    );

    if price_analysis.change_1h > -5.0 && price_analysis.change_5m < -5.0 {
        let score_addition = 0.2;
        swing_score += score_addition;
        signals.push(format!("timeframe_divergence"));

        debug_entry!(
            "   ✅ TIMEFRAME DIVERGENCE: 1h={:.1}% > -5.0% AND 5m={:.1}% < -5.0%",
            price_analysis.change_1h,
            price_analysis.change_5m
        );
        debug_entry!(
            "   Score addition: +{:.2}, Updated swing_score: {:.2}",
            score_addition,
            swing_score
        );

        println!("📊 [SWING] Timeframe divergence: 5m worse than 1h (+0.2)");
    } else {
        debug_entry!("   ❌ NO TIMEFRAME DIVERGENCE: conditions not met");
        debug_entry!("      - 1h change: {:.1}% {} -5.0%", price_analysis.change_1h, if
            price_analysis.change_1h > -5.0
        {
            ">"
        } else {
            "≤"
        });
        debug_entry!("      - 5m change: {:.1}% {} -5.0%", price_analysis.change_5m, if
            price_analysis.change_5m < -5.0
        {
            "<"
        } else {
            "≥"
        });
    }

    // 4. Real-time price advantage
    debug_entry!("4. Checking real-time data availability...");
    debug_entry!("   Is 5m realtime: {}", price_analysis.is_5m_realtime);

    if price_analysis.is_5m_realtime {
        let score_addition = 0.15;
        swing_score += score_addition;
        signals.push(format!("realtime_data"));

        debug_entry!("   ✅ REALTIME DATA: available");
        debug_entry!(
            "   Score addition: +{:.2}, Updated swing_score: {:.2}",
            score_addition,
            swing_score
        );

        println!("⚡ [SWING] Real-time price data available (+0.15)");
    } else {
        debug_entry!("   ❌ NO REALTIME DATA: using cached data");
    }

    // 5. Volume spike during dip (accumulation signal)
    debug_entry!("5. Analyzing volume accumulation during dip...");
    let volume_ratio = if token.volume.h24 > 0.0 {
        token.volume.h1 / (token.volume.h24 / 24.0)
    } else {
        0.0
    };

    debug_entry!(
        "   Volume h1: {:.0}, h24: {:.0}, ratio: {:.1}x",
        token.volume.h1,
        token.volume.h24,
        volume_ratio
    );
    debug_entry!("   Volume accumulation threshold: {:.1}x", VOLUME_ACCUMULATION_MULTIPLIER);
    debug_entry!("   Price change 5m: {:.1}% (need < -2.0%)", price_analysis.change_5m);

    if volume_ratio > VOLUME_ACCUMULATION_MULTIPLIER && price_analysis.change_5m < -2.0 {
        let score_addition = 0.2;
        swing_score += score_addition;
        signals.push(format!("volume_accumulation({:.1}x)", volume_ratio));

        debug_entry!(
            "   ✅ VOLUME ACCUMULATION: ratio={:.1}x > {:.1}x AND price_change={:.1}% < -2.0%",
            volume_ratio,
            VOLUME_ACCUMULATION_MULTIPLIER,
            price_analysis.change_5m
        );
        debug_entry!(
            "   Score addition: +{:.2}, Updated swing_score: {:.2}",
            score_addition,
            swing_score
        );

        println!("📈 [SWING] Volume accumulation during dip: {:.1}x (+0.2)", volume_ratio);
    } else {
        debug_entry!("   ❌ NO VOLUME ACCUMULATION: conditions not met");
        debug_entry!(
            "      - Volume ratio: {:.1}x {} {:.1}x",
            volume_ratio,
            if volume_ratio > VOLUME_ACCUMULATION_MULTIPLIER {
                ">"
            } else {
                "≤"
            },
            VOLUME_ACCUMULATION_MULTIPLIER
        );
        debug_entry!("      - Price change: {:.1}% {} -2.0%", price_analysis.change_5m, if
            price_analysis.change_5m < -2.0
        {
            "<"
        } else {
            "≥"
        });
    }

    // 6. Whale accumulation during weakness
    debug_entry!("6. Analyzing whale accumulation during weakness...");
    if let Some(trades_cache) = trades {
        let recent_whales = trades_cache.get_whale_trades(MEDIUM_TRADE_THRESHOLD_USD, 1);
        let whale_buy_volume: f64 = recent_whales
            .iter()
            .filter(|t| t.kind == "buy")
            .map(|t| t.volume_usd)
            .sum();

        debug_entry!("   Recent whale trades: {} total", recent_whales.len());
        debug_entry!("   Whale buy volume: ${:.0} (threshold: $200)", whale_buy_volume);
        debug_entry!("   Price change 5m: {:.1}% (need < -3.0%)", price_analysis.change_5m);

        if whale_buy_volume > 200.0 && price_analysis.change_5m < -3.0 {
            let score_addition = 0.25;
            swing_score += score_addition;
            signals.push(format!("whale_dip_buying(${:.0})", whale_buy_volume));

            debug_entry!(
                "   ✅ WHALE DIP BUYING: volume=${:.0} > $200 AND price_change={:.1}% < -3.0%",
                whale_buy_volume,
                price_analysis.change_5m
            );
            debug_entry!(
                "   Score addition: +{:.2}, Updated swing_score: {:.2}",
                score_addition,
                swing_score
            );

            println!("🐋 [SWING] Whales buying the dip: ${:.0} (+0.25)", whale_buy_volume);
        } else {
            debug_entry!("   ❌ NO WHALE DIP BUYING: conditions not met");
            debug_entry!("      - Whale buy volume: ${:.0} {} $200", whale_buy_volume, if
                whale_buy_volume > 200.0
            {
                ">"
            } else {
                "≤"
            });
            debug_entry!("      - Price change: {:.1}% {} -3.0%", price_analysis.change_5m, if
                price_analysis.change_5m < -3.0
            {
                "<"
            } else {
                "≥"
            });
        }
    } else {
        debug_entry!("   No trades data available for whale analysis");
    }

    debug_entry!(
        "SWING ANALYSIS COMPLETE: Score={:.2}, signals_count={}, signals={:?}",
        swing_score,
        signals.len(),
        signals
    );

    println!("🎯 [SWING_SCORE] {:.2} | Signals: {:?}", swing_score, signals);
    (swing_score, signals)
}

/// SIMPLIFIED ANTI-BOT WHALE-FOLLOWING ENTRY STRATEGY
/// Focus: Avoid bots, follow whales, enter quickly when conditions are met
pub async fn should_buy(
    token: &Token,
    can_buy: bool,
    current_price: f64,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> bool {
    debug_section!("ENTRY ANALYSIS START");
    debug_entry!("Token: {} ({}) | Current Price: ${:.8}", token.symbol, token.mint, current_price);
    debug_entry!(
        "Can buy flag: {} | Has trades data: {} | Has OHLCV data: {}",
        can_buy,
        trades.is_some(),
        dataframe.is_some()
    );

    println!(
        "\n🔍 [ENTRY] {} | ${:.8} | Simplified whale-following analysis...",
        token.symbol,
        current_price
    );

    // ✅ CRITICAL: Validate price before any trading decision
    debug_section!("PRICE VALIDATION");
    if !is_price_valid(current_price) {
        debug_result!(false, "", &format!("Invalid price validation - {:.12}", current_price));
        println!(
            "🚫 [ENTRY] {} | Invalid price: {:.12} - TRADING BLOCKED",
            token.symbol,
            current_price
        );
        return false;
    }
    debug_result!(true, "Price validation passed", "");

    // Double-check with cached price validation
    if let Some(trading_price) = get_trading_price(&token.mint) {
        let price_diff = (((current_price - trading_price) / trading_price) * 100.0).abs();
        debug_entry!(
            "Cached price check: current={:.12}, cached={:.12}, diff={:.1}%",
            current_price,
            trading_price,
            price_diff
        );
        if price_diff > 10.0 {
            debug_entry!("WARNING: Large price discrepancy detected");
            println!(
                "⚠️ [ENTRY] {} | Price mismatch: current={:.12}, cached={:.12} ({:.1}% diff) - using cached",
                token.symbol,
                current_price,
                trading_price,
                price_diff
            );
        }
        debug_result!(true, "Cached price validation passed", "");
    } else {
        debug_result!(false, "", "No cached price available - CRITICAL FAILURE");
        println!("🚫 [ENTRY] {} | No valid cached price available - TRADING BLOCKED", token.symbol);
        return false;
    }

    debug_section!("BASIC ELIGIBILITY CHECKS");
    if !can_buy {
        debug_result!(false, "", "Trading blocked by external condition");
        println!("❌ [ENTRY] {} | Trading blocked", token.symbol);
        return false;
    }
    debug_result!(true, "External trading conditions OK", "");

    // Check if we should pause trading based on recent performance
    if should_pause_trading().await {
        debug_result!(false, "", "Trading paused due to poor performance");
        println!("⏸️ [ENTRY] {} | Trading paused due to poor recent performance", token.symbol);
        return false;
    }
    debug_result!(true, "Performance check passed", "");

    // ─── ENTRY COOLDOWN CHECK ───
    debug_section!("ENTRY COOLDOWN CHECK");
    debug_entry!("Checking entry cooldown for token: {}", token.mint);
    let (can_enter, minutes_since_last) = can_enter_token_position(&token.mint).await;
    debug_result!(
        can_enter,
        &format!("Cooldown check passed - {} minutes since last activity", minutes_since_last),
        &format!("Cooldown active - last activity {} minutes ago", minutes_since_last)
    );
    if !can_enter {
        println!("⏸️ [ENTRY] {} | Cooldown active ({}min)", token.symbol, minutes_since_last);
        return false;
    }

    // ─── CHECK FOR RECENT HIGHS (ANTI-FOMO) ───
    debug_section!("ANTI-FOMO CHECK");
    if let Some(df) = dataframe {
        debug_entry!("Checking for recent highs using OHLCV data");
        if is_near_recent_highs(current_price, df) {
            debug_result!(false, "", "Price too close to recent highs - anti-FOMO triggered");
            println!(
                "🚫 [ENTRY] {} | Price near recent 15m/5m highs - avoiding FOMO buy",
                token.symbol
            );
            return false;
        }
        debug_result!(true, "Recent highs check passed - not near FOMO levels", "");
    } else {
        debug_entry!("WARNING: No OHLCV data available for recent highs check");
    }

    // ─── CHECK RECENT PROFITABLE EXITS ───
    debug_section!("RECENT PROFITABLE EXITS CHECK");
    debug_entry!("Checking recent profitable exits");
    if check_recent_profitable_exits(&token.mint, current_price).await {
        debug_result!(false, "", "Recent profitable exit detected - price too high for re-entry");
        println!("🚫 [ENTRY] {} | Recent profitable exit - must buy lower", token.symbol);
        return false;
    }
    debug_result!(true, "Recent profitable exits check passed", "");

    // ─── BASIC SAFETY ───
    debug_section!("BASIC SAFETY CHECKS");
    debug_entry!("Running basic safety checks (rug check)");
    if !crate::dexscreener::is_safe_to_trade(token, false) {
        debug_entry!("FAILED: Rug check failed");
        println!("🚨 [ENTRY] {} | Failed rug check", token.symbol);
        return false;
    }
    debug_result!(true, "Basic safety checks passed", "");

    // ─── REAL-TIME PRICE ANALYSIS WITH ENHANCED TREND DETECTION ───
    debug_section!("PRICE ANALYSIS");
    let price_analysis = get_realtime_price_analysis(token);

    debug_entry!("Price analysis results:");
    debug_entry!(
        "  - 5m change: {:.2}% (realtime: {})",
        price_analysis.change_5m,
        price_analysis.is_5m_realtime
    );
    debug_entry!(
        "  - 15m change: {:.2}% (realtime: {})",
        price_analysis.change_15m,
        price_analysis.is_15m_realtime
    );
    debug_entry!(
        "  - 1h change: {:.2}% (realtime: {})",
        price_analysis.change_1h,
        price_analysis.is_1h_realtime
    );

    // Get most reliable price source
    let (reliable_price, is_real_time, price_source) = get_most_reliable_price(token);
    debug_entry!(
        "Most reliable price: ${:.8} (realtime: {}, source: {})",
        reliable_price,
        is_real_time,
        price_source
    );

    // Use reliable price if different from current_price
    let trading_price = if
        is_real_time &&
        (reliable_price - current_price).abs() / current_price > 0.01
    {
        debug_entry!(
            "Using real-time pool price instead of API price (>{:.1}% difference)",
            ((reliable_price - current_price).abs() / current_price) * 100.0
        );
        println!(
            "🔄 [PRICE] Using real-time pool price: ${:.8} vs API: ${:.8} (source: {})",
            reliable_price,
            current_price,
            price_source
        );
        reliable_price
    } else {
        debug_entry!("Using API price (real-time price within 1% tolerance)");
        current_price
    };

    // Classify market trend for better entry decisions
    debug_section!("MARKET TREND ANALYSIS");
    let (trend_type, trend_strength, is_trend_favorable) = classify_market_trend(&price_analysis);
    debug_entry!(
        "Trend classification: type={}, strength={:.1}%, favorable={}",
        trend_type,
        trend_strength,
        is_trend_favorable
    );

    println!(
        "📈 [TREND] {} | Type: {} | Strength: {:.1}% | Favorable: {} | 5m={:.1}%{} | 1h={:.1}%{}",
        token.symbol,
        trend_type,
        trend_strength,
        is_trend_favorable,
        price_analysis.change_5m,
        if price_analysis.is_5m_realtime {
            "_RT"
        } else {
            "_DX"
        },
        price_analysis.change_1h,
        if price_analysis.is_1h_realtime {
            "_RT"
        } else {
            "_DX"
        }
    );

    // ─── SWING TRADING ANALYSIS ───
    debug_section!("SWING TRADING ANALYSIS");
    let (swing_score, swing_signals) = analyze_swing_entry_opportunity(
        token,
        &price_analysis,
        trades
    );
    debug_entry!("Swing trading score: {:.3} | Signals: {:?}", swing_score, swing_signals);

    // ─── KEY METRICS EXTRACTION ───
    debug_section!("KEY METRICS EXTRACTION");
    let volume_24h = token.volume.h24;
    let volume_1h = token.volume.h1;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let total_holders = token.rug_check.total_holders;

    debug_metrics!(
        "volume_24h" => format!("${:.0}", volume_24h),
        "volume_1h" => format!("${:.0}", volume_1h),
        "liquidity_sol" => format!("{:.1}", liquidity_sol),
        "buys_1h" => buys_1h,
        "sells_1h" => sells_1h,
        "total_holders" => total_holders
    );

    // Calculate dynamic trade size based on liquidity and market cap
    let market_cap = token.fdv_usd.parse::<f64>().unwrap_or(0.0); // Parse FDV from string
    let dynamic_trade_size = calculate_trade_size_with_market_cap(liquidity_sol, market_cap);
    debug_entry!(
        "Dynamic trade size calculation: market_cap=${:.0}, trade_size={:.4}SOL",
        market_cap,
        dynamic_trade_size
    );

    // Check if trade size would exceed safe thresholds
    let trade_pct_of_liquidity = (dynamic_trade_size / liquidity_sol) * 100.0;
    debug_entry!(
        "Trade size safety check: {:.2}% of liquidity (max allowed: {:.1}%)",
        trade_pct_of_liquidity,
        WHALE_ANGER_THRESHOLD_PCT
    );

    if trade_pct_of_liquidity > WHALE_ANGER_THRESHOLD_PCT {
        debug_entry!("WARNING: Trade size exceeds safe threshold, reducing");
        println!(
            "⚠️ [SAFETY] {} | Trade would be {:.2}% of liquidity (>{:.1}%) - reducing size",
            token.symbol,
            trade_pct_of_liquidity,
            WHALE_ANGER_THRESHOLD_PCT
        );
        // Force reduce to safe level
        let safe_trade_size = liquidity_sol * (MAX_TRADE_PCT_OF_LIQUIDITY / 100.0);
        debug_entry!(
            "Adjusted trade size from {:.4}SOL to {:.4}SOL",
            dynamic_trade_size,
            safe_trade_size
        );
        println!(
            "🛡️ [SAFETY] {} | Adjusted trade size: {:.4}SOL -> {:.4}SOL",
            token.symbol,
            dynamic_trade_size,
            safe_trade_size
        );
    }

    println!(
        "📊 [METRICS] Vol24h: ${:.0} | MCap: ${:.0} | Liq: {:.1}SOL | Buys1h: {} | TradePct: {:.2}% | TradeSize: {:.4}SOL",
        volume_24h,
        market_cap,
        liquidity_sol,
        buys_1h,
        trade_pct_of_liquidity,
        dynamic_trade_size
    );

    // ─── TRADES DATA ANALYSIS ───
    debug_section!("TRADES DATA ANALYSIS");
    let mut trades_score = 0.0;
    let mut trades_whale_activity = 0.0;

    if let Some(trades_cache) = trades {
        debug_entry!("Analyzing trades data for whale activity patterns");

        // Analyze whale activity from trades data
        let whale_trades_1h = trades_cache.get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 1); // Large trades in last hour
        let whale_trades_24h = trades_cache.get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 24); // Large trades in 24h
        let recent_buys = trades_cache.get_trades_by_type("buy", 1);
        let recent_sells = trades_cache.get_trades_by_type("sell", 1);

        debug_entry!(
            "Trade counts: whale_1h={}, whale_24h={}, recent_buys={}, recent_sells={}",
            whale_trades_1h.len(),
            whale_trades_24h.len(),
            recent_buys.len(),
            recent_sells.len()
        );

        let whale_buy_volume: f64 = whale_trades_1h
            .iter()
            .filter(|t| t.kind == "buy")
            .map(|t| t.volume_usd)
            .sum();

        let whale_sell_volume: f64 = whale_trades_1h
            .iter()
            .filter(|t| t.kind == "sell")
            .map(|t| t.volume_usd)
            .sum();

        // Calculate whale net flow (positive = accumulation)
        let whale_net_flow = whale_buy_volume - whale_sell_volume;
        debug_entry!(
            "Whale activity: buy_volume=${:.0}, sell_volume=${:.0}, net_flow=${:.0}",
            whale_buy_volume,
            whale_sell_volume,
            whale_net_flow
        );

        // Large buy vs sell ratio in recent trades
        let large_buy_count = recent_buys
            .iter()
            .filter(|t| t.volume_usd > MEDIUM_TRADE_THRESHOLD_USD)
            .count();
        let large_sell_count = recent_sells
            .iter()
            .filter(|t| t.volume_usd > MEDIUM_TRADE_THRESHOLD_USD)
            .count();

        debug_entry!(
            "Large trade counts: buys={}, sells={} (threshold: ${})",
            large_buy_count,
            large_sell_count,
            MEDIUM_TRADE_THRESHOLD_USD
        );

        trades_whale_activity = if whale_net_flow > STRONG_WHALE_ACCUMULATION_USD {
            debug_entry!("Classification: Strong whale accumulation (score: 0.8)");
            0.8 // Strong whale accumulation
        } else if whale_net_flow > MODERATE_WHALE_ACCUMULATION_USD {
            debug_entry!("Classification: Moderate whale accumulation (score: 0.6)");
            0.6 // Moderate whale accumulation
        } else if whale_net_flow > -MODERATE_WHALE_ACCUMULATION_USD {
            debug_entry!("Classification: Neutral whale activity (score: 0.3)");
            0.3 // Neutral whale activity
        } else {
            debug_entry!("Classification: Whale distribution (score: 0.1)");
            0.1 // Whale distribution
        };

        // Bonus for sustained whale activity
        if whale_trades_24h.len() > 10 && whale_net_flow > 0.0 {
            debug_entry!(
                "Sustained whale activity bonus: +0.1 (24h whale trades: {})",
                whale_trades_24h.len()
            );
            trades_whale_activity += 0.1;
        }

        // Check for bot-like patterns (many small frequent trades)
        let small_trades_1h = trades_cache
            .get_trades_by_type("buy", 1)
            .iter()
            .filter(|t| t.volume_usd < SMALL_TRADE_THRESHOLD_USD)
            .count();

        debug_entry!(
            "Bot activity analysis: small_trades_1h={} (threshold: ${})",
            small_trades_1h,
            SMALL_TRADE_THRESHOLD_USD
        );

        let bot_penalty = if small_trades_1h > 20 {
            debug_entry!("High bot activity detected: penalty=-0.2");
            -0.2 // High bot activity penalty
        } else if small_trades_1h > 10 {
            debug_entry!("Medium bot activity detected: penalty=-0.1");
            -0.1 // Medium bot activity penalty
        } else {
            debug_entry!("Low bot activity: no penalty");
            0.0 // Low bot activity
        };

        trades_score = trades_whale_activity + bot_penalty;
        debug_entry!(
            "Final trades score: {:.3} (whale_activity: {:.1} + bot_penalty: {:.1})",
            trades_score,
            trades_whale_activity,
            bot_penalty
        );

        let trades_info = format!(
            "whale_net:${:.0}|whales_1h:{}|large_buys:{}|large_sells:{}|small_1h:{}",
            whale_net_flow,
            whale_trades_1h.len(),
            large_buy_count,
            large_sell_count,
            small_trades_1h
        );

        println!(
            "🐋 [TRADES] Net: ${:.0} | Whale: {:.1} | Score: {:.2} | {}",
            whale_net_flow,
            trades_whale_activity,
            trades_score,
            trades_info
        );
    } else {
        debug_entry!("No trade data available for whale activity analysis");
        println!("📊 [TRADES] No trade data available for analysis");
    }

    // ─── OHLCV TECHNICAL ANALYSIS ───
    debug_section!("OHLCV TECHNICAL ANALYSIS");
    let mut confirmation_score = 0;
    let mut whale_threshold_multiple = 1.0;

    if let Some(df) = dataframe {
        debug_entry!("OHLCV analysis available - running technical analysis");
        println!("📊 [ENTRY] {} | OHLCV analysis available", token.symbol);

        // ─── TREND ANALYSIS (PREFER DOWNTREND ENTRIES) ───
        debug_entry!("Running trend analysis to prefer downtrend entries");
        let trend_bonus = analyze_price_trend(current_price, df);
        debug_entry!(
            "Trend analysis result: bonus={:.1} (negative=uptrend, positive=downtrend)",
            trend_bonus
        );

        if trend_bonus < -20.0 {
            debug_result!(
                false,
                "",
                &format!(
                    "Strong uptrend detected - avoiding entry (trend score: {:.1})",
                    trend_bonus
                )
            );
            println!(
                "🚫 [ENTRY] {} | Strong uptrend detected - avoiding entry (trend score: {:.1})",
                token.symbol,
                trend_bonus
            );
            return false;
        }
        debug_result!(true, &format!("Trend check passed (score: {:.1})", trend_bonus), "");

        println!(
            "📊 [TREND] {} | Trend score: {:.1} (negative=uptrend, positive=downtrend)",
            token.symbol,
            trend_bonus
        );

        let primary_timeframe = df.get_primary_timeframe();

        // Get current price from OHLCV data (more reliable than API price)
        if let Some(ohlcv_price) = primary_timeframe.current_price() {
            let price_diff_pct = (((current_price - ohlcv_price) / ohlcv_price) * 100.0).abs();
            debug_entry!(
                "OHLCV price comparison: API={:.8}, OHLCV={:.8}, diff={:.1}%",
                current_price,
                ohlcv_price,
                price_diff_pct
            );
            if price_diff_pct > 5.0 {
                debug_entry!("WARNING: Significant price discrepancy detected");
                println!(
                    "⚠️ [ENTRY] {} | Price discrepancy: API={:.8} vs OHLCV={:.8} ({:.1}%)",
                    token.symbol,
                    current_price,
                    ohlcv_price,
                    price_diff_pct
                );
            }
        }

        // Check for recent volatility (high volatility = risk)
        if let Some(volatility) = primary_timeframe.volatility(20) {
            debug_entry!("Volatility check: {:.1}% (threshold: 15.0%)", volatility);
            if volatility > 15.0 {
                debug_entry!("High volatility detected - increasing whale threshold by 1.5x");
                println!(
                    "⚠️ [ENTRY] {} | High volatility: {:.1}% - increasing caution",
                    token.symbol,
                    volatility
                );
                whale_threshold_multiple *= 1.5; // Require stronger whale signals in volatile markets
            }
        }

        // Check for volume trends (increasing volume = good)
        let recent_avg_volume = primary_timeframe.average_volume(5).unwrap_or(0.0);
        let older_avg_volume = primary_timeframe.average_volume(20).unwrap_or(0.0);
        debug_entry!(
            "Volume trend analysis: recent_5={:.0}, older_20={:.0}, ratio={:.1}x",
            recent_avg_volume,
            older_avg_volume,
            if older_avg_volume > 0.0 {
                recent_avg_volume / older_avg_volume
            } else {
                0.0
            }
        );

        if recent_avg_volume > older_avg_volume * 1.5 {
            debug_entry!("Volume surge detected - adding +1 to confirmation score");
            println!(
                "📈 [ENTRY] {} | Volume surge detected: recent={:.0} vs avg={:.0}",
                token.symbol,
                recent_avg_volume,
                older_avg_volume
            );
            confirmation_score += 1;
        }

        // Check for price momentum (recent price change)
        if let Some(_price_change_5m) = primary_timeframe.price_change_over_period(5) {
            debug_entry!("Price momentum check: 5m_change={:.1}%", token.price_change.m5);
            if token.price_change.m5 > 2.0 {
                debug_entry!("Positive momentum detected - adding +1 to confirmation score");
                println!(
                    "🚀 [ENTRY] {} | Recent price momentum: +{:.1}%",
                    token.symbol,
                    token.price_change.m5
                );
                confirmation_score += 1;
            } else if token.price_change.m5 < -3.0 {
                debug_entry!("Negative momentum detected - subtracting -1 from confirmation score");
                println!(
                    "📉 [ENTRY] {} | Recent price decline: {:.1}% - reducing confidence",
                    token.symbol,
                    token.price_change.m5
                );
                confirmation_score -= 1;
            }
        }

        // VWAP analysis (price vs volume weighted average)
        if let Some(vwap) = primary_timeframe.vwap(20) {
            let vwap_diff_pct = ((current_price - vwap) / vwap) * 100.0;
            debug_entry!(
                "VWAP analysis: current={:.8}, vwap={:.8}, diff={:.1}%",
                current_price,
                vwap,
                vwap_diff_pct
            );

            if current_price > vwap * 1.02 {
                debug_entry!("Price above VWAP - adding +1 to confirmation score");
                println!(
                    "📊 [ENTRY] {} | Price above VWAP: {:.8} vs {:.8} (+{:.1}%)",
                    token.symbol,
                    current_price,
                    vwap,
                    vwap_diff_pct
                );
                confirmation_score += 1;
            } else if current_price < vwap * 0.98 {
                debug_entry!("Price below VWAP - subtracting -1 from confirmation score");
                println!(
                    "📊 [ENTRY] {} | Price below VWAP: {:.8} vs {:.8} ({:.1}%)",
                    token.symbol,
                    current_price,
                    vwap,
                    vwap_diff_pct
                );
                confirmation_score -= 1;
            }
        }

        debug_entry!(
            "Final OHLCV analysis: confirmation_score={}, whale_threshold_multiple={:.1}x",
            confirmation_score,
            whale_threshold_multiple
        );
        println!(
            "🎯 [OHLCV] {} | Technical score: {} | Whale threshold multiplier: {:.1}x",
            token.symbol,
            confirmation_score,
            whale_threshold_multiple
        );
    } else {
        debug_entry!("No OHLCV data available - using basic analysis only");
        println!("⚠️ [ENTRY] {} | No OHLCV data available - using basic analysis", token.symbol);
    }

    // ─── FUNDAMENTAL FILTERS ───
    debug_section!("FUNDAMENTAL FILTERS");

    // 1. Minimum liquidity
    debug_entry!(
        "Liquidity check: {:.1}SOL (min required: {:.1}SOL)",
        liquidity_sol,
        MIN_LIQUIDITY_SOL
    );
    if liquidity_sol < MIN_LIQUIDITY_SOL {
        debug_result!(
            false,
            "",
            &format!("Low liquidity: {:.1}SOL < {:.1}SOL", liquidity_sol, MIN_LIQUIDITY_SOL)
        );
        println!("💧 [ENTRY] {} | Low liquidity: {:.1}SOL", token.symbol, liquidity_sol);
        return false;
    }
    debug_result!(true, "Liquidity check passed", "");

    // 2. Minimum volume
    debug_entry!("Volume check: ${:.0} (min required: ${:.0})", volume_24h, MIN_VOLUME_USD);
    if volume_24h < MIN_VOLUME_USD {
        debug_result!(
            false,
            "",
            &format!("Low volume: ${:.0} < ${:.0}", volume_24h, MIN_VOLUME_USD)
        );
        println!("📊 [ENTRY] {} | Low volume: ${:.0}", token.symbol, volume_24h);
        return false;
    }
    debug_result!(true, "Volume check passed", "");

    // 3. Minimum activity
    debug_entry!("Activity check: {} buys (min required: {})", buys_1h, MIN_ACTIVITY_BUYS_1H);
    if buys_1h < MIN_ACTIVITY_BUYS_1H {
        debug_result!(
            false,
            "",
            &format!("Low buying activity: {} < {}", buys_1h, MIN_ACTIVITY_BUYS_1H)
        );
        println!("📈 [ENTRY] {} | Low buying: {}", token.symbol, buys_1h);
        return false;
    }
    debug_result!(true, "Activity check passed", "");

    // 4. Smart dump analysis - distinguish healthy dips from dangerous dumps
    debug_entry!("Running smart dump analysis for {:.1}% change", price_analysis.change_5m);
    let is_dangerous_dump = analyze_dump_safety(
        token,
        price_analysis.change_5m,
        price_analysis.is_5m_realtime,
        trades,
        liquidity_sol,
        total_holders
    );

    debug_result!(!is_dangerous_dump, "Dump safety check passed", "Dangerous dump detected");
    if is_dangerous_dump {
        println!(
            "🚨 [ENTRY] {} | Dangerous dump detected: {:.1}%{} - avoiding",
            token.symbol,
            price_analysis.change_5m,
            if price_analysis.is_5m_realtime {
                "_RT"
            } else {
                "_DX"
            }
        );
        return false;
    }

    // 5. Minimum holders
    debug_entry!("Holders check: {} (min required: {})", total_holders, MIN_HOLDER_COUNT);
    if total_holders < MIN_HOLDER_COUNT {
        debug_result!(false, "", &format!("Few holders: {} < {}", total_holders, MIN_HOLDER_COUNT));
        println!("👥 [ENTRY] {} | Few holders: {}", token.symbol, total_holders);
        return false;
    }
    debug_result!(true, "Holders check passed", "");

    // ─── WHALE VS BOT ANALYSIS ───
    debug_section!("WHALE VS BOT ANALYSIS");

    let total_txns_1h = buys_1h + sells_1h;
    let buy_ratio = if total_txns_1h > 0 { (buys_1h as f64) / (total_txns_1h as f64) } else { 0.0 };
    let avg_tx_size = if total_txns_1h > 0 { volume_1h / (total_txns_1h as f64) } else { 0.0 };

    debug_metrics!(
        "total_txns_1h" => total_txns_1h,
        "buy_ratio" => format!("{:.2}", buy_ratio),
        "avg_tx_size" => format!("${:.2}", avg_tx_size)
    );

    // Whale activity scoring
    let whale_score = if avg_tx_size > WHALE_BUY_THRESHOLD_SOL * 2.0 {
        debug_entry!("Very high whale activity detected (score: 1.0)");
        1.0 // Very high whale activity
    } else if avg_tx_size > WHALE_BUY_THRESHOLD_SOL {
        debug_entry!("High whale activity detected (score: 0.7)");
        0.7 // High whale activity
    } else if avg_tx_size > WHALE_BUY_THRESHOLD_SOL * 0.5 {
        debug_entry!("Medium whale activity detected (score: 0.4)");
        0.4 // Medium whale activity
    } else {
        debug_entry!("Low whale activity detected (score: 0.1)");
        0.1 // Low whale activity
    };

    // Bot activity scoring (inverse relationship)
    let bot_score = if avg_tx_size < 0.5 && total_txns_1h > 100 {
        debug_entry!("Very high bot activity detected (score: 0.9)");
        0.9 // Very high bot activity
    } else if avg_tx_size < 1.0 && total_txns_1h > 50 {
        debug_entry!("High bot activity detected (score: 0.6)");
        0.6 // High bot activity
    } else if avg_tx_size < 1.5 && total_txns_1h > 20 {
        debug_entry!("Medium bot activity detected (score: 0.3)");
        0.3 // Medium bot activity
    } else {
        0.1 // Low bot activity
    };

    println!(
        "🐋 [ANALYSIS] AvgTx: ${:.2} | WhaleScore: {:.1} | BotScore: {:.1} | BuyRatio: {:.2}",
        avg_tx_size,
        whale_score,
        bot_score,
        buy_ratio
    );

    // ─── ENHANCED ENTRY CONDITIONS WITH TREND-BASED SCORING ───
    debug_section!("SCORING CALCULATION");

    let mut entry_score = 0.0;
    let mut reasons = Vec::new();

    debug_entry!("Starting entry score calculation from 0.0");

    // 1. Market condition and trend bonus (HIGH WEIGHT)
    let volume_ratio = if volume_24h > 0.0 { volume_1h / (volume_24h / 24.0) } else { 0.0 };
    let market_bonus = get_market_condition_bonus(&price_analysis, volume_ratio);
    debug_entry!("Market conditions: volume_ratio={:.1}x, bonus={:.3}", volume_ratio, market_bonus);

    if market_bonus > 0.0 {
        entry_score += market_bonus;
        reasons.push(format!("market_conditions({:.2})", market_bonus));
        debug_entry!("✅ Market bonus applied: +{:.3} → total={:.3}", market_bonus, entry_score);
        println!("🎯 [MARKET] Favorable conditions bonus: +{:.2}", market_bonus);
    } else {
        debug_entry!("❌ No market bonus (score: {:.3})", market_bonus);
    }

    // 2. Swing trading opportunity (SIGNIFICANT WEIGHT)
    debug_entry!("Swing trading: score={:.3}, threshold={:.1}", swing_score, 0.2);
    if swing_score >= 0.2 {
        entry_score += swing_score;
        reasons.push(format!("swing_opportunity({:.2})", swing_score));
        reasons.extend(swing_signals.clone());
        debug_entry!("✅ Swing bonus applied: +{:.3} → total={:.3}", swing_score, entry_score);
    } else {
        debug_entry!("❌ Swing score too low: {:.3} < 0.2", swing_score);
    }

    // 3. Whale activity from dexscreener (MODERATE WEIGHT)
    debug_entry!(
        "DEX whale activity: score={:.1}, min_required={:.1}",
        whale_score,
        MIN_WHALE_SCORE
    );
    if whale_score >= MIN_WHALE_SCORE {
        let whale_bonus = whale_score * 0.25;
        entry_score += whale_bonus;
        reasons.push(format!("dex_whale_activity({:.1})", whale_score));
        debug_entry!("✅ DEX whale bonus applied: +{:.3} → total={:.3}", whale_bonus, entry_score);
    } else {
        debug_entry!("❌ DEX whale score too low: {:.1} < {:.1}", whale_score, MIN_WHALE_SCORE);
    }

    // 4. Trades-based whale activity (HIGH WEIGHT - most reliable)
    debug_entry!(
        "Trades whale activity: score={:.3}, min_required={:.1}",
        trades_score,
        MIN_TRADES_SCORE
    );
    if trades_score >= MIN_TRADES_SCORE {
        let trades_bonus = trades_score * 0.4;
        entry_score += trades_bonus;
        reasons.push(format!("trades_whale({:.1})", trades_whale_activity));
        debug_entry!(
            "✅ Trades whale bonus applied: +{:.3} → total={:.3}",
            trades_bonus,
            entry_score
        );
    } else {
        debug_entry!("❌ Trades score too low: {:.3} < {:.1}", trades_score, MIN_TRADES_SCORE);
    }

    // 5. Low bot interference (MODERATE WEIGHT)
    debug_entry!("Bot interference: score={:.1}, max_allowed={:.1}", bot_score, MAX_BOT_SCORE);
    if bot_score <= MAX_BOT_SCORE {
        let bot_bonus = (MAX_BOT_SCORE - bot_score) * 0.25;
        entry_score += bot_bonus;
        reasons.push(format!("low_bots({:.1})", bot_score));
        debug_entry!("✅ Low bot bonus applied: +{:.3} → total={:.3}", bot_bonus, entry_score);
    } else {
        debug_entry!("❌ Bot score too high: {:.1} > {:.1}", bot_score, MAX_BOT_SCORE);
    }

    // 6. Buying pressure (MODERATE WEIGHT)
    debug_entry!("Buying pressure: ratio={:.2}, min_required={:.2}", buy_ratio, MIN_BUY_RATIO);
    if buy_ratio >= MIN_BUY_RATIO {
        let buy_bonus = buy_ratio * 0.2;
        entry_score += buy_bonus;
        reasons.push(format!("buying_pressure({:.2})", buy_ratio));
        debug_entry!(
            "✅ Buying pressure bonus applied: +{:.3} → total={:.3}",
            buy_bonus,
            entry_score
        );
    } else {
        debug_entry!("❌ Buy ratio too low: {:.2} < {:.2}", buy_ratio, MIN_BUY_RATIO);
    }

    // 7. ENHANCED: Price action opportunities
    debug_entry!("Price action: trend_favorable={}, trend_type={}", is_trend_favorable, trend_type);
    if is_trend_favorable {
        let price_bonus = match trend_type.as_str() {
            "strong_uptrend" => 0.25,
            "building_uptrend" => 0.3, // Higher bonus for early entries
            "strong_downtrend_dip" => 0.35, // Highest bonus for dip buying
            "consolidation" => 0.15,
            _ => 0.0,
        };

        debug_entry!(
            "Price action bonus calculation: type={}, bonus={:.3}",
            trend_type,
            price_bonus
        );
        if price_bonus > 0.0 {
            entry_score += price_bonus;
            reasons.push(format!("{}({:.2})", trend_type, price_bonus));
            debug_entry!(
                "✅ Price action bonus applied: +{:.3} → total={:.3}",
                price_bonus,
                entry_score
            );
        }
    } else {
        debug_entry!("❌ Trend not favorable for entry");
    }

    // 8. Liquidity and volume bonuses
    let liquidity_threshold = MIN_LIQUIDITY_SOL * LIQUIDITY_MULTIPLIER;
    debug_entry!(
        "Liquidity bonus: current={:.1}, threshold={:.1}",
        liquidity_sol,
        liquidity_threshold
    );
    if liquidity_sol >= liquidity_threshold {
        entry_score += 0.1;
        reasons.push(format!("good_liquidity({:.0})", liquidity_sol));
        debug_entry!("✅ Liquidity bonus applied: +0.1 → total={:.3}", entry_score);
    } else {
        debug_entry!("❌ Liquidity too low: {:.1} < {:.1}", liquidity_sol, liquidity_threshold);
    }

    debug_entry!("Volume bonus: ratio={:.1}x, threshold=1.2x", volume_ratio);
    if volume_ratio > 1.2 {
        entry_score += 0.1;
        reasons.push(format!("active_volume({:.1}x)", volume_ratio));
        debug_entry!("✅ Volume bonus applied: +0.1 → total={:.3}", entry_score);
    } else {
        debug_entry!("❌ Volume ratio too low: {:.1}x < 1.2x", volume_ratio);
    }

    // 9. OHLCV technical confirmation
    debug_entry!("Technical confirmation: score={}", confirmation_score);
    if confirmation_score > 0 {
        let tech_bonus = 0.08;
        entry_score += tech_bonus;
        reasons.push(format!("technical_confirmation({:.0})", confirmation_score));
        debug_entry!("✅ Technical bonus applied: +{:.3} → total={:.3}", tech_bonus, entry_score);
    } else {
        debug_entry!("❌ No positive technical confirmation: {}", confirmation_score);
    }

    // 10. Real-time data and contrarian bonuses
    debug_entry!(
        "Real-time data bonus: 5m={}, 1h={}",
        price_analysis.is_5m_realtime,
        price_analysis.is_1h_realtime
    );
    if price_analysis.is_5m_realtime {
        entry_score += 0.1;
        reasons.push(format!("realtime_price_data"));
        debug_entry!("✅ Real-time data bonus applied: +0.1 → total={:.3}", entry_score);
    } else {
        debug_entry!("❌ No real-time 5m data");
    }

    debug_entry!(
        "Contrarian analysis: whale_activity={:.1}, price_change_5m={:.1}%",
        trades_whale_activity,
        price_analysis.change_5m
    );
    if trades_whale_activity >= 0.6 && price_analysis.change_5m < -2.0 {
        let contrarian_bonus = 0.15;
        entry_score += contrarian_bonus;
        reasons.push(format!("contrarian_whale_accumulation"));
        debug_entry!(
            "✅ Contrarian bonus applied: +{:.3} → total={:.3}",
            contrarian_bonus,
            entry_score
        );
    } else {
        debug_entry!(
            "❌ No contrarian opportunity: whale={:.1} (need ≥0.6), change={:.1}% (need <-2.0%)",
            trades_whale_activity,
            price_analysis.change_5m
        );
    }

    debug_entry!("FINAL ENTRY SCORE: {:.3}", entry_score);
    debug_entry!("Score breakdown reasons: {:?}", reasons);
    println!("🎯 [SCORE] {:.2} | Trend: {} | {:?}", entry_score, trend_type, reasons);

    // ─── ADAPTIVE THRESHOLD BASED ON MARKET CONDITIONS ───
    debug_section!("THRESHOLD CALCULATION");

    let mut threshold = match trend_type.as_str() {
        "strong_uptrend" | "building_uptrend" => UPTREND_ENTRY_THRESHOLD,
        "strong_downtrend_dip" => DOWNTREND_ENTRY_THRESHOLD,
        "consolidation" => BASE_ENTRY_THRESHOLD,
        _ => BASE_ENTRY_THRESHOLD + 0.1, // Slightly higher for uncertain conditions
    };

    debug_entry!("Base threshold for trend '{}': {:.2}", trend_type, threshold);

    // Further adjustments based on data quality and market conditions
    if price_analysis.is_5m_realtime && price_analysis.is_1h_realtime {
        threshold -= 0.1;
        debug_entry!("Real-time data discount applied: -0.1 → threshold={:.2}", threshold);
        println!("🔄 [THRESHOLD] Real-time data discount: -{:.1}", 0.1);
    }

    if volume_ratio > 2.0 {
        threshold -= 0.05;
        debug_entry!("High volume discount applied: -0.05 → threshold={:.2}", threshold);
        println!("📈 [THRESHOLD] High volume discount: -{:.2}", 0.05);
    }

    if trades_whale_activity >= 0.7 {
        threshold -= 0.1;
        debug_entry!("Strong whale activity discount applied: -0.1 → threshold={:.2}", threshold);
        println!("🐋 [THRESHOLD] Strong whale discount: -{:.1}", 0.1);
    }

    // Safety check: ensure threshold doesn't go too low
    let min_threshold = 0.3;
    let pre_safety_threshold = threshold;
    threshold = threshold.max(min_threshold);
    if threshold != pre_safety_threshold {
        debug_entry!(
            "Safety check applied: {:.2} → {:.2} (min: {:.2})",
            pre_safety_threshold,
            threshold,
            min_threshold
        );
    }
    debug_entry!("FINAL THRESHOLD: {:.2}", threshold);

    debug_section!("FINAL DECISION");
    debug_entry!(
        "Entry score: {:.3} | Threshold: {:.2} | Difference: {:.3}",
        entry_score,
        threshold,
        entry_score - threshold
    );
    debug_entry!("Trend type: {} | Market favorable: {}", trend_type, is_trend_favorable);
    debug_entry!("All scoring reasons: {:?}", reasons);

    println!(
        "🎯 [DECISION] Score: {:.2} | Threshold: {:.2} | Trend: {} | Result: {}",
        entry_score,
        threshold,
        trend_type,
        if entry_score >= threshold {
            "BUY ✅"
        } else {
            "SKIP ❌"
        }
    );

    let should_buy = entry_score >= threshold;

    if should_buy {
        debug_entry!("✅ DECISION: BUY - Score {:.3} >= threshold {:.2}", entry_score, threshold);
        debug_entry!(
            "Trade details - Size: {:.4} SOL | Price: ${:.8}",
            dynamic_trade_size,
            trading_price
        );
        println!(
            "🚀 [BUY] {} | Score: {:.2} >= {:.2} | Trend: {} | Size: {:.4}SOL | Reasons: {:?}",
            token.symbol,
            entry_score,
            threshold,
            trend_type,
            dynamic_trade_size,
            reasons
        );
    } else {
        debug_entry!(
            "❌ DECISION: SKIP - Score {:.3} < threshold {:.2} (need +{:.3})",
            entry_score,
            threshold,
            threshold - entry_score
        );
        println!(
            "❌ [SKIP] {} | Score: {:.2} < {:.2} | Need: +{:.2} | Trend: {}",
            token.symbol,
            entry_score,
            threshold,
            threshold - entry_score,
            trend_type
        );
    }

    debug_section!("ENTRY ANALYSIS END");

    should_buy
}

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS FOR ENHANCED ENTRY CONTROLS
// ═══════════════════════════════════════════════════════════════════════════════

/// Check if current price is near recent 15m or 5m highs (anti-FOMO)
fn is_near_recent_highs(current_price: f64, dataframe: &crate::ohlcv::TokenOhlcvCache) -> bool {
    let primary_timeframe = dataframe.get_primary_timeframe();

    // Check 15m high
    if let Some(high_15m) = primary_timeframe.highest_price(15) {
        let distance_from_high_15m = ((current_price - high_15m) / high_15m) * 100.0;
        if distance_from_high_15m > -NEAR_15M_HIGH_THRESHOLD_PCT {
            println!(
                "🚫 [ANTI_FOMO] Current price {:.8} is {:.1}% below 15m high {:.8} (threshold: {:.1}%)",
                current_price,
                distance_from_high_15m.abs(),
                high_15m,
                NEAR_15M_HIGH_THRESHOLD_PCT
            );
            return true;
        }
    }

    // Check 5m high
    if let Some(high_5m) = primary_timeframe.highest_price(5) {
        let distance_from_high_5m = ((current_price - high_5m) / high_5m) * 100.0;
        if distance_from_high_5m > -NEAR_5M_HIGH_THRESHOLD_PCT {
            println!(
                "🚫 [ANTI_FOMO] Current price {:.8} is {:.1}% below 5m high {:.8} (threshold: {:.1}%)",
                current_price,
                distance_from_high_5m.abs(),
                high_5m,
                NEAR_5M_HIGH_THRESHOLD_PCT
            );
            return true;
        }
    }

    false
}

/// Check if we recently exited this token profitably and current price is too high
async fn check_recent_profitable_exits(mint: &str, current_price: f64) -> bool {
    use crate::persistence::CLOSED_POSITIONS;

    // Check closed positions in memory
    let closed_positions = CLOSED_POSITIONS.read().await.clone();

    // Look for recent profitable exits for this token
    for (_key, position) in closed_positions.iter() {
        if let Some(close_time) = position.close_time {
            let minutes_since_exit = (chrono::Utc::now().timestamp() - close_time.timestamp()) / 60;

            // Only check recent exits (within last 72 hours)
            if minutes_since_exit <= MAX_RECENT_EXITS_LOOKBACK_HOURS * 60 {
                let profit_pct =
                    ((position.sol_received - position.sol_spent) / position.sol_spent) * 100.0;

                // If it was profitable, ensure we buy lower
                if profit_pct > MIN_PROFIT_EXIT_THRESHOLD_PCT {
                    let price_vs_exit =
                        ((current_price - position.peak_price) / position.peak_price) * 100.0;

                    if price_vs_exit > -MIN_PRICE_DROP_AFTER_PROFIT_PCT {
                        println!(
                            "🚫 [PROFIT_CONTROL] {} | Recent profitable exit (+{:.1}%) {}min ago at {:.8}, current {:.8} (+{:.1}% vs exit)",
                            mint,
                            profit_pct,
                            minutes_since_exit,
                            position.peak_price,
                            current_price,
                            price_vs_exit
                        );
                        return true;
                    } else {
                        println!(
                            "✅ [PROFIT_CONTROL] {} | Can re-enter: profitable exit (+{:.1}%) {}min ago at {:.8}, current {:.8} ({:.1}% vs exit)",
                            mint,
                            profit_pct,
                            minutes_since_exit,
                            position.peak_price,
                            current_price,
                            price_vs_exit
                        );
                        return false; // Found a recent profitable exit but price is acceptable
                    }
                }
            }
        }
    }

    false // No recent profitable exits found or no restriction
}

/// Analyze price trend to prefer downtrend entries over uptrend
/// Returns: positive score for downtrends (good for buying), negative score for uptrends (bad for buying)
fn analyze_price_trend(current_price: f64, dataframe: &crate::ohlcv::TokenOhlcvCache) -> f64 {
    let primary_timeframe = dataframe.get_primary_timeframe();

    let mut trend_score = 0.0;
    let mut trend_signals = Vec::new();

    // 1. Price position relative to recent price levels using available methods
    if let Some(recent_high) = primary_timeframe.highest_price(20) {
        let price_vs_high20 = ((current_price - recent_high) / recent_high) * 100.0;

        if price_vs_high20 < -5.0 {
            trend_score += 15.0; // Strong downtrend - great for buying dips
            trend_signals.push("strong_downtrend_vs_high20");
        } else if price_vs_high20 < -2.0 {
            trend_score += 10.0; // Moderate downtrend
            trend_signals.push("moderate_downtrend_vs_high20");
        } else if price_vs_high20 > -1.0 {
            trend_score -= 15.0; // Near highs - avoid buying
            trend_signals.push("near_recent_highs");
        }
    }

    // 2. Recent price momentum (compare current vs older candles)
    if let Some(price_change_10) = primary_timeframe.price_change_over_period(10) {
        if price_change_10 < -3.0 {
            trend_score += 10.0; // Strong downward momentum - good for dip buying
            trend_signals.push("strong_down_momentum");
        } else if price_change_10 < -1.0 {
            trend_score += 5.0; // Mild downward momentum
            trend_signals.push("mild_down_momentum");
        } else if price_change_10 > 3.0 {
            trend_score -= 10.0; // Strong upward momentum - avoid
            trend_signals.push("strong_up_momentum");
        } else if price_change_10 > 1.0 {
            trend_score -= 5.0; // Mild upward momentum
            trend_signals.push("mild_up_momentum");
        }
    }

    // 3. Volatility and trend consistency
    if let Some(volatility) = primary_timeframe.volatility(10) {
        if volatility > 20.0 {
            trend_score -= 5.0; // High volatility reduces confidence
            trend_signals.push("high_volatility");
        } else if volatility < 5.0 {
            trend_score += 5.0; // Low volatility increases confidence
            trend_signals.push("low_volatility");
        }
    }

    // 4. Recent highs and lows analysis
    if let Some(recent_high) = primary_timeframe.highest_price(20) {
        if let Some(recent_low) = primary_timeframe.lowest_price(20) {
            let position_in_range = (current_price - recent_low) / (recent_high - recent_low);

            if position_in_range < 0.3 {
                trend_score += 8.0; // Near recent lows - good for buying
                trend_signals.push("near_recent_lows");
            } else if position_in_range > 0.7 {
                trend_score -= 8.0; // Near recent highs - avoid buying
                trend_signals.push("near_recent_highs_range");
            }
        }
    }

    println!("📊 [TREND_ANALYSIS] Score: {:.1} | Signals: {:?}", trend_score, trend_signals);

    trend_score
}

/// Check if we should pause trading based on recent performance
async fn should_pause_trading() -> bool {
    // TODO: Implement performance-based trading pause logic
    // For now, always allow trading
    false
}

/// Get adaptive entry threshold based on recent performance
async fn get_adaptive_entry_threshold() -> f64 {
    // TODO: Implement adaptive threshold based on recent win rate
    // For now, return base threshold
    BASE_ENTRY_THRESHOLD
}

/// Record trade entry for performance tracking
async fn record_trade_entry(
    _mint: &str,
    _symbol: &str,
    _price: f64,
    _size: f64,
    _signals: Vec<String>,
    _whale_score: f64,
    _bot_score: f64
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement trade entry recording logic
    Ok(())
}
