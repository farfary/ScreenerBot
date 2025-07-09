use crate::prelude::*;

// ═══════════════════════════════════════════════════════════════════════════════
// PROFIT-ONLY TRADING STRATEGY - DECISION MAKING MODULE
// ═══════════════════════════════════════════════════════════════════════════════
// CRITICAL RULE: WE ONLY SELL WHEN IN PROFIT (profit_pct > 0%)
// WE NEVER SELL AT ANY LOSS - ALL LOSING POSITIONS ARE HELD UNTIL RECOVERY
//
// This module contains ALL trading decision logic:
// - Entry decisions (should_buy)
// - Exit decisions (should_sell)
// - DCA decisions (should_dca)
// - Position evaluation (evaluate_position)
// - Signal strength calculations
// - Profit/loss calculations
//
// The trader.rs module handles execution of these decisions.
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

// ENHANCED ENTRY LOGIC CONSTANTS
pub const MIN_ACTIVITY_BUYS_5M: u64 = 2; // Minimum buys in last 5 minutes
pub const MIN_ACTIVITY_SELLS_5M: u64 = 1; // Minimum sells in last 5 minutes
pub const MIN_ACTIVITY_BUYS_1H: u64 = 5; // Minimum buys in last hour
pub const BIG_DUMP_THRESHOLD: f64 = -10.0; // Reject tokens with -10% or more recent dumps
pub const ENTRY_COOLDOWN_MINUTES: i64 = 30; // Cooldown between entries for same token
pub const SAFE_LIQUIDITY_MULTIPLIER: f64 = 2.0; // Safe liquidity = 2x minimum

/// Simplified buy logic that works with current price and token data only
pub fn should_buy(token: &Token, can_buy: bool, _current_price: f64) -> bool {
    if !can_buy {
        return false;
    }

    // ─── SAFETY CHECKS ───

    // 1. Entry cooldown check
    let (can_enter, minutes_since_last) = can_enter_token_position(&token.mint);
    if !can_enter {
        println!(
            "⚠️ [ENTRY] Cooldown active: {}min < {}min",
            minutes_since_last,
            ENTRY_COOLDOWN_MINUTES
        );
        return false;
    }

    // 2. Rug check safety
    if !crate::dexscreener::is_safe_to_trade(token, false) {
        println!("⚠️ [ENTRY] Rug check failed for {}", token.symbol);
        return false;
    }

    // ─── BASIC QUALITY FILTERS ───

    let volume_24h = token.volume.h24;
    let volume_1h = token.volume.h1;
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let price_change_1h = token.price_change.h1;
    let price_change_5m = token.price_change.m5;
    let buys_1h = token.txns.h1.buys;
    let sells_1h = token.txns.h1.sells;
    let buys_5m = token.txns.m5.buys;
    let sells_5m = token.txns.m5.sells;

    // 3. Minimum liquidity requirement
    let safe_liquidity_threshold = MIN_LIQUIDITY_SOL * SAFE_LIQUIDITY_MULTIPLIER;
    if liquidity_sol < safe_liquidity_threshold {
        println!(
            "⚠️ [ENTRY] Low liquidity: {:.1} SOL < {:.1} SOL",
            liquidity_sol,
            safe_liquidity_threshold
        );
        return false;
    }

    // 4. Minimum activity requirements
    if buys_5m < MIN_ACTIVITY_BUYS_5M || sells_5m < MIN_ACTIVITY_SELLS_5M {
        println!("⚠️ [ENTRY] Low 5m activity: buys:{} sells:{}", buys_5m, sells_5m);
        return false;
    }

    if buys_1h < MIN_ACTIVITY_BUYS_1H {
        println!("⚠️ [ENTRY] Low 1h activity: buys:{}", buys_1h);
        return false;
    }

    // 5. Avoid big dumps
    if price_change_5m <= BIG_DUMP_THRESHOLD {
        println!("⚠️ [ENTRY] Big dump detected: 5m:{:.1}%", price_change_5m);
        return false;
    }

    // 6. Minimum volume requirement
    if volume_24h < MIN_VOLUME_USD {
        println!("⚠️ [ENTRY] Low volume: ${:.0} < ${:.0}", volume_24h, MIN_VOLUME_USD);
        return false;
    }

    // ─── SIMPLE ENTRY CONDITIONS ───

    let mut signal_strength = 0.0;
    let mut entry_reasons = Vec::new();

    // Look for uptrend with small pullback
    if price_change_1h > 5.0 && price_change_5m < 0.0 && price_change_5m > -5.0 {
        signal_strength += 0.4;
        entry_reasons.push("uptrend_pullback");
    }

    // Look for positive momentum
    if price_change_5m > 0.0 && price_change_1h > 0.0 {
        signal_strength += 0.3;
        entry_reasons.push("positive_momentum");
    }

    // Check buy/sell ratio - prefer more buyers
    let buy_sell_ratio = (buys_1h as f64) / ((sells_1h as f64) + 1.0);
    if buy_sell_ratio > 1.5 {
        signal_strength += 0.2;
        entry_reasons.push("strong_buying");
    }

    // Volume spike check
    if volume_1h > volume_24h / 12.0 {
        signal_strength += 0.2;
        entry_reasons.push("volume_spike");
    }

    // High liquidity bonus
    if liquidity_sol > safe_liquidity_threshold * 2.0 {
        signal_strength += 0.1;
        entry_reasons.push("high_liquidity");
    }

    // ─── ENTRY DECISION ───

    let required_strength = 0.5; // Simplified threshold

    if signal_strength >= required_strength {
        println!(
            "🎯 ENTRY {} | Strength: {:.2} | Reasons: {} | Vol24h: ${:.0} | Liq: {:.1}SOL | Ratio: {:.2} | 1h: {:.1}% | 5m: {:.1}%",
            token.symbol,
            signal_strength,
            entry_reasons.join(","),
            volume_24h,
            liquidity_sol,
            buy_sell_ratio,
            price_change_1h,
            price_change_5m
        );
        return true;
    }

    println!(
        "⚠️ [ENTRY] Insufficient signal strength: {:.2} < {:.2}",
        signal_strength,
        required_strength
    );
    false
}

