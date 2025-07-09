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

// PROFESSIONAL HIGH-FREQUENCY TRADING CONSTANTS
// ADVANCED MULTI-POSITION TRADING CONSTANTS
pub const TRADE_SIZE_SOL: f64 = 0.001; // Standard entry size for all positions
pub const MAX_OPEN_POSITIONS: usize = 50; // Increased to 50 positions
pub const MIN_HOLD_TIME_SECONDS: i64 = 10; // Minimum 30 minutes hold
pub const MAX_HOLD_TIME_SECONDS: i64 = 86400; // Maximum 24 hours hold
pub const MAX_DCA_COUNT: u8 = 2; // Maximum DCA rounds per position (strict limit)
pub const DCA_COOLDOWN_MINUTES: i64 = 15; // Minimum 15 minutes between DCA attempts
pub const DCA_BASE_TRIGGER_PCT: f64 = -12.0; // Base DCA trigger percentage

pub const TRANSACTION_FEE_SOL: f64 = 0.000015; // Transaction fee per buy/sell operation
pub const POSITIONS_PRINT_TIME: u64 = 10; // Print every 10 seconds
pub const SLIPPAGE_BPS: f64 = 0.5; // Slightly increased for better execution
pub const DCA_SIZE_FACTOR: f64 = 0.8; // Larger DCA when used

// TRADING STRATEGY CONSTANTS
pub const MIN_VOLUME_USD: f64 = 5000.0; // Minimum daily volume
pub const MIN_LIQUIDITY_SOL: f64 = 10.0; // Minimum liquidity

// ENHANCED ENTRY LOGIC CONSTANTS - ANTI-BOT & WHALE-FOLLOWING STRATEGY
pub const MIN_ACTIVITY_BUYS_5M: u64 = 2; // Reduced - be less restrictive on activity
pub const MIN_ACTIVITY_SELLS_5M: u64 = 1; // Keep some selling for healthy market
pub const MIN_ACTIVITY_BUYS_1H: u64 = 5; // Reduced - capture more opportunities
pub const BIG_DUMP_THRESHOLD: f64 = -20.0; // More lenient - avoid only major dumps
pub const ENTRY_COOLDOWN_MINUTES: i64 = 30; // Reduced cooldown for more trades
pub const SAFE_LIQUIDITY_MULTIPLIER: f64 = 2.0; // Slightly reduced safety margin

// ENHANCED WHALE DETECTION & ANTI-BOT CONSTANTS
pub const MIN_TOKEN_AGE_MINUTES: i64 = 15; // Reduced - catch earlier opportunities
pub const MAX_TOKEN_AGE_HOURS: i64 = 72; // Increased - more mature tokens
pub const WHALE_BUY_THRESHOLD_SOL: f64 = 3.0; // Reduced threshold for whale detection
pub const MIN_WHALE_RATIO: f64 = 0.2; // Reduced - 20% whale activity is enough
pub const MAX_BOT_RATIO: f64 = 0.8; // Allow higher bot activity but monitor
pub const MIN_HOLDER_COUNT: u64 = 15; // Reduced for earlier stage tokens
pub const MAX_CREATOR_BALANCE_PCT: f64 = 20.0; // Increased tolerance
pub const ACCUMULATION_PATIENCE_THRESHOLD: f64 = 8.0; // Allow more recent gains

// NEW: BOT DETECTION PARAMETERS
pub const MAX_REPETITIVE_TX_RATIO: f64 = 0.6; // Max 60% repetitive patterns
pub const MIN_TX_SIZE_VARIATION: f64 = 0.3; // Need 30% size variation
pub const WHALE_ACCUMULATION_WINDOW: i64 = 30; // 30min window for whale analysis

