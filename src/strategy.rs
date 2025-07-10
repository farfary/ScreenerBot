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
pub async fn should_buy(token: &Token, can_buy: bool, current_price: f64) -> bool {
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
    let price_change_1h = token.price_change.h1;
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

    // Strong whale activity
    if whale_score >= 0.6 {
        entry_score += 0.4;
        reasons.push(format!("whale_activity({:.1})", whale_score));
    }

    // Low bot interference
    if bot_score <= 0.4 {
        entry_score += 0.3;
        reasons.push(format!("low_bots({:.1})", bot_score));
    }

    // Good buying pressure
    if buy_ratio >= 0.6 {
        entry_score += 0.2;
        reasons.push(format!("buying_pressure({:.2})", buy_ratio));
    }

    // Controlled price movement (not FOMO)
    if price_change_5m >= -10.0 && price_change_5m <= ACCUMULATION_PATIENCE_THRESHOLD {
        entry_score += 0.2;
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
pub fn should_dca(token: &Token, pos: &Position, current_price: f64) -> bool {
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

    // 6. Whale activity check (simplified)
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buy_ratio = if buys_1h + sells_1h > 0 {
        (buys_1h as f64) / ((buys_1h + sells_1h) as f64)
    } else {
        0.0
    };

    if buy_ratio < 0.4 {
        println!("📉 [DCA] {} | Poor buying pressure: {:.2}", token.symbol, buy_ratio);
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
pub fn should_sell(token: &Token, pos: &Position, current_price: f64) -> (bool, String) {
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

    // 3. AGGRESSIVE PROFIT TAKING

    // Quick profits (0.5-3%) - Take profit on any weakness
    if profit_pct >= 0.5 && profit_pct < 3.0 {
        if token.price_change.m5 < -2.0 || drop_from_peak > 5.0 {
            println!("💸 [SELL] {} | QUICK PROFIT: {:.2}% + weakness", token.symbol, profit_pct);
            return (true, format!("quick_profit({:.2}%)", profit_pct));
        }
    }

    // Small profits (3-10%) - Take profit on negative momentum
    if profit_pct >= 3.0 && profit_pct < 10.0 {
        if token.price_change.m5 < -3.0 || drop_from_peak > 10.0 {
            println!("💸 [SELL] {} | SMALL PROFIT: {:.2}% + momentum", token.symbol, profit_pct);
            return (true, format!("small_profit({:.2}%)", profit_pct));
        }
    }

    // Medium profits (10-25%) - Use trailing stops
    if profit_pct >= 10.0 && profit_pct < 25.0 {
        if drop_from_peak > 15.0 || token.price_change.m5 < -5.0 {
            println!("💸 [SELL] {} | MEDIUM PROFIT: {:.2}% + trailing", token.symbol, profit_pct);
            return (true, format!("medium_profit({:.2}%)", profit_pct));
        }
    }

    // Large profits (25%+) - Let them run with wider stops
    if profit_pct >= 25.0 {
        if drop_from_peak > 25.0 || token.price_change.m5 < -8.0 {
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

    // 1. Check DCA
    if should_dca(token, pos, current_price) {
        return PositionAction::DCA { sol_amount: TRADE_SIZE_SOL };
    }

    // 2. Check sell
    let (should_sell_signal, sell_reason) = should_sell(token, pos, current_price);
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
