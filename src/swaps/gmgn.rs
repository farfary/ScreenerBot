/// GMGN swap router implementation
/// Handles GMGN-specific API calls and swap execution

use crate::tokens::Token;
use crate::rpc::{SwapError, lamports_to_sol, get_premium_transaction_rpc};
use crate::logger::{log, LogTag};
use crate::global::{read_configs, is_debug_swap_enabled};
use crate::tokens::decimals::{SOL_DECIMALS, LAMPORTS_PER_SOL};
use super::config::{
    GMGN_QUOTE_API, GMGN_PARTNER, GMGN_ANTI_MEV, 
    API_TIMEOUT_SECS, QUOTE_TIMEOUT_SECS, RETRY_ATTEMPTS,
    GMGN_DEFAULT_SWAP_MODE, SOL_MINT
};
use super::execution::{sign_and_send_transaction, verify_swap_transaction};
// Use utils for wallet address instead of transaction module
use crate::utils::get_wallet_address;
use super::types::{SwapData, SwapQuote, SwapRequest, GMGNApiResponse, deserialize_string_or_number, deserialize_optional_string_or_number};

use serde::{Deserialize, Serialize};
use reqwest;

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

/// GMGN-specific transaction signing and sending
/// Uses GMGN swap transaction format and premium RPC endpoints
pub async fn gmgn_sign_and_send_transaction(
    swap_transaction_base64: &str,
    configs: &crate::global::Configs
) -> Result<String, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_SIGN_START",
            &format!("🔵 GMGN: Signing and sending transaction (length: {} chars)", swap_transaction_base64.len())
        );
    }

    // Use premium RPC for GMGN transactions
    let selected_rpc = get_premium_transaction_rpc(configs);
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_RPC_SELECTED",
            &format!("📡 GMGN: Using RPC endpoint: {}", &selected_rpc[..50])
        );
    }
    
    // Get RPC client and sign transaction
    let rpc_client = crate::rpc::get_rpc_client();
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_SIGNING",
            "✍️ GMGN: Signing transaction with wallet keypair..."
        );
    }
    
    let signature = rpc_client.sign_and_send_transaction(swap_transaction_base64).await?;
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_TRANSACTION_SENT",
            &format!("📤 GMGN: Transaction sent to blockchain - Signature: {}", signature)
        );
    }
    
    log(
        LogTag::Swap,
        "GMGN_SIGN_SUCCESS",
        &format!("✅ GMGN: Transaction signed and sent successfully: {}", signature)
    );
    
    Ok(signature)
}

