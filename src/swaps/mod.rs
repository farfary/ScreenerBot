/// Swap module for handling multiple DEX routers
/// Supports GMGN and Jupiter routers with unified interface
///
/// All configuration now centralized in config module - use with_config()
/// All constants migrated to centralized config system

pub mod gmgn;
pub mod jupiter;
pub mod types;

use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::errors::{ BlockchainError, ScreenerBotError };
use crate::logger::{ log, LogTag };
use crate::tokens::Token;
use futures::future;
use std::future::Future;
use std::pin::Pin;

// =============================================================================
// RE-EXPORTS - Clean interface for external use
// =============================================================================

// Common types and structures
pub use types::{
    GMGNApiResponse,
    JupiterQuoteResponse,
    JupiterSwapResponse,
    RawTransaction,
    RouterType,
    SwapData,
    SwapQuote,
    SwapRequest,
    SwapResult,
};

// Router-specific functions
pub use gmgn::{ execute_gmgn_swap, get_gmgn_quote, gmgn_sign_and_send_transaction, GMGNSwapResult };
pub use jupiter::{
    execute_jupiter_swap,
    get_jupiter_quote,
    jupiter_sign_and_send_transaction,
    JupiterSwapResult,
};

// =============================================================================
// UNIFIED ROUTER INTERFACE
// =============================================================================

// RouterType is now defined in types.rs

/// Unified quote structure for comparing routes
#[derive(Debug, Clone)]
pub struct UnifiedQuote {
    pub router: RouterType,
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
    pub price_impact_pct: f64,
    pub fee_lamports: u64,
    pub slippage_bps: u16,
    pub route_plan: String,
    pub execution_data: QuoteExecutionData,
    pub swap_mode: String,
}

/// Router-specific execution data
#[derive(Debug, Clone)]
pub enum QuoteExecutionData {
    GMGN(types::SwapData),
    Jupiter(types::SwapData),
}

// =============================================================================
// SIMPLIFIED BEST QUOTE FUNCTIONS
// =============================================================================

