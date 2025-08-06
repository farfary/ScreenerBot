use crate::global::{ read_configs, is_debug_wallet_enabled, is_debug_swap_enabled };
use crate::tokens::Token;
use crate::logger::{ log, LogTag };
use crate::trader::{ SWAP_FEE_PERCENT, SLIPPAGE_TOLERANCE_PERCENT };
use crate::rpc::{ get_premium_transaction_rpc, SwapError, lamports_to_sol, sol_to_lamports };
use crate::swaps::{
    get_best_quote, execute_best_swap, UnifiedSwapResult,
    INITIAL_CONFIRMATION_DELAY_MS, MAX_CONFIRMATION_DELAY_SECS, CONFIRMATION_BACKOFF_MULTIPLIER,
    CONFIRMATION_TIMEOUT_SECS, RATE_LIMIT_BASE_DELAY_SECS, RATE_LIMIT_INCREMENT_SECS,
    EARLY_ATTEMPT_DELAY_MS, EARLY_ATTEMPTS_COUNT
};
use crate::swaps::types::{SwapData, SwapRequest, SwapApiResponse, SOL_MINT, ANTI_MEV, PARTNER, deserialize_string_or_number, deserialize_optional_string_or_number};

// Re-export for backward compatibility
pub use crate::swaps::interface::SwapResult;

use reqwest;
use serde::{ Deserialize, Serialize, Deserializer };
use std::error::Error;
use std::fmt;
use base64::{ Engine as _, engine::general_purpose };
use solana_sdk::{
    signature::Keypair,
    transaction::VersionedTransaction,
    signer::Signer,
    pubkey::Pubkey,
    instruction::{ Instruction, AccountMeta },
    transaction::Transaction,
};
use spl_token::instruction::close_account;
use bs58;
use std::str::FromStr;
use std::collections::HashSet;
use std::sync::{ Arc as StdArc, Mutex as StdMutex };
use once_cell::sync::Lazy;
use chrono::{ Utc, DateTime };

/// CRITICAL: Global tracking of pending transactions to prevent duplicates
static PENDING_TRANSACTIONS: Lazy<StdArc<StdMutex<HashSet<String>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashSet::new()))
});

/// CRITICAL: Global tracking of recent transaction attempts to prevent rapid retries
static RECENT_TRANSACTION_ATTEMPTS: Lazy<StdArc<StdMutex<HashSet<String>>>> = Lazy::new(|| {
    StdArc::new(StdMutex::new(HashSet::new()))
});

/// Prevents duplicate transactions by checking if a similar swap is already pending
pub fn check_and_reserve_transaction_slot(token_mint: &str, direction: &str) -> Result<(), SwapError> {
    let transaction_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut pending) = PENDING_TRANSACTIONS.lock() {
        if pending.contains(&transaction_key) {
            return Err(
                SwapError::TransactionError(
                    format!(
                        "Duplicate transaction prevented: {} already has a pending {} transaction",
                        token_mint,
                        direction
                    )
                )
            );
        }
        pending.insert(transaction_key);
        Ok(())
    } else {
        Err(SwapError::TransactionError("Failed to acquire transaction lock".to_string()))
    }
}

/// Releases transaction slot after completion (success or failure)
fn release_transaction_slot(token_mint: &str, direction: &str) {
    let transaction_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut pending) = PENDING_TRANSACTIONS.lock() {
        pending.remove(&transaction_key);
    }
}

/// Checks for recent transaction attempts to prevent rapid retries
pub fn check_recent_transaction_attempt(token_mint: &str, direction: &str) -> Result<(), SwapError> {
    let attempt_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut recent) = RECENT_TRANSACTION_ATTEMPTS.lock() {
        if recent.contains(&attempt_key) {
            return Err(
                SwapError::TransactionError(
                    format!(
                        "Recent transaction attempt detected for {} {}. Please wait before retrying.",
                        token_mint,
                        direction
                    )
                )
            );
        }
        recent.insert(attempt_key.clone());

        // Schedule removal after 30 seconds to allow retries
        let attempt_key_for_cleanup = attempt_key.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Ok(mut recent) = RECENT_TRANSACTION_ATTEMPTS.lock() {
                recent.remove(&attempt_key_for_cleanup);
            }
        });

        Ok(())
    } else {
        Err(SwapError::TransactionError("Failed to check recent attempts".to_string()))
    }
}

/// Clears recent transaction attempts to allow immediate retry
/// Used internally for automatic retry logic with increased slippage
pub fn clear_recent_transaction_attempt(token_mint: &str, direction: &str) {
    let attempt_key = format!("{}_{}", token_mint, direction);

    if let Ok(mut recent) = RECENT_TRANSACTION_ATTEMPTS.lock() {
        recent.remove(&attempt_key);
    }
}

