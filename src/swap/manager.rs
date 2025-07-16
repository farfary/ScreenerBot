use super::{ gmgn::GmgnProvider, jupiter::JupiterProvider, types::* };
use crate::config::SwapConfig;
use crate::rpc::RpcManager;
use anyhow::Result;
use base64::{ engine::general_purpose, Engine as _ };
use bincode;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{ Keypair, Signature },
    signer::Signer,
    transaction::VersionedTransaction,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::RwLock;

pub struct SwapManager {
    config: SwapConfig,
    jupiter: JupiterProvider,
    gmgn: GmgnProvider,
    rpc_manager: Arc<RpcManager>,
    stats: Arc<RwLock<SwapStats>>,
}

impl SwapManager {
    pub fn new(config: SwapConfig, rpc_manager: Arc<RpcManager>) -> Self {
        let jupiter = JupiterProvider::new(config.jupiter.clone());
        let gmgn = GmgnProvider::new(config.gmgn.clone());

        Self {
            config,
            jupiter,
            gmgn,
            rpc_manager,
            stats: Arc::new(RwLock::new(SwapStats::default())),
        }
    }

    /// Get quotes from all available providers
    pub async fn get_all_quotes(
        &self,
        request: &SwapRequest
    ) -> HashMap<SwapProvider, SwapResult<SwapQuote>> {
        let mut quotes = HashMap::new();

        // Validate request
        if let Err(e) = self.validate_swap_request(request).await {
            log::error!("Invalid swap request: {}", e);
            return quotes;
        }

        // Get quotes from all enabled providers sequentially
        if self.jupiter.is_available() {
            let result = self.jupiter.get_quote(
                &request.input_mint,
                &request.output_mint,
                request.amount,
                request.slippage_bps
            ).await;
            quotes.insert(SwapProvider::Jupiter, result);
        }

        if self.gmgn.is_available() {
            let result = self.gmgn.get_quote(
                &request.input_mint,
                &request.output_mint,
                request.amount,
                request.slippage_bps
            ).await;
            quotes.insert(SwapProvider::Gmgn, result);
        }

        quotes
    }