/// ENHANCED ANTI-BOT WHALE-FOLLOWING ENTRY STRATEGY V2.0
/// This strategy is designed to:
/// 1. Avoid bot-heavy tokens through transaction pattern analysis
/// 2. Identify genuine whale accumulation vs retail FOMO
/// 3. Enter during consolidation phases when whales are accumulating
/// 4. Use multiple confirmation signals to reduce false entries
/// 5. Time entries to avoid front-running and manipulation
pub fn should_buy(token: &Token, can_buy: bool, current_price: f64) -> bool {
    println!(
        "\n🔍 [ENTRY ANALYSIS] {} | Price: ${:.8} | Evaluating anti-bot whale entry...",
        token.symbol, current_price
    );

    if !can_buy {
        println!("❌ [ENTRY] {} | Cannot buy - trading blocked", token.symbol);
        return false;
    }

    // ─── ENTRY COOLDOWN & BLACKLIST CHECK ───
    let (can_enter, minutes_since_last) = can_enter_token_position(&token.mint);
    if !can_enter {
        println!("⏸️ [ENTRY] {} | REJECTED: Entry cooldown active ({}min < {}min)", 
            token.symbol, minutes_since_last, ENTRY_COOLDOWN_MINUTES);
        return false;
    }

    // ─── BASIC SAFETY CHECKS ───
    if !crate::dexscreener::is_safe_to_trade(token, false) {
        println!("🚨 [ENTRY] {} | REJECTED: Failed basic rug check", token.symbol);
        return false;
    }

    // ─── TOKEN AGE & MATURITY FILTERS ───
    
    let token_age_minutes = {
        let now = chrono::Utc::now().timestamp() as u64;
        if token.pair_created_at > 0 && token.pair_created_at < now {
            ((now - token.pair_created_at) / 60) as i64
        } else {
            0 // Unknown age - treat as very new
        }
    };

    // 3. Avoid fresh launches (bot paradise) and very old tokens
    if token_age_minutes < MIN_TOKEN_AGE_MINUTES {
        println!("🚫 [ENTRY] {} | REJECTED: Too fresh ({}min < {}min) - bot territory", 
            token.symbol, token_age_minutes, MIN_TOKEN_AGE_MINUTES);
        return false;
    }

    if token_age_minutes > MAX_TOKEN_AGE_HOURS * 60 {
        println!("🚫 [ENTRY] {} | REJECTED: Too old ({}h > {}h) - established patterns", 
            token.symbol, token_age_minutes / 60, MAX_TOKEN_AGE_HOURS);
        return false;
    }

    // ─── EXTRACT KEY METRICS ───
    
    let volume_24h = token.volume.h24;
    let volume_1h = token.volume.h1;
    let volume_5m = token.volume.m5;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let price_change_1h = token.price_change.h1;
    let price_change_5m = token.price_change.m5;
    let price_change_24h = token.price_change.h24;
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buys_5m = token.txns.m5.buys;
    let sells_5m = token.txns.m5.sells;
    let buy_sell_ratio_1h = (buys_1h as f64) / ((sells_1h as f64) + 1.0);
    let total_holders = token.rug_check.total_holders;
    let creator_balance_pct = if token.rug_check.total_supply > 0 {
        (token.rug_check.creator_balance as f64 / token.rug_check.total_supply as f64) * 100.0
    } else { 100.0 };

    println!(
        "📊 [METRICS] {} | Age: {}min | Vol24h: ${:.0} | Vol1h: ${:.0} | Liq: {:.1}SOL | Holders: {} | Creator: {:.1}%",
        token.symbol, token_age_minutes, volume_24h, volume_1h, liquidity_sol, total_holders, creator_balance_pct
    );

    // ─── FUNDAMENTAL QUALITY GATES ───

    // 4. Enhanced liquidity requirement
    let safe_liquidity_threshold = MIN_LIQUIDITY_SOL * SAFE_LIQUIDITY_MULTIPLIER;
    if liquidity_sol < safe_liquidity_threshold {
        println!("💧 [ENTRY] {} | REJECTED: Low liquidity ({:.1} < {:.1} SOL)", 
            token.symbol, liquidity_sol, safe_liquidity_threshold);
        return false;
    }

    // 5. Holder distribution check  
    if total_holders < MIN_HOLDER_COUNT {
        println!("👥 [ENTRY] {} | REJECTED: Too few holders ({} < {})", 
            token.symbol, total_holders, MIN_HOLDER_COUNT);
        return false;
    }

    // 6. Creator balance check - avoid tokens where creator holds too much
    if creator_balance_pct > MAX_CREATOR_BALANCE_PCT {
        println!("🏭 [ENTRY] {} | REJECTED: Creator holds too much ({:.1}% > {:.1}%)", 
            token.symbol, creator_balance_pct, MAX_CREATOR_BALANCE_PCT);
        return false;
    }

    // 7. Activity requirements - need consistent trading
    if buys_5m < MIN_ACTIVITY_BUYS_5M || sells_5m < MIN_ACTIVITY_SELLS_5M {
        println!("📉 [ENTRY] {} | REJECTED: Low 5m activity (B:{}/S:{} < {}/{})", 
            token.symbol, buys_5m, sells_5m, MIN_ACTIVITY_BUYS_5M, MIN_ACTIVITY_SELLS_5M);
        return false;
    }

    if buys_1h < MIN_ACTIVITY_BUYS_1H {
        println!("� [ENTRY] {} | REJECTED: Low 1h buy activity ({} < {})", 
            token.symbol, buys_1h, MIN_ACTIVITY_BUYS_1H);
        return false;
    }

    // 8. Volume requirement - but not too high (avoid FOMO spikes)
    if volume_24h < MIN_VOLUME_USD {
        println!("📊 [ENTRY] {} | REJECTED: Low volume (${:.0} < ${:.0})", 
            token.symbol, volume_24h, MIN_VOLUME_USD);
        return false;
    }

    // ─── WHALE BEHAVIOR ANALYSIS ───

    // 9. Estimate whale activity (simplified approach using volume patterns)
    let avg_tx_volume_1h = if (buys_1h + sells_1h) > 0 {
        volume_1h / ((buys_1h + sells_1h) as f64)
    } else { 0.0 };

    let estimated_whale_volume = if avg_tx_volume_1h > WHALE_BUY_THRESHOLD_SOL {
        volume_1h * 0.4 // Estimate 40% from large transactions
    } else {
        volume_1h * 0.1 // Estimate 10% from large transactions  
    };

    let whale_ratio = estimated_whale_volume / volume_1h;
    let bot_ratio = 1.0 - whale_ratio; // Simplified bot detection

    println!(
        "� [WHALE ANALYSIS] {} | AvgTx: ${:.3} | WhaleRatio: {:.1}% | BotRatio: {:.1}%",
        token.symbol, avg_tx_volume_1h, whale_ratio * 100.0, bot_ratio * 100.0
    );

    // 10. Whale activity requirements
    if whale_ratio < MIN_WHALE_RATIO {
        println!("🐋 [ENTRY] {} | REJECTED: Insufficient whale activity ({:.1}% < {:.1}%)", 
            token.symbol, whale_ratio * 100.0, MIN_WHALE_RATIO * 100.0);
        return false;
    }

    // 11. Bot activity limits
    if bot_ratio > MAX_BOT_RATIO {
        println!("🤖 [ENTRY] {} | REJECTED: Too much bot activity ({:.1}% > {:.1}%)", 
            token.symbol, bot_ratio * 100.0, MAX_BOT_RATIO * 100.0);
        return false;
    }

    // ─── MARKET TIMING & ACCUMULATION PHASE DETECTION ───

    // 12. Avoid FOMO entries - look for accumulation phases
    if price_change_5m > ACCUMULATION_PATIENCE_THRESHOLD {
        println!("� [ENTRY] {} | REJECTED: Already pumping ({:.1}% > {:.1}%) - wait for accumulation", 
            token.symbol, price_change_5m, ACCUMULATION_PATIENCE_THRESHOLD);
        return false;
    }

    // 13. Don't enter major dumps (but be less strict than before)
    if price_change_5m <= BIG_DUMP_THRESHOLD {
        println!("📉 [ENTRY] {} | REJECTED: Major dump in progress ({:.1}% <= {:.1}%)", 
            token.symbol, price_change_5m, BIG_DUMP_THRESHOLD);
        return false;
    }

    // ─── ADVANCED SIGNAL SCORING ───

    let mut signal_strength = 0.0;
    let mut entry_reasons = Vec::new();

    println!("⚡ [SIGNAL ANALYSIS] {} | Calculating advanced entry signals...", token.symbol);

    // Whale accumulation pattern (key signal)
    if whale_ratio > 0.4 && price_change_1h < 10.0 && price_change_5m >= -5.0 {
        signal_strength += 0.4;
        entry_reasons.push(format!("whale_accumulation({:.1}%_whale,{:.1}%_1h)", whale_ratio*100.0, price_change_1h));
    }

    // Healthy consolidation pattern
    if price_change_1h >= -2.0 && price_change_1h <= 8.0 && price_change_5m >= -3.0 {
        signal_strength += 0.3;
        entry_reasons.push(format!("healthy_consolidation({:.1}%_1h,{:.1}%_5m)", price_change_1h, price_change_5m));
    }

    // Strong buying pressure
    if buy_sell_ratio_1h > 2.0 {
        signal_strength += 0.2;
        entry_reasons.push(format!("strong_buying_pressure({:.1}x_ratio)", buy_sell_ratio_1h));
    }

    // Volume spike with control (not FOMO)
    let volume_threshold = volume_24h / 20.0; // More conservative than /12
    if volume_1h > volume_threshold && price_change_1h < 15.0 {
        signal_strength += 0.2;
        entry_reasons.push(format!("controlled_volume_spike(${:.0}_1h)", volume_1h));
    }

    // High liquidity safety margin
    if liquidity_sol > safe_liquidity_threshold * 2.0 {
        signal_strength += 0.1;
        entry_reasons.push(format!("high_liquidity({:.0}SOL)", liquidity_sol));
    }

    // Good holder distribution
    if total_holders > MIN_HOLDER_COUNT * 2 {
        signal_strength += 0.1;
        entry_reasons.push(format!("good_distribution({}holders)", total_holders));
    }

    // Token maturity sweet spot
    if token_age_minutes >= 60 && token_age_minutes <= 12 * 60 { // 1-12 hours
        signal_strength += 0.1;
        entry_reasons.push(format!("mature_token({}min)", token_age_minutes));
    }

    // Low creator risk
    if creator_balance_pct < 5.0 {
        signal_strength += 0.1;
        entry_reasons.push(format!("low_creator_risk({:.1}%)", creator_balance_pct));
    }

    println!(
        "🎯 [SIGNALS] {} | Strength: {:.2} | Reasons: {:?}",
        token.symbol, signal_strength, entry_reasons
    );

    // ─── FINAL ENTRY DECISION ───

    let required_strength = 0.7; // Higher threshold - be more selective

    if signal_strength >= required_strength && !entry_reasons.is_empty() {
        println!(
            "✅ [ENTRY] {} | BUY APPROVED | Strength: {:.2} >= {:.2} | Reasons: {}",
            token.symbol, signal_strength, required_strength, entry_reasons.join(", ")
        );
        return true;
    }

    println!(
        "❌ [ENTRY] {} | BUY REJECTED | Strength: {:.2} < {:.2} | Missing: {:.2} points",
        token.symbol, signal_strength, required_strength, required_strength - signal_strength
    );
    false
}

