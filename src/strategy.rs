use crate::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// ENHANCED ANTI-BOT WHALE-FOLLOWING MEME COIN STRATEGY V2.0
// ═══════════════════════════════════════════════════════════════════════════════
//
// ⚠️  OPTIMIZED FOR SOLANA MEME TRADING WITH HEAVY BOT MANIPULATION
//
// 🎯 CORE OBJECTIVES:
// • Follow whale accumulation patterns while avoiding bot front-running
// • Use historical performance data to adapt strategy parameters
// • Take quick profits to offset inevitable rug pull losses
// • Minimize bot detection through unpredictable entry timing
// • Never sell at loss - hold losers until recovery or rug
//
// 🤖 ENHANCED ANTI-BOT MEASURES:
// • Transaction pattern analysis to detect bot vs whale activity
// • Entry timing randomization to avoid predictable patterns
// • Whale/retail ratio analysis using average transaction size
// • Volume spike detection to avoid pump schemes
// • Multiple confirmation signals before entry
//
// 🐋 IMPROVED WHALE DETECTION:
// • Large transaction monitoring (>2 SOL threshold)
// • Accumulation phase identification (low volatility + whale buys)
// • Distribution phase avoidance (high sell pressure from large holders)
// • Smart money following vs retail FOMO detection
//
// 💰 AGGRESSIVE PROFIT OPTIMIZATION:
// • Quick profit targets: 0.5%, 1%, 2%, 4%, 8%, 15%+
// • Take profits on ANY negative momentum when profitable
// • Faster exits to capture more winning trades
// • Historical win rate tracking for strategy adaptation
//
// 🔄 ADAPTIVE RISK MANAGEMENT:
// • Performance-based position sizing (reduce after losses)
// • Token blacklisting after failed trades
// • DCA only during confirmed whale accumulation
// • Emergency exits on bot flood detection
//
// 📊 TARGET METRICS:
// • Win rate: 65-75% (more small wins, fewer big losses)
// • Average win: 1-8% (quick scalps preferred)
// • Risk/reward: 2:1 minimum (2% avg win vs 1% avg loss)
// • Rug loss offset: 10+ small wins per rug loss
// ═══════════════════════════════════════════════════════════════════════════════

pub const POSITIONS_CHECK_TIME_SEC: u64 = 15;
pub const TOKEN_DISCOVERY_CHECK_TIME_SEC: u64 = 30;

// TRADING CONSTANTS - OPTIMIZED FOR 0.001 SOL TRADES
pub const TRADE_SIZE_SOL: f64 = 0.001; // Your specified trade size
pub const MAX_OPEN_POSITIONS: usize = 20; // Reduced for better management
pub const MIN_HOLD_TIME_SECONDS: i64 = 30; // Faster exits allowed
pub const MAX_HOLD_TIME_SECONDS: i64 = 21600; // 6 hours max hold time
pub const MAX_DCA_COUNT: u8 = 1; // Only 1 DCA round to limit risk
pub const DCA_COOLDOWN_MINUTES: i64 = 30; // Faster DCA attempts
pub const DCA_BASE_TRIGGER_PCT: f64 = -15.0; // DCA trigger at -15%

pub const TRANSACTION_FEE_SOL: f64 = 0.000015; // Transaction fee
pub const POSITIONS_PRINT_TIME: u64 = 10; // Print every 10 seconds
pub const SLIPPAGE_BPS: f64 = 1.0; // Slightly higher slippage for execution
pub const DCA_SIZE_FACTOR: f64 = 1.0; // Same size DCA as initial

// STRATEGY CONSTANTS - SIMPLIFIED & EFFECTIVE
pub const MIN_VOLUME_USD: f64 = 3000.0; // Lowered for more opportunities
pub const MIN_LIQUIDITY_SOL: f64 = 8.0; // Lowered for early tokens

// ENHANCED ENTRY LOGIC - FOCUS ON BOT AVOIDANCE
pub const MIN_ACTIVITY_BUYS_1H: u64 = 3; // Minimum buying activity
pub const BIG_DUMP_THRESHOLD: f64 = -25.0; // Avoid major dumps
pub const ENTRY_COOLDOWN_MINUTES: i64 = 15; // Faster re-entry
pub const ACCUMULATION_PATIENCE_THRESHOLD: f64 = 12.0; // Allow more pump

