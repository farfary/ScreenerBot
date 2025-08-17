/// Raydium Router Implementation
/// Handles swap quotes and execution via Raydium Trade API
/// Based on Raydium Trade API documentation provided by user

use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::{SwapError, lamports_to_sol};
use crate::global::{is_debug_swap_enabled, is_debug_api_enabled, read_configs};
use crate::swaps::types::{SwapData, SwapQuote, RawTransaction};
use super::config::{
    SOL_MINT, QUOTE_SLIPPAGE_PERCENT, SWAP_FEE_PERCENT, QUOTE_TIMEOUT_SECS, 
    API_TIMEOUT_SECS, RETRY_ATTEMPTS, RAYDIUM_SWAP_API,
    TRANSACTION_CONFIRMATION_MAX_ATTEMPTS, TRANSACTION_CONFIRMATION_RETRY_DELAY_MS
};
use super::execution::{sign_and_send_transaction};
// Use utils for wallet address instead of transaction module  
use crate::utils::get_wallet_address;

use serde::{Deserialize, Serialize};
use reqwest;
use tokio::time::{Duration, timeout};

/// Raydium quote response structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RaydiumQuoteResponse {
    pub id: String,
    pub success: bool,
    pub version: String,
    pub data: RaydiumQuoteData,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RaydiumQuoteData {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inputAmount")]
    pub input_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outputAmount")]
    pub output_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u32,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: f64,
    #[serde(rename = "routePlan")]
    pub route_plan: Vec<RaydiumRoutePlan>,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RaydiumRoutePlan {
    #[serde(rename = "poolId")]
    pub pool_id: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
    #[serde(rename = "feeRate")]
    pub fee_rate: f64,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
}

/// Raydium swap transaction response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct RaydiumSwapResponse {
    pub id: String,
    pub success: bool,
    pub version: String,
    pub data: RaydiumSwapData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RaydiumSwapData {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: Option<u64>,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
}

/// Raydium swap result structure
#[derive(Debug)]
pub struct RaydiumSwapResult {
    pub success: bool,
    pub transaction_signature: Option<String>,
    pub input_amount: String,
    pub output_amount: String,
    pub price_impact: String,
    pub fee_lamports: u64,
    pub execution_time: f64,
    pub effective_price: Option<f64>, // Price per token in SOL
    pub swap_data: Option<SwapData>, // Complete swap data for reference
    pub error: Option<String>,
}

/// Raydium-specific transaction signing and sending
/// Uses Raydium swap transaction format
pub async fn raydium_sign_and_send_transaction(
    swap_transaction_base64: &str,
    configs: &crate::global::Configs
) -> Result<String, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_SIGN_START",
            &format!("🟣 Raydium: Signing and sending transaction (length: {} chars)", swap_transaction_base64.len())
        );
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_SIGNING",
            "✍️ Raydium: Signing transaction with wallet keypair..."
        );
    }
    
    // Get RPC client and sign transaction
    let rpc_client = crate::rpc::get_rpc_client();
    
    let signature = rpc_client.sign_and_send_transaction(swap_transaction_base64).await?;
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_TRANSACTION_SENT",
            &format!("📤 Raydium: Transaction sent to blockchain - Signature: {}", signature)
        );
    }
    
    // Wait for transaction confirmation before proceeding
    log(
        LogTag::Swap,
        "RAYDIUM_CONFIRMING",
        &format!("⏳ Raydium: Waiting for transaction confirmation: {}", &signature[..8])
    );
    
    match rpc_client.wait_for_transaction_confirmation_smart(&signature, TRANSACTION_CONFIRMATION_MAX_ATTEMPTS, TRANSACTION_CONFIRMATION_RETRY_DELAY_MS).await {
        Ok(true) => {
            log(
                LogTag::Swap,
                "RAYDIUM_CONFIRMED",
                &format!("✅ Raydium: Transaction confirmed on-chain: {}", &signature[..8])
            );
        }
        Ok(false) => {
            log(
                LogTag::Swap,
                "RAYDIUM_TIMEOUT",
                &format!("⏰ Raydium: Transaction confirmation timeout: {}", &signature[..8])
            );
            return Err(SwapError::TransactionError(
                format!("Transaction confirmation timeout: {}", signature)
            ));
        }
        Err(e) => {
            log(
                LogTag::Swap,
                "RAYDIUM_CONFIRMATION_ERROR",
                &format!("❌ Raydium: Transaction confirmation error: {} - {}", &signature[..8], e)
            );
            return Err(e);
        }
    }
    
    log(
        LogTag::Swap,
        "RAYDIUM_SIGN_SUCCESS",
        &format!("✅ Raydium: Transaction signed, sent and confirmed: {}", signature)
    );
    
    Ok(signature)
}