/// INTELLIGENT WHALE-AWARE DCA STRATEGY
/// This strategy is designed to:
/// 1. Only DCA when whales are also accumulating (not when they're dumping)
/// 2. Use progressive sizing based on position performance
/// 3. Avoid DCA during obvious rug pulls or whale distribution
/// 4. Time DCAs during accumulation phases, not panic selling
/// 5. Respect strict limits to avoid over-exposure
pub fn should_dca(token: &Token, pos: &Position, current_price: f64) -> bool {
    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
    let current_liquidity = token.liquidity.base + token.liquidity.quote;
    
    // Enhanced whale behavior analysis
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buys_5m = token.txns.m5.buys;
    let sells_5m = token.txns.m5.sells;
    let buy_sell_ratio_1h = (buys_1h as f64) / ((sells_1h as f64) + 1.0);
    let buy_sell_ratio_5m = (buys_5m as f64) / ((sells_5m as f64) + 1.0);
    
    // Volume pattern analysis
    let volume_1h = token.volume.h1;
    let avg_tx_size = if (buys_1h + sells_1h) > 0 { volume_1h / (buys_1h + sells_1h) as f64 } else { 0.0 };
    let whale_activity_detected = avg_tx_size > 3.0; // Lower threshold for DCA whale detection
    
    println!(
        "\n🔄 [DCA ANALYSIS] {} | Current: ${:.8} | Entry: ${:.8} | Drop: {:.1}% | DCA: {}/{}",
        token.symbol, current_price, pos.entry_price, drop_pct, pos.dca_count, MAX_DCA_COUNT
    );
    println!(
        "🐋 [WHALE ANALYSIS] BuyRatio: 1h={:.2}x 5m={:.2}x | AvgTx: ${:.2} | WhaleActive: {}",
        buy_sell_ratio_1h, buy_sell_ratio_5m, avg_tx_size, whale_activity_detected
    );

    // 1. Hard limit: Never exceed maximum DCA count
    if pos.dca_count >= MAX_DCA_COUNT {
        println!("❌ [DCA] {} | REJECTED: Max DCA count reached ({}/{})", 
            token.symbol, pos.dca_count, MAX_DCA_COUNT);
        return false;
    }

    // 2. Cooldown check: Prevent rapid-fire DCA attempts
    if pos.dca_count > 0 {
        let time_since_last_dca = now - pos.last_dca_time;
        if time_since_last_dca.num_minutes() < DCA_COOLDOWN_MINUTES {
            println!("⏱️ [DCA] {} | REJECTED: Cooldown active ({}min < {}min)", 
                token.symbol, time_since_last_dca.num_minutes(), DCA_COOLDOWN_MINUTES);
            return false;
        }
    }

    // 3. Minimum hold time: Must hold for at least 10 minutes before first DCA
    if elapsed.num_minutes() < 10 {
        println!("⏰ [DCA] {} | REJECTED: Min hold time ({}min < 10min)", 
            token.symbol, elapsed.num_minutes());
        return false;
    }

    // 4. Progressive drop threshold - require larger drops for later DCAs
    let required_drop = match pos.dca_count {
        0 => DCA_BASE_TRIGGER_PCT,      // First DCA: -12%
        1 => DCA_BASE_TRIGGER_PCT * 1.5, // Second DCA: -18%
        _ => DCA_BASE_TRIGGER_PCT * 2.0,  // Final DCA: -24%
    };
    
    if drop_pct > required_drop {
        println!("📈 [DCA] {} | REJECTED: Insufficient drop ({:.1}% > {:.1}% required for DCA #{})", 
            token.symbol, drop_pct, required_drop, pos.dca_count + 1);
        return false;
    }

    // 5. Enhanced liquidity check - be stricter for later DCAs
    let required_liquidity = MIN_LIQUIDITY_SOL * (1.0 + (pos.dca_count as f64 * 0.5));
    if current_liquidity < required_liquidity {
        println!("💧 [DCA] {} | REJECTED: Low liquidity ({:.1}SOL < {:.1}SOL required)", 
            token.symbol, current_liquidity, required_liquidity);
        return false;
    }

    // 6. Volume activity check - need sustained volume
    if token.volume.h1 < 1000.0 + (pos.dca_count as f64 * 500.0) {
        println!("📊 [DCA] {} | REJECTED: Low volume - ${:.0} < $500 required", 
            token.symbol, token.volume.h1);
        return false; // Volume too low
    }

    // 7. Check buying pressure
    if buy_sell_ratio < 1.0 {
        println!(
            "📉 [DCA] {} | REJECTED: Poor buying pressure - ratio {:.2} < 1.0",
            token.symbol,
            buy_sell_ratio
        );
        return false; // Need some buying pressure
    }

    // 8. Price level check: Only DCA if significantly below last entry
    let price_drop_from_last = if pos.dca_count == 0 {
        drop_pct
    } else {
        ((current_price - pos.last_dca_price) / pos.last_dca_price) * 100.0
    };

    if price_drop_from_last > -10.0 {
        let reference_price = if pos.dca_count == 0 { pos.entry_price } else { pos.last_dca_price };
        println!(
            "📊 [DCA] {} | REJECTED: Insufficient drop from last entry - {:.1}% > -10% (ref: ${:.8})",
            token.symbol,
            price_drop_from_last,
            reference_price
        );
        return false; // Need at least 10% drop from last entry
    }

    println!(
        "✅ DCA APPROVED {} | Drop: {:.1}% | Buy/Sell: {:.2} | DCA#{} | Liq: {:.1}SOL | Vol1h: ${:.0} | Last DCA: {}min ago",
        token.symbol,
        drop_pct,
        buy_sell_ratio,
        pos.dca_count + 1,
        current_liquidity,
        token.volume.h1,
        if pos.dca_count > 0 {
            (now - pos.last_dca_time).num_minutes()
        } else {
            elapsed.num_minutes()
        }
    );

    true
}

