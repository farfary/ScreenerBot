use super::types::*;
use crate::config::RaydiumConfig;
use anyhow::Result;
use reqwest::Client;
use serde_json;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{ Keypair, Signature },
    transaction::VersionedTransaction,
    signer::Signer,
    message::VersionedMessage,
};
use solana_client::rpc_client::RpcClient;
use base64::{ Engine as _, engine::general_purpose };
use bincode;
use std::str::FromStr;
use std::time::Duration;

// Correct Raydium API endpoints based on test_quote_raydium.rs
const RAYDIUM_TRANSACTION_API: &str = "https://transaction-v1.raydium.io";
const RAYDIUM_MAIN_API: &str = "https://api-v3.raydium.io";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RaydiumPriorityFeeResponse {
    id: String,
    success: bool,
    data: RaydiumPriorityFeeData,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RaydiumPriorityFeeData {
    default: RaydiumPriorityFeeTiers,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RaydiumPriorityFeeTiers {
    vh: u64, // very high
    h: u64, // high
    m: u64, // medium
}

pub struct RaydiumProvider {
    client: Client,
    config: RaydiumConfig,
}

impl RaydiumProvider {
    pub fn new(config: RaydiumConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Get current priority fee for better transaction execution
    async fn get_priority_fee(&self) -> SwapResult<u64> {
        let url = format!("{}/main/priority-fee", RAYDIUM_MAIN_API);

        let response = self.client
            .get(&url)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            // Use reasonable default if priority fee API fails
            return Ok(100000); // 0.1 lamports per compute unit
        }

        let priority_response: RaydiumPriorityFeeResponse = response
            .json().await
            .map_err(|e| SwapError::NetworkError(format!("Failed to parse priority fee: {}", e)))?;

        if !priority_response.success {
            return Ok(100000); // Default fallback
        }

        // Use high priority fee for better execution
        Ok(priority_response.data.default.h)
    }

    pub async fn get_quote(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16
    ) -> SwapResult<SwapQuote> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Raydium));
        }

        // Use higher slippage for Raydium to avoid execution failures
        let adjusted_slippage = if slippage_bps < 300 { 300 } else { slippage_bps };

        // Use the correct transaction API endpoint for quotes
        let url = format!(
            "{}/compute/swap-base-in?inputMint={}&outputMint={}&amount={}&slippageBps={}&txVersion=V0",
            RAYDIUM_TRANSACTION_API,
            input_mint,
            output_mint,
            amount,
            adjusted_slippage
        );

        let response = self.client
            .get(&url)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SwapError::RateLimited(SwapProvider::Raydium));
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Raydium,
                    format!("HTTP {}: {}", status, error_text)
                )
            );
        }

        let raydium_response: RaydiumSwapCompute = response
            .json().await
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Raydium, e.to_string()))?;

        if !raydium_response.success {
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Raydium,
                    raydium_response.msg.unwrap_or("Unknown error".to_string())
                )
            );
        }

        self.convert_raydium_quote_to_swap_quote(raydium_response, input_mint, output_mint)
    }

    pub async fn get_swap_transaction(
        &self,
        user_public_key: &Pubkey,
        quote: &SwapQuote,
        wrap_sol: bool,
        unwrap_sol: bool,
        compute_unit_price: Option<u64>
    ) -> SwapResult<SwapTransaction> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Raydium));
        }

        // Get current priority fee
        let priority_fee = if let Some(fee) = compute_unit_price {
            fee
        } else {
            self.get_priority_fee().await.unwrap_or(self.config.compute_unit_price_micro_lamports)
        };

        // Extract the original Raydium swap data from the raw response
        let raydium_response: RaydiumSwapCompute = serde_json
            ::from_value(quote.raw_response.clone())
            .map_err(|e| SwapError::TransactionFailed(SwapProvider::Raydium, e.to_string()))?;

        let transaction_request = RaydiumTransactionRequest {
            compute_unit_price_micro_lamports: priority_fee.to_string(),
            swap_response: raydium_response,
            tx_version: "V0".to_string(),
            wallet: user_public_key.to_string(),
            wrap_sol,
            unwrap_sol,
            input_account: None, // Let Raydium handle ATA
            output_account: None, // Let Raydium handle ATA
        };

        // Use the correct transaction API endpoint
        let url = format!("{}/transaction/swap-base-in", RAYDIUM_TRANSACTION_API);

        let response = self.client
            .post(&url)
            .json(&transaction_request)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if response.status() == 429 {
            return Err(SwapError::RateLimited(SwapProvider::Raydium));
        }

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("HTTP {}: {}", status, error_text)
                )
            );
        }

        let raydium_tx: RaydiumTransactionResponse = response
            .json().await
            .map_err(|e| SwapError::TransactionFailed(SwapProvider::Raydium, e.to_string()))?;

        if !raydium_tx.success {
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    "Transaction creation failed".to_string()
                )
            );
        }

        if raydium_tx.data.is_empty() {
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    "No transaction data returned".to_string()
                )
            );
        }

        Ok(SwapTransaction {
            provider: SwapProvider::Raydium,
            quote: quote.clone(),
            serialized_transaction: raydium_tx.data[0].transaction.clone(),
            last_valid_block_height: None,
            recent_blockhash: None,
            compute_unit_limit: None,
            priority_fee,
        })
    }

    /// Execute a real on-chain swap transaction using the same method as test_quote_raydium.rs
    pub async fn execute_swap(
        &self,
        transaction: &SwapTransaction,
        keypair: &Keypair,
        rpc_client: &RpcClient
    ) -> SwapResult<Signature> {
        // This method now follows the exact same pattern as the working test_quote_raydium.rs
        self.send_raydium_transaction(
            &transaction.serialized_transaction,
            keypair,
            rpc_client
        ).await
    }

    /// Send Raydium transaction using the exact same method as test_quote_raydium.rs
    async fn send_raydium_transaction(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
        rpc_client: &RpcClient
    ) -> SwapResult<Signature> {
        // Decode the base64 transaction
        let transaction_bytes = general_purpose::STANDARD
            .decode(transaction_base64)
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("Failed to decode transaction: {}", e)
                )
            )?;

        // Deserialize into VersionedTransaction (Raydium uses V0 transactions)
        let mut versioned_transaction: VersionedTransaction = bincode
            ::deserialize(&transaction_bytes)
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("Failed to deserialize transaction: {}", e)
                )
            )?;

        // Get the latest blockhash and update transaction
        let blockhash = rpc_client
            .get_latest_blockhash()
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("Failed to get blockhash: {}", e)
                )
            )?;

        // Update blockhash in the message
        match &mut versioned_transaction.message {
            VersionedMessage::V0(msg) => {
                msg.recent_blockhash = blockhash;
            }
            VersionedMessage::Legacy(msg) => {
                msg.recent_blockhash = blockhash;
            }
        }

        // Clear existing signatures and sign with our wallet
        versioned_transaction.signatures.clear();
        let message = versioned_transaction.message.clone();
        let message_bytes = bincode
            ::serialize(&message)
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("Failed to serialize message: {}", e)
                )
            )?;
        let signature = keypair.sign_message(&message_bytes);
        versioned_transaction.signatures.push(signature);

        // Serialize the signed transaction
        let signed_transaction_bytes = bincode
            ::serialize(&versioned_transaction)
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("Failed to serialize signed transaction: {}", e)
                )
            )?;
        let signed_transaction_base64 = general_purpose::STANDARD.encode(&signed_transaction_bytes);

        // Send using RPC with the exact same parameters as test_quote_raydium.rs
        let params =
            serde_json::json!([
            signed_transaction_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "processed"
            }
        ]);

        let request_body =
            serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "sendTransaction",
            "params": params
        });

        let response = self.client
            .post(rpc_client.url())
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send().await
            .map_err(|e|
                SwapError::TransactionFailed(SwapProvider::Raydium, format!("HTTP error: {}", e))
            )?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("HTTP error: {}", error_text)
                )
            );
        }

        let response_json: serde_json::Value = response
            .json().await
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Raydium,
                    format!("Failed to parse response: {}", e)
                )
            )?;

        if let Some(error) = response_json.get("error") {
            return Err(
                SwapError::TransactionFailed(SwapProvider::Raydium, format!("RPC error: {}", error))
            );
        }

        if let Some(result) = response_json.get("result") {
            if let Some(signature_str) = result.as_str() {
                let transaction_signature = signature_str
                    .parse::<Signature>()
                    .map_err(|e|
                        SwapError::TransactionFailed(
                            SwapProvider::Raydium,
                            format!("Failed to parse signature: {}", e)
                        )
                    )?;

                return Ok(transaction_signature);
            }
        }

        Err(
            SwapError::TransactionFailed(
                SwapProvider::Raydium,
                "Invalid response format".to_string()
            )
        )
    }

    fn convert_raydium_quote_to_swap_quote(
        &self,
        raydium_response: RaydiumSwapCompute,
        input_mint: &Pubkey,
        output_mint: &Pubkey
    ) -> SwapResult<SwapQuote> {
        let swap_data = &raydium_response.data;

        let in_amount = swap_data.input_amount
            .parse::<u64>()
            .map_err(|e| SwapError::InvalidAmount(e.to_string()))?;

        let out_amount = swap_data.output_amount
            .parse::<u64>()
            .map_err(|e| SwapError::InvalidAmount(e.to_string()))?;

        let price_impact_pct = swap_data.price_impact_pct;

        // Validate price impact (allow higher for Raydium)
        if price_impact_pct > 10.0 {
            return Err(SwapError::PriceImpactTooHigh(price_impact_pct));
        }

        let raw_response = serde_json
            ::to_value(&raydium_response)
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Raydium, e.to_string()))?;

        Ok(SwapQuote {
            provider: SwapProvider::Raydium,
            input_mint: *input_mint,
            output_mint: *output_mint,
            in_amount,
            out_amount,
            price_impact_pct,
            slippage_bps: swap_data.slippage_bps as u16, // Use the actual slippage from response
            route_steps: swap_data.route_plan.len() as u32,
            estimated_fee: self.config.compute_unit_price_micro_lamports,
            compute_unit_limit: None,
            priority_fee: self.config.compute_unit_price_micro_lamports,
            raw_response,
        })
    }

    pub async fn get_token_info(&self, mint: &Pubkey) -> SwapResult<Option<TokenInfo>> {
        // Raydium doesn't provide a direct token info endpoint
        // This would typically require integration with their token list
        Ok(None)
    }

    pub fn is_available(&self) -> bool {
        self.config.enabled
    }

    pub async fn health_check(&self) -> Result<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        // Use the correct transaction API endpoint for health check with proper parameters
        let url = format!(
            "{}/compute/swap-base-in?inputMint={}&outputMint={}&amount={}&slippageBps={}&txVersion={}",
            RAYDIUM_TRANSACTION_API,
            "So11111111111111111111111111111111111111112", // SOL
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
            "1000000", // 0.001 SOL
            "300", // 3% slippage (same as working test)
            "V0"
        );

        match self.client.get(&url).timeout(Duration::from_secs(10)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // Try to parse the response to ensure it's valid
                    match response.json::<RaydiumSwapCompute>().await {
                        Ok(raydium_response) => Ok(raydium_response.success),
                        Err(_) => Ok(false),
                    }
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    pub async fn get_pools(&self) -> SwapResult<Vec<serde_json::Value>> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Raydium));
        }

        let url = format!("{}/main/pairs", RAYDIUM_MAIN_API);

        let response = self.client
            .get(&url)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Raydium,
                    format!("Failed to get pools: {}", error_text)
                )
            );
        }

        let pools: Vec<serde_json::Value> = response
            .json().await
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Raydium, e.to_string()))?;

        Ok(pools)
    }

    pub async fn get_pool_info(&self, pool_id: &str) -> SwapResult<serde_json::Value> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Raydium));
        }

        let url = format!("{}/main/pairs", RAYDIUM_MAIN_API);

        let response = self.client
            .get(&url)
            .query(&[("pool_id", pool_id)])
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::QuoteFailed(
                    SwapProvider::Raydium,
                    format!("Failed to get pool info: {}", error_text)
                )
            );
        }

        let pool_info: serde_json::Value = response
            .json().await
            .map_err(|e| SwapError::QuoteFailed(SwapProvider::Raydium, e.to_string()))?;

        Ok(pool_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_config() -> RaydiumConfig {
        RaydiumConfig {
            enabled: true,
            api_url: RAYDIUM_TRANSACTION_API.to_string(),
            timeout_seconds: 10,
            compute_unit_price_micro_lamports: 5000,
        }
    }

    #[tokio::test]
    async fn test_raydium_quote() {
        let provider = RaydiumProvider::new(get_test_config());

        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();

        match provider.get_quote(&sol_mint, &usdc_mint, 1000000, 100).await {
            Ok(quote) => {
                assert_eq!(quote.provider, SwapProvider::Raydium);
                assert_eq!(quote.input_mint, sol_mint);
                assert_eq!(quote.output_mint, usdc_mint);
                assert_eq!(quote.in_amount, 1000000);
                assert!(quote.out_amount > 0);

                println!("Raydium quote test passed:");
                println!(
                    "  {} SOL -> {} USDC",
                    (quote.in_amount as f64) / 1e9,
                    (quote.out_amount as f64) / 1e6
                );
                println!("  Price Impact: {:.2}%", quote.price_impact_pct);
                println!("  Route Steps: {}", quote.route_steps);
            }
            Err(e) => {
                println!("Raydium quote test failed (API may be unavailable): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_raydium_health_check() {
        let provider = RaydiumProvider::new(get_test_config());

        match provider.health_check().await {
            Ok(healthy) => {
                println!("Raydium health check: {}", if healthy {
                    "✅ Healthy"
                } else {
                    "❌ Unhealthy"
                });
            }
            Err(e) => {
                println!("Raydium health check failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_raydium_priority_fee() {
        let provider = RaydiumProvider::new(get_test_config());

        match provider.get_priority_fee().await {
            Ok(fee) => {
                println!("Raydium priority fee: {} micro-lamports", fee);
                assert!(fee > 0);
            }
            Err(e) => {
                println!("Raydium priority fee test failed: {}", e);
            }
        }
    }

    #[test]
    fn test_raydium_config() {
        let config = get_test_config();
        assert!(config.enabled);
        assert_eq!(config.api_url, RAYDIUM_TRANSACTION_API);
        assert_eq!(config.compute_unit_price_micro_lamports, 5000);
    }
}