/// Gets a swap quote from Jupiter API specifically asking for Raydium routes
/// Note: Raydium doesn't have a public API anymore, so we use Jupiter with Raydium filtering
pub async fn get_raydium_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    from_address: &str,
    slippage: f64,
    fee: f64,
    is_anti_mev: bool,
) -> Result<SwapData, SwapError> {
    // Always log the issue for debugging
    log(
        LogTag::Swap,
        "RAYDIUM_DEPRECATED",
        "� Raydium direct API is no longer available. Raydium integration should use Jupiter with Raydium routes."
    );
    // Avoid printing directly to stdout in dashboard mode to prevent terminal scrolling
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_QUOTE_START",
            &format!(
                "🟣 Raydium Quote Request (DEPRECATED):
  Input: {} ({} units)
  Output: {}
  From: {}
  Slippage: {}%
  Fee: {}%
  Anti-MEV: {}",
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                input_amount,
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                &from_address[..8],
                slippage,
                fee,
                is_anti_mev
            )
        );
    }

    // Return an error indicating that Raydium API is no longer available
    Err(SwapError::ApiError(
        "Raydium direct API is deprecated and no longer available. Use Jupiter aggregator instead which includes Raydium routes.".to_string()
    ))
}

/// Builds a swap transaction from Raydium API
pub async fn get_raydium_swap_transaction(
    quote: &SwapData,
    user_public_key: &str,
    compute_unit_price: Option<u64>,
) -> Result<RaydiumSwapResponse, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_BUILD",
            &format!(
                "🟣 Raydium Building Transaction for {} -> {}",
                if quote.quote.input_mint == SOL_MINT { "SOL" } else { &quote.quote.input_mint[..8] },
                if quote.quote.output_mint == SOL_MINT { "SOL" } else { &quote.quote.output_mint[..8] }
            )
        );
    }

    // Convert SwapData back to Raydium quote format for transaction building
    let raydium_quote = convert_swap_data_to_raydium_quote(quote)?;

    // Build request body
    let mut request_body = serde_json::json!({
        "quoteResponse": raydium_quote,
        "userPublicKey": user_public_key,
        "wrapAndUnwrapSol": true,
        "useSharedAccounts": true,
    });

    // Add compute unit price if specified
    if let Some(price) = compute_unit_price {
        request_body["computeUnitPriceMicroLamports"] = serde_json::json!(price);
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_BUILD_REQUEST",
            &format!("🟣 Raydium Swap Request Body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_else(|_| "Failed to serialize".to_string()))
        );
    }

    let client = reqwest::Client::new();
    
    let response = timeout(
        Duration::from_secs(API_TIMEOUT_SECS),
        client.post(RAYDIUM_SWAP_API)
            .json(&request_body)
            .send()
    ).await
        .map_err(|_| {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "RAYDIUM_BUILD_TIMEOUT", "⏰ Raydium swap build timeout");
            }
            SwapError::ApiError("Raydium swap build timeout".to_string())
        })?
        .map_err(|e| {
            if is_debug_swap_enabled() {
                log(LogTag::Swap, "RAYDIUM_BUILD_ERROR", &format!("❌ Raydium swap build error: {}", e));
            }
            SwapError::NetworkError(e)
        })?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_BUILD_RESPONSE",
            &format!("📡 Raydium Build Response - Status: {}", response.status())
        );
    }

    if !response.status().is_success() {
        return Err(SwapError::ApiError(
            format!("Raydium swap build error: {}", response.status())
        ));
    }

    let swap_response: RaydiumSwapResponse = response.json().await
        .map_err(|e| SwapError::InvalidResponse(
            format!("Failed to parse Raydium swap response: {}", e)
        ))?;

    if !swap_response.success {
        return Err(SwapError::ApiError(
            format!("Raydium swap build failed: id={}", swap_response.id)
        ));
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "RAYDIUM_BUILD_SUCCESS",
            &format!("✅ Raydium Transaction built successfully: {}", swap_response.id)
        );
    }

    Ok(swap_response)
}

/// Executes a Raydium swap operation (DEPRECATED - API no longer available)
pub async fn execute_raydium_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    swap_data: SwapData
) -> Result<RaydiumSwapResult, SwapError> {
    log(
        LogTag::Swap,
        "RAYDIUM_SWAP_DEPRECATED",
        &format!(
            "🟣 Raydium direct API is deprecated for {} ({}) - Use Jupiter aggregator instead",
            token.symbol,
            token.name
        )
    );

    // Return a failed result indicating deprecation
    Ok(RaydiumSwapResult {
        success: false,
        transaction_signature: None,
        input_amount: swap_data.quote.in_amount.clone(),
        output_amount: "0".to_string(),
        price_impact: swap_data.quote.price_impact_pct.clone(),
        fee_lamports: 0,
        execution_time: 0.0,
        effective_price: None,
        swap_data: Some(swap_data),
        error: Some("Raydium direct API is deprecated and no longer available. Use Jupiter aggregator instead.".to_string()),
    })
}

