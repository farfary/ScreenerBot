use async_trait::async_trait;
use anyhow::Result;
use serde::{ Deserialize, Serialize };
use super::types::*;

/// Core trait for swap providers (GMGN, Jupiter, etc.)
#[async_trait]
pub trait SwapProvider: Send + Sync {
    /// Get a unique identifier for this provider
    fn id(&self) -> &str;

    /// Get a quote for a swap without executing
    async fn get_quote(&self, request: &SwapRequest) -> Result<SwapQuote>;

    /// Execute a swap and return the result
    async fn execute_swap(&self, request: &SwapRequest, quote: &SwapQuote) -> Result<SwapResult>;

    /// Get the status of a transaction
    async fn get_transaction_status(&self, tx_signature: &str) -> Result<TransactionStatus>;

    /// Validate if a token pair is supported by this provider
    fn supports_token_pair(&self, token_in: &str, token_out: &str) -> bool;

    /// Get provider-specific configuration
    fn get_config(&self) -> &ProviderConfig;
}

/// Configuration for a swap provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub max_slippage_bps: u16,
    pub min_amount_lamports: u64,
    pub max_amount_lamports: u64,
    pub supported_chains: Vec<String>,
    pub default_fee_bps: u16,
    pub priority_fee_lamports: u64,
    pub timeout_seconds: u64,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            max_slippage_bps: 1000, // 10%
            min_amount_lamports: 1000, // 0.000001 SOL
            max_amount_lamports: 1000000000000, // 1000 SOL
            supported_chains: vec!["solana".to_string()],
            default_fee_bps: 0,
            priority_fee_lamports: 5000,
            timeout_seconds: 30,
        }
    }
}