/// RAII guard to ensure transaction slots are always released
pub struct TransactionSlotGuard {
    token_mint: String,
    direction: String,
}

impl TransactionSlotGuard {
    pub fn new(token_mint: &str, direction: &str) -> Self {
        Self {
            token_mint: token_mint.to_string(),
            direction: direction.to_string(),
        }
    }
}

impl Drop for TransactionSlotGuard {
    fn drop(&mut self) {
        release_transaction_slot(&self.token_mint, &self.direction);
    }
}

/// Gets wallet address from configs by deriving from private key
pub fn get_wallet_address() -> Result<String, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Decode the private key from base58
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key format: {}", e)))?;

    // Create keypair from private key
    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Return the public key as base58 string
    Ok(keypair.pubkey().to_string())
}

/// Signs and sends a transaction
pub async fn sign_and_send_transaction(
    swap_transaction_base64: &str,
    rpc_url: &str
) -> Result<String, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.sign_and_send_transaction(swap_transaction_base64).await
}

/// Gets transaction details from RPC to analyze balance changes
async fn get_transaction_details(
    _client: &reqwest::Client,
    transaction_signature: &str,
    _rpc_url: &str
) -> Result<crate::rpc::TransactionDetails, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_transaction_details(transaction_signature).await
}

/// CRITICAL NEW FUNCTION: Verifies transaction confirmation and extracts actual amounts
/// This function waits for transaction confirmation and returns actual blockchain results
/// Uses smart exponential backoff to prevent rate limiting with configurable delays
pub async fn verify_transaction_and_get_actual_amounts(
    transaction_signature: &str,
    input_mint: &str,
    output_mint: &str,
    configs: &crate::global::Configs
) -> Result<(bool, Option<String>, Option<String>), SwapError> {
    let wallet_address = get_wallet_address()?;
    let max_duration = tokio::time::Duration::from_secs(CONFIRMATION_TIMEOUT_SECS);
    let start_time = tokio::time::Instant::now();
    
    // Configurable delay parameters from swaps module
    let initial_delay = tokio::time::Duration::from_millis(INITIAL_CONFIRMATION_DELAY_MS);
    let max_delay = tokio::time::Duration::from_secs(MAX_CONFIRMATION_DELAY_SECS);
    let backoff_multiplier = CONFIRMATION_BACKOFF_MULTIPLIER;
    
    let mut current_delay = initial_delay;
    let mut attempt = 1;
    let mut consecutive_rate_limits = 0;

    log(
        LogTag::Wallet,
        "VERIFY",
        &format!(
            "🔍 Waiting for transaction confirmation: {} (smart retry with configurable delays)",
            transaction_signature
        )
    );

    loop {
        // Check if we've exceeded the total timeout
        if start_time.elapsed() >= max_duration {
            break;
        }

        // Wait before checking (except first attempt)
        if attempt > 1 {
            // Increase delay if we're hitting rate limits
            if consecutive_rate_limits > 2 {
                let rate_limit_delay = RATE_LIMIT_BASE_DELAY_SECS + consecutive_rate_limits * RATE_LIMIT_INCREMENT_SECS;
                current_delay = std::cmp::min(max_delay, tokio::time::Duration::from_secs(rate_limit_delay));
                log(
                    LogTag::Wallet,
                    "RATE_LIMIT",
                    &format!("⚠️ Multiple rate limits detected, using longer delay: {}s", current_delay.as_secs())
                );
            }
            
            tokio::time::sleep(current_delay).await;
        }

        match get_transaction_details(
            &reqwest::Client::new(),
            transaction_signature,
            &configs.rpc_url
        ).await {
            Ok(tx_details) => {
                // Reset rate limit counter on successful call
                consecutive_rate_limits = 0;
                
                // Check if transaction has metadata (confirmed)
                if let Some(meta) = &tx_details.meta {
                    // Check if transaction succeeded (err should be None for success)
                    let transaction_success = meta.err.is_none();

                    if !transaction_success {
                        log(
                            LogTag::Wallet,
                            "FAILED",
                            &format!(
                                "❌ Transaction {} FAILED on-chain: {:?}",
                                transaction_signature,
                                meta.err
                            )
                        );
                        return Ok((false, None, None));
                    }

                    log(
                        LogTag::Wallet,
                        "CONFIRMED",
                        &format!(
                            "✅ Transaction {} CONFIRMED successfully on attempt {} after {:.1}s",
                            transaction_signature,
                            attempt,
                            start_time.elapsed().as_secs_f64()
                        )
                    );

                    // Extract actual amounts from transaction metadata
                    let (actual_input, actual_output) = extract_actual_amounts_from_transaction(
                        &tx_details,
                        input_mint,
                        output_mint,
                        &wallet_address
                    );

                    return Ok((true, actual_input, actual_output));
                } else {
                    // Transaction not yet confirmed - use configurable delays for early attempts
                    if attempt <= EARLY_ATTEMPTS_COUNT {
                        current_delay = tokio::time::Duration::from_millis(EARLY_ATTEMPT_DELAY_MS);
                    } else {
                        current_delay = std::cmp::min(max_delay, 
                            tokio::time::Duration::from_millis(
                                (current_delay.as_millis() as f64 * backoff_multiplier) as u64
                            )
                        );
                    }
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Wallet,
                            "PENDING",
                            &format!(
                                "⏳ Transaction {} still pending... (attempt {}, next check in {:.1}s)",
                                transaction_signature,
                                attempt,
                                current_delay.as_secs_f64()
                            )
                        );
                    }
                }
            }
            Err(e) => {
                // Check if this is a rate limit error
                let error_str = e.to_string().to_lowercase();
                if error_str.contains("429") || error_str.contains("rate limit") || error_str.contains("too many requests") {
                    consecutive_rate_limits += 1;
                    // Use configurable delay for rate limits
                    let rate_limit_delay = RATE_LIMIT_BASE_DELAY_SECS + consecutive_rate_limits * RATE_LIMIT_INCREMENT_SECS;
                    current_delay = tokio::time::Duration::from_secs(rate_limit_delay);
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Wallet,
                            "RATE_LIMIT",
                            &format!(
                                "⚠️ Rate limit hit (attempt {}), waiting {}s before retry",
                                attempt,
                                current_delay.as_secs()
                            )
                        );
                    }
                } else {
                    // Reset rate limit counter for non-rate-limit errors
                    consecutive_rate_limits = 0;
                    
                    // For other errors, use exponential backoff but shorter than rate limits
                    current_delay = std::cmp::min(max_delay, 
                        tokio::time::Duration::from_millis(
                            (current_delay.as_millis() as f64 * backoff_multiplier) as u64
                        )
                    );
                    
                    if is_debug_swap_enabled() {
                        log(
                            LogTag::Wallet,
                            "PENDING",
                            &format!(
                                "⏳ Transaction {} not found yet (attempt {}, next check in {:.1}s) - {}",
                                transaction_signature,
                                attempt,
                                current_delay.as_secs_f64(),
                                e
                            )
                        );
                    }
                }
            }
        }
        
        attempt += 1;
    }

    // Timeout reached
    log(
        LogTag::Wallet,
        "TIMEOUT",
        &format!(
            "⏰ Transaction verification timeout for {} after {:.1}s ({} attempts)",
            transaction_signature,
            start_time.elapsed().as_secs_f64(),
            attempt - 1
        )
    );

    Err(
        SwapError::TransactionError(
            format!("Transaction confirmation timeout after {:.1}s", start_time.elapsed().as_secs_f64())
        )
    )
}

