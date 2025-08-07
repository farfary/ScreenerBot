/// Swap execution and quote management functions
/// Handles GMGN router integration, quote fetching, and swap execution

use crate::global::{read_configs, is_debug_swap_enabled};
use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::{get_premium_transaction_rpc, SwapError, lamports_to_sol};
use crate::swaps::types::{SwapData, SwapRequest, GMGNApiResponse};
use crate::swaps::interface::SwapResult;
use crate::swaps::transaction::{sign_and_send_transaction, verify_swap_transaction, get_wallet_address};
use super::config::{SOL_MINT, GMGN_ANTI_MEV as ANTI_MEV, GMGN_PARTNER as PARTNER};

/// Validates swap parameters before execution
fn validate_swap_request(request: &SwapRequest) -> Result<(), SwapError> {
    if request.input_mint.is_empty() {
        return Err(SwapError::InvalidAmount("Input mint cannot be empty".to_string()));
    }

    if request.output_mint.is_empty() {
        return Err(SwapError::InvalidAmount("Output mint cannot be empty".to_string()));
    }

    if request.from_address.is_empty() {
        return Err(SwapError::InvalidAmount("From address cannot be empty".to_string()));
    }

    if request.input_amount == 0 {
        return Err(SwapError::InvalidAmount("Input amount must be greater than 0".to_string()));
    }

    if request.slippage < 0.0 || request.slippage > 100.0 {
        return Err(
            SwapError::InvalidAmount("Slippage must be between 0 and 100 percent".to_string())
        );
    }

    if request.fee < 0.0 {
        return Err(SwapError::InvalidAmount("Fee cannot be negative".to_string()));
    }

    if request.swap_mode != "ExactIn" && request.swap_mode != "ExactOut" {
        return Err(SwapError::InvalidAmount("Swap mode must be either 'ExactIn' or 'ExactOut'".to_string()));
    }

    Ok(())
}