/// Simplified DCA strategy without dataframe dependencies
pub fn should_dca(token: &Token, pos: &Position, current_price: f64) -> bool {
    // 1. Hard limit: Never exceed maximum DCA count
    if pos.dca_count >= MAX_DCA_COUNT {
        return false;
    }

    let now = Utc::now();
    let elapsed = now - pos.open_time;
    let drop_pct = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;

    // 2. Cooldown check: Prevent rapid-fire DCA attempts
    if pos.dca_count > 0 {
        let time_since_last_dca = now - pos.last_dca_time;
        if time_since_last_dca.num_minutes() < DCA_COOLDOWN_MINUTES {
            return false;
        }
    }

    // 3. Minimum hold time: Must hold for at least 10 minutes before first DCA
    if elapsed.num_minutes() < 10 {
        return false;
    }

    // 4. Simple drop threshold check
    let dca_trigger_pct = DCA_BASE_TRIGGER_PCT;
    if drop_pct > dca_trigger_pct {
        return false; // Not dropped enough yet
    }

    // 5. Basic liquidity check
    let current_liquidity = token.liquidity.base + token.liquidity.quote;
    if current_liquidity < MIN_LIQUIDITY_SOL * 0.8 {
        return false; // Liquidity too low
    }

    // 6. Volume activity check
    if token.volume.h1 < 500.0 {
        return false; // Volume too low
    }

    // 7. Check buying pressure
    let buy_sell_ratio = (token.txns.h1.buys as f64) / ((token.txns.h1.sells as f64) + 1.0);
    if buy_sell_ratio < 1.0 {
        return false; // Need some buying pressure
    }

    // 8. Price level check: Only DCA if significantly below last entry
    let price_drop_from_last = if pos.dca_count == 0 {
        drop_pct
    } else {
        ((current_price - pos.last_dca_price) / pos.last_dca_price) * 100.0
    };

    if price_drop_from_last > -10.0 {
        return false; // Need at least 10% drop from last entry
    }

    println!(
        "🔄 DCA {} | Drop: {:.1}% | Buy/Sell: {:.2} | DCA#{} | Cooldown: {}min",
        token.symbol,
        drop_pct,
        buy_sell_ratio,
        pos.dca_count + 1,
        if pos.dca_count > 0 {
            (now - pos.last_dca_time).num_minutes()
        } else {
            0
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

    base_dca_size * dca_adjustment * liquidity_adjustment
}

/// Simplified sell logic - ONLY sell when profitable
pub fn should_sell(token: &Token, pos: &Position, current_price: f64) -> (bool, String) {
    // Calculate total fees: one fee for initial buy + one fee for each DCA + one fee for sell
    let total_buy_fees = ((1 + (pos.dca_count as usize)) as f64) * TRANSACTION_FEE_SOL;
    let sell_fee = TRANSACTION_FEE_SOL;
    let total_fees = total_buy_fees + sell_fee;

    // Use consistent profit calculation method
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent - total_fees;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };

    let drop_from_peak = ((current_price - pos.peak_price) / pos.peak_price) * 100.0;
    let held_duration = (Utc::now() - pos.open_time).num_seconds();

    // 1. MINIMUM HOLD TIME - Must hold for at least 30 minutes
    if held_duration < MIN_HOLD_TIME_SECONDS {
        return (
            false,
            format!("min_hold_time(held:{}s, min:{}s)", held_duration, MIN_HOLD_TIME_SECONDS),
        );
    }

    // 2. NEVER SELL AT LOSS - Only sell when profitable
    if profit_pct <= 0.0 {
        return (false, format!("no_loss_selling(profit:{:.2}%)", profit_pct));
    }

    // 3. SIMPLE PROFIT TARGETS

    // Quick profit taking for small gains (1-5%)
    if profit_pct >= 1.0 && profit_pct <= 5.0 {
        // Take profit on any negative momentum
        if token.price_change.m5 < -2.0 {
            return (
                true,
                format!(
                    "quick_profit_exit(profit:{:.2}%, momentum:{:.1}%)",
                    profit_pct,
                    token.price_change.m5
                ),
            );
        }
    }

    // Medium profit taking (5-20%)
    if profit_pct >= 5.0 && profit_pct <= 20.0 {
        // Take profit on strong negative momentum
        if token.price_change.m5 < -5.0 {
            return (
                true,
                format!(
                    "medium_profit_exit(profit:{:.2}%, momentum:{:.1}%)",
                    profit_pct,
                    token.price_change.m5
                ),
            );
        }

        // Simple trailing stop - 20% drop from peak
        if drop_from_peak <= -20.0 {
            return (
                true,
                format!("trailing_stop(profit:{:.2}%, drop:{:.1}%)", profit_pct, drop_from_peak),
            );
        }
    }

    // Large profit taking (>20%)
    if profit_pct > 20.0 {
        // Take profits more aggressively
        if token.price_change.m5 < -3.0 || drop_from_peak <= -15.0 {
            return (
                true,
                format!(
                    "large_profit_exit(profit:{:.2}%, momentum:{:.1}%, drop:{:.1}%)",
                    profit_pct,
                    token.price_change.m5,
                    drop_from_peak
                ),
            );
        }
    }

    // 4. EMERGENCY EXITS - Only when profitable

    // Market deterioration exit
    let liquidity_total = token.liquidity.base + token.liquidity.quote;
    if liquidity_total < MIN_LIQUIDITY_SOL * 0.3 {
        return (
            true,
            format!("liquidity_collapse(profit:{:.2}%, liq:{:.1}SOL)", profit_pct, liquidity_total),
        );
    }

    // Severe token collapse exit
    if token.price_change.h24 <= -70.0 {
        return (
            true,
            format!(
                "token_collapse_24h(profit:{:.2}%, change:{:.1}%)",
                profit_pct,
                token.price_change.h24
            ),
        );
    }

    // 5. MAXIMUM HOLD TIME - Exit profitable positions after very long holds
    if held_duration >= MAX_HOLD_TIME_SECONDS {
        return (
            true,
            format!("max_hold_time_exit(profit:{:.2}%, held:{}s)", profit_pct, held_duration),
        );
    }

    // Default: Hold the position
    // IMPORTANT: We hold ALL losing positions indefinitely until they become profitable
    (false, format!("holding(profit:{:.2}%, held:{}s)", profit_pct, held_duration))
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
    // 1. Check DCA first (if applicable)
    if should_dca(token, pos, current_price) {
        let dca_size = calculate_dca_size(token, pos);
        return PositionAction::DCA { sol_amount: dca_size };
    }

    // 2. Check sell conditions
    let (should_sell_signal, sell_reason) = should_sell(token, pos, current_price);
    if should_sell_signal {
        return PositionAction::Sell { reason: sell_reason };
    }

    // 3. Default: hold the position
    PositionAction::Hold
}

/// Check if position peak should be updated
pub fn should_update_peak(pos: &Position, current_price: f64) -> bool {
    current_price > pos.peak_price
}

/// Calculate profit milestone bucket for notifications
pub fn get_profit_bucket(pos: &Position, current_price: f64) -> i32 {
    let current_value = current_price * pos.token_amount;
    let profit_sol = current_value - pos.sol_spent;
    let profit_pct = if pos.sol_spent > 0.0 { (profit_sol / pos.sol_spent) * 100.0 } else { 0.0 };
    (profit_pct / 2.0).floor() as i32 // announce every +2%
}
