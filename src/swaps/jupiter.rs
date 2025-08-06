/// Jupiter Router Implementation
/// Handles swap quotes and execution via Jupiter DEX router
/// Based on official Jupiter API documentation: https://dev.jup.ag/docs/swap-api/

use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::SwapError;
use crate::global::{is_debug_swap_enabled, is_debug_api_enabled, read_configs};
use crate::swaps::types::{SwapData, SwapQuote, RawTransaction, SOL_MINT, JupiterQuoteResponse, JupiterSwapResponse};

use serde::{Deserialize, Serialize};
use reqwest;
use tokio::time::{Duration, timeout};

// Jupiter API Configuration
const JUPITER_QUOTE_API: &str = "https://lite-api.jup.ag/swap/v1/quote";
const JUPITER_SWAP_API: &str = "https://lite-api.jup.ag/swap/v1/swap";
const API_TIMEOUT_SECS: u64 = 30;
const QUOTE_TIMEOUT_SECS: u64 = 15;

/// Jupiter swap result structure
#[derive(Debug)]
pub struct JupiterSwapResult {
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

/// Jupiter-specific transaction signing and sending
/// Uses Jupiter swap transaction format with priority fees and compute units
pub async fn jupiter_sign_and_send_transaction(
    swap_transaction_base64: &str,
    priority_fee_lamports: Option<u64>,
    compute_unit_limit: Option<u64>
) -> Result<String, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_SIGN_START",
            &format!(
                "🟡 Jupiter: Signing transaction (length: {} chars, priority fee: {:?}, compute limit: {:?})",
                swap_transaction_base64.len(),
                priority_fee_lamports,
                compute_unit_limit
            )
        );
    }

    // Get RPC client and sign transaction
    let rpc_client = crate::rpc::get_rpc_client();
    let signature = rpc_client.sign_and_send_transaction(swap_transaction_base64).await?;
    
    log(
        LogTag::Swap,
        "JUPITER_SIGN_SUCCESS",
        &format!("✅ Jupiter: Transaction signed and sent successfully: {}", signature)
    );
    
    Ok(signature)
}

/// Gets a Jupiter quote for token swap
pub async fn get_jupiter_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    from_address: &str,
    slippage: f64,
    fee: f64,
    is_anti_mev: bool,
) -> Result<SwapData, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_QUOTE_START",
            &format!(
                "🟡 Jupiter Quote: {} -> {} (amount: {}, slippage: {:.1}%)",
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                input_amount,
                slippage
            )
        );
    }

    let slippage_bps = ((slippage * 100.0) as u16).max(1).min(5000);
    
    let mut params = vec![
        ("inputMint".to_string(), input_mint.to_string()),
        ("outputMint".to_string(), output_mint.to_string()),
        ("amount".to_string(), input_amount.to_string()),
        ("slippageBps".to_string(), slippage_bps.to_string()),
        ("swapMode".to_string(), "ExactIn".to_string()),
    ];

    let url = format!("{}?{}", JUPITER_QUOTE_API, 
        params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    );

    if is_debug_api_enabled() {
        log(LogTag::Swap, "JUPITER_API", &format!("Jupiter Quote URL: {}", url));
    }

    let client = reqwest::Client::new();
    let response = timeout(Duration::from_secs(QUOTE_TIMEOUT_SECS), client.get(&url).send())
        .await
        .map_err(|_| SwapError::ApiError("Jupiter quote request timeout".to_string()))?
        .map_err(|e| SwapError::NetworkError(e))?;

    if !response.status().is_success() {
        return Err(SwapError::ApiError(
            format!("Jupiter API error: {}", response.status())
        ));
    }

    // Parse response
    let quote_response: JupiterQuoteResponse = response.json().await
        .map_err(|e| SwapError::InvalidResponse(
            format!("Failed to parse Jupiter quote response: {}", e)
        ))?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_SUCCESS",
            &format!(
                "✅ Jupiter Quote: {} -> {} (price impact: {}%, time: {:.3}s)",
                quote_response.in_amount,
                quote_response.out_amount,
                quote_response.price_impact_pct,
                quote_response.time_taken.unwrap_or(0.0)
            )
        );
    }

    // Convert Jupiter quote to unified SwapData format
    convert_jupiter_quote_to_swap_data(quote_response)
}

