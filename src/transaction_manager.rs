#![allow(warnings)]
use crate::prelude::*;

use std::collections::HashMap;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use tokio::{ fs, time::{ sleep, Duration } };
use anyhow::Result;
use std::sync::Arc;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Signature;
use std::str::FromStr;

pub const PENDING_TRANSACTIONS_FILE: &str = "pending_transactions.json";
pub const TRANSACTION_TIMEOUT_SECONDS: i64 = 300; // 5 minutes timeout
pub const CONFIRMATION_CHECK_INTERVAL: u64 = 3; // Check every 3 seconds

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    Buy,
    Sell,
    DCA,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending, // Transaction submitted, waiting for confirmation
    Confirmed, // Transaction confirmed on-chain
    Failed, // Transaction failed
    Timeout, // Transaction timed out waiting for confirmation
    Cancelled, // Transaction was cancelled
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingTransaction {
    pub signature: String,
    pub transaction_type: TransactionType,
    pub token_mint: String,
    pub token_symbol: String,
    pub amount_sol: f64,
    pub token_amount: f64,
    pub price: f64,
    pub status: TransactionStatus,
    pub submitted_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub retry_count: u8,
    pub error_message: Option<String>,
    pub position_data: Option<serde_json::Value>, // Store position data for recovery
}

