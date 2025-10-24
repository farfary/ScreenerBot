/// GMGN swap router implementation
/// Handles GMGN-specific API calls and swap execution
use super::types::{GMGNApiResponse, SwapData};
use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::errors::ScreenerBotError;
use crate::logger::{self, LogTag};
use crate::tokens::Token;
use crate::utils::lamports_to_sol;

use reqwest;
use serde_json::Value;

// ============================================================================
// TIMING CONSTANTS - Hardcoded for optimal GMGN swap performance
// ============================================================================

/// Quote API timeout in seconds - GMGN can be slower, 15s is safe
const QUOTE_TIMEOUT_SECS: u64 = 15;

/// Retry attempts for failed operations
const RETRY_ATTEMPTS: usize = 3;

// ============================================================================
// TYPE DEFINITIONS
// ============================================================================

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
    pub swap_data: Option<SwapData>,  // Complete swap data for reference
    pub error: Option<String>,
}

/// GMGN-specific transaction signing and sending
/// Uses GMGN swap transaction format and premium RPC endpoints
pub async fn gmgn_sign_and_send_transaction(
    swap_transaction_base64: &str,
) -> Result<String, ScreenerBotError> {
    logger::debug(
        LogTag::Swap,
        &format!(
            "🔵 GMGN: Signing and sending transaction (length: {} chars)",
            swap_transaction_base64.len()
        ),
    );

    logger::debug(
        LogTag::Swap,
        &"📡 GMGN: Using centralized RPC client".to_string(),
    );

    // Get RPC client and sign transaction
    let rpc_client = crate::rpc::get_rpc_client();

    logger::debug(
        LogTag::Swap,
        &"✍️ GMGN: Signing transaction with wallet keypair...".to_string(),
    );

    // Use Solana SDK send_and_confirm via centralized RPC client
    let signature = rpc_client
        .sign_send_and_confirm_transaction(swap_transaction_base64)
        .await?;

    logger::debug(
        LogTag::Swap,
        &format!(
            "📤 GMGN: Transaction sent to blockchain - Signature: {}",
            signature
        ),
    );

    // Confirmed signature returned
    logger::info(
        LogTag::Swap,
        &format!("✅ GMGN: Transaction confirmed: {}", &signature[..8]),
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
) -> Result<SwapData, ScreenerBotError> {
    // Load config values
    let gmgn_fee_sol = with_config(|cfg| cfg.swaps.gmgn.fee_sol);
    let gmgn_anti_mev = with_config(|cfg| cfg.swaps.gmgn.anti_mev);
    let gmgn_quote_api = with_config(|cfg| cfg.swaps.gmgn.quote_api.clone());
    let gmgn_partner = with_config(|cfg| cfg.swaps.gmgn.partner.clone());
    let quote_timeout_secs = QUOTE_TIMEOUT_SECS;
    let retry_attempts = RETRY_ATTEMPTS;

    logger::debug(
        LogTag::Swap,
        &format!(
            "🔵 GMGN Quote Request:\n  Input: {} ({} units)\n  Output: {}\n  From: {}\n  Slippage: {}%\n  Swap Mode: {}\n  Fee: {}%\n  Anti-MEV: {}",
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
            &from_address[..8],
            slippage,
            swap_mode,
            gmgn_fee_sol,
            gmgn_anti_mev
        ),
    );

    let url = format!(
        "{}?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}&partner={}",
        gmgn_quote_api,
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
        swap_mode,
        gmgn_fee_sol,
        gmgn_anti_mev,
        gmgn_partner
    );

    logger::debug(LogTag::Swap, &format!("🌐 GMGN API URL: {}", url));

    logger::debug(
        LogTag::Swap,
        &format!(
            "📊 GMGN Quote Parameters:\n  URL: {}\n  Input Amount: {} lamports\n  Slippage BPS: {}\n  Partner: {}",
            url,
            input_amount,
            (slippage * 100.0) as u16,
            gmgn_partner
        ),
    );

    logger::debug(
        LogTag::Swap,
        &format!(
            "📊 GMGN Quote Debug:\n  • Input Mint: {}\n  • Output Mint: {}\n  • Amount: {} lamports\n  • Slippage: {}% ({} BPS)\n  • From Address: {}",
            input_mint,
            output_mint,
            input_amount,
            slippage,
            (slippage * 100.0) as u16,
            from_address
        ),
    );

    logger::info(
        LogTag::Swap,
        &format!(
            "🔵 Requesting GMGN quote: {} units {} -> {}",
            input_amount,
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

    let client = reqwest::Client::new();
    let mut last_error = None;

    // Retry up to configured attempts with increasing delays
    for attempt in 1..=retry_attempts {
        logger::info(
            LogTag::Swap,
            &format!("🔄 GMGN Quote attempt {}/{}", attempt, retry_attempts),
        );

        match client
            .get(&url)
            .timeout(tokio::time::Duration::from_secs(quote_timeout_secs))
            .send()
            .await
        {
            Ok(response) => {
                logger::debug(
                    LogTag::Swap,
                    &format!(
                        "📡 GMGN API Response - Status: {}, Headers: {:?}",
                        response.status(),
                        response.headers()
                    ),
                );

                if response.status().is_success() {
                    // Capture raw response text
                    let response_text = match response.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            last_error = Some(ScreenerBotError::invalid_response(format!(
                                "Failed to get response text: {}",
                                e
                            )));
                            continue;
                        }
                    };

                    logger::debug(
                        LogTag::Swap,
                        &format!(
                            "📄 GMGN Raw Response: {}",
                            &response_text[..response_text.len().min(500)]
                        ),
                    );

                    // First parse into generic JSON to inspect error code quickly
                    if let Ok(value) = serde_json::from_str::<Value>(&response_text) {
                        let code_opt = value.get("code").and_then(|c| c.as_i64());
                        let msg_opt = value.get("msg").and_then(|m| m.as_str()).unwrap_or("");

                        if let Some(code) = code_opt {
                            if code != 0 {
                                // Terminal error: no route (40000402) or other API code
                                if code == 40000402 || msg_opt.contains("no route") {
                                    logger::debug(
                                        LogTag::Swap,
                                        &format!(
                                            "GMGN_NO_ROUTE: 🛑 GMGN No route for pair (code {}): {} -- treating as terminal (no further retries)",
                                            code,
                                            msg_opt
                                        )
                                    );
                                    return Err(ScreenerBotError::api_error(format!(
                                        "GMGN no route: {} (code {})",
                                        msg_opt, code
                                    )));
                                } else {
                                    logger::debug(
                                        LogTag::Swap,
                                        &format!(
                                            "GMGN_API_ERROR: ❌ GMGN API Error pre-parse - Code: {}, Message: {}",
                                            code,
                                            msg_opt
                                        )
                                    );
                                    last_error = Some(ScreenerBotError::api_error(format!(
                                        "GMGN API error: {} - {}",
                                        code, msg_opt
                                    )));
                                    // Continue to next attempt (may be transient)
                                    continue;
                                }
                            }
                        }
                    }

                    // Fall back to full structured parse (success path)
                    match serde_json::from_str::<GMGNApiResponse>(&response_text) {
                        Ok(api_response) => {
                            logger::debug(
                                LogTag::Swap,
                                &format!(
                                    "GMGN_RESPONSE_PARSED: ✅ GMGN Response - Code: {}, Msg: {}, TID: {:?}",
                                    api_response.code, api_response.msg, api_response.tid
                                ),
                            );
                            if api_response.code == 0 {
                                if let Some(data) = api_response.data {
                                    logger::debug(
                                        LogTag::Swap,
                                        &format!(
                                            "GMGN_QUOTE_SUCCESS: 🎯 GMGN Quote Success:\n  In: {} {} ({})\n  Out: {} {} ({})\n  Price Impact: {}%\n  Slippage: {} BPS\n  Time: {:.3}s",
                                            data.quote.in_amount,
                                            if data.quote.input_mint == SOL_MINT {
                                                "SOL"
                                            } else {
                                                &data.quote.input_mint[..8]
                                            },
                                            data.quote.in_decimals,
                                            data.quote.out_amount,
                                            if data.quote.output_mint == SOL_MINT {
                                                "SOL"
                                            } else {
                                                &data.quote.output_mint[..8]
                                            },
                                            data.quote.out_decimals,
                                            data.quote.price_impact_pct,
                                            data.quote.slippage_bps,
                                            data.quote.time_taken
                                        )
                                    );

                                    logger::info(
                                        LogTag::Swap,
                                        &format!(
                                            "GMGN_SUCCESS: ✅ GMGN quote received: {} -> {} (impact: {}%, time: {:.3}s)",
                                            data.quote.in_amount,
                                            data.quote.out_amount,
                                            data.quote.price_impact_pct,
                                            data.quote.time_taken
                                        )
                                    );
                                    return Ok(data);
                                } else {
                                    logger::debug(
                                        LogTag::Swap,
                                        "GMGN_EMPTY_DATA: ❌ GMGN API returned empty data field",
                                    );
                                    last_error = Some(ScreenerBotError::invalid_response(
                                        "GMGN API returned empty data".to_string(),
                                    ));
                                }
                            } else {
                                logger::debug(
                                    LogTag::Swap,
                                    &format!(
                                        "GMGN_API_ERROR: ❌ GMGN API Error - Code: {}, Message: {}",
                                        api_response.code, api_response.msg
                                    ),
                                );
                                last_error = Some(ScreenerBotError::api_error(format!(
                                    "GMGN API error: {} - {}",
                                    api_response.code, api_response.msg
                                )));
                            }
                        }
                        Err(e) => {
                            logger::debug(
                                LogTag::Swap,
                                &format!("GMGN_PARSE_ERROR: ❌ GMGN Response parsing failed: {}", e),
                            );
                            last_error = Some(ScreenerBotError::invalid_response(format!(
                                "GMGN API JSON parse error: {}",
                                e
                            )));
                        }
                    }
                } else {
                    logger::debug(
                        LogTag::Swap,
                        &format!(
                            "GMGN_HTTP_ERROR: ❌ GMGN HTTP Error: {} - {}",
                            response.status(),
                            response.status().canonical_reason().unwrap_or("Unknown")
                        ),
                    );
                    last_error = Some(ScreenerBotError::api_error(format!(
                        "GMGN API HTTP error: {}",
                        response.status()
                    )));
                }
            }
            Err(e) => {
                logger::debug(
                    LogTag::Swap,
                    &format!("GMGN_NETWORK_ERROR: ❌ GMGN Network error on attempt {}: {}", attempt, e),
                );
                last_error = Some(ScreenerBotError::network_error(e.to_string()));
            }
        }

        // Wait before retry (except on last attempt)
        if attempt < 3 {
            let delay = tokio::time::Duration::from_millis(1000 * (attempt as u64));
            logger::debug(
                LogTag::Swap,
                &format!(
                    "GMGN_RETRY_DELAY: ⏳ GMGN Retry delay: {}ms before attempt {}",
                    delay.as_millis(),
                    attempt + 1
                ),
            );
            logger::info(
                LogTag::Swap,
                &format!(
                    "RETRY: GMGN attempt {} failed, retrying in {}ms...",
                    attempt,
                    delay.as_millis()
                ),
            );
            tokio::time::sleep(delay).await;
        }
    }

    // If we get here, all retries failed
    logger::debug(
        LogTag::Swap,
        "GMGN_ALL_RETRIES_FAILED: ❌ All GMGN retry attempts failed",
    );
    Err(last_error.unwrap_or_else(|| {
        ScreenerBotError::api_error("All GMGN retry attempts failed".to_string())
    }))
}

/// Executes a GMGN swap operation with a pre-fetched quote
pub async fn execute_gmgn_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    swap_data: SwapData,
) -> Result<GMGNSwapResult, ScreenerBotError> {
    // Determine if this is SOL to token or token to SOL
    let is_sol_to_token = input_mint == SOL_MINT;
    let input_display = if is_sol_to_token {
        format!("{:.6} SOL", lamports_to_sol(input_amount))
    } else {
        format!("{} tokens", input_amount)
    };

    logger::info(
        LogTag::Swap,
        &format!(
            "GMGN_SWAP: 🔵 Executing GMGN swap for {} ({}) - {} {} -> {} (using cached quote)",
            token.symbol,
            token.name,
            input_display,
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

    // Sign and send the transaction using GMGN-specific method
    let transaction_signature =
        gmgn_sign_and_send_transaction(&swap_data.raw_tx.swap_transaction).await?;

    logger::info(
        LogTag::Swap,
        &format!(
            "GMGN_PENDING: 🔵 GMGN transaction submitted! TX: {} - Now adding to monitoring service...",
            transaction_signature
        ),
    );

    // Record swap event for durability
    crate::events::record_swap_event(
        &transaction_signature,
        input_mint,
        output_mint,
        swap_data.quote.in_amount.parse().unwrap_or(input_amount),
        swap_data.quote.out_amount.parse().unwrap_or(0),
        true,
        None,
    )
    .await;

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
) -> Result<(), ScreenerBotError> {
    let output_amount_str = &swap_data.quote.out_amount;
    logger::debug(
        LogTag::Swap,
        &format!(
            "GMGN_DEBUG: GMGN quote validation - Raw out_amount string: '{}'",
            output_amount_str
        ),
    );

    let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
        logger::error(
            LogTag::Swap,
            &format!(
                "GMGN_ERROR: GMGN quote validation - Failed to parse out_amount '{}': {}",
                output_amount_str, e
            ),
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

    let price_difference =
        (((actual_price_per_token - expected_price) / expected_price) * 100.0).abs();

    logger::debug(
        LogTag::Swap,
        &format!(
            "GMGN_PRICE: GMGN quote validation - Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
            expected_price,
            actual_price_per_token,
            price_difference
        )
    );

    if price_difference > slippage_tolerance {
        return Err(ScreenerBotError::slippage_exceeded(format!(
            "GMGN price difference {:.2}% exceeds tolerance {:.2}%",
            price_difference, slippage_tolerance
        )));
    }

    Ok(())
}
