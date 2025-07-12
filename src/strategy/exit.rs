use crate::prelude::*;
use crate::price_validation::{ is_price_valid, get_trading_price };
use super::config::*;
use super::helpers::*;

/// NEW SMART PROFIT-TAKING STRATEGY V3.0
///
/// Focus: Always profit through smart exit timing
/// Approach: Multiple profit targets with dynamic sizing
/// Key: Never sell at loss - always wait for profit
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
    let validated_price = if let Some(trading_price) = get_trading_price(&token.mint) {
        let price_diff = (((current_price - trading_price) / trading_price) * 100.0).abs();
        if price_diff > PRICE_VALIDATION_TOLERANCE * 100.0 {
            println!(
                "⚠️ [SELL] {} | Price mismatch: current={:.12}, cached={:.12} ({:.1}% diff) - using cached",
                token.symbol,
                current_price,
                trading_price,
                price_diff
            );
            trading_price
        } else {
            current_price
        }
    } else {
        current_price
    };

    let current_profit = ((validated_price - pos.entry_price) / pos.entry_price) * 100.0;
    let elapsed = Utc::now() - pos.open_time;

    println!(
        "\n💰 [EXIT] {} | Profit: {:.2}% | Held: {}min | DCA: {}",
        token.symbol,
        current_profit,
        elapsed.num_minutes(),
        pos.dca_count
    );

    // ✅ STEP 1: NEVER SELL AT LOSS (CORE PRINCIPLE)
    if current_profit < 0.0 {
        // Check for emergency exit conditions (rug protection)
        if should_emergency_exit(token, pos, current_profit, elapsed) {
            return (true, format!("emergency_exit({:.2}%)", current_profit));
        }

        println!("❌ [EXIT] {} | Never sell at loss: {:.2}%", token.symbol, current_profit);
        return (false, format!("holding_for_profit({:.2}%)", current_profit));
    }

    // ✅ STEP 2: CREATE TOKEN PROFILE FOR DYNAMIC EXITS
    let token_profile = TokenProfile::from_token(token);
    let profit_calculator = ProfitTargetCalculator::default();

    // ✅ STEP 3: CHECK FOR IMMEDIATE PROFIT OPPORTUNITIES
    if
        let Some(profit_decision) = profit_calculator.should_take_immediate_profit(
            token,
            pos,
            validated_price
        )
    {
        if profit_decision.should_sell {
            let reason = format!(
                "profit_target({:.2}%_to_{:.1}%)",
                current_profit,
                profit_decision.target.percentage
            );

            println!(
                "✅ [PROFIT] {} | {} | Confidence: {:.2} | Size: {:.0}%",
                token.symbol,
                profit_decision.target.reason,
                profit_decision.confidence,
                profit_decision.target.size_to_sell * 100.0
            );

            return (true, reason);
        }
    }

    // ✅ STEP 4: DYNAMIC PROFIT TAKING BASED ON CONDITIONS
    if
        let Some((should_exit, reason)) = check_dynamic_exit_conditions(
            token,
            pos,
            validated_price,
            current_profit,
            &token_profile,
            elapsed
        )
    {
        if should_exit {
            return (true, reason);
        }
    }

    // ✅ STEP 5: TIME-BASED PROFIT TAKING
    if
        let Some((should_exit, reason)) = check_time_based_exits(
            token,
            pos,
            current_profit,
            elapsed,
            &token_profile
        )
    {
        if should_exit {
            return (true, reason);
        }
    }

    // ✅ STEP 6: MOMENTUM-BASED EXITS
    if
        let Some((should_exit, reason)) = check_momentum_exits(
            token,
            pos,
            validated_price,
            current_profit,
            dataframe
        )
    {
        if should_exit {
            return (true, reason);
        }
    }

    println!("❌ [EXIT] {} | Holding for better profit opportunity", token.symbol);
    (false, format!("holding({:.2}%)", current_profit))
}

/// Check for emergency exit conditions (rug protection)
fn should_emergency_exit(
    token: &Token,
    pos: &Position,
    current_profit: f64,
    elapsed: chrono::Duration
) -> bool {
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;

    // Emergency exit for extreme losses (rug protection)
    if current_profit <= -50.0 {
        println!("🚨 [EMERGENCY] {} | Extreme loss: {:.2}%", token.symbol, current_profit);
        return true;
    }

    // Emergency exit for liquidity collapse
    if liquidity_sol < MIN_LIQUIDITY_SOL * 0.5 {
        println!("🚨 [EMERGENCY] {} | Liquidity collapse: {:.1} SOL", token.symbol, liquidity_sol);
        return true;
    }

    // Emergency exit for very old positions with significant loss
    if elapsed.num_hours() > (MAX_POSITION_HOLD_HOURS as i64) && current_profit <= -20.0 {
        println!(
            "🚨 [EMERGENCY] {} | Old position with loss: {:.2}% after {}h",
            token.symbol,
            current_profit,
            elapsed.num_hours()
        );
        return true;
    }

    false
}

