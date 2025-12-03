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
use crate::errors::{BlockchainError, ScreenerBotError};
use crate::logger::{self, LogTag};
use crate::tokens::Token;
use futures::future;
use std::future::Future;
use std::pin::Pin;

// =============================================================================
// RE-EXPORTS - Clean interface for external use
// =============================================================================

// Common types and structures
pub use types::{
    ExitType, GMGNApiResponse, JupiterQuoteResponse, JupiterSwapResponse, RawTransaction,
    RouterType, SwapData, SwapQuote, SwapRequest, SwapResult,
};

// Router-specific functions
pub use gmgn::{execute_gmgn_swap, get_gmgn_quote, gmgn_sign_and_send_transaction, GMGNSwapResult};
pub use jupiter::{
    execute_jupiter_swap, get_jupiter_quote, jupiter_sign_and_send_transaction, JupiterSwapResult,
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
    swap_mode: &str,
) -> Result<UnifiedQuote, ScreenerBotError> {
    logger::info(
        LogTag::Swap,
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
        ),
    );

    let mut futures: Vec<
        Pin<Box<dyn Future<Output = Result<UnifiedQuote, ScreenerBotError>> + Send>>,
    > = Vec::new();

    let gmgn_enabled = with_config(|cfg| cfg.swaps.gmgn.enabled);
    let jupiter_enabled = with_config(|cfg| cfg.swaps.jupiter.enabled);

    // Prepare GMGN quote future with timeout
    if gmgn_enabled {
        logger::debug(LogTag::Swap, "🔵 Starting GMGN quote request...");
        let gmgn_future = async {
            // Apply 15-second timeout to GMGN quote
            let quote_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(15),
                gmgn::get_gmgn_quote(
                    input_mint,
                    output_mint,
                    input_amount,
                    from_address,
                    slippage,
                    swap_mode,
                ),
            )
            .await;

            match quote_result {
                Ok(Ok(gmgn_data)) => {
                    // Quote succeeded within timeout - validate numeric fields
                    let output_amount = gmgn_data.quote.out_amount.parse::<u64>().map_err(|e| {
                        ScreenerBotError::invalid_amount(
                            format!("GMGN.out_amount={}", gmgn_data.quote.out_amount),
                            format!("Failed to parse as u64: {}", e),
                        )
                    })?;

                    // Validate output amount is non-zero
                    if output_amount == 0 {
                        logger::warning(
                            LogTag::Swap,
                            "⚠️ GMGN quote returned zero output amount - rejecting",
                        );
                        return Err(ScreenerBotError::invalid_amount(
                            "0",
                            "GMGN quote returned zero output - invalid quote",
                        ));
                    }

                    let price_impact_pct = gmgn_data
                        .quote
                        .price_impact_pct
                        .parse::<f64>()
                        .unwrap_or(0.0); // Non-critical, default to 0
                    let slippage_bps = gmgn_data.quote.slippage_bps.parse::<u16>().unwrap_or(0); // Non-critical, default to 0

                    let unified_quote = UnifiedQuote {
                        router: RouterType::GMGN,
                        input_mint: input_mint.to_string(),
                        output_mint: output_mint.to_string(),
                        input_amount,
                        output_amount,
                        price_impact_pct,
                        fee_lamports: gmgn_data.raw_tx.prioritization_fee_lamports,
                        slippage_bps,
                        route_plan: format!(
                            "GMGN Route: {}",
                            serde_json::to_string(&gmgn_data.quote.route_plan).unwrap_or_default()
                        ),
                        execution_data: QuoteExecutionData::GMGN(gmgn_data),
                        swap_mode: swap_mode.to_string(),
                    };

                    logger::info(
                        LogTag::Swap,
                        &format!(
                            "✅ GMGN quote: {} tokens, impact: {:.2}%, fee: {} lamports",
                            unified_quote.output_amount,
                            unified_quote.price_impact_pct,
                            unified_quote.fee_lamports
                        ),
                    );

                    Ok(unified_quote)
                }
                Ok(Err(e)) => {
                    // Quote failed (API error, not timeout)
                    logger::error(LogTag::Swap, &format!("❌ GMGN quote failed: {}", e));
                    Err(e)
                }
                Err(_timeout) => {
                    // Quote timed out - use network error
                    let error = ScreenerBotError::network_error(
                        "GMGN quote request exceeded 15 second timeout",
                    );
                    logger::warning(LogTag::Swap, "⏰ GMGN quote timed out after 15s");
                    Err(error)
                }
            }
        };
        futures.push(Box::pin(gmgn_future));
    } else {
        logger::debug(LogTag::Swap, "⏸️ GMGN router disabled in config");
    }

    // Prepare Jupiter quote future with timeout
    if jupiter_enabled {
        logger::debug(LogTag::Swap, "🟡 Starting Jupiter quote request...");
        let jupiter_future = async {
            // Apply 15-second timeout to Jupiter quote
            let quote_result = tokio::time::timeout(
                tokio::time::Duration::from_secs(15),
                jupiter::get_jupiter_quote(
                    input_mint,
                    output_mint,
                    input_amount,
                    slippage,
                    swap_mode,
                ),
            )
            .await;

            match quote_result {
                Ok(Ok(jupiter_data)) => {
                    // Quote succeeded within timeout - validate numeric fields
                    let output_amount =
                        jupiter_data.quote.out_amount.parse::<u64>().map_err(|e| {
                            ScreenerBotError::invalid_amount(
                                format!("Jupiter.out_amount={}", jupiter_data.quote.out_amount),
                                format!("Failed to parse as u64: {}", e),
                            )
                        })?;

                    // Validate output amount is non-zero
                    if output_amount == 0 {
                        logger::warning(
                            LogTag::Swap,
                            "⚠️ Jupiter quote returned zero output amount - rejecting",
                        );
                        return Err(ScreenerBotError::invalid_amount(
                            "0",
                            "Jupiter quote returned zero output - invalid quote",
                        ));
                    }

                    let price_impact_pct = jupiter_data
                        .quote
                        .price_impact_pct
                        .parse::<f64>()
                        .unwrap_or(0.0); // Non-critical, default to 0
                    let slippage_bps = jupiter_data.quote.slippage_bps.parse::<u16>().unwrap_or(0); // Non-critical, default to 0

                    let unified_quote = UnifiedQuote {
                        router: RouterType::Jupiter,
                        input_mint: input_mint.to_string(),
                        output_mint: output_mint.to_string(),
                        input_amount,
                        output_amount,
                        price_impact_pct,
                        fee_lamports: jupiter_data.raw_tx.prioritization_fee_lamports,
                        slippage_bps,
                        route_plan: format!(
                            "Jupiter Route: {}",
                            serde_json::to_string(&jupiter_data.quote.route_plan)
                                .unwrap_or_default()
                        ),
                        execution_data: QuoteExecutionData::Jupiter(jupiter_data),
                        swap_mode: swap_mode.to_string(),
                    };

                    logger::info(
                        LogTag::Swap,
                        &format!(
                            "✅ Jupiter quote: {} tokens, impact: {:.2}%, fee: {} lamports",
                            unified_quote.output_amount,
                            unified_quote.price_impact_pct,
                            unified_quote.fee_lamports
                        ),
                    );

                    Ok(unified_quote)
                }
                Ok(Err(e)) => {
                    // Quote failed (API error, not timeout)
                    logger::info(LogTag::Swap, &format!("❌ Jupiter quote failed: {}", e));
                    Err(e)
                }
                Err(_timeout) => {
                    // Quote timed out - use network error
                    let error = ScreenerBotError::network_error(
                        "Jupiter quote request exceeded 15 second timeout",
                    );
                    logger::info(LogTag::Swap, "⏰ Jupiter quote timed out after 15s");
                    Err(error)
                }
            }
        };
        futures.push(Box::pin(jupiter_future));
    } else {
        logger::info(LogTag::Swap, "⏸️ Jupiter router disabled in config");
    }

    // Execute all quote requests concurrently
    logger::info(
        LogTag::Swap,
        &format!(
            "⚡ Executing {} quote requests concurrently...",
            futures.len()
        ),
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
        let error_msg = if gmgn_enabled && jupiter_enabled {
            "No swap routes available - all routers failed to find a route"
        } else if jupiter_enabled {
            "No swap routes available - Jupiter router failed to find a route"
        } else if gmgn_enabled {
            "No swap routes available - GMGN router failed to find a route"
        } else {
            "No swap routers enabled"
        };
        logger::error(LogTag::Swap, &format!("❌ {}", error_msg));

        // Log detailed failure summary for debugging
        logger::debug(LogTag::Swap, &format!("🔍 Quote failure summary - GMGN: {}, Jupiter: {} (check token liquidity and API status)",
                if gmgn_enabled {
                    "enabled but failed"
                } else {
                    "disabled"
                },
                if jupiter_enabled {
                    "enabled but failed"
                } else {
                    "disabled"
                }));

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
        logger::info(
            LogTag::Swap,
            &format!(
                "⚖️ Quote comparison: GMGN vs Jupiter - Winner: {:?}",
                best_quote.router
            ),
        );

        // Show detailed comparison
        for quote in &quotes {
            logger::info(
                LogTag::Swap,
                &format!(
                    "  • {:?}: {} tokens (impact: {:.2}%, fee: {} lamports)",
                    quote.router, quote.output_amount, quote.price_impact_pct, quote.fee_lamports
                ),
            );
        }
    }

    logger::info(
        LogTag::Swap,
        &format!(
            "🏆 Best route selected: {:?} with {} tokens (impact: {:.2}%, fee: {} lamports)",
            best_quote.router,
            best_quote.output_amount,
            best_quote.price_impact_pct,
            best_quote.fee_lamports
        ),
    );

    Ok(best_quote)
}