/// Extracts actual input/output amounts from confirmed transaction metadata
fn extract_actual_amounts_from_transaction(
    tx_details: &crate::rpc::TransactionDetails,
    input_mint: &str,
    output_mint: &str,
    wallet_address: &str
) -> (Option<String>, Option<String>) {
    let meta = match &tx_details.meta {
        Some(meta) => meta,
        None => {
            log(LogTag::Wallet, "ERROR", "Cannot extract amounts - no transaction metadata");
            return (None, None);
        }
    };

    // For SOL transactions, use pre/post balance differences
    if input_mint == SOL_MINT || output_mint == SOL_MINT {
        // Find wallet's account index in transaction
        if let Ok(wallet_pubkey) = solana_sdk::pubkey::Pubkey::from_str(wallet_address) {
            // Try to find wallet in account keys (this is simplified - in practice we'd need to parse the message)
            // For now, assume wallet is the first account (fee payer)
            if meta.pre_balances.len() > 0 && meta.post_balances.len() > 0 {
                let sol_difference = if meta.post_balances[0] > meta.pre_balances[0] {
                    meta.post_balances[0] - meta.pre_balances[0] // Gained SOL (sell transaction)
                } else {
                    meta.pre_balances[0] - meta.post_balances[0] // Lost SOL (buy transaction)
                };

                if input_mint == SOL_MINT {
                    // SOL -> Token swap: return SOL spent and tokens received
                    let sol_spent = sol_difference.to_string();
                    let tokens_received = extract_token_amount_from_balances(
                        meta,
                        output_mint,
                        false
                    );
                    return (Some(sol_spent), tokens_received);
                } else {
                    // Token -> SOL swap: return tokens spent and SOL received
                    let tokens_spent = extract_token_amount_from_balances(meta, input_mint, true);
                    let sol_received = sol_difference.to_string();
                    return (tokens_spent, Some(sol_received));
                }
            }
        }
    }

    // For token-to-token swaps or if SOL extraction failed, try token balance extraction
    let input_amount = extract_token_amount_from_balances(meta, input_mint, true);
    let output_amount = extract_token_amount_from_balances(meta, output_mint, false);

    (input_amount, output_amount)
}