/// Converts Raydium quote response to unified SwapData format
fn convert_raydium_quote_to_swap_data(quote_response: RaydiumQuoteResponse) -> Result<SwapData, SwapError> {
    // Determine decimals (default to 9 for SOL, 6 for USDC-like tokens, 9 for others)
    let in_decimals = if quote_response.data.input_mint == SOL_MINT { 9 } else { 9 };
    let out_decimals = if quote_response.data.output_mint == SOL_MINT { 9 } else { 9 };

    let swap_quote = SwapQuote {
        input_mint: quote_response.data.input_mint.clone(),
        in_amount: quote_response.data.input_amount.clone(),
        output_mint: quote_response.data.output_mint.clone(),
        out_amount: quote_response.data.output_amount.clone(),
        other_amount_threshold: quote_response.data.other_amount_threshold.clone(),
        in_decimals,
        out_decimals,
        swap_mode: quote_response.data.swap_mode.clone(),
        slippage_bps: quote_response.data.slippage_bps.to_string(),
        platform_fee: None,
        price_impact_pct: quote_response.data.price_impact_pct.to_string(),
        route_plan: serde_json::to_value(&quote_response.data.route_plan).unwrap_or_default(),
        context_slot: None,
        time_taken: quote_response.data.time_taken,
    };

    // Create a placeholder raw transaction (will be filled when building actual transaction)
    let raw_tx = RawTransaction {
        swap_transaction: String::new(), // Filled later during transaction building
        last_valid_block_height: 0,     // Filled later during transaction building
        prioritization_fee_lamports: 0, // Filled later during transaction building
        recent_blockhash: String::new(), // Filled later during transaction building
        version: Some("1.0.0".to_string()),
    };

    Ok(SwapData {
        quote: swap_quote,
        raw_tx,
        amount_in_usd: None,
        amount_out_usd: None,
        jito_order_id: None,
        sol_cost: None,
    })
}

/// Converts unified SwapData back to Raydium quote format
fn convert_swap_data_to_raydium_quote(swap_data: &SwapData) -> Result<RaydiumQuoteData, SwapError> {
    // Parse route plan back from JSON
    let route_plan: Vec<RaydiumRoutePlan> = serde_json::from_value(swap_data.quote.route_plan.clone())
        .map_err(|e| SwapError::InvalidResponse(format!("Failed to parse route plan: {}", e)))?;

    Ok(RaydiumQuoteData {
        input_mint: swap_data.quote.input_mint.clone(),
        input_amount: swap_data.quote.in_amount.clone(),
        output_mint: swap_data.quote.output_mint.clone(),
        output_amount: swap_data.quote.out_amount.clone(),
        other_amount_threshold: swap_data.quote.other_amount_threshold.clone(),
        swap_mode: swap_data.quote.swap_mode.clone(),
        slippage_bps: swap_data.quote.slippage_bps.parse().unwrap_or(100),
        price_impact_pct: swap_data.quote.price_impact_pct.parse().unwrap_or(0.0),
        route_plan,
        time_taken: swap_data.quote.time_taken,
    })
}

/// Validates the price from a Raydium swap quote against expected price
pub fn validate_raydium_quote_price(
    swap_data: &SwapData,
    input_amount: u64,
    expected_price: f64,
    is_sol_to_token: bool,
    slippage_tolerance: f64,
) -> Result<(), SwapError> {
    let output_amount_str = &swap_data.quote.out_amount;
    log(
        LogTag::Swap,
        "RAYDIUM_DEBUG",
        &format!("Raydium quote validation - Raw out_amount string: '{}'", output_amount_str)
    );

    let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
        log(
            LogTag::Swap,
            "RAYDIUM_ERROR",
            &format!("Raydium quote validation - Failed to parse out_amount '{}': {}", output_amount_str, e)
        );
        0.0
    });

    // Use actual token decimals from quote response
    let token_decimals = swap_data.quote.out_decimals as u32;
    let output_tokens = output_amount_raw / (10_f64).powi(token_decimals as i32);

    let actual_price_per_token = if is_sol_to_token {
        // For SOL to token: price = SOL spent / tokens received
        let input_sol = lamports_to_sol(input_amount);
        if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        }
    } else {
        // For token to SOL: price = SOL received / tokens spent
        let input_token_decimals = swap_data.quote.in_decimals as u32;
        let input_tokens = (input_amount as f64) / (10_f64).powi(input_token_decimals as i32);
        let output_sol = lamports_to_sol(output_amount_raw as u64);
        if input_tokens > 0.0 {
            output_sol / input_tokens
        } else {
            0.0
        }
    };

    let price_difference = (((actual_price_per_token - expected_price) / expected_price) * 100.0).abs();

    log(
        LogTag::Swap,
        "RAYDIUM_PRICE",
        &format!(
            "Raydium quote validation - Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
            expected_price,
            actual_price_per_token,
            price_difference
        )
    );

    if price_difference > slippage_tolerance {
        return Err(SwapError::SlippageExceeded(
            format!(
                "Raydium price difference {:.2}% exceeds tolerance {:.2}%",
                price_difference,
                slippage_tolerance
            )
        ));
    }

    Ok(())
}