/// Execute swap with unified quote (with fallback retry mechanism)
pub async fn execute_best_swap(
    token: &Token,
    input_mint: &str,
    output_mint: &str,
    input_amount: u64,
    quote: UnifiedQuote,
) -> Result<SwapResult, ScreenerBotError> {
    // Create action for progress tracking
    let action_id = uuid::Uuid::new_v4().to_string();
    let action_type = if input_mint == SOL_MINT {
        crate::actions::ActionType::SwapBuy
    } else {
        crate::actions::ActionType::SwapSell
    };

    let mint = if input_mint == SOL_MINT {
        output_mint
    } else {
        input_mint
    };

    let action = crate::actions::Action::new(
        action_id.clone(),
        action_type,
        mint.to_string(),
        vec![
            "Validating quote".to_string(),
            "Building transaction".to_string(),
            "Signing transaction".to_string(),
            "Submitting to blockchain".to_string(),
            "Confirming transaction".to_string(),
        ],
        serde_json::json!({
            "symbol": token.symbol,
            "router": format!("{:?}", quote.router),
            "input_amount": input_amount,
            "expected_output": quote.output_amount,
        }),
    );

    if let Err(e) = crate::actions::register_action(action).await {
        logger::error(
            LogTag::Swap,
            &format!("Failed to register swap action: {} - continuing anyway", e),
        );
    }

    logger::info(
        LogTag::Swap,
        &format!(
            "🚀 Executing swap via {:?}: {} -> {} (amount: {}) | Action: {}",
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
            input_amount,
            &action_id[..8]
        ),
    );

    // Step 1: Validating quote
    crate::actions::update_step(
        &action_id,
        0,
        crate::actions::StepStatus::InProgress,
        None,
        None,
    )
    .await;
    crate::actions::update_step(
        &action_id,
        0,
        crate::actions::StepStatus::Completed,
        None,
        None,
    )
    .await;

    // Step 1: Validating quote
    crate::actions::update_step(
        &action_id,
        0,
        crate::actions::StepStatus::InProgress,
        None,
        None,
    )
    .await;
    crate::actions::update_step(
        &action_id,
        0,
        crate::actions::StepStatus::Completed,
        None,
        None,
    )
    .await;

    // Step 2: Building transaction (starts in router execution)
    crate::actions::update_step(
        &action_id,
        1,
        crate::actions::StepStatus::InProgress,
        None,
        None,
    )
    .await;

    // Try primary router first
    let primary_result = match quote.execution_data {
        QuoteExecutionData::GMGN(ref gmgn_data) => {
            match gmgn::execute_gmgn_swap(
                token,
                input_mint,
                output_mint,
                input_amount,
                gmgn_data.clone(),
            )
            .await
            {
                Ok(result) => {
                    crate::actions::update_step(
                        &action_id,
                        1,
                        crate::actions::StepStatus::Completed,
                        None,
                        None,
                    )
                    .await;
                    crate::actions::update_step(
                        &action_id,
                        2,
                        crate::actions::StepStatus::Completed,
                        None,
                        None,
                    )
                    .await;
                    crate::actions::update_step(
                        &action_id,
                        3,
                        crate::actions::StepStatus::Completed,
                        None,
                        None,
                    )
                    .await;
                    crate::actions::update_step(
                        &action_id,
                        4,
                        crate::actions::StepStatus::InProgress,
                        None,
                        None,
                    )
                    .await;

                    let swap_result = SwapResult {
                        success: result.success,
                        router_used: Some(RouterType::GMGN),
                        transaction_signature: result.transaction_signature.clone(),
                        input_amount: result.input_amount,
                        output_amount: result.output_amount,
                        price_impact: result.price_impact,
                        fee_lamports: result.fee_lamports,
                        execution_time: result.execution_time,
                        effective_price: result.effective_price,
                        swap_data: result.swap_data,
                        error: result.error,
                    };

                    if result.success {
                        crate::actions::update_step(
                            &action_id,
                            4,
                            crate::actions::StepStatus::Completed,
                            None,
                            Some(serde_json::json!({"tx": result.transaction_signature})),
                        )
                        .await;
                        crate::actions::complete_action_success(&action_id).await;
                    } else {
                        crate::actions::update_step(
                            &action_id,
                            4,
                            crate::actions::StepStatus::Failed,
                            result
                                .transaction_signature
                                .clone()
                                .map(|s| format!("Confirmation failed: TX {}", s)),
                            None,
                        )
                        .await;
                        crate::actions::complete_action_failed(
                            &action_id,
                            swap_result
                                .error
                                .clone()
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        )
                        .await;
                    }

                    Ok(swap_result)
                }
                Err(e) => {
                    crate::actions::update_step(
                        &action_id,
                        1,
                        crate::actions::StepStatus::Failed,
                        Some(e.to_string()),
                        None,
                    )
                    .await;
                    Err(e)
                }
            }
        }
        QuoteExecutionData::Jupiter(ref jupiter_data) => {
            match jupiter::execute_jupiter_swap(
                token,
                input_mint,
                output_mint,
                jupiter_data.clone(),
            )
            .await
            {
                Ok(result) => {
                    crate::actions::update_step(
                        &action_id,
                        1,
                        crate::actions::StepStatus::Completed,
                        None,
                        None,
                    )
                    .await;
                    crate::actions::update_step(
                        &action_id,
                        2,
                        crate::actions::StepStatus::Completed,
                        None,
                        None,
                    )
                    .await;
                    crate::actions::update_step(
                        &action_id,
                        3,
                        crate::actions::StepStatus::Completed,
                        None,
                        None,
                    )
                    .await;
                    crate::actions::update_step(
                        &action_id,
                        4,
                        crate::actions::StepStatus::InProgress,
                        None,
                        None,
                    )
                    .await;

                    let swap_result = SwapResult {
                        success: result.success,
                        router_used: Some(RouterType::Jupiter),
                        transaction_signature: result.transaction_signature.clone(),
                        input_amount: result.input_amount,
                        output_amount: result.output_amount,
                        price_impact: result.price_impact,
                        fee_lamports: result.fee_lamports,
                        execution_time: result.execution_time,
                        effective_price: result.effective_price,
                        swap_data: result.swap_data,
                        error: result.error,
                    };

                    if result.success {
                        crate::actions::update_step(
                            &action_id,
                            4,
                            crate::actions::StepStatus::Completed,
                            None,
                            Some(serde_json::json!({"tx": result.transaction_signature})),
                        )
                        .await;
                        crate::actions::complete_action_success(&action_id).await;
                    } else {
                        crate::actions::update_step(
                            &action_id,
                            4,
                            crate::actions::StepStatus::Failed,
                            result
                                .transaction_signature
                                .clone()
                                .map(|s| format!("Confirmation failed: TX {}", s)),
                            None,
                        )
                        .await;
                        crate::actions::complete_action_failed(
                            &action_id,
                            swap_result
                                .error
                                .clone()
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        )
                        .await;
                    }

                    Ok(swap_result)
                }
                Err(e) => {
                    crate::actions::update_step(
                        &action_id,
                        1,
                        crate::actions::StepStatus::Failed,
                        Some(e.to_string()),
                        None,
                    )
                    .await;
                    Err(e)
                }
            }
        }
    };

    // Check if primary router failed and fallback is available
    if let Err(ref primary_error) = primary_result {
        logger::info(
            LogTag::Swap,
            &format!(
                "⚠️ Primary router {:?} failed: {}",
                quote.router, primary_error
            ),
        );

        // Mark action as failed for now (will update if fallback succeeds)
        crate::actions::complete_action_failed(&action_id, primary_error.to_string()).await;

        // Only try fallback for certain error types (propagation failures, transaction errors)
        let should_fallback = match primary_error {
            ScreenerBotError::Blockchain(BlockchainError::TransactionDropped {
                reason, ..
            }) if reason.contains("not propagated") => true,
            ScreenerBotError::Blockchain(BlockchainError::TransactionDropped {
                reason, ..
            }) if reason.contains("dropped") => true,
            ScreenerBotError::Network(_) => true,
            _ => false,
        };

        if should_fallback {
            logger::info(
                LogTag::Swap,
                "🔄 Attempting fallback to alternative router...",
            );

            // Get fallback quote from the other router
            let wallet_address = match crate::config::get_wallet_pubkey_string() {
                Ok(addr) => addr,
                Err(_) => {
                    return primary_result;
                } // If can't get wallet, return original error
            };

            let gmgn_enabled_fallback = with_config(|cfg| cfg.swaps.gmgn.enabled);
            let jupiter_enabled_fallback = with_config(|cfg| cfg.swaps.jupiter.enabled);

            let fallback_quote = match quote.router {
                RouterType::Jupiter => {
                    // Jupiter failed, try GMGN
                    if gmgn_enabled_fallback {
                        logger::info(LogTag::Swap, "🔵 Falling back to GMGN router...");

                        match gmgn::get_gmgn_quote(
                            input_mint,
                            output_mint,
                            input_amount,
                            &wallet_address,
                            (quote.slippage_bps as f64) / 100.0, // Convert bps to percentage
                            &quote.swap_mode,
                        )
                        .await
                        {
                            Ok(gmgn_data) => {
                                logger::info(
                                    LogTag::Swap,
                                    &format!(
                                        "✅ GMGN fallback quote: {} tokens, impact: {:.2}%",
                                        gmgn_data.quote.out_amount,
                                        gmgn_data
                                            .quote
                                            .price_impact_pct
                                            .parse::<f64>()
                                            .unwrap_or(0.0)
                                    ),
                                );
                                Some(gmgn_data)
                            }
                            Err(e) => {
                                logger::info(
                                    LogTag::Swap,
                                    &format!("❌ GMGN fallback quote failed: {}", e),
                                );
                                None
                            }
                        }
                    } else {
                        logger::info(LogTag::Swap, "❌ GMGN fallback not available (disabled)");
                        None
                    }
                }
                RouterType::GMGN => {
                    // GMGN failed, try Jupiter
                    if jupiter_enabled_fallback {
                        logger::info(LogTag::Swap, "🟡 Falling back to Jupiter router...");

                        match jupiter::get_jupiter_quote(
                            input_mint,
                            output_mint,
                            input_amount,
                            (quote.slippage_bps as f64) / 100.0, // Convert bps to percentage
                            &quote.swap_mode,
                        )
                        .await
                        {
                            Ok(jupiter_data) => {
                                logger::info(
                                    LogTag::Swap,
                                    &format!(
                                        "✅ Jupiter fallback quote: {} tokens, impact: {:.2}%",
                                        jupiter_data.quote.out_amount,
                                        jupiter_data
                                            .quote
                                            .price_impact_pct
                                            .parse::<f64>()
                                            .unwrap_or(0.0)
                                    ),
                                );
                                Some(jupiter_data)
                            }
                            Err(e) => {
                                logger::info(
                                    LogTag::Swap,
                                    &format!("❌ Jupiter fallback quote failed: {}", e),
                                );
                                None
                            }
                        }
                    } else {
                        logger::info(LogTag::Swap, "❌ Jupiter fallback not available (disabled)");
                        None
                    }
                }
            };

            // Execute fallback if we got a quote
            if let Some(fallback_data) = fallback_quote {
                let fallback_result = match quote.router {
                    RouterType::Jupiter => {
                        // Fallback to GMGN
                        logger::info(LogTag::Swap, "🔵 Executing GMGN fallback swap...");
                        match gmgn::execute_gmgn_swap(
                            token,
                            input_mint,
                            output_mint,
                            input_amount,
                            fallback_data,
                        )
                        .await
                        {
                            Ok(result) => Ok(SwapResult {
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
                        logger::info(LogTag::Swap, "🟡 Executing Jupiter fallback swap...");
                        match jupiter::execute_jupiter_swap(
                            token,
                            input_mint,
                            output_mint,
                            fallback_data,
                        )
                        .await
                        {
                            Ok(result) => Ok(SwapResult {
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
                        logger::info(
                            LogTag::Swap,
                            &format!(
                                "✅ Fallback swap succeeded via {:?}! TX: {}",
                                result.router_used.as_ref().unwrap(),
                                result
                                    .transaction_signature
                                    .as_ref()
                                    .unwrap_or(&"None".to_string())
                            ),
                        );
                        return Ok(result);
                    }
                    Err(fallback_error) => {
                        logger::info(
                            LogTag::Swap,
                            &format!("❌ Fallback swap also failed: {}", fallback_error),
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
    token_symbol: &str,
) -> Result<UnifiedQuote, ScreenerBotError> {
    // Call the regular quote function
    match get_best_quote(
        input_mint,
        output_mint,
        input_amount,
        from_address,
        slippage,
        "ExactIn",
    )
    .await
    {
        Ok(quote) => Ok(quote),
        Err(e) => {
            // Check if this is a "no route" error
            let error_msg = e.to_string();
            let is_no_route_error = error_msg.contains("no route")
                || error_msg.contains("No routers available for quote")
                || error_msg.contains("jupiter has no route")
                || error_msg.contains("Jupiter API error: 400")
                || error_msg.contains("400 Bad Request")
                || (error_msg.contains("Jupiter") && error_msg.contains("400"));

            if is_no_route_error {
                // Track the route failure for blacklisting (only for opening positions)
                if let Some(db) = crate::tokens::database::get_global_database() {
                    let _ = crate::tokens::cleanup::blacklist_token(output_mint, "NoRoute", &db);
                }

                logger::info(
                    LogTag::Swap,
                    &format!(
                        "🚫 No route error tracked for {} ({}): {}",
                        token_symbol,
                        &output_mint[..8],
                        error_msg
                    ),
                );
            }

            Err(e)
        }
    }
}

// =============================================================================
// HELPER FUNCTIONS FOR PARTIAL EXITS
// =============================================================================

/// Calculate the token amount for a partial exit
pub fn calculate_partial_amount(total_amount: u64, percentage: f64) -> u64 {
    if percentage <= 0.0 {
        return 0;
    }
    if percentage >= 100.0 {
        return total_amount;
    }

    let partial = (total_amount as f64 * percentage / 100.0) as u64;

    // Ensure we don't exceed total amount due to rounding
    std::cmp::min(partial, total_amount)
}
