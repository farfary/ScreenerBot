/// GMGN Router Implementation
/// Handles swap quotes and execution via GMGN.ai DEX router

use crate::global::{read_configs, is_debug_wallet_enabled, is_debug_swap_enabled};
use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::{get_premium_transaction_rpc, SwapError, lamports_to_sol, sol_to_lamports};
use crate::wallet::{get_wallet_address, sign_and_send_transaction, verify_transaction_and_get_actual_amounts};

use reqwest;
use serde::{Deserialize, Serialize, Deserializer};
use std::fmt;

/// Configuration constants for GMGN swap operations
pub const ANTI_MEV: bool = false; // Enable anti-MEV by default
pub const PARTNER: &str = "screenerbot"; // Partner identifier
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Custom deserializer for fields that can be either string or number
fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{self, Visitor};

    struct StringOrNumber;

    impl<'de> Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or number")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E> where E: de::Error {
            Ok(value.to_owned())
        }

        fn visit_i64<E>(self, value: i64) -> Result<String, E> where E: de::Error {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E> where E: de::Error {
            Ok(value.to_string())
        }

        fn visit_f64<E>(self, value: f64) -> Result<String, E> where E: de::Error {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

/// Custom deserializer for optional fields that can be either string or number
fn deserialize_optional_string_or_number<'de, D>(
    deserializer: D
) -> Result<Option<String>, D::Error>
    where D: Deserializer<'de>
{
    use serde::de::{self, Visitor};

    struct OptionalStringOrNumber;

    impl<'de> Visitor<'de> for OptionalStringOrNumber {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional string or number")
        }

        fn visit_none<E>(self) -> Result<Option<String>, E> where E: de::Error {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Option<String>, D::Error>
            where D: Deserializer<'de>
        {
            deserialize_string_or_number(deserializer).map(Some)
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_owned()))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Option<String>, E> where E: de::Error {
            Ok(Some(value.to_string()))
        }

        fn visit_unit<E>(self) -> Result<Option<String>, E> where E: de::Error {
            Ok(None)
        }
    }

    deserializer.deserialize_option(OptionalStringOrNumber)
}

/// Quote information from the GMGN swap router
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "inDecimals")]
    pub in_decimals: u8,
    #[serde(rename = "outDecimals")]
    pub out_decimals: u8,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps", deserialize_with = "deserialize_string_or_number")]
    pub slippage_bps: String,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<String>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: serde_json::Value,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

/// Raw transaction data from the GMGN swap router
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawTransaction {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: u64,
    #[serde(rename = "recentBlockhash")]
    pub recent_blockhash: String,
    pub version: Option<String>,
}

/// Complete GMGN swap response data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapData {
    pub quote: SwapQuote,
    pub raw_tx: RawTransaction,
    pub amount_in_usd: Option<String>,
    pub amount_out_usd: Option<String>,
    pub jito_order_id: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_string_or_number")]
    pub sol_cost: Option<String>,
}

/// GMGN API response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct GMGNApiResponse {
    pub code: i32,
    pub msg: String,
    pub tid: Option<String>,
    pub data: Option<SwapData>,
}

/// GMGN swap result structure
#[derive(Debug)]
pub struct GMGNSwapResult {
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

/// Gets a swap quote from the GMGN router API with retry logic
pub async fn get_gmgn_quote(
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
            "GMGN_QUOTE_START",
            &format!(
                "🔵 GMGN Quote: {} -> {} (amount: {}, slippage: {:.1}%)",
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                input_amount,
                slippage
            )
        );
    }

    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&fee={}&is_anti_mev={}&partner={}",
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
        fee,
        is_anti_mev,
        PARTNER
    );

    if is_debug_swap_enabled() {
        log(LogTag::Swap, "GMGN_URL", &format!("🌐 GMGN API URL: {}", url));
    }

    log(
        LogTag::Wallet,
        "GMGN_QUOTE",
        &format!(
            "Requesting GMGN quote: {} units {} -> {}",
            input_amount,
            if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] }
        )
    );

    let client = reqwest::Client::new();
    let mut last_error = None;

    // Retry up to 3 times with increasing delays
    for attempt in 1..=3 {
        match client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<GMGNApiResponse>().await {
                        Ok(api_response) => {
                            if api_response.code == 0 {
                                if let Some(data) = api_response.data {
                                    log(
                                        LogTag::Wallet,
                                        "GMGN_SUCCESS",
                                        &format!(
                                            "✅ GMGN quote received: {} -> {} (impact: {}%, time: {:.3}s)",
                                            data.quote.in_amount,
                                            data.quote.out_amount,
                                            data.quote.price_impact_pct,
                                            data.quote.time_taken
                                        )
                                    );
                                    return Ok(data);
                                } else {
                                    last_error = Some(SwapError::InvalidResponse(
                                        "GMGN API returned empty data".to_string()
                                    ));
                                }
                            } else {
                                last_error = Some(SwapError::ApiError(
                                    format!("GMGN API error: {} - {}", api_response.code, api_response.msg)
                                ));
                            }
                        }
                        Err(e) => {
                            last_error = Some(SwapError::InvalidResponse(
                                format!("GMGN API JSON parse error: {}", e)
                            ));
                        }
                    }
                } else {
                    last_error = Some(SwapError::ApiError(
                        format!("GMGN API HTTP error: {}", response.status())
                    ));
                }
            }
            Err(e) => {
                last_error = Some(SwapError::NetworkError(e));
            }
        }

        // Wait before retry (except on last attempt)
        if attempt < 3 {
            let delay = tokio::time::Duration::from_millis(1000 * attempt);
            log(
                LogTag::Wallet,
                "RETRY",
                &format!("GMGN attempt {} failed, retrying in {}ms...", attempt, delay.as_millis())
            );
            tokio::time::sleep(delay).await;
        }
    }

    // If we get here, all retries failed
    Err(last_error.unwrap_or_else(|| SwapError::ApiError("All GMGN retry attempts failed".to_string())))
}