/// Check dynamic exit conditions based on market and token characteristics
fn check_dynamic_exit_conditions(
    token: &Token,
    pos: &Position,
    current_price: f64,
    current_profit: f64,
    profile: &TokenProfile,
    elapsed: chrono::Duration
) -> Option<(bool, String)> {
    // Quick profit for small gains (high success rate strategy)
    if current_profit >= MIN_PROFIT_TARGET && current_profit < 1.0 {
        // Take quick profits for non-famous tokens
        if !profile.is_famous && elapsed.num_minutes() >= 10 {
            return Some((true, format!("quick_profit({:.2}%)", current_profit)));
        }
    }

    // Medium profit targets
    if current_profit >= QUICK_PROFIT_TARGET {
        // Check liquidity conditions for safe exit
        let liquidity_sol = token.liquidity.base + token.liquidity.quote;
        if liquidity_sol > MIN_LIQUIDITY_SOL * 2.0 && token.volume.h1 > 1000.0 {
            return Some((true, format!("medium_profit({:.2}%)", current_profit)));
        }
    }

    // Large profit targets (secure gains)
    if current_profit >= MAIN_PROFIT_TARGET {
        return Some((true, format!("large_profit({:.2}%)", current_profit)));
    }

    // Huge profit targets (moon shots)
    if current_profit >= BIG_PROFIT_TARGET {
        return Some((true, format!("moon_profit({:.2}%)", current_profit)));
    }

    None
}

/// Check time-based exit conditions
fn check_time_based_exits(
    token: &Token,
    pos: &Position,
    current_profit: f64,
    elapsed: chrono::Duration,
    profile: &TokenProfile
) -> Option<(bool, String)> {
    let hours_held = elapsed.num_hours();

    // Quick exits for small profits after reasonable time
    if current_profit >= 0.5 && hours_held >= 2 {
        return Some((true, format!("time_profit({:.2}%_after_{}h)", current_profit, hours_held)));
    }

    // Medium time exits for decent profits
    if current_profit >= 1.0 && hours_held >= 4 {
        return Some((
            true,
            format!("medium_time_profit({:.2}%_after_{}h)", current_profit, hours_held),
        ));
    }

    // Long hold exits for any profit
    if
        current_profit >= 0.3 &&
        hours_held >= (profile.get_trading_config().max_hold_time_hours as i64)
    {
        return Some((
            true,
            format!("max_time_profit({:.2}%_after_{}h)", current_profit, hours_held),
        ));
    }

    None
}

/// Check momentum-based exit conditions
fn check_momentum_exits(
    token: &Token,
    pos: &Position,
    current_price: f64,
    current_profit: f64,
    dataframe: Option<&crate::ohlcv::TokenOhlcvCache>
) -> Option<(bool, String)> {
    // Only exit on momentum if we have decent profit
    if current_profit < 1.0 {
        return None;
    }

    // Check for negative momentum indicators
    let recent_change = token.price_change.m5;

    // Exit on negative momentum with good profit
    if recent_change <= -5.0 && current_profit >= 2.0 {
        return Some((
            true,
            format!("momentum_exit({:.2}%_on_{:.1}%_drop)", current_profit, recent_change),
        ));
    }

    // Exit on very negative momentum with any profit
    if recent_change <= -10.0 && current_profit >= 0.5 {
        return Some((
            true,
            format!("strong_momentum_exit({:.2}%_on_{:.1}%_drop)", current_profit, recent_change),
        ));
    }

    None
}

/// Calculate optimal exit size (partial vs full exit)
pub fn calculate_exit_size(
    token: &Token,
    pos: &Position,
    current_profit: f64,
    exit_reason: &str
) -> f64 {
    // Full exit conditions
    if
        exit_reason.contains("emergency") ||
        exit_reason.contains("moon") ||
        current_profit >= BIG_PROFIT_TARGET
    {
        return 1.0; // 100% exit
    }

    // Large profit - take most but leave some
    if current_profit >= MAIN_PROFIT_TARGET {
        return 0.8; // 80% exit
    }

    // Medium profit - take half
    if current_profit >= QUICK_PROFIT_TARGET {
        return 0.6; // 60% exit
    }

    // Small profit - take some
    if current_profit >= MIN_PROFIT_TARGET {
        return 0.4; // 40% exit
    }

    // Default full exit
    1.0
}

/// Get current position value in SOL
pub fn get_position_value_sol(pos: &Position, current_price: f64) -> f64 {
    pos.token_amount * current_price
}

/// Check if position should be considered for forced exit due to time
pub fn is_stale_position(pos: &Position, max_hours: u64) -> bool {
    let elapsed = Utc::now() - pos.open_time;
    elapsed.num_hours() >= (max_hours as i64)
}