/// Get the best quote from available routers (TRUE COMPARISON IMPLEMENTATION - CONCURRENT)
pub async fn get_best_quote(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    from_address: &str,
    slippage: f64,
    swap_mode: &str
) -> Result<UnifiedQuote, ScreenerBotError> {
    log(
        LogTag::Swap,
        "BEST_QUOTE",
        &format!(
            "🔍 Finding best route: {} -> {} (amount: {}) - CONCURRENT QUOTES",
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                &input_mint[..8]
            },
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                &output_mint[..8]
            },
            input_amount
        )
    );

    let mut futures: Vec<
        Pin<Box<dyn Future<Output = Result<UnifiedQuote, ScreenerBotError>> + Send>>
    > = Vec::new();

    let gmgn_enabled = with_config(|cfg| cfg.swaps.gmgn_enabled);
    let jupiter_enabled = with_config(|cfg| cfg.swaps.jupiter_enabled);

    // Prepare GMGN quote future
    if gmgn_enabled {
        log(LogTag::Swap, "QUOTE_GMGN_START", "🔵 Starting GMGN quote request...");
        let gmgn_future = async {
            match
                gmgn::get_gmgn_quote(
                    input_mint,
                    output_mint,
                    input_amount,
                    from_address,
                    slippage,
                    swap_mode
                ).await
            {
                Ok(gmgn_data) => {
                    let unified_quote = UnifiedQuote {
                        router: RouterType::GMGN,
                        input_mint: input_mint.to_string(),
                        output_mint: output_mint.to_string(),
                        input_amount,
                        output_amount: gmgn_data.quote.out_amount.parse().unwrap_or(0),
                        price_impact_pct: gmgn_data.quote.price_impact_pct.parse().unwrap_or(0.0),
                        fee_lamports: gmgn_data.raw_tx.prioritization_fee_lamports,
                        slippage_bps: gmgn_data.quote.slippage_bps.parse().unwrap_or(0),
                        route_plan: format!(
                            "GMGN Route: {}",
                            serde_json::to_string(&gmgn_data.quote.route_plan).unwrap_or_default()
                        ),
                        execution_data: QuoteExecutionData::GMGN(gmgn_data),
                        swap_mode: swap_mode.to_string(),
                    };

                    log(
                        LogTag::Swap,
                        "QUOTE_GMGN_SUCCESS",
                        &format!(
                            "✅ GMGN quote: {} tokens, impact: {:.2}%, fee: {} lamports",
                            unified_quote.output_amount,
                            unified_quote.price_impact_pct,
                            unified_quote.fee_lamports
                        )
                    );

                    Ok(unified_quote)
                }
                Err(e) => {
                    log(LogTag::Swap, "QUOTE_GMGN_ERROR", &format!("❌ GMGN quote failed: {}", e));
                    Err(e)
                }
            }
        };
        futures.push(Box::pin(gmgn_future));
    } else {
        log(LogTag::Swap, "QUOTE_GMGN_DISABLED", "⏸️ GMGN router disabled in config");
    }

    // Prepare Jupiter quote future
    if jupiter_enabled {
        log(LogTag::Swap, "QUOTE_JUPITER_START", "🟡 Starting Jupiter quote request...");
        let jupiter_future = async {
            match
                jupiter::get_jupiter_quote(
                    input_mint,
                    output_mint,
                    input_amount,
                    slippage,
                    swap_mode
                ).await
            {
                Ok(jupiter_data) => {
                    let unified_quote = UnifiedQuote {
                        router: RouterType::Jupiter,
                        input_mint: input_mint.to_string(),
                        output_mint: output_mint.to_string(),
                        input_amount,
                        output_amount: jupiter_data.quote.out_amount.parse().unwrap_or(0),
                        price_impact_pct: jupiter_data.quote.price_impact_pct
                            .parse()
                            .unwrap_or(0.0),
                        fee_lamports: jupiter_data.raw_tx.prioritization_fee_lamports,
                        slippage_bps: jupiter_data.quote.slippage_bps.parse().unwrap_or(0),
                        route_plan: format!(
                            "Jupiter Route: {}",
                            serde_json
                                ::to_string(&jupiter_data.quote.route_plan)
                                .unwrap_or_default()
                        ),
                        execution_data: QuoteExecutionData::Jupiter(jupiter_data),
                        swap_mode: swap_mode.to_string(),
                    };

                    log(
                        LogTag::Swap,
                        "QUOTE_JUPITER_SUCCESS",
                        &format!(
                            "✅ Jupiter quote: {} tokens, impact: {:.2}%, fee: {} lamports",
                            unified_quote.output_amount,
                            unified_quote.price_impact_pct,
                            unified_quote.fee_lamports
                        )
                    );

                    Ok(unified_quote)
                }
                Err(e) => {
                    log(
                        LogTag::Swap,
                        "QUOTE_JUPITER_ERROR",
                        &format!("❌ Jupiter quote failed: {}", e)
                    );
                    Err(e)
                }
            }
        };
        futures.push(Box::pin(jupiter_future));
    } else {
        log(LogTag::Swap, "QUOTE_JUPITER_DISABLED", "⏸️ Jupiter router disabled in config");
    }

    // Execute all quote requests concurrently
    log(
        LogTag::Swap,
        "CONCURRENT_EXECUTION",
        &format!("⚡ Executing {} quote requests concurrently...", futures.len())
    );

    let results = future::join_all(futures).await;

    // Collect successful quotes
    let mut quotes = Vec::new();
    for result in results {
        if let Ok(quote) = result {
            quotes.push(quote);
        }
    }

    // Check if we have any quotes
    if quotes.is_empty() {
        let error_msg = "No routers available for quote - GMGN and Jupiter all failed";
        log(LogTag::Swap, "QUOTE_ERROR", &format!("❌ {}", error_msg));

        // Log detailed failure summary for debugging
        log(
            LogTag::Swap,
            "FAILURE_SUMMARY",
            &format!(
                "🔍 Quote failure summary - GMGN: {}, Jupiter: {} (check token liquidity and API status)",
                if gmgn_enabled {
                    "enabled but failed"
                } else {
                    "disabled"
                },
                if jupiter_enabled {
                    "enabled but failed"
                } else {
                    "disabled"
                }
            )
        );

        return Err(ScreenerBotError::api_error(error_msg.to_string()));
    }

    // Compare quotes and select the best one (highest output amount = better rate)
    let best_quote = quotes
        .iter()
        .max_by_key(|q| q.output_amount)
        .cloned()
        .ok_or_else(|| ScreenerBotError::api_error("Failed to select best quote".to_string()))?;

    // Log comparison results if we have multiple quotes
    if quotes.len() > 1 {
        log(
            LogTag::Swap,
            "QUOTE_COMPARISON",
            &format!("⚖️ Quote comparison: GMGN vs Jupiter - Winner: {:?}", best_quote.router)
        );

        // Show detailed comparison
        for quote in &quotes {
            log(
                LogTag::Swap,
                "QUOTE_DETAILS",
                &format!(
                    "  • {:?}: {} tokens (impact: {:.2}%, fee: {} lamports)",
                    quote.router,
                    quote.output_amount,
                    quote.price_impact_pct,
                    quote.fee_lamports
                )
            );
        }
    }

    log(
        LogTag::Swap,
        "BEST_ROUTE",
        &format!(
            "🏆 Best route selected: {:?} with {} tokens (impact: {:.2}%, fee: {} lamports)",
            best_quote.router,
            best_quote.output_amount,
            best_quote.price_impact_pct,
            best_quote.fee_lamports
        )
    );

    Ok(best_quote)
}