/// Extracts token amount changes from transaction token balance metadata
fn extract_token_amount_from_balances(
    meta: &crate::rpc::TransactionMeta,
    mint: &str,
    is_decrease: bool // true for input (decrease), false for output (increase)
) -> Option<String> {
    let pre_balances = meta.pre_token_balances.as_ref()?;
    let post_balances = meta.post_token_balances.as_ref()?;

    // Find token balance changes for the specific mint
    for post_balance in post_balances {
        if post_balance.mint == mint {
            // Find corresponding pre-balance
            let pre_amount = pre_balances
                .iter()
                .find(|pre| pre.account_index == post_balance.account_index && pre.mint == mint)
                .map(|pre| pre.ui_token_amount.amount.parse::<u64>().unwrap_or(0))
                .unwrap_or(0);

            let post_amount = post_balance.ui_token_amount.amount.parse::<u64>().unwrap_or(0);

            let amount_change = if is_decrease {
                // For input tokens, we want the decrease (pre - post)
                if pre_amount > post_amount {
                    pre_amount - post_amount
                } else {
                    0
                }
            } else {
                // For output tokens, we want the increase (post - pre)
                if post_amount > pre_amount {
                    post_amount - pre_amount
                } else {
                    0
                }
            };

            if amount_change > 0 {
                return Some(amount_change.to_string());
            }
        }
    }

    None
}

// calculate_effective_price function has been moved to transactions/analyzer.rs
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
                "🔍 Getting swap quote\n  📊 Amount: {} units\n  💱 Route: {} -> {}\n  ⚙️ Slippage: {}%, Fee: {}%, Anti-MEV: {}",
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
                },
                request.slippage,
                request.fee,
                request.is_anti_mev
            )
        );
    }

    let url = format!(
        "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&fee={}&is_anti_mev={}&partner={}",
        request.input_mint,
        request.output_mint,
        request.input_amount,
        request.from_address,
        request.slippage,
        request.fee,
        request.is_anti_mev,
        PARTNER
    );

    if is_debug_swap_enabled() {
        log(LogTag::Swap, "QUOTE_URL", &format!("🌐 API URL: {}", url));
    }

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "Swap request details: input_amount={}, slippage={}, fee={}, anti_mev={}, from_address={}",
                request.input_amount,
                request.slippage,
                request.fee,
                request.is_anti_mev,
                &request.from_address[..8]
            )
        );
        log(LogTag::Wallet, "DEBUG", &format!("API URL: {}", url));
    }

    log(
        LogTag::Wallet,
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
                            LogTag::Wallet,
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
                                LogTag::Wallet,
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
                if is_debug_wallet_enabled() {
                    log(
                        LogTag::Wallet,
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
                let api_response: SwapApiResponse = match
                    serde_json::from_str::<SwapApiResponse>(&response_text)
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
                                LogTag::Wallet,
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
                            LogTag::Wallet,
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
                                LogTag::Wallet,
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
                        LogTag::Wallet,
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
        LogTag::Wallet,
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

    // Sign and send the transaction using premium RPC
    let selected_rpc = get_premium_transaction_rpc(&configs);
    let transaction_signature = sign_and_send_transaction(
        &swap_data.raw_tx.swap_transaction,
        &selected_rpc
    ).await?;

    log(
        LogTag::Wallet,
        "PENDING",
        &format!("Transaction submitted! TX: {} - Now verifying confirmation...", transaction_signature)
    );

    // CRITICAL FIX: Wait for transaction confirmation and verify actual results
    match
        verify_transaction_and_get_actual_amounts(
            &transaction_signature,
            input_mint,
            output_mint,
            &configs
        ).await
    {
        Ok((confirmed_success, actual_input_amount, actual_output_amount)) => {
            if confirmed_success {
                // Clone the amounts to avoid move errors
                let input_amount_str = actual_input_amount
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| swap_data.quote.in_amount.clone());
                let output_amount_str = actual_output_amount
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| swap_data.quote.out_amount.clone());

                log(
                    LogTag::Wallet,
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
                    // Use ACTUAL amounts from blockchain, not quote predictions
                    input_amount: actual_input_amount.unwrap_or_else(||
                        swap_data.quote.in_amount.clone()
                    ),
                    output_amount: actual_output_amount.unwrap_or_else(||
                        swap_data.quote.out_amount.clone()
                    ),
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
                    execution_time: swap_data.quote.time_taken,
                    effective_price: None, // Will be calculated later using actual amounts
                    swap_data: Some(swap_data), // Include the complete swap data
                    error: None,
                })
            } else {
                log(
                    LogTag::Wallet,
                    "FAILED",
                    &format!("❌ Transaction FAILED on-chain! TX: {}", transaction_signature)
                );

                Ok(SwapResult {
                    success: false,
                    transaction_signature: Some(transaction_signature),
                    input_amount: swap_data.quote.in_amount.clone(),
                    output_amount: "0".to_string(), // Zero output for failed transaction
                    price_impact: swap_data.quote.price_impact_pct.clone(),
                    fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
                    execution_time: swap_data.quote.time_taken,
                    effective_price: None,
                    swap_data: Some(swap_data),
                    error: Some("Transaction failed on-chain".to_string()),
                })
            }
        }
        Err(e) => {
            log(
                LogTag::Wallet,
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







/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>), SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        "⚠️ Manual ATA cleanup triggered (normally handled by background service)"
    );
    let wallet_address = get_wallet_address()?;
    close_all_empty_atas(&wallet_address).await
}

