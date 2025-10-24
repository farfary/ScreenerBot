/// Jupiter Router Implementation
/// Handles swap quotes and execution via Jupiter DEX router
/// Based on official Jupiter API documentation: https://dev.jup.ag/docs/swap-api/
use crate::config::with_config;
use crate::constants::SOL_DECIMALS;
use crate::constants::SOL_MINT;
use crate::errors::ScreenerBotError;
// debug flags removed from global; no direct imports needed here
use crate::logger::{self, LogTag};
use crate::swaps::types::{
    JupiterQuoteResponse, JupiterSwapResponse, RawTransaction, SwapData, SwapQuote,
};
use crate::tokens::get_decimals;
use crate::tokens::Token;

use reqwest;
use tokio::time::{timeout, Duration};

// ============================================================================
// TIMING CONSTANTS - Hardcoded for optimal Jupiter swap performance
// ============================================================================

/// Quote API timeout in seconds - Jupiter is fast, 15s is sufficient
const QUOTE_TIMEOUT_SECS: u64 = 15;

/// Swap API timeout in seconds - includes execution, 20s for safety
const API_TIMEOUT_SECS: u64 = 20;

/// Retry attempts for failed operations
const RETRY_ATTEMPTS: usize = 3;

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

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
    pub swap_data: Option<SwapData>,  // Complete swap data for reference
    pub error: Option<String>,
}

/// Jupiter-specific transaction signing and sending
/// Uses Jupiter swap transaction format with priority fees and compute units
pub async fn jupiter_sign_and_send_transaction(
    swap_transaction_base64: &str,
    priority_fee_lamports: Option<u64>,
    compute_unit_limit: Option<u64>,
) -> Result<String, ScreenerBotError> {
    logger::debug(
        LogTag::Swap,
        &format!(
            "🟡 Jupiter: Signing transaction (length: {} chars)
  Priority Fee: {:?} lamports
  Compute Limit: {:?}",
            swap_transaction_base64.len(),
            priority_fee_lamports,
            compute_unit_limit
        ),
    );

    logger::debug(
        LogTag::Swap,
        "✍️ Jupiter: Signing transaction with wallet keypair...",
    );

    // Get RPC client and sign+send+confirm transaction
    let rpc_client = crate::rpc::get_rpc_client();
    let signature = rpc_client
        .sign_send_and_confirm_transaction(swap_transaction_base64)
        .await?;

    // Confirmed signature returned
    logger::debug(
        LogTag::Swap,
        &format!("✅ Jupiter: Transaction confirmed: {}", &signature[..8]),
    );

    Ok(signature)
}

