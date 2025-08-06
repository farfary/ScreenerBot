/// Jupiter Router Implementation
/// Handles swap quotes and execution via Jupiter DEX router
/// Based on official Jupiter API documentation: https://dev.jup.ag/docs/swap-api/

use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::SwapError;
use crate::global::{is_debug_swap_enabled, is_debug_api_enabled, read_configs};
use crate::swaps::types::{SwapData, SwapQuote, RawTransaction, SOL_MINT, JupiterQuoteResponse, JupiterSwapResponse};
use crate::swaps::{UnifiedSwapResult, RouterType};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformFee {
    pub amount: String,
    #[serde(rename = "feeBps")]
    pub fee_bps: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoutePlan {
    #[serde(rename = "swapInfo")]
    pub swap_info: SwapInfo,
    pub percent: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapInfo {
    #[serde(rename = "ammKey")]
    pub amm_key: String,
    pub label: String,
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "feeAmount")]
    pub fee_amount: String,
    #[serde(rename = "feeMint")]
    pub fee_mint: String,
}

/// Jupiter swap transaction response structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "computeUnitLimit")]
    pub compute_unit_limit: Option<u64>,
    #[serde(rename = "prioritizationType")]
    pub prioritization_type: Option<serde_json::Value>,
    #[serde(rename = "dynamicSlippageReport")]
    pub dynamic_slippage_report: Option<DynamicSlippageReport>,
    #[serde(rename = "simulationError")]
    pub simulation_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DynamicSlippageReport {
    #[serde(rename = "slippageBps")]
    pub slippage_bps: u16,
    #[serde(rename = "otherAmount")]
    pub other_amount: String,
    #[serde(rename = "simulatedIncurredSlippageBps")]
    pub simulated_incurred_slippage_bps: i32,
    #[serde(rename = "amplificationRatio")]
    pub amplification_ratio: String,
    #[serde(rename = "categoryName")]
    pub category_name: String,
    #[serde(rename = "heuristicMaxSlippageBps")]
    pub heuristic_max_slippage_bps: u16,
}

/// Jupiter prioritization fee configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrioritizationFeeLamports {
    #[serde(rename = "priorityLevelWithMaxLamports")]
    pub priority_level_with_max_lamports: PriorityLevelConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PriorityLevelConfig {
    #[serde(rename = "maxLamports")]
    pub max_lamports: u64,
    pub global: bool,
    #[serde(rename = "priorityLevel")]
    pub priority_level: String, // "medium", "high", "veryHigh"
}

