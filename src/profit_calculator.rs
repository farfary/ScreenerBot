use crate::database::Database;
use crate::logger::Logger;
use crate::types::{ WalletPosition, ProfitLossCalculation, TransactionType };
use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ProfitLossCalculator {
    database: Arc<Database>,
}

impl ProfitLossCalculator {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Calculate comprehensive profit/loss for a specific token
    pub async fn calculate_token_pnl(
        &self,
        mint: &str,
        current_price_sol: f64
    ) -> Result<ProfitLossCalculation> {
        Logger::wallet(&format!("📊 Calculating P&L for token: {mint}"));

        // Get all transactions for this token, ordered by block_time
        let transactions = self.database.get_wallet_transactions_for_mint(mint)?;

        if transactions.is_empty() {
            return Ok(ProfitLossCalculation {
                mint: mint.to_string(),
                total_bought: 0,
                total_sold: 0,
                current_balance: 0,
                average_buy_price_sol: 0.0,
                average_sell_price_sol: 0.0,
                total_invested_sol: 0.0,
                total_received_sol: 0.0,
                realized_pnl_sol: 0.0,
                unrealized_pnl_sol: 0.0,
                total_pnl_sol: 0.0,
                roi_percentage: 0.0,
                current_value_sol: 0.0,
            });
        }

        let mut total_bought = 0u64;
        let mut total_sold = 0u64;
        let mut total_invested_sol = 0.0;
        let mut total_received_sol = 0.0;
        let mut weighted_buy_price_sum = 0.0;
        let mut weighted_sell_price_sum = 0.0;

        // Process all transactions
        for tx in &transactions {
            match tx.transaction_type {
                TransactionType::Buy => {
                    total_bought += tx.amount;
                    if let Some(value) = tx.value_sol {
                        total_invested_sol += value;
                    }
                    if let Some(price) = tx.price_sol {
                        weighted_buy_price_sum += price * (tx.amount as f64);
                    }
                }
                TransactionType::Sell => {
                    total_sold += tx.amount;
                    if let Some(value) = tx.value_sol {
                        total_received_sol += value;
                    }
                    if let Some(price) = tx.price_sol {
                        weighted_sell_price_sum += price * (tx.amount as f64);
                    }
                }
                TransactionType::Receive => {
                    // Treat receives as buys with zero cost basis
                    total_bought += tx.amount;
                    // Don't add to invested amount since it was received for free
                }
                TransactionType::Transfer => {
                    // Treat transfers out as sells with zero proceeds
                    total_sold += tx.amount;
                    // Don't add to received amount since no SOL was received
                }
            }
        }

        // Calculate averages
        let average_buy_price_sol = if total_bought > 0 && weighted_buy_price_sum > 0.0 {
            weighted_buy_price_sum / (total_bought as f64)
        } else {
            0.0
        };

        let average_sell_price_sol = if total_sold > 0 && weighted_sell_price_sum > 0.0 {
            weighted_sell_price_sum / (total_sold as f64)
        } else {
            0.0
        };

        // Calculate current balance
        let current_balance = total_bought.saturating_sub(total_sold);

        // Calculate realized P&L (from completed trades)
        let realized_pnl_sol =
            total_received_sol -
            (total_invested_sol * (total_sold as f64)) / (total_bought as f64).max(1.0);

        // Calculate unrealized P&L (current holdings at current price)
        let current_value_sol = (current_balance as f64) * current_price_sol;
        let unrealized_cost_basis = if total_bought > 0 {
            (total_invested_sol * (current_balance as f64)) / (total_bought as f64)
        } else {
            0.0
        };
        let unrealized_pnl_sol = current_value_sol - unrealized_cost_basis;

        // Total P&L
        let total_pnl_sol = realized_pnl_sol + unrealized_pnl_sol;

        // ROI calculation
        let roi_percentage = if total_invested_sol > 0.0 {
            (total_pnl_sol / total_invested_sol) * 100.0
        } else {
            0.0
        };

        let result = ProfitLossCalculation {
            mint: mint.to_string(),
            total_bought,
            total_sold,
            current_balance,
            average_buy_price_sol,
            average_sell_price_sol,
            total_invested_sol,
            total_received_sol,
            realized_pnl_sol,
            unrealized_pnl_sol,
            total_pnl_sol,
            roi_percentage,
            current_value_sol,
        };

        Logger::success(&format!("📊 P&L calculation completed for {mint}"));
        Logger::print_key_value("Total Bought", &format!("{} tokens", total_bought));
        Logger::print_key_value("Total Sold", &format!("{} tokens", total_sold));
        Logger::print_key_value("Current Balance", &format!("{} tokens", current_balance));
        Logger::print_key_value("Average Buy Price", &format!("{:.8} SOL", average_buy_price_sol));
        Logger::print_key_value("Total Invested", &format!("{:.6} SOL", total_invested_sol));
        Logger::print_key_value("Realized P&L", &format!("{:.6} SOL", realized_pnl_sol));
        Logger::print_key_value("Unrealized P&L", &format!("{:.6} SOL", unrealized_pnl_sol));
        Logger::print_key_value(
            "Total P&L",
            &format!("{:.6} SOL ({:.2}%)", total_pnl_sol, roi_percentage)
        );

        Ok(result)
    }