/// Gets current token price by requesting a small quote
pub async fn get_token_price_sol(token_mint: &str) -> Result<f64, SwapError> {
    let wallet_address = get_wallet_address()?;
    let small_amount = 0.001; // 0.001 SOL

    // Get best quote using the unified swap system
    let quote = crate::swaps::get_best_quote(
        SOL_MINT,
        token_mint,
        sol_to_lamports(small_amount),
        &wallet_address,
        1.0, // 1% slippage for price checking
        0.0, // No fee for quote
        false, // No anti-MEV for price checking
    ).await?;

    let output_tokens = quote.output_amount as f64;
    let price_per_token = (small_amount * 1_000_000_000.0) / output_tokens; // Price in lamports per token

    Ok(price_per_token / 1_000_000_000.0) // Convert back to SOL
}

/// Checks wallet balance for SOL
pub async fn get_sol_balance(wallet_address: &str) -> Result<f64, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_sol_balance(wallet_address).await
}

/// Checks wallet balance for a specific token
pub async fn get_token_balance(wallet_address: &str, mint: &str) -> Result<u64, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_token_balance(wallet_address, mint).await
}

/// Validates if the current price is near the expected price within tolerance
pub fn validate_price_near_expected(
    current_price: f64,
    expected_price: f64,
    tolerance_percent: f64
) -> bool {
    let price_difference = (((current_price - expected_price) / expected_price) * 100.0).abs();
    price_difference <= tolerance_percent
}

/// Calculates the effective price per token from a successful buy swap result
/// Returns the price in SOL per token based on actual input/output amounts
pub fn calculate_effective_price_buy(swap_result: &SwapResult) -> Result<f64, SwapError> {
    if !swap_result.success {
        return Err(SwapError::InvalidAmount("Cannot calculate price from failed swap".to_string()));
    }

    // Parse input amount (SOL in lamports)
    let input_lamports: u64 = swap_result.input_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid input amount in swap result".to_string()))?;

    // Parse output amount (tokens in smallest unit)
    let output_tokens_raw: u64 = swap_result.output_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid output amount in swap result".to_string()))?;

    if output_tokens_raw == 0 {
        return Err(
            SwapError::InvalidAmount("Cannot calculate price with zero token output".to_string())
        );
    }

    // Convert lamports to SOL
    let input_sol = lamports_to_sol(input_lamports);

    // Get the actual token decimals from swap data if available
    let token_decimals = if let Some(swap_data) = &swap_result.swap_data {
        swap_data.quote.out_decimals as u32
    } else {
        log(LogTag::Wallet, "ERROR", "Cannot calculate effective price without swap data decimals");
        return Err(SwapError::InvalidResponse("Missing decimals in swap data".to_string()));
    };

    // Convert raw token amount to actual tokens using correct decimals
    let output_tokens = (output_tokens_raw as f64) / (10_f64).powi(token_decimals as i32);

    // Calculate effective price: SOL spent / tokens received
    let effective_price = input_sol / output_tokens;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "💰 EFFECTIVE PRICE CALCULATION (BUY):\n  📥 Input: {} SOL ({} lamports)\n  📤 Output: {:.6} tokens ({} raw)\n  🔢 Token Decimals: {}\n  💎 Effective Price: {:.10} SOL per token",
                input_sol,
                input_lamports,
                output_tokens,
                output_tokens_raw,
                token_decimals,
                effective_price
            )
        );
    }

    Ok(effective_price)
}