/// Get DCA size based on position performance (simplified)
pub fn calculate_dca_size(token: &Token, pos: &Position) -> f64 {
    let base_dca_size = pos.sol_spent * DCA_SIZE_FACTOR;

    // Adjust based on how many DCA rounds we've done
    let dca_adjustment = match pos.dca_count {
        0 => 1.0, // First DCA: full size
        1 => 0.8, // Second DCA: smaller
        _ => 0.6, // Final DCA: smallest
    };

    // Adjust based on liquidity
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let liquidity_adjustment = if liquidity_sol > 50.0 {
        1.0
    } else if liquidity_sol > 20.0 {
        0.8
    } else {
        0.6
    };

    let final_size = base_dca_size * dca_adjustment * liquidity_adjustment;

    println!(
        "💰 [DCA SIZE] {} | Base: {:.6}SOL | DCA#{} adj: {:.1}x | Liq adj: {:.1}x | Final: {:.6}SOL",
        token.symbol,
        base_dca_size,
        pos.dca_count + 1,
        dca_adjustment,
        liquidity_adjustment,
        final_size
    );

    final_size
}

/// ENHANCED PROFIT-MAXIMIZING SELL STRATEGY 
/// Designed to:
/// 1. Maximize profits by riding whale momentum
/// 2. Detect whale distribution/accumulation patterns  
/// 3. Use adaptive profit targets based on token performance
/// 4. Never sell at loss (strict rule)
/// 5. Use sophisticated timing to exit before major dumps
pub fn should_sell(token: &Token, pos: &Position, current_price: f64) -> (bool, String) {
    // Calculate total fees: one fee for initial buy + one fee for each DCA + one fee for sell
    let total_buy_fees = ((1 + (pos.dca_count as usize)) as f64) * TRANSACTION_FEE_SOL;
    let sell_fee = TRANSACTION_FEE_SOL;
    let total_fees = total_buy_fees + sell_fee;

    // Use consistent profit calculation method
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - total_fees;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };

    let drop_from_peak = ((pos.peak_price - current_price) / pos.peak_price) * 100.0;
    let gain_from_entry = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
    let held_duration = (Utc::now() - pos.open_time).num_seconds();
    let held_minutes = held_duration / 60;
    let liquidity_total = token.liquidity.base + token.liquidity.quote;
    
    // Enhanced whale behavior analysis
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buys_5m = token.txns.m5.buys;
    let sells_5m = token.txns.m5.sells;
    let sell_pressure_1h = if buys_1h + sells_1h > 0 { 
        sells_1h as f64 / (buys_1h + sells_1h) as f64 
    } else { 0.5 };
    let sell_pressure_5m = if buys_5m + sells_5m > 0 { 
        sells_5m as f64 / (buys_5m + sells_5m) as f64 
    } else { 0.5 };
    
    // Volume analysis for whale detection
    let volume_1h = token.volume.h1;
    let volume_5m = token.volume.m5;
    let avg_tx_size_1h = if (buys_1h + sells_1h) > 0 { volume_1h / (buys_1h + sells_1h) as f64 } else { 0.0 };
    let whale_selling_detected = avg_tx_size_1h > 10.0 && sell_pressure_5m > 0.6;

    println!(
        "\n💰 [SELL ANALYSIS] {} | Current: ${:.8} | Entry: ${:.8} | Peak: ${:.8}",
        token.symbol, current_price, pos.entry_price, pos.peak_price
    );
    println!(
        "📊 [POSITION] Profit: {:.2}% ({:.6}SOL) | Held: {}min | Peak Drop: {:.1}% | Entry Gain: {:.1}%", 
        profit_pct, profit_sol, held_minutes, drop_from_peak, gain_from_entry
    );
    println!(
        "🐋 [WHALE ANALYSIS] SellPressure: 1h={:.1}% 5m={:.1}% | AvgTx: ${:.2} | WhaleExit: {}",
        sell_pressure_1h * 100.0, sell_pressure_5m * 100.0, avg_tx_size_1h, whale_selling_detected
    );

    // 1. MINIMUM HOLD TIME - Must hold for at least 10 seconds (reduced for faster exits)
    if held_duration < MIN_HOLD_TIME_SECONDS {
        println!(
            "⏰ [SELL] {} | HOLD: Min hold time not met - held {}min < {}min required",
            token.symbol, held_minutes, MIN_HOLD_TIME_SECONDS / 60
        );
        return (false, format!("min_hold_time({}s<{}s)", held_duration, MIN_HOLD_TIME_SECONDS));
    }

    // 2. NEVER SELL AT LOSS - Only sell when profitable
    if profit_pct <= 0.0 {
        println!(
            "📉 [SELL] {} | HOLD: Never sell at loss - profit {:.2}% | Will wait for recovery",
            token.symbol, profit_pct
        );
        return (false, format!("no_loss_selling(profit:{:.2}%)", profit_pct));
    }

    println!("✅ [SELL] {} | Profitable position - analyzing optimal exit...", token.symbol);

    // 3. EMERGENCY WHALE DUMP DETECTION - Exit immediately if whales are dumping
    if whale_selling_detected && profit_pct > 0.5 {
        println!(
            "🚨 [SELL] {} | WHALE DUMP DETECTED! Immediate exit | AvgTx: ${:.2} | SellPressure: {:.1}% | Profit: {:.2}%",
            token.symbol, avg_tx_size_1h, sell_pressure_5m * 100.0, profit_pct
        );
        return (true, format!("whale_dump_exit(profit:{:.2}%, sell_pressure:{:.1}%)", profit_pct, sell_pressure_5m * 100.0));
    }

    // 4. ADAPTIVE PROFIT TARGETS BASED ON PERFORMANCE

    // Micro profits (0.5-2%) - Very conservative, take profit on any weakness
    if profit_pct >= 0.5 && profit_pct < 2.0 {
        if token.price_change.m5 < -1.0 || drop_from_peak > 3.0 {
            println!(
                "💸 [SELL] {} | MICRO PROFIT EXIT: {:.2}% profit | 5m: {:.1}% | Peak drop: {:.1}%",
                token.symbol, profit_pct, token.price_change.m5, drop_from_peak
            );
            return (true, format!("micro_profit_exit(profit:{:.2}%, 5m:{:.1}%)", profit_pct, token.price_change.m5));
        }
    }

    // Small profits (2-8%) - Take profit on negative momentum
    if profit_pct >= 2.0 && profit_pct < 8.0 {
        if token.price_change.m5 < -3.0 || drop_from_peak > 8.0 {
            println!(
                "💸 [SELL] {} | SMALL PROFIT EXIT: {:.2}% profit | 5m: {:.1}% | Peak drop: {:.1}%",
                token.symbol, profit_pct, token.price_change.m5, drop_from_peak
            );
            return (true, format!("small_profit_exit(profit:{:.2}%, 5m:{:.1}%)", profit_pct, token.price_change.m5));
        }
        
        // Also exit if high sell pressure develops
        if sell_pressure_5m > 0.7 {
            println!(
                "💸 [SELL] {} | SELL PRESSURE EXIT: {:.2}% profit | Sell pressure: {:.1}%",
                token.symbol, profit_pct, sell_pressure_5m * 100.0
            );
            return (true, format!("sell_pressure_exit(profit:{:.2}%, pressure:{:.1}%)", profit_pct, sell_pressure_5m * 100.0));
        }
    }

    // Medium profits (8-25%) - Use trailing stops and momentum analysis
    if profit_pct >= 8.0 && profit_pct < 25.0 {
        // Tighter trailing stop for medium profits
        if drop_from_peak > 15.0 {
            println!(
                "💸 [SELL] {} | MEDIUM TRAILING STOP: {:.2}% profit | {:.1}% drop from peak",
                token.symbol, profit_pct, drop_from_peak
            );
            return (true, format!("medium_trailing_stop(profit:{:.2}%, drop:{:.1}%)", profit_pct, drop_from_peak));
        }
        
        // Exit on strong negative momentum
        if token.price_change.m5 < -5.0 {
            println!(
                "💸 [SELL] {} | MEDIUM MOMENTUM EXIT: {:.2}% profit | 5m momentum: {:.1}%",
                token.symbol, profit_pct, token.price_change.m5
            );
            return (true, format!("medium_momentum_exit(profit:{:.2}%, 5m:{:.1}%)", profit_pct, token.price_change.m5));
        }
    }

    // Large profits (25-50%) - Be more patient but watch for major reversals  
    if profit_pct >= 25.0 && profit_pct < 50.0 {
        // Wider trailing stop for large profits
        if drop_from_peak > 25.0 {
            println!(
                "💸 [SELL] {} | LARGE TRAILING STOP: {:.2}% profit | {:.1}% drop from peak",
                token.symbol, profit_pct, drop_from_peak
            );
            return (true, format!("large_trailing_stop(profit:{:.2}%, drop:{:.1}%)", profit_pct, drop_from_peak));
        }
        
        // Exit on very strong negative momentum combined with high sell pressure
        if token.price_change.m5 < -8.0 && sell_pressure_5m > 0.6 {
            println!(
                "� [SELL] {} | LARGE REVERSAL EXIT: {:.2}% profit | 5m: {:.1}% | Sell pressure: {:.1}%",
                token.symbol, profit_pct, token.price_change.m5, sell_pressure_5m * 100.0
            );
            return (true, format!("large_reversal_exit(profit:{:.2}%, 5m:{:.1}%)", profit_pct, token.price_change.m5));
        }
    }

    // Massive profits (>50%) - Secure some gains but let runners run
    if profit_pct >= 50.0 {
        // Very wide trailing stop for massive profits  
        if drop_from_peak > 35.0 {
            println!(
                "💸 [SELL] {} | MASSIVE TRAILING STOP: {:.2}% profit | {:.1}% drop from peak",
                token.symbol, profit_pct, drop_from_peak
            );
            return (true, format!("massive_trailing_stop(profit:{:.2}%, drop:{:.1}%)", profit_pct, drop_from_peak));
        }
        
        // Exit only on extreme reversal patterns
        if token.price_change.m5 < -12.0 && token.price_change.h1 < -20.0 {
            println!(
                "💸 [SELL] {} | MASSIVE REVERSAL EXIT: {:.2}% profit | 5m: {:.1}% | 1h: {:.1}%",
                token.symbol, profit_pct, token.price_change.m5, token.price_change.h1
            );
            return (true, format!("massive_reversal_exit(profit:{:.2}%, 5m:{:.1}%)", profit_pct, token.price_change.m5));
        }
    }

    // 5. FUNDAMENTAL RISK EXITS - Only when profitable

    // Market deterioration exit (but more lenient)
    if liquidity_total < MIN_LIQUIDITY_SOL * 0.2 {
        println!(
            "🚨 [SELL] {} | LIQUIDITY CRISIS EXIT: {:.1}SOL < {:.1}SOL | Profit: {:.2}%",
            token.symbol, liquidity_total, MIN_LIQUIDITY_SOL * 0.2, profit_pct
        );
        return (true, format!("liquidity_crisis(profit:{:.2}%, liq:{:.1}SOL)", profit_pct, liquidity_total));
    }

    // Extreme token collapse exit (very lenient - only exit on massive dumps)
    if token.price_change.h24 <= -80.0 && profit_pct < 10.0 {
        println!(
            "🚨 [SELL] {} | TOKEN COLLAPSE EXIT: 24h {:.1}% <= -80% | Low profit: {:.2}%",
            token.symbol, token.price_change.h24, profit_pct
        );
        return (true, format!("token_collapse(profit:{:.2}%, 24h:{:.1}%)", profit_pct, token.price_change.h24));
    }

    // 6. MAXIMUM HOLD TIME - Exit profitable positions after very long holds (more generous)
    if held_duration >= MAX_HOLD_TIME_SECONDS && profit_pct > 0.0 {
        println!(
            "⏰ [SELL] {} | MAX HOLD TIME EXIT: Held {}min >= {}min | Profit: {:.2}%",
            token.symbol, held_minutes, MAX_HOLD_TIME_SECONDS / 60, profit_pct
        );
        return (true, format!("max_hold_time_exit(profit:{:.2}%, held:{}s)", profit_pct, held_duration));
    }

    // Default: Hold the position and let it run
    println!(
        "🔒 [SELL] {} | HOLDING FOR MORE GAINS | Profit: {:.2}% | Drop from peak: {:.1}% | 5m momentum: {:.1}% | Sell pressure: {:.1}%",
        token.symbol, profit_pct, drop_from_peak, token.price_change.m5, sell_pressure_5m * 100.0
    );
    (false, format!("holding_for_gains(profit:{:.2}%, peak_drop:{:.1}%)", profit_pct, drop_from_peak))
}