    /// Calculate P&L for all tokens in the wallet
    pub async fn calculate_portfolio_pnl(
        &self,
        current_prices: &HashMap<String, f64>
    ) -> Result<HashMap<String, ProfitLossCalculation>> {
        Logger::wallet("📊 Calculating portfolio-wide P&L...");

        let mut portfolio_pnl = HashMap::new();

        // Get all unique mints from transactions
        let mints = self.database.get_all_transaction_mints()?;

        Logger::wallet(&format!("📊 Found {} unique tokens in transaction history", mints.len()));

        for mint in mints {
            let current_price = current_prices.get(&mint).copied().unwrap_or(0.0);

            match self.calculate_token_pnl(&mint, current_price).await {
                Ok(pnl) => {
                    portfolio_pnl.insert(mint.clone(), pnl);
                }
                Err(e) => {
                    Logger::error(&format!("Failed to calculate P&L for {}: {}", mint, e));
                }
            }
        }

        // Calculate portfolio totals
        let total_invested: f64 = portfolio_pnl
            .values()
            .map(|p| p.total_invested_sol)
            .sum();
        let total_current_value: f64 = portfolio_pnl
            .values()
            .map(|p| p.current_value_sol)
            .sum();
        let total_realized_pnl: f64 = portfolio_pnl
            .values()
            .map(|p| p.realized_pnl_sol)
            .sum();
        let total_unrealized_pnl: f64 = portfolio_pnl
            .values()
            .map(|p| p.unrealized_pnl_sol)
            .sum();
        let total_pnl = total_realized_pnl + total_unrealized_pnl;
        let portfolio_roi = if total_invested > 0.0 {
            (total_pnl / total_invested) * 100.0
        } else {
            0.0
        };

        Logger::separator();
        Logger::wallet("📊 PORTFOLIO SUMMARY:");
        Logger::print_key_value("Total Invested", &format!("{:.6} SOL", total_invested));
        Logger::print_key_value("Current Value", &format!("{:.6} SOL", total_current_value));
        Logger::print_key_value("Realized P&L", &format!("{:.6} SOL", total_realized_pnl));
        Logger::print_key_value("Unrealized P&L", &format!("{:.6} SOL", total_unrealized_pnl));
        Logger::print_key_value(
            "Total P&L",
            &format!("{:.6} SOL ({:.2}%)", total_pnl, portfolio_roi)
        );
        Logger::separator();

        Ok(portfolio_pnl)
    }

    /// Update wallet position with calculated P&L
    pub async fn update_position_with_pnl(
        &self,
        mint: &str,
        balance: u64,
        decimals: u8,
        current_price_sol: f64
    ) -> Result<WalletPosition> {
        let pnl = self.calculate_token_pnl(mint, current_price_sol).await?;

        let position = WalletPosition {
            mint: mint.to_string(),
            name: None, // Will be set by position manager
            symbol: None, // Will be set by position manager
            balance,
            decimals,
            value_sol: Some(pnl.current_value_sol),
            entry_price_sol: Some(pnl.average_buy_price_sol),
            current_price_sol: Some(current_price_sol),
            pnl_sol: Some(pnl.total_pnl_sol),
            pnl_percentage: Some(pnl.roi_percentage),
            realized_pnl_sol: Some(pnl.realized_pnl_sol),
            unrealized_pnl_sol: Some(pnl.unrealized_pnl_sol),
            total_invested_sol: Some(pnl.total_invested_sol),
            average_entry_price_sol: Some(pnl.average_buy_price_sol),
            last_updated: Utc::now(),
        };

        Ok(position)
    }