/// Execute swap with unified quote (with fallback retry mechanism)
pub async fn execute_best_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    quote: UnifiedQuote
) -> Result<SwapResult, ScreenerBotError> {
    log(
        LogTag::Swap,
        "EXECUTE",
        &format!(
            "🚀 Executing swap via {:?}: {} -> {} (amount: {})",
            quote.router,
            if input_mint == SOL_MINT {
                "SOL"
            } else {
                &input_mint[..8]
            },
            if output_mint == SOL_MINT {
                "SOL"
            } else {
                &output_mint[..8]
            },
            input_amount
        )
    );

    // Try primary router first
    let primary_result = match quote.execution_data {
        QuoteExecutionData::GMGN(ref gmgn_data) => {
            match
                gmgn::execute_gmgn_swap(
                    token,
                    input_mint,
                    output_mint,
                    input_amount,
                    gmgn_data.clone()
                ).await
            {
                Ok(result) =>
                    Ok(SwapResult {
                        success: result.success,
                        router_used: Some(RouterType::GMGN),
                        transaction_signature: result.transaction_signature,
                        input_amount: result.input_amount,
                        output_amount: result.output_amount,
                        price_impact: result.price_impact,
                        fee_lamports: result.fee_lamports,
                        execution_time: result.execution_time,
                        effective_price: result.effective_price,
                        swap_data: result.swap_data,
                        error: result.error,
                    }),
                Err(e) => Err(e),
            }
        }
        QuoteExecutionData::Jupiter(ref jupiter_data) => {
            match
                jupiter::execute_jupiter_swap(
                    token,
                    input_mint,
                    output_mint,
                    jupiter_data.clone()
                ).await
            {
                Ok(result) =>
                    Ok(SwapResult {
                        success: result.success,
                        router_used: Some(RouterType::Jupiter),
                        transaction_signature: result.transaction_signature,
                        input_amount: result.input_amount,
                        output_amount: result.output_amount,
                        price_impact: result.price_impact,
                        fee_lamports: result.fee_lamports,
                        execution_time: result.execution_time,
                        effective_price: result.effective_price,
                        swap_data: result.swap_data,
                        error: result.error,
                    }),
                Err(e) => Err(e),
            }
        }
    };

    // Check if primary router failed and fallback is available
    if let Err(ref primary_error) = primary_result {
        log(
            LogTag::Swap,
            "FALLBACK_TRIGGERED",
            &format!("⚠️ Primary router {:?} failed: {}", quote.router, primary_error)
        );

        // Only try fallback for certain error types (propagation failures, transaction errors)
        let should_fallback = match primary_error {
            ScreenerBotError::Blockchain(BlockchainError::TransactionDropped { reason, .. }) if
                reason.contains("not propagated")
            => true,
            ScreenerBotError::Blockchain(BlockchainError::TransactionDropped { reason, .. }) if
                reason.contains("dropped")
            => true,
            ScreenerBotError::Network(_) => true,
            _ => false,
        };

        if should_fallback {
            log(
                LogTag::Swap,
                "FALLBACK_ATTEMPT",
                "🔄 Attempting fallback to alternative router..."
            );

            // Get fallback quote from the other router
            let wallet_address = match crate::configs::read_configs() {
                Ok(configs) =>
                    match crate::configs::get_wallet_pubkey_string(&configs) {
                        Ok(addr) => addr,
                        Err(_) => {
                            return primary_result;
                        } // If can't get wallet, return original error
                    }
                Err(_) => {
                    return primary_result;
                } // If can't get wallet, return original error
            };

            let gmgn_enabled_fallback = with_config(|cfg| cfg.swaps.gmgn_enabled);
            let jupiter_enabled_fallback = with_config(|cfg| cfg.swaps.jupiter_enabled);

            let fallback_quote = match quote.router {
                RouterType::Jupiter => {
                    // Jupiter failed, try GMGN
                    if gmgn_enabled_fallback {
                        log(LogTag::Swap, "FALLBACK_GMGN", "🔵 Falling back to GMGN router...");

                        match
                            gmgn::get_gmgn_quote(
                                input_mint,
                                output_mint,
                                input_amount,
                                &wallet_address,
                                (quote.slippage_bps as f64) / 100.0, // Convert bps to percentage
                                &quote.swap_mode
                            ).await
                        {
                            Ok(gmgn_data) => {
                                log(
                                    LogTag::Swap,
                                    "FALLBACK_QUOTE_SUCCESS",
                                    &format!(
                                        "✅ GMGN fallback quote: {} tokens, impact: {:.2}%",
                                        gmgn_data.quote.out_amount,
                                        gmgn_data.quote.price_impact_pct
                                            .parse::<f64>()
                                            .unwrap_or(0.0)
                                    )
                                );
                                Some(gmgn_data)
                            }
                            Err(e) => {
                                log(
                                    LogTag::Swap,
                                    "FALLBACK_QUOTE_FAILED",
                                    &format!("❌ GMGN fallback quote failed: {}", e)
                                );
                                None
                            }
                        }
                    } else {
                        log(
                            LogTag::Swap,
                            "FALLBACK_UNAVAILABLE",
                            "❌ GMGN fallback not available (disabled)"
                        );
                        None
                    }
                }
                RouterType::GMGN => {
                    // GMGN failed, try Jupiter
                    if jupiter_enabled_fallback {
                        log(
                            LogTag::Swap,
                            "FALLBACK_JUPITER",
                            "🟡 Falling back to Jupiter router..."
                        );

                        match
                            jupiter::get_jupiter_quote(
                                input_mint,
                                output_mint,
                                input_amount,
                                (quote.slippage_bps as f64) / 100.0, // Convert bps to percentage
                                &quote.swap_mode
                            ).await
                        {
                            Ok(jupiter_data) => {
                                log(
                                    LogTag::Swap,
                                    "FALLBACK_QUOTE_SUCCESS",
                                    &format!(
                                        "✅ Jupiter fallback quote: {} tokens, impact: {:.2}%",
                                        jupiter_data.quote.out_amount,
                                        jupiter_data.quote.price_impact_pct
                                            .parse::<f64>()
                                            .unwrap_or(0.0)
                                    )
                                );
                                Some(jupiter_data)
                            }
                            Err(e) => {
                                log(
                                    LogTag::Swap,
                                    "FALLBACK_QUOTE_FAILED",
                                    &format!("❌ Jupiter fallback quote failed: {}", e)
                                );
                                None
                            }
                        }
                    } else {
                        log(
                            LogTag::Swap,
                            "FALLBACK_UNAVAILABLE",
                            "❌ Jupiter fallback not available (disabled)"
                        );
                        None
                    }
                }
            };

            // Execute fallback if we got a quote
            if let Some(fallback_data) = fallback_quote {
                let fallback_result = match quote.router {
                    RouterType::Jupiter => {
                        // Fallback to GMGN
                        log(LogTag::Swap, "FALLBACK_EXECUTE", "🔵 Executing GMGN fallback swap...");
                        match
                            gmgn::execute_gmgn_swap(
                                token,
                                input_mint,
                                output_mint,
                                input_amount,
                                fallback_data
                            ).await
                        {
                            Ok(result) =>
                                Ok(SwapResult {
                                    success: result.success,
                                    router_used: Some(RouterType::GMGN),
                                    transaction_signature: result.transaction_signature,
                                    input_amount: result.input_amount,
                                    output_amount: result.output_amount,
                                    price_impact: result.price_impact,
                                    fee_lamports: result.fee_lamports,
                                    execution_time: result.execution_time,
                                    effective_price: result.effective_price,
                                    swap_data: result.swap_data,
                                    error: result.error,
                                }),
                            Err(e) => Err(e),
                        }
                    }
                    RouterType::GMGN => {
                        // Fallback to Jupiter
                        log(
                            LogTag::Swap,
                            "FALLBACK_EXECUTE",
                            "🟡 Executing Jupiter fallback swap..."
                        );
                        match
                            jupiter::execute_jupiter_swap(
                                token,
                                input_mint,
                                output_mint,
                                fallback_data
                            ).await
                        {
                            Ok(result) =>
                                Ok(SwapResult {
                                    success: result.success,
                                    router_used: Some(RouterType::Jupiter),
                                    transaction_signature: result.transaction_signature,
                                    input_amount: result.input_amount,
                                    output_amount: result.output_amount,
                                    price_impact: result.price_impact,
                                    fee_lamports: result.fee_lamports,
                                    execution_time: result.execution_time,
                                    effective_price: result.effective_price,
                                    swap_data: result.swap_data,
                                    error: result.error,
                                }),
                            Err(e) => Err(e),
                        }
                    }
                };

                match fallback_result {
                    Ok(result) => {
                        log(
                            LogTag::Swap,
                            "FALLBACK_SUCCESS",
                            &format!(
                                "✅ Fallback swap succeeded via {:?}! TX: {}",
                                result.router_used.as_ref().unwrap(),
                                result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
                            )
                        );
                        return Ok(result);
                    }
                    Err(fallback_error) => {
                        log(
                            LogTag::Swap,
                            "FALLBACK_FAILED",
                            &format!("❌ Fallback swap also failed: {}", fallback_error)
                        );
                        // Return the original error, not the fallback error
                        return primary_result;
                    }
                }
            }
        }
    }

    // Return primary result (success or error if no fallback was attempted)
    primary_result
}

