use crate::core::{
    BotResult,
    BotError,
    WalletTransaction,
    TransactionType,
    TransactionStatus,
    ParsedTransactionData,
};
use crate::core::RpcManager;
use crate::cache::CacheManager;
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{ UiTransactionEncoding };
use chrono::Utc;
use std::collections::HashMap;

/// Manages wallet transaction queries and parsing
pub struct TransactionManager<'a> {
    rpc: &'a RpcManager,
    cache: Option<&'a CacheManager>,
    max_cache_age_hours: u64,
}

impl<'a> TransactionManager<'a> {
    pub fn new(rpc: &'a RpcManager) -> Self {
        Self {
            rpc,
            cache: None,
            max_cache_age_hours: 24, // Default cache age
        }
    }

    pub fn with_cache(rpc: &'a RpcManager, cache: &'a CacheManager) -> Self {
        Self {
            rpc,
            cache: Some(cache),
            max_cache_age_hours: 24,
        }
    }

    pub fn set_cache_age(&mut self, hours: u64) {
        self.max_cache_age_hours = hours;
    }

    /// Get recent transactions for a wallet address with caching
    pub async fn get_recent_transactions(
        &self,
        address: &Pubkey,
        limit: usize
    ) -> BotResult<Vec<WalletTransaction>> {
        log::info!("📜 Fetching recent transactions for wallet: {}", address);

        // Try to get from cache first if available
        if let Some(cache) = self.cache {
            if let Ok(cached_transactions) = cache.get_cached_transactions(address, limit).await {
                if !cached_transactions.is_empty() {
                    log::info!("📦 Retrieved {} cached transactions", cached_transactions.len());
                    return Ok(cached_transactions);
                }
            }
        }

        // Fetch from RPC if not in cache or cache is empty
        let mut transactions = Vec::new();

        // Get signature list from RPC
        match self.get_signatures_for_address(address, limit).await {
            Ok(signatures) => {
                log::info!("🔍 Found {} signatures, processing transactions...", signatures.len());

                // Process each signature to get full transaction details
                for (i, sig_info) in signatures.iter().enumerate().take(limit) {
                    match self.fetch_and_parse_transaction(&sig_info.signature, address).await {
                        Ok(transaction) => {
                            transactions.push(transaction.clone());

                            // Cache individual transaction if cache is available
                            if let Some(cache) = self.cache {
                                if
                                    let Err(e) = cache.cache_transaction(
                                        address,
                                        &transaction
                                    ).await
                                {
                                    log::warn!("⚠️ Failed to cache transaction: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!(
                                "⚠️ Failed to process transaction {}: {}",
                                sig_info.signature,
                                e
                            );
                        }
                    }

                    // Log progress for large batches
                    if i % 10 == 0 && i > 0 {
                        log::debug!("📊 Processed {}/{} transactions", i + 1, signatures.len());
                    }
                }

                log::info!("✅ Successfully processed {} transactions", transactions.len());
            }
            Err(e) => {
                log::warn!("⚠️ Failed to get signatures for address {}: {}", address, e);
                // Return empty list instead of failing completely
            }
        }

        Ok(transactions)
    }

    /// Get signatures for an address using RPC
    async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: usize
    ) -> BotResult<Vec<solana_client::rpc_response::RpcConfirmedTransactionStatusWithSignature>> {
        self.rpc.get_signatures_for_address(address, limit).await
    }

    /// Fetch and parse a single transaction
    async fn fetch_and_parse_transaction(
        &self,
        signature: &str,
        wallet_address: &Pubkey
    ) -> BotResult<WalletTransaction> {
        let signature = signature
            .parse()
            .map_err(|e| BotError::Parse(format!("Invalid signature: {}", e)))?;

        let transaction = self.rpc.get_transaction_with_config(
            &signature,
            UiTransactionEncoding::JsonParsed
        ).await?;

        self.parse_transaction_data(transaction, wallet_address).await
    }

    /// Parse transaction data into WalletTransaction
    async fn parse_transaction_data(
        &self,
        transaction: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        wallet_address: &Pubkey
    ) -> BotResult<WalletTransaction> {
        let meta = transaction.transaction.meta
            .as_ref()
            .ok_or_else(|| BotError::Parse("Transaction missing metadata".to_string()))?;

        let block_time = transaction.block_time;
        let slot = transaction.slot;

        // Parse transaction type and changes
        let (transaction_type, tokens_involved, sol_change, token_changes, parsed_data) =
            self.parse_transaction_effects(&transaction, wallet_address)?;

        // Determine transaction status
        let status = if meta.err.is_some() {
            TransactionStatus::Failed
        } else {
            TransactionStatus::Success
        };

        // Calculate fees
        let fees = meta.fee;

        Ok(WalletTransaction {
            signature: "placeholder_sig".to_string(), // TODO: Extract from encoded transaction
            block_time,
            slot,
            transaction_type,
            tokens_involved,
            sol_change,
            token_changes,
            fees,
            status,
            parsed_data,
        })
    }

    /// Parse transaction effects (token changes, SOL changes, etc.)
    fn parse_transaction_effects(
        &self,
        transaction: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        wallet_address: &Pubkey
    ) -> BotResult<
        (TransactionType, Vec<Pubkey>, i64, HashMap<Pubkey, i64>, Option<ParsedTransactionData>)
    > {
        let meta = transaction.transaction.meta
            .as_ref()
            .ok_or_else(|| BotError::Parse("Missing transaction metadata".to_string()))?;

        // For encoded transactions, we'll need to handle this differently
        // For now, let's skip the complex parsing and return basic info
        let mut transaction_type = TransactionType::Unknown;
        let tokens_involved = Vec::new();
        let mut sol_change = 0i64;
        let token_changes: HashMap<Pubkey, i64> = HashMap::new();

        // Calculate SOL balance change if possible
        if meta.pre_balances.len() > 0 && meta.post_balances.len() > 0 {
            // For encoded transactions, we need to find wallet index differently
            // For now, assume wallet is at index 0 if balances exist
            if
                let (Some(&pre_balance), Some(&post_balance)) = (
                    meta.pre_balances.get(0),
                    meta.post_balances.get(0),
                )
            {
                sol_change = (post_balance as i64) - (pre_balance as i64);
            }
        }

        // Parse token balance changes - simplified for encoded transactions
        // TODO: Properly parse token balance changes from encoded transaction metadata
        // For now, we'll detect transaction type based on other indicators

        // Determine transaction type based on changes
        transaction_type = self.determine_transaction_type(sol_change, &token_changes);

        // Try to parse additional data for swaps/trades
        let parsed_data = self.try_parse_swap_data(transaction, &tokens_involved);

        Ok((transaction_type, tokens_involved, sol_change, token_changes, parsed_data))
    }

    /// Determine transaction type based on balance changes
    fn determine_transaction_type(
        &self,
        sol_change: i64,
        token_changes: &HashMap<Pubkey, i64>
    ) -> TransactionType {
        if token_changes.is_empty() {
            if sol_change != 0 { TransactionType::Transfer } else { TransactionType::Unknown }
        } else if token_changes.len() == 1 {
            let change = token_changes.values().next().unwrap();
            if *change > 0 {
                TransactionType::Buy // Received tokens
            } else {
                TransactionType::Sell // Sent tokens
            }
        } else {
            TransactionType::Swap // Multiple token changes
        }
    }

    /// Try to parse swap/trade specific data
    fn try_parse_swap_data(
        &self,
        _transaction: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        tokens_involved: &[Pubkey]
    ) -> Option<ParsedTransactionData> {
        // This would be enhanced to parse specific DEX instruction data
        // For now, return basic structure if tokens are involved
        if tokens_involved.len() >= 2 {
            Some(ParsedTransactionData {
                input_token: tokens_involved.get(0).copied(),
                output_token: tokens_involved.get(1).copied(),
                input_amount: None, // Would be parsed from instruction data
                output_amount: None, // Would be parsed from instruction data
                price_per_token: None,
                pool_address: None, // Would be extracted from accounts
                dex: Some("Unknown".to_string()),
            })
        } else {
            None
        }
    }

    /// Get specific transaction details by signature
    pub async fn get_transaction_details(
        &self,
        signature: &str
    ) -> BotResult<Option<WalletTransaction>> {
        log::debug!("🔍 Fetching transaction details for: {}", signature);

        // Try cache first if available
        if let Some(cache) = self.cache {
            // Implementation would check cache by signature
            // For now, we'll fetch from RPC
        }

        let signature = signature
            .parse()
            .map_err(|e| BotError::Parse(format!("Invalid signature: {}", e)))?;

        match
            self.rpc.client.get_transaction_with_config(
                &signature,
                solana_client::rpc_config::RpcTransactionConfig {
                    encoding: Some(UiTransactionEncoding::JsonParsed),
                    commitment: Some(self.rpc.client.commitment()),
                    max_supported_transaction_version: Some(0),
                }
            )
        {
            Ok(transaction) => {
                // We need the wallet address to parse properly
                // This is a limitation of this method - we don't know which wallet to analyze for
                // For now, return None and log a warning
                log::warn!("⚠️ get_transaction_details requires wallet address context");
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    /// Parse transaction data from raw transaction (enhanced)
    pub async fn parse_raw_transaction(
        &self,
        raw_data: &[u8]
    ) -> BotResult<Option<WalletTransaction>> {
        // This would deserialize and parse raw transaction bytes
        // For now, return None as this requires more complex implementation
        log::debug!("📝 Parsing raw transaction data ({} bytes)", raw_data.len());
        Ok(None)
    }

    /// Get token-specific transactions for a wallet address
    pub async fn get_token_transactions(
        &self,
        address: &Pubkey,
        token_mint: &Pubkey
    ) -> BotResult<Vec<WalletTransaction>> {
        log::info!("🪙 Getting transactions for token {} from wallet {}", token_mint, address);

        // Get all recent transactions first
        let all_transactions = self.get_recent_transactions(address, 100).await?;

        // Filter for transactions involving the specific token
        let token_transactions: Vec<WalletTransaction> = all_transactions
            .into_iter()
            .filter(|tx| tx.tokens_involved.contains(token_mint))
            .collect();

        log::info!(
            "🎯 Found {} transactions involving token {}",
            token_transactions.len(),
            token_mint
        );
        Ok(token_transactions)
    }

    /// Clear old cached transactions
    pub async fn cleanup_old_cache(&self) -> BotResult<()> {
        if let Some(cache) = self.cache {
            // Implementation would clean up old cached transactions
            log::info!("🧹 Cleaning up old cached transactions");
            // This would be implemented in the cache manager
        }
        Ok(())
    }

    /// Get transaction statistics
    pub async fn get_transaction_stats(
        &self,
        address: &Pubkey,
        days: u32
    ) -> BotResult<TransactionStats> {
        let transactions = self.get_recent_transactions(address, 1000).await?;

        let cutoff_time = Utc::now().timestamp() - (days as i64) * 24 * 60 * 60;
        let recent_transactions: Vec<&WalletTransaction> = transactions
            .iter()
            .filter(|tx| tx.block_time.unwrap_or(0) > cutoff_time)
            .collect();

        let total_transactions = recent_transactions.len();
        let successful_transactions = recent_transactions
            .iter()
            .filter(|tx| matches!(tx.status, TransactionStatus::Success))
            .count();

        let total_fees: u64 = recent_transactions
            .iter()
            .map(|tx| tx.fees)
            .sum();

        let buy_count = recent_transactions
            .iter()
            .filter(|tx| matches!(tx.transaction_type, TransactionType::Buy))
            .count();

        let sell_count = recent_transactions
            .iter()
            .filter(|tx| matches!(tx.transaction_type, TransactionType::Sell))
            .count();

        Ok(TransactionStats {
            total_transactions,
            successful_transactions,
            failed_transactions: total_transactions - successful_transactions,
            total_fees_lamports: total_fees,
            buy_transactions: buy_count,
            sell_transactions: sell_count,
            swap_transactions: recent_transactions
                .iter()
                .filter(|tx| matches!(tx.transaction_type, TransactionType::Swap))
                .count(),
            period_days: days,
        })
    }
}

/// Transaction statistics for a wallet
#[derive(Debug, Clone)]
pub struct TransactionStats {
    pub total_transactions: usize,
    pub successful_transactions: usize,
    pub failed_transactions: usize,
    pub total_fees_lamports: u64,
    pub buy_transactions: usize,
    pub sell_transactions: usize,
    pub swap_transactions: usize,
    pub period_days: u32,
}
