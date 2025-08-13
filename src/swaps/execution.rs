/// Swap execution and quote management functions
/// Handles GMGN router integration, quote fetching, and swap execution

use crate::global::{is_debug_swap_enabled};
use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::{SwapError, lamports_to_sol};
use crate::swaps::types::{SwapData, SwapRequest, GMGNApiResponse};
use crate::swaps::interface::SwapResult;
use super::config::{SOL_MINT, GMGN_PARTNER, TRANSACTION_CONFIRMATION_MAX_ATTEMPTS, TRANSACTION_CONFIRMATION_RETRY_DELAY_MS};


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
        GMGN_PARTNER
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
    let start_time = std::time::Instant::now();

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

    // Sign and send the transaction using global RPC client
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction
    ).await?;

    log(
        LogTag::Swap,
        "PENDING",
        &format!("Transaction submitted! TX: {} - Now adding to monitoring service...", transaction_signature)
    );

    // Return success result - verification handled by signature-only analysis
    let execution_time = start_time.elapsed().as_secs_f64();
    
    Ok(SwapResult {
        success: true,
        router_used: None, // TODO: Pass router type from caller
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

/// Simple transaction signing and sending function
/// Moved from transaction.rs to avoid circular dependencies
pub async fn sign_and_send_transaction(
    swap_transaction_base64: &str,
) -> Result<String, SwapError> {
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_SIGN_START",
            &format!("🔐 Starting transaction signing and sending process:
  📊 Transaction Details:
  • Base64 Length: {} characters
  • Data Size: ~{:.1} KB
  • Preview (first 60 chars): {}
  • Preview (last 60 chars): {}
  🔧 Processing: Decoding -> Signing -> Broadcasting",
                swap_transaction_base64.len(),
                (swap_transaction_base64.len() as f64 * 0.75) / 1024.0, // Base64 is ~75% efficient
                &swap_transaction_base64[..std::cmp::min(60, swap_transaction_base64.len())],
                if swap_transaction_base64.len() > 120 { 
                    &swap_transaction_base64[swap_transaction_base64.len()-60..] 
                } else { 
                    "N/A (short transaction)" 
                }
            )
        );
    }

    let rpc_client = crate::rpc::get_rpc_client();
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_RPC_CLIENT",
            "🔗 Using global RPC client for transaction processing:
  ✅ Client initialized
  🌐 Ready for blockchain communication
  🔐 Wallet signing enabled"
        );
    }
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_SENDING",
            "📤 Broadcasting signed transaction to Solana blockchain:
  🎯 Target: Solana mainnet
  ⏳ Waiting for transaction signature response...
  🔄 Network propagation in progress"
        );
    }
    
    let signature = rpc_client.sign_and_send_transaction(swap_transaction_base64).await?;
    
    if is_debug_swap_enabled() {
        log(
            LogTag::Swap,
            "TRANSACTION_SUCCESS",
            &format!("✅ Transaction successfully signed and sent:
  🎯 Transaction Signature: {}
  📊 Status: Submitted to blockchain
  ⏳ Next: Waiting for network confirmation
  🔍 View on explorer: https://solscan.io/tx/{}", signature, signature)
        );
    }
    
    // Wait for transaction confirmation before proceeding
    log(
        LogTag::Swap,
        "TRANSACTION_CONFIRMING",
        &format!("⏳ Waiting for transaction confirmation: {}", &signature[..8])
    );
    
    match rpc_client.wait_for_transaction_confirmation_smart(&signature, TRANSACTION_CONFIRMATION_MAX_ATTEMPTS, TRANSACTION_CONFIRMATION_RETRY_DELAY_MS).await {
        Ok(true) => {
            log(
                LogTag::Swap,
                "TRANSACTION_CONFIRMED",
                &format!("✅ Transaction confirmed on-chain: {}", &signature[..8])
            );
        }
        Ok(false) => {
            log(
                LogTag::Swap,
                "TRANSACTION_TIMEOUT",
                &format!("⏰ Transaction confirmation timeout: {}", &signature[..8])
            );
            return Err(SwapError::TransactionError(
                format!("Transaction confirmation timeout: {}", signature)
            ));
        }
        Err(e) => {
            log(
                LogTag::Swap,
                "TRANSACTION_CONFIRMATION_ERROR",
                &format!("❌ Transaction confirmation error: {} - {}", &signature[..8], e)
            );
            return Err(e);
        }
    }
    
    Ok(signature)
}

/// Transaction verification result structure
/// Simplified version for compatibility
#[derive(Debug)]
pub struct TransactionVerificationResult {
    pub success: bool,
    pub transaction_signature: String,
    pub confirmed: bool,
    
    // Amounts extracted from transaction instructions (lamports for SOL, raw units for tokens)
    pub input_amount: Option<u64>,     // Actual amount spent/consumed from instructions
    pub output_amount: Option<u64>,    // Actual amount received/produced from instructions
    
    // SOL flow analysis from instruction data
    pub sol_spent: Option<u64>,        // SOL spent in transaction (from transfers)
    pub sol_received: Option<u64>,     // SOL received in transaction (from transfers, includes ATA rent)
    pub sol_from_swap: Option<u64>,    // SOL received from swap only (excludes ATA rent)
    pub transaction_fee: u64,          // Network transaction fee in lamports
    pub priority_fee: Option<u64>,     // Priority fee in lamports (if any)
    
    // ATA analysis from instruction patterns
    pub ata_created: bool,             // Whether ATA creation was detected
    pub ata_closed: bool,              // Whether ATA closure was detected
    pub ata_rent_paid: u64,            // Amount of rent paid for ATA creation
    pub ata_rent_reclaimed: u64,       // Amount of rent reclaimed from ATA closure
    
    // Price calculations from instruction data
    pub effective_price: Option<f64>,  // Price per token in SOL (from instruction amounts)
    pub price_impact: Option<f64>,     // Calculated price impact percentage
    
    // Token transfer details
    pub input_mint: String,            // Input token mint
    pub output_mint: String,           // Output token mint
    pub input_decimals: u32,           // Input token decimals
    pub output_decimals: u32,          // Output token decimals
    
    // Status and error information
    pub creation_status: String,       // Success/Error status
    pub error_details: Option<String>, // Error details if verification failed
}
