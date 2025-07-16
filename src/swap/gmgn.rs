use super::types::*;
use crate::config::GmgnConfig;
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
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use base64::{ Engine as _, engine::general_purpose };
use bincode;

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

    pub async fn get_quote(
        &self,
        input_mint: &Pubkey,
        output_mint: &Pubkey,
        amount: u64,
        slippage_bps: u16
    ) -> SwapResult<SwapQuote> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Gmgn));
        }

        // Convert slippage from bps to percentage for GMGN
        let slippage_percent = (slippage_bps as f64) / 100.0;

        // Use a dummy address for getting quotes
        let dummy_address = "11111111111111111111111111111112";

        let mut params = HashMap::new();
        params.insert("token_in_address", input_mint.to_string());
        params.insert("token_out_address", output_mint.to_string());
        params.insert("in_amount", amount.to_string());
        params.insert("from_address", dummy_address.to_string());
        params.insert("slippage", slippage_percent.to_string());
        params.insert("swap_mode", "ExactIn".to_string());
        params.insert("fee", "0.001".to_string());
        params.insert("is_anti_mev", "false".to_string());

        let url = format!("{}/get_swap_route", self.config.api_url);

        let response = self.client
            .get(&url)
            .query(&params)
            .timeout(Duration::from_secs(10))
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
                    format!("GMGN API error: {} - {}", gmgn_response.code, gmgn_response.msg)
                )
            );
        }

        self.convert_gmgn_quote_to_swap_quote(&gmgn_response, input_mint, output_mint)
    }

    pub async fn get_swap_transaction(
        &self,
        user_public_key: &Pubkey,
        quote: &SwapQuote,
        wrap_sol: bool,
        unwrap_sol: bool,
        priority_fee: Option<u64>
    ) -> SwapResult<SwapTransaction> {
        if !self.config.enabled {
            return Err(SwapError::ProviderNotAvailable(SwapProvider::Gmgn));
        }

        // GMGN provides the transaction in the quote response, so we need to get it again with real user address
        let slippage_percent = (quote.slippage_bps as f64) / 100.0;

        let mut params = HashMap::new();
        params.insert("token_in_address", quote.input_mint.to_string());
        params.insert("token_out_address", quote.output_mint.to_string());
        params.insert("in_amount", quote.in_amount.to_string());
        params.insert("from_address", user_public_key.to_string());
        params.insert("slippage", slippage_percent.to_string());
        params.insert("swap_mode", "ExactIn".to_string());
        params.insert("fee", "0.001".to_string());
        params.insert("is_anti_mev", "false".to_string());

        let url = format!("{}/get_swap_route", self.config.api_url);

        let response = self.client
            .get(&url)
            .query(&params)
            .timeout(Duration::from_secs(10))
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("HTTP {}: {}", status, error_text)
                )
            );
        }

        let gmgn_response: GmgnQuoteResponse = response
            .json().await
            .map_err(|e| SwapError::TransactionFailed(SwapProvider::Gmgn, e.to_string()))?;

        if gmgn_response.code != 0 {
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("GMGN API error: {} - {}", gmgn_response.code, gmgn_response.msg)
                )
            );
        }

        self.convert_gmgn_transaction(&gmgn_response, quote)
    }

    pub async fn execute_swap(
        &self,
        transaction: &SwapTransaction,
        keypair: &Keypair,
        rpc_client: &RpcClient
    ) -> SwapResult<Signature> {
        // Decode the base64 transaction
        let transaction_bytes = general_purpose::STANDARD
            .decode(&transaction.serialized_transaction)
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("Failed to decode transaction: {}", e)
                )
            )?;

        // Deserialize into VersionedTransaction
        let mut versioned_transaction: VersionedTransaction = bincode
            ::deserialize(&transaction_bytes)
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("Failed to deserialize transaction: {}", e)
                )
            )?;

        // Get the latest blockhash and update transaction
        let blockhash = rpc_client
            .get_latest_blockhash()
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("Failed to get latest blockhash: {}", e)
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
                    SwapProvider::Gmgn,
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
                    SwapProvider::Gmgn,
                    format!("Failed to serialize signed transaction: {}", e)
                )
            )?;
        let signed_transaction_base64 = general_purpose::STANDARD.encode(&signed_transaction_bytes);

        // Send using RPC
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
            .timeout(Duration::from_secs(30))
            .send().await
            .map_err(|e|
                SwapError::TransactionFailed(SwapProvider::Gmgn, format!("HTTP error: {}", e))
            )?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("HTTP error: {}", error_text)
                )
            );
        }

        let response_json: serde_json::Value = response
            .json().await
            .map_err(|e|
                SwapError::TransactionFailed(
                    SwapProvider::Gmgn,
                    format!("Failed to parse response: {}", e)
                )
            )?;

        if let Some(error) = response_json.get("error") {
            return Err(
                SwapError::TransactionFailed(SwapProvider::Gmgn, format!("RPC error: {}", error))
            );
        }

        if let Some(result) = response_json.get("result") {
            if let Some(signature_str) = result.as_str() {
                let transaction_signature = signature_str
                    .parse::<Signature>()
                    .map_err(|e|
                        SwapError::TransactionFailed(
                            SwapProvider::Gmgn,
                            format!("Failed to parse signature: {}", e)
                        )
                    )?;

                return Ok(transaction_signature);
            }
        }

        Err(SwapError::TransactionFailed(SwapProvider::Gmgn, "Invalid response format".to_string()))
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

        // Increase price impact tolerance for GMGN (similar to other DEXs)
        if price_impact_pct > 10.0 {
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

        // GMGN health check with a small test quote using the correct endpoint
        let sol_mint = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let usdc_mint = Pubkey::from_str("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v").unwrap();
        let dummy_address = "11111111111111111111111111111112";

        let mut params = HashMap::new();
        params.insert("token_in_address", sol_mint.to_string());
        params.insert("token_out_address", usdc_mint.to_string());
        params.insert("in_amount", "1000000".to_string()); // 0.001 SOL
        params.insert("from_address", dummy_address.to_string());
        params.insert("slippage", "1.0".to_string());
        params.insert("swap_mode", "ExactIn".to_string());
        params.insert("fee", "0.001".to_string());
        params.insert("is_anti_mev", "false".to_string());

        let url = format!("{}/get_swap_route", self.config.api_url);

        match self.client.get(&url).query(&params).timeout(Duration::from_secs(10)).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    // Try to parse the response to ensure it's valid
                    match response.json::<GmgnQuoteResponse>().await {
                        Ok(gmgn_response) => Ok(gmgn_response.code == 0),
                        Err(_) => Ok(false), // Invalid response format
                    }
                } else {
                    Ok(false)
                }
            }
            Err(_) => Ok(false), // Network error or timeout
        }
    }

    pub async fn get_supported_tokens(&self) -> SwapResult<Vec<TokenInfo>> {
        // GMGN doesn't provide a standard supported tokens endpoint
        // This would need to be implemented based on their specific API
        Ok(vec![])
    }
}
