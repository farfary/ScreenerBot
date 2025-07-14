use crate::core::{
    BotResult,
    BotError,
    Position,
    TokenBalance,
    WalletTransaction,
    TransactionType,
};
use solana_sdk::pubkey::Pubkey;
use chrono::{ Utc, DateTime };
use std::collections::HashMap;

/// Position tracker for building and maintaining portfolio positions
#[derive(Debug)]
pub struct PositionTracker;

impl PositionTracker {
    pub fn new() -> Self {
        Self
    }

    /// Build positions from current balances and transaction history
    pub async fn build_positions_from_balances(
        &self,
        balances: &[TokenBalance],
        transactions: &[WalletTransaction]
    ) -> BotResult<Vec<Position>> {
        let mut positions = Vec::new();

        for balance in balances {
            // Skip SOL and empty balances
            if balance.mint.to_string() == crate::core::WSOL_MINT || balance.amount == 0 {
                continue;
            }

            // Get transactions for this token
            let token_transactions: Vec<&WalletTransaction> = transactions
                .iter()
                .filter(|tx| tx.tokens_involved.contains(&balance.mint))
                .collect();

            if token_transactions.is_empty() {
                // This might be an old position or airdrop
                let position = self.create_position_from_balance_only(balance).await?;
                positions.push(position);
                continue;
            }

            // Build position from transaction history
            let position = self.build_position_from_transactions(
                balance,
                &token_transactions
            ).await?;
            positions.push(position);
        }

        Ok(positions)
    }

    /// Create position when we only have balance data (no transaction history)
    async fn create_position_from_balance_only(
        &self,
        balance: &TokenBalance
    ) -> BotResult<Position> {
        // Estimate entry price (this is a fallback when we don't have transaction data)
        let estimated_price = balance.price_usd.unwrap_or(0.0);
        let current_value = balance.value_usd.unwrap_or(0.0) / 1_000_000_000.0; // Assume SOL price conversion

        Ok(Position {
            token: balance.mint,
            symbol: balance.symbol.clone().unwrap_or("UNKNOWN".to_string()),
            total_amount: balance.amount,
            average_entry_price: estimated_price,
            current_price: estimated_price,
            total_invested_sol: current_value, // Estimate
            current_value_sol: current_value,
            unrealized_pnl: 0.0, // Can't calculate without entry data
            unrealized_pnl_percentage: 0.0,
            first_buy_time: Utc::now(), // Unknown, use current time
            last_buy_time: Utc::now(),
            trade_count: 1, // Estimate
            dca_opportunities: 0,
        })
    }