/// Gets a swap quote from Jupiter API
pub async fn get_jupiter_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    slippage_bps: u16,
    restrict_intermediate_tokens: bool,
) -> Result<JupiterQuote, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_QUOTE",
            &format!(
                "🟡 Jupiter Quote Request: {} -> {} (amount: {}, slippage: {}bps)",
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                input_amount,
                slippage_bps
            )
        );
    }

    // Build query parameters
    let mut params = vec![
        ("inputMint", input_mint.to_string()),
        ("outputMint", output_mint.to_string()),
        ("amount", input_amount.to_string()),
        ("slippageBps", slippage_bps.to_string()),
    ];

    if restrict_intermediate_tokens {
        params.push(("restrictIntermediateTokens", "true".to_string()));
    }

    // Construct URL with query parameters
    let url = format!("{}?{}", JUPITER_QUOTE_API, 
        params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&")
    );

    if is_debug_api_enabled() {
        log(LogTag::Swap, "JUPITER_URL", &format!("🌐 Jupiter Quote URL: {}", url));
    }

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(QUOTE_TIMEOUT_SECS))
        .build()
        .map_err(|e| SwapError::NetworkError(e))?;

    // Make the request with timeout
    let response = timeout(
        Duration::from_secs(QUOTE_TIMEOUT_SECS),
        client.get(&url).send()
    )
    .await
    .map_err(|_| SwapError::ApiError("Jupiter quote request timeout".to_string()))?
    .map_err(|e| SwapError::NetworkError(e))?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(SwapError::ApiError(
            format!("Jupiter quote API error {}: {}", status, error_text)
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
    quote: &JupiterQuote,
    user_public_key: &str,
    dynamic_compute_unit_limit: bool,
    priority_fee: Option<PrioritizationFeeLamports>,
) -> Result<JupiterSwapResponse, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_BUILD",
            &format!(
                "� Jupiter Building Transaction for {} -> {}",
                if quote.input_mint == SOL_MINT { "SOL" } else { &quote.input_mint[..8] },
                if quote.output_mint == SOL_MINT { "SOL" } else { &quote.output_mint[..8] }
            )
        );
    }

    // Build request body
    let mut request_body = serde_json::json!({
        "quoteResponse": quote,
        "userPublicKey": user_public_key,
        "dynamicComputeUnitLimit": dynamic_compute_unit_limit,
    });

    // Add priority fee configuration if provided
    if let Some(priority_config) = priority_fee {
        request_body["prioritizationFeeLamports"] = serde_json::to_value(priority_config)
            .map_err(|e| SwapError::ApiError(format!("Failed to serialize priority fee config: {}", e)))?;
    }

    if is_debug_api_enabled() {
        log(LogTag::Swap, "JUPITER_REQUEST", &format!("📤 Request Body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_default()));
    }

    // Create HTTP client with timeout
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(API_TIMEOUT_SECS))
        .build()
        .map_err(|e| SwapError::NetworkError(e))?;

    // Make the request
    let response = timeout(
        Duration::from_secs(API_TIMEOUT_SECS),
        client
            .post(JUPITER_SWAP_API)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
    )
    .await
    .map_err(|_| SwapError::ApiError("Jupiter swap request timeout".to_string()))?
    .map_err(|e| SwapError::NetworkError(e))?;

    // Check response status
    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(SwapError::ApiError(
            format!("Jupiter swap API error {}: {}", status, error_text)
        ));
    }

    // Parse response
    let swap_response: JupiterSwapResponse = response.json().await
        .map_err(|e| SwapError::InvalidResponse(
            format!("Failed to parse Jupiter swap response: {}", e)
        ))?;

    // Check for simulation errors
    if let Some(error) = &swap_response.simulation_error {
        return Err(SwapError::TransactionError(
            format!("Jupiter transaction simulation failed: {}", error)
        ));
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_BUILT",
            &format!(
                "✅ Jupiter Transaction Built: block_height={}, compute_limit={:?}, priority_fee={:?}",
                swap_response.last_valid_block_height,
                swap_response.compute_unit_limit,
                swap_response.prioritization_fee_lamports
            )
        );
    }

    Ok(swap_response)
}

/// Converts Jupiter quote to unified SwapData format
pub fn jupiter_quote_to_swap_data(
    quote: &JupiterQuote,
    swap_response: &JupiterSwapResponse,
) -> Result<SwapData, SwapError> {
    // Convert Jupiter data to our unified format
    let swap_quote = SwapQuote {
        input_mint: quote.input_mint.clone(),
        in_amount: quote.in_amount.clone(),
        output_mint: quote.output_mint.clone(),
        out_amount: quote.out_amount.clone(),
        other_amount_threshold: quote.other_amount_threshold.clone(),
        in_decimals: if quote.input_mint == SOL_MINT { 9 } else { 6 }, // Default to 6 for tokens, 9 for SOL
        out_decimals: if quote.output_mint == SOL_MINT { 9 } else { 6 }, // Default to 6 for tokens, 9 for SOL
        swap_mode: quote.swap_mode.clone(),
        slippage_bps: quote.slippage_bps.to_string(),
        platform_fee: quote.platform_fee.as_ref().map(|pf| 
            format!("{}:{}", pf.amount, pf.fee_bps)
        ),
        price_impact_pct: quote.price_impact_pct.clone(),
        route_plan: serde_json::to_value(&quote.route_plan)
            .unwrap_or(serde_json::Value::Array(vec![])),
        context_slot: quote.context_slot,
        time_taken: quote.time_taken.unwrap_or(0.0),
    };

    let raw_tx = RawTransaction {
        swap_transaction: swap_response.swap_transaction.clone(),
        last_valid_block_height: swap_response.last_valid_block_height,
        prioritization_fee_lamports: swap_response.prioritization_fee_lamports.unwrap_or(0),
        recent_blockhash: "".to_string(), // Jupiter doesn't provide this directly
        version: Some("0".to_string()), // Jupiter uses versioned transactions
    };

    Ok(SwapData {
        quote: swap_quote,
        raw_tx,
        amount_in_usd: None, // Jupiter doesn't provide USD amounts
        amount_out_usd: None,
        jito_order_id: None,
        sol_cost: None,
    })
}

