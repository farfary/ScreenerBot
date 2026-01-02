/// Core Swap Operations - High-level swap functions
/// Provides get_best_quote() and execute_swap_with_fallback()
use crate::constants::SOL_MINT;
use crate::errors::ScreenerBotError;
use crate::logger::{self, LogTag};
use crate::swaps::registry::get_registry;
use crate::swaps::router::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use futures::future;
use std::time::Instant;

// ============================================================================
// CONCURRENT QUOTE FETCHING
// ============================================================================

/// Get best quote from all enabled routers (concurrent)
/// Fetches quotes from all enabled routers simultaneously
/// Returns the quote with highest output amount
pub async fn get_best_quote(request: QuoteRequest) -> Result<Quote, ScreenerBotError> {
    let registry = get_registry();
    let enabled = registry.enabled_routers();

    if enabled.is_empty() {
        return Err(ScreenerBotError::configuration_error(
            "No swap routers enabled in config",
        ));
    }

    logger::info(
        LogTag::Swap,
        &format!(
            "Fetching quotes from {} routers concurrently: {}",
            enabled.len(),
            enabled
                .iter()
                .map(|r| r.name())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );

    // Fetch all quotes concurrently
    let start = Instant::now();
    let futures: Vec<_> = enabled
        .iter()
        .map(|router| {
            let req = request.clone();
            let r = router.clone();
            async move {
                match r.get_quote(&req).await {
                    Ok(quote) => {
                        logger::info(
                            LogTag::Swap,
                            &format!(
                                "{}: {} output, {:.2}% impact",
                                r.name(),
                                quote.output_amount,
                                quote.price_impact_pct
                            ),
                        );
                        Some(quote)
                    }
                    Err(e) => {
                        logger::warning(LogTag::Swap, &format!("{} quote failed: {}", r.name(), e));
                        None
                    }
                }
            }
        })
        .collect();

    let results = future::join_all(futures).await;
    let quotes: Vec<Quote> = results.into_iter().flatten().collect();

    let elapsed = start.elapsed();

    if quotes.is_empty() {
        return Err(ScreenerBotError::api_error(
            "All routers failed to provide quotes",
        ));
    }

    // Select best quote (highest output)
    let best = quotes
        .into_iter()
        .max_by_key(|q| q.output_amount)
        .expect("quotes is non-empty, guaranteed by check above");

    logger::info(
        LogTag::Swap,
        &format!(
            "Best quote: {} with {} output ({:.2}% impact) - fetched in {:.2}s",
            best.router_name,
            best.output_amount,
            best.price_impact_pct,
            elapsed.as_secs_f64()
        ),
    );

    Ok(best)
}

// ============================================================================
// SWAP EXECUTION WITH FALLBACK
// ============================================================================

/// Execute swap with automatic fallback on failure
/// Tries primary router, falls back to others by priority on retryable errors
pub async fn execute_swap_with_fallback(
    token: &Token,
    quote: Quote,
) -> Result<SwapResult, ScreenerBotError> {
    // Block swap execution during force stop
    if crate::global::is_force_stopped() {
        return Err(ScreenerBotError::internal_error(
            "Trading halted - Force stop is active",
        ));
    }

    let registry = get_registry();

    // Get primary router
    let primary = registry.get_router(&quote.router_id).ok_or_else(|| {
        ScreenerBotError::internal_error(format!("Router {} not found", quote.router_id))
    })?;

    logger::info(
        LogTag::Swap,
        &format!(
            "Executing swap via {} (quote: {} → {})",
            primary.name(),
            quote.input_amount,
            quote.output_amount
        ),
    );

    let start = Instant::now();

    // Try primary router
    match primary.execute_swap(token, &quote).await {
        Ok(mut result) => {
            result.execution_time_ms = start.elapsed().as_millis() as u64;
            logger::info(
                LogTag::Swap,
                &format!(
                    "Swap succeeded via {} in {:.2}s - sig: {}",
                    result.router_name,
                    result.execution_time_ms as f64 / 1000.0,
                    result.transaction_signature
                ),
            );
            return Ok(result);
        }
        Err(primary_error) => {
            // Check if error is retryable
            if !is_retryable_error(&primary_error) {
                logger::error(
                    LogTag::Swap,
                    &format!(
                        "{} swap failed (non-retryable): {}",
                        primary.name(),
                        primary_error
                    ),
                );
                return Err(primary_error);
            }

            logger::warning(
                LogTag::Swap,
                &format!(
                    "{} swap failed (retryable): {} - trying fallback...",
                    primary.name(),
                    primary_error
                ),
            );

            // Try fallback chain
            let fallbacks = registry.get_fallback_chain(&quote.router_id);

            if fallbacks.is_empty() {
                logger::error(
                    LogTag::Swap,
                    &format!(
                        "No fallback routers available (only {} was enabled)",
                        primary.name()
                    ),
                );
                return Err(primary_error);
            }

            logger::info(
                LogTag::Swap,
                &format!(
                    "Attempting {} fallback routers: {}",
                    fallbacks.len(),
                    fallbacks
                        .iter()
                        .map(|r| r.name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );

            for fallback_router in fallbacks {
                logger::info(
                    LogTag::Swap,
                    &format!("Attempting fallback to {}", fallback_router.name()),
                );

                // Get fresh quote from fallback router
                let fallback_request = QuoteRequest {
                    input_mint: quote.input_mint.clone(),
                    output_mint: quote.output_mint.clone(),
                    input_amount: quote.input_amount,
                    wallet_address: quote.wallet_address.clone(),
                    slippage_pct: (quote.slippage_bps as f64) / 100.0,
                    swap_mode: quote.swap_mode,
                };

                let fallback_quote = match fallback_router.get_quote(&fallback_request).await {
                    Ok(q) => q,
                    Err(e) => {
                        logger::warning(
                            LogTag::Swap,
                            &format!("{} quote failed: {}", fallback_router.name(), e),
                        );
                        continue;
                    }
                };

                // Execute fallback swap
                match fallback_router.execute_swap(token, &fallback_quote).await {
                    Ok(mut result) => {
                        result.execution_time_ms = start.elapsed().as_millis() as u64;
                        logger::info(
                            LogTag::Swap,
                            &format!(
                                "Fallback succeeded via {} in {:.2}s - sig: {}",
                                result.router_name,
                                result.execution_time_ms as f64 / 1000.0,
                                result.transaction_signature
                            ),
                        );
                        return Ok(result);
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Swap,
                            &format!("{} execution failed: {}", fallback_router.name(), e),
                        );
                        continue;
                    }
                }
            }

            // All fallbacks failed - return original error
            logger::error(LogTag::Swap, "All routers failed (primary + all fallbacks)");
            Err(primary_error)
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Check if error is retryable (network/transient issues)
fn is_retryable_error(error: &ScreenerBotError) -> bool {
    matches!(
        error,
        ScreenerBotError::Network(_)
            | ScreenerBotError::RpcProvider(_)
            | ScreenerBotError::RateLimit(_)
    )
}

// ============================================================================
// SPECIALIZED QUOTE FUNCTIONS
// ============================================================================

/// Get best quote for opening positions with route failure tracking
/// Blacklists tokens after repeated no-route failures
pub async fn get_best_quote_for_opening(
    request: QuoteRequest,
    token_symbol: &str,
) -> Result<Quote, ScreenerBotError> {
    match get_best_quote(request.clone()).await {
        Ok(quote) => Ok(quote),
        Err(e) => {
            let error_msg = e.to_string();
            let is_no_route_error = error_msg.contains("no route")
                || error_msg.contains("No routers available for quote")
                || error_msg.contains("jupiter has no route")
                || error_msg.contains("Jupiter API error: 400")
                || error_msg.contains("400 Bad Request")
                || (error_msg.contains("Jupiter") && error_msg.contains("400"));

            if is_no_route_error {
                let output_mint = if request.input_mint == SOL_MINT {
                    &request.output_mint
                } else {
                    &request.input_mint
                };

                if let Some(db) = crate::tokens::database::get_global_database() {
                    let _ = crate::tokens::cleanup::blacklist_token(output_mint, "NoRoute", &db);
                }

                logger::info(
                    LogTag::Swap,
                    &format!(
                        "No route error tracked for {} ({}): {}",
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
