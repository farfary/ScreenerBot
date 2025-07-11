use crate::prelude::*;
use super::config::*;
use super::price_analysis::{ calculate_trade_size_sol, calculate_trade_size_with_market_cap };
use super::dca::should_dca;
use super::exit::should_sell;

/// Check if we can enter a position for this token (cooldown management)
pub async fn can_enter_token_position(token_mint: &str) -> (bool, i64) {
    let now = Utc::now();

    // Check if we currently have an open position for this token
    let has_open_position = {
        let positions = OPEN_POSITIONS.read().await;
        positions.contains_key(token_mint)
    };

    if has_open_position {
        return (false, 0); // Can't enter if we already have a position
    }

    // Check recent closed positions for this token
    let (can_enter, minutes_since_last) = {
        let closed_positions = CLOSED_POSITIONS.read().await;

        if let Some(last_position) = closed_positions.get(token_mint) {
            if let Some(close_time) = last_position.close_time {
                let time_since_close = now - close_time;
                let minutes_since = time_since_close.num_minutes();

                // Calculate profit from last position
                let profit_pct = if last_position.sol_spent > 0.0 {
                    ((last_position.sol_received - last_position.sol_spent) /
                        last_position.sol_spent) *
                        100.0
                } else {
                    0.0
                };

                // Determine cooldown based on exit outcome
                let required_cooldown_hours = if profit_pct >= MIN_PROFIT_EXIT_THRESHOLD_PCT {
                    PROFITABLE_EXIT_COOLDOWN_HOURS
                } else {
                    LOSS_EXIT_COOLDOWN_HOURS
                };

                let required_cooldown_minutes = required_cooldown_hours * 60;

                if minutes_since < required_cooldown_minutes {
                    (false, minutes_since)
                } else {
                    (true, minutes_since)
                }
            } else {
                // Position without close time, use general cooldown
                let required_cooldown_minutes = SAME_TOKEN_ENTRY_COOLDOWN_HOURS * 60;
                (true, required_cooldown_minutes + 1) // Allow entry
            }
        } else {
            // No previous position found
            (true, ENTRY_COOLDOWN_MINUTES + 1) // Allow entry
        }
    };

    (can_enter, minutes_since_last)
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

pub async fn evaluate_position(
    token: &Token,
    pos: &Position,
    current_price: f64
) -> PositionAction {
    let profit_pct = if pos.sol_spent > 0.0 {
        let current_value = current_price * pos.token_amount;
        ((current_value - pos.sol_spent) / pos.sol_spent) * 100.0
    } else {
        0.0
    };

    // Calculate dynamic trade size based on current liquidity and market cap
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let market_cap = token.fdv_usd.parse::<f64>().unwrap_or(0.0); // Parse FDV from string
    let dynamic_trade_size = calculate_trade_size_with_market_cap(liquidity_sol, market_cap);

    println!(
        "🎯 [POSITION] {} | Price: ${:.8} | Profit: {:.2}% | DCA: {}/{} | TradeSize: {:.4}SOL",
        token.symbol,
        current_price,
        profit_pct,
        pos.dca_count,
        MAX_DCA_COUNT,
        dynamic_trade_size
    );

    // Get trades data for this token
    let trades_data = crate::trades::get_token_trades(&token.mint).await;

    // Get OHLCV dataframe for this token
    let ohlcv_dataframe = crate::ohlcv::get_token_ohlcv_dataframe(&token.mint).await;

    // ✅ FIXED: Add validation for DCA timing and conditions
    let now = Utc::now();
    let time_since_open = now - pos.open_time;
    let time_since_last_dca = if pos.dca_count > 0 {
        now - pos.last_dca_time
    } else {
        time_since_open
    };

    // 1. Check DCA with enhanced validation
    if should_dca(token, pos, current_price, trades_data.as_ref(), ohlcv_dataframe.as_ref()) {
        // ✅ Additional safety checks for DCA
        if time_since_open.num_minutes() < 30 {
            println!(
                "⏰ [POSITION] {} | DCA blocked: position too new ({} min)",
                token.symbol,
                time_since_open.num_minutes()
            );
            return PositionAction::Hold;
        }

        if pos.dca_count > 0 && time_since_last_dca.num_minutes() < 20 {
            println!(
                "⏰ [POSITION] {} | DCA blocked: last DCA too recent ({} min ago)",
                token.symbol,
                time_since_last_dca.num_minutes()
            );
            return PositionAction::Hold;
        }

        // Check if price has actually dropped significantly since entry
        let drop_from_entry = ((current_price - pos.entry_price) / pos.entry_price) * 100.0;
        if drop_from_entry > -5.0 {
            println!(
                "📈 [POSITION] {} | DCA blocked: insufficient drop from entry ({:.1}%)",
                token.symbol,
                drop_from_entry
            );
            return PositionAction::Hold;
        }

        return PositionAction::DCA { sol_amount: dynamic_trade_size };
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

/// Calculate DCA size based on current liquidity and market cap
pub fn calculate_dca_size(token: &Token, _pos: &Position) -> f64 {
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let market_cap = token.fdv_usd.parse::<f64>().unwrap_or(0.0);
    let base_size = calculate_trade_size_with_market_cap(liquidity_sol, market_cap);

    // Apply DCA size factor
    base_size * DCA_SIZE_FACTOR
}
