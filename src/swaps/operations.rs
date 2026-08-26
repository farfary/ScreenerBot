//! Core Swap Operations - High-level swap functions
//! Provides get_best_quote() and execute_swap_with_fallback()

use crate::logger::{self, LogTag};
use crate::swaps::error::{QuoteError, QuoteResult};
use crate::swaps::registry::{get_registry, try_get_registry};
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use futures::future;
use std::time::Instant;

// ============================================================================
// CONCURRENT QUOTE FETCHING
// ============================================================================

/// Get best quote from all enabled routers (concurrent), folded into the crate
/// error channel. Callers that must react to WHY quoting failed — the opening
/// path deciding whether to blacklist, the trade dialog choosing a message —
/// take [`try_get_best_quote`] instead and match the [`QuoteError`] variant.
pub async fn get_best_quote(request: QuoteRequest) -> Result<Quote> {
    try_get_best_quote(request).await.map_err(Error::from)
}

/// Get best quote from all enabled routers (concurrent)
/// Fetches quotes from all enabled routers simultaneously
/// Returns the quote with highest output amount
pub async fn try_get_best_quote(request: QuoteRequest) -> QuoteResult<Quote> {
    // The registry failure stays a ServiceError all the way through: a swap
    // service that never started is not a fact about the token, and callers on
    // the crate channel must still see WHICH service failed.
    let registry = try_get_registry().ok_or_else(|| {
        QuoteError::RegistryUnavailable(crate::errors::ServiceError::Initialize {
            service: "swaps.registry".to_owned(),
            message: "router factory has not been registered".to_owned(),
        })
    })?;
    let enabled = registry.enabled_routers_for(request.chain);

    if enabled.is_empty() {
        return Err(QuoteError::NoRoutersEnabled {
            chain: request.chain,
        });
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
                        logger::warning(LogTag::Swap, &format!("{} quote failed: {e}", r.name()));
                        Err(e)
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
    let mut errors: Vec<QuoteError> = Vec::new();
    for res in results {
        match res {
            Ok(q) => quotes.push(q),
            Err(e) => errors.push(e),
        }
    }

    if quotes.is_empty() {
        return Err(select_quote_failure(errors));
    }

    // Select best quote (highest output)
    let best = quotes
        .into_iter()
        .max_by_key(|q| q.output_amount)
        .expect("quotes is non-empty, guaranteed by check above");

    // A router's response is untrusted input on a money path. A quote that
    // would spend the input for nothing, or that prices a pair we never asked
    // for, must never reach the builder — selection is the last point where
    // either is still cheap to refuse.
    if best.output_amount == 0 {
        return Err(QuoteError::RouterRejected {
            router: best.router_name.clone(),
            detail: format!(
                "zero-output quote for {} -> {}",
                best.input_mint, best.output_mint
            ),
        });
    }
    if best.input_mint != request.input_mint || best.output_mint != request.output_mint {
        return Err(QuoteError::RouterRejected {
            router: best.router_name.clone(),
            detail: format!(
                "quoted {} -> {} but {} -> {} was requested",
                best.input_mint, best.output_mint, request.input_mint, request.output_mint
            ),
        });
    }

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

/// Reduce the per-router failures to the one verdict that best describes the
/// attempt as a whole.
///
/// Routers disagree: Jupiter may say the token is not tradable while GMGN
/// times out. The most specific answer wins, because that is the one a caller
/// can act on — a token no provider can trade is a durable fact, whereas one
/// router's timeout says nothing about the token. Ordering is therefore by how
/// much the variant licenses the caller to conclude, most-concluding first.
///
/// This replaced a function that re-read its own output: it rendered a friendly
/// message for the "no route" case, and the opening path then searched that
/// message for the word "no route" — which the friendly wording no longer
/// contained, so no token was ever blacklisted for having no market.
fn select_quote_failure(errors: Vec<QuoteError>) -> QuoteError {
    fn specificity(err: &QuoteError) -> u8 {
        match err {
            QuoteError::NotTradable { .. } => 0,
            QuoteError::NoRoute { .. } => 1,
            QuoteError::RouterRejected { .. } => 2,
            QuoteError::RateLimited { .. } => 3,
            QuoteError::Timeout { .. } => 4,
            QuoteError::Unavailable { .. } => 5,
            QuoteError::NoRoutersEnabled { .. } => 6,
            // Unreachable from the per-router loop (the registry is resolved
            // before any router is asked), and least specific if it ever is.
            QuoteError::RegistryUnavailable(_) => 7,
        }
    }

    errors
        .into_iter()
        .min_by_key(specificity)
        .unwrap_or_else(|| QuoteError::Unavailable {
            router: "all".to_owned(),
            detail: "no router returned a quote or an error".to_owned(),
        })
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

    // A recovered "signature" is written to the position and handed to verification, so a
    // value that cannot exist on chain is worse than none at all: it produces an exit that
    // stays pending forever. Reject anything that is not shaped like a real hash.
    crate::chains::adapter()
        .looks_like_transaction_hash(candidate)
        .then(|| candidate.to_owned())
}

#[cfg(test)]
mod submitted_timeout_tests {
    use super::unconfirmed_swap_signature_from_message;

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
        let adapter = crate::chains::adapter();
        assert!(adapter.looks_like_transaction_hash(SIGNATURE));
        assert!(!adapter.looks_like_transaction_hash(""));
        assert!(!adapter.looks_like_transaction_hash("short"));
        // Right length, but '0' is not in the base58 alphabet.
        assert!(!adapter.looks_like_transaction_hash(&SIGNATURE.replacen('5', "0", 1)));
    }
}

/// Check if error is retryable (network/transient issues)
fn is_retryable_error(error: &Error) -> bool {
    match error {
        Error::Rpc(e) => e.is_retryable(),
        Error::Network(_) | Error::RpcProvider(_) => true,
        _ => false,
    }
}

// ============================================================================
// SPECIALIZED QUOTE FUNCTIONS
// ============================================================================

/// How many separate no-route failures a token may collect before the opening
/// path stops offering it. One is not enough: a route can be missing for the
/// requested size alone and reappear minutes later, while the blacklist is
/// permanent and can only be lifted by hand.
const NO_ROUTE_STRIKES_BEFORE_BLACKLIST: u32 = 3;

/// How long a strike stays on a token's record. A token that fails once a day
/// is not a token without a market, so strikes must decay rather than
/// accumulate for the life of the process.
const NO_ROUTE_STRIKE_TTL: std::time::Duration = std::time::Duration::from_secs(30 * 60);

/// Consecutive no-route strikes per mint. Bounded and self-expiring, so a long
/// discovery session cannot grow it without limit.
static NO_ROUTE_STRIKES: std::sync::LazyLock<moka::sync::Cache<String, u32>> =
    std::sync::LazyLock::new(|| {
        moka::sync::Cache::builder()
            .max_capacity(10_000)
            .time_to_live(NO_ROUTE_STRIKE_TTL)
            .build()
    });

/// Get best quote for opening a position, recording route failures against the
/// token.
///
/// A provider saying the token is not tradable at all is a durable verdict and
/// retires the token immediately. A provider merely failing to route the
/// requested size is not: it has to repeat
/// [`NO_ROUTE_STRIKES_BEFORE_BLACKLIST`] times inside
/// [`NO_ROUTE_STRIKE_TTL`] before the token is retired, and any successful
/// quote clears the record.
pub async fn get_best_quote_for_opening(
    request: QuoteRequest,
    token_symbol: &str,
) -> Result<Quote> {
    // The token being assessed is whichever side of the pair is not the chain's
    // native asset — a buy spends SOL for it, a sell spends it for SOL.
    let subject_mint = if crate::chains::adapter().is_native_asset(&request.input_mint) {
        request.output_mint.clone()
    } else {
        request.input_mint.clone()
    };

    match try_get_best_quote(request).await {
        Ok(quote) => {
            NO_ROUTE_STRIKES.invalidate(&subject_mint);
            Ok(quote)
        }
        Err(e) => {
            if let Some(reason) = e.permanent_token_verdict() {
                retire_token(&subject_mint, token_symbol, reason, &e);
            } else if e.is_route_failure() {
                let strikes = NO_ROUTE_STRIKES.get(&subject_mint).unwrap_or(0) + 1;
                NO_ROUTE_STRIKES.insert(subject_mint.clone(), strikes);

                if strikes >= NO_ROUTE_STRIKES_BEFORE_BLACKLIST {
                    NO_ROUTE_STRIKES.invalidate(&subject_mint);
                    retire_token(&subject_mint, token_symbol, "NoRoute", &e);
                } else {
                    logger::info(
                        LogTag::Swap,
                        &format!(
                            "No route for {token_symbol} ({}): strike {strikes}/{} - {e}",
                            short_mint(&subject_mint),
                            NO_ROUTE_STRIKES_BEFORE_BLACKLIST
                        ),
                    );
                }
            }

            Err(Error::from(e))
        }
    }
}

/// Blacklist a token the routers cannot trade, recording it as an automatic
/// decision. The source matters: the dashboard's blacklist summary counts
/// `manual` entries as the owner's own choices, so an automatic retirement
/// filed under `manual` would misreport who excluded the token.
fn retire_token(mint: &str, symbol: &str, reason: &str, cause: &QuoteError) {
    let Some(db) = crate::tokens::database::get_global_database() else {
        return;
    };
    match crate::tokens::cleanup::blacklist_token(mint, reason, "auto_swap_route", &db) {
        Ok(()) => logger::info(
            LogTag::Swap,
            &format!(
                "Blacklisted {symbol} ({}) as {reason}: {cause}",
                short_mint(mint)
            ),
        ),
        Err(e) => logger::warning(
            LogTag::Swap,
            &format!(
                "Failed to blacklist {symbol} ({}) as {reason}: {e}",
                short_mint(mint)
            ),
        ),
    }
}

/// First 8 characters of a mint for logs, without panicking on a short or
/// non-ASCII value.
fn short_mint(mint: &str) -> &str {
    mint.get(..8).unwrap_or(mint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::ChainId;
    use crate::errors::ErrorClass;

    fn not_tradable(router: &str) -> QuoteError {
        QuoteError::NotTradable {
            router: router.to_owned(),
            detail: "TOKEN_NOT_TRADABLE".to_owned(),
        }
    }

    fn no_route(router: &str) -> QuoteError {
        QuoteError::NoRoute {
            router: router.to_owned(),
            detail: "COULD_NOT_FIND_ANY_ROUTE".to_owned(),
        }
    }

    fn timeout(router: &str) -> QuoteError {
        QuoteError::Timeout {
            router: router.to_owned(),
        }
    }

    /// The regression this file's rewrite exists for. The old classifier
    /// rendered a friendly message for a no-route failure and the opening path
    /// then searched THAT message for the words it no longer contained, so a
    /// token with no market was never retired. The verdict now survives as a
    /// value, so no wording can lose it.
    #[test]
    fn a_no_market_verdict_survives_aggregation() {
        assert!(matches!(
            select_quote_failure(vec![timeout("GMGN"), not_tradable("Jupiter")]),
            QuoteError::NotTradable { .. }
        ));
        assert!(matches!(
            select_quote_failure(vec![timeout("GMGN"), no_route("Jupiter")]),
            QuoteError::NoRoute { .. }
        ));
    }

    /// One router timing out says nothing about the token, so it must not be
    /// what the caller acts on when another router gave a real verdict — and
    /// on its own it must never license retiring a mint.
    #[test]
    fn a_router_fault_never_becomes_a_verdict_on_the_token() {
        let only_faults = select_quote_failure(vec![
            timeout("GMGN"),
            QuoteError::Unavailable {
                router: "Jupiter".to_owned(),
                detail: "HTTP 503".to_owned(),
            },
        ]);
        assert!(only_faults.permanent_token_verdict().is_none());
        assert!(!only_faults.is_route_failure());
    }

    /// A token the routers cannot trade at all is a durable fact and retires
    /// the mint at once; failing to route a given size is not, and must repeat
    /// before it counts. The blacklist is permanent and hand-removable only.
    #[test]
    fn only_a_no_market_verdict_retires_a_token_immediately() {
        assert_eq!(
            not_tradable("Jupiter").permanent_token_verdict(),
            Some("NotTradable")
        );
        assert!(no_route("Jupiter").permanent_token_verdict().is_none());
        assert!(no_route("Jupiter").is_route_failure());
    }

    /// Being throttled must be answerable from the value, so back-off does not
    /// depend on a provider's wording.
    #[test]
    fn rate_limiting_is_visible_through_error_class() {
        let throttled = QuoteError::RateLimited {
            router: "Jupiter".to_owned(),
            retry_after: Some(std::time::Duration::from_secs(3)),
        };
        assert!(throttled.is_rate_limited());
        assert_eq!(throttled.http_status(), 429);
        assert_eq!(
            throttled.retry_after(),
            Some(std::time::Duration::from_secs(3))
        );

        // And it must still be answerable after folding into the crate channel.
        let folded = Error::from(throttled);
        assert!(folded.is_rate_limited());

        assert!(!not_tradable("Jupiter").is_rate_limited());
    }

    /// Statuses and codes are read by the trade dialog; they come from the
    /// variant, never from prose.
    #[test]
    fn every_variant_answers_with_its_own_status_and_code() {
        let cases = [
            (
                QuoteError::NoRoutersEnabled {
                    chain: ChainId::Solana,
                },
                503,
                "NoRouters",
            ),
            (not_tradable("Jupiter"), 422, "TokenNotTradable"),
            (no_route("Jupiter"), 422, "NoRoute"),
            (timeout("Jupiter"), 504, "QuoteTimeout"),
            (
                QuoteError::RouterRejected {
                    router: "Jupiter".to_owned(),
                    detail: "zero output".to_owned(),
                },
                502,
                "QuoteRejected",
            ),
        ];
        for (err, status, code) in cases {
            assert_eq!(err.http_status(), status, "{err}");
            assert_eq!(err.code(), code, "{err}");
            assert!(!err.hint().is_empty(), "{err}");
            assert!(!err.title().is_empty(), "{err}");
        }
    }

    /// A router handing back something unusable is our refusal of untrusted
    /// input on a money path, and must be loud — it is never retried and never
    /// blamed on the token.
    #[test]
    fn an_unusable_quote_is_critical_and_not_retryable() {
        let rejected = QuoteError::RouterRejected {
            router: "Jupiter".to_owned(),
            detail: "zero-output quote".to_owned(),
        };
        assert_eq!(rejected.severity(), crate::errors::Severity::Critical);
        assert!(!rejected.is_retryable());
        assert!(rejected.permanent_token_verdict().is_none());
    }
}
