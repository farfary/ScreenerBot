use crate::database::Database;
use crate::logger::Logger;
use crate::rpc::RpcManager;
use crate::types::{ WalletTransaction, TransactionType };
use anyhow::{ Result, Context };
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct TransactionCacheManager {
    database: Arc<Database>,
    rpc_manager: Arc<RpcManager>,
    wallet_pubkey: Pubkey,
    max_cache_size: usize,
    is_running: Arc<RwLock<bool>>,
    last_processed_signature: Arc<RwLock<Option<String>>>,
}

impl TransactionCacheManager {
    pub fn new(
        database: Arc<Database>,
        rpc_manager: Arc<RpcManager>,
        wallet_pubkey: Pubkey,
        max_cache_size: Option<usize>
    ) -> Self {
        Self {
            database,
            rpc_manager,
            wallet_pubkey,
            max_cache_size: max_cache_size.unwrap_or(1000),
            is_running: Arc::new(RwLock::new(false)),
            last_processed_signature: Arc::new(RwLock::new(None)),
        }
    }

    /// Start background transaction caching task
    pub async fn start_background_caching(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            Logger::warn("Transaction cache manager is already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::wallet("🚀 Starting background transaction caching...");

        // Initial cache setup
        let initial_count = self.cache_historical_transactions().await?;
        Logger::success(
            &format!("✅ Initial cache setup completed with {} transactions", initial_count)
        );

        // Start background update task
        let cache_manager = self.clone();
        tokio::spawn(async move {
            cache_manager.run_background_update_loop().await;
        });

        Ok(())
    }

    /// Stop background caching
    pub async fn stop_background_caching(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Logger::wallet("🛑 Stopping background transaction caching");
    }

    /// Main background update loop
    async fn run_background_update_loop(&self) {
        Logger::wallet("🔄 Starting background transaction update loop...");
        let mut interval = tokio::time::interval(Duration::from_secs(30)); // Check every 30 seconds

        loop {
            interval.tick().await;

            let is_running = self.is_running.read().await;
            if !*is_running {
                Logger::wallet("🛑 Background transaction caching loop stopping");
                break;
            }
            drop(is_running);

            // Update cache with new transactions
            match self.update_cache_with_new_transactions().await {
                Ok(new_count) => {
                    if new_count > 0 {
                        Logger::success(
                            &format!("📦 Added {} new transactions to cache", new_count)
                        );
                    }
                }
                Err(e) => {
                    Logger::error(&format!("❌ Failed to update transaction cache: {}", e));
                }
            }
        }

        Logger::success("Background transaction caching loop stopped");
    }

    /// Cache historical transactions (initial setup)
    pub async fn cache_historical_transactions(&self) -> Result<usize> {
        Logger::wallet("� Caching historical transactions...");

        let existing_count = self.database.get_transaction_count()?;
        Logger::wallet(&format!("📊 Found {} existing transactions in cache", existing_count));

        if existing_count >= 100 {
            Logger::wallet("✅ Sufficient transactions already cached, skipping historical fetch");
            return Ok(existing_count as usize);
        }

        // Fetch signatures
        Logger::rpc(&format!("📡 Fetching {} transaction signatures...", self.max_cache_size));
        let signatures = tokio::time
            ::timeout(
                Duration::from_secs(60),
                self.rpc_manager.get_signatures_for_address(
                    &self.wallet_pubkey,
                    Some(self.max_cache_size)
                )
            ).await
            .context("RPC call timed out after 60 seconds")?
            .context("Failed to get transaction signatures")?;

        Logger::rpc(&format!("📡 Received {} transaction signatures", signatures.len()));

        let mut cached_count = 0;
        let mut processed = 0;
        let mut skipped_existing = 0;

        for signature_info in signatures.iter() {
            let signature = &signature_info.signature;

            // Check if we already have this transaction
            if self.database.transaction_exists(signature)? {
                skipped_existing += 1;
                continue;
            }

            // Fetch and parse transaction
            if let Ok(wallet_tx) = self.fetch_and_parse_transaction(signature).await {
                // Save to database
                if let Err(e) = self.database.save_wallet_transaction(&wallet_tx) {
                    Logger::error(&format!("Failed to save transaction {}: {}", signature, e));
                } else {
                    cached_count += 1;
                }
            }

            processed += 1;

            // Progress update every 50 transactions
            if processed % 50 == 0 {
                Logger::wallet(
                    &format!(
                        "📊 Progress: {}/{} processed, {} new, {} existing",
                        processed,
                        signatures.len(),
                        cached_count,
                        skipped_existing
                    )
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }

        // Update last processed signature
        if let Some(first_sig) = signatures.first() {
            *self.last_processed_signature.write().await = Some(first_sig.signature.clone());
        }

        Logger::success(
            &format!(
                "✅ Historical caching completed - processed {}, new {}, existing {}",
                processed,
                cached_count,
                skipped_existing
            )
        );

        Ok(cached_count)
    }

    /// Update cache with new transactions since last check
    pub async fn update_cache_with_new_transactions(&self) -> Result<usize> {
        let last_signature = self.last_processed_signature.read().await.clone();

        // Fetch recent signatures
        let signatures = tokio::time
            ::timeout(
                Duration::from_secs(30),
                self.rpc_manager.get_signatures_for_address(&self.wallet_pubkey, Some(50))
            ).await
            .context("RPC call timed out")?
            .context("Failed to get recent transaction signatures")?;

        let mut new_count = 0;
        let mut found_existing = false;

        for signature_info in signatures.iter() {
            let signature = &signature_info.signature;

            // If we reach a signature we've already processed, stop
            if let Some(ref last_sig) = last_signature {
                if signature == last_sig {
                    found_existing = true;
                    break;
                }
            }

            // Check if we already have this transaction
            if self.database.transaction_exists(signature)? {
                found_existing = true;
                break;
            }

            // Fetch and parse new transaction
            if let Ok(wallet_tx) = self.fetch_and_parse_transaction(signature).await {
                if let Err(e) = self.database.save_wallet_transaction(&wallet_tx) {
                    Logger::error(&format!("Failed to save transaction {}: {}", signature, e));
                } else {
                    new_count += 1;
                }
            }
        }

        // Update last processed signature
        if let Some(first_sig) = signatures.first() {
            *self.last_processed_signature.write().await = Some(first_sig.signature.clone());
        }

        Ok(new_count)
    }

    /// Fetch and parse a single transaction
    async fn fetch_and_parse_transaction(&self, signature: &str) -> Result<WalletTransaction> {
        // Fetch transaction with timeout
        let transaction = tokio::time
            ::timeout(Duration::from_secs(10), self.rpc_manager.get_transaction(signature)).await
            .context("Transaction fetch timed out")?
            .context("Failed to get transaction")?;

        // Parse transaction for token operations
        self.parse_transaction_for_tokens(&transaction, signature).await.context(
            "Failed to parse transaction"
        )
    }

    /// Parse transaction for token operations
    async fn parse_transaction_for_tokens(
        &self,
        transaction: &EncodedConfirmedTransactionWithStatusMeta,
        signature: &str
    ) -> Result<WalletTransaction> {
        let block_time = transaction.block_time.unwrap_or(0);
        let slot = transaction.slot;

        // For now, create a placeholder transaction
        // This should be enhanced to actually parse the transaction instructions
        Ok(WalletTransaction {
            signature: signature.to_string(),
            mint: "placeholder".to_string(),
            transaction_type: TransactionType::Transfer,
            amount: 0,
            price_sol: None,
            value_sol: None,
            sol_amount: None,
            fee: None,
            block_time,
            slot,
            created_at: chrono::Utc::now(),
        })
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<(usize, String)> {
        let count = self.database.get_transaction_count()?;
        let last_sig = self.last_processed_signature.read().await.clone();
        let status = if *self.is_running.read().await {
            "Running".to_string()
        } else {
            "Stopped".to_string()
        };

        Ok((
            count as usize,
            format!(
                "{} (Last: {})",
                status,
                last_sig.map(|s| format!("{}...", &s[..8])).unwrap_or("None".to_string())
            ),
        ))
    }

    /// Clean old transactions to maintain cache size
    pub async fn cleanup_old_transactions(&self) -> Result<i64> {
        self.database.clean_old_transactions(self.max_cache_size)
    }
}

impl Clone for TransactionCacheManager {
    fn clone(&self) -> Self {
        Self {
            database: Arc::clone(&self.database),
            rpc_manager: Arc::clone(&self.rpc_manager),
            wallet_pubkey: self.wallet_pubkey,
            max_cache_size: self.max_cache_size,
            is_running: Arc::clone(&self.is_running),
            last_processed_signature: Arc::clone(&self.last_processed_signature),
        }
    }
}