/// Gets a swap quote from the GMGN router API with retry logic
pub async fn get_swap_quote(request: &SwapRequest) -> Result<SwapData, SwapError> {
    validate_swap_request(request)?;

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "QUOTE_START",
            &format!(
                "🔄 Generic Quote Request:\n  Input: {} ({} units)\n  Output: {}\n  From: {}\n  Slippage: {}%\n  Swap Mode: {}\n  Fee: {}%\n  Anti-MEV: {}",
                if request.input_mint == SOL_MINT {
                    "SOL"
                } else {
                    &request.input_mint[..8]
                },
                request.input_amount,
                if request.output_mint == SOL_MINT {
                    "SOL"
                } else {
                    &request.output_mint[..8]
                },
                &request.from_address[..8],
                request.slippage,
                request.swap_mode,
                request.fee,
                request.is_anti_mev
            )
        );
    }

    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}&partner={}",
        request.input_mint,
        request.output_mint,
        request.input_amount,
        request.from_address,
        request.slippage,
        request.swap_mode,
        request.fee,
        request.is_anti_mev,
        PARTNER
    );

    if is_debug_swap_enabled() {
        log(LogTag::Swap, "QUOTE_URL", &format!("🌐 API URL: {}", url));
    }

    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "DEBUG",
            &format!(
                "Swap request details: input_amount={}, slippage={}, swap_mode={}, fee={}, anti_mev={}, from_address={}",
                request.input_amount,
                request.slippage,
                request.swap_mode,
                request.fee,
                request.is_anti_mev,
                &request.from_address[..8]
            )
        );
        log(LogTag::Swap, "DEBUG", &format!("API URL: {}", url));
    }

    log(
        LogTag::Swap,
        "QUOTE",
        &format!(
            "Requesting swap quote: {} units {} -> {}",
            request.input_amount,
            if request.input_mint == SOL_MINT {
                "SOL"
            } else {
                &request.input_mint[..8]
            },
            if request.output_mint == SOL_MINT {
                "SOL"
            } else {
                &request.output_mint[..8]
            }
        )
    );

    let client = reqwest::Client::new();
    let mut last_error = None;

    // Retry up to 3 times with increasing delays
    for attempt in 1..=3 {
        if is_debug_swap_enabled() {
            log(
                LogTag::Swap,
                "QUOTE_ATTEMPT",
                &format!("🔄 Generic Quote attempt {}/3", attempt)
            );
        }

        match client.get(&url).send().await {
            Ok(response) => {
                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "QUOTE_RESPONSE",
                        &format!(
                            "📡 API response received - Status: {}, Attempt: {}/3",
                            response.status(),
                            attempt
                        )
                    );
                }

                if !response.status().is_success() {
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "QUOTE_HTTP_ERROR",
                            &format!("❌ HTTP Error: {} - {}", response.status(), response.status().canonical_reason().unwrap_or("Unknown"))
                        );
                    }
                    let status_code = response.status().as_u16();
                    let error_text = response
                        .text().await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    let error = SwapError::ApiError(
                        format!("HTTP error {}: {}", status_code, error_text)
                    );

                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Swap,
                            "QUOTE_ERROR",
                            &format!("❌ API error {}: {}", status_code, error_text)
                        );
                    }

                    if attempt < 3 && status_code >= 500 {
                        log(
                            LogTag::Swap,
                            "WARNING",
                            &format!("API error on attempt {}: {}, retrying...", attempt, error)
                        );
                        last_error = Some(error);
                        tokio::time::sleep(
                            tokio::time::Duration::from_millis(1000 * attempt)
                        ).await;
                        continue;
                    } else {
                        return Err(error);
                    }
                }

                // Get the raw response text first to handle parsing errors better
                let response_text = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        let error = SwapError::NetworkError(e);
                        if attempt < 3 {
                            log(
                                LogTag::Swap,
                                "WARNING",
                                &format!(
                                    "Network error on attempt {}: {}, retrying...",
                                    attempt,
                                    error
                                )
                            );
                            last_error = Some(error);
                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(1000 * attempt)
                            ).await;
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                };

                // Log the raw response for debugging
                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "DEBUG",
                        &format!(
                            "Raw API response: {}",
                            &response_text[..response_text.len().min(500)]
                        )
                    );
                }

                if is_debug_swap_enabled() {
                    log(
                        LogTag::Swap,
                        "QUOTE_RAW",
                        &format!("📄 Raw response length: {} chars", response_text.len())
                    );
                }

                // Try to parse the JSON response with better error handling
                let api_response: GMGNApiResponse = match
                    serde_json::from_str::<GMGNApiResponse>(&response_text)
                {
                    Ok(response) => {
                        if is_debug_swap_enabled() {
                            log(
                                LogTag::Swap,
                                "QUOTE_PARSED",
                                &format!(
                                    "✅ JSON parsing successful - Code: {}, Msg: {}",
                                    response.code,
                                    response.msg
                                )
                            );
                        }
                        response
                    }
                    Err(e) => {
                        if is_debug_swap_enabled() {
                            log(
                                LogTag::Swap,
                                "QUOTE_PARSE_ERR",
                                &format!("❌ JSON parsing failed: {}", e)
                            );
                        }
                        let error = SwapError::InvalidResponse(
                            format!("JSON parsing error: {} - Response: {}", e, response_text)
                        );
                        if attempt < 3 {
                            log(
                                LogTag::Swap,
                                "WARNING",
                                &format!(
                                    "Parse error on attempt {}: {}, retrying...",
                                    attempt,
                                    error
                                )
                            );
                            last_error = Some(error);
                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(1000 * attempt)
                            ).await;
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                };

                // Add delay to prevent rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                if api_response.code != 0 {
                    return Err(
                        SwapError::ApiError(
                            format!("API error: {} - {}", api_response.code, api_response.msg)
                        )
                    );
                }

                match api_response.data {
                    Some(data) => {
                        if is_debug_swap_enabled() {
                            let in_amount_sol = lamports_to_sol(
                                data.quote.in_amount.parse().unwrap_or(0)
                            );
                            let out_amount_sol = lamports_to_sol(
                                data.quote.out_amount.parse().unwrap_or(0)
                            );
                            log(
                                LogTag::Swap,
                                "QUOTE_SUCCESS",
                                &format!(
                                    "🎯 Quote successful\n  📊 Input: {:.6} SOL ({} lamports)\n  📊 Output: {:.6} SOL ({} lamports)\n  💹 Price Impact: {:.3}%\n  ⏱️ Time: {:.3}s",
                                    in_amount_sol,
                                    data.quote.in_amount,
                                    out_amount_sol,
                                    data.quote.out_amount,
                                    data.quote.price_impact_pct,
                                    data.quote.time_taken
                                )
                            );
                        }

                        log(
                            LogTag::Swap,
                            "QUOTE",
                            &format!(
                                "Quote received: {} -> {} (Impact: {}%, Time: {:.3}s)",
                                lamports_to_sol(data.quote.in_amount.parse().unwrap_or(0)),
                                lamports_to_sol(data.quote.out_amount.parse().unwrap_or(0)),
                                data.quote.price_impact_pct,
                                data.quote.time_taken
                            )
                        );
                        return Ok(data);
                    }
                    None => {
                        let error = SwapError::InvalidResponse("No data in response".to_string());
                        if attempt < 3 {
                            log(
                                LogTag::Swap,
                                "WARNING",
                                &format!("No data on attempt {}, retrying...", attempt)
                            );
                            last_error = Some(error);
                            tokio::time::sleep(
                                tokio::time::Duration::from_millis(1000 * attempt)
                            ).await;
                            continue;
                        } else {
                            return Err(error);
                        }
                    }
                }
            }
            Err(e) => {
                let error = SwapError::NetworkError(e);
                if attempt < 3 {
                    log(
                        LogTag::Swap,
                        "WARNING",
                        &format!("Network error on attempt {}: {}, retrying...", attempt, error)
                    );
                    last_error = Some(error);
                    tokio::time::sleep(tokio::time::Duration::from_millis(1000 * attempt)).await;
                    continue;
                } else {
                    return Err(error);
                }
            }
        }
    }

    // If we get here, all retries failed
    Err(last_error.unwrap_or_else(|| SwapError::ApiError("All retry attempts failed".to_string())))
}