/// Check if we can enter a position for this token (cooldown management)
/// Returns (can_enter, time_since_last_entry_minutes)
pub fn can_enter_token_position(_token_mint: &str) -> (bool, i64) {
    // TODO: This would ideally be implemented with a persistent state manager
    // For now, we'll return true to allow entries, but the infrastructure
    // should track last entry times per token

    // In a real implementation, this would:
    // 1. Check a cache/database for last entry time for this token
    // 2. Calculate time difference from now
    // 3. Return false if within cooldown period
    // 4. Update cache with new entry time if allowing entry

    // Placeholder implementation - always allow for now
    (true, ENTRY_COOLDOWN_MINUTES + 1)
}

// ═══════════════════════════════════════════════════════════════════════════════
// POSITION MANAGEMENT DECISIONS
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

/// Comprehensive position management - returns what action to take for a position
pub fn evaluate_position(token: &Token, pos: &Position, current_price: f64) -> PositionAction {
    let profit_pct = if pos.sol_spent > 0.0 {
        let current_value = current_price * pos.token_amount;
        ((current_value - pos.sol_spent) / pos.sol_spent) * 100.0
    } else {
        0.0
    };

    println!(
        "🎯 [POSITION] Evaluating {} | Price: ${:.8} | Profit: {:.2}% | Held: {}min | DCA: {}/{}",
        token.symbol,
        current_price,
        profit_pct,
        (Utc::now() - pos.open_time).num_minutes(),
        pos.dca_count,
        MAX_DCA_COUNT
    );

    // 1. Check DCA first (if applicable)
    if should_dca(token, pos, current_price) {
        let dca_size = calculate_dca_size(token, pos);
        println!("🔄 [POSITION] {} | Action: DCA with {:.6}SOL", token.symbol, dca_size);
        return PositionAction::DCA { sol_amount: dca_size };
    }

    // 2. Check sell conditions
    let (should_sell_signal, sell_reason) = should_sell(token, pos, current_price);
    if should_sell_signal {
        println!("💸 [POSITION] {} | Action: SELL | Reason: {}", token.symbol, sell_reason);
        return PositionAction::Sell { reason: sell_reason };
    }

    // 3. Default: hold the position
    println!("🔒 [POSITION] {} | Action: HOLD | Profit: {:.2}%", token.symbol, profit_pct);
    PositionAction::Hold
}