/// Builds a swap transaction from Jupiter API
pub async fn get_jupiter_swap_transaction(
    quote: &SwapData,
    user_public_key: &str,
    dynamic_compute_unit_limit: bool,
    priority_fee_lamports: Option<u64>,
) -> Result<JupiterSwapResponse, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_BUILD",
            &format!(
                "🟡 Jupiter Building Transaction for {} -> {}",
                if quote.quote.input_mint == SOL_MINT { "SOL" } else { &quote.quote.input_mint[..8] },
                if quote.quote.output_mint == SOL_MINT { "SOL" } else { &quote.quote.output_mint[..8] }
            )
        );
    }

    // Convert SwapData back to Jupiter quote format for transaction building
    let jupiter_quote = convert_swap_data_to_jupiter_quote(quote)?;

    // Build request body
    let mut request_body = serde_json::json!({
        "quoteResponse": jupiter_quote,
        "userPublicKey": user_public_key,
        "dynamicComputeUnitLimit": dynamic_compute_unit_limit,
    });

    // Add priority fee if specified
    if let Some(fee) = priority_fee_lamports {
        request_body["prioritizationFeeLamports"] = serde_json::json!(fee);
    }

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(API_TIMEOUT_SECS),
        client.post(JUPITER_SWAP_API)
            .json(&request_body)
            .send()
    )
    .await
    .map_err(|_| SwapError::ApiError("Jupiter swap transaction timeout".to_string()))?
    .map_err(|e| SwapError::NetworkError(e))?;

        let response_status = response.status();
        if !response_status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(
                format!("Jupiter swap API error {}: {}", response_status, error_text)
            ));
        }    let swap_response: JupiterSwapResponse = response.json().await
        .map_err(|e| SwapError::InvalidResponse(
            format!("Failed to parse Jupiter swap response: {}", e)
        ))?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_BUILD_SUCCESS",
            &format!(
                "✅ Jupiter transaction built successfully (priority fee: {} lamports)",
                swap_response.prioritization_fee_lamports
            )
        );
    }

    Ok(swap_response)
}

/// Execute a complete Jupiter swap operation
pub async fn execute_jupiter_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    swap_data: SwapData
) -> Result<JupiterSwapResult, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;
    let wallet_address = crate::swaps::transaction::get_wallet_address()?;

    log(
        LogTag::Swap,
        "JUPITER_SWAP",
        &format!(
            "🟡 Executing Jupiter swap for {} - {} -> {}",
            token.symbol,
            if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] }
        )
    );

    let start_time = std::time::Instant::now();

    // Get swap transaction from Jupiter
    let jupiter_tx = get_jupiter_swap_transaction(
        &swap_data,
        &wallet_address,
        true, // dynamic compute unit limit
        Some(100_000), // default priority fee
    ).await?;

    // Sign and send transaction using Jupiter-specific method
    let transaction_signature = jupiter_sign_and_send_transaction(
        &jupiter_tx.swap_transaction,
        Some(jupiter_tx.prioritization_fee_lamports),
        None,
    ).await?;

    log(
        LogTag::Swap,
        "JUPITER_PENDING",
        &format!("🟡 Jupiter transaction submitted! TX: {} - Now verifying confirmation...", transaction_signature)
    );

    // Wait for transaction confirmation and verify actual results
    match crate::swaps::transaction::verify_transaction_and_get_actual_amounts(
        &transaction_signature,
        input_mint,
        output_mint,
        &configs
    ).await {
        Ok((success, actual_input, actual_output)) => {
            let execution_time = start_time.elapsed().as_secs_f64();

            if success {
                let input_str = actual_input.unwrap_or_else(|| swap_data.quote.in_amount.clone());
                let output_str = actual_output.unwrap_or_else(|| swap_data.quote.out_amount.clone());

                log(
                    LogTag::Swap,
                    "JUPITER_SUCCESS",
                    &format!(
                        "✅ Jupiter swap completed! {} -> {} in {:.2}s",
                        input_str, output_str, execution_time
                    )
                );

                Ok(JupiterSwapResult {
                    success: true,
                    transaction_signature: Some(transaction_signature),
                    input_amount: input_str,
                    output_amount: output_str,
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: jupiter_tx.prioritization_fee_lamports,
                    execution_time,
                    effective_price: None, // Will be calculated separately
                    swap_data: Some(swap_data),
                    error: None,
                })
            } else {
                log(
                    LogTag::Swap,
                    "JUPITER_FAILED",
                    &format!("❌ Jupiter transaction failed on-chain: {}", transaction_signature)
                );

                Ok(JupiterSwapResult {
                    success: false,
                    transaction_signature: Some(transaction_signature),
                    input_amount: String::new(),
                    output_amount: String::new(),
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: jupiter_tx.prioritization_fee_lamports,
                    execution_time,
                    effective_price: None,
                    swap_data: Some(swap_data),
                    error: Some("Transaction failed on-chain".to_string()),
                })
            }
        }
        Err(e) => {
            let execution_time = start_time.elapsed().as_secs_f64();
            log(
                LogTag::Swap,
                "JUPITER_ERROR",
                &format!("❌ Jupiter transaction verification failed: {}", e)
            );

            Ok(JupiterSwapResult {
                success: false,
                transaction_signature: Some(transaction_signature),
                input_amount: String::new(),
                output_amount: String::new(),
                price_impact: swap_data.quote.price_impact_pct.clone(),
                fee_lamports: 0,
                execution_time,
                effective_price: None,
                swap_data: Some(swap_data),
                error: Some(e.to_string()),
            })
        }
    }
}