/// Executes the Jupiter swap by signing and sending the pre-built transaction
pub async fn send_jupiter_transaction(
    token: &Token,
    swap_data: &SwapData,
) -> Result<UnifiedSwapResult, SwapError> {
    use crate::global::read_configs;
    use crate::rpc::get_rpc_client;

    let start_time = std::time::Instant::now();

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_SEND",
            &format!(
                "🚀 Jupiter Sending Transaction: {} ({}) - {} -> {}",
                token.symbol,
                token.name,
                if swap_data.quote.input_mint == SOL_MINT { "SOL" } else { &swap_data.quote.input_mint[..8] },
                if swap_data.quote.output_mint == SOL_MINT { "SOL" } else { &swap_data.quote.output_mint[..8] }
            )
        );
    }

    // Send the transaction using our RPC client method
    let rpc_client = get_rpc_client();
    
    let signature = rpc_client
        .sign_and_send_transaction(&swap_data.raw_tx.swap_transaction)
        .await
        .map_err(|e| SwapError::TransactionError(format!("Transaction failed: {}", e)))?;

    let execution_time = start_time.elapsed().as_secs_f64();

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_SUCCESS",
            &format!(
                "✅ Jupiter Swap Complete: {} (signature: {}, time: {:.3}s)",
                token.symbol,
                signature,
                execution_time
            )
        );
    }

    // Get transaction details for result
    let in_amount = swap_data.quote.in_amount.parse().unwrap_or(0);
    let out_amount = swap_data.quote.out_amount.parse().unwrap_or(0);
    let price_impact = swap_data.quote.price_impact_pct.parse().unwrap_or(0.0);

    // Calculate effective price (tokens per SOL)
    let effective_price = if in_amount > 0 && out_amount > 0 {
        if swap_data.quote.input_mint == SOL_MINT {
            // Buying tokens with SOL
            let sol_amount = in_amount as f64 / 1_000_000_000.0; // Convert lamports to SOL
            let token_amount = out_amount as f64 / 10_f64.powi(swap_data.quote.out_decimals as i32);
            Some(token_amount / sol_amount)
        } else {
            // Selling tokens for SOL
            let token_amount = in_amount as f64 / 10_f64.powi(swap_data.quote.in_decimals as i32);
            let sol_amount = out_amount as f64 / 1_000_000_000.0; // Convert lamports to SOL
            Some(token_amount / sol_amount)
        }
    } else {
        None
    };

    Ok(UnifiedSwapResult {
        success: true,
        router_used: RouterType::Jupiter,
        transaction_signature: Some(signature.to_string()),
        input_amount: swap_data.quote.in_amount.clone(),
        output_amount: swap_data.quote.out_amount.clone(),
        price_impact: swap_data.quote.price_impact_pct.clone(),
        fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
        execution_time,
        effective_price,
        error: None,
    })
}