/// Executes a GMGN swap operation with a pre-fetched quote
pub async fn execute_gmgn_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    swap_data: SwapData
) -> Result<GMGNSwapResult, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Determine if this is SOL to token or token to SOL
    let is_sol_to_token = input_mint == SOL_MINT;
    let input_display = if is_sol_to_token {
        format!("{:.6} SOL", lamports_to_sol(input_amount))
    } else {
        format!("{} tokens", input_amount)
    };

    log(
        LogTag::Wallet,
        "GMGN_SWAP",
        &format!(
            "🔵 Executing GMGN swap for {} ({}) - {} {} -> {} (using cached quote)",
            token.symbol,
            token.name,
            input_display,
            if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] }
        )
    );

    let start_time = std::time::Instant::now();

    // Sign and send the transaction using premium RPC
    let selected_rpc = get_premium_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Wallet,
        "GMGN_PENDING",
        &format!("🔵 GMGN transaction submitted! TX: {} - Now verifying confirmation...", transaction_signature)
    );

    // Wait for transaction confirmation and verify actual results
    match verify_transaction_and_get_actual_amounts(
        &transaction_signature,
        input_mint,
        output_mint,
        &configs
    ).await {
        Ok((confirmed_success, actual_input_amount, actual_output_amount)) => {
            let execution_time = start_time.elapsed().as_secs_f64();

            if confirmed_success {
                log(
                    LogTag::Wallet,
                    "GMGN_SUCCESS",
                    &format!(
                        "✅ GMGN swap confirmed! TX: {} (execution: {:.2}s)",
                        transaction_signature,
                        execution_time
                    )
                );

                Ok(GMGNSwapResult {
                    success: true,
                    transaction_signature: Some(transaction_signature),
                    input_amount: actual_input_amount.unwrap_or_else(|| swap_data.quote.in_amount.clone()),
                    output_amount: actual_output_amount.unwrap_or_else(|| swap_data.quote.out_amount.clone()),
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
                    execution_time,
                    effective_price: None, // Will be calculated by caller
                    swap_data: Some(swap_data),
                    error: None,
                })
            } else {
                log(
                    LogTag::Wallet,
                    "GMGN_FAILED",
                    &format!("❌ GMGN transaction failed: {}", transaction_signature)
                );

                Ok(GMGNSwapResult {
                    success: false,
                    transaction_signature: Some(transaction_signature),
                    input_amount: swap_data.quote.in_amount.clone(),
                    output_amount: "0".to_string(),
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
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
                LogTag::Wallet,
                "GMGN_ERROR",
                &format!("❌ GMGN transaction verification failed: {}", e)
            );

            // Return as failed transaction
            Ok(GMGNSwapResult {
                success: false,
                transaction_signature: Some(transaction_signature),
                input_amount: swap_data.quote.in_amount.clone(),
                output_amount: "0".to_string(),
                price_impact: swap_data.quote.price_impact_pct.clone(),
                fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
                execution_time,
                effective_price: None,
                swap_data: Some(swap_data),
                error: Some(format!("Transaction verification failed: {}", e)),
            })
        }
    }
}

/// Validates the price from a GMGN swap quote against expected price
pub fn validate_gmgn_quote_price(
    swap_data: &SwapData,
    input_amount: u64,
    expected_price: f64,
    is_sol_to_token: bool,
    slippage_tolerance: f64,
) -> Result<(), SwapError> {
    let output_amount_str = &swap_data.quote.out_amount;
    log(
        LogTag::Wallet,
        "GMGN_DEBUG",
        &format!("GMGN quote validation - Raw out_amount string: '{}'", output_amount_str)
    );

    let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
        log(
            LogTag::Wallet,
            "GMGN_ERROR",
            &format!("GMGN quote validation - Failed to parse out_amount '{}': {}", output_amount_str, e)
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
        LogTag::Wallet,
        "GMGN_PRICE",
        &format!(
            "GMGN quote validation - Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
            expected_price,
            actual_price_per_token,
            price_difference
        )
    );

    if price_difference > slippage_tolerance {
        return Err(SwapError::SlippageExceeded(
            format!(
                "GMGN price difference {:.2}% exceeds tolerance {:.2}%",
                price_difference,
                slippage_tolerance
            )
        ));
    }

    Ok(())
}
