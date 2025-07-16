use super::types::*;
use crate::config::GmgnConfig;
use anyhow::Result;
use reqwest::Client;
use serde_json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::time::Duration;

pub struct GmgnProvider {
    client: Client,
    config: GmgnConfig,
}

impl GmgnProvider {
    pub fn new(config: GmgnConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    pub async fn get_quote_and_transaction(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16,
        user_public_key: &Pubkey
    ) -> SwapResult<(SwapQuote, SwapTransaction)> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Gmgn));
        }

        let slippage_percent = (slippage_bps as f64) / 100.0;

        let quote_request = GmgnQuoteRequest {
            token_in_address: input_mint.to_string(),
            token_out_address: output_mint.to_string(),
            in_amount: amount.to_string(),
            from_address: user_public_key.to_string(),
            slippage: slippage_percent,
            swap_mode: self.config.swap_mode.clone(),
            fee: self.config.fee,
            is_anti_mev: self.config.anti_mev,
        };

        let url = format!("{}/swap", self.config.api_url);

        let response = self.client
            .post(&url)
            .json(&quote_request)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SwapError::RateLimited(SwapProvider::Gmgn));
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Gmgn,
                    format!("HTTP {}: {}", status, error_text)
                )
            );
        }

        let gmgn_response: GmgnQuoteResponse = response
            .json().await
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Gmgn, e.to_string()))?;

        if gmgn_response.code != 0 {
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Gmgn,
                    format!("GMGN API error: {}", gmgn_response.msg)
                )
            );
        }

        let quote = self.convert_gmgn_quote_to_swap_quote(&gmgn_response, input_mint, output_mint)?;
        let transaction = self.convert_gmgn_transaction(&gmgn_response, &quote)?;

        Ok((quote, transaction))
    }

    fn convert_gmgn_quote_to_swap_quote(
        &self,
        gmgn_response: &GmgnQuoteResponse,
        input_mint: &Pubkey,
        output_mint: &Pubkey
    ) -> SwapResult<SwapQuote> {
        let quote = &gmgn_response.data.quote;

        let in_amount = quote.in_amount
            .parse::<u64>()
            .map_err(|e| SwapError::InvalidAmount(e.to_string()))?;

        let out_amount = quote.out_amount
            .parse::<u64>()
            .map_err(|e| SwapError::InvalidAmount(e.to_string()))?;

        let price_impact_pct = quote.price_impact_pct.parse::<f64>().unwrap_or(0.0);

        // Validate price impact
        if price_impact_pct > 5.0 {
            return Err(SwapError::PriceImpactTooHigh(price_impact_pct));
        }

        let slippage_bps = quote.slippage_bps.parse::<u16>().unwrap_or(100);

        let raw_response = serde_json
            ::to_value(&gmgn_response)
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Gmgn, e.to_string()))?;

        Ok(SwapQuote {
            provider: SwapProvider::Gmgn,
            input_mint: *input_mint,
            output_mint: *output_mint,
            in_amount,
            out_amount,
            price_impact_pct,
            slippage_bps,
            route_steps: quote.route_plan.len() as u32,
            estimated_fee: gmgn_response.data.raw_tx.prioritization_fee_lamports,
            compute_unit_limit: None,
            priority_fee: gmgn_response.data.raw_tx.prioritization_fee_lamports,
            raw_response,
        })
    }

    fn convert_gmgn_transaction(
        &self,
        gmgn_response: &GmgnQuoteResponse,
        quote: &SwapQuote
    ) -> SwapResult<SwapTransaction> {
        let raw_tx = &gmgn_response.data.raw_tx;

        Ok(SwapTransaction {
            provider: SwapProvider::Gmgn,
            quote: quote.clone(),
            serialized_transaction: raw_tx.swap_transaction.clone(),
            last_valid_block_height: Some(raw_tx.last_valid_block_height),
            recent_blockhash: Some(raw_tx.recent_blockhash.clone()),
            compute_unit_limit: None,
            priority_fee: raw_tx.prioritization_fee_lamports,
        })
    }

    pub async fn get_token_info(&self, mint: &Pubkey) -> SwapResult<Option<TokenInfo>> {
        // GMGN doesn't provide a separate token info endpoint
        // This would require integration with their token data API if available
        Ok(None)
    }

    pub fn is_available(&self) -> bool {
        self.config.enabled
    }

    pub async fn health_check(&self) -> Result<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        // GMGN health check with a small test quote
        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let test_wallet = Pubkey::from_str("11111111111111111111111111111112").unwrap(); // Dummy wallet for health check

        let quote_request = GmgnQuoteRequest {
            token_in_address: sol_mint.to_string(),
            token_out_address: usdc_mint.to_string(),
            in_amount: "1000000".to_string(), // 0.001 SOL
            from_address: test_wallet.to_string(),
            slippage: 1.0,
            swap_mode: "ExactIn".to_string(),
            fee: 0.001,
            is_anti_mev: false,
        };

        let url = format!("{}/swap", self.config.api_url);

        let response = self.client
            .post(&url)
            .json(&quote_request)
            .timeout(Duration::from_secs(5))
            .send().await?;

        Ok(response.status().is_success())
    }

    pub async fn get_supported_tokens(&self) -> SwapResult<Vec<TokenInfo>> {
        // GMGN doesn't provide a standard supported tokens endpoint
        // This would need to be implemented based on their specific API
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_config() -> GmgnConfig {
        GmgnConfig {
            enabled: true,
            api_url: "https://gmgn.ai/api/v1/sol".to_string(),
            timeout_seconds: 10,
            swap_mode: "ExactIn".to_string(),
            fee: 0.001,
            anti_mev: false,
        }
    }

    #[tokio::test]
    async fn test_gmgn_quote_and_transaction() {
        let provider = GmgnProvider::new(get_test_config());

        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let user_wallet = Pubkey::from_str("B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59").unwrap();

        match
            provider.get_quote_and_transaction(
                &sol_mint,
                &usdc_mint,
                1000000,
                100,
                &user_wallet
            ).await
        {
            Ok((quote, transaction)) => {
                assert_eq!(quote.provider, SwapProvider::Gmgn);
                assert_eq!(quote.input_mint, sol_mint);
                assert_eq!(quote.output_mint, usdc_mint);
                assert_eq!(quote.in_amount, 1000000);
                assert!(quote.out_amount > 0);
                assert_eq!(transaction.provider, SwapProvider::Gmgn);
                assert!(!transaction.serialized_transaction.is_empty());

                println!("GMGN quote and transaction test passed:");
                println!(
                    "  {} SOL -> {} USDC",
                    (quote.in_amount as f64) / 1e9,
                    (quote.out_amount as f64) / 1e6
                );
                println!("  Price Impact: {:.2}%", quote.price_impact_pct);
                println!("  Priority Fee: {} lamports", transaction.priority_fee);
            }
            Err(e) => {
                println!("GMGN test failed (API may be unavailable): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_gmgn_health_check() {
        let provider = GmgnProvider::new(get_test_config());

        match provider.health_check().await {
            Ok(healthy) => {
                println!("GMGN health check: {}", if healthy {
                    "✅ Healthy"
                } else {
                    "❌ Unhealthy"
                });
            }
            Err(e) => {
                println!("GMGN health check failed: {}", e);
            }
        }
    }

    #[test]
    fn test_gmgn_config() {
        let config = get_test_config();
        assert!(config.enabled);
        assert_eq!(config.swap_mode, "ExactIn");
        assert_eq!(config.fee, 0.001);
        assert!(!config.anti_mev);
    }
}