/// Get the best quote for opening positions with route failure tracking
/// This function tracks "no route" failures and blacklists tokens after 5 attempts
pub async fn get_best_quote_for_opening(
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    from_address: &str,
    slippage: f64,
    token_symbol: &str
) -> Result<UnifiedQuote, ScreenerBotError> {
    // Call the regular quote function
    match
        get_best_quote(
            input_mint,
            output_mint,
            input_amount,
            from_address,
            slippage,
            "ExactIn"
        ).await
    {
        Ok(quote) => Ok(quote),
        Err(e) => {
            // Check if this is a "no route" error
            let error_msg = e.to_string();
            let is_no_route_error =
                error_msg.contains("no route") ||
                error_msg.contains("No routers available for quote") ||
                error_msg.contains("jupiter has no route") ||
                error_msg.contains("Jupiter API error: 400") ||
                error_msg.contains("400 Bad Request") ||
                (error_msg.contains("Jupiter") && error_msg.contains("400"));

            if is_no_route_error {
                // Track the route failure for blacklisting (only for opening positions)
                use crate::tokens::blacklist::track_route_failure_db;
                track_route_failure_db(output_mint, token_symbol, "no_route");

                log(
                    LogTag::Swap,
                    "NO_ROUTE_TRACKED",
                    &format!(
                        "🚫 No route error tracked for {} ({}): {}",
                        token_symbol,
                        &output_mint[..8],
                        error_msg
                    )
                );
            }

            Err(e)
        }
    }
}
