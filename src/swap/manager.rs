use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::*;
use super::traits::SwapProvider;
use super::providers::{ GmgnProvider };
use anyhow::Result;

/// Manager for coordinating multiple swap providers
pub struct SwapManager {
    providers: RwLock<HashMap<String, Arc<dyn SwapProvider + Send + Sync>>>,
    default_provider: String,
}

impl SwapManager {
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            default_provider: "gmgn".to_string(),
        }
    }

    /// Initialize with standard providers
    pub async fn with_defaults() -> Self {
        let manager = Self::new();

        // Add GMGN provider
        let gmgn = Arc::new(GmgnProvider::new(None));
        manager.add_provider(gmgn).await;

        manager
    }

    /// Add a new swap provider
    pub async fn add_provider(&self, provider: Arc<dyn SwapProvider + Send + Sync>) {
        let mut providers = self.providers.write().await;
        providers.insert(provider.id().to_string(), provider);
    }

    /// Remove a provider
    pub async fn remove_provider(&self, provider_id: &str) {
        let mut providers = self.providers.write().await;
        providers.remove(provider_id);
    }

    /// Get a specific provider by ID
    pub async fn get_provider(
        &self,
        provider_id: &str
    ) -> Option<Arc<dyn SwapProvider + Send + Sync>> {
        let providers = self.providers.read().await;
        providers.get(provider_id).cloned()
    }

    /// List all available providers
    pub async fn list_providers(&self) -> Vec<String> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// Set the default provider
    pub fn set_default_provider(&mut self, provider_id: String) {
        self.default_provider = provider_id;
    }

    /// Get the best provider for a token pair
    pub async fn get_best_provider(
        &self,
        token_in: &str,
        token_out: &str
    ) -> Option<Arc<dyn SwapProvider + Send + Sync>> {
        let providers = self.providers.read().await;

        // First try default provider if it supports the pair
        if let Some(default_provider) = providers.get(&self.default_provider) {
            if default_provider.supports_token_pair(token_in, token_out) {
                return Some(default_provider.clone());
            }
        }

        // Find any provider that supports the pair
        for provider in providers.values() {
            if provider.supports_token_pair(token_in, token_out) {
                return Some(provider.clone());
            }
        }

        None
    }

    /// Get quotes from all supporting providers
    pub async fn get_all_quotes(
        &self,
        request: &SwapRequest
    ) -> HashMap<String, Result<SwapQuote>> {
        let providers = self.providers.read().await;
        let mut quotes = HashMap::new();

        for (id, provider) in providers.iter() {
            if provider.supports_token_pair(&request.token_in_address, &request.token_out_address) {
                let quote_result = provider.get_quote(request).await;
                quotes.insert(id.clone(), quote_result);
            }
        }

        quotes
    }

    /// Get the best quote from all providers
    pub async fn get_best_quote(&self, request: &SwapRequest) -> Result<(String, SwapQuote)> {
        let quotes = self.get_all_quotes(request).await;

        if quotes.is_empty() {
            return Err(anyhow::anyhow!("No providers support this token pair"));
        }

        let mut best_quote: Option<(String, SwapQuote)> = None;
        let mut best_output_amount = 0u64;

        for (provider_id, quote_result) in quotes {
            if let Ok(quote) = quote_result {
                if quote.out_amount > best_output_amount {
                    best_output_amount = quote.out_amount;
                    best_quote = Some((provider_id, quote));
                }
            }
        }

        best_quote.ok_or_else(|| anyhow::anyhow!("No valid quotes received from providers"))
    }

    /// Execute swap with the best available provider
    pub async fn execute_best_swap(&self, request: &SwapRequest) -> Result<SwapResult> {
        let (provider_id, quote) = self.get_best_quote(request).await?;

        if let Some(provider) = self.get_provider(&provider_id).await {
            provider.execute_swap(request, &quote).await
        } else {
            Err(anyhow::anyhow!("Provider not found"))
        }
    }

    /// Execute swap with a specific provider
    pub async fn execute_swap_with_provider(
        &self,
        request: &SwapRequest,
        provider_id: &str
    ) -> Result<SwapResult> {
        if let Some(provider) = self.get_provider(provider_id).await {
            let quote = provider.get_quote(request).await?;
            provider.execute_swap(request, &quote).await
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", provider_id))
        }
    }

    /// Monitor transaction status across providers
    pub async fn monitor_transaction(
        &self,
        signature: &str,
        provider_id: &str
    ) -> Result<TransactionStatus> {
        if let Some(provider) = self.get_provider(provider_id).await {
            provider.get_transaction_status(signature).await
        } else {
            Err(anyhow::anyhow!("Provider '{}' not found", provider_id))
        }
    }

    /// Helper method for quick buy operations
    pub async fn quick_buy(
        &self,
        token_address: &str,
        amount_sol: f64,
        wallet_address: &str,
        slippage_bps: Option<u16>
    ) -> Result<SwapResult> {
        let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;
        let slippage = slippage_bps.unwrap_or(500); // 5% default

        let request = SwapRequest::new_buy(
            token_address,
            amount_lamports,
            wallet_address,
            slippage,
            100000 // Default priority fee
        );

        self.execute_best_swap(&request).await
    }

    /// Helper method for quick sell operations
    pub async fn quick_sell(
        &self,
        token_address: &str,
        amount_tokens: u64,
        wallet_address: &str,
        slippage_bps: Option<u16>
    ) -> Result<SwapResult> {
        let slippage = slippage_bps.unwrap_or(500); // 5% default

        let request = SwapRequest::new_sell(
            token_address,
            amount_tokens,
            wallet_address,
            slippage,
            100000 // Default priority fee
        );

        self.execute_best_swap(&request).await
    }

    /// Print status of all providers
    pub async fn print_provider_status(&self) {
        let providers = self.providers.read().await;

        println!("🔧 SWAP MANAGER STATUS");
        println!("  Default Provider: {}", self.default_provider);
        println!("  Available Providers: {}", providers.len());

        for (id, provider) in providers.iter() {
            let config = provider.get_config();
            println!(
                "    • {} (supports: {} chains, max_slippage: {}bps)",
                id,
                config.supported_chains.len(),
                config.max_slippage_bps
            );
        }
    }
}

impl Default for SwapManager {
    fn default() -> Self {
        Self::new()
    }
}