    /// Get detailed transaction history for a token with running P&L
    pub async fn get_token_transaction_history_with_pnl(
        &self,
        mint: &str
    ) -> Result<Vec<TransactionWithRunningPnL>> {
        let transactions = self.database.get_wallet_transactions_for_mint(mint)?;
        let mut history = Vec::new();
        let mut running_balance = 0i64;
        let mut running_invested = 0.0;
        let mut total_bought = 0u64;

        for tx in transactions {
            match tx.transaction_type {
                TransactionType::Buy | TransactionType::Receive => {
                    running_balance += tx.amount as i64;
                    total_bought += tx.amount;
                    if let Some(value) = tx.value_sol {
                        running_invested += value;
                    }
                }
                TransactionType::Sell | TransactionType::Transfer => {
                    running_balance -= tx.amount as i64;
                }
            }

            let avg_entry_price = if total_bought > 0 {
                running_invested / (total_bought as f64)
            } else {
                0.0
            };

            let current_value = if let Some(price) = tx.price_sol {
                (running_balance as f64) * price
            } else {
                0.0
            };

            let unrealized_pnl = current_value - running_invested;

            let tx_with_pnl = TransactionWithRunningPnL {
                transaction: tx,
                running_balance: running_balance.max(0) as u64,
                running_invested_sol: running_invested,
                average_entry_price_sol: avg_entry_price,
                unrealized_pnl_sol: unrealized_pnl,
            };

            history.push(tx_with_pnl);
        }

        Ok(history)
    }
}

#[derive(Debug, Clone)]
pub struct TransactionWithRunningPnL {
    pub transaction: crate::types::WalletTransaction,
    pub running_balance: u64,
    pub running_invested_sol: f64,
    pub average_entry_price_sol: f64,
    pub unrealized_pnl_sol: f64,
}

impl ProfitLossCalculator {
    /// Calculate FIFO (First In, First Out) cost basis for more accurate P&L
    pub async fn calculate_fifo_pnl(
        &self,
        mint: &str,
        current_price_sol: f64
    ) -> Result<FifoPnLCalculation> {
        let transactions = self.database.get_wallet_transactions_for_mint(mint)?;

        let mut buy_queue: Vec<BuyTransaction> = Vec::new();
        let mut total_realized_pnl = 0.0;
        let mut total_sold = 0u64;
        let mut total_bought = 0u64;

        for tx in transactions {
            match tx.transaction_type {
                TransactionType::Buy | TransactionType::Receive => {
                    total_bought += tx.amount;
                    let cost_per_token = tx.value_sol.unwrap_or(0.0) / (tx.amount as f64);
                    buy_queue.push(BuyTransaction {
                        amount: tx.amount,
                        cost_per_token,
                        timestamp: tx.block_time,
                    });
                }
                TransactionType::Sell | TransactionType::Transfer => {
                    total_sold += tx.amount;
                    let mut remaining_to_sell = tx.amount;
                    let sale_price_per_token = tx.price_sol.unwrap_or(0.0);

                    // Match against earliest buys (FIFO)
                    while remaining_to_sell > 0 && !buy_queue.is_empty() {
                        let mut buy = buy_queue.remove(0);
                        let amount_to_match = remaining_to_sell.min(buy.amount);

                        // Calculate realized P&L for this portion
                        let cost_basis = (amount_to_match as f64) * buy.cost_per_token;
                        let proceeds = (amount_to_match as f64) * sale_price_per_token;
                        total_realized_pnl += proceeds - cost_basis;

                        remaining_to_sell -= amount_to_match;
                        buy.amount -= amount_to_match;

                        // If buy transaction is partially used, put it back
                        if buy.amount > 0 {
                            buy_queue.insert(0, buy);
                        }
                    }
                }
            }
        }

        // Calculate unrealized P&L from remaining holdings
        let mut unrealized_pnl = 0.0;
        let mut remaining_balance = 0u64;
        let mut weighted_avg_cost = 0.0;
        let mut total_cost_basis = 0.0;

        for buy in &buy_queue {
            remaining_balance += buy.amount;
            let cost_basis = (buy.amount as f64) * buy.cost_per_token;
            total_cost_basis += cost_basis;
        }

        if remaining_balance > 0 {
            weighted_avg_cost = total_cost_basis / (remaining_balance as f64);
            let current_value = (remaining_balance as f64) * current_price_sol;
            unrealized_pnl = current_value - total_cost_basis;
        }

        Ok(FifoPnLCalculation {
            mint: mint.to_string(),
            total_bought,
            total_sold,
            remaining_balance,
            weighted_average_cost: weighted_avg_cost,
            total_cost_basis,
            realized_pnl_sol: total_realized_pnl,
            unrealized_pnl_sol: unrealized_pnl,
            total_pnl_sol: total_realized_pnl + unrealized_pnl,
            current_price_sol,
            current_value_sol: (remaining_balance as f64) * current_price_sol,
        })
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct BuyTransaction {
    amount: u64,
    cost_per_token: f64,
    timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct FifoPnLCalculation {
    pub mint: String,
    pub total_bought: u64,
    pub total_sold: u64,
    pub remaining_balance: u64,
    pub weighted_average_cost: f64,
    pub total_cost_basis: f64,
    pub realized_pnl_sol: f64,
    pub unrealized_pnl_sol: f64,
    pub total_pnl_sol: f64,
    pub current_price_sol: f64,
    pub current_value_sol: f64,
}
