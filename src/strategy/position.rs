use crate::prelude::*;
use super::config::*;
use super::price_analysis::calculate_trade_size_sol;
use super::dca::should_dca;
use super::exit::should_sell;

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

    // Calculate dynamic trade size based on current liquidity
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    let dynamic_trade_size = calculate_trade_size_sol(liquidity_sol);

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

/// Calculate DCA size based on current liquidity
pub fn calculate_dca_size(token: &Token, _pos: &Position) -> f64 {
    let liquidity_sol = token.liquidity.base + token.liquidity.quote;
    calculate_trade_size_sol(liquidity_sol)
}
