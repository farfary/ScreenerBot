/// Main swap interface - clean single-purpose functions with transaction monitoring
/// This module provides the main swap functions used by the trading system

use crate::tokens::Token;
use crate::rpc::{SwapError, sol_to_lamports, lamports_to_sol};
use crate::logger::{log, LogTag};
use crate::global::{is_debug_swap_enabled};
use crate::utils::get_token_balance;
// Use utils instead of transaction module for wallet address
use crate::utils::get_wallet_address;
// Using wallet transaction manager instead of transactions_tools for analysis
use super::{get_best_quote, execute_best_swap, UnifiedSwapResult};
use super::types::{SwapData};
use super::config::{SOL_MINT, QUOTE_SLIPPAGE_PERCENT, SWAP_FEE_PERCENT, SELL_RETRY_SLIPPAGES, GMGN_DEFAULT_SWAP_MODE};

/// Result of a swap operation (compatible with existing wallet::SwapResult)
#[derive(Debug)]
pub struct SwapResult {
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
        GMGN_DEFAULT_SWAP_MODE, // Use config value instead of hardcoded
        SWAP_FEE_PERCENT,
        false, // Anti-MEV
    ).await?;

    if is_debug_swap_enabled() {
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

    let unified_result = execute_best_swap(
        token,
        SOL_MINT,
        &token.mint,
        sol_to_lamports(amount_sol),
        best_quote
    ).await?;

    // Convert UnifiedSwapResult to SwapResult for backward compatibility
    let mut swap_result = SwapResult {
        success: unified_result.success,
        transaction_signature: unified_result.transaction_signature,
        input_amount: unified_result.input_amount,
        output_amount: unified_result.output_amount,
        price_impact: unified_result.price_impact,
        fee_lamports: unified_result.fee_lamports,
        execution_time: unified_result.execution_time,
        effective_price: unified_result.effective_price,
        swap_data: unified_result.swap_data, // preserve swap_data for decimals/fallbacks
        error: unified_result.error,
    };

    // Add transaction to monitoring service if successful
    if swap_result.success {
        // Use wallet transaction manager for verification instead of standalone analysis
        if let Some(ref signature) = swap_result.transaction_signature {
            log(
                LogTag::Swap,
                "VERIFY", 
                &format!("🔍 Requesting wallet transaction manager to verify buy transaction: {}", signature)
            );
            
            match crate::wallet_transactions::verify_swap_transaction_global(signature, "buy").await {
                Ok(verified_data) => {
                    // Update swap result with verified data
                    swap_result.effective_price = Some(verified_data.effective_price);

                    log(
                        LogTag::Swap,
                        "VERIFIED",
                        &format!(
                            "✅ BUY TRANSACTION VERIFIED - Effective Price: {:.10} SOL per {} token | Token Amount: {} | SOL Amount: {:.6}",
                            verified_data.effective_price,
                            token.symbol,
                            verified_data.token_amount,
                            verified_data.sol_amount
                        )
                    );

                    if is_debug_swap_enabled() {
                        if let Some(expected) = expected_price {
                            let price_diff = ((verified_data.effective_price - expected) / expected) * 100.0;
                            log(
                                LogTag::Swap,
                                "PRICE",
                                &format!(
                                    "Price vs expected: {:.10} vs {:.10} SOL ({:+.2}%)",
                                    verified_data.effective_price,
                                    expected,
                                    price_diff
                                )
                            );
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Swap,
                        "ERROR", 
                        &format!("❌ Failed to verify buy transaction {}: {}", signature, e)
                    );
                    // Don't fail the swap, but log the verification failure
                }
            }
        } else {
            log(LogTag::Swap, "WARNING", "No transaction signature available for wallet transaction manager verification");
        }

        // Update wallet tracker after successful buy
        crate::wallet_tracker::update_wallet_after_swap().await;
    }

    if is_debug_swap_enabled() {
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
    expected_sol_output: Option<f64>
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

        match sell_token_with_slippage(token, token_amount, expected_sol_output, slippage).await {
            Ok(result) => {
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

                // If this isn't the last attempt, wait and clear recent attempt to allow retry
                if attempt < slippages.len() - 1 {
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
    expected_sol_output: Option<f64>,
    slippage: f64
) -> Result<SwapResult, SwapError> {
    // Simplified approach (no complex transaction monitoring)
    if is_debug_swap_enabled() {
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
        GMGN_DEFAULT_SWAP_MODE, // Use config value instead of hardcoded
        SWAP_FEE_PERCENT,
        false,
    ).await?;

    // Execute the swap
    let unified_result = crate::swaps::execute_best_swap(
        token,
        &token.mint,
        SOL_MINT,
        actual_sell_amount,
        best_quote,
    ).await?;

    // Convert UnifiedSwapResult to SwapResult for compatibility
    let mut swap_result = SwapResult {
        success: unified_result.success,
        transaction_signature: unified_result.transaction_signature,
        input_amount: unified_result.input_amount,
        output_amount: unified_result.output_amount,
        price_impact: unified_result.price_impact,
        fee_lamports: unified_result.fee_lamports,
        execution_time: unified_result.execution_time,
        effective_price: unified_result.effective_price,
        swap_data: unified_result.swap_data, // carry decimals/quote for fallbacks
        error: unified_result.error,
    };

    // Use wallet transaction manager for verification instead of standalone analysis
    if swap_result.success {
        if let Some(ref signature) = swap_result.transaction_signature {
            log(
                LogTag::Swap,
                "VERIFY", 
                &format!("🔍 Requesting wallet transaction manager to verify sell transaction: {}", signature)
            );
            
            match crate::wallet_transactions::verify_swap_transaction_global(signature, "sell").await {
                Ok(verified_data) => {
                    swap_result.effective_price = Some(verified_data.effective_price);
                    
                    log(
                        LogTag::Swap,
                        "VERIFIED",
                        &format!(
                            "✅ SELL TRANSACTION VERIFIED - Effective Price: {:.10} SOL per {} token | Token Amount: {} | SOL Amount: {:.6}",
                            verified_data.effective_price,
                            token.symbol,
                            verified_data.token_amount,
                            verified_data.sol_amount
                        )
                    );
                }
                Err(e) => {
                    log(
                        LogTag::Swap, 
                        "ERROR", 
                        &format!("❌ Failed to verify sell transaction {}: {}", signature, e)
                    );
                    
                    // Fallback to original method with warning
                    log(LogTag::Swap, "WARNING", "Using fallback price calculation due to verification failure");
                    let input_tokens_raw: u64 = swap_result.input_amount.parse().unwrap_or(0);
                    let output_lamports: u64 = swap_result.output_amount.parse().unwrap_or(0);
                    
                    if input_tokens_raw > 0 && output_lamports > 0 {
                        let token_decimals = crate::tokens::get_token_decimals(&token.mint).await.unwrap_or(9);
                        let input_tokens = (input_tokens_raw as f64) / (10_f64).powi(token_decimals as i32);
                        let output_sol = lamports_to_sol(output_lamports);
                        let effective_price = output_sol / input_tokens;
                        
                        swap_result.effective_price = Some(effective_price);
                        
                        log(
                            LogTag::Swap,
                            "PRICE",
                            &format!(
                                "⚠️ SELL COMPLETED - Effective Price: {:.10} SOL per {} token (fallback - may be inaccurate)",
                                effective_price,
                                token.symbol
                            )
                        );
                    }
                }
            }
        } else {
            log(LogTag::Swap, "WARNING", "No transaction signature available for wallet transaction manager verification");
        }

        // Update wallet tracker after successful sell
        crate::wallet_tracker::update_wallet_after_swap().await;
    }

    if is_debug_swap_enabled() {
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
