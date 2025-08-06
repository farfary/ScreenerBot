/// Transaction management and verification for swap operations
/// Handles transaction slot reservation, confirmation tracking, and result verification

use crate::global::{read_configs, is_debug_wallet_enabled, is_debug_swap_enabled};
use crate::tokens::Token;
use crate::logger::{log, LogTag};
use crate::rpc::{get_premium_transaction_rpc, SwapError, lamports_to_sol, sol_to_lamports};
use crate::swaps::types::{SwapData, SwapRequest, SwapApiResponse, SOL_MINT};
use crate::swaps::{
    INITIAL_CONFIRMATION_DELAY_MS, MAX_CONFIRMATION_DELAY_SECS, CONFIRMATION_BACKOFF_MULTIPLIER,
    CONFIRMATION_TIMEOUT_SECS, RATE_LIMIT_BASE_DELAY_SECS, RATE_LIMIT_INCREMENT_SECS,
    EARLY_ATTEMPT_DELAY_MS, EARLY_ATTEMPTS_COUNT
};

use std::collections::HashSet;
use std::sync::{Arc as StdArc, Mutex as StdMutex};
use once_cell::sync::Lazy;
use bs58;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::str::FromStr;

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
