use async_trait::async_trait;
use std::error::Error;
use super::super::types::*;
use super::super::traits::{ SwapProvider, ProviderConfig };
use anyhow::Result;
use chrono::{ DateTime, Utc };
use serde_json::Value as JsonValue;
use tokio::time::{ sleep, Duration, Instant };
use reqwest::Client;
use std::collections::HashMap;

/// GMGN-specific swap provider
pub struct GmgnProvider {
    client: Client,
    base_url: String,
    config: ProviderConfig,
}

#[async_trait]
impl SwapProvider for GmgnProvider {
    fn id(&self) -> &str {
        "gmgn"
    }

    async fn get_quote(&self, request: &SwapRequest) -> Result<SwapQuote> {
        let start_time = Instant::now();

        // Build quote request based on swap type
        let quote_data = match request.swap_type {
            SwapType::Buy => self.get_buy_quote(request).await?,
            SwapType::Sell => self.get_sell_quote(request).await?,
        };

        let quote_time_ms = start_time.elapsed().as_millis() as u64;

        // Convert GMGN response to standard quote format
        let quote = SwapQuote {
            provider_id: "gmgn".to_string(),
            request_id: request.request_id.clone(),
            in_amount: request.amount_in,
            out_amount: quote_data
                .get("out_amount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            in_decimals: if request.swap_type == SwapType::Buy {
                9
            } else {
                6
            }, // SOL or token decimals
            out_decimals: if request.swap_type == SwapType::Buy {
                6
            } else {
                9
            }, // Token or SOL decimals
            price_impact_bps: quote_data
                .get("price_impact_bps")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as u16,
            fee_amount: quote_data
                .get("fee_amount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            minimum_out_amount: quote_data
                .get("minimum_out_amount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            route_info: RouteInfo {
                route_steps: vec![RouteStep {
                    amm_id: "gmgn".to_string(),
                    amm_label: "GMGN".to_string(),
                    percent: 100,
                    in_amount: request.amount_in,
                    out_amount: quote_data
                        .get("out_amount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    fee_amount: quote_data
                        .get("fee_amount")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                }],
                total_fee_bps: self.config.default_fee_bps,
                price_impact_pct: quote_data
                    .get("price_impact_pct")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0),
                liquidity_sources: vec!["GMGN".to_string()],
            },
            estimated_gas: 100000, // Default estimate
            valid_until: Utc::now() + chrono::Duration::minutes(5),
            quote_time_ms,
        };

        Ok(quote)
    }

    async fn execute_swap(&self, request: &SwapRequest, quote: &SwapQuote) -> Result<SwapResult> {
        let start_time = Instant::now();

        let result = match request.swap_type {
            SwapType::Buy => self.execute_buy(request, quote).await,
            SwapType::Sell => self.execute_sell(request, quote).await,
        };

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        match result {
            Ok(signature) => {
                Ok(
                    SwapResult::new_success(
                        request.clone(),
                        quote.clone(),
                        signature,
                        "gmgn".to_string(),
                        execution_time_ms
                    )
                )
            }
            Err(e) => {
                Ok(
                    SwapResult::new_error(
                        request.clone(),
                        SwapError::ProviderError(e.to_string()),
                        "gmgn".to_string(),
                        execution_time_ms
                    )
                )
            }
        }
    }

    async fn get_transaction_status(&self, signature: &str) -> Result<TransactionStatus> {
        let url = format!("{}/transaction/{}/status", self.base_url, signature);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(status_data) = response.json::<JsonValue>().await {
                        let status_str = status_data
                            .get("status")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");

                        match status_str {
                            "confirmed" => Ok(TransactionStatus::Confirmed),
                            "pending" => Ok(TransactionStatus::Pending),
                            "failed" => Ok(TransactionStatus::Failed),
                            "expired" => Ok(TransactionStatus::Expired),
                            _ => Ok(TransactionStatus::Unknown),
                        }
                    } else {
                        Ok(TransactionStatus::Unknown)
                    }
                } else {
                    Ok(TransactionStatus::Unknown)
                }
            }
            Err(_) => Ok(TransactionStatus::Unknown),
        }
    }

    fn supports_token_pair(&self, token_in: &str, token_out: &str) -> bool {
        // GMGN supports SOL <-> SPL token swaps
        let wsol = "So11111111111111111111111111111111111111112";

        (token_in == wsol && token_out != wsol) || (token_in != wsol && token_out == wsol)
    }

    fn get_config(&self) -> &ProviderConfig {
        &self.config
    }
}

impl GmgnProvider {
    pub fn new(config: Option<ProviderConfig>) -> Self {
        let config = config.unwrap_or_default();

        Self {
            client: Client::new(),
            base_url: "https://gmgn.ai/api".to_string(),
            config,
        }
    }