/// Gets a Jupiter quote for token swap
pub async fn get_jupiter_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    slippage: f64,
    swap_mode: &str,
) -> Result<SwapData, ScreenerBotError> {
    logger::debug(
        LogTag::Swap,
        &format!(
            "🟡 Jupiter Quote Request:
  Input: {} ({} units)
  Output: {}
  Slippage: {}%",
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                &input_mint[..8]
            },
            input_amount,
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                &output_mint[..8]
            },
            slippage
        ),
    );

    let slippage_bps = ((slippage * 100.0) as u16).max(1).min(5000);

    let params = vec![
        ("inputMint".to_string(), input_mint.to_string()),
        ("outputMint".to_string(), output_mint.to_string()),
        ("amount".to_string(), input_amount.to_string()),
        ("slippageBps".to_string(), slippage_bps.to_string()),
        ("swapMode".to_string(), swap_mode.to_string()),
    ];

    let jupiter_quote_api = with_config(|cfg| cfg.swaps.jupiter.quote_api.clone());
    let url = format!(
        "{}?{}",
        jupiter_quote_api,
        params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    );

    logger::debug(LogTag::Swap, &format!("🌐 Jupiter API URL: {}", url));

    logger::debug(
        LogTag::Swap,
        &format!("🔗 Final Jupiter URL being called: {}", url),
    );

    logger::debug(LogTag::Swap, &format!("Jupiter Quote URL: {}", url));

    let quote_timeout_secs = QUOTE_TIMEOUT_SECS;

    logger::debug(
        LogTag::Swap,
        &format!(
            "📊 Jupiter Quote Parameters:
  URL: {}
  Input Amount: {} lamports
  Slippage BPS: {}
  Timeout: {}s",
            url, input_amount, slippage_bps, quote_timeout_secs
        ),
    );

    let client = reqwest::Client::new();

    logger::debug(LogTag::Swap, "📤 Jupiter: Sending quote request...");

    logger::debug(
        LogTag::Swap,
        &format!(
            "📊 Jupiter Quote Debug:
  • Input Mint: {}
  • Output Mint: {}
  • Amount: {} lamports
  • Slippage: {}% ({} BPS)
  • Swap Mode: {}",
            input_mint, output_mint, input_amount, slippage, slippage_bps, swap_mode
        ),
    );

    let response = timeout(
        Duration::from_secs(quote_timeout_secs),
        client.get(&url).send(),
    )
    .await
    .map_err(|_| {
        logger::debug(LogTag::Swap, "⏰ Jupiter quote request timeout");
        ScreenerBotError::api_error("Jupiter quote request timeout".to_string())
    })?
    .map_err(|e| {
        logger::debug(LogTag::Swap, &format!("❌ Jupiter network error: {}", e));
        ScreenerBotError::network_error(e.to_string())
    })?;

    logger::debug(
        LogTag::Swap,
        &format!("📡 Jupiter API Response - Status: {}", response.status()),
    );

    if !response.status().is_success() {
        let status = response.status();
        let body_snippet = match response.text().await {
            Ok(t) => t.chars().take(300).collect::<String>(),
            Err(_) => "<failed to read body>".to_string(),
        };
        logger::debug(
            LogTag::Swap,
            &format!(
                "❌ Jupiter HTTP Error: {} - {} | Body: {}",
                status,
                status.canonical_reason().unwrap_or("Unknown"),
                body_snippet.replace('\n', " ")
            ),
        );
        return Err(ScreenerBotError::api_error(format!(
            "Jupiter API error: {}",
            status
        )));
    }

    // Parse response
    logger::debug(LogTag::Swap, "🔄 Jupiter: Parsing JSON response...");

    let quote_response: JupiterQuoteResponse = response.json().await.map_err(|e| {
        logger::debug(
            LogTag::Swap,
            &format!("❌ Jupiter Response parsing failed: {}", e),
        );
        ScreenerBotError::invalid_response(format!("Failed to parse Jupiter quote response: {}", e))
    })?;

    logger::debug(
        LogTag::Swap,
        &format!(
            "🎯 Jupiter Quote Success:\n  In: {} {} \n  Out: {} {} \n  Price Impact: {}%\n  Slippage: {} BPS\n  Time: {:.3}s",
            quote_response.in_amount,
            if quote_response.input_mint == SOL_MINT {
                "SOL"
            } else {
                &quote_response.input_mint[..8]
            },
            quote_response.out_amount,
            if quote_response.output_mint == SOL_MINT {
                "SOL"
            } else {
                &quote_response.output_mint[..8]
            },
            quote_response.price_impact_pct,
            quote_response.slippage_bps,
            quote_response.time_taken.unwrap_or(0.0)
        ),
    );

    logger::debug(
        LogTag::Swap,
        &format!(
            "✅ Jupiter Quote: {} -> {} (price impact: {}%, time: {:.3}s)",
            quote_response.in_amount,
            quote_response.out_amount,
            quote_response.price_impact_pct,
            quote_response.time_taken.unwrap_or(0.0)
        ),
    );

    // Convert Jupiter quote to unified SwapData format
    convert_jupiter_quote_to_swap_data(quote_response).await
}

