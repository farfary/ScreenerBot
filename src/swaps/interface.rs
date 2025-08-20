/// Main swap interface - clean single-purpose functions with transaction monitoring
/// This module provides the main swap functions used by the trading system
use crate::tokens::Token;
use crate::rpc::{SwapError, sol_to_lamports, lamports_to_sol};
use crate::logger::{log, LogTag};
use crate::global::{is_debug_swaps_enabled};
use crate::utils::{get_token_balance, check_shutdown_or_delay};
use crate::utils::get_wallet_address;
use super::{get_best_quote, execute_best_swap, RouterType};
use super::types::{SwapData};
use super::config::{SOL_MINT, QUOTE_SLIPPAGE_PERCENT, GMGN_FEE_SOL, SELL_RETRY_SLIPPAGES, GMGN_DEFAULT_SWAP_MODE};
use std::sync::Arc;
use tokio::sync::Notify;

/// Enhanced swap result with comprehensive routing information
#[derive(Debug)]
pub struct SwapResult {
    pub success: bool,
    pub router_used: Option<RouterType>, // Track which router was used for the swap
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

/// Buy tokens with SOL - single purpose function
pub async fn buy_token(
    token: &Token,
    amount_sol: f64,
    expected_price: Option<f64>
) -> Result<SwapResult, SwapError> {
    // CRITICAL SAFETY CHECK: Validate expected price if provided
    if let Some(price) = expected_price {
        if price <= 0.0 || !price.is_finite() {
            log(
                LogTag::Swap,
                "ERROR",
                &format!(
                    "❌ REFUSING TO BUY: Invalid expected_price for {} ({}). Price = {:.10}",
                    token.symbol,
                    token.mint,
                    price
                )
            );
            return Err(SwapError::InvalidAmount(format!("Invalid expected price: {:.10}", price)));
        }
    }

    // Simplified anti-spam protection (no complex transaction monitoring)
    log(
        LogTag::Swap,
        "BUY_START",
        &format!(
            "🟢 BUYING {} SOL worth of {} tokens (mint: {})",
            amount_sol,
            token.symbol,
            token.mint
        )
    );

    // Get wallet address
    let wallet_address = get_wallet_address()?;

    // Get the best quote from all available routers
    let best_quote = get_best_quote(
        SOL_MINT,
        &token.mint,
        sol_to_lamports(amount_sol),
        &wallet_address,
        QUOTE_SLIPPAGE_PERCENT,
    ).await?;

    if is_debug_swaps_enabled() {
        log(
            LogTag::Swap,
            "QUOTE",
            &format!(
                "📊 Best quote from {:?}: {} SOL -> {} tokens",
                best_quote.router,
                lamports_to_sol(best_quote.input_amount),
                best_quote.output_amount
            )
        );
    }

    // Execute the swap
    log(LogTag::Swap, "SWAP", &format!("🚀 Executing swap with best quote via {:?}...", best_quote.router));

    let swap_result = execute_best_swap(
        token,
        SOL_MINT,
        &token.mint,
        sol_to_lamports(amount_sol),
        best_quote
    ).await?;

    // Log transaction signature (no longer using priority queue)
    if let Some(ref signature) = swap_result.transaction_signature {
        log(LogTag::Swap, "TRANSACTION", &format!(
            "Transaction {} will be monitored by positions manager", 
            &signature[..8]
        ));
    }

    if is_debug_swaps_enabled() {
        log(
            LogTag::Swap,
            "BUY_COMPLETE",
            &format!(
                "🟢 BUY operation completed for {} - Success: {} | TX: {}",
                token.symbol,
                swap_result.success,
                swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
            )
        );
    }

    Ok(swap_result)
}

/// Sell tokens for SOL with automatic slippage retry - single purpose function
pub async fn sell_token(
    token: &Token,
    token_amount: u64, // Position amount (used for validation only - actual sale uses full wallet balance)
    expected_sol_output: Option<f64>,
    shutdown: Option<Arc<Notify>>
) -> Result<SwapResult, SwapError> {
    // CRITICAL SAFETY CHECK: Validate expected SOL output if provided
    if let Some(expected_sol) = expected_sol_output {
        if expected_sol <= 0.0 || !expected_sol.is_finite() {
            return Err(SwapError::InvalidAmount(
                format!("Invalid expected SOL output: {:.10}", expected_sol)
            ));
        }
    }

    // Auto-retry with progressive slippage from config
    let slippages = &SELL_RETRY_SLIPPAGES;

    for (attempt, &slippage) in slippages.iter().enumerate() {
        // Abort before starting a new attempt if shutdown is in progress
        if let Some(ref s) = shutdown {
            if check_shutdown_or_delay(s, tokio::time::Duration::from_millis(0)).await {
                log(
                    LogTag::Swap,
                    "SHUTDOWN",
                    &format!(
                        "⏹️  Aborting further sell attempts for {} due to shutdown (before attempt {} with {:.1}% slippage)",
                        token.symbol,
                        attempt + 1,
                        slippage
                    )
                );
                return Err(SwapError::ConfigError("Shutdown in progress - aborting sell".to_string()));
            }
        }
        log(
            LogTag::Swap,
            "SELL_ATTEMPT",
            &format!(
                "🔴 Sell attempt {} for {} with {:.1}% slippage",
                attempt + 1,
                token.symbol,
                slippage
            )
        );

    match sell_token_with_slippage(token, token_amount, slippage).await {
            Ok(result) => {
                // Add transaction to monitoring if we have a signature
                if let Some(ref signature) = result.transaction_signature {
                    log(LogTag::Swap, "TRANSACTION", &format!(
                        "Sell transaction {} will be monitored by positions manager", 
                        &signature[..8]
                    ));
                }
                
                log(
                    LogTag::Swap,
                    "SELL_SUCCESS",
                    &format!(
                        "✅ Sell successful for {} on attempt {} with {:.1}% slippage",
                        token.symbol,
                        attempt + 1,
                        slippage
                    )
                );
                return Ok(result);
            }
            Err(e) => {
                log(
                    LogTag::Swap,
                    "SELL_RETRY",
                    &format!(
                        "⚠️ Sell attempt {} failed for {} with {:.1}% slippage: {}",
                        attempt + 1,
                        token.symbol,
                        slippage,
                        e
                    )
                );

                // Check for error types that should not be retried
                match &e {
                    SwapError::InsufficientBalance(_) => {
                        log(
                            LogTag::Swap,
                            "SELL_FAILED_NO_RETRY",
                            &format!(
                                "❌ Stopping retries for {} - insufficient balance (tokens may have been sold in previous attempt)",
                                token.symbol
                            )
                        );
                        return Err(e);
                    }
                    SwapError::InvalidAmount(_) => {
                        log(
                            LogTag::Swap,
                            "SELL_FAILED_NO_RETRY",
                            &format!(
                                "❌ Stopping retries for {} - invalid amount (unretryable error)",
                                token.symbol
                            )
                        );
                        return Err(e);
                    }
                    SwapError::ConfigError(_) => {
                        log(
                            LogTag::Swap,
                            "SELL_FAILED_NO_RETRY",
                            &format!(
                                "❌ Stopping retries for {} - configuration error (unretryable error)",
                                token.symbol
                            )
                        );
                        return Err(e);
                    }
                    _ => {
                        // Continue with retry logic for other error types
                    }
                }

                // If this isn't the last attempt, wait and clear recent attempt to allow retry
                if attempt < slippages.len() - 1 {
                    // Before retry delay, check for shutdown and abort if requested
                    if let Some(ref s) = shutdown {
                        if check_shutdown_or_delay(s, tokio::time::Duration::from_millis(0)).await {
                            log(
                                LogTag::Swap,
                                "SHUTDOWN",
                                &format!(
                                    "⏹️  Skipping sell retry for {} due to shutdown (next slippage would be {:.1}%)",
                                    token.symbol,
                                    slippages[attempt + 1]
                                )
                            );
                            return Err(SwapError::ConfigError("Shutdown in progress - aborting sell retries".to_string()));
                        }
                    }

                    // Wait before retry (simplified - no transaction attempt clearing)
                    tokio::time::sleep(tokio::time::Duration::from_secs((attempt + 1) as u64 * 2)).await;
                } else {
                    // Last attempt failed
                    log(
                        LogTag::Swap,
                        "SELL_FAILED",
                        &format!(
                            "❌ All sell attempts failed for {} after {} tries",
                            token.symbol,
                            slippages.len()
                        )
                    );
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

/// Internal sell function with specific slippage
async fn sell_token_with_slippage(
    token: &Token,
    token_amount: u64, // Position amount (used for validation only - actual sale uses full wallet balance)
    slippage: f64
) -> Result<SwapResult, SwapError> {
    // Simplified approach (no complex transaction monitoring)
    if is_debug_swaps_enabled() {
        log(
            LogTag::Swap,
            "SELL_START",
            &format!(
                "🔴 Starting SELL operation for {} ({}) - Expected amount: {} tokens, Slippage: {:.1}%",
                token.symbol,
                token.mint,
                token_amount,
                slippage
            )
        );
    }

    // Get wallet address
    let wallet_address = get_wallet_address()?;

    // Get actual wallet balance (sell ALL tokens, not just position amount)
    let actual_wallet_balance = get_token_balance(&wallet_address, &token.mint).await?;

    if actual_wallet_balance == 0 {
        log(
            LogTag::Swap,
            "WARNING",
            &format!(
                "⚠️ No {} tokens in wallet to sell (expected: {}, actual: 0)",
                token.symbol,
                token_amount
            )
        );
        return Err(SwapError::InsufficientBalance(
            format!("No {} tokens in wallet", token.symbol)
        ));
    }

    // Use actual wallet balance, not position amount
    let actual_sell_amount = actual_wallet_balance;
    
    log(
        LogTag::Swap,
        "SELL_AMOUNT",
        &format!(
            "💰 Selling {} {} tokens (position: {}, wallet: {})",
            actual_sell_amount,
            token.symbol,
            token_amount,
            actual_wallet_balance
        )
    );

    // Get the best quote
    let best_quote = crate::swaps::get_best_quote(
        &token.mint,
        SOL_MINT,
        actual_sell_amount,
        &wallet_address,
        slippage,
    ).await?;

    // Execute the swap
    let swap_result = crate::swaps::execute_best_swap(
        token,
        &token.mint,
        SOL_MINT,
        actual_sell_amount,
        best_quote,
    ).await?;

    if is_debug_swaps_enabled() {
        log(
            LogTag::Swap,
            "SELL_COMPLETE",
            &format!(
                "🔴 SELL operation completed for {} - Success: {} | TX: {}",
                token.symbol,
                swap_result.success,
                swap_result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
            )
        );
    }

    Ok(swap_result)
}