/// Calculates the effective price per token from a successful sell swap result
/// Returns the price in SOL per token based on actual input/output amounts
pub fn calculate_effective_price_sell(swap_result: &SwapResult) -> Result<f64, SwapError> {
    if !swap_result.success {
        return Err(SwapError::InvalidAmount("Cannot calculate price from failed swap".to_string()));
    }

    // Parse input amount (tokens in smallest unit)
    let input_tokens_raw: u64 = swap_result.input_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid input amount in swap result".to_string()))?;

    // Parse output amount (SOL in lamports)
    let output_lamports: u64 = swap_result.output_amount
        .parse()
        .map_err(|_| SwapError::ParseError("Invalid output amount in swap result".to_string()))?;

    if input_tokens_raw == 0 {
        return Err(
            SwapError::InvalidAmount("Cannot calculate price with zero token input".to_string())
        );
    }

    // Convert lamports to SOL
    let output_sol = lamports_to_sol(output_lamports);

    // Get the actual token decimals from swap data if available
    let token_decimals = if let Some(swap_data) = &swap_result.swap_data {
        swap_data.quote.in_decimals as u32
    } else {
        log(LogTag::Wallet, "ERROR", "Cannot calculate effective price without swap data decimals");
        return Err(SwapError::InvalidResponse("Missing decimals in swap data".to_string()));
    };

    // Convert raw token amount to actual tokens using correct decimals
    let input_tokens = (input_tokens_raw as f64) / (10_f64).powi(token_decimals as i32);

    // Calculate effective price: SOL received / tokens sold
    let effective_price = output_sol / input_tokens;

    if is_debug_wallet_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "💰 EFFECTIVE PRICE CALCULATION (SELL):\n  📥 Input: {:.6} tokens ({} raw)\n  📤 Output: {} SOL ({} lamports)\n  � Token Decimals: {}\n  �💎 Effective Price: {:.10} SOL per token",
                input_tokens,
                input_tokens_raw,
                output_sol,
                output_lamports,
                token_decimals,
                effective_price
            )
        );
    }
    Ok(effective_price)
}

/// Validates the price from a swap quote against expected price
pub fn validate_quote_price(
    swap_data: &SwapData,
    input_amount: u64,
    expected_price: f64,
    is_sol_to_token: bool
) -> Result<(), SwapError> {
    let output_amount_str = &swap_data.quote.out_amount;
    log(
        LogTag::Wallet,
        "DEBUG",
        &format!("Quote validation - Raw out_amount string: '{}'", output_amount_str)
    );

    let output_amount_raw = output_amount_str.parse::<f64>().unwrap_or_else(|e| {
        log(
            LogTag::Wallet,
            "ERROR",
            &format!("Quote validation - Failed to parse out_amount '{}': {}", output_amount_str, e)
        );
        0.0
    });

    log(
        LogTag::Wallet,
        "DEBUG",
        &format!("Quote validation - Parsed output_amount_raw: {}", output_amount_raw)
    );

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

    log(
        LogTag::Wallet,
        "DEBUG",
        &format!(
            "Quote validation - Price calc debug: input_amount={}, output_amount_raw={}, output_decimals={}, actual_price={:.12}",
            input_amount,
            output_amount_raw,
            token_decimals,
            actual_price_per_token
        )
    );

    let price_difference = (
        ((actual_price_per_token - expected_price) / expected_price) *
        100.0
    ).abs();

    log(
        LogTag::Wallet,
        "PRICE",
        &format!(
            "Quote validation - Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
            expected_price,
            actual_price_per_token,
            price_difference
        )
    );

    if price_difference > SLIPPAGE_TOLERANCE_PERCENT {
        return Err(
            SwapError::SlippageExceeded(
                format!(
                    "Price difference {:.2}% exceeds slippage tolerance {:.2}%",
                    price_difference,
                    SLIPPAGE_TOLERANCE_PERCENT
                )
            )
        );
    }

    Ok(())
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(
    wallet_address: &str
) -> Result<Vec<crate::rpc::TokenAccountInfo>, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_all_token_accounts(wallet_address).await
}

/// Closes a single empty ATA (Associated Token Account) for a specific mint
/// Returns the transaction signature if successful
pub async fn close_single_ata(wallet_address: &str, mint: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close single ATA for mint {}", &mint[..8]));

    // Get all token accounts to find the specific one
    let token_accounts = get_all_token_accounts(wallet_address).await?;

    // Find the account for this mint
    let target_account = token_accounts
        .iter()
        .find(|account| account.mint == mint && account.balance == 0);

    match target_account {
        Some(account) => {
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Found empty ATA {} for mint {}", account.account, &mint[..8])
            );

            // Close the ATA
            match close_ata(wallet_address, &account.account, mint, account.is_token_2022).await {
                Ok(signature) => {
                    log(
                        LogTag::Wallet,
                        "SUCCESS",
                        &format!(
                            "Closed ATA {} for mint {}. TX: {}",
                            account.account,
                            &mint[..8],
                            signature
                        )
                    );
                    Ok(signature)
                }
                Err(e) => {
                    log(
                        LogTag::Wallet,
                        "ERROR",
                        &format!(
                            "Failed to close ATA {} for mint {}: {}",
                            account.account,
                            &mint[..8],
                            e
                        )
                    );
                    Err(e)
                }
            }
        }
        None => {
            let error_msg = format!("No empty ATA found for mint {}", &mint[..8]);
            log(LogTag::Wallet, "WARNING", &error_msg);
            Err(SwapError::InvalidAmount(error_msg))
        }
    }
}