    /// Get the best quote based on output amount and other factors
    pub async fn get_best_quote(&self, request: &SwapRequest) -> SwapResult<SwapQuote> {
        let quotes = self.get_all_quotes(request).await;

        if quotes.is_empty() {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Jupiter));
        }

        let mut best_quote: Option<SwapQuote> = None;
        let mut best_score = 0.0;

        for (provider, quote_result) in quotes {
            match quote_result {
                Ok(quote) => {
                    let score = self.calculate_quote_score(&quote).await;
                    log::info!(
                        "Quote from {}: {} -> {} (score: {:.2})",
                        provider,
                        quote.in_amount,
                        quote.out_amount,
                        score
                    );

                    if score > best_score {
                        best_score = score;
                        best_quote = Some(quote);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to get quote from {}: {}", provider, e);
                }
            }
        }

        best_quote.ok_or(
            SwapError::QuoteFailed(SwapProvider::Jupiter, "No valid quotes available".to_string())
        )
    }

    pub async fn execute_swap(
        &self,
        request: &SwapRequest,
        quote: &SwapQuote,
        keypair: &Keypair
    ) -> SwapResult<SwapExecutionResult> {
        let start_time = Instant::now();

        // Validate balance
        self.validate_balance(request, keypair).await?;

        // Get swap transaction based on provider
        let transaction = match quote.provider {
            SwapProvider::Jupiter => {
                self.jupiter.get_swap_transaction(
                    &request.user_public_key,
                    quote,
                    request.wrap_unwrap_sol,
                    request.use_shared_accounts,
                    request.priority_fee,
                    request.compute_unit_price
                ).await?
            }
            SwapProvider::Gmgn => {
                self.gmgn.get_swap_transaction(
                    &request.user_public_key,
                    quote,
                    request.wrap_unwrap_sol,
                    request.wrap_unwrap_sol,
                    request.priority_fee
                ).await?
            }
        };

        // Execute the transaction
        let signature = self.send_transaction(&transaction, keypair).await?;

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Record statistics
        self.record_swap_result(
            &transaction.provider,
            true,
            execution_time,
            quote.out_amount as f64
        ).await;

        log::info!(
            "✅ Swap executed successfully via {}: {} in {} ms",
            transaction.provider,
            signature,
            execution_time
        );

        Ok(SwapExecutionResult {
            provider: transaction.provider,
            signature,
            input_amount: quote.in_amount,
            output_amount: quote.out_amount,
            actual_fee: transaction.priority_fee,
            execution_time_ms: execution_time,
            success: true,
            error_message: None,
        })
    }

    pub async fn swap(
        &self,
        request: &SwapRequest,
        keypair: &Keypair
    ) -> SwapResult<SwapExecutionResult> {
        log::info!(
            "🔄 Starting swap: {} -> {} (amount: {})",
            request.input_mint,
            request.output_mint,
            request.amount
        );

        // Get best quote
        let quote = self.get_best_quote(request).await?;

        log::info!(
            "📊 Best quote from {}: {} -> {} (impact: {:.2}%)",
            quote.provider,
            quote.in_amount,
            quote.out_amount,
            quote.price_impact_pct
        );

        // Execute swap
        match self.execute_swap(request, &quote, keypair).await {
            Ok(result) => Ok(result),
            Err(e) => {
                self.record_swap_result(&quote.provider, false, 0, 0.0).await;
                Err(e)
            }
        }
    }

    pub async fn swap_with_provider(
        &self,
        request: &SwapRequest,
        provider: SwapProvider,
        keypair: &Keypair
    ) -> SwapResult<SwapExecutionResult> {
        log::info!(
            "🔄 Starting swap with {}: {} -> {} (amount: {})",
            provider,
            request.input_mint,
            request.output_mint,
            request.amount
        );

        // Get quote from specific provider
        let quote = match provider {
            SwapProvider::Jupiter => {
                self.jupiter.get_quote(
                    &request.input_mint,
                    &request.output_mint,
                    request.amount,
                    request.slippage_bps
                ).await?
            }
            SwapProvider::Gmgn => {
                self.gmgn.get_quote(
                    &request.input_mint,
                    &request.output_mint,
                    request.amount,
                    request.slippage_bps
                ).await?
            }
        };

        log::info!(
            "📊 Quote from {}: {} -> {} (impact: {:.2}%)",
            provider,
            quote.in_amount,
            quote.out_amount,
            quote.price_impact_pct
        );

        // Execute swap
        match self.execute_swap(request, &quote, keypair).await {
            Ok(result) => Ok(result),
            Err(e) => {
                self.record_swap_result(&provider, false, 0, 0.0).await;
                Err(e)
            }
        }
    }

    async fn send_transaction(
        &self,
        transaction: &SwapTransaction,
        keypair: &Keypair
    ) -> SwapResult<Signature> {
        // For GMGN, use specialized execution method
        if transaction.provider == SwapProvider::Gmgn {
            let rpc_client = self.rpc_manager
                .get_rpc_client()
                .map_err(|e|
                    SwapError::TransactionFailed(
                        SwapProvider::Gmgn,
                        format!("Failed to get RPC client: {}", e)
                    )
                )?;

            return self.gmgn.execute_swap(transaction, keypair, &rpc_client).await;
        }

        // For Jupiter, use the existing method
        let transaction_bytes = general_purpose::STANDARD
            .decode(&transaction.serialized_transaction)
            .map_err(|e|
                SwapError::TransactionFailed(
                    transaction.provider.clone(),
                    format!("Failed to decode transaction: {}", e)
                )
            )?;

        let mut versioned_transaction: VersionedTransaction = bincode
            ::deserialize(&transaction_bytes)
            .map_err(|e|
                SwapError::TransactionFailed(
                    transaction.provider.clone(),
                    format!("Failed to deserialize transaction: {}", e)
                )
            )?;

        // Get recent blockhash for signing
        let recent_blockhash = self.rpc_manager
            .get_latest_blockhash().await
            .map_err(|e|
                SwapError::TransactionFailed(
                    transaction.provider.clone(),
                    format!("Failed to get recent blockhash: {:?}", e)
                )
            )?;

        // Handle both V0 and Legacy transactions properly
        use solana_sdk::message::VersionedMessage;

        match &mut versioned_transaction.message {
            VersionedMessage::V0(ref mut v0_msg) => {
                v0_msg.recent_blockhash = recent_blockhash;

                // Clear existing signatures and sign with our keypair
                versioned_transaction.signatures.clear();
                let message = versioned_transaction.message.clone();
                let message_bytes = bincode
                    ::serialize(&message)
                    .map_err(|e|
                        SwapError::TransactionFailed(
                            transaction.provider.clone(),
                            format!("Failed to serialize V0 message: {}", e)
                        )
                    )?;
                let signature = keypair.sign_message(&message_bytes);
                versioned_transaction.signatures.push(signature);
            }
            VersionedMessage::Legacy(ref mut legacy_msg) => {
                legacy_msg.recent_blockhash = recent_blockhash;

                // Create a legacy transaction for signing
                let legacy_transaction = solana_sdk::transaction::Transaction::new(
                    &[keypair],
                    legacy_msg.clone(),
                    recent_blockhash
                );

                // Copy signatures back to versioned transaction
                versioned_transaction.signatures = legacy_transaction.signatures;
            }
        }

        // Send the transaction using RPC manager
        let signature = self.rpc_manager
            .send_transaction(&versioned_transaction, None).await
            .map_err(|e|
                SwapError::TransactionFailed(
                    transaction.provider.clone(),
                    format!("Failed to send transaction: {}", e)
                )
            )?;

        Ok(signature)
    }

    async fn validate_swap_request(&self, request: &SwapRequest) -> SwapResult<()> {
        // Validate amount
        if request.amount == 0 {
            return Err(SwapError::InvalidAmount("Amount cannot be zero".to_string()));
        }

        let amount_sol = (request.amount as f64) / 1e9;
        if amount_sol < self.config.min_amount_sol {
            return Err(
                SwapError::InvalidAmount(
                    format!(
                        "Amount {} SOL is below minimum {}",
                        amount_sol,
                        self.config.min_amount_sol
                    )
                )
            );
        }

        if amount_sol > self.config.max_amount_sol {
            return Err(
                SwapError::InvalidAmount(
                    format!(
                        "Amount {} SOL is above maximum {}",
                        amount_sol,
                        self.config.max_amount_sol
                    )
                )
            );
        }

        // Validate slippage
        if request.slippage_bps > self.config.max_slippage_bps {
            return Err(
                SwapError::InvalidAmount(
                    format!(
                        "Slippage {}bps exceeds maximum {}bps",
                        request.slippage_bps,
                        self.config.max_slippage_bps
                    )
                )
            );
        }

        Ok(())
    }

    async fn validate_balance(&self, request: &SwapRequest, keypair: &Keypair) -> SwapResult<()> {
        let balance = self.rpc_manager
            .get_balance(&keypair.pubkey()).await
            .map_err(|e| SwapError::InsufficientBalance(0, 0))?;

        if balance < request.amount {
            return Err(SwapError::InsufficientBalance(request.amount, balance));
        }

        Ok(())
    }

    async fn calculate_quote_score(&self, quote: &SwapQuote) -> f64 {
        // Score based on output amount (primary factor)
        let output_score = quote.out_amount as f64;

        // Penalty for price impact
        let impact_penalty = if quote.price_impact_pct > 1.0 {
            1.0 - quote.price_impact_pct / 10.0
        } else {
            1.0
        };

        // Provider preference (based on historical performance)
        let provider_bonus = match quote.provider {
            SwapProvider::Jupiter => 1.1, // Slight preference for Jupiter
            SwapProvider::Gmgn => 1.0,
        };

        output_score * impact_penalty * provider_bonus
    }

    async fn record_swap_result(
        &self,
        provider: &SwapProvider,
        success: bool,
        execution_time_ms: u64,
        volume: f64
    ) {
        let mut stats = self.stats.write().await;
        stats.total_swaps += 1;

        if success {
            stats.successful_swaps += 1;
            stats.total_volume += volume;

            // Update average execution time
            let total_successful = stats.successful_swaps;
            stats.average_execution_time_ms =
                (stats.average_execution_time_ms * (total_successful - 1) + execution_time_ms) /
                total_successful;
        } else {
            stats.failed_swaps += 1;
        }

        // Update provider-specific stats
        let provider_stats = stats.provider_stats.entry(provider.clone()).or_default();
        provider_stats.swaps_count += 1;

        if success {
            provider_stats.total_volume += volume;

            // Update average execution time for this provider
            let provider_successful = ((provider_stats.swaps_count as f64) *
                provider_stats.success_rate) as u64;
            if provider_successful > 0 {
                provider_stats.average_execution_time_ms =
                    (provider_stats.average_execution_time_ms * (provider_successful - 1) +
                        execution_time_ms) /
                    provider_successful;
            }
        } else {
            provider_stats.error_count += 1;
        }

        // Calculate success rate
        provider_stats.success_rate =
            ((provider_stats.swaps_count - provider_stats.error_count) as f64) /
            (provider_stats.swaps_count as f64);
    }

    /// Get current swap statistics
    pub async fn get_stats(&self) -> SwapStats {
        self.stats.read().await.clone()
    }

    /// Health check for all providers
    pub async fn health_check(&self) -> HashMap<SwapProvider, bool> {
        let mut health_status = HashMap::new();

        let jupiter_health = self.jupiter.health_check().await.unwrap_or(false);
        health_status.insert(SwapProvider::Jupiter, jupiter_health);

        let gmgn_health = self.gmgn.health_check().await.unwrap_or(false);
        health_status.insert(SwapProvider::Gmgn, gmgn_health);

        health_status
    }

    /// Get available providers
    pub fn get_available_providers(&self) -> Vec<SwapProvider> {
        let mut providers = Vec::new();

        if self.jupiter.is_available() {
            providers.push(SwapProvider::Jupiter);
        }
        if self.gmgn.is_available() {
            providers.push(SwapProvider::Gmgn);
        }

        providers
    }
}

