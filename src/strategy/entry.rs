use crate::prelude::*;
use crate::price_validation::{ is_price_valid, get_trading_price };
use super::config::*;
use super::position::can_enter_token_position;
use super::price_analysis::{
    get_realtime_price_analysis,
    calculate_trade_size_sol,
    get_price_change_with_fallback,
};
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
        return false;
    }

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
    if change_5m <= EXTREME_DUMP_THRESHOLD {
        danger_signals += 3;
        println!("🚨 [DIP] Extreme dump: {:.1}% (+3 danger)", change_5m);
    } else if change_5m <= DANGEROUS_DUMP_THRESHOLD {
        danger_signals += 2;
        println!("⚠️ [DIP] Major dump: {:.1}% (+2 danger)", change_5m);
    } else if change_5m <= HEALTHY_DIP_MAX {
        danger_signals += 1;
        println!("📉 [DIP] Significant dump: {:.1}% (+1 danger)", change_5m);
    } else {
        safety_signals += 1;
        println!("✅ [DIP] Moderate dip: {:.1}% (+1 safety)", change_5m);
    }

    // 2. Liquidity safety check
    if liquidity_sol < 5.0 {
        danger_signals += 2;
        println!("💧 [DIP] Very low liquidity: {:.1}SOL (+2 danger)", liquidity_sol);
    } else if liquidity_sol < 15.0 {
        danger_signals += 1;
        println!("💧 [DIP] Low liquidity: {:.1}SOL (+1 danger)", liquidity_sol);
    } else if liquidity_sol > 50.0 {
        safety_signals += 1;
        println!("💪 [DIP] Strong liquidity: {:.1}SOL (+1 safety)", liquidity_sol);
    }

    // 3. Holder count safety
    if total_holders < 5 {
        danger_signals += 2;
        println!("👥 [DIP] Very few holders: {} (+2 danger)", total_holders);
    } else if total_holders < 20 {
        danger_signals += 1;
        println!("👥 [DIP] Few holders: {} (+1 danger)", total_holders);
    } else if total_holders > 100 {
        safety_signals += 1;
        println!("👥 [DIP] Good holder base: {} (+1 safety)", total_holders);
    }

    // 4. Whale activity during the dump
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

        if whale_buys.len() > whale_sells.len() && whale_buys.len() > 0 {
            safety_signals += 2;
            println!(
                "🐋 [DIP] Whales buying the dip: {} buys vs {} sells (+2 safety)",
                whale_buys.len(),
                whale_sells.len()
            );
        } else if whale_sells.len() > whale_buys.len() * 2 {
            danger_signals += 2;
            println!(
                "🐋 [DIP] Whale exodus: {} sells vs {} buys (+2 danger)",
                whale_sells.len(),
                whale_buys.len()
            );
        }

        // Check for panic selling (many small sells)
        let small_sells = trades_cache
            .get_trades_by_type("sell", 1)
            .iter()
            .filter(|t| t.volume_usd < SMALL_TRADE_THRESHOLD_USD)
            .count();

        if small_sells > (PANIC_SELLING_THRESHOLD as usize) {
            danger_signals += 1;
            println!("😰 [DIP] Panic selling detected: {} small sells (+1 danger)", small_sells);
        }
    }

    // 5. Volume analysis
    let volume_ratio = if token.volume.h24 > 0.0 {
        token.volume.h1 / (token.volume.h24 / 24.0)
    } else {
        0.0
    };

    if volume_ratio > 3.0 {
        safety_signals += 1;
        println!("📊 [DIP] High volume during drop: {:.1}x avg (+1 safety)", volume_ratio);
    } else if volume_ratio < 0.5 {
        danger_signals += 1;
        println!("📊 [DIP] Low volume dump: {:.1}x avg (+1 danger)", volume_ratio);
    }

    let total_signals = danger_signals + safety_signals;
    let danger_ratio = if total_signals > 0 {
        (danger_signals as f64) / (total_signals as f64)
    } else {
        0.5
    };

    println!(
        "⚖️ [DIP_RESULT] Danger: {} | Safety: {} | Ratio: {:.1}%",
        danger_signals,
        safety_signals,
        danger_ratio * 100.0
    );

    // Consider dangerous if danger signals outweigh safety significantly
    danger_ratio > MAX_DANGER_RATIO
}