/// Closes all empty ATAs (Associated Token Accounts) for a wallet
/// This reclaims the rent SOL (~0.002 SOL per account) from all empty token accounts
/// Returns the number of accounts closed and total signatures
pub async fn close_all_empty_atas(wallet_address: &str) -> Result<(u32, Vec<String>), SwapError> {
    log(LogTag::Wallet, "ATA", "🔍 Checking for empty token accounts to close...");

    // Get all token accounts for the wallet
    let all_accounts = get_all_token_accounts(wallet_address).await?;

    if all_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No token accounts found in wallet");
        return Ok((0, vec![]));
    }

    // Filter for empty accounts (balance = 0)
    let empty_accounts: Vec<&crate::rpc::TokenAccountInfo> = all_accounts
        .iter()
        .filter(|account| account.balance == 0)
        .collect();

    if empty_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No empty token accounts found to close");
        return Ok((0, vec![]));
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Found {} empty token accounts to close", empty_accounts.len())
    );

    let mut signatures = Vec::new();
    let mut closed_count = 0u32;

    // Close each empty account
    for account_info in empty_accounts {
        log(
            LogTag::Wallet,
            "ATA",
            &format!(
                "Closing empty {} account {} for mint {}",
                if account_info.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                account_info.account,
                account_info.mint
            )
        );

        match
            close_ata(
                wallet_address,
                &account_info.account,
                &account_info.mint,
                account_info.is_token_2022
            ).await
        {
            Ok(signature) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("✅ Closed empty ATA {}. TX: {}", account_info.account, signature)
                );
                signatures.push(signature);
                closed_count += 1;

                // Small delay between closures to avoid overwhelming the network
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("❌ Failed to close ATA {}: {}", account_info.account, e)
                );
                // Continue with other accounts even if one fails
            }
        }
    }

    let rent_reclaimed = (closed_count as f64) * 0.00203928; // Approximate ATA rent in SOL
    log(
        LogTag::Wallet,
        "ATA",
        &format!(
            "🎉 ATA cleanup complete! Closed {} accounts, reclaimed ~{:.6} SOL in rent",
            closed_count,
            rent_reclaimed
        )
    );

    Ok((closed_count, signatures))
}

/// Closes the Associated Token Account (ATA) for a given token mint after selling all tokens
/// This reclaims the rent SOL (~0.002 SOL) from empty token accounts
/// Supports both regular SPL tokens and Token-2022 tokens
pub async fn close_token_account(mint: &str, wallet_address: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close token account for mint: {}", mint));

    // First verify the token balance is actually zero
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => {
            if balance > 0 {
                return Err(
                    SwapError::InvalidAmount(
                        format!("Cannot close token account - still has {} tokens", balance)
                    )
                );
            }
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Verified zero balance for {}, proceeding to close ATA", mint)
            );
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not verify token balance before closing ATA: {}", e)
            );
            // Continue anyway - the close instruction will fail if tokens remain
        }
    }

    // Get the associated token account address
    let token_account = match get_associated_token_account(wallet_address, mint).await {
        Ok(account) => account,
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not find associated token account for {}: {}", mint, e)
            );
            return Err(e);
        }
    };

    log(LogTag::Wallet, "ATA", &format!("Found token account to close: {}", token_account));

    // Determine if this is a Token-2022 account by checking the token ACCOUNT's program (not the mint)
    let rpc_client = crate::rpc::get_rpc_client();
    let is_token_2022 = rpc_client
        .is_token_account_token_2022(&token_account).await
        .unwrap_or(false);

    if is_token_2022 {
        log(LogTag::Wallet, "ATA", "Detected Token-2022, using Token Extensions program");
    } else {
        log(LogTag::Wallet, "ATA", "Using standard SPL Token program");
    }

    // Create and send the close account instruction using GMGN API approach
    match close_ata(wallet_address, &token_account, mint, is_token_2022).await {
        Ok(signature) => {
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!("Successfully closed token account for {}. TX: {}", mint, signature)
            );
            Ok(signature)
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to close token account for {}: {}", mint, e)
            );
            Err(e)
        }
    }
}

/// Gets the associated token account address for a wallet and mint
async fn get_associated_token_account(
    wallet_address: &str,
    mint: &str
) -> Result<String, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_associated_token_account(wallet_address, mint).await
}