    /// Build position from transaction history
    async fn build_position_from_transactions(
        &self,
        balance: &TokenBalance,
        transactions: &[&WalletTransaction]
    ) -> BotResult<Position> {
        let mut total_tokens_bought = 0u64;
        let mut total_sol_invested = 0.0;
        let mut buy_count = 0u32;
        let mut first_buy_time = None;
        let mut last_buy_time = None;

        // Process all buy transactions
        for tx in transactions {
            if matches!(tx.transaction_type, TransactionType::Buy | TransactionType::Swap) {
                // Check if this was a buy of our token
                if let Some(token_change) = tx.token_changes.get(&balance.mint) {
                    if *token_change > 0 {
                        // This was a buy
                        let tokens_acquired = *token_change as u64;
                        let sol_spent = (-tx.sol_change as f64) / 1_000_000_000.0; // Convert lamports to SOL

                        if sol_spent > 0.0 {
                            total_tokens_bought += tokens_acquired;
                            total_sol_invested += sol_spent;
                            buy_count += 1;

                            // Track timing
                            if let Some(block_time) = tx.block_time {
                                let tx_time = DateTime::from_timestamp(block_time, 0).unwrap_or(
                                    Utc::now()
                                );

                                if first_buy_time.is_none() || tx_time < first_buy_time.unwrap() {
                                    first_buy_time = Some(tx_time);
                                }

                                if last_buy_time.is_none() || tx_time > last_buy_time.unwrap() {
                                    last_buy_time = Some(tx_time);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate average entry price
        let average_entry_price = if total_tokens_bought > 0 {
            (total_sol_invested * 1_000_000_000.0) / (total_tokens_bought as f64) // SOL per token
        } else {
            0.0
        };

        // Current metrics
        let current_price = balance.price_usd.unwrap_or(0.0);
        let current_value_sol = balance.value_usd.unwrap_or(0.0) / 50.0; // Rough SOL conversion (assuming $50 SOL)

        // Calculate PnL
        let unrealized_pnl = current_value_sol - total_sol_invested;
        let unrealized_pnl_percentage = if total_sol_invested > 0.0 {
            (unrealized_pnl / total_sol_invested) * 100.0
        } else {
            0.0
        };

        // Estimate DCA opportunities (times price dropped significantly)
        let dca_opportunities = self.count_dca_opportunities(transactions, average_entry_price);

        Ok(Position {
            token: balance.mint,
            symbol: balance.symbol.clone().unwrap_or("UNKNOWN".to_string()),
            total_amount: balance.amount,
            average_entry_price,
            current_price,
            total_invested_sol: total_sol_invested,
            current_value_sol,
            unrealized_pnl,
            unrealized_pnl_percentage,
            first_buy_time: first_buy_time.unwrap_or(Utc::now()),
            last_buy_time: last_buy_time.unwrap_or(Utc::now()),
            trade_count: buy_count,
            dca_opportunities,
        })
    }

    /// Count potential DCA opportunities from price movements
    fn count_dca_opportunities(
        &self,
        _transactions: &[&WalletTransaction],
        _avg_price: f64
    ) -> u32 {
        // This would analyze price movements to identify DCA opportunities
        // For now, return 0 as we'd need price history data
        0
    }

    /// Update an existing position with new transaction
    pub fn update_position_with_transaction(
        &self,
        position: &mut Position,
        transaction: &WalletTransaction
    ) -> BotResult<()> {
        if let Some(token_change) = transaction.token_changes.get(&position.token) {
            if
                *token_change > 0 &&
                matches!(transaction.transaction_type, TransactionType::Buy | TransactionType::Swap)
            {
                // This is a buy/DCA
                let tokens_acquired = *token_change as u64;
                let sol_spent = (-transaction.sol_change as f64) / 1_000_000_000.0;

                if sol_spent > 0.0 {
                    // Update average price
                    let old_total_value = position.total_invested_sol;
                    let old_total_tokens = position.total_amount;

                    position.total_amount += tokens_acquired;
                    position.total_invested_sol += sol_spent;
                    position.trade_count += 1;

                    // Recalculate average entry price
                    position.average_entry_price =
                        (old_total_value + sol_spent) /
                        ((old_total_tokens + tokens_acquired) as f64);

                    // Update timing
                    if let Some(block_time) = transaction.block_time {
                        if
                            let Ok(tx_time) = DateTime::from_timestamp(block_time, 0).ok_or(
                                Utc::now()
                            )
                        {
                            position.last_buy_time = tx_time;
                        }
                    }
                }
            } else if
                *token_change < 0 &&
                matches!(transaction.transaction_type, TransactionType::Sell)
            {
                // This is a sell - reduce position
                let tokens_sold = -*token_change as u64;

                if tokens_sold >= position.total_amount {
                    // Position completely closed
                    position.total_amount = 0;
                } else {
                    // Partial sell
                    let sell_ratio = (tokens_sold as f64) / (position.total_amount as f64);
                    position.total_amount -= tokens_sold;
                    position.total_invested_sol *= 1.0 - sell_ratio;
                }
            }
        }

        Ok(())
    }

    /// Calculate position metrics after price update
    pub fn update_position_metrics(&self, position: &mut Position, current_price: f64) {
        position.current_price = current_price;

        // Calculate current value in SOL (this would need actual price conversion)
        position.current_value_sol = ((position.total_amount as f64) * current_price) / 50.0; // Rough SOL conversion

        // Update PnL
        position.unrealized_pnl = position.current_value_sol - position.total_invested_sol;

        if position.total_invested_sol > 0.0 {
            position.unrealized_pnl_percentage =
                (position.unrealized_pnl / position.total_invested_sol) * 100.0;
        } else {
            position.unrealized_pnl_percentage = 0.0;
        }
    }

    /// Check if position qualifies for DCA
    pub fn should_dca(&self, position: &Position, current_price: f64, dca_threshold: f64) -> bool {
        if position.average_entry_price == 0.0 {
            return false;
        }

        let price_drop_percentage =
            ((position.average_entry_price - current_price) / position.average_entry_price) * 100.0;
        price_drop_percentage >= dca_threshold
    }

    /// Merge positions if we have multiple entries for the same token
    pub fn merge_positions(&self, positions: Vec<Position>) -> Vec<Position> {
        let mut merged_map: HashMap<Pubkey, Position> = HashMap::new();

        for position in positions {
            if let Some(existing) = merged_map.get_mut(&position.token) {
                // Merge with existing position
                let total_invested = existing.total_invested_sol + position.total_invested_sol;
                let total_amount = existing.total_amount + position.total_amount;

                // Weighted average entry price
                existing.average_entry_price = total_invested / (total_amount as f64);
                existing.total_amount = total_amount;
                existing.total_invested_sol = total_invested;
                existing.trade_count += position.trade_count;

                // Keep earliest first buy time
                if position.first_buy_time < existing.first_buy_time {
                    existing.first_buy_time = position.first_buy_time;
                }

                // Keep latest buy time
                if position.last_buy_time > existing.last_buy_time {
                    existing.last_buy_time = position.last_buy_time;
                }

                // Recalculate metrics
                existing.current_value_sol =
                    existing.current_value_sol + position.current_value_sol;
                existing.unrealized_pnl = existing.current_value_sol - existing.total_invested_sol;

                if existing.total_invested_sol > 0.0 {
                    existing.unrealized_pnl_percentage =
                        (existing.unrealized_pnl / existing.total_invested_sol) * 100.0;
                }
            } else {
                merged_map.insert(position.token, position);
            }
        }

        merged_map.into_values().collect()
    }
}