/// High-level Jupiter swap function that handles the complete flow
pub async fn execute_jupiter_swap(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    user_public_key: &str,
    slippage_bps: u16,
    use_dynamic_features: bool,
) -> Result<SwapData, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_EXEC",
            &format!(
                "🚀 Jupiter Execute Swap: {} -> {} (amount: {}, slippage: {}bps)",
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                input_amount,
                slippage_bps
            )
        );
    }

    // Step 1: Get quote
    let quote = get_jupiter_quote(
        input_mint,
        output_mint,
        input_amount,
        slippage_bps,
        true, // Restrict intermediate tokens for stability
    ).await?;

    // Step 2: Configure priority fees if using dynamic features
    let priority_fee = if use_dynamic_features {
        Some(PrioritizationFeeLamports {
            priority_level_with_max_lamports: PriorityLevelConfig {
                max_lamports: 1_000_000, // 0.001 SOL max
                global: false,
                priority_level: "high".to_string(),
            },
        })
    } else {
        None
    };

    // Step 3: Build swap transaction
    let swap_response = get_jupiter_swap_transaction(
        &quote,
        user_public_key,
        use_dynamic_features, // Dynamic compute unit limit
        priority_fee,
    ).await?;

    // Step 4: Convert to unified format
    let swap_data = jupiter_quote_to_swap_data(&quote, &swap_response)?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "JUPITER_COMPLETE",
            &format!(
                "✅ Jupiter Swap Ready: {} -> {} tokens (price impact: {}%)",
                swap_data.quote.in_amount,
                swap_data.quote.out_amount,
                swap_data.quote.price_impact_pct
            )
        );
    }

    Ok(swap_data)
}

/// Helper function to get optimal slippage in basis points
pub fn get_optimal_slippage_bps(slippage_percent: f64) -> u16 {
    // Convert percentage to basis points (1% = 100 bps)
    let bps = (slippage_percent * 100.0) as u16;
    
    // Ensure reasonable bounds
    if bps < 1 {
        1 // Minimum 0.01%
    } else if bps > 5000 {
        5000 // Maximum 50%
    } else {
        bps
    }
}

/// Creates default priority fee configuration for Jupiter swaps
pub fn create_default_priority_config() -> PrioritizationFeeLamports {
    PrioritizationFeeLamports {
        priority_level_with_max_lamports: PriorityLevelConfig {
            max_lamports: 500_000, // 0.0005 SOL max
            global: false,
            priority_level: "medium".to_string(),
        },
    }
}

/// Validates Jupiter quote response
pub fn validate_jupiter_quote(quote: &JupiterQuote) -> Result<(), SwapError> {
    // Ensure we have valid amounts
    let in_amount: u64 = quote.in_amount.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid input amount in quote".to_string()))?;
        
    let out_amount: u64 = quote.out_amount.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid output amount in quote".to_string()))?;

    if in_amount == 0 {
        return Err(SwapError::InvalidResponse("Zero input amount in quote".to_string()));
    }

    if out_amount == 0 {
        return Err(SwapError::InvalidResponse("Zero output amount in quote".to_string()));
    }

    // Validate price impact (should be a valid percentage)
    let price_impact: f64 = quote.price_impact_pct.parse()
        .map_err(|_| SwapError::InvalidResponse("Invalid price impact in quote".to_string()))?;

    if price_impact < 0.0 || price_impact > 100.0 {
        return Err(SwapError::InvalidResponse(
            format!("Invalid price impact: {}%", price_impact)
        ));
    }

    // Warn if price impact is very high
    if price_impact > 10.0 {
        log(
            LogTag::Swap,
            "HIGH_IMPACT",
            &format!("⚠️ High price impact detected: {:.2}%", price_impact)
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slippage_conversion() {
        assert_eq!(get_optimal_slippage_bps(1.0), 100);
        assert_eq!(get_optimal_slippage_bps(0.5), 50);
        assert_eq!(get_optimal_slippage_bps(15.0), 1500);
        assert_eq!(get_optimal_slippage_bps(0.0), 1);
        assert_eq!(get_optimal_slippage_bps(100.0), 5000);
    }

    #[test]
    fn test_priority_config_creation() {
        let config = create_default_priority_config();
        assert_eq!(config.priority_level_with_max_lamports.max_lamports, 500_000);
        assert_eq!(config.priority_level_with_max_lamports.priority_level, "medium");
        assert!(!config.priority_level_with_max_lamports.global);
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