/// Converts Jupiter quote response to unified SwapData format
fn convert_jupiter_quote_to_swap_data(jupiter_quote: JupiterQuoteResponse) -> Result<SwapData, SwapError> {
    // Create SwapQuote from Jupiter response
    let swap_quote = SwapQuote {
        input_mint: jupiter_quote.input_mint,
        in_amount: jupiter_quote.in_amount,
        output_mint: jupiter_quote.output_mint,
        out_amount: jupiter_quote.out_amount,
        other_amount_threshold: jupiter_quote.other_amount_threshold,
        in_decimals: 9, // Default for Jupiter
        out_decimals: 9, // Default for Jupiter
        swap_mode: jupiter_quote.swap_mode,
        slippage_bps: jupiter_quote.slippage_bps.to_string(),
        platform_fee: jupiter_quote.platform_fee.map(|pf| serde_json::to_string(&pf).unwrap_or_default()),
        price_impact_pct: jupiter_quote.price_impact_pct,
        route_plan: serde_json::Value::Array(jupiter_quote.route_plan),
        context_slot: jupiter_quote.context_slot,
        time_taken: jupiter_quote.time_taken.unwrap_or(0.0),
    };

    // Create a placeholder RawTransaction (will be filled when building actual transaction)
    let raw_tx = RawTransaction {
        swap_transaction: String::new(),
        last_valid_block_height: 0,
        prioritization_fee_lamports: 0,
        recent_blockhash: String::new(),
        version: Some("jupiter".to_string()),
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

/// Converts SwapData back to Jupiter quote format for transaction building
fn convert_swap_data_to_jupiter_quote(swap_data: &SwapData) -> Result<JupiterQuoteResponse, SwapError> {
    let slippage_bps: u16 = swap_data.quote.slippage_bps.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid slippage_bps".to_string()))?;

    let route_plan = match &swap_data.quote.route_plan {
        serde_json::Value::Array(arr) => arr.clone(),
        _ => vec![],
    };

    Ok(JupiterQuoteResponse {
        input_mint: swap_data.quote.input_mint.clone(),
        in_amount: swap_data.quote.in_amount.clone(),
        output_mint: swap_data.quote.output_mint.clone(),
        out_amount: swap_data.quote.out_amount.clone(),
        other_amount_threshold: swap_data.quote.other_amount_threshold.clone(),
        swap_mode: swap_data.quote.swap_mode.clone(),
        slippage_bps,
        platform_fee: swap_data.quote.platform_fee.as_ref()
            .and_then(|pf| serde_json::from_str(pf).ok()),
        price_impact_pct: swap_data.quote.price_impact_pct.clone(),
        route_plan,
        context_slot: swap_data.quote.context_slot,
        time_taken: Some(swap_data.quote.time_taken),
    })
}

/// Validates Jupiter quote response for completeness and safety
pub fn validate_jupiter_quote(quote: &SwapData) -> Result<(), SwapError> {
    if quote.quote.input_mint.is_empty() {
        return Err(SwapError::InvalidResponse("Missing input mint".to_string()));
    }
    
    if quote.quote.output_mint.is_empty() {
        return Err(SwapError::InvalidResponse("Missing output mint".to_string()));
    }
    
    let in_amount: u64 = quote.quote.in_amount.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid in_amount".to_string()))?;
    
    let out_amount: u64 = quote.quote.out_amount.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid out_amount".to_string()))?;
    
    if in_amount == 0 {
        return Err(SwapError::InvalidResponse("Zero input amount".to_string()));
    }
    
    if out_amount == 0 {
        return Err(SwapError::InvalidResponse("Zero output amount".to_string()));
    }
    
    // Check for reasonable price impact (less than 50%)
    let price_impact: f64 = quote.quote.price_impact_pct.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid price impact".to_string()))?;
    
    if price_impact > 50.0 {
        return Err(SwapError::InvalidResponse(
            format!("Price impact too high: {:.2}%", price_impact)
        ));
    }
    
    Ok(())
}