/// Executes a swap operation with a pre-fetched quote to avoid duplicate API calls
/// NEW: Now includes transaction confirmation and actual result verification
pub async fn execute_swap_with_quote(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    swap_data: SwapData
) -> Result<SwapResult, SwapError> {
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
        "SWAP",
        &format!(
            "Executing swap for {} ({}) - {} {} -> {} (using cached quote)",
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
        )
    );

    // Get wallet address for logging
    let wallet_address = get_wallet_address()?;

    // Sign and send the transaction using global RPC client
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction
    ).await?;

    log(
        LogTag::Swap,
        "PENDING",
        &format!("Transaction submitted! TX: {} - Now verifying confirmation...", transaction_signature)
    );

    // CRITICAL FIX: Wait for transaction confirmation and verify actual results using instruction analysis
    let expected_direction = if input_mint == SOL_MINT { "buy" } else { "sell" };
    
    match verify_swap_transaction(
        &transaction_signature,
        input_mint,
        output_mint,
        expected_direction
    ).await {
        Ok(verification_result) => {
            if verification_result.success && verification_result.confirmed {
                // CRITICAL FIX: Use actual amounts if available, otherwise fail - don't mask extraction failures
                let input_amount_str = verification_result.input_amount
                    .map(|n| n.to_string())
                    .ok_or_else(|| SwapError::TransactionError(
                        "Failed to extract actual input amount from transaction".to_string()
                    ))?;
                let output_amount_str = verification_result.output_amount
                    .map(|n| n.to_string())
                    .ok_or_else(|| SwapError::TransactionError(
                        "Failed to extract actual output amount from transaction".to_string()
                    ))?;

                // CRITICAL FIX: Validate actual amounts vs quote expectations
                if let Err(validation_error) = validate_transaction_vs_quote(
                    &swap_data,
                    &verification_result,
                    input_mint,
                    output_mint
                ).await {
                    log(
                        LogTag::Swap,
                        "VALIDATION_WARNING",
                        &format!("⚠️ Transaction validation warning: {}", validation_error)
                    );
                    // Log warning but don't fail transaction - user should be aware of deviations
                }

                log(
                    LogTag::Swap,
                    "CONFIRMED",
                    &format!(
                        "✅ Transaction CONFIRMED on-chain! TX: {} | Actual Input: {} | Actual Output: {}",
                        transaction_signature,
                        input_amount_str,
                        output_amount_str
                    )
                );

                Ok(SwapResult {
                    success: true,
                    transaction_signature: Some(transaction_signature),
                    // Use ACTUAL amounts from blockchain verification
                    input_amount: input_amount_str,
                    output_amount: output_amount_str,
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: verification_result.transaction_fee,
                    execution_time: swap_data.quote.time_taken,
                    effective_price: verification_result.effective_price, // From blockchain verification
                    swap_data: Some(swap_data), // Include the complete swap data
                    error: None,
                })
            } else {
                let error_msg = verification_result.error.unwrap_or_else(|| "Transaction failed on blockchain".to_string());
                log(
                    LogTag::Swap,
                    "FAILED",
                    &format!("❌ Transaction FAILED on-chain! TX: {} - Error: {}", transaction_signature, error_msg)
                );

                Ok(SwapResult {
                    success: false,
                    transaction_signature: Some(transaction_signature),
                    input_amount: swap_data.quote.in_amount.clone(),
                    output_amount: "0".to_string(), // Zero output for failed transaction
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: verification_result.transaction_fee,
                    execution_time: swap_data.quote.time_taken,
                    effective_price: None,
                    swap_data: Some(swap_data),
                    error: Some("Transaction failed on-chain".to_string()),
                })
            }
        }
        Err(e) => {
            log(
                LogTag::Swap,
                "ERROR",
                &format!(
                    "❌ Transaction verification failed for TX: {} - Error: {}",
                    transaction_signature,
                    e
                )
            );

            // Return as failed transaction
            Ok(SwapResult {
                success: false,
                transaction_signature: Some(transaction_signature),
                input_amount: swap_data.quote.in_amount.clone(),
                output_amount: "0".to_string(),
                price_impact: swap_data.quote.price_impact_pct.clone(),
                fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
                execution_time: swap_data.quote.time_taken,
                effective_price: None,
                swap_data: Some(swap_data),
                error: Some(format!("Transaction verification failed: {}", e)),
            })
        }
    }
}