// WHALE DETECTION CONSTANTS
pub const WHALE_BUY_THRESHOLD_SOL: f64 = 2.0; // Lower whale threshold
pub const MIN_HOLDER_COUNT: u64 = 10; // Lower holder requirement

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
    let (can_enter, minutes_since_last) = can_enter_token_position(&token.mint);
    if !can_enter {
        println!("⏸️ [ENTRY] {} | Cooldown active ({}min)", token.symbol, minutes_since_last);
        return false;
    }

    // ─── BASIC SAFETY ───
    if !crate::dexscreener::is_safe_to_trade(token, false) {
        println!("🚨 [ENTRY] {} | Failed rug check", token.symbol);
        return false;
    }

    // ─── KEY METRICS ───
    let volume_24h = token.volume.h24;
    let volume_1h = token.volume.h1;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let price_change_5m = token.price_change.m5;
    let _price_change_1h = token.price_change.h1;
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let total_holders = token.rug_check.total_holders;

    println!(
        "📊 [METRICS] Vol24h: ${:.0} | Liq: {:.1}SOL | Buys1h: {} | Price5m: {:.1}% | Holders: {}",
        volume_24h,
        liquidity_sol,
        buys_1h,
        price_change_5m,
        total_holders
    );

    // ─── TRADES DATA ANALYSIS ───
    let mut trades_score = 0.0;
    let mut trades_whale_activity = 0.0;
    let mut trades_info = String::from("no_data");

    if let Some(trades_cache) = trades {
        // Analyze whale activity from trades data
        let whale_trades_1h = trades_cache.get_whale_trades(100.0, 1); // $100+ trades in last hour
        let whale_trades_24h = trades_cache.get_whale_trades(100.0, 24); // $100+ trades in 24h
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
            .filter(|t| t.volume_usd > 50.0)
            .count();
        let large_sell_count = recent_sells
            .iter()
            .filter(|t| t.volume_usd > 50.0)
            .count();

        trades_whale_activity = if whale_net_flow > 500.0 {
            0.8 // Strong whale accumulation
        } else if whale_net_flow > 100.0 {
            0.6 // Moderate whale accumulation
        } else if whale_net_flow > -100.0 {
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
            .filter(|t| t.volume_usd < 10.0)
            .count();

        let bot_penalty = if small_trades_1h > 20 {
            -0.2 // High bot activity penalty
        } else if small_trades_1h > 10 {
            -0.1 // Medium bot activity penalty
        } else {
            0.0 // Low bot activity
        };

        trades_score = trades_whale_activity + bot_penalty;

        trades_info = format!(
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
        if let Some(price_change_5m) = primary_timeframe.price_change_over_period(5) {
            if price_change_5m > 2.0 {
                println!(
                    "🚀 [ENTRY] {} | Recent price momentum: +{:.1}%",
                    token.symbol,
                    price_change_5m
                );
                confirmation_score += 1;
            } else if price_change_5m < -3.0 {
                println!(
                    "📉 [ENTRY] {} | Recent price decline: {:.1}% - reducing confidence",
                    token.symbol,
                    price_change_5m
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

    // 4. Avoid major dumps
    if price_change_5m <= BIG_DUMP_THRESHOLD {
        println!("📉 [ENTRY] {} | Major dump: {:.1}%", token.symbol, price_change_5m);
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

    // Strong whale activity (from dexscreener data)
    if whale_score >= 0.6 {
        entry_score += 0.3; // Reduced weight
        reasons.push(format!("dex_whale_activity({:.1})", whale_score));
    }

    // Trades-based whale activity (more accurate)
    if trades_score >= 0.5 {
        entry_score += 0.4; // Higher weight for real trade data
        reasons.push(format!("trades_whale({:.1})", trades_whale_activity));
    }

    // Low bot interference
    if bot_score <= 0.4 {
        entry_score += 0.2; // Reduced weight
        reasons.push(format!("low_bots({:.1})", bot_score));
    }

    // Good buying pressure
    if buy_ratio >= 0.6 {
        entry_score += 0.15;
        reasons.push(format!("buying_pressure({:.2})", buy_ratio));
    }

    // Controlled price movement (not FOMO)
    if price_change_5m >= -10.0 && price_change_5m <= ACCUMULATION_PATIENCE_THRESHOLD {
        entry_score += 0.15;
        reasons.push(format!("controlled_movement({:.1}%)", price_change_5m));
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

    // OHLCV Technical Analysis (if available)
    if let Some(df) = dataframe {
        println!("📊 [ENTRY] {} | OHLCV analysis available", token.symbol);

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
        if let Some(price_change_5m) = primary_timeframe.price_change_over_period(5) {
            if price_change_5m > 2.0 {
                println!(
                    "🚀 [ENTRY] {} | Recent price momentum: +{:.1}%",
                    token.symbol,
                    price_change_5m
                );
                confirmation_score += 1;
            } else if price_change_5m < -3.0 {
                println!(
                    "📉 [ENTRY] {} | Recent price decline: {:.1}% - reducing confidence",
                    token.symbol,
                    price_change_5m
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
    } else {
        println!("⚠️ [ENTRY] {} | No OHLCV data available - using basic analysis", token.symbol);
    }

    println!("🎯 [SCORE] {:.2} | {:?}", entry_score, reasons);

    // ─── FINAL DECISION ───
    let required_score = get_adaptive_entry_threshold().await; // Use adaptive threshold

    if entry_score >= required_score && whale_score >= 0.4 && bot_score <= 0.6 {
        println!(
            "✅ [ENTRY] {} | APPROVED | Score: {:.2} | Whale: {:.1} | Bot: {:.1} | Threshold: {:.2}",
            token.symbol,
            entry_score,
            whale_score,
            bot_score,
            required_score
        );

        // Record the entry for performance tracking
        let entry_signals: Vec<String> = reasons
            .iter()
            .map(|r| r.clone())
            .collect();
        let _result = record_trade_entry(
            &token.mint,
            &token.symbol,
            current_price,
            TRADE_SIZE_SOL,
            entry_signals,
            whale_score,
            bot_score
        ).await;

        return true;
    }

    println!(
        "❌ [ENTRY] {} | REJECTED | Score: {:.2} < {:.2} | Need: {:.2} more | Adaptive threshold: {:.2}",
        token.symbol,
        entry_score,
        required_score,
        required_score - entry_score,
        required_score
    );
    false
}

/// SIMPLIFIED WHALE-AWARE DCA STRATEGY
/// Only DCA when whales are also accumulating, not panic selling
pub fn should_dca(
    token: &Token,
    pos: &Position,
    current_price: f64,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> bool {
    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;

    println!(
        "\n🔄 [DCA] {} | Drop: {:.1}% | DCA: {}/{} | Held: {}min",
        token.symbol,
        drop_pct,
        pos.dca_count,
        MAX_DCA_COUNT,
        elapsed.num_minutes()
    );

    // 1. Hard limits
    if pos.dca_count >= MAX_DCA_COUNT {
        println!("❌ [DCA] {} | Max DCA reached", token.symbol);
        return false;
    }

    // 2. Cooldown check
    if pos.dca_count > 0 && (now - pos.last_dca_time).num_minutes() < DCA_COOLDOWN_MINUTES {
        println!("⏰ [DCA] {} | Cooldown active", token.symbol);
        return false;
    }

    // 3. Minimum hold time
    if elapsed.num_minutes() < 15 {
        println!("⏰ [DCA] {} | Hold longer", token.symbol);
        return false;
    }

    // 4. Drop requirement
    if drop_pct > DCA_BASE_TRIGGER_PCT {
        println!(
            "📈 [DCA] {} | Drop insufficient: {:.1}% > {:.1}%",
            token.symbol,
            drop_pct,
            DCA_BASE_TRIGGER_PCT
        );
        return false;
    }

    // 5. Liquidity check
    if liquidity_sol < MIN_LIQUIDITY_SOL {
        println!("💧 [DCA] {} | Low liquidity: {:.1}SOL", token.symbol, liquidity_sol);
        return false;
    }

    // 6. Whale activity check (both dexscreener and trades data)
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buy_ratio = if buys_1h + sells_1h > 0 {
        (buys_1h as f64) / ((buys_1h + sells_1h) as f64)
    } else {
        0.0
    };

    let mut whale_accumulation_signal = false;

    if let Some(trades_cache) = trades {
        // Check for recent whale accumulation in trades data
        let whale_trades_30min = trades_cache.get_whale_trades(50.0, 0); // Last 30 min whale trades
        let recent_whale_buys: f64 = whale_trades_30min
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
            .sum();

        let recent_whale_sells: f64 = whale_trades_30min
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
            .sum();

        let whale_net_flow = recent_whale_buys - recent_whale_sells;

        if whale_net_flow > 100.0 {
            // Whales are net buying
            whale_accumulation_signal = true;
            println!(
                "🐋 [DCA] {} | Whale accumulation detected: ${:.0} net flow",
                token.symbol,
                whale_net_flow
            );
        } else {
            println!(
                "📉 [DCA] {} | No whale accumulation: ${:.0} net flow",
                token.symbol,
                whale_net_flow
            );
        }
    }

    // ─── OHLCV TECHNICAL ANALYSIS FOR DCA ───
    let mut technical_signal = false;

    if let Some(df) = dataframe {
        let primary_timeframe = df.get_primary_timeframe();

        // Check if current price is near support levels (recent lows)
        let recent_candles = primary_timeframe.get_recent_candles(60); // Last hour
        if !recent_candles.is_empty() {
            let recent_low = recent_candles
                .iter()
                .map(|c| c.low)
                .fold(f64::INFINITY, f64::min);
            let price_above_recent_low = ((current_price - recent_low) / recent_low) * 100.0;

            if price_above_recent_low < 2.0 {
                technical_signal = true;
                println!(
                    "📊 [DCA] {} | Price near recent support: current={:.8} vs low={:.8} (+{:.1}%)",
                    token.symbol,
                    current_price,
                    recent_low,
                    price_above_recent_low
                );
            }
        }

        // Check for volume confirmation (increasing volume during dip = good)
        let recent_avg_volume = primary_timeframe.average_volume(3).unwrap_or(0.0);
        let older_avg_volume = primary_timeframe.average_volume(10).unwrap_or(0.0);

        if recent_avg_volume > older_avg_volume * 1.3 {
            technical_signal = true;
            println!(
                "📈 [DCA] {} | Volume increase during dip: recent={:.0} vs avg={:.0}",
                token.symbol,
                recent_avg_volume,
                older_avg_volume
            );
        }

        // Check volatility - avoid DCA during extreme volatility
        if let Some(volatility) = primary_timeframe.volatility(10) {
            if volatility > 25.0 {
                println!(
                    "⚠️ [DCA] {} | Extreme volatility: {:.1}% - avoiding DCA",
                    token.symbol,
                    volatility
                );
                return false;
            }
        }
    }

    // Require either good buying pressure OR whale accumulation signal OR technical signal
    if buy_ratio < 0.4 && !whale_accumulation_signal && !technical_signal {
        println!(
            "📉 [DCA] {} | Poor conditions: buy_ratio={:.2}, whale_signal={}, technical_signal={}",
            token.symbol,
            buy_ratio,
            whale_accumulation_signal,
            technical_signal
        );
        return false;
    }

    println!(
        "✅ [DCA] {} | APPROVED | Drop: {:.1}% | BuyRatio: {:.2}",
        token.symbol,
        drop_pct,
        buy_ratio
    );
    true
}

/// AGGRESSIVE PROFIT-TAKING SELL STRATEGY
/// Take profits quickly to offset rug losses
pub fn should_sell(
    token: &Token,
    pos: &Position,
    current_price: f64,
    trades: Option<&TokenTradesCache>,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> (bool, String) {
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

    // 1. Minimum hold time
    if held_duration < MIN_HOLD_TIME_SECONDS {
        return (false, format!("min_hold_time({}s)", held_duration));
    }

    // 2. NEVER sell at loss
    if profit_pct <= 0.0 {
        println!("📉 [SELL] {} | HOLD: Never sell at loss: {:.2}%", token.symbol, profit_pct);
        return (false, format!("no_loss_selling({:.2}%)", profit_pct));
    }

    println!(
        "✅ [SELL] {} | Profitable: {:.2}% - checking exit conditions...",
        token.symbol,
        profit_pct
    );

    // ─── TRADES DATA ANALYSIS FOR SELLING ───
    let mut whale_distribution_detected = false;
    let mut sell_pressure_multiplier = 1.0;

    if let Some(trades_cache) = trades {
        // Check for whale distribution (large sells)
        let recent_whale_sells = trades_cache
            .get_whale_trades(100.0, 0) // Last 30 min
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
            .get_whale_trades(100.0, 0) // Last 30 min
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

        if whale_net_flow < -200.0 {
            // Heavy whale distribution
            whale_distribution_detected = true;
            sell_pressure_multiplier = 1.5; // Be more aggressive with profit taking
            println!(
                "🚨 [SELL] {} | Whale distribution detected: ${:.0} net outflow",
                token.symbol,
                whale_net_flow.abs()
            );
        } else if whale_net_flow < -50.0 {
            // Moderate distribution
            sell_pressure_multiplier = 1.2;
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
    let mut technical_sell_signal = false;
    let mut momentum_multiplier = 1.0;

    if let Some(df) = dataframe {
        let primary_timeframe = df.get_primary_timeframe();

        // Check for bearish momentum
        if let Some(price_change_recent) = primary_timeframe.price_change_over_period(3) {
            if price_change_recent < -1.0 {
                technical_sell_signal = true;
                momentum_multiplier = 1.3; // Be more aggressive
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

            if distance_from_high < 1.0 {
                technical_sell_signal = true;
                momentum_multiplier *= 1.2;
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

        if recent_avg_volume < older_avg_volume * 0.7 && profit_pct > 2.0 {
            technical_sell_signal = true;
            println!(
                "📉 [SELL] {} | Volume declining on pump: recent={:.0} vs avg={:.0}",
                token.symbol,
                recent_avg_volume,
                older_avg_volume
            );
        }

        // VWAP check - if price significantly above VWAP and profitable, consider selling
        if let Some(vwap) = primary_timeframe.vwap(20) {
            if current_price > vwap * 1.05 && profit_pct > 1.0 {
                momentum_multiplier *= 1.15;
                println!(
                    "📊 [SELL] {} | Price extended above VWAP: {:.8} vs {:.8} (+{:.1}%)",
                    token.symbol,
                    current_price,
                    vwap,
                    ((current_price - vwap) / vwap) * 100.0
                );
            }
        }

        println!(
            "🎯 [SELL TECH] {} | Signal: {}, Momentum mult: {:.1}x",
            token.symbol,
            technical_sell_signal,
            momentum_multiplier
        );
    }

    // Combine all multipliers
    sell_pressure_multiplier *= momentum_multiplier;

    // 3. AGGRESSIVE PROFIT TAKING (Enhanced with trades data)

    // Apply sell pressure multiplier to thresholds
    let weak_threshold = -2.0 * sell_pressure_multiplier;
    let medium_threshold = -3.0 * sell_pressure_multiplier;
    let strong_threshold = -5.0 * sell_pressure_multiplier;

    // Emergency exit on whale distribution
    if whale_distribution_detected && profit_pct > 0.3 {
        println!(
            "🚨 [SELL] {} | WHALE DUMP: {:.2}% profit + distribution",
            token.symbol,
            profit_pct
        );
        return (true, format!("whale_distribution({:.2}%)", profit_pct));
    }

    // Quick profits (0.5-3%) - Take profit on any weakness
    if profit_pct >= 0.5 && profit_pct < 3.0 {
        if token.price_change.m5 < weak_threshold || drop_from_peak > 5.0 {
            println!("💸 [SELL] {} | QUICK PROFIT: {:.2}% + weakness", token.symbol, profit_pct);
            return (true, format!("quick_profit({:.2}%)", profit_pct));
        }
    }

    // Small profits (3-10%) - Take profit on negative momentum
    if profit_pct >= 3.0 && profit_pct < 10.0 {
        if
            token.price_change.m5 < medium_threshold ||
            drop_from_peak > 10.0 / sell_pressure_multiplier
        {
            println!("💸 [SELL] {} | SMALL PROFIT: {:.2}% + momentum", token.symbol, profit_pct);
            return (true, format!("small_profit({:.2}%)", profit_pct));
        }
    }

    // Medium profits (10-25%) - Use trailing stops
    if profit_pct >= 10.0 && profit_pct < 25.0 {
        if
            drop_from_peak > 15.0 / sell_pressure_multiplier ||
            token.price_change.m5 < strong_threshold
        {
            println!("💸 [SELL] {} | MEDIUM PROFIT: {:.2}% + trailing", token.symbol, profit_pct);
            return (true, format!("medium_profit({:.2}%)", profit_pct));
        }
    }

    // Large profits (25%+) - Let them run with wider stops
    if profit_pct >= 25.0 {
        if
            drop_from_peak > 25.0 / sell_pressure_multiplier ||
            token.price_change.m5 < strong_threshold * 1.5
        {
            println!(
                "💸 [SELL] {} | LARGE PROFIT: {:.2}% + wide trailing",
                token.symbol,
                profit_pct
            );
            return (true, format!("large_profit({:.2}%)", profit_pct));
        }
    }

    // 4. Emergency exits
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    if liquidity_sol < MIN_LIQUIDITY_SOL * 0.3 {
        println!("🚨 [SELL] {} | LIQUIDITY CRISIS: {:.1}SOL", token.symbol, liquidity_sol);
        return (true, format!("liquidity_crisis({:.1}SOL)", liquidity_sol));
    }

    // 5. Maximum hold time for profitable positions
    if held_duration >= MAX_HOLD_TIME_SECONDS && profit_pct > 0.0 {
        println!("⏰ [SELL] {} | MAX HOLD TIME: {}min", token.symbol, held_minutes);
        return (true, format!("max_hold_time({:.2}%)", profit_pct));
    }

    // Default: Hold
    println!("🔒 [SELL] {} | HOLDING: {:.2}% profit", token.symbol, profit_pct);
    (false, format!("holding({:.2}%)", profit_pct))
}

/// Check if we can enter a position for this token (cooldown management)
pub fn can_enter_token_position(_token_mint: &str) -> (bool, i64) {
    // Simplified - always allow for now
    // In production, implement persistent cooldown tracking
    (true, ENTRY_COOLDOWN_MINUTES + 1)
}

// ═══════════════════════════════════════════════════════════════════════════════
// POSITION MANAGEMENT
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub enum PositionAction {
    Hold,
    DCA {
        sol_amount: f64,
    },
    Sell {
        reason: String,
    },
}

pub fn evaluate_position(token: &Token, pos: &Position, current_price: f64) -> PositionAction {
    let profit_pct = if pos.sol_spent > 0.0 {
        let current_value = current_price * pos.token_amount;
        ((current_value - pos.sol_spent) / pos.sol_spent) * 100.0
    } else {
        0.0
    };

    println!(
        "🎯 [POSITION] {} | Price: ${:.8} | Profit: {:.2}% | DCA: {}/{}",
        token.symbol,
        current_price,
        profit_pct,
        pos.dca_count,
        MAX_DCA_COUNT
    );

    // Get trades data for this token
    let trades_data = futures::executor::block_on(async {
        crate::trades::get_token_trades(&token.mint).await
    });

    // Get OHLCV dataframe for this token
    let ohlcv_dataframe = futures::executor::block_on(async {
        crate::ohlcv::get_token_ohlcv_dataframe(&token.mint).await
    });

    // 1. Check DCA
    if should_dca(token, pos, current_price, trades_data.as_ref(), ohlcv_dataframe.as_ref()) {
        return PositionAction::DCA { sol_amount: TRADE_SIZE_SOL };
    }

    // 2. Check sell
    let (should_sell_signal, sell_reason) = should_sell(
        token,
        pos,
        current_price,
        trades_data.as_ref(),
        ohlcv_dataframe.as_ref()
    );
    if should_sell_signal {
        return PositionAction::Sell { reason: sell_reason };
    }

    // 3. Hold
    PositionAction::Hold
}

pub fn should_update_peak(pos: &Position, current_price: f64) -> bool {
    current_price > pos.peak_price
}

pub fn get_profit_bucket(pos: &Position, current_price: f64) -> i32 {
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };
    (profit_pct / 2.0).floor() as i32 // Every 2%
}

/// Calculate DCA size (simplified)
pub fn calculate_dca_size(_token: &Token, _pos: &Position) -> f64 {
    TRADE_SIZE_SOL // Same size as initial entry
}

// ═══════════════════════════════════════════════════════════════════════════════
// PERFORMANCE TRACKING (STUB - TO BE IMPLEMENTED)
// ═══════════════════════════════════════════════════════════════════════════════

pub fn calculate_performance_multiplier() -> f64 {
    // TODO: Implement based on recent win/loss history
    1.0
}

pub fn calculate_adaptive_position_size(base_size: f64, _token: &Token) -> f64 {
    // TODO: Implement adaptive sizing based on token quality and recent performance
    base_size
}