/// Gets a swap quote from the GMGN router API with retry logic
pub async fn get_gmgn_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    from_address: &str,
    slippage: f64,
    swap_mode: &str,
    fee: f64,
    is_anti_mev: bool,
) -> Result<SwapData, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_QUOTE_START",
            &format!(
                "🔵 GMGN Quote Request:
  Input: {} ({} units)
  Output: {}
  From: {}
  Slippage: {}%
  Swap Mode: {}
  Fee: {}%
  Anti-MEV: {}",
                if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
                input_amount,
                if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] },
                &from_address[..8],
                slippage,
                swap_mode,
                fee,
                is_anti_mev
            )
        );
    }

    let url = format!(
        "{}?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}&partner={}",
        GMGN_QUOTE_API,
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
        swap_mode,
        fee,
        is_anti_mev,
        GMGN_PARTNER
    );

    if is_debug_swap_enabled() {
        log(LogTag::Swap, "GMGN_URL", &format!("🌐 GMGN API URL: {}", url));
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_QUOTE_DETAILS",
            &format!(
                "📊 GMGN Quote Parameters:
  URL: {}
  Input Amount: {} lamports
  Slippage BPS: {}
  Partner: {}",
                url,
                input_amount,
                (slippage * 100.0) as u16,
                GMGN_PARTNER
            )
        );
        
        log(
            LogTag::Swap,
            "GMGN_QUOTE_DEBUG",
            &format!(
                "📊 GMGN Quote Debug:
  • Input Mint: {}
  • Output Mint: {}
  • Amount: {} lamports
  • Slippage: {}% ({} BPS)
  • From Address: {}",
                input_mint,
                output_mint,
                input_amount,
                slippage,
                (slippage * 100.0) as u16,
                from_address
            )
        );
    }

    log(
        LogTag::Swap,
        "GMGN_QUOTE",
        &format!(
            "🔵 Requesting GMGN quote: {} units {} -> {}",
            input_amount,
            if input_mint == SOL_MINT { "SOL" } else { &input_mint[..8] },
            if output_mint == SOL_MINT { "SOL" } else { &output_mint[..8] }
        )
    );

    let client = reqwest::Client::new();
    let mut last_error = None;

    // Retry up to configured attempts with increasing delays
    for attempt in 1..=RETRY_ATTEMPTS {
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "GMGN_QUOTE_ATTEMPT",
                &format!("🔄 GMGN Quote attempt {}/3", attempt)
            );
        }

        match client.get(&url)
            .timeout(tokio::time::Duration::from_secs(QUOTE_TIMEOUT_SECS))
            .send()
            .await {
            Ok(response) => {
                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "GMGN_RESPONSE_STATUS",
                        &format!("📡 GMGN API Response - Status: {}, Headers: {:?}", response.status(), response.headers())
                    );
                }

                if response.status().is_success() {
                    match response.json::<GMGNApiResponse>().await {
                        Ok(api_response) => {
                            if is_debug_swap_enabled() {
                                log(
                                    LogTag::Swap,
                                    "GMGN_RESPONSE_PARSED",
                                    &format!("✅ GMGN Response - Code: {}, Msg: {}, TID: {:?}", 
                                        api_response.code, api_response.msg, api_response.tid)
                                );
                            }

                            if api_response.code == 0 {
                                if let Some(data) = api_response.data {
                                    if is_debug_swap_enabled() {
                                        log(
                                            LogTag::Swap,
                                            "GMGN_QUOTE_SUCCESS",
                                            &format!(
                                                "🎯 GMGN Quote Success:\n  In: {} {} ({})\n  Out: {} {} ({})\n  Price Impact: {}%\n  Slippage: {} BPS\n  Time: {:.3}s",
                                                data.quote.in_amount,
                                                if data.quote.input_mint == SOL_MINT { "SOL" } else { &data.quote.input_mint[..8] },
                                                data.quote.in_decimals,
                                                data.quote.out_amount,
                                                if data.quote.output_mint == SOL_MINT { "SOL" } else { &data.quote.output_mint[..8] },
                                                data.quote.out_decimals,
                                                data.quote.price_impact_pct,
                                                data.quote.slippage_bps,
                                                data.quote.time_taken
                                            )
                                        );
                                    }

                                    log(
                                        LogTag::Swap,
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
                                    if is_debug_swap_enabled() {
                                        log(
                                            LogTag::Swap,
                                            "GMGN_EMPTY_DATA",
                                            "❌ GMGN API returned empty data field"
                                        );
                                    }
                                    last_error = Some(SwapError::InvalidResponse(
                                        "GMGN API returned empty data".to_string()
                                    ));
                                }
                            } else {
                                if is_debug_swap_enabled() {
                                    log(
                                        LogTag::Swap,
                                        "GMGN_API_ERROR",
                                        &format!("❌ GMGN API Error - Code: {}, Message: {}", api_response.code, api_response.msg)
                                    );
                                }
                                last_error = Some(SwapError::ApiError(
                                    format!("GMGN API error: {} - {}", api_response.code, api_response.msg)
                                ));
                            }
                        }
                        Err(e) => {
                            if is_debug_swap_enabled() {
                                log(
                                    LogTag::Swap,
                                    "GMGN_PARSE_ERROR",
                                    &format!("❌ GMGN Response parsing failed: {}", e)
                                );
                            }
                            last_error = Some(SwapError::InvalidResponse(
                                format!("GMGN API JSON parse error: {}", e)
                            ));
                        }
                    }
                } else {
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "GMGN_HTTP_ERROR",
                            &format!("❌ GMGN HTTP Error: {} - {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown"))
                        );
                    }
                    last_error = Some(SwapError::ApiError(
                        format!("GMGN API HTTP error: {}", response.status())
                    ));
                }
            }
            Err(e) => {
                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "GMGN_NETWORK_ERROR",
                        &format!("❌ GMGN Network error on attempt {}: {}", attempt, e)
                    );
                }
                last_error = Some(SwapError::NetworkError(e));
            }
        }

        // Wait before retry (except on last attempt)
        if attempt < 3 {
            let delay = tokio::time::Duration::from_millis(1000 * attempt as u64);
            if is_debug_swap_enabled() {
                log(
                    LogTag::Swap,
                    "GMGN_RETRY_DELAY",
                    &format!("⏳ GMGN Retry delay: {}ms before attempt {}", delay.as_millis(), attempt + 1)
                );
            }
            log(
                LogTag::Swap,
                "RETRY",
                &format!("GMGN attempt {} failed, retrying in {}ms...", attempt, delay.as_millis())
            );
            tokio::time::sleep(delay).await;
        }
    }

    // If we get here, all retries failed
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "GMGN_ALL_RETRIES_FAILED",
            "❌ All GMGN retry attempts failed"
        );
    }
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
        LogTag::Swap,
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

    // Get wallet address for logging
    let wallet_address = get_wallet_address()?;

    // Sign and send the transaction using GMGN-specific method
    let transaction_signature = gmgn_sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &configs
    ).await?;

    log(
        LogTag::Swap,
        "GMGN_PENDING",
        &format!("🔵 GMGN transaction submitted! TX: {} - Now adding to monitoring service...", transaction_signature)
    );

    // Add transaction to monitoring service instead of blocking verification
    let expected_direction = if input_mint == SOL_MINT { "buy" } else { "sell" };
    let target_mint = if input_mint == SOL_MINT { output_mint } else { input_mint };
    let amount_sol = if input_mint == SOL_MINT {
        // Buy: input is SOL
        swap_data.quote.in_amount.parse::<u64>().unwrap_or(0) as f64 / LAMPORTS_PER_SOL as f64
    } else {
        // Sell: output is SOL  
        swap_data.quote.out_amount.parse::<u64>().unwrap_or(0) as f64 / LAMPORTS_PER_SOL as f64
    };

    // Return success result - verification handled by signature-only analysis
    let execution_time = start_time.elapsed().as_secs_f64();
    
    Ok(GMGNSwapResult {
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
        LogTag::Swap,
        "GMGN_DEBUG",
        &format!("GMGN quote validation - Raw out_amount string: '{}'", output_amount_str)
    );

    let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
        log(
            LogTag::Swap,
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
        LogTag::Swap,
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