/// Validate actual transaction results against quote expectations
async fn validate_transaction_vs_quote(
    swap_data: &SwapData,
    verification_result: &crate::swaps::transaction::TransactionVerificationResult,
    input_mint: &str,
    output_mint: &str
) -> Result<(), SwapError> {
    use super::config::INTERNAL_SLIPPAGE_PERCENT;
    
    // Get quote expectations
    let quoted_input = swap_data.quote.in_amount.parse::<u64>()
        .map_err(|_| SwapError::ParseError("Invalid quoted input amount".to_string()))?;
    let quoted_output = swap_data.quote.out_amount.parse::<u64>()
        .map_err(|_| SwapError::ParseError("Invalid quoted output amount".to_string()))?;
    
    // Get actual amounts
    let actual_input = verification_result.input_amount
        .ok_or_else(|| SwapError::TransactionError("Missing actual input amount".to_string()))?;
    let actual_output = verification_result.output_amount
        .ok_or_else(|| SwapError::TransactionError("Missing actual output amount".to_string()))?;
    
    // Calculate deviations
    let input_deviation = if quoted_input > 0 {
        ((actual_input as f64 - quoted_input as f64) / quoted_input as f64 * 100.0).abs()
    } else { 0.0 };
    
    let output_deviation = if quoted_output > 0 {
        ((actual_output as f64 - quoted_output as f64) / quoted_output as f64 * 100.0).abs()
    } else { 0.0 };
    
    // Validate within acceptable tolerance (use slippage tolerance as reference)
    let tolerance = INTERNAL_SLIPPAGE_PERCENT * 2.0; // Allow 2x slippage tolerance for amount deviations
    
    if input_deviation > tolerance {
        return Err(SwapError::TransactionError(
            format!("Input amount deviation {:.2}% exceeds tolerance {:.2}% (quoted: {}, actual: {})",
                input_deviation, tolerance, quoted_input, actual_input)
        ));
    }
    
    if output_deviation > tolerance {
        return Err(SwapError::TransactionError(
            format!("Output amount deviation {:.2}% exceeds tolerance {:.2}% (quoted: {}, actual: {})",
                output_deviation, tolerance, quoted_output, actual_output)
        ));
    }
    
    // Validate effective price if available
    if let Some(effective_price) = verification_result.effective_price {
        // Calculate expected price from quote
        let is_buy = input_mint == SOL_MINT;
        let quoted_price = if is_buy {
            // Buy: SOL per token
            let sol_amount = crate::rpc::lamports_to_sol(quoted_input);
            let token_decimals = swap_data.quote.out_decimals as u32;
            let token_amount = (quoted_output as f64) / (10_f64).powi(token_decimals as i32);
            if token_amount > 0.0 { sol_amount / token_amount } else { 0.0 }
        } else {
            // Sell: SOL per token
            let sol_amount = crate::rpc::lamports_to_sol(quoted_output);
            let token_decimals = swap_data.quote.in_decimals as u32;
            let token_amount = (quoted_input as f64) / (10_f64).powi(token_decimals as i32);
            if token_amount > 0.0 { sol_amount / token_amount } else { 0.0 }
        };
        
        if quoted_price > 0.0 {
            let price_deviation = ((effective_price - quoted_price) / quoted_price * 100.0).abs();
            if price_deviation > INTERNAL_SLIPPAGE_PERCENT {
                return Err(SwapError::TransactionError(
                    format!("Price deviation {:.2}% exceeds slippage tolerance {:.2}% (quoted: {:.10}, actual: {:.10})",
                        price_deviation, INTERNAL_SLIPPAGE_PERCENT, quoted_price, effective_price)
                ));
            }
        }
    }
    
    log(
        LogTag::Swap,
        "QUOTE_VALIDATION",
        &format!(
            "✅ Quote validation passed: Input dev: {:.2}%, Output dev: {:.2}%",
            input_deviation, output_deviation
        )
    );
    
    Ok(())
}