/// Closes ATA using proper Solana SDK for real ATA closing
async fn close_ata(
    wallet_address: &str,
    token_account: &str,
    mint: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        &format!("Closing ATA {} for mint {} using {} program", token_account, mint, if
            is_token_2022
        {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Use proper Solana SDK to build and send close instruction
    match build_and_send_close_instruction(wallet_address, token_account, is_token_2022).await {
        Ok(signature) => {
            log(LogTag::Wallet, "SUCCESS", &format!("ATA closed successfully. TX: {}", signature));
            Ok(signature)
        }
        Err(e) => {
            log(LogTag::Wallet, "ERROR", &format!("Failed to close ATA: {}", e));
            Err(e)
        }
    }
}

/// Builds and sends close account instruction using Solana SDK
async fn build_and_send_close_instruction(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Parse addresses
    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid wallet address: {}", e))
    )?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid token account: {}", e))
    )?;

    // Decode private key
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Build close account instruction
    let close_instruction = if is_token_2022 {
        // For Token-2022, use the Token Extensions program
        build_token_2022_close_instruction(&token_account_pubkey, &owner_pubkey)?
    } else {
        // For regular SPL tokens, use standard close_account instruction
        close_account(
            &spl_token::id(),
            &token_account_pubkey,
            &owner_pubkey,
            &owner_pubkey,
            &[]
        ).map_err(|e|
            SwapError::TransactionError(format!("Failed to build close instruction: {}", e))
        )?
    };

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Built close instruction for {} account", if is_token_2022 {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Get recent blockhash via RPC
    let rpc_client = crate::rpc::get_rpc_client();
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    // Build transaction
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash
    );

    log(LogTag::Wallet, "ATA", "Built and signed close transaction");

    // Send transaction via RPC
    rpc_client.send_transaction(&transaction).await
}

/// Builds close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey
) -> Result<Instruction, SwapError> {
    // Token-2022 uses the same close account instruction format as SPL Token
    // but with different program ID
    let token_2022_program_id = Pubkey::from_str(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ).map_err(|e| SwapError::TransactionError(format!("Invalid Token-2022 program ID: {}", e)))?;

    // Manually build the close account instruction for Token-2022
    // CloseAccount instruction: [9] (instruction discriminator)
    let instruction_data = vec![9u8]; // CloseAccount instruction ID

    let accounts = vec![
        AccountMeta::new(*token_account, false), // Token account to close
        AccountMeta::new(*owner, false), // Destination for lamports
        AccountMeta::new_readonly(*owner, true) // Authority (signer)
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}

/// Validates the price from a unified quote against expected price
pub fn validate_best_quote_price(
    quote: &crate::swaps::UnifiedQuote,
    expected_price: f64,
    is_sol_to_token: bool,
) -> Result<(), SwapError> {
    log(
        LogTag::Wallet,
        "VALIDATE",
        &format!(
            "Validating {:?} quote price - Expected: {:.12} SOL/token",
            quote.router,
            expected_price
        )
    );

    // Get token decimals from the execution data
    let (input_decimals, output_decimals) = match &quote.execution_data {
        crate::swaps::QuoteExecutionData::GMGN(gmgn_data) => {
            (gmgn_data.quote.in_decimals, gmgn_data.quote.out_decimals)
        }
        crate::swaps::QuoteExecutionData::Jupiter(_) => {
            // For Jupiter, we'll need to implement this later
            // For now, assume SOL (9 decimals) and common token decimals (6)
            if is_sol_to_token {
                (9u8, 6u8) // SOL to token
            } else {
                (6u8, 9u8) // Token to SOL
            }
        }
    };

    let actual_price_per_token = if is_sol_to_token {
        // For SOL to token: price = SOL spent / tokens received
        let input_sol = (quote.input_amount as f64) / (10_f64).powi(input_decimals as i32);
        let output_tokens = (quote.output_amount as f64) / (10_f64).powi(output_decimals as i32);
        if output_tokens > 0.0 {
            input_sol / output_tokens
        } else {
            0.0
        }
    } else {
        // For token to SOL: price = SOL received / tokens spent
        let input_tokens = (quote.input_amount as f64) / (10_f64).powi(input_decimals as i32);
        let output_sol = (quote.output_amount as f64) / (10_f64).powi(output_decimals as i32);
        if input_tokens > 0.0 {
            output_sol / input_tokens
        } else {
            0.0
        }
    };

    let price_difference = (((actual_price_per_token - expected_price) / expected_price) * 100.0).abs();

    log(
        LogTag::Wallet,
        "VALIDATE",
        &format!(
            "Quote validation - Expected {:.12} SOL/token, Actual {:.12} SOL/token, Diff: {:.2}%",
            expected_price,
            actual_price_per_token,
            price_difference
        )
    );

    if price_difference > SLIPPAGE_TOLERANCE_PERCENT {
        return Err(SwapError::SlippageExceeded(
            format!(
                "{:?} price difference {:.2}% exceeds tolerance {:.2}%",
                quote.router,
                price_difference,
                SLIPPAGE_TOLERANCE_PERCENT
            )
        ));
    }

    Ok(())
}