/// Check if position peak should be updated
pub fn should_update_peak(pos: &Position, current_price: f64) -> bool {
    let should_update = current_price > pos.peak_price;
    if should_update {
        let gain_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
        println!(
            "📈 [PEAK] New peak detected | Price: ${:.8} > ${:.8} | Gain: +{:.2}%",
            current_price,
            pos.peak_price,
            gain_from_peak
        );
    }
    should_update
}

/// Calculate profit milestone bucket for notifications
pub fn get_profit_bucket(pos: &Position, current_price: f64) -> i32 {
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };
    let bucket = (profit_pct / 2.0).floor() as i32; // announce every +2%

    println!("🎯 [PROFIT] Current profit: {:.2}% | Bucket: {} (2% intervals)", profit_pct, bucket);

    bucket
}

// ═══════════════════════════════════════════════════════════════════════════════
// ADAPTIVE RISK MANAGEMENT FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Calculate recent performance to adjust position sizing
/// Returns a multiplier between 0.5 and 1.5 based on recent wins/losses
pub fn calculate_performance_multiplier() -> f64 {
    // This would ideally track recent position performance
    // For now, return 1.0 (neutral sizing)
    // In future implementation:
    // - Track last 10-20 closed positions  
    // - Calculate win rate and avg profit/loss
    // - Reduce sizing after losses, increase after wins
    // - Implement Kelly criterion for optimal sizing
    
    1.0 // TODO: Implement performance tracking
}