impl PendingTransaction {
    pub fn new(
        signature: String,
        transaction_type: TransactionType,
        token_mint: String,
        token_symbol: String,
        amount_sol: f64,
        token_amount: f64,
        price: f64,
        position_data: Option<serde_json::Value>
    ) -> Self {
        Self {
            signature,
            transaction_type,
            token_mint,
            token_symbol,
            amount_sol,
            token_amount,
            price,
            status: TransactionStatus::Pending,
            submitted_at: Utc::now(),
            confirmed_at: None,
            retry_count: 0,
            error_message: None,
            position_data,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = Utc::now();
        let elapsed = now - self.submitted_at;
        elapsed.num_seconds() > TRANSACTION_TIMEOUT_SECONDS
    }

    pub fn should_retry(&self) -> bool {
        self.retry_count < 3 && !self.is_expired()
    }
}

// Global pending transactions store
pub static PENDING_TRANSACTIONS: Lazy<RwLock<HashMap<String, PendingTransaction>>> = Lazy::new(||
    RwLock::new(HashMap::new())
);

// Transaction blocking flags
pub static TRANSACTION_PROCESSING: Lazy<RwLock<bool>> = Lazy::new(|| RwLock::new(false));

/// Transaction Manager - handles all transaction state management
pub struct TransactionManager {
    rpc_client: Arc<RpcClient>,
}

impl TransactionManager {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_client: Arc::new(RpcClient::new(rpc_url)),
        }
    }

    /// Load pending transactions from disk at startup
    pub async fn load_pending_transactions() -> Result<()> {
        if let Ok(data) = fs::read(PENDING_TRANSACTIONS_FILE).await {
            let transactions: HashMap<String, PendingTransaction> = serde_json::from_slice(&data)?;
            *PENDING_TRANSACTIONS.write().await = transactions;

            let count = PENDING_TRANSACTIONS.read().await.len();
            if count > 0 {
                println!("📋 Loaded {} pending transactions from disk", count);
            }
        }
        Ok(())
    }

    /// Save pending transactions to disk
    pub async fn save_pending_transactions() -> Result<()> {
        let snapshot = PENDING_TRANSACTIONS.read().await.clone();
        let data = serde_json::to_vec_pretty(&snapshot)?;
        crate::persistence::atomic_write(PENDING_TRANSACTIONS_FILE, &data).await?;
        Ok(())
    }

    /// Add a new pending transaction
    pub async fn add_pending_transaction(transaction: PendingTransaction) -> Result<()> {
        let signature = transaction.signature.clone();

        println!(
            "📝 Adding pending {} transaction: {} for {} ({})",
            match transaction.transaction_type {
                TransactionType::Buy => "BUY",
                TransactionType::Sell => "SELL",
                TransactionType::DCA => "DCA",
            },
            signature,
            transaction.token_symbol,
            transaction.token_mint
        );

        PENDING_TRANSACTIONS.write().await.insert(signature.clone(), transaction);
        Self::save_pending_transactions().await?;

        // Set processing flag
        *TRANSACTION_PROCESSING.write().await = true;

        Ok(())
    }

    /// Remove a transaction from pending list
    pub async fn remove_pending_transaction(signature: &str) -> Result<()> {
        if let Some(tx) = PENDING_TRANSACTIONS.write().await.remove(signature) {
            println!(
                "🗑️ Removed {} transaction from pending: {} ({})",
                match tx.transaction_type {
                    TransactionType::Buy => "BUY",
                    TransactionType::Sell => "SELL",
                    TransactionType::DCA => "DCA",
                },
                signature,
                tx.status as u8
            );
        }

        Self::save_pending_transactions().await?;

        // Clear processing flag if no more pending transactions
        if PENDING_TRANSACTIONS.read().await.is_empty() {
            *TRANSACTION_PROCESSING.write().await = false;
        }

        Ok(())
    }

    /// Check if trading is blocked due to pending transactions
    pub async fn is_trading_blocked() -> bool {
        let processing = *TRANSACTION_PROCESSING.read().await;
        let has_pending = !PENDING_TRANSACTIONS.read().await.is_empty();

        if processing || has_pending {
            let pending_count = PENDING_TRANSACTIONS.read().await.len();
            if pending_count > 0 {
                println!("⏳ Trading blocked - {} pending transactions", pending_count);
            }
            return true;
        }

        false
    }

    /// Get pending transaction for a specific token
    pub async fn get_pending_transaction_for_token(token_mint: &str) -> Option<PendingTransaction> {
        PENDING_TRANSACTIONS.read().await
            .values()
            .find(|tx| tx.token_mint == token_mint && tx.status == TransactionStatus::Pending)
            .cloned()
    }

    /// Check if a specific token has pending transactions
    pub async fn has_pending_transaction_for_token(token_mint: &str) -> bool {
        PENDING_TRANSACTIONS.read().await
            .values()
            .any(|tx| tx.token_mint == token_mint && tx.status == TransactionStatus::Pending)
    }

    /// Get all pending transactions
    pub async fn get_all_pending_transactions() -> Vec<PendingTransaction> {
        PENDING_TRANSACTIONS.read().await.values().cloned().collect()
    }

    /// Check transaction status on-chain
    pub async fn check_transaction_status(&self, signature: &str) -> Result<TransactionStatus> {
        match Signature::from_str(signature) {
            Ok(sig) => {
                match self.rpc_client.get_signature_status(&sig) {
                    Ok(status_result) => {
                        match status_result {
                            Some(result) => {
                                match result {
                                    Ok(_) => Ok(TransactionStatus::Confirmed),
                                    Err(_) => Ok(TransactionStatus::Failed),
                                }
                            }
                            None => Ok(TransactionStatus::Pending),
                        }
                    }
                    Err(e) => {
                        println!("❌ Error checking transaction status for {}: {}", signature, e);
                        Ok(TransactionStatus::Pending) // Assume pending on RPC error
                    }
                }
            }
            Err(e) => {
                println!("❌ Invalid signature format {}: {}", signature, e);
                Ok(TransactionStatus::Failed)
            }
        }
    }

    /// Update transaction status
    pub async fn update_transaction_status(
        &self,
        signature: &str,
        new_status: TransactionStatus,
        error_message: Option<String>
    ) -> Result<()> {
        if let Some(mut tx) = PENDING_TRANSACTIONS.write().await.get_mut(signature) {
            tx.status = new_status.clone();
            tx.error_message = error_message;

            if new_status == TransactionStatus::Confirmed {
                tx.confirmed_at = Some(Utc::now());
            }

            println!(
                "📊 Updated {} transaction status: {} -> {:?}",
                match tx.transaction_type {
                    TransactionType::Buy => "BUY",
                    TransactionType::Sell => "SELL",
                    TransactionType::DCA => "DCA",
                },
                signature,
                new_status
            );
        }

        Self::save_pending_transactions().await?;
        Ok(())
    }

    /// Process all pending transactions - check their status and handle accordingly
    pub async fn process_pending_transactions(&self) -> Result<()> {
        let mut transactions_to_remove = Vec::new();
        let mut transactions_to_update = Vec::new();

        // Get snapshot of pending transactions
        let pending_txs = {
            PENDING_TRANSACTIONS.read().await
                .values()
                .filter(|tx| tx.status == TransactionStatus::Pending)
                .cloned()
                .collect::<Vec<_>>()
        };

        if pending_txs.is_empty() {
            return Ok(());
        }

        println!("🔍 Checking {} pending transactions...", pending_txs.len());

        for tx in pending_txs {
            // Check if transaction has expired
            if tx.is_expired() {
                println!(
                    "⏰ Transaction {} expired after {} seconds",
                    tx.signature,
                    TRANSACTION_TIMEOUT_SECONDS
                );
                transactions_to_update.push((
                    tx.signature.clone(),
                    TransactionStatus::Timeout,
                    Some("Transaction timeout".to_string()),
                ));
                transactions_to_remove.push(tx.signature.clone());
                continue;
            }

            // Check on-chain status
            match self.check_transaction_status(&tx.signature).await {
                Ok(status) => {
                    match status {
                        TransactionStatus::Confirmed => {
                            println!("✅ Transaction {} confirmed on-chain", tx.signature);
                            transactions_to_update.push((tx.signature.clone(), status, None));
                            transactions_to_remove.push(tx.signature.clone());

                            // Handle post-confirmation logic
                            self.handle_confirmed_transaction(&tx).await?;
                        }
                        TransactionStatus::Failed => {
                            println!("❌ Transaction {} failed on-chain", tx.signature);
                            transactions_to_update.push((
                                tx.signature.clone(),
                                status,
                                Some("Transaction failed on-chain".to_string()),
                            ));
                            transactions_to_remove.push(tx.signature.clone());

                            // Handle failed transaction
                            self.handle_failed_transaction(&tx).await?;
                        }
                        TransactionStatus::Pending => {
                            // Still pending, continue monitoring
                            println!("⏳ Transaction {} still pending...", tx.signature);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    println!("❌ Error checking status for {}: {}", tx.signature, e);
                    // Continue monitoring on error
                }
            }
        }

        // Update transaction statuses
        for (signature, status, error) in transactions_to_update {
            self.update_transaction_status(&signature, status, error).await?;
        }

        // Remove completed transactions
        for signature in transactions_to_remove {
            Self::remove_pending_transaction(&signature).await?;
        }

        Ok(())
    }

    /// Handle confirmed transaction - update positions, etc.
    async fn handle_confirmed_transaction(&self, tx: &PendingTransaction) -> Result<()> {
        match tx.transaction_type {
            TransactionType::Buy => {
                println!(
                    "🎯 Buy transaction confirmed for {} - position should be opened",
                    tx.token_symbol
                );
                // Buy transaction confirmed - position should already be in OPEN_POSITIONS
                // Just log for now, position was added when transaction was submitted
            }
            TransactionType::Sell => {
                println!(
                    "🎯 Sell transaction confirmed for {} - position should be closed",
                    tx.token_symbol
                );
                // Sell transaction confirmed - position should already be removed from OPEN_POSITIONS
                // Just log for now, position was removed when transaction was submitted
            }
            TransactionType::DCA => {
                println!(
                    "🎯 DCA transaction confirmed for {} - position should be updated",
                    tx.token_symbol
                );
                // DCA transaction confirmed - position should already be updated
                // Just log for now, position was updated when transaction was submitted
            }
        }
        Ok(())
    }

    /// Handle failed transaction - revert position changes if needed
    async fn handle_failed_transaction(&self, tx: &PendingTransaction) -> Result<()> {
        match tx.transaction_type {
            TransactionType::Buy => {
                println!("❌ Buy transaction failed for {} - removing position", tx.token_symbol);
                // Remove the position that was added when transaction was submitted
                crate::persistence::OPEN_POSITIONS.write().await.remove(&tx.token_mint);
                crate::persistence::save_open().await;
            }
            TransactionType::Sell => {
                println!("❌ Sell transaction failed for {} - restoring position", tx.token_symbol);
                // Restore the position that was removed when transaction was submitted
                if let Some(position_data) = &tx.position_data {
                    if
                        let Ok(position) = serde_json::from_value::<crate::persistence::Position>(
                            position_data.clone()
                        )
                    {
                        crate::persistence::OPEN_POSITIONS
                            .write().await
                            .insert(tx.token_mint.clone(), position);
                        crate::persistence::save_open().await;
                    }
                }
            }
            TransactionType::DCA => {
                println!("❌ DCA transaction failed for {} - reverting position", tx.token_symbol);
                // Revert the position changes that were made when transaction was submitted
                if let Some(position_data) = &tx.position_data {
                    if
                        let Ok(position) = serde_json::from_value::<crate::persistence::Position>(
                            position_data.clone()
                        )
                    {
                        crate::persistence::OPEN_POSITIONS
                            .write().await
                            .insert(tx.token_mint.clone(), position);
                        crate::persistence::save_open().await;
                    }
                }
            }
        }
        Ok(())
    }

    /// Start background monitoring of pending transactions
    pub async fn start_monitoring(&self) -> Result<()> {
        println!("🔄 Starting transaction monitoring service...");

        loop {
            if let Err(e) = self.process_pending_transactions().await {
                println!("❌ Error processing pending transactions: {}", e);
            }

            // Wait before next check
            sleep(Duration::from_secs(CONFIRMATION_CHECK_INTERVAL)).await;
        }
    }

    /// Clean up old completed transactions from memory and disk
    pub async fn cleanup_old_transactions() -> Result<()> {
        let mut removed_count = 0;
        let cutoff_time = Utc::now() - chrono::Duration::hours(24); // Remove transactions older than 24 hours

        {
            let mut pending = PENDING_TRANSACTIONS.write().await;
            let initial_count = pending.len();

            pending.retain(|_, tx| {
                if tx.status != TransactionStatus::Pending && tx.submitted_at < cutoff_time {
                    removed_count += 1;
                    false
                } else {
                    true
                }
            });

            if removed_count > 0 {
                println!("🧹 Cleaned up {} old transactions", removed_count);
            }
        }

        if removed_count > 0 {
            Self::save_pending_transactions().await?;
        }

        Ok(())
    }

    /// Get transaction summary for logging
    pub async fn get_transaction_summary() -> String {
        let pending = PENDING_TRANSACTIONS.read().await;
        let total = pending.len();
        let confirmed = pending
            .values()
            .filter(|tx| tx.status == TransactionStatus::Confirmed)
            .count();
        let failed = pending
            .values()
            .filter(|tx| tx.status == TransactionStatus::Failed)
            .count();
        let timeout = pending
            .values()
            .filter(|tx| tx.status == TransactionStatus::Timeout)
            .count();
        let pending_count = pending
            .values()
            .filter(|tx| tx.status == TransactionStatus::Pending)
            .count();

        format!(
            "📊 Transactions: {} total (⏳ {} pending, ✅ {} confirmed, ❌ {} failed, ⏰ {} timeout)",
            total,
            pending_count,
            confirmed,
            failed,
            timeout
        )
    }
}

/// Helper function to create position data for transaction recovery
pub fn create_position_data(position: &crate::persistence::Position) -> serde_json::Value {
    serde_json::to_value(position).unwrap_or(serde_json::Value::Null)
}

/// Global transaction manager instance
pub static TRANSACTION_MANAGER: Lazy<Option<TransactionManager>> = Lazy::new(|| None);

/// Initialize transaction manager
pub fn init_transaction_manager(rpc_url: String) -> TransactionManager {
    TransactionManager::new(rpc_url)
}