/// Builds a swap transaction from Jupiter API
pub async fn get_jupiter_swap_transaction(
    quote: &SwapData,
    user_public_key: &str,
    dynamic_compute_unit_limit: bool,
    priority_fee_lamports: Option<u64>,
) -> Result<JupiterSwapResponse, ScreenerBotError> {
    logger::debug(
        LogTag::Swap,
        &format!(
            "🟡 Jupiter Building Transaction for {} -> {}",
            if quote.quote.input_mint == SOL_MINT {
                "SOL"
            } else {
                &quote.quote.input_mint[..8]
            },
            if quote.quote.output_mint == SOL_MINT {
                "SOL"
            } else {
                &quote.quote.output_mint[..8]
            }
        ),
    );

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

        logger::debug(
            LogTag::Swap,
            &format!("💰 Jupiter: Adding priority fee: {} lamports", fee),
        );
    }

    logger::debug(
        LogTag::Swap,
        &format!(
            "📤 Jupiter Transaction Build Request:\n  • Quote Response: {} chars\n  • User: {}\n  • Dynamic Compute: {}\n  • Priority Fee: {:?} lamports",
            serde_json::to_string(&request_body["quoteResponse"]).unwrap_or_default().len(),
            &user_public_key[..8],
            dynamic_compute_unit_limit,
            priority_fee_lamports
        ),
    );

    let client = reqwest::Client::new();
    let jupiter_swap_api = with_config(|cfg| cfg.swaps.jupiter.swap_api.clone());
    let api_timeout_secs = API_TIMEOUT_SECS;

    logger::debug(
        LogTag::Swap,
        &format!(
            "📡 Jupiter: Sending transaction build request to {}",
            jupiter_swap_api
        ),
    );

    let response = timeout(
        Duration::from_secs(api_timeout_secs),
        client.post(&jupiter_swap_api).json(&request_body).send(),
    )
    .await
    .map_err(|_| {
        logger::debug(LogTag::Swap, "⏰ Jupiter swap transaction build timeout");
        ScreenerBotError::api_error("Jupiter swap transaction timeout".to_string())
    })?
    .map_err(|e| {
        logger::debug(
            LogTag::Swap,
            &format!("❌ Jupiter build network error: {}", e),
        );
        ScreenerBotError::network_error(e.to_string())
    })?;

    let response_status = response.status();

    logger::debug(
        LogTag::Swap,
        &format!(
            "📡 Jupiter Build API Response - Status: {}",
            response_status
        ),
    );

    if !response_status.is_success() {
        let error_text = response.text().await.unwrap_or_default();

        logger::debug(
            LogTag::Swap,
            &format!(
                "❌ Jupiter Build API Error: {} - {}",
                response_status, error_text
            ),
        );

        return Err(ScreenerBotError::api_error(format!(
            "Jupiter swap API error {}: {}",
            response_status, error_text
        )));
    }
    let swap_response: JupiterSwapResponse = response.json().await.map_err(|e| {
        logger::debug(
            LogTag::Swap,
            &format!("❌ Jupiter Build Response parsing failed: {}", e),
        );
        ScreenerBotError::invalid_response(format!("Failed to parse Jupiter swap response: {}", e))
    })?;

    logger::debug(
        LogTag::Swap,
        &format!(
            "✅ Jupiter transaction built successfully (priority fee: {} lamports)",
            swap_response.prioritization_fee_lamports
        ),
    );

    Ok(swap_response)
}

/// Execute a complete Jupiter swap operation
pub async fn execute_jupiter_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    swap_data: SwapData,
) -> Result<JupiterSwapResult, ScreenerBotError> {
    let wallet_address = crate::utils::get_wallet_address()?;

    logger::info(
        LogTag::Swap,
        &format!(
            "🟡 Executing Jupiter swap for {} - {} -> {}",
            token.symbol,
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                &input_mint[..8]
            },
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                &output_mint[..8]
            }
        ),
    );

    let start_time = std::time::Instant::now();

    let jupiter_dynamic_compute_unit_limit =
        with_config(|cfg| cfg.swaps.jupiter.dynamic_compute_unit_limit);
    let jupiter_default_priority_fee = with_config(|cfg| cfg.swaps.jupiter.default_priority_fee);

    // Get swap transaction from Jupiter
    let jupiter_tx = get_jupiter_swap_transaction(
        &swap_data,
        &wallet_address,
        jupiter_dynamic_compute_unit_limit,
        Some(jupiter_default_priority_fee), // default priority fee
    )
    .await?;

    // Sign and send transaction using Jupiter-specific method
    let transaction_signature = jupiter_sign_and_send_transaction(
        &jupiter_tx.swap_transaction,
        Some(jupiter_tx.prioritization_fee_lamports),
        None,
    )
    .await?;

    logger::info(
        LogTag::Swap,
        &format!(
            "🟡 Jupiter transaction submitted! TX: {} - Now adding to monitoring service...",
            transaction_signature
        ),
    );

    // Simplified approach - no complex transaction monitoring
    logger::info(
        LogTag::Swap,
        &format!(
            "📝 Jupiter swap transaction completed: {}",
            &transaction_signature[..8]
        ),
    );

    // Record swap event for durability
    crate::events::record_swap_event(
        &transaction_signature,
        &swap_data.quote.input_mint,
        &swap_data.quote.output_mint,
        swap_data.quote.in_amount.parse().unwrap_or(0),
        swap_data.quote.out_amount.parse().unwrap_or(0),
        true,
        None,
    )
    .await;

    // Return success result with quote data
    let execution_time = start_time.elapsed().as_secs_f64();

    Ok(JupiterSwapResult {
        success: true,
        transaction_signature: Some(transaction_signature),
        input_amount: swap_data.quote.in_amount.clone(),
        output_amount: swap_data.quote.out_amount.clone(),
        price_impact: swap_data.quote.price_impact_pct.clone(),
        fee_lamports: 0, // Will be calculated by monitoring service
        execution_time,
        effective_price: None, // Will be calculated by monitoring service
        swap_data: Some(swap_data),
        error: None,
    })
}

