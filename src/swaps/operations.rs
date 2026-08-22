//! Core Swap Operations - High-level swap functions
//! Provides get_best_quote() and execute_swap_with_fallback()

use crate::chains::solana::constants::SOL_MINT;
use crate::logger::{self, LogTag};
use crate::swaps::registry::get_registry;
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use futures::future;
use std::time::Instant;

// ============================================================================
// CONCURRENT QUOTE FETCHING
// ============================================================================

/// Get best quote from all enabled routers (concurrent)
/// Fetches quotes from all enabled routers simultaneously
/// Returns the quote with highest output amount
pub async fn get_best_quote(request: QuoteRequest) -> Result<Quote> {
    let registry = get_registry()?;
    let enabled = registry.enabled_routers_for(request.chain);

    if enabled.is_empty() {
        return Err(Error::configuration_error(
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
                        Ok(quote)
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        logger::warning(LogTag::Swap, &format!("{} quote failed: {msg}", r.name()));
                        Err((r.name().to_owned(), msg))
                    }
                }
            }
        })
        .collect();

    let results = future::join_all(futures).await;
    let elapsed = start.elapsed();

    // Partition into successful quotes and per-router failures. Keeping the
    // failures lets us report the ACTUAL reason (e.g. token not tradable) to the
    // trade dialog instead of a generic "all routers failed" that hides it.
    let mut quotes: Vec<Quote> = Vec::new();
    let mut errors: Vec<(String, String)> = Vec::new();
    for res in results {
        match res {
            Ok(q) => quotes.push(q),
            Err(e) => errors.push(e),
        }
    }

    if quotes.is_empty() {
        return Err(classify_quote_failure(&errors));
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

/// Turn the collected per-router quote failures into a single, user-meaningful
/// error message. The common case — a token with no pool/liquidity — is reported
/// by routers as "not tradable"; surface that plainly so the trade dialog can
/// explain WHY the swap can't be previewed instead of a generic failure string.
///
/// The message is kept clean and self-describing (the quote route matches on it
/// to pick a friendly title/hint for the UI), so it must not be wrapped in extra
/// prefixes here.
fn classify_quote_failure(errors: &[(String, String)]) -> Error {
    if errors.is_empty() {
        return Error::api_error("All routers failed to provide quotes");
    }

    let joined = errors
        .iter()
        .map(|(name, msg)| format!("{name}: {msg}"))
        .collect::<Vec<_>>()
        .join("; ");
    let lower = joined.to_lowercase();

    if lower.contains("not tradable") || lower.contains("token_not_tradable") {
        return Error::api_error(
            "Token not tradable: no liquidity or swap route is available for this token",
        );
    }
    if lower.contains("no route")
        || lower.contains("no routes")
        || lower.contains("could not find any route")
    {
        return Error::api_error("No swap route available for this token at the requested amount");
    }

    Error::api_error(format!("No swap route available ({joined})"))
}

// ============================================================================
// SWAP EXECUTION WITH FALLBACK
// ============================================================================

/// Execute swap with automatic fallback on failure
/// Tries primary router, falls back to others by priority on retryable errors
pub async fn execute_swap_with_fallback(token: &Token, quote: Quote) -> Result<SwapResult> {
    // Block swap execution during force stop
    if crate::global::is_force_stopped() {
        return Err(Error::internal_error(
            "Trading halted - Force stop is active",
        ));
    }

    let registry = get_registry()?;

    // Get primary router
    let primary = registry
        .get_router(&quote.router_id)
        .ok_or_else(|| Error::internal_error(format!("Router {} not found", quote.router_id)))?;

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
            // NEVER fall back on a swap that was already SUBMITTED. The confirmation poll
            // timed out, but the transaction can still land — re-sending it through another
            // router is a second, real swap.
            if let Some(signature) = unconfirmed_swap_signature(&primary_error) {
                logger::warning(
                    LogTag::Swap,
                    &format!(
                        "Swap {signature} submitted via {} but not confirmed in time - NOT retrying (it may still land); verification will reconcile it",
                        primary.name()
                    ),
                );
                return Err(primary_error);
            }

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
            let fallbacks = registry.get_fallback_chain_for(quote.chain, &quote.router_id);

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
                    chain: quote.chain,
                    input_mint: quote.input_mint.clone(),
                    output_mint: quote.output_mint.clone(),
                    input_amount: quote.input_amount,
                    wallet_address: quote.wallet_address.clone(),
                    slippage_pct: (quote.slippage_bps as f64) / 100.0,
                    swap_mode: quote.swap_mode,
                    exclude_dexes: None,
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
                        // Same rule as the primary: a submitted-but-unconfirmed swap must not
                        // be re-sent through yet another router.
                        if let Some(signature) = unconfirmed_swap_signature(&e) {
                            logger::warning(
                                LogTag::Swap,
                                &format!(
                                    "Fallback swap {signature} submitted via {} but not confirmed in time - stopping the chain (it may still land)",
                                    fallback_router.name()
                                ),
                            );
                            return Err(e);
                        }

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

/// The signature of a swap that WAS SUBMITTED but whose confirmation timed out.
///
/// `sign_send_and_confirm_transaction` sends the transaction and then polls for it; on
/// timeout it returns an error even though the transaction may still land — a Solana
/// transaction stays valid until its blockhash expires, well beyond our poll window.
///
/// Retrying such a swap is a DOUBLE SPEND: the fallback chain would submit the same sell
/// through another router, and the exit's slippage ladder would submit it again at the next
/// rung. On a full close the second sell usually just fails on an empty balance (wasted
/// fee), but on a PARTIAL exit the tokens are still there — so a "sell 25%" that timed out
/// once actually sells 25% twice, and the position records only one of them.
///
/// The signature is embedded in the error text, so a caller can stop retrying and hand it
/// to verification, which reconciles what really happened on chain.
pub fn unconfirmed_swap_signature(error: &Error) -> Option<String> {
    unconfirmed_swap_signature_from_message(&error.to_string())
}

pub fn unconfirmed_swap_signature_from_message(message: &str) -> Option<String> {
    // The marker sentence is the anchor, not the word "Transaction": callers wrap swap
    // errors in their own prose, and any earlier "Transaction " in that prose used to win
    // the match and hand back everything in between as the "signature". What came out was
    // a sentence fragment, and `close_position_direct` wrote it straight into
    // `exit_transaction_signature` — an exit that verification could never settle because
    // the chain has no such signature.
    let (before_marker, _) = message.split_once(" not confirmed within timeout")?;

    // The signature is the last whitespace-separated token before the marker.
    let candidate = before_marker.split_whitespace().next_back()?;

    is_plausible_signature(candidate).then(|| candidate.to_owned())
}

/// Base58 alphabet used by Solana signatures (no 0, O, I or l).
const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Whether a string can actually BE a Solana transaction signature.
///
/// A recovered "signature" is written to the position and handed to verification, so a
/// value that cannot exist on chain is worse than none at all: it produces an exit that
/// stays pending forever. An ed25519 signature is 64 bytes, which is 86-88 base58
/// characters; anything outside that, or carrying a character base58 cannot encode, is
/// prose that leaked out of an error message.
fn is_plausible_signature(candidate: &str) -> bool {
    (86..=88).contains(&candidate.len()) && candidate.chars().all(|c| BASE58_ALPHABET.contains(c))
}

#[cfg(test)]
mod submitted_timeout_tests {
    use super::{is_plausible_signature, unconfirmed_swap_signature_from_message};

    /// A real (well-formed) mainnet signature: 88 base58 characters.
    const SIGNATURE: &str =
        "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW";

    #[test]
    fn submitted_timeout_recovers_the_signature_for_verification() {
        assert_eq!(
            unconfirmed_swap_signature_from_message(&format!(
                "RPC error: Transaction {SIGNATURE} not confirmed within timeout"
            )),
            Some(SIGNATURE.to_owned())
        );
    }

    #[test]
    fn a_pre_submission_failure_is_not_treated_as_submitted() {
        assert_eq!(
            unconfirmed_swap_signature_from_message("quote rejected before submission"),
            None
        );
    }

    /// The regression that made a stuck sell unrecoverable: a router wrapped the
    /// send/confirm error in prose that itself contained "Transaction ", and the old
    /// first-match parser returned the prose as the signature.
    #[test]
    fn a_wrapped_error_still_yields_the_real_signature_only() {
        let recovered = unconfirmed_swap_signature_from_message(&format!(
            "Transaction send failed: RPC error: Transaction {SIGNATURE} not confirmed within timeout"
        ));
        assert_eq!(recovered, Some(SIGNATURE.to_owned()));
    }

    /// Nothing that cannot exist on chain may ever be returned: it would be written to
    /// `exit_transaction_signature` and leave the position pending verification forever.
    #[test]
    fn prose_is_never_returned_as_a_signature() {
        for message in [
            "Transaction  not confirmed within timeout",
            "Transaction send failed: not confirmed within timeout",
            "Transaction 5abcXYZ not confirmed within timeout",
            "Transaction 0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0O not confirmed within timeout",
        ] {
            assert_eq!(
                unconfirmed_swap_signature_from_message(message),
                None,
                "message must not yield a signature: {message}"
            );
        }
    }

    #[test]
    fn signature_shape_is_checked_against_base58_and_length() {
        assert!(is_plausible_signature(SIGNATURE));
        assert!(!is_plausible_signature(""));
        assert!(!is_plausible_signature("short"));
        // Right length, but '0' is not in the base58 alphabet.
        assert!(!is_plausible_signature(&SIGNATURE.replacen('5', "0", 1)));
    }
}

/// Check if error is retryable (network/transient issues)
fn is_retryable_error(error: &Error) -> bool {
    match error {
        Error::Rpc(e) => e.is_retryable(),
        Error::Network(_) | Error::RpcProvider(_) | Error::RateLimit(_) => true,
        _ => false,
    }
}

// ============================================================================
// SPECIALIZED QUOTE FUNCTIONS
// ============================================================================

/// Get best quote for opening positions with route failure tracking
/// Blacklists tokens after repeated no-route failures
pub async fn get_best_quote_for_opening(
    request: QuoteRequest,
    token_symbol: &str,
) -> Result<Quote> {
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