    async fn get_buy_quote(&self, request: &SwapRequest) -> Result<JsonValue> {
        let url = format!("{}/quote/buy", self.base_url);

        let payload =
            serde_json::json!({
            "mint": request.token_out_address,
            "amount": request.amount_in,
            "slippage": request.slippage_bps,
            "chain": "solana"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send().await
            .map_err(|e| anyhow::anyhow!("Network error: {}", e))?;

        if response.status().is_success() {
            let quote_data: JsonValue = response
                .json().await
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
            Ok(quote_data)
        } else {
            Err(anyhow::anyhow!("GMGN quote failed: {}", response.status()))
        }
    }

    async fn get_sell_quote(&self, request: &SwapRequest) -> Result<JsonValue> {
        let url = format!("{}/quote/sell", self.base_url);

        let payload =
            serde_json::json!({
            "mint": request.token_in_address,
            "amount": request.amount_in,
            "slippage": request.slippage_bps,
            "chain": "solana"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send().await
            .map_err(|e| anyhow::anyhow!("Network error: {}", e))?;

        if response.status().is_success() {
            let quote_data: JsonValue = response
                .json().await
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
            Ok(quote_data)
        } else {
            Err(anyhow::anyhow!("GMGN quote failed: {}", response.status()))
        }
    }

    async fn execute_buy(&self, request: &SwapRequest, _quote: &SwapQuote) -> Result<String> {
        let url = format!("{}/swap/buy", self.base_url);

        let payload =
            serde_json::json!({
            "mint": request.token_out_address,
            "amount": request.amount_in,
            "slippage": request.slippage_bps,
            "wallet": request.from_address,
            "priorityFee": request.priority_fee_lamports,
            "chain": "solana"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send().await
            .map_err(|e| anyhow::anyhow!("Network error: {}", e))?;

        if response.status().is_success() {
            let swap_data: JsonValue = response
                .json().await
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

            if let Some(signature) = swap_data.get("signature").and_then(|v| v.as_str()) {
                Ok(signature.to_string())
            } else {
                Err(anyhow::anyhow!("No transaction signature in response"))
            }
        } else {
            Err(anyhow::anyhow!("GMGN buy failed: {}", response.status()))
        }
    }

    async fn execute_sell(&self, request: &SwapRequest, _quote: &SwapQuote) -> Result<String> {
        let url = format!("{}/swap/sell", self.base_url);

        let payload =
            serde_json::json!({
            "mint": request.token_in_address,
            "amount": request.amount_in,
            "slippage": request.slippage_bps,
            "wallet": request.from_address,
            "priorityFee": request.priority_fee_lamports,
            "chain": "solana"
        });

        let response = self.client
            .post(&url)
            .json(&payload)
            .send().await
            .map_err(|e| anyhow::anyhow!("Network error: {}", e))?;

        if response.status().is_success() {
            let swap_data: JsonValue = response
                .json().await
                .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

            if let Some(signature) = swap_data.get("signature").and_then(|v| v.as_str()) {
                Ok(signature.to_string())
            } else {
                Err(anyhow::anyhow!("No transaction signature in response"))
            }
        } else {
            Err(anyhow::anyhow!("GMGN sell failed: {}", response.status()))
        }
    }

    /// GMGN-specific detailed buy function (keeping original interface for compatibility)
    pub async fn buy_gmgn_detailed(
        &self,
        mint: &str,
        amount_sol: f64,
        slippage: u16,
        wallet_address: &str,
        priority_fee: u64
    ) -> Result<SwapResult> {
        let amount_lamports = (amount_sol * 1_000_000_000.0) as u64;

        let request = SwapRequest::new_buy(
            mint,
            amount_lamports,
            wallet_address,
            slippage,
            priority_fee
        );

        let quote = self.get_quote(&request).await?;
        let result = self.execute_swap(&request, &quote).await?;

        Ok(result)
    }

    /// GMGN-specific detailed sell function (keeping original interface for compatibility)
    pub async fn sell_gmgn_detailed(
        &self,
        mint: &str,
        amount_tokens: u64,
        slippage: u16,
        wallet_address: &str,
        priority_fee: u64
    ) -> Result<SwapResult> {
        let request = SwapRequest::new_sell(
            mint,
            amount_tokens,
            wallet_address,
            slippage,
            priority_fee
        );

        let quote = self.get_quote(&request).await?;
        let result = self.execute_swap(&request, &quote).await?;

        Ok(result)
    }

    /// Wait for transaction confirmation
    pub async fn wait_for_confirmation(
        &self,
        signature: &str,
        timeout_secs: u64
    ) -> Result<TransactionStatus> {
        let start_time = Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs);

        while start_time.elapsed() < timeout_duration {
            let status = self.get_transaction_status(signature).await?;

            match status {
                | TransactionStatus::Confirmed
                | TransactionStatus::Failed
                | TransactionStatus::Expired => {
                    return Ok(status);
                }
                TransactionStatus::Pending | TransactionStatus::Unknown => {
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }

        Ok(TransactionStatus::Expired)
    }
}