/// Converts Jupiter quote response to unified SwapData format
async fn convert_jupiter_quote_to_swap_data(
    jupiter_quote: JupiterQuoteResponse,
) -> Result<SwapData, ScreenerBotError> {
    // Create SwapQuote from Jupiter response
    // Get token decimals from cache
    let input_decimals = if jupiter_quote.input_mint == SOL_MINT {
        SOL_DECIMALS
    } else {
        get_decimals(&jupiter_quote.input_mint)
            .await
            .unwrap_or(SOL_DECIMALS)
    };

    let output_decimals = if jupiter_quote.output_mint == SOL_MINT {
        SOL_DECIMALS
    } else {
        get_decimals(&jupiter_quote.output_mint)
            .await
            .unwrap_or(SOL_DECIMALS)
    };

    logger::debug(
        LogTag::Swap,
        &format!(
            "🔢 Jupiter Decimals Resolution:
  Input mint: {} -> {} decimals
  Output mint: {} -> {} decimals",
            &jupiter_quote.input_mint[..8],
            input_decimals,
            &jupiter_quote.output_mint[..8],
            output_decimals
        ),
    );

    let swap_quote = SwapQuote {
        input_mint: jupiter_quote.input_mint,
        in_amount: jupiter_quote.in_amount,
        output_mint: jupiter_quote.output_mint,
        out_amount: jupiter_quote.out_amount,
        other_amount_threshold: jupiter_quote.other_amount_threshold,
        in_decimals: input_decimals as u8,
        out_decimals: output_decimals as u8,
        swap_mode: jupiter_quote.swap_mode,
        slippage_bps: jupiter_quote.slippage_bps.to_string(),
        platform_fee: jupiter_quote
            .platform_fee
            .map(|pf| serde_json::to_string(&pf).unwrap_or_default()),
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
fn convert_swap_data_to_jupiter_quote(
    swap_data: &SwapData,
) -> Result<JupiterQuoteResponse, ScreenerBotError> {
    let slippage_bps: u16 = swap_data
        .quote
        .slippage_bps
        .parse()
        .map_err(|_| ScreenerBotError::invalid_response("Invalid slippage_bps".to_string()))?;

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
        platform_fee: swap_data
            .quote
            .platform_fee
            .as_ref()
            .and_then(|pf| serde_json::from_str(pf).ok()),
        price_impact_pct: swap_data.quote.price_impact_pct.clone(),
        route_plan,
        context_slot: swap_data.quote.context_slot,
        time_taken: Some(swap_data.quote.time_taken),
    })
}

/// Validates Jupiter quote response for completeness and safety
pub fn validate_jupiter_quote(quote: &SwapData) -> Result<(), ScreenerBotError> {
    if quote.quote.input_mint.is_empty() {
        return Err(ScreenerBotError::invalid_response(
            "Missing input mint".to_string(),
        ));
    }

    if quote.quote.output_mint.is_empty() {
        return Err(ScreenerBotError::invalid_response(
            "Missing output mint".to_string(),
        ));
    }

    let in_amount: u64 = quote
        .quote
        .in_amount
        .parse()
        .map_err(|_| ScreenerBotError::invalid_response("Invalid in_amount".to_string()))?;

    let out_amount: u64 = quote
        .quote
        .out_amount
        .parse()
        .map_err(|_| ScreenerBotError::invalid_response("Invalid out_amount".to_string()))?;

    if in_amount == 0 {
        return Err(ScreenerBotError::invalid_response(
            "Zero input amount".to_string(),
        ));
    }

    if out_amount == 0 {
        return Err(ScreenerBotError::invalid_response(
            "Zero output amount".to_string(),
        ));
    }

    // Check for reasonable price impact (less than 50%)
    let price_impact: f64 = quote
        .quote
        .price_impact_pct
        .parse()
        .map_err(|_| ScreenerBotError::invalid_response("Invalid price impact".to_string()))?;

    if price_impact > 50.0 {
        return Err(ScreenerBotError::invalid_response(format!(
            "Price impact too high: {:.2}%",
            price_impact
        )));
    }

    Ok(())
}