// Helper function to create a swap request
pub fn create_swap_request(
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount: u64,
    user_public_key: Pubkey,
    slippage_bps: Option<u16>,
    preferred_provider: Option<SwapProvider>
) -> SwapRequest {
    SwapRequest {
        input_mint,
        output_mint,
        amount,
        slippage_bps: slippage_bps.unwrap_or(50), // 0.5% default
        user_public_key,
        preferred_provider,
        priority_fee: None,
        compute_unit_price: None,
        wrap_unwrap_sol: true,
        use_shared_accounts: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ Config, RpcConfig };
    use std::str::FromStr;

    fn get_test_config() -> SwapConfig {
        SwapConfig::default()
    }

    fn get_test_rpc_manager() -> Arc<RpcManager> {
        let rpc_config = RpcConfig::default();
        Arc::new(
            RpcManager::new(
                "https://api.mainnet-beta.solana.com".to_string(),
                vec![],
                rpc_config
            ).unwrap()
        )
    }

    #[tokio::test]
    async fn test_swap_manager_creation() {
        let config = get_test_config();
        let rpc_manager = get_test_rpc_manager();
        let swap_manager = SwapManager::new(config, rpc_manager);

        let providers = swap_manager.get_available_providers();
        assert!(!providers.is_empty(), "Should have at least one provider available");

        println!("Available providers: {:?}", providers);
    }

    #[tokio::test]
    async fn test_health_check() {
        let config = get_test_config();
        let rpc_manager = get_test_rpc_manager();
        let swap_manager = SwapManager::new(config, rpc_manager);

        let health_status = swap_manager.health_check().await;

        for (provider, healthy) in health_status {
            println!("{}: {}", provider, if healthy { "✅" } else { "❌" });
        }
    }

    #[test]
    fn test_create_swap_request() {
        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let user_key = Pubkey::from_str("B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59").unwrap();

        let request = create_swap_request(
            sol_mint,
            usdc_mint,
            1000000,
            user_key,
            Some(100),
            Some(SwapProvider::Jupiter)
        );

        assert_eq!(request.input_mint, sol_mint);
        assert_eq!(request.output_mint, usdc_mint);
        assert_eq!(request.amount, 1000000);
        assert_eq!(request.slippage_bps, 100);
        assert_eq!(request.preferred_provider, Some(SwapProvider::Jupiter));
    }
}