/// Enhanced swing trading entry analysis for dip opportunities
fn analyze_swing_entry_opportunity(
    token: &Token,
    price_analysis: &PriceAnalysis,
    trades: Option<&TokenTradesCache>
) -> (f64, Vec<String>) {
    let mut swing_score = 0.0;
    let mut signals = Vec::new();

    // 1. Dip buying opportunity (healthy pullbacks)
    if price_analysis.change_5m >= HEALTHY_DIP_MAX && price_analysis.change_5m <= HEALTHY_DIP_MIN {
        let dip_strength = (
            (price_analysis.change_5m.abs() - HEALTHY_DIP_MIN.abs()) /
            (HEALTHY_DIP_MAX.abs() - HEALTHY_DIP_MIN.abs())
        ).min(1.0);
        swing_score += dip_strength * 0.3;
        signals.push(format!("healthy_dip({:.1}%)", price_analysis.change_5m));
        println!(
            "📉 [SWING] Healthy dip opportunity: {:.1}% (+{:.2})",
            price_analysis.change_5m,
            dip_strength * 0.3
        );
    }

    // 2. Momentum reversal signals
    let (change_1m, _is_1m_realtime) = get_price_change_with_fallback(token, 1);
    let (_change_15m, _is_15m_realtime) = get_price_change_with_fallback(token, 15);

    // Look for momentum shifts (short-term recovery from longer-term decline)
    if change_1m > MOMENTUM_REVERSAL_THRESHOLD && price_analysis.change_5m < -2.0 {
        swing_score += 0.25;
        signals.push(format!("momentum_reversal(1m:+{:.1}%)", change_1m));
        println!(
            "🔄 [SWING] Momentum reversal: 1m:+{:.1}% vs 5m:{:.1}% (+0.25)",
            change_1m,
            price_analysis.change_5m
        );
    }

    // 3. Multi-timeframe analysis
    if price_analysis.change_1h > -5.0 && price_analysis.change_5m < -5.0 {
        swing_score += 0.2;
        signals.push(format!("timeframe_divergence"));
        println!("📊 [SWING] Timeframe divergence: 5m worse than 1h (+0.2)");
    }

    // 4. Real-time price advantage
    if price_analysis.is_5m_realtime {
        swing_score += 0.15;
        signals.push(format!("realtime_data"));
        println!("⚡ [SWING] Real-time price data available (+0.15)");
    }

    // 5. Volume spike during dip (accumulation signal)
    let volume_ratio = if token.volume.h24 > 0.0 {
        token.volume.h1 / (token.volume.h24 / 24.0)
    } else {
        0.0
    };

    if volume_ratio > VOLUME_ACCUMULATION_MULTIPLIER && price_analysis.change_5m < -2.0 {
        swing_score += 0.2;
        signals.push(format!("volume_accumulation({:.1}x)", volume_ratio));
        println!("📈 [SWING] Volume accumulation during dip: {:.1}x (+0.2)", volume_ratio);
    }

    // 6. Whale accumulation during weakness
    if let Some(trades_cache) = trades {
        let recent_whales = trades_cache.get_whale_trades(MEDIUM_TRADE_THRESHOLD_USD, 1);
        let whale_buy_volume: f64 = recent_whales
            .iter()
            .filter(|t| t.kind == "buy")
            .map(|t| t.volume_usd)
            .sum();

        if whale_buy_volume > 200.0 && price_analysis.change_5m < -3.0 {
            swing_score += 0.25;
            signals.push(format!("whale_dip_buying(${:.0})", whale_buy_volume));
            println!("🐋 [SWING] Whales buying the dip: ${:.0} (+0.25)", whale_buy_volume);
        }
    }

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
    println!(
        "\n🔍 [ENTRY] {} | ${:.8} | Simplified whale-following analysis...",
        token.symbol,
        current_price
    );

    // ✅ CRITICAL: Validate price before any trading decision
    if !is_price_valid(current_price) {
        println!(
            "🚫 [ENTRY] {} | Invalid price: {:.12} - TRADING BLOCKED",
            token.symbol,
            current_price
        );
        return false;
    }

    // Double-check with cached price validation
    if let Some(trading_price) = get_trading_price(&token.mint) {
        let price_diff = (((current_price - trading_price) / trading_price) * 100.0).abs();
        if price_diff > 10.0 {
            println!(
                "⚠️ [ENTRY] {} | Price mismatch: current={:.12}, cached={:.12} ({:.1}% diff) - using cached",
                token.symbol,
                current_price,
                trading_price,
                price_diff
            );
        }
    } else {
        println!("🚫 [ENTRY] {} | No valid cached price available - TRADING BLOCKED", token.symbol);
        return false;
    }

    if !can_buy {
        println!("❌ [ENTRY] {} | Trading blocked", token.symbol);
        return false;
    }

    // Check if we should pause trading based on recent performance
    if should_pause_trading().await {
        println!("⏸️ [ENTRY] {} | Trading paused due to poor recent performance", token.symbol);
        return false;
    }

    // ─── ENTRY COOLDOWN CHECK ───
    let (can_enter, minutes_since_last) = can_enter_token_position(&token.mint).await;
    if !can_enter {
        println!("⏸️ [ENTRY] {} | Cooldown active ({}min)", token.symbol, minutes_since_last);
        return false;
    }

    // ─── CHECK FOR RECENT HIGHS (ANTI-FOMO) ───
    if let Some(df) = dataframe {
        if is_near_recent_highs(current_price, df) {
            println!(
                "🚫 [ENTRY] {} | Price near recent 15m/5m highs - avoiding FOMO buy",
                token.symbol
            );
            return false;
        }
    }

    // ─── CHECK RECENT PROFITABLE EXITS ───
    if check_recent_profitable_exits(&token.mint, current_price) {
        println!("🚫 [ENTRY] {} | Recent profitable exit - must buy lower", token.symbol);
        return false;
    }

    // ─── BASIC SAFETY ───
    if !crate::dexscreener::is_safe_to_trade(token, false) {
        println!("🚨 [ENTRY] {} | Failed rug check", token.symbol);
        return false;
    }

    // ─── REAL-TIME PRICE ANALYSIS ───
    let price_analysis = get_realtime_price_analysis(token);

    println!(
        "📈 [PRICE] Real-time analysis: 5m={:.1}% 1h={:.1}% | Sources: {}",
        price_analysis.change_5m,
        price_analysis.change_1h,
        price_analysis.get_data_source_info()
    );

    // ─── SWING TRADING ANALYSIS ───
    let (swing_score, swing_signals) = analyze_swing_entry_opportunity(
        token,
        &price_analysis,
        trades
    );

    // ─── KEY METRICS ───
    let volume_24h = token.volume.h24;
    let volume_1h = token.volume.h1;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let total_holders = token.rug_check.total_holders;

    // Calculate dynamic trade size based on liquidity
    let dynamic_trade_size = calculate_trade_size_sol(liquidity_sol);

    println!(
        "📊 [METRICS] Vol24h: ${:.0} | Liq: {:.1}SOL | Buys1h: {} | Price5m: {:.1}%{} | Holders: {} | TradeSize: {:.4}SOL",
        volume_24h,
        liquidity_sol,
        buys_1h,
        price_analysis.change_5m,
        if price_analysis.is_5m_realtime {
            "_RT"
        } else {
            "_DX"
        },
        total_holders,
        dynamic_trade_size
    );

    // ─── TRADES DATA ANALYSIS ───
    let mut trades_score = 0.0;
    let mut trades_whale_activity = 0.0;

    if let Some(trades_cache) = trades {
        // Analyze whale activity from trades data
        let whale_trades_1h = trades_cache.get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 1); // Large trades in last hour
        let whale_trades_24h = trades_cache.get_whale_trades(LARGE_TRADE_THRESHOLD_USD, 24); // Large trades in 24h
        let recent_buys = trades_cache.get_trades_by_type("buy", 1);
        let recent_sells = trades_cache.get_trades_by_type("sell", 1);

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

        // Large buy vs sell ratio in recent trades
        let large_buy_count = recent_buys
            .iter()
            .filter(|t| t.volume_usd > MEDIUM_TRADE_THRESHOLD_USD)
            .count();
        let large_sell_count = recent_sells
            .iter()
            .filter(|t| t.volume_usd > MEDIUM_TRADE_THRESHOLD_USD)
            .count();

        trades_whale_activity = if whale_net_flow > STRONG_WHALE_ACCUMULATION_USD {
            0.8 // Strong whale accumulation
        } else if whale_net_flow > MODERATE_WHALE_ACCUMULATION_USD {
            0.6 // Moderate whale accumulation
        } else if whale_net_flow > -MODERATE_WHALE_ACCUMULATION_USD {
            0.3 // Neutral whale activity
        } else {
            0.1 // Whale distribution
        };

        // Bonus for sustained whale activity
        if whale_trades_24h.len() > 10 && whale_net_flow > 0.0 {
            trades_whale_activity += 0.1;
        }

        // Check for bot-like patterns (many small frequent trades)
        let small_trades_1h = trades_cache
            .get_trades_by_type("buy", 1)
            .iter()
            .filter(|t| t.volume_usd < SMALL_TRADE_THRESHOLD_USD)
            .count();

        let bot_penalty = if small_trades_1h > 20 {
            -0.2 // High bot activity penalty
        } else if small_trades_1h > 10 {
            -0.1 // Medium bot activity penalty
        } else {
            0.0 // Low bot activity
        };

        trades_score = trades_whale_activity + bot_penalty;

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
        println!("📊 [TRADES] No trade data available for analysis");
    }

    // ─── OHLCV TECHNICAL ANALYSIS ───
    let mut confirmation_score = 0;
    let mut whale_threshold_multiple = 1.0;

    if let Some(df) = dataframe {
        println!("📊 [ENTRY] {} | OHLCV analysis available", token.symbol);

        // ─── TREND ANALYSIS (PREFER DOWNTREND ENTRIES) ───
        let trend_bonus = analyze_price_trend(current_price, df);

        if trend_bonus < -20.0 {
            println!(
                "🚫 [ENTRY] {} | Strong uptrend detected - avoiding entry (trend score: {:.1})",
                token.symbol,
                trend_bonus
            );
            return false;
        }

        println!(
            "📊 [TREND] {} | Trend score: {:.1} (negative=uptrend, positive=downtrend)",
            token.symbol,
            trend_bonus
        );

        let primary_timeframe = df.get_primary_timeframe();

        // Get current price from OHLCV data (more reliable than API price)
        if let Some(ohlcv_price) = primary_timeframe.current_price() {
            let price_diff_pct = (((current_price - ohlcv_price) / ohlcv_price) * 100.0).abs();
            if price_diff_pct > 5.0 {
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
            if volatility > 15.0 {
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

        if recent_avg_volume > older_avg_volume * 1.5 {
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
            if token.price_change.m5 > 2.0 {
                println!(
                    "🚀 [ENTRY] {} | Recent price momentum: +{:.1}%",
                    token.symbol,
                    token.price_change.m5
                );
                confirmation_score += 1;
            } else if token.price_change.m5 < -3.0 {
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
            if current_price > vwap * 1.02 {
                println!(
                    "📊 [ENTRY] {} | Price above VWAP: {:.8} vs {:.8} (+{:.1}%)",
                    token.symbol,
                    current_price,
                    vwap,
                    ((current_price - vwap) / vwap) * 100.0
                );
                confirmation_score += 1;
            } else if current_price < vwap * 0.98 {
                println!(
                    "📊 [ENTRY] {} | Price below VWAP: {:.8} vs {:.8} ({:.1}%)",
                    token.symbol,
                    current_price,
                    vwap,
                    ((current_price - vwap) / vwap) * 100.0
                );
                confirmation_score -= 1;
            }
        }

        println!(
            "🎯 [OHLCV] {} | Technical score: {} | Whale threshold multiplier: {:.1}x",
            token.symbol,
            confirmation_score,
            whale_threshold_multiple
        );
    } else {
        println!("⚠️ [ENTRY] {} | No OHLCV data available - using basic analysis", token.symbol);
    }

    // ─── FUNDAMENTAL FILTERS ───

    // 1. Minimum liquidity
    if liquidity_sol < MIN_LIQUIDITY_SOL {
        println!("💧 [ENTRY] {} | Low liquidity: {:.1}SOL", token.symbol, liquidity_sol);
        return false;
    }

    // 2. Minimum volume
    if volume_24h < MIN_VOLUME_USD {
        println!("📊 [ENTRY] {} | Low volume: ${:.0}", token.symbol, volume_24h);
        return false;
    }

    // 3. Minimum activity
    if buys_1h < MIN_ACTIVITY_BUYS_1H {
        println!("📈 [ENTRY] {} | Low buying: {}", token.symbol, buys_1h);
        return false;
    }

    // 4. Smart dump analysis - distinguish healthy dips from dangerous dumps
    let is_dangerous_dump = analyze_dump_safety(
        token,
        price_analysis.change_5m,
        price_analysis.is_5m_realtime,
        trades,
        liquidity_sol,
        total_holders
    );

    if is_dangerous_dump {
        println!(
            "� [ENTRY] {} | Dangerous dump detected: {:.1}%{} - avoiding",
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
    if total_holders < MIN_HOLDER_COUNT {
        println!("👥 [ENTRY] {} | Few holders: {}", token.symbol, total_holders);
        return false;
    }

    // ─── WHALE VS BOT ANALYSIS ───

    let total_txns_1h = buys_1h + sells_1h;
    let buy_ratio = if total_txns_1h > 0 { (buys_1h as f64) / (total_txns_1h as f64) } else { 0.0 };
    let avg_tx_size = if total_txns_1h > 0 { volume_1h / (total_txns_1h as f64) } else { 0.0 };

    // Whale activity scoring
    let whale_score = if avg_tx_size > WHALE_BUY_THRESHOLD_SOL * 2.0 {
        1.0 // Very high whale activity
    } else if avg_tx_size > WHALE_BUY_THRESHOLD_SOL {
        0.7 // High whale activity
    } else if avg_tx_size > WHALE_BUY_THRESHOLD_SOL * 0.5 {
        0.4 // Medium whale activity
    } else {
        0.1 // Low whale activity
    };

    // Bot activity scoring (inverse relationship)
    let bot_score = if avg_tx_size < 0.5 && total_txns_1h > 100 {
        0.9 // Very high bot activity
    } else if avg_tx_size < 1.0 && total_txns_1h > 50 {
        0.6 // High bot activity
    } else if avg_tx_size < 1.5 && total_txns_1h > 20 {
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

    // ─── ENTRY CONDITIONS ───

    let mut entry_score = 0.0;
    let mut reasons = Vec::new();

    // ENHANCED: Add swing trading score (significant weight for dip opportunities)
    if swing_score >= MIN_SWING_SCORE_THRESHOLD {
        entry_score += swing_score; // Direct swing score addition
        reasons.push(format!("swing_opportunity({:.2})", swing_score));
        reasons.extend(swing_signals.clone());
    }

    // Strong whale activity (from dexscreener data)
    if whale_score >= 0.6 {
        entry_score += 0.25; // Slightly reduced to balance with swing signals
        reasons.push(format!("dex_whale_activity({:.1})", whale_score));
    }

    // Trades-based whale activity (more accurate)
    if trades_score >= 0.5 {
        entry_score += 0.35; // Still high weight for real trade data
        reasons.push(format!("trades_whale({:.1})", trades_whale_activity));
    }

    // ENHANCED: Bonus for whale activity during price weakness (contrarian signal)
    if trades_whale_activity >= 0.6 && price_analysis.change_5m < -2.0 {
        entry_score += 0.2;
        reasons.push(format!("contrarian_whale_accumulation"));
        println!("💪 [SWING] Contrarian whale accumulation during weakness (+0.2)");
    }

    // Low bot interference
    if bot_score <= 0.4 {
        entry_score += 0.15; // Slightly reduced weight
        reasons.push(format!("low_bots({:.1})", bot_score));
    }

    // Good buying pressure
    if buy_ratio >= 0.6 {
        entry_score += 0.15;
        reasons.push(format!("buying_pressure({:.2})", buy_ratio));
    }

    // ENHANCED: More flexible price movement analysis
    let price_movement_score = if
        price_analysis.change_5m >= HEALTHY_DIP_MAX &&
        price_analysis.change_5m <= HEALTHY_DIP_MIN
    {
        // Healthy dip range - bonus points
        0.2
    } else if
        price_analysis.change_5m >= HEALTHY_DIP_MIN &&
        price_analysis.change_5m <= ACCUMULATION_PATIENCE_THRESHOLD
    {
        // Normal accumulation range
        0.15
    } else if price_analysis.change_5m > ACCUMULATION_PATIENCE_THRESHOLD {
        // Mild FOMO territory - reduced score
        0.05
    } else {
        // Major dump territory - handled by dump analysis above
        0.0
    };

    if price_movement_score > 0.0 {
        entry_score += price_movement_score;
        reasons.push(format!("price_movement({:.1}%)", price_analysis.change_5m));
    }

    // Good liquidity
    if liquidity_sol >= MIN_LIQUIDITY_SOL * 2.0 {
        entry_score += 0.1;
        reasons.push(format!("good_liquidity({:.0})", liquidity_sol));
    }

    // Reasonable volume activity
    if volume_1h > volume_24h / 24.0 {
        entry_score += 0.1;
        reasons.push(format!("active_volume"));
    }

    // Add OHLCV technical analysis score
    if confirmation_score > 0 {
        entry_score += 0.1;
        reasons.push(format!("technical_confirmation({:.0})", confirmation_score));
    }

    // ENHANCED: Real-time data bonus
    if price_analysis.is_5m_realtime {
        entry_score += 0.1;
        reasons.push(format!("realtime_price_data"));
    }

    // ENHANCED: Trend analysis bonus/penalty
    if let Some(_) = dataframe {
        let trend_bonus = analyze_price_trend(current_price, dataframe.unwrap());
        if trend_bonus > 0.0 {
            // Positive score for downtrend (good for buying dips)
            let trend_score = (trend_bonus / 100.0).min(0.25); // Cap at 0.25
            entry_score += trend_score;
            reasons.push(format!("downtrend_dip_opportunity({:.2})", trend_score));
            println!(
                "📉 [TREND] Downtrend bonus: +{:.2} (trend score: {:.1})",
                trend_score,
                trend_bonus
            );
        } else if trend_bonus > -10.0 {
            // Small penalty for mild uptrend
            let trend_penalty = (trend_bonus.abs() / 100.0).min(0.1);
            entry_score -= trend_penalty;
            reasons.push(format!("mild_uptrend_penalty(-{:.2})", trend_penalty));
            println!(
                "⚠️ [TREND] Mild uptrend penalty: -{:.2} (trend score: {:.1})",
                trend_penalty,
                trend_bonus
            );
        }
        // Strong uptrends are already blocked above
    }

    println!("🎯 [SCORE] {:.2} | {:?}", entry_score, reasons);

    // ─── FINAL DECISION WITH ADAPTIVE THRESHOLDS ───
    let base_threshold = get_adaptive_entry_threshold().await;

    // ENHANCED: Dynamic threshold adjustment based on market conditions
    let mut adjusted_threshold = base_threshold;

    // Lower threshold for high-quality swing opportunities
    if swing_score >= STRONG_SWING_SCORE_THRESHOLD {
        adjusted_threshold -= SWING_THRESHOLD_REDUCTION_STRONG;
        println!(
            "📉 [THRESHOLD] Strong swing opportunity - lowering threshold by {:.2}",
            SWING_THRESHOLD_REDUCTION_STRONG
        );
    } else if swing_score >= MIN_SWING_SCORE_THRESHOLD {
        adjusted_threshold -= SWING_THRESHOLD_REDUCTION_MODERATE;
        println!(
            "📉 [THRESHOLD] Moderate swing opportunity - lowering threshold by {:.2}",
            SWING_THRESHOLD_REDUCTION_MODERATE
        );
    }

    // Lower threshold for strong whale accumulation during weakness
    if trades_whale_activity >= 0.7 && price_analysis.change_5m < HEALTHY_DIP_MIN {
        adjusted_threshold -= WHALE_CONTRARIAN_THRESHOLD_REDUCTION;
        println!(
            "🐋 [THRESHOLD] Strong contrarian whale activity - lowering threshold by {:.2}",
            WHALE_CONTRARIAN_THRESHOLD_REDUCTION
        );
    }

    // Lower threshold for real-time data advantage
    if price_analysis.is_5m_realtime {
        adjusted_threshold -= REALTIME_DATA_THRESHOLD_REDUCTION;
        println!(
            "⚡ [THRESHOLD] Real-time data advantage - lowering threshold by {:.2}",
            REALTIME_DATA_THRESHOLD_REDUCTION
        );
    }

    // Ensure minimum threshold
    adjusted_threshold = adjusted_threshold.max(MIN_ADAPTIVE_THRESHOLD);

    // ENHANCED: More flexible whale and bot requirements for swing trades
    let whale_requirement = if swing_score >= STRONG_SWING_SCORE_THRESHOLD - 0.1 {
        0.3
    } else {
        0.4
    }; // Lower whale requirement for good swings
    let bot_limit = if swing_score >= STRONG_SWING_SCORE_THRESHOLD - 0.1 { 0.7 } else { 0.6 }; // More lenient bot limit for good swings

    if
        entry_score >= adjusted_threshold &&
        whale_score >= whale_requirement &&
        bot_score <= bot_limit
    {
        println!(
            "✅ [ENTRY] {} | APPROVED | Score: {:.2} | Whale: {:.1} | Bot: {:.1} | Threshold: {:.2} (adjusted from {:.2}) | Swing: {:.2}",
            token.symbol,
            entry_score,
            whale_score,
            bot_score,
            adjusted_threshold,
            base_threshold,
            swing_score
        );

        // Record the entry for performance tracking
        let mut entry_signals: Vec<String> = reasons
            .iter()
            .map(|r| r.clone())
            .collect();

        // Add swing signals to entry record
        entry_signals.extend(swing_signals);

        let _result = record_trade_entry(
            &token.mint,
            &token.symbol,
            current_price,
            dynamic_trade_size,
            entry_signals,
            whale_score,
            bot_score
        ).await;

        return true;
    }

    println!(
        "❌ [ENTRY] {} | REJECTED | Score: {:.2} < {:.2} | Need: {:.2} more | Whale: {:.1} (need: {:.1}) | Bot: {:.1} (max: {:.1}) | Swing: {:.2}",
        token.symbol,
        entry_score,
        adjusted_threshold,
        adjusted_threshold - entry_score,
        whale_score,
        whale_requirement,
        bot_score,
        bot_limit,
        swing_score
    );
    false
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
fn check_recent_profitable_exits(mint: &str, current_price: f64) -> bool {
    use crate::persistence::CLOSED_POSITIONS;

    // Check closed positions in memory
    let rt = tokio::runtime::Handle::current();
    let closed_positions = rt.block_on(async { CLOSED_POSITIONS.read().await.clone() });

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
