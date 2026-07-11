//! Position profit & loss calculations — P&L for open, closed, and partially exited positions.

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::types::Position;
use crate::tokens::get_decimals;
use crate::utils::lamports_to_sol;

// ==================== P&L CALCULATION ====================

/// Unified profit/loss calculation for both open and closed positions
/// Supports partial exits and DCA with average entry price
/// Returns (pnl_sol, pnl_percent)
/// For open positions: calculates both realized (from partial exits) and unrealized P&L
/// For closed positions: returns only realized P&L
///
/// NOTE: Returns (0.0, 0.0) if calculation fails due to invalid prices.
/// Use calculate_position_pnl_safe() wrapper to distinguish between zero PnL and errors.
pub async fn calculate_position_pnl(position: &Position, current_price: Option<f64>) -> (f64, f64) {
    // Use average_entry_price for positions with DCA support
    let entry_price =
        if position.average_entry_price > 0.0 && position.average_entry_price.is_finite() {
            position.average_entry_price
        } else {
            // Fallback to legacy prices
            position
                .effective_entry_price
                .unwrap_or(position.entry_price)
        };

    if entry_price <= 0.0 || !entry_price.is_finite() {
        // Log invalid entry price for diagnostics; logger will filter by level
        logger::debug(
            LogTag::Positions,
            &format!(
                "Invalid entry price for {}: {}",
                position.symbol, entry_price
            ),
        );
        // Invalid entry price - return neutral P&L to avoid triggering emergency exits
        return (0.0, 0.0);
    }

    // For open positions, validate current price if provided
    if let Some(current) = current_price {
        if current <= 0.0 || !current.is_finite() {
            // Invalid current price - return neutral P&L to avoid false emergency signals
            return (0.0, 0.0);
        }
    }

    // For positions with pending exit transactions (closing in progress), use current price for estimation
    if position.exit_transaction_signature.is_some() && !position.transaction_exit_verified {
        if let Some(current) = current_price {
            let entry_price = position
                .effective_entry_price
                .unwrap_or(position.entry_price);
            // total_size_sol includes DCA adds; entry_size_sol is only the first buy, so
            // it understated the cost of every averaged-in position being closed.
            let entry_cost = position.total_size_sol;
            // Proceeds from any partial exits taken before this close.
            let realized_sol = position.sol_received.unwrap_or_default();

            // Calculate estimated P&L based on current price (closing in progress). Value
            // what is LEFT — after partial exits the original token_amount is no longer in
            // the wallet.
            if let Some(token_amount) = position.remaining_token_amount.or(position.token_amount) {
                let token_decimals_opt = get_decimals(&position.mint).await;
                if let Some(token_decimals) = token_decimals_opt {
                    let ui_token_amount =
                        (token_amount as f64) / (10_f64).powi(token_decimals as i32);
                    let current_value = ui_token_amount * current;

                    // Account for actual fees only — profit_extra_needed is a decision
                    // buffer, not an actual cost (inflates losses ~2% on small trades)
                    let buy_fee = position
                        .entry_fee_lamports
                        .map_or(0.0, |fee| lamports_to_sol(fee));
                    let estimated_sell_fee = buy_fee;
                    let total_fees = buy_fee + estimated_sell_fee;
                    let net_pnl_sol = current_value + realized_sol - entry_cost - total_fees;
                    let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

                    return (net_pnl_sol, net_pnl_percent);
                }
            }

            // Fallback calculation for closing positions
            let price_change = (current - entry_price) / entry_price;
            let buy_fee = position
                .entry_fee_lamports
                .map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee;
            let total_fees = buy_fee + estimated_sell_fee;
            let fee_percent = (total_fees / entry_cost) * 100.0;
            let net_pnl_percent = price_change * 100.0 - fee_percent;
            let net_pnl_sol = (net_pnl_percent / 100.0) * entry_cost;

            return (net_pnl_sol, net_pnl_percent);
        }
    }

    // Is the position actually CLOSED? `exit_price` alone does NOT mean closed: it is the
    // market price stamped by close_position_direct BEFORE the swap, and a close that then
    // failed (or only partially filled) leaves it set on a position that is still open —
    // ExitFailedClearForRetry clears the signature, not this. Keying "closed" off it meant a
    // live position with a failed close fell into the closed branches below and had its P&L
    // computed as proceeds-minus-cost, ignoring every token it still holds: a near-total
    // loss on a healthy position. check_risk_limits force-exits at >90% loss on that number.
    let is_closed = position.exit_time.is_some();

    // For closed positions, prioritize sol_received for most accurate P&L
    if let (true, Some(exit_price), Some(sol_received)) =
        (is_closed, position.exit_price, position.sol_received)
    {
        // Total SOL invested vs total SOL received. `total_size_sol` includes every DCA add
        // (it equals entry_size_sol when there was none), whereas entry_size_sol counts only
        // the first buy — so averaging down understated the cost basis and overstated profit.
        // `sol_received` accumulates partial exits plus the final close.
        let sol_invested = position.total_size_sol;

        // Use actual transaction fees only — profit_extra_needed is a decision buffer,
        // not an actual cost. Including it here inflates losses by ~2% on small trades.
        let buy_fee = position
            .entry_fee_lamports
            .map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position
            .exit_fee_lamports
            .map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee;

        let net_pnl_sol = sol_received - sol_invested - total_fees;
        let safe_invested = if sol_invested < 0.00001 {
            0.00001
        } else {
            sol_invested
        };
        let net_pnl_percent = (net_pnl_sol / safe_invested) * 100.0;

        return (net_pnl_sol, net_pnl_percent);
    }

    // Fallback for closed positions without sol_received (backward compatibility).
    // Same gate: `exit_price` is set before the swap, so it is present on open positions
    // whose close failed — those must fall through to the OPEN calculation below.
    if let (true, Some(exit_price)) = (is_closed, position.exit_price) {
        let entry_price = position
            .effective_entry_price
            .unwrap_or(position.entry_price);
        let effective_exit = position.effective_exit_price.unwrap_or(exit_price);

        // For closed positions: actual transaction-based calculation
        if let Some(token_amount) = position.token_amount {
            // Get token decimals from cache (async)
            let token_decimals_opt = get_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    logger::error(
                        LogTag::Positions,
                        &format!(
              "Cannot calculate P&L for {} - decimals not available, skipping calculation",
              position.mint
            ),
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let entry_cost = position.entry_size_sol;
            let exit_value = ui_token_amount * effective_exit;

            // Account for actual buy + sell fees only (no profit buffer for realized P&L)
            let buy_fee = position
                .entry_fee_lamports
                .map_or(0.0, |fee| lamports_to_sol(fee));
            let sell_fee = position
                .exit_fee_lamports
                .map_or(0.0, |fee| lamports_to_sol(fee));
            let total_fees = buy_fee + sell_fee;
            let net_pnl_sol = exit_value - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for closed positions without token amount
        let price_change = (effective_exit - entry_price) / entry_price;
        let buy_fee = position
            .entry_fee_lamports
            .map_or(0.0, |fee| lamports_to_sol(fee));
        let sell_fee = position
            .exit_fee_lamports
            .map_or(0.0, |fee| lamports_to_sol(fee));
        let total_fees = buy_fee + sell_fee;
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // For open positions (including those with partial exits), use current price
    if let Some(current) = current_price {
        // Use remaining_token_amount for positions with partial exits, fallback to token_amount
        let remaining_amount = position.remaining_token_amount.or(position.token_amount);

        if let Some(token_amount) = remaining_amount {
            // Get token decimals from cache (async)
            let token_decimals_opt = get_decimals(&position.mint).await;

            // CRITICAL: Skip P&L calculation if decimals are not available
            let token_decimals = match token_decimals_opt {
                Some(decimals) => decimals,
                None => {
                    logger::info(
                        LogTag::Positions,
                        &format!(
              "Cannot calculate P&L for {} - decimals not available, skipping calculation",
              position.mint
            ),
                    );
                    return (0.0, 0.0); // Return zero P&L instead of wrong calculation
                }
            };

            let ui_token_amount = (token_amount as f64) / (10_f64).powi(token_decimals as i32);
            let current_value = ui_token_amount * current;

            // Use total_size_sol (includes DCA) instead of entry_size_sol
            let entry_cost = position.total_size_sol;

            // SOL already taken off the table by partial exits. `sol_received` accumulates
            // across them, and for a still-open position it is exactly the realized
            // proceeds. Omitting it valued the REMAINING tokens against the FULL entry
            // cost, so a position that sold half at 2x — already break-even in cash —
            // reported roughly a 50% LOSS, and the exit monitor sees that number.
            let realized_sol = position.sol_received.unwrap_or_default();

            // Account for actual fees only — profit_extra_needed is a decision
            // buffer, not an actual cost (inflates losses ~2% on small trades)
            let buy_fee = position
                .entry_fee_lamports
                .map_or(0.0, |fee| lamports_to_sol(fee));
            let estimated_sell_fee = buy_fee;
            let total_fees = buy_fee + estimated_sell_fee;
            let net_pnl_sol = current_value + realized_sol - entry_cost - total_fees;
            let net_pnl_percent = (net_pnl_sol / entry_cost) * 100.0;

            return (net_pnl_sol, net_pnl_percent);
        }

        // Fallback for open positions without token amount
        let price_change = (current - entry_price) / entry_price;
        let buy_fee = position
            .entry_fee_lamports
            .map_or(0.0, |fee| lamports_to_sol(fee));
        let estimated_sell_fee = buy_fee;
        let total_fees = buy_fee + estimated_sell_fee;
        let fee_percent = (total_fees / position.entry_size_sol) * 100.0;
        let net_pnl_percent = price_change * 100.0 - fee_percent;
        let net_pnl_sol = (net_pnl_percent / 100.0) * position.entry_size_sol;

        return (net_pnl_sol, net_pnl_percent);
    }

    // No price available
    (0.0, 0.0)
}

/// Calculate total fees for a position
pub fn calculate_position_total_fees(position: &Position) -> f64 {
    // Sum entry and exit fees in SOL (excluding ATA rent from trading costs)
    let entry_fees_sol = lamports_to_sol(position.entry_fee_lamports.unwrap_or_default());
    let exit_fees_sol = lamports_to_sol(position.exit_fee_lamports.unwrap_or_default());
    entry_fees_sol + exit_fees_sol
}

/// Safe wrapper around calculate_position_pnl that returns Option to distinguish errors
/// Returns None if calculation fails (invalid prices, missing decimals, etc.)
/// Returns Some((pnl_sol, pnl_percent)) for successful calculations (may be zero for break-even)
pub async fn calculate_position_pnl_safe(
    position: &Position,
    current_price: Option<f64>,
) -> Option<(f64, f64)> {
    // Pre-validate entry price
    let entry_price =
        if position.average_entry_price > 0.0 && position.average_entry_price.is_finite() {
            position.average_entry_price
        } else {
            position
                .effective_entry_price
                .unwrap_or(position.entry_price)
        };

    if entry_price <= 0.0 || !entry_price.is_finite() {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Cannot calculate PnL for {} - invalid entry price: {}",
                position.symbol, entry_price
            ),
        );
        return None;
    }

    // Pre-validate current price for open positions
    if let Some(current) = current_price {
        if current <= 0.0 || !current.is_finite() {
            logger::debug(
                LogTag::Positions,
                &format!(
                    "Cannot calculate PnL for {} - invalid current price: {}",
                    position.symbol, current
                ),
            );
            return None;
        }
    }

    // Call the calculation function
    let result = calculate_position_pnl(position, current_price).await;

    // Return result wrapped in Some - caller can now distinguish None (error) from Some((0.0, 0.0)) (break-even)
    Some(result)
}

// ==================== SPLIT P&L CALCULATION (PARTIAL EXIT SUPPORT) ====================

/// Calculate split P&L for positions with partial exits
/// Returns (realized_pnl_sol, unrealized_pnl_sol, total_pnl_sol, total_pnl_percent)
pub async fn calculate_split_pnl(
    position: &Position,
    current_price: Option<f64>,
) -> (f64, f64, f64, f64) {
    let entry_price =
        if position.average_entry_price > 0.0 && position.average_entry_price.is_finite() {
            position.average_entry_price
        } else {
            position
                .effective_entry_price
                .unwrap_or(position.entry_price)
        };

    if entry_price <= 0.0 || !entry_price.is_finite() {
        return (0.0, 0.0, 0.0, 0.0);
    }

    // Calculate realized P&L from partial exits
    let realized_pnl_sol = if position.total_exited_amount > 0 {
        if let Some(avg_exit_price) = position.average_exit_price {
            let sol_received = position.sol_received.unwrap_or_default();
            let exit_portion =
                position.total_exited_amount as f64 / (position.token_amount.unwrap_or(1) as f64);
            let invested_in_exited = position.total_size_sol * exit_portion;
            let exit_fees = lamports_to_sol(position.exit_fee_lamports.unwrap_or_default());
            sol_received - invested_in_exited - exit_fees
        } else {
            0.0
        }
    } else {
        0.0
    };

    // Calculate unrealized P&L from remaining holdings
    let unrealized_pnl_sol = if let Some(remaining) = position.remaining_token_amount {
        if let Some(current) = current_price {
            if let Some(decimals) = get_decimals(&position.mint).await {
                let ui_remaining = (remaining as f64) / (10_f64).powi(decimals as i32);
                let current_value = ui_remaining * current;
                let remaining_portion =
                    remaining as f64 / (position.token_amount.unwrap_or(1) as f64);
                let invested_in_remaining = position.total_size_sol * remaining_portion;
                let entry_fees_portion =
                    lamports_to_sol(position.entry_fee_lamports.unwrap_or_default())
                        * remaining_portion;
                current_value - invested_in_remaining - entry_fees_portion
            } else {
                0.0
            }
        } else {
            0.0
        }
    } else if position.exit_time.is_none() {
        // No partial exits yet, full position unrealized
        if let Some(current) = current_price {
            let (total_pnl, _) = calculate_position_pnl(position, Some(current)).await;
            total_pnl
        } else {
            0.0
        }
    } else {
        0.0
    };

    let total_pnl_sol = realized_pnl_sol + unrealized_pnl_sol;
    let total_pnl_percent = (total_pnl_sol / position.total_size_sol) * 100.0;

    (
        realized_pnl_sol,
        unrealized_pnl_sol,
        total_pnl_sol,
        total_pnl_percent,
    )
}