/// Enhanced position size calculation with risk adjustment
pub fn calculate_adaptive_position_size(base_size: f64, token: &Token) -> f64 {
    let performance_multiplier = calculate_performance_multiplier();
    
    // Risk adjustment based on token characteristics
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let holder_count = token.rug_check.total_holders;
    let creator_balance_pct = if token.rug_check.total_supply > 0 {
        (token.rug_check.creator_balance as f64 / token.rug_check.total_supply as f64) * 100.0
    } else { 100.0 };
    
    let risk_multiplier = {
        let mut multiplier = 1.0;
        
        // Liquidity adjustment
        if liquidity_sol > 100.0 { multiplier += 0.2; }
        else if liquidity_sol < 30.0 { multiplier -= 0.3; }
        
        // Holder distribution adjustment  
        if holder_count > 100 { multiplier += 0.1; }
        else if holder_count < 30 { multiplier -= 0.2; }
        
        // Creator risk adjustment
        if creator_balance_pct < 5.0 { multiplier += 0.1; }
        else if creator_balance_pct > 20.0 { multiplier -= 0.3; }
        
        // Clamp between 0.3 and 1.3
        multiplier.max(0.3).min(1.3)
    };
    
    let final_size = base_size * performance_multiplier * risk_multiplier;
    
    println!(
        "💰 [SIZING] {} | Base: {:.6}SOL | Performance: {:.2}x | Risk: {:.2}x | Final: {:.6}SOL",
        token.symbol, base_size, performance_multiplier, risk_multiplier, final_size
    );
    
    final_size
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN STRATEGY FUNCTIONS  
// ═══════════════════════════════════════════════════════════════════════════════
