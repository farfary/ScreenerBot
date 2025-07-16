use super::types::*;
use crate::config::JupiterConfig;
use anyhow::Result;
use reqwest::Client;
use serde_json;
use solana_sdk::pubkey::Pubkey;
use std::time::Duration;

const JUPITER_API_BASE: &str = "https://quote-api.jup.ag/v6";

pub struct JupiterProvider {
    client: Client,
    config: JupiterConfig,
}

impl JupiterProvider {
    pub fn new(config: JupiterConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub async fn get_quote(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16
    ) -> SwapResult<SwapQuote> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Jupiter));
        }

        let quote_request = JupiterQuoteRequest {
            input_mint: input_mint.to_string(),
            output_mint: output_mint.to_string(),
            amount: amount.to_string(),
            slippage_bps,
        };

        let url = format!("{}/quote", JUPITER_API_BASE);
        let query_params = [
            ("inputMint", &quote_request.input_mint),
            ("outputMint", &quote_request.output_mint),
            ("amount", &quote_request.amount),
            ("slippageBps", &slippage_bps.to_string()),
        ];

        let response = self.client
            .get(&url)
            .query(&query_params)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SwapError::RateLimited(SwapProvider::Jupiter));
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Jupiter,
                    format!("HTTP {}: {}", status, error_text)
                )
            );
        }

        let jupiter_quote: JupiterQuoteResponse = response
            .json().await
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Jupiter, e.to_string()))?;

        self.convert_jupiter_quote_to_swap_quote(jupiter_quote, input_mint, output_mint)
    }

    pub async fn get_swap_transaction(
        &self,
        user_public_key: &Pubkey,
        quote: &SwapQuote,
        wrap_unwrap_sol: bool,
        use_shared_accounts: bool,
        priority_fee: Option<u64>,
        compute_unit_price: Option<u64>
    ) -> SwapResult<SwapTransaction> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Jupiter));
        }

        // Extract the original Jupiter quote from the raw response
        let jupiter_quote: JupiterQuoteResponse = serde_json
            ::from_value(quote.raw_response.clone())
            .map_err(|e| SwapError::TransactionFailed(SwapProvider::Jupiter, e.to_string()))?;

        let swap_request = JupiterSwapRequest {
            user_public_key: user_public_key.to_string(),
            quote_response: jupiter_quote,
            wrap_and_unwrap_sol: wrap_unwrap_sol,
            use_shared_accounts,
            fee_account: None,
            tracking_account: None,
            compute_unit_price_micro_lamports: compute_unit_price,
            prioritization_fee_lamports: priority_fee,
            as_legacy_transaction: self.config.as_legacy_transaction,
            use_token_ledger: self.config.use_token_ledger,
            destination_token_account: None,
        };

        let url = format!("{}/swap", JUPITER_API_BASE);

        let response = self.client
            .post(&url)
            .json(&swap_request)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SwapError::RateLimited(SwapProvider::Jupiter));
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Jupiter,
                    format!("HTTP {}: {}", status, error_text)
                )
            );
        }

        let jupiter_tx: JupiterSwapResponse = response
            .json().await
            .map_err(|e| SwapError::TransactionFailed(SwapProvider::Jupiter, e.to_string()))?;

        Ok(SwapTransaction {
            provider: SwapProvider::Jupiter,
            quote: quote.clone(),
            serialized_transaction: jupiter_tx.swap_transaction,
            last_valid_block_height: Some(jupiter_tx.last_valid_block_height),
            recent_blockhash: None,
            compute_unit_limit: Some(jupiter_tx.compute_unit_limit),
            priority_fee: jupiter_tx.prioritization_fee_lamports,
        })
    }

    fn convert_jupiter_quote_to_swap_quote(
        &self,
        jupiter_quote: JupiterQuoteResponse,
        input_mint: &Pubkey,
        output_mint: &Pubkey
    ) -> SwapResult<SwapQuote> {
        let in_amount = jupiter_quote.in_amount
            .parse::<u64>()
            .map_err(|e| SwapError::InvalidAmount(e.to_string()))?;

        let out_amount = jupiter_quote.out_amount
            .parse::<u64>()
            .map_err(|e| SwapError::InvalidAmount(e.to_string()))?;

        let price_impact_pct = jupiter_quote.price_impact_pct.parse::<f64>().unwrap_or(0.0);

        // Validate price impact
        if price_impact_pct > 5.0 {
            return Err(SwapError::PriceImpactTooHigh(price_impact_pct));
        }

        let raw_response = serde_json
            ::to_value(&jupiter_quote)
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Jupiter, e.to_string()))?;

        Ok(SwapQuote {
            provider: SwapProvider::Jupiter,
            input_mint: *input_mint,
            output_mint: *output_mint,
            in_amount,
            out_amount,
            price_impact_pct,
            slippage_bps: jupiter_quote.slippage_bps,
            route_steps: jupiter_quote.route_plan.len() as u32,
            estimated_fee: 5000, // Jupiter default fee
            compute_unit_limit: None,
            priority_fee: 1000, // Default priority fee
            raw_response,
        })
    }

    pub async fn get_token_info(&self, mint: &Pubkey) -> SwapResult<Option<TokenInfo>> {
        // Jupiter doesn't have a direct token info endpoint in v6
        // This would typically require integration with a token list or metadata service
        // For now, return None to indicate the information is not available from Jupiter
        Ok(None)
    }

    pub fn is_available(&self) -> bool {
        self.config.enabled
    }

    pub async fn health_check(&self) -> Result<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        let url = format!("{}/quote", JUPITER_API_BASE);
        let sol_mint = "So11111111111111111111111111111111111111112";
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

        let query_params = [
            ("inputMint", sol_mint),
            ("outputMint", usdc_mint),
            ("amount", "1000000"), // 0.001 SOL
            ("slippageBps", "100"),
        ];

        let response = self.client
            .get(&url)
            .query(&query_params)
            .timeout(Duration::from_secs(5))
            .send().await?;

        Ok(response.status().is_success())
    }
}
