use crate::config::TransactionManagerConfig;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::{ Transaction, TransactionType };
use crate::wallet::WalletTracker;
use anyhow::{ Context, Result };
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Manages all trading transactions with caching and P&L tracking
pub struct TransactionManager {
    config: TransactionManagerConfig,
    database: Arc<Database>,
    wallet_tracker: Arc<WalletTracker>,
    cached_transactions: Arc<RwLock<HashMap<String, Transaction>>>,
    is_running: Arc<RwLock<bool>>,
}

impl TransactionManager {
    pub fn new(
        config: TransactionManagerConfig,
        database: Arc<Database>,
        wallet_tracker: Arc<WalletTracker>
    ) -> Self {
        Self {
            config,
            database,
            wallet_tracker,
            cached_transactions: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Transaction Manager started");

        if self.config.cache_transactions {
            self.load_cached_transactions().await?;
        }

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Logger::info("Transaction Manager stopped");
    }

    pub async fn record_transaction(
        &self,
        signature: String,
        transaction_type: TransactionType,
        token_mint: String,
        amount_sol: f64,
        amount_tokens: f64,
        price: f64,
        block_height: u64,
        fee_sol: f64,
        position_id: Option<String>
    ) -> Result<String> {
        let transaction_id = Uuid::new_v4().to_string();

        let transaction = Transaction {
            id: transaction_id.clone(),
            signature,
            transaction_type: transaction_type.clone(),
            token_mint: token_mint.clone(),
            amount_sol,
            amount_tokens,
            price,
            timestamp: Utc::now(),
            block_height,
            fee_sol,
            position_id,
        };

        // Save to database
        self.save_transaction(&transaction).await?;

        // Cache if enabled
        if self.config.cache_transactions {
            self.cached_transactions
                .write().await
                .insert(transaction_id.clone(), transaction.clone());
        }

        Logger::trader(
            &format!(
                "📝 Transaction recorded: {:?} | {} | {:.6} SOL | Fee: {:.6} SOL",
                transaction_type,
                token_mint,
                amount_sol,
                fee_sol
            )
        );

        Ok(transaction_id)
    }

    pub async fn get_transactions_for_token(&self, token_mint: &str) -> Result<Vec<Transaction>> {
        if self.config.cache_transactions {
            let cached = self.cached_transactions.read().await;
            Ok(
                cached
                    .values()
                    .filter(|tx| tx.token_mint == token_mint)
                    .cloned()
                    .collect()
            )
        } else {
            self.load_transactions_for_token(token_mint).await
        }
    }

    pub async fn get_transactions_for_position(
        &self,
        position_id: &str
    ) -> Result<Vec<Transaction>> {
        if self.config.cache_transactions {
            let cached = self.cached_transactions.read().await;
            Ok(
                cached
                    .values()
                    .filter(|tx| tx.position_id.as_ref() == Some(&position_id.to_string()))
                    .cloned()
                    .collect()
            )
        } else {
            self.load_transactions_for_position(position_id).await
        }
    }

    pub async fn calculate_position_pnl(&self, position_id: &str) -> Result<(f64, f64)> {
        let transactions = self.get_transactions_for_position(position_id).await?;

        let mut total_cost = 0.0;
        let mut total_proceeds = 0.0;
        let mut total_fees = 0.0;

        for tx in &transactions {
            total_fees += tx.fee_sol;

            match tx.transaction_type {
                TransactionType::Buy => {
                    total_cost += tx.amount_sol + tx.fee_sol;
                }
                TransactionType::Sell => {
                    total_proceeds += tx.amount_sol - tx.fee_sol;
                }
                TransactionType::Transfer => {
                    // Handle transfers if needed
                }
            }
        }

        let net_pnl = total_proceeds - total_cost;
        let pnl_percentage = if total_cost > 0.0 { (net_pnl / total_cost) * 100.0 } else { 0.0 };

        Ok((net_pnl, pnl_percentage))
    }

    pub async fn get_wallet_transaction_history(&self, hours: u64) -> Result<Vec<Transaction>> {
        let cutoff_time = Utc::now() - chrono::Duration::hours(hours as i64);

        if self.config.cache_transactions {
            let cached = self.cached_transactions.read().await;
            Ok(
                cached
                    .values()
                    .filter(|tx| tx.timestamp > cutoff_time)
                    .cloned()
                    .collect()
            )
        } else {
            self.load_recent_transactions(hours).await
        }
    }

    pub async fn cleanup_old_transactions(&self) -> Result<u64> {
        if !self.config.cache_transactions {
            return Ok(0);
        }

        let cutoff_time =
            Utc::now() - chrono::Duration::hours(self.config.cache_duration_hours as i64);
        let mut cached = self.cached_transactions.write().await;

        let initial_count = cached.len();
        cached.retain(|_, tx| tx.timestamp > cutoff_time);
        let removed_count = initial_count - cached.len();

        if removed_count > 0 {
            Logger::trader(&format!("Cleaned up {} old transactions from cache", removed_count));
        }

        Ok(removed_count as u64)
    }

    async fn load_cached_transactions(&self) -> Result<()> {
        Logger::trader("Loading transaction cache...");

        let recent_transactions = self.load_recent_transactions(
            self.config.cache_duration_hours
        ).await?;
        let mut cached = self.cached_transactions.write().await;

        for tx in recent_transactions {
            cached.insert(tx.id.clone(), tx);
        }

        Logger::trader(&format!("Loaded {} transactions into cache", cached.len()));
        Ok(())
    }

    async fn save_transaction(&self, transaction: &Transaction) -> Result<()> {
        // TODO: Implement database save for Transaction
        // This should save to a transactions table in the database
        Ok(())
    }

    async fn load_transactions_for_token(&self, token_mint: &str) -> Result<Vec<Transaction>> {
        // TODO: Implement database load for transactions by token
        Ok(Vec::new())
    }

    async fn load_transactions_for_position(&self, position_id: &str) -> Result<Vec<Transaction>> {
        // TODO: Implement database load for transactions by position
        Ok(Vec::new())
    }

    async fn load_recent_transactions(&self, hours: u64) -> Result<Vec<Transaction>> {
        // TODO: Implement database load for recent transactions
        Ok(Vec::new())
    }
}
