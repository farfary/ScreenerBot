// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Core Swap Operations - High-level swap functions
//! Provides get_best_quote() and execute_swap_with_fallback()

use crate::chains::RawAmount;
use crate::logger::{self, LogTag};
use crate::swaps::error::{QuoteError, QuoteResult};
use crate::swaps::registry::{get_registry, try_get_registry, RouterRegistry};
use crate::swaps::router::SwapRouter;
use crate::swaps::types::{Quote, QuoteRequest, SwapAmountLimit, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    best_quote_on(quote_registry()?, request).await
}

/// The registry every quote is compared on. Its absence stays a ServiceError
/// all the way through: a swap service that never started is not a fact about
/// the token, and callers on the crate channel must still see WHICH service
/// failed.
fn quote_registry() -> QuoteResult<&'static RouterRegistry> {
    try_get_registry().ok_or_else(|| {
        QuoteError::RegistryUnavailable(crate::errors::ServiceError::Initialize {
            service: "swaps.registry".to_owned(),
            message: "router factory has not been registered".to_owned(),
        })
    })
}

/// The same comparison against an explicit registry.
///
/// Every path that picks a router for the user — the trading engine, the trade
/// dialog, the wallet tools' "Auto" — goes through THIS function, so "best" can
/// only ever mean one thing. Tests drive it with stub routers.
pub(crate) async fn best_quote_on(
    registry: &RouterRegistry,
    request: QuoteRequest,
) -> QuoteResult<Quote> {
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

    let deadline = crate::config::with_config(|cfg| {
        Duration::from_millis(cfg.chains.swaps(request.chain).quote_deadline_ms)
    });
    let start = Instant::now();
    let results = collect_quotes(&enabled, &request, deadline).await;
    let elapsed = start.elapsed();

    // Partition into successful quotes and per-router failures. Keeping the
    // failures lets us report the ACTUAL reason (e.g. token not tradable) to the
    // trade dialog instead of a generic "all routers failed" that hides it.
    let mut quotes: Vec<(u8, Quote, RawAmount)> = Vec::new();
    let mut errors: Vec<QuoteError> = Vec::new();
    for res in results {
        match res {
            Ok((q, net)) => {
                let priority = enabled
                    .iter()
                    .find(|router| router.id() == q.router_id)
                    .map(|router| router.priority())
                    .unwrap_or(u8::MAX);
                quotes.push((priority, q, net));
            }
            Err(e) => errors.push(e),
        }
    }

    if quotes.is_empty() {
        return Err(select_quote_failure(errors));
    }

    // Select best quote: the most output the wallet keeps after network fees.
    let best = quotes
        .into_iter()
        .max_by(
            |(left_priority, _, left_net), (right_priority, _, right_net)| {
                left_net
                    .cmp(right_net)
                    // `max_by` wins a greater ordering; reverse priority so the
                    // lower configured priority deterministically wins a tie.
                    .then_with(|| right_priority.cmp(left_priority))
            },
        )
        .map(|(_, quote, _)| quote)
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

/// Ask every router at once and collect each answer, validated, in router order.
///
/// The comparison waits for every router until the first VALID quote arrives;
/// from then on the remaining routers get `deadline` more, and a router still
/// pending after it becomes [`QuoteError::Timeout`] for this request. Before any
/// valid quote exists nothing is cut off: each router is bounded only by its
/// own transport timeout, so a market where every router is slow still trades,
/// and a fast refusal never arms the deadline against a slower router that can
/// still price the trade. A straggler's timeout is operational, never evidence
/// about the token, and it is only ever reported beside a valid quote.
async fn collect_quotes(
    routers: &[Arc<dyn SwapRouter>],
    request: &QuoteRequest,
    deadline: Duration,
) -> Vec<QuoteResult<(Quote, RawAmount)>> {
    let mut pending: FuturesUnordered<_> = routers
        .iter()
        .enumerate()
        .map(|(index, router)| {
            let router = router.clone();
            async move { (index, quote_from(router.as_ref(), request).await) }
        })
        .collect();

    let mut answers: Vec<Option<QuoteResult<(Quote, RawAmount)>>> =
        routers.iter().map(|_| None).collect();
    let mut cutoff: Option<tokio::time::Instant> = None;
    loop {
        let next = match cutoff {
            None => pending.next().await,
            Some(at) => match tokio::time::timeout_at(at, pending.next()).await {
                Ok(next) => next,
                Err(_) => break,
            },
        };
        let Some((index, answer)) = next else {
            break;
        };
        if cutoff.is_none() && answer.is_ok() {
            cutoff = Some(tokio::time::Instant::now() + deadline);
        }
        answers[index] = Some(answer);
    }

    routers
        .iter()
        .zip(answers)
        .map(|(router, answer)| {
            answer.unwrap_or_else(|| {
                logger::warning(
                    LogTag::Swap,
                    &format!(
                        "{} did not quote within {} ms of the first valid quote; compared without it",
                        router.name(),
                        deadline.as_millis()
                    ),
                );
                Err(QuoteError::Timeout {
                    router: router.name().to_owned(),
                })
            })
        })
        .collect()
}

/// One router's quote, validated and paired with its fee-net output.
async fn quote_from(
    router: &dyn SwapRouter,
    request: &QuoteRequest,
) -> QuoteResult<(Quote, RawAmount)> {
    match router.get_quote(request).await {
        Ok(quote) => {
            logger::info(
                LogTag::Swap,
                &format!(
                    "{}: {} output, {:.2}% impact",
                    router.name(),
                    quote.output_amount,
                    quote.price_impact_pct
                ),
            );
            validate_quote_with_net(router, request, quote)
        }
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!("{} quote failed: {e}", router.name()),
            );
            Err(e)
        }
    }
}

/// A quote's output with its estimated network fee taken out, so routers that
/// ask for different priority fees are compared on what the wallet keeps.
///
/// The fee is lamports of the native asset. When the native asset is the output
/// it is subtracted directly; when it is the input it is converted into output
/// units at the quote's own rate. A pair with no native leg, or a quote with no
/// estimate, compares on its raw output.
fn output_after_network_fee(quote: &Quote, router: &dyn SwapRouter) -> QuoteResult<RawAmount> {
    let output = quote.output_amount;
    let Some(fee) = quote.estimated_network_fee_lamports else {
        return Ok(output);
    };
    let fee = RawAmount::from(fee);
    let rejected = |detail: &str| QuoteError::RouterRejected {
        router: router.name().to_owned(),
        detail: detail.to_owned(),
    };
    let adapter = crate::chains::adapter();
    if adapter.is_native_asset(&quote.output_mint) {
        if fee >= output {
            return Err(rejected("network fee consumes the native output"));
        }
        output
            .checked_sub(fee)
            .ok_or_else(|| rejected("network fee exceeds output"))
    } else if adapter.is_native_asset(&quote.input_mint) {
        let input = quote.input_amount;
        if input == RawAmount::ZERO || fee >= input {
            return Err(rejected("network fee consumes the native input"));
        }
        let fee_in_output = output
            .checked_mul_div(fee, input)
            .ok_or_else(|| rejected("network fee conversion failed"))?;
        output
            .checked_sub(fee_in_output)
            .filter(|net| *net > RawAmount::ZERO)
            .ok_or_else(|| rejected("network fee consumes the quoted output"))
    } else {
        Ok(output)
    }
}

/// Reduce the per-router failures to the one verdict that best describes the
/// attempt as a whole.
///
/// Routers disagree: Jupiter may say the token is not tradable while the direct
/// engine merely found no pool it can build against. A verdict about the TOKEN
/// (`NotTradable`, then `NoRoute`) is returned only when every router that
/// answered supports it, because the opening path retires a token on exactly
/// that answer. Any other mix is an operational failure and is reported as the
/// most specific one, so a rate limit or a build fault can never be mistaken for
/// a dead market.
///
/// A router that does not offer the trade at all ([`QuoteError::NotOffered`])
/// abstains: it is left out before the unanimity test, so its capability never
/// suppresses the other routers' verdict, and a set made only of abstentions is
/// reported as one, never as a verdict.
///
/// This replaced a function that re-read its own output: it rendered a friendly
/// message for the "no route" case, and the opening path then searched that
/// message for the word "no route" — which the friendly wording no longer
/// contained, so no token was ever blacklisted for having no market.
fn select_quote_failure(errors: Vec<QuoteError>) -> QuoteError {
    let (abstentions, mut errors): (Vec<_>, Vec<_>) = errors
        .into_iter()
        .partition(|error| matches!(error, QuoteError::NotOffered { .. }));
    if errors.is_empty() {
        if let Some(abstention) = abstentions.into_iter().next() {
            return abstention;
        }
    }

    // A verdict ABOUT THE TOKEN may only be returned when every router that
    // answered agreed on it. One router saying "not tradable" while another
    // merely timed out is not evidence about the mint -- and the opening path
    // retires a token on exactly this answer, so a mixed result must degrade to
    // the operational failure it really was.
    let unanimous = |all: fn(&QuoteError) -> bool, errors: &[QuoteError]| {
        !errors.is_empty() && errors.iter().all(all)
    };
    if unanimous(
        |error| matches!(error, QuoteError::NotTradable { .. }),
        &errors,
    ) {
        return errors.remove(0);
    }
    if unanimous(
        |error| {
            matches!(
                error,
                QuoteError::NotTradable { .. } | QuoteError::NoRoute { .. }
            )
        },
        &errors,
    ) {
        // Every router could price nothing, but not all of them called the mint
        // dead: "no route" is the strongest claim the set supports.
        return errors
            .iter()
            .position(|error| matches!(error, QuoteError::NoRoute { .. }))
            .map(|index| errors.remove(index))
            .unwrap_or_else(|| errors.remove(0));
    }

    // Mixed set: report the most specific OPERATIONAL failure, and never a
    // token verdict -- a caller must not conclude anything about the mint from
    // a set that contains one router's rate limit or build fault.
    fn specificity(err: &QuoteError) -> u8 {
        match err {
            QuoteError::RouterRejected { .. } => 0,
            QuoteError::RateLimited { .. } => 1,
            QuoteError::Timeout { .. } => 2,
            QuoteError::Unavailable { .. } => 3,
            // Token verdicts rank last here BECAUSE the set is mixed: they are
            // the two variants a caller is licensed to act on permanently.
            QuoteError::NoRoute { .. } => 4,
            QuoteError::NotTradable { .. } => 5,
            QuoteError::NoRoutersEnabled { .. } => 6,
            // Unreachable from the per-router loop (the registry is resolved
            // before any router is asked), and least specific if it ever is.
            QuoteError::RegistryUnavailable(_) => 7,
            // Abstentions were set aside above and never reach this ranking.
            QuoteError::NotOffered { .. } => 8,
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

/// A router's answer is untrusted input on a money path.
///
/// Selection is the last point at which a quote that prices a different pair,
/// spends a different amount, or guarantees nothing is still cheap to refuse --
/// after it, the quote becomes a signed transaction. Each check names what it
/// rejected so a real provider regression is diagnosable from one log line.
pub(crate) fn validate_quote(
    router: &dyn SwapRouter,
    request: &QuoteRequest,
    quote: Quote,
) -> QuoteResult<Quote> {
    validate_quote_with_net(router, request, quote).map(|(quote, _)| quote)
}

fn validate_quote_with_net(
    router: &dyn SwapRouter,
    request: &QuoteRequest,
    quote: Quote,
) -> QuoteResult<(Quote, RawAmount)> {
    let reject = |detail: String| {
        Err(QuoteError::RouterRejected {
            router: router.name().to_owned(),
            detail,
        })
    };

    if quote.router_id != router.id() {
        return reject(format!(
            "quote claims router '{}' but came from '{}'",
            quote.router_id,
            router.id()
        ));
    }
    if quote.chain != request.chain {
        return reject(format!(
            "quoted chain {:?} but {:?} was requested",
            quote.chain, request.chain
        ));
    }
    if quote.input_mint != request.input_mint || quote.output_mint != request.output_mint {
        return reject(format!(
            "quoted {} -> {} but {} -> {} was requested",
            quote.input_mint, quote.output_mint, request.input_mint, request.output_mint
        ));
    }
    if quote.wallet_address != request.wallet_address {
        return reject("quote is addressed to a different wallet".to_owned());
    }
    if quote.input_amount != request.input_amount {
        return reject(format!(
            "quote spends {} but {} was requested",
            quote.input_amount, request.input_amount
        ));
    }
    if quote.input_amount == RawAmount::ZERO {
        return reject("zero-input quote".to_owned());
    }
    if quote.swap_mode != request.swap_mode {
        return reject(format!(
            "quoted {:?} but {:?} was requested",
            quote.swap_mode, request.swap_mode
        ));
    }
    if quote.output_amount == RawAmount::ZERO {
        return reject(format!(
            "zero-output quote for {} -> {}",
            quote.input_mint, quote.output_mint
        ));
    }
    // A quote with no floor is a swap with no protection: the guaranteed
    // minimum is what the instruction (or the aggregator's threshold) enforces
    // on chain, and comparing routers on expected output alone would let an
    // unprotected quote win.
    if quote.minimum_output_amount == RawAmount::ZERO {
        return reject("quote guarantees no minimum output".to_owned());
    }
    if quote.minimum_output_amount > quote.output_amount {
        return reject(format!(
            "guaranteed minimum {} exceeds the expected output {}",
            quote.minimum_output_amount, quote.output_amount
        ));
    }
    if !quote.price_impact_pct.is_finite() || quote.price_impact_pct < 0.0 {
        return reject(format!("unusable price impact {}", quote.price_impact_pct));
    }

    let net = output_after_network_fee(&quote, router)?;
    Ok((quote, net))
}

// ============================================================================
// SWAP EXECUTION WITH FALLBACK
// ============================================================================

/// Who signs a swap.
#[derive(Clone, Copy)]
pub(crate) enum SwapSigner<'a> {
    /// The main trading wallet, trading this token.
    MainWallet(&'a Token),
    /// One wallet of the wallet tools, by id.
    Wallet(i64),
}

/// Which routers a swap that failed before submission may be re-quoted on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fallback {
    /// Every other enabled router of the chain, by priority.
    AnyRouter,
    /// Only the router that produced the quote: a caller who named a router
    /// wants that router or an error.
    SameRouter,
}

/// Execute `quote` on its own router, signing as `signer`.
async fn execute_on(
    router: &dyn SwapRouter,
    signer: SwapSigner<'_>,
    quote: &Quote,
) -> Result<SwapResult> {
    match signer {
        SwapSigner::MainWallet(token) => router.execute_swap(token, quote).await,
        SwapSigner::Wallet(wallet_id) => router.execute_swap_for_wallet(quote, wallet_id).await,
    }
}

/// Execute a main-wallet swap, falling back to the other enabled routers when
/// the primary provably sent nothing.
pub async fn execute_swap_with_fallback(
    token: &Token,
    quote: Quote,
    amount_limit: SwapAmountLimit,
) -> Result<SwapResult> {
    // Block swap execution during force stop
    if crate::global::is_force_stopped() {
        return Err(Error::internal_error(
            "Trading halted - Force stop is active",
        ));
    }
    let registry = get_registry()?;
    execute_with_fallback_on(
        registry,
        SwapSigner::MainWallet(token),
        quote,
        amount_limit,
        Fallback::AnyRouter,
    )
    .await
    .map(|(_, result)| result)
}

/// The one execution chain for every signer: the quote's own router first, a
/// re-quote on that router without a venue that cost too much, then — when
/// `fallback` allows and the failure provably sent nothing — every other
/// enabled router by priority. Returns the quote that actually executed with
/// its result.
pub(crate) async fn execute_with_fallback_on(
    registry: &RouterRegistry,
    signer: SwapSigner<'_>,
    quote: Quote,
    amount_limit: SwapAmountLimit,
    fallback: Fallback,
) -> Result<(Quote, SwapResult)> {
    // Get primary router
    let primary = registry
        .get_router(&quote.router_id)
        .ok_or_else(|| Error::internal_error(format!("Router {} not found", quote.router_id)))?;
    // The registry answers live: a router disabled between quoting and
    // execution must not still execute the quote it produced.
    if !primary.is_enabled() {
        return Err(Error::configuration_error(format!(
            "Router {} was disabled before its quote could be executed",
            quote.router_name
        )));
    }

    amount_limit.check_quote(&quote).map_err(Error::from)?;

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

    super::progress::report_swap_stage(super::progress::SwapStage::Submitting {
        router: primary.name().to_owned(),
    })
    .await;
    let primary_error = match execute_on(primary.as_ref(), signer, &quote).await {
        Ok(result) => return finish(signer, quote, result, start, amount_limit),
        Err(error) => error,
    };

    // NEVER fall back on a swap that was already SUBMITTED. The confirmation poll
    // timed out, but the transaction can still land — re-sending it through another
    // router is a second, real swap.
    let primary_failure = failed_swap(&primary_error);
    if let FailedSwap::Reconcile { signature } = &primary_failure {
        logger::warning(
            LogTag::Swap,
            &format!(
                "Swap {signature} submitted via {} but not confirmed in time - NOT retrying (it may still land); verification will reconcile it",
                primary.name()
            ),
        );
        return Err(primary_error);
    }

    // A refusal that NAMES a venue is answerable without giving up on this
    // router: the same aggregator can usually price the same trade through a
    // different venue, and that is a better trade than the next router's quote.
    // Bounded to one attempt by the exclusion itself — the retry carries the
    // venue in `exclude_dexes`, so a second refusal for the same venue cannot
    // recur.
    if let Some(outcome) = retry_excluding_venue(
        signer,
        &quote,
        &primary_error,
        primary.as_ref(),
        start,
        amount_limit,
    )
    .await
    {
        return outcome;
    }

    if primary_failure == FailedSwap::Unresolved {
        logger::error(
            LogTag::Swap,
            &format!(
                "{} swap failed, and nothing proves it was not sent - not falling back: {}",
                primary.name(),
                primary_error
            ),
        );
        return Err(primary_error);
    }
    if fallback == Fallback::SameRouter {
        logger::warning(
            LogTag::Swap,
            &format!(
                "{} swap was not sent and only that router was asked for: {primary_error}",
                primary.name()
            ),
        );
        return Err(primary_error);
    }

    logger::warning(
        LogTag::Swap,
        &format!(
            "{} swap was not sent: {} - trying fallback...",
            primary.name(),
            primary_error
        ),
    );

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

        let fallback_request = requote_request(&quote, quote.exclude_dexes.clone());

        // The fallback's answer is untrusted input exactly like the one that
        // won the original comparison: a quote that prices another pair or
        // guarantees nothing must not reach the builder just because it
        // arrived on the retry path.
        let fallback_quote = match fallback_router
            .get_quote(&fallback_request)
            .await
            .and_then(|quote| validate_quote(fallback_router.as_ref(), &fallback_request, quote))
        {
            Ok(q) => q,
            Err(e) => {
                logger::warning(
                    LogTag::Swap,
                    &format!("{} quote failed: {}", fallback_router.name(), e),
                );
                continue;
            }
        };

        if let Err(e) = amount_limit.check_quote(&fallback_quote) {
            logger::warning(
                LogTag::Swap,
                &format!("{} quote rejected: {e}", fallback_router.name()),
            );
            continue;
        }

        super::progress::report_swap_stage(super::progress::SwapStage::Submitting {
            router: fallback_router.name().to_owned(),
        })
        .await;
        match execute_on(fallback_router.as_ref(), signer, &fallback_quote).await {
            Ok(result) => return finish(signer, fallback_quote, result, start, amount_limit),
            Err(e) => {
                // Same rule as the primary: a submitted-but-unconfirmed swap must
                // not be re-sent through yet another router, and neither may a
                // failure that cannot prove it was never sent.
                match failed_swap(&e) {
                    FailedSwap::Reconcile { signature } => {
                        logger::warning(
                            LogTag::Swap,
                            &format!(
                                "Fallback swap {signature} submitted via {} but not confirmed in time - stopping the chain (it may still land)",
                                fallback_router.name()
                            ),
                        );
                        return Err(e);
                    }
                    FailedSwap::Unresolved => {
                        logger::error(
                            LogTag::Swap,
                            &format!(
                                "{} fallback failed, and nothing proves it was not sent - stopping the chain: {e}",
                                fallback_router.name()
                            ),
                        );
                        return Err(e);
                    }
                    FailedSwap::Resendable => {
                        logger::warning(
                            LogTag::Swap,
                            &format!("{} execution failed: {}", fallback_router.name(), e),
                        );
                        continue;
                    }
                }
            }
        }
    }

    // All fallbacks failed - return original error
    logger::error(LogTag::Swap, "All routers failed (primary + all fallbacks)");
    Err(primary_error)
}

/// Record a completed swap: post-swap cleanup for the main wallet, the total
/// execution time, and the caller's amount range.
fn finish(
    signer: SwapSigner<'_>,
    quote: Quote,
    mut result: SwapResult,
    start: Instant,
    amount_limit: SwapAmountLimit,
) -> Result<(Quote, SwapResult)> {
    if matches!(signer, SwapSigner::MainWallet(_)) {
        schedule_post_swap_cleanup(&quote);
    }
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
    Ok((quote, amount_limit.check_result(result)?))
}

/// The request that re-prices `quote` on another router or without a venue.
fn requote_request(quote: &Quote, exclude_dexes: Option<Vec<String>>) -> QuoteRequest {
    QuoteRequest {
        chain: quote.chain,
        input_mint: quote.input_mint.clone(),
        output_mint: quote.output_mint.clone(),
        input_amount: quote.input_amount,
        wallet_address: quote.wallet_address.clone(),
        slippage_pct: (quote.slippage_bps as f64) / 100.0,
        swap_mode: quote.swap_mode,
        exclude_dexes,
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Whether a fresh quote still pays at least the floor the caller accepted.
///
/// A re-quote derives a fresh minimum from the new price, so taking it as-is
/// would apply slippage a second time and execute below what the comparison
/// chose. The fresh EXPECTED output must reach the accepted guaranteed minimum.
fn holds_accepted_floor(accepted: &Quote, fresh: &Quote) -> bool {
    fresh.output_amount >= accepted.minimum_output_amount
}

/// Re-quote and re-execute through `router` with the venue that just refused
/// the trade excluded.
///
/// Returns `None` when there is nothing to retry — the failure named no venue,
/// the caller turned the retry off, the aggregator has no label for that
/// program, the venue was already excluded, or the fresh quote falls under the
/// accepted floor — leaving the normal fallback chain to run. It returns
/// `Some(Err)` only for a retry whose transaction may have reached the
/// network, which must never be re-sent by anyone else.
async fn retry_excluding_venue(
    signer: SwapSigner<'_>,
    quote: &Quote,
    error: &Error,
    router: &dyn crate::swaps::router::SwapRouter,
    start: Instant,
    amount_limit: SwapAmountLimit,
) -> Option<Result<(Quote, SwapResult)>> {
    let Error::Swaps(crate::swaps::SwapExecutionError::NotSubmitted {
        reason: crate::swaps::NotSubmittedReason::CostExceeded { venue_address, .. },
        ..
    }) = error
    else {
        return None;
    };
    if !crate::chains::solana::swaps::cost_guard::CostGuardSettings::current().retry_excluding_venue
    {
        return None;
    }

    let label =
        crate::chains::solana::swaps::routers::venue_label_for_program(venue_address).await?;
    let already_excluded = quote
        .exclude_dexes
        .iter()
        .flatten()
        .any(|excluded| excluded.trim().eq_ignore_ascii_case(&label));
    if already_excluded {
        return None;
    }

    let mut exclude_dexes = quote.exclude_dexes.clone().unwrap_or_default();
    exclude_dexes.push(label.clone());
    let request = requote_request(quote, Some(exclude_dexes));

    logger::info(
        LogTag::Swap,
        &format!("Re-quoting on {} without {label}", router.name()),
    );

    let retry_quote = match router
        .get_quote(&request)
        .await
        .and_then(|quote| validate_quote(router, &request, quote))
    {
        Ok(quote) => quote,
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!(
                    "{} could not price the trade without {label}: {e}",
                    router.name()
                ),
            );
            return None;
        }
    };

    if !holds_accepted_floor(quote, &retry_quote) {
        logger::warning(
            LogTag::Swap,
            &format!(
                "{} without {label} expects {}, below the {} floor the trade was accepted at",
                router.name(),
                retry_quote.output_amount,
                quote.minimum_output_amount
            ),
        );
        return None;
    }

    if let Err(e) = amount_limit.check_quote(&retry_quote) {
        logger::warning(
            LogTag::Swap,
            &format!("{} quote rejected: {e}", router.name()),
        );
        return None;
    }

    super::progress::report_swap_stage(super::progress::SwapStage::Submitting {
        router: router.name().to_owned(),
    })
    .await;
    match execute_on(router, signer, &retry_quote).await {
        Ok(result) => Some(finish(signer, retry_quote, result, start, amount_limit)),
        Err(e) => {
            if failed_swap(&e) != FailedSwap::Resendable {
                logger::warning(
                    LogTag::Swap,
                    &format!(
                        "Retry via {} without {label} may have reached the network - NOT retrying further: {e}",
                        router.name()
                    ),
                );
                return Some(Err(e));
            }
            logger::warning(
                LogTag::Swap,
                &format!(
                    "{} still could not execute without {label}: {e}",
                    router.name()
                ),
            );
            None
        }
    }
}

/// Hand a confirmed swap with a native-asset leg to the post-swap cleanup,
/// which reclaims the wrapped-SOL account a router may have left open.
fn schedule_post_swap_cleanup(quote: &Quote) {
    let adapter = crate::chains::adapter();
    if adapter.is_native_asset(&quote.input_mint) || adapter.is_native_asset(&quote.output_mint) {
        crate::chains::solana::assets::ata::schedule_wsol_sweep();
    }
}

/// The signature of a swap that reached the chain: submitted but unconfirmed, or completed outside its caller's amount range.
///
/// A confirmation poll that runs out returns an error even though the transaction may still
/// land — a Solana transaction stays valid until its blockhash expires, well beyond our poll
/// window.
///
/// Retrying such a swap is a DOUBLE SPEND: the fallback chain would submit the same sell
/// through another router, and the exit's slippage ladder would submit it again at the next
/// rung. On a full close the second sell usually just fails on an empty balance (wasted
/// fee), but on a PARTIAL exit the tokens are still there — so a "sell 25%" that timed out
/// once actually sells 25% twice, and the position records only one of them.
///
/// The signature is recovered so a caller can stop retrying and hand it to verification,
/// which reconciles what really happened on chain. Every path answers from a typed
/// outcome that carries the signature as data, never from the error's text.
pub fn unconfirmed_swap_signature(error: &Error) -> Option<String> {
    match error {
        // A completed swap whose amounts exceed the caller's range is a trade that
        // happened: hand it to verification, never send it again.
        Error::Swaps(crate::swaps::SwapExecutionError::CompletedAmountOutOfRange {
            signature,
            ..
        }) => Some(signature.clone()),
        // An aggregator transaction that was sent and whose confirmation poll ran out.
        Error::Solana(crate::chains::solana::Error::Execution(
            crate::chains::ExecutionFailure::ConfirmationTimeout { reference, .. },
        )) => Some(reference.clone()),
        // The direct engine already knows the answer as DATA. `settled_signature` covers
        // both a confirmation that timed out (it may still land) and a swap that
        // CONFIRMED WITHOUT ERROR whose receipt could not be measured -- the latter is a
        // trade that provably happened, and treating it as one that never did leaves the
        // wallet holding tokens no position was ever created for.
        Error::Solana(crate::chains::solana::Error::DirectSwap(direct)) => {
            direct.settled_signature().map(str::to_owned)
        }
        _ => None,
    }
}

#[cfg(test)]
mod submitted_timeout_tests {
    use super::unconfirmed_swap_signature;
    use crate::chains::ExecutionFailure;
    use crate::Error;

    /// A real (well-formed) mainnet signature: 88 base58 characters.
    const SIGNATURE: &str =
        "5VERv8NMvzbJMEkV8xnrLkEaWRtSz9CosKDYjCJjBRnbJLgp8uirBgmQpjKhoR4tjF3ZpRzrFmBV6UjKdiSZkQUW";

    fn execution(failure: ExecutionFailure) -> Error {
        Error::Solana(crate::chains::solana::Error::Execution(failure))
    }

    #[test]
    fn a_submitted_timeout_recovers_the_signature_for_verification() {
        assert_eq!(
            unconfirmed_swap_signature(&execution(ExecutionFailure::ConfirmationTimeout {
                reference: SIGNATURE.to_owned(),
                waited_ms: 60_000,
            })),
            Some(SIGNATURE.to_owned())
        );
    }

    /// A transaction the chain reverted moved nothing and can never land again, and
    /// a failure before submission has no signature at all: neither is handed to
    /// verification as a trade that may have happened.
    #[test]
    fn a_reverted_or_never_sent_swap_is_not_treated_as_submitted() {
        assert_eq!(
            unconfirmed_swap_signature(&execution(ExecutionFailure::Reverted {
                reference: SIGNATURE.to_owned(),
                detail: "custom program error".to_owned(),
            })),
            None
        );
        assert_eq!(
            unconfirmed_swap_signature(&Error::api_error(format!(
                "Transaction {SIGNATURE} not confirmed within timeout"
            ))),
            None,
            "prose that merely reads like a timeout is never a signature"
        );
    }
}

/// What a failed swap leaves behind, read from its type alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailedSwap {
    /// It reached the chain or may still land: reconcile `signature`, never
    /// send the trade again.
    Reconcile { signature: String },
    /// Provably nothing moved and nothing can still land: the trade may be
    /// quoted and sent again.
    Resendable,
    /// Nothing proves it was not sent, and there is no signature to reconcile:
    /// stop without sending again.
    Unresolved,
}

/// The one reading every caller that may send a trade again decides from:
/// the fallback chain, the venue-exclusion retry and the exit ladders. A
/// recoverable signature always wins, and only [`is_fallback_safe`] licenses
/// another send.
pub fn failed_swap(error: &Error) -> FailedSwap {
    match unconfirmed_swap_signature(error) {
        Some(signature) => FailedSwap::Reconcile { signature },
        None if is_fallback_safe(error) => FailedSwap::Resendable,
        None => FailedSwap::Unresolved,
    }
}

/// Whether a failed swap proves nothing moved and nothing can still land: the
/// one licence to quote and execute the same trade through another router, and
/// to forget the attempt was made.
///
/// Decided from the outcome's TYPE only, never from an RPC code or the error's
/// text. Every match below is exhaustive, so a new variant cannot compile
/// without a decision; anything outside the swap vocabularies proves nothing.
pub fn is_fallback_safe(error: &Error) -> bool {
    match error {
        Error::Swaps(error) => swap_execution_fallback_safe(error),
        Error::Solana(error) => solana_fallback_safe(error),
        _ => false,
    }
}

/// Why a swap stopped before it was sent, when its outcome says so: the
/// shared [`NotSubmittedReason`], or the direct engine's own oversized refusal
/// read in the same vocabulary.
pub fn not_submitted_reason(error: &Error) -> Option<crate::swaps::NotSubmittedReason> {
    match error {
        Error::Swaps(crate::swaps::SwapExecutionError::NotSubmitted { reason, .. }) => {
            Some(reason.clone())
        }
        Error::Solana(crate::chains::solana::Error::DirectSwap(
            crate::chains::solana::swaps::direct::DirectSwapError::TransactionTooLarge {
                bytes,
                limit,
            },
        )) => Some(crate::swaps::NotSubmittedReason::TransactionTooLarge {
            bytes: *bytes,
            limit: *limit,
        }),
        _ => None,
    }
}

fn swap_execution_fallback_safe(error: &crate::swaps::SwapExecutionError) -> bool {
    use crate::swaps::SwapExecutionError;
    match error {
        SwapExecutionError::NotSubmitted { .. } => true,
        // The trade happened; it is reconciled, never sent again.
        SwapExecutionError::CompletedAmountOutOfRange { .. } => false,
    }
}

fn solana_fallback_safe(error: &crate::chains::solana::Error) -> bool {
    use crate::chains::solana::Error as Solana;
    match error {
        // The direct engine answers from its own typed outcome: only a failure
        // that PROVES nothing moved (and nothing can still land) qualifies.
        Solana::DirectSwap(direct) => direct.safe_to_fallback(),
        // A transaction the chain reverted at `Confirmed` or that provably
        // expired unseen can never land: nothing moved but a fee.
        Solana::Execution(
            crate::chains::ExecutionFailure::Reverted { .. }
            | crate::chains::ExecutionFailure::Expired { .. },
        ) => true,
        // Refused before or at its send: it never reached the chain.
        Solana::NotSent(_) => true,
        // A confirmation that ran out may still land, and the rest are not
        // outcomes of a send at all; nothing about them proves the trade
        // did not happen.
        Solana::Execution(
            crate::chains::ExecutionFailure::ConfirmationTimeout { .. }
            | crate::chains::ExecutionFailure::NotFound { .. }
            | crate::chains::ExecutionFailure::IndexingDelay { .. },
        )
        | Solana::InvalidAddress { .. }
        | Solana::InvalidKeypair { .. }
        | Solana::KeypairUnavailable { .. }
        | Solana::SecureStorage(_)
        | Solana::Rpc { .. }
        | Solana::RpcFailure { .. }
        | Solana::AccountNotFound { .. }
        | Solana::Decode { .. }
        | Solana::InvalidPool { .. }
        | Solana::InstructionBuild { .. } => false,
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
    opening_quote_on(quote_registry()?, request, token_symbol).await
}

/// The opening comparison against an explicit registry. Tests drive it with
/// stub routers.
async fn opening_quote_on(
    registry: &RouterRegistry,
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

    match best_quote_on(registry, request).await {
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
    let Some(db) = crate::tokens::database::database(crate::chains::active_chain()) else {
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
    use crate::swaps::error::NotOfferedReason;
    use crate::swaps::types::SwapMode;
    use async_trait::async_trait;

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

    fn not_offered(router: &str, reason: NotOfferedReason) -> QuoteError {
        QuoteError::NotOffered {
            router: router.to_owned(),
            reason,
        }
    }

    /// One failure of every variant, named by the letter the table below uses.
    fn one_of_each() -> Vec<(char, QuoteError)> {
        vec![
            (
                'G',
                QuoteError::RegistryUnavailable(crate::errors::ServiceError::Initialize {
                    service: "swaps.registry".to_owned(),
                    message: "stub".to_owned(),
                }),
            ),
            (
                'E',
                QuoteError::NoRoutersEnabled {
                    chain: ChainId::Solana,
                },
            ),
            ('T', not_tradable("T")),
            ('N', no_route("N")),
            (
                'L',
                QuoteError::RateLimited {
                    router: "L".to_owned(),
                    retry_after: None,
                },
            ),
            ('O', timeout("O")),
            (
                'J',
                QuoteError::RouterRejected {
                    router: "J".to_owned(),
                    detail: "stub".to_owned(),
                },
            ),
            ('A', not_offered("A", NotOfferedReason::UnsupportedVenue)),
            (
                'U',
                QuoteError::Unavailable {
                    router: "U".to_owned(),
                    detail: "stub".to_owned(),
                },
            ),
        ]
    }

    fn letter(error: &QuoteError) -> char {
        match error {
            QuoteError::RegistryUnavailable(_) => 'G',
            QuoteError::NoRoutersEnabled { .. } => 'E',
            QuoteError::NotTradable { .. } => 'T',
            QuoteError::NoRoute { .. } => 'N',
            QuoteError::RateLimited { .. } => 'L',
            QuoteError::Timeout { .. } => 'O',
            QuoteError::RouterRejected { .. } => 'J',
            QuoteError::NotOffered { .. } => 'A',
            QuoteError::Unavailable { .. } => 'U',
        }
    }

    /// Every ordered pair of failure variants, reduced. Rows and columns follow
    /// `one_of_each`: G registry unavailable, E no routers enabled, T not
    /// tradable, N no route, L rate limited, O timeout, J router rejected,
    /// A not offered (an abstention), U unavailable.
    ///
    /// What the table pins: an abstention never changes the other router's
    /// answer (row and column A repeat the diagonal), two abstentions stay an
    /// abstention, a token verdict needs every voter (T with N is N, T or N
    /// beside an operational failure is that failure), and any mix reports
    /// the most specific operational failure.
    #[test]
    fn select_quote_failure_reduces_every_pair_of_variants() {
        const COLUMNS: &str = "GETNLOJAU";
        const TABLE: [(char, &str); 9] = [
            ('G', "GETNLOJGU"),
            ('E', "EETNLOJEU"),
            ('T', "TTTNLOJTU"),
            ('N', "NNNNLOJNU"),
            ('L', "LLLLLLJLL"),
            ('O', "OOOOLOJOO"),
            ('J', "JJJJJJJJJ"),
            ('A', "GETNLOJAU"),
            ('U', "UUUULOJUU"),
        ];
        let failures = one_of_each();
        assert_eq!(
            failures.iter().map(|(name, _)| *name).collect::<String>(),
            COLUMNS
        );
        for ((row, first), (expected_row, expected)) in failures.iter().zip(TABLE) {
            assert_eq!(*row, expected_row);
            for ((column, second), want) in failures.iter().zip(expected.chars()) {
                let got = letter(&select_quote_failure(vec![first.clone(), second.clone()]));
                assert_eq!(got, want, "{row} with {column}");
            }
        }
    }

    /// The set an always-on direct router produces most often: the aggregator
    /// finds no route and the direct engine does not offer the pool. The
    /// abstention must not hide the aggregator's verdict, and an RPC fault in
    /// its place still must.
    #[test]
    fn an_abstention_is_not_a_vote() {
        let reduced = select_quote_failure(vec![
            no_route("Jupiter"),
            not_offered("Direct Pool", NotOfferedReason::UnsupportedVenue),
        ]);
        assert!(matches!(&reduced, QuoteError::NoRoute { router, .. } if router == "Jupiter"));
        assert!(reduced.is_route_failure());

        let only_abstentions = select_quote_failure(vec![
            not_offered("Direct Pool", NotOfferedReason::ExactOut),
            not_offered("Raptor", NotOfferedReason::ExactOut),
        ]);
        assert!(matches!(only_abstentions, QuoteError::NotOffered { .. }));
        assert!(!only_abstentions.is_route_failure());
        assert!(only_abstentions.permanent_token_verdict().is_none());
        assert!(!only_abstentions.is_retryable());

        let rpc_fault = select_quote_failure(vec![
            no_route("Jupiter"),
            QuoteError::Unavailable {
                router: "Direct Pool".to_owned(),
                detail: "account read failed".to_owned(),
            },
        ]);
        assert!(!rpc_fault.is_route_failure());
    }

    /// The regression this file's rewrite exists for. The old classifier
    /// rendered a friendly message for a no-route failure and the opening path
    /// then searched THAT message for the words it no longer contained, so a
    /// token with no market was never retired. The verdict now survives as a
    /// value, so no wording can lose it.
    #[test]
    fn a_no_market_verdict_survives_aggregation() {
        assert!(matches!(
            select_quote_failure(vec![not_tradable("Direct Pool"), not_tradable("Jupiter")]),
            QuoteError::NotTradable { .. }
        ));
        assert!(matches!(
            select_quote_failure(vec![no_route("Direct Pool"), no_route("Jupiter")]),
            QuoteError::NoRoute { .. }
        ));
    }

    /// Retiring a token is permanent, so it takes agreement: only a set in
    /// which every router that answered spoke about the MINT may return a
    /// verdict about the mint. One router calling it dead while another merely
    /// had no pool is worth a no-route strike, never retirement.
    #[test]
    fn a_token_verdict_requires_every_router_to_agree() {
        assert!(matches!(
            select_quote_failure(vec![not_tradable("Jupiter"), no_route("Direct Pool")]),
            QuoteError::NoRoute { .. }
        ));
    }

    /// The case that must never blacklist: the direct engine having no pool
    /// says nothing about a token whose aggregator quote merely timed out, and
    /// neither does an aggregator verdict beside a rate-limited second router.
    #[test]
    fn an_operational_failure_never_becomes_a_token_verdict() {
        for errors in [
            vec![no_route("Direct Pool"), timeout("Jupiter")],
            vec![not_tradable("Jupiter"), timeout("Direct Pool")],
            vec![
                not_tradable("Jupiter"),
                QuoteError::RateLimited {
                    router: "Direct Pool".to_owned(),
                    retry_after: None,
                },
            ],
        ] {
            let verdict = select_quote_failure(errors);
            assert!(
                !verdict.is_route_failure(),
                "a mixed set must not license a token verdict, got {verdict}"
            );
        }
    }

    /// One router timing out says nothing about the token, so it must not be
    /// what the caller acts on when another router gave a real verdict — and
    /// on its own it must never license retiring a mint.
    #[test]
    fn a_router_fault_never_becomes_a_verdict_on_the_token() {
        let only_faults = select_quote_failure(vec![
            timeout("Raydium"),
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

    /// Statuses and catalog messages are read by the trade dialog; they come from the
    /// variant, never from prose.
    #[test]
    fn every_variant_answers_with_its_own_status_and_message() {
        let cases = [
            (
                QuoteError::NoRoutersEnabled {
                    chain: ChainId::Solana,
                },
                503,
                "errors-trade-quote-no-routers-enabled",
            ),
            (
                not_tradable("Jupiter"),
                422,
                "errors-trade-quote-not-tradable",
            ),
            (no_route("Jupiter"), 422, "errors-trade-quote-no-route"),
            (timeout("Jupiter"), 504, "errors-trade-quote-timeout"),
            (
                QuoteError::RouterRejected {
                    router: "Jupiter".to_owned(),
                    detail: "zero output".to_owned(),
                },
                502,
                "errors-trade-quote-router-rejected",
            ),
            (
                not_offered("Direct Pool", NotOfferedReason::ExactOut),
                422,
                "errors-trade-quote-not-offered-exact-out",
            ),
            (
                not_offered("Direct Pool", NotOfferedReason::UnsupportedVenue),
                422,
                "errors-trade-quote-not-offered-unsupported-venue",
            ),
        ];
        let source: crate::i18n::LanguageIdentifier = crate::i18n::source_locale().parse().unwrap();
        for (err, status, id) in cases {
            assert_eq!(err.http_status(), status, "{err}");
            assert_eq!(err.ui_text().id, id, "{err}");
            let message = crate::i18n::format_message(&source, id, None).expect("catalog message");
            assert!(
                message.value.is_some_and(|title| !title.is_empty()),
                "{err}"
            );
            assert!(
                message
                    .attributes
                    .iter()
                    .any(|(name, hint)| name == "hint" && !hint.is_empty()),
                "{err}"
            );
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

    struct StubRouter;

    #[async_trait]
    impl SwapRouter for StubRouter {
        fn id(&self) -> &'static str {
            "direct"
        }
        fn name(&self) -> &'static str {
            "Direct Pool"
        }
        fn is_enabled(&self) -> bool {
            true
        }
        fn priority(&self) -> u8 {
            1
        }
        fn chain(&self) -> ChainId {
            ChainId::Solana
        }
        async fn get_quote(&self, _request: &QuoteRequest) -> crate::swaps::QuoteResult<Quote> {
            Err(QuoteError::NoRoute {
                router: self.name().to_owned(),
                detail: "stub".to_owned(),
            })
        }
        async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> crate::Result<SwapResult> {
            Err(crate::Error::internal_error("stub"))
        }
    }

    fn request() -> QuoteRequest {
        QuoteRequest {
            chain: ChainId::Solana,
            input_mint: "So11111111111111111111111111111111111111112".to_owned(),
            output_mint: "TokenMint111111111111111111111111111111111".to_owned(),
            input_amount: 1_000_000u64.into(),
            wallet_address: "Wallet1111111111111111111111111111111111111".to_owned(),
            slippage_pct: 1.0,
            swap_mode: SwapMode::ExactIn,
            exclude_dexes: None,
        }
    }

    fn quote_for(request: &QuoteRequest) -> Quote {
        Quote {
            chain: request.chain,
            router_id: "direct".to_owned(),
            router_name: "Direct Pool".to_owned(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount: 1_000u64.into(),
            minimum_output_amount: 950u64.into(),
            price_impact_pct: 0.5,
            platform_fee_lamports: None,
            estimated_network_fee_lamports: None,
            slippage_bps: 100,
            route_plan: "RAYDIUM CLMM".to_owned(),
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            exclude_dexes: request.exclude_dexes.clone(),
            execution_data: Vec::new(),
        }
    }

    #[test]
    fn a_well_formed_quote_survives_validation() {
        let request = request();
        assert!(validate_quote(&StubRouter, &request, quote_for(&request)).is_ok());
    }

    /// Each of these would reach the transaction builder if selection let it
    /// through: a quote for another pair, another wallet, another size, or one
    /// that guarantees nothing at all.
    #[test]
    fn a_quote_that_does_not_answer_the_request_is_refused() {
        let request = request();
        let mutations: Vec<(&str, fn(&mut Quote))> = vec![
            ("wrong pair", |quote| {
                std::mem::swap(&mut quote.input_mint, &mut quote.output_mint)
            }),
            ("wrong wallet", |quote| {
                quote.wallet_address = "OtherWallet11111111111111111111111111111111".to_owned()
            }),
            ("wrong size", |quote| {
                quote.input_amount = quote.input_amount.checked_add(1u64.into()).unwrap()
            }),
            ("zero output", |quote| quote.output_amount = RawAmount::ZERO),
            ("no floor", |quote| {
                quote.minimum_output_amount = RawAmount::ZERO
            }),
            ("floor above output", |quote| {
                quote.minimum_output_amount = quote.output_amount.checked_add(1u64.into()).unwrap()
            }),
            ("unusable impact", |quote| quote.price_impact_pct = f64::NAN),
            ("foreign router", |quote| {
                quote.router_id = "jupiter".to_owned()
            }),
        ];

        for (case, mutate) in mutations {
            let mut quote = quote_for(&request);
            mutate(&mut quote);
            assert!(
                matches!(
                    validate_quote(&StubRouter, &request, quote),
                    Err(QuoteError::RouterRejected { .. })
                ),
                "{case} must be refused before it can be built"
            );
        }
    }

    #[test]
    fn network_fee_requires_positive_wallet_output() {
        let request = request();
        let mut quote = quote_for(&request);
        quote.input_mint = "TokenMint111111111111111111111111111111111".to_owned();
        quote.output_mint = request.input_mint.clone();
        quote.output_amount = 1_000u64.into();
        quote.estimated_network_fee_lamports = Some(999);
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(1u64)
        );
        for fee in [1_000, 1_001, u64::MAX] {
            quote.estimated_network_fee_lamports = Some(fee);
            assert!(matches!(
                output_after_network_fee(&quote, &StubRouter),
                Err(QuoteError::RouterRejected { .. })
            ));
        }
        quote.input_mint = request.input_mint;
        quote.output_mint = request.output_mint;
        quote.input_amount = 1_000u64.into();
        quote.output_amount = u64::MAX.into();
        quote.estimated_network_fee_lamports = Some(1);
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(u64::MAX - u64::MAX / 1_000)
        );
        quote.output_amount = 1_000u64.into();
        quote.estimated_network_fee_lamports = Some(1);
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(999u64)
        );
        quote.estimated_network_fee_lamports = Some(0);
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(1_000u64)
        );
        quote.estimated_network_fee_lamports = Some(1_000);
        assert!(matches!(
            output_after_network_fee(&quote, &StubRouter),
            Err(QuoteError::RouterRejected { .. })
        ));
        quote.estimated_network_fee_lamports = Some(1_001);
        assert!(matches!(
            output_after_network_fee(&quote, &StubRouter),
            Err(QuoteError::RouterRejected { .. })
        ));
        quote.estimated_network_fee_lamports = Some(1);
        quote.output_amount = 1u64.into();
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(1u64)
        );
        quote.input_amount = 0u64.into();
        assert!(matches!(
            output_after_network_fee(&quote, &StubRouter),
            Err(QuoteError::RouterRejected { .. })
        ));
        quote.estimated_network_fee_lamports = None;
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(1u64)
        );
        quote.input_mint = "OtherMint".to_owned();
        quote.estimated_network_fee_lamports = Some(u64::MAX);
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            RawAmount::from(1u64)
        );
    }

    #[test]
    fn zero_input_is_rejected_even_without_fee_estimate() {
        let mut request = request();
        request.input_amount = RawAmount::ZERO;
        let quote = quote_for(&request);
        assert!(matches!(
            validate_quote(&StubRouter, &request, quote),
            Err(QuoteError::RouterRejected { .. })
        ));
    }

    struct FeeRouter {
        id: &'static str,
        output: RawAmount,
        fee: u64,
    }

    #[async_trait]
    impl SwapRouter for FeeRouter {
        fn id(&self) -> &'static str {
            self.id
        }
        fn name(&self) -> &'static str {
            self.id
        }
        fn is_enabled(&self) -> bool {
            true
        }
        fn priority(&self) -> u8 {
            1
        }
        fn chain(&self) -> ChainId {
            ChainId::Solana
        }
        async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
            let mut quote = quote_for(request);
            quote.router_id = self.id.to_owned();
            quote.router_name = self.id.to_owned();
            quote.output_amount = self.output;
            quote.estimated_network_fee_lamports = Some(self.fee);
            Ok(quote)
        }
        async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> crate::Result<SwapResult> {
            Err(crate::Error::internal_error("stub"))
        }
    }

    #[tokio::test]
    async fn auto_skips_invalid_fee_quote_and_specific_validation_refuses_it() {
        crate::config::utils::install_default_config();
        let request = request();
        let invalid = Arc::new(FeeRouter {
            id: "invalid",
            output: 2_000u64.into(),
            fee: 1_000_000,
        });
        let valid = Arc::new(FeeRouter {
            id: "valid",
            output: 1_000u64.into(),
            fee: 1,
        });
        let registry = RouterRegistry::new(vec![invalid.clone(), valid]);
        let selected = best_quote_on(&registry, request.clone()).await.unwrap();
        assert_eq!(selected.router_id, "valid");
        let invalid_quote = invalid.get_quote(&request).await.unwrap();
        assert!(matches!(
            validate_quote(invalid.as_ref(), &request, invalid_quote),
            Err(QuoteError::RouterRejected { .. })
        ));
    }

    #[tokio::test]
    async fn wide_quotes_rank_by_exact_fee_net_output() {
        crate::config::utils::install_default_config();

        let mut request = request();
        request.input_amount = RawAmount::new(u128::from(u64::MAX) + 1);
        request.input_mint = "TokenMint111111111111111111111111111111111".to_owned();
        request.output_mint = "So11111111111111111111111111111111111111112".to_owned();
        let max = RawAmount::MAX;
        let registry = RouterRegistry::new(vec![
            Arc::new(FeeRouter {
                id: "gross",
                output: max,
                fee: 2,
            }),
            Arc::new(FeeRouter {
                id: "net",
                output: max.checked_sub(1u64.into()).unwrap(),
                fee: 0,
            }),
        ]);
        let selected = best_quote_on(&registry, request.clone()).await.unwrap();
        assert_eq!(selected.router_id, "net");
        assert_eq!(selected.input_amount, request.input_amount);
        assert_eq!(
            selected.output_amount,
            max.checked_sub(1u64.into()).unwrap()
        );

        let mut quote = quote_for(&request);
        quote.input_amount = request.input_amount;
        quote.output_amount = max;
        quote.minimum_output_amount = max;
        quote.estimated_network_fee_lamports = Some(1);
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            max.checked_sub(1u64.into()).unwrap()
        );

        quote.input_mint = "So11111111111111111111111111111111111111112".to_owned();
        quote.output_mint = "TokenMint111111111111111111111111111111111".to_owned();
        assert_eq!(
            output_after_network_fee(&quote, &StubRouter).unwrap(),
            max.checked_sub(u64::MAX.into()).unwrap()
        );
    }

    fn configured_deadline() -> Duration {
        crate::config::utils::install_default_config();
        crate::config::with_config(|cfg| {
            Duration::from_millis(cfg.chains.swaps(ChainId::Solana).quote_deadline_ms)
        })
    }

    /// What a [`TimedRouter`] answers once its delay has passed.
    #[derive(Clone, Copy)]
    enum Answer {
        Quote(u64),
        NoRoute,
        NotTradable,
        NotOffered(NotOfferedReason),
        Unavailable,
        TransportTimeout,
    }

    /// A router that answers after a fixed delay on the (paused) tokio clock.
    struct TimedRouter {
        id: &'static str,
        priority: u8,
        delay: Duration,
        answer: Answer,
    }

    impl TimedRouter {
        fn new(id: &'static str, priority: u8, delay: Duration, answer: Answer) -> Arc<Self> {
            Arc::new(Self {
                id,
                priority,
                delay,
                answer,
            })
        }
    }

    #[async_trait]
    impl SwapRouter for TimedRouter {
        fn id(&self) -> &'static str {
            self.id
        }
        fn name(&self) -> &'static str {
            self.id
        }
        fn is_enabled(&self) -> bool {
            true
        }
        fn priority(&self) -> u8 {
            self.priority
        }
        fn chain(&self) -> ChainId {
            ChainId::Solana
        }
        async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
            tokio::time::sleep(self.delay).await;
            let router = self.id.to_owned();
            match self.answer {
                Answer::Quote(output) => {
                    let mut quote = quote_for(request);
                    quote.router_id = router.clone();
                    quote.router_name = router;
                    quote.output_amount = output.into();
                    quote.minimum_output_amount = (output / 2).max(1).into();
                    Ok(quote)
                }
                Answer::NoRoute => Err(no_route(&router)),
                Answer::NotTradable => Err(not_tradable(&router)),
                Answer::NotOffered(reason) => Err(not_offered(&router, reason)),
                Answer::Unavailable => Err(QuoteError::Unavailable {
                    router,
                    detail: "account read failed".to_owned(),
                }),
                Answer::TransportTimeout => Err(timeout(&router)),
            }
        }
        async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> crate::Result<SwapResult> {
            Err(crate::Error::internal_error("stub"))
        }
    }

    fn registry_of(routers: Vec<Arc<TimedRouter>>) -> RouterRegistry {
        RouterRegistry::new(
            routers
                .into_iter()
                .map(|router| router as Arc<dyn SwapRouter>)
                .collect(),
        )
    }

    /// A router that has not answered `deadline` after the first valid quote is
    /// dropped from the comparison, even when it would have paid more, and the
    /// comparison returns at the deadline instead of waiting for it. A router
    /// that answers inside the deadline still competes and can win.
    #[tokio::test(start_paused = true)]
    async fn a_router_slower_than_the_deadline_is_dropped_from_the_comparison() {
        let deadline = configured_deadline();
        let first = Duration::from_millis(300);
        let registry = registry_of(vec![
            TimedRouter::new("fast", 0, first, Answer::Quote(1_000)),
            TimedRouter::new("inside", 1, first + deadline / 2, Answer::Quote(1_500)),
            TimedRouter::new("straggler", 2, first + deadline * 20, Answer::Quote(9_000)),
        ]);

        let started = tokio::time::Instant::now();
        let best = best_quote_on(&registry, request()).await.unwrap();
        assert_eq!(best.router_id, "inside");
        assert_eq!(started.elapsed(), first + deadline);

        let enabled = registry.enabled_routers_for(ChainId::Solana);
        let answers = collect_quotes(&enabled, &request(), deadline).await;
        assert!(answers[0].is_ok() && answers[1].is_ok());
        assert!(
            matches!(&answers[2], Err(QuoteError::Timeout { router }) if router == "straggler")
        );
    }

    /// The deadline is armed by the first valid quote, never by the clock alone:
    /// when every router is slower than the deadline, the first one to answer
    /// still trades, and only routers slower than that answer plus the
    /// deadline are dropped.
    #[tokio::test(start_paused = true)]
    async fn when_every_router_is_slow_the_first_answer_still_trades() {
        let deadline = configured_deadline();
        let first = deadline * 7;
        let registry = registry_of(vec![
            TimedRouter::new("slow", 0, first, Answer::Quote(1_000)),
            TimedRouter::new("slower", 1, first + deadline * 3, Answer::Quote(5_000)),
        ]);

        let started = tokio::time::Instant::now();
        let best = best_quote_on(&registry, request()).await.unwrap();
        assert_eq!(best.router_id, "slow");
        assert_eq!(started.elapsed(), first + deadline);
    }

    /// A deadline can only drop a router beside a valid quote, so it never
    /// produces a verdict about the token: a fast refusal does not arm it
    /// against a slower router that prices the trade, a straggler's own
    /// verdict is never heard, and a router that runs into its transport
    /// timeout beside a refusal leaves an operational failure, not a strike.
    #[tokio::test(start_paused = true)]
    async fn the_deadline_never_yields_a_token_verdict() {
        let deadline = configured_deadline();
        let fast = Duration::from_millis(100);
        let late = deadline * 20;

        let refused_then_priced = registry_of(vec![
            TimedRouter::new("refuses", 0, fast, Answer::NoRoute),
            TimedRouter::new("prices", 1, late, Answer::Quote(1_000)),
        ]);
        let best = best_quote_on(&refused_then_priced, request())
            .await
            .unwrap();
        assert_eq!(best.router_id, "prices");

        let priced_then_condemned = registry_of(vec![
            TimedRouter::new("prices", 0, fast, Answer::Quote(1_000)),
            TimedRouter::new("condemns", 1, late, Answer::NotTradable),
        ]);
        let enabled = priced_then_condemned.enabled_routers_for(ChainId::Solana);
        let answers = collect_quotes(&enabled, &request(), deadline).await;
        let straggler = answers[1].as_ref().unwrap_err();
        assert!(matches!(straggler, QuoteError::Timeout { .. }));
        assert!(!straggler.is_route_failure());
        assert!(best_quote_on(&priced_then_condemned, request())
            .await
            .is_ok());

        let refused_then_unreachable = registry_of(vec![
            TimedRouter::new("refuses", 0, fast, Answer::NoRoute),
            TimedRouter::new("unreachable", 1, late, Answer::TransportTimeout),
        ]);
        let failure = best_quote_on(&refused_then_unreachable, request())
            .await
            .unwrap_err();
        assert!(!failure.is_route_failure(), "got {failure}");
        assert!(failure.permanent_token_verdict().is_none());
    }

    /// A token the aggregator cannot route collects its no-route strikes while
    /// the direct router abstains from the token's pool, up to retirement; the
    /// same refusal beside a direct RPC fault collects none, because a node
    /// failure is not evidence about the token.
    #[tokio::test]
    async fn an_abstaining_direct_router_does_not_shield_a_token_from_its_strikes() {
        crate::config::utils::install_default_config();
        let abstaining = registry_of(vec![
            TimedRouter::new("jupiter", 0, Duration::ZERO, Answer::NoRoute),
            TimedRouter::new(
                "direct",
                1,
                Duration::ZERO,
                Answer::NotOffered(NotOfferedReason::UnsupportedVenue),
            ),
        ]);
        let mut opening = request();
        opening.output_mint = "AbstainStrikeMint11111111111111111111111111".to_owned();
        let mint = opening.output_mint.clone();
        NO_ROUTE_STRIKES.invalidate(&mint);

        for strike in 1..NO_ROUTE_STRIKES_BEFORE_BLACKLIST {
            assert!(opening_quote_on(&abstaining, opening.clone(), "ABST")
                .await
                .is_err());
            assert_eq!(NO_ROUTE_STRIKES.get(&mint), Some(strike));
        }
        // The last strike retires the token and clears its record.
        assert!(opening_quote_on(&abstaining, opening.clone(), "ABST")
            .await
            .is_err());
        assert_eq!(NO_ROUTE_STRIKES.get(&mint), None);

        let faulting = registry_of(vec![
            TimedRouter::new("jupiter", 0, Duration::ZERO, Answer::NoRoute),
            TimedRouter::new("direct", 1, Duration::ZERO, Answer::Unavailable),
        ]);
        let mut shielded = request();
        shielded.output_mint = "FaultNoStrikeMint11111111111111111111111111".to_owned();
        let mint = shielded.output_mint.clone();
        NO_ROUTE_STRIKES.invalidate(&mint);
        assert!(opening_quote_on(&faulting, shielded, "FALT").await.is_err());
        assert_eq!(NO_ROUTE_STRIKES.get(&mint), None);
    }

    // ------------------------------------------------------------------------
    // Fallback decided from the typed never-sent outcome
    // ------------------------------------------------------------------------

    use crate::chains::solana::swaps::direct::DirectSwapError;
    use crate::chains::solana::Error as Solana;
    use crate::swaps::{NotSubmittedReason, SwapExecutionError};

    fn not_submitted(reason: NotSubmittedReason) -> Error {
        Error::Swaps(SwapExecutionError::NotSubmitted {
            router: "Jupiter".to_owned(),
            reason,
        })
    }

    fn too_large() -> Error {
        not_submitted(NotSubmittedReason::TransactionTooLarge {
            bytes: 1329,
            limit: 1232,
        })
    }

    fn reason_name(reason: &NotSubmittedReason) -> &'static str {
        match reason {
            NotSubmittedReason::BuildUnavailable(_) => "BuildUnavailable",
            NotSubmittedReason::BuildUnusable { .. } => "BuildUnusable",
            NotSubmittedReason::TransactionTooLarge { .. } => "TransactionTooLarge",
            NotSubmittedReason::UnsupportedFormat { .. } => "UnsupportedFormat",
            NotSubmittedReason::RequestRejected { .. } => "RequestRejected",
            NotSubmittedReason::SimulationFailed { .. } => "SimulationFailed",
            NotSubmittedReason::CostExceeded { .. } => "CostExceeded",
        }
    }

    fn solana_name(error: &Solana) -> &'static str {
        match error {
            Solana::Execution(failure) => match failure {
                crate::chains::ExecutionFailure::NotFound { .. } => "Execution::NotFound",
                crate::chains::ExecutionFailure::ConfirmationTimeout { .. } => {
                    "Execution::ConfirmationTimeout"
                }
                crate::chains::ExecutionFailure::IndexingDelay { .. } => "Execution::IndexingDelay",
                crate::chains::ExecutionFailure::Reverted { .. } => "Execution::Reverted",
                crate::chains::ExecutionFailure::Expired { .. } => "Execution::Expired",
            },
            Solana::InvalidAddress { .. } => "InvalidAddress",
            Solana::InvalidKeypair { .. } => "InvalidKeypair",
            Solana::KeypairUnavailable { .. } => "KeypairUnavailable",
            Solana::SecureStorage(_) => "SecureStorage",
            Solana::Rpc { .. } => "Rpc",
            Solana::RpcFailure { .. } => "RpcFailure",
            Solana::AccountNotFound { .. } => "AccountNotFound",
            Solana::Decode { .. } => "Decode",
            Solana::InvalidPool { .. } => "InvalidPool",
            Solana::InstructionBuild { .. } => "InstructionBuild",
            Solana::DirectSwap(_) => "DirectSwap",
            Solana::NotSent(_) => "NotSent",
        }
    }

    fn direct_name(error: &DirectSwapError) -> &'static str {
        match error {
            DirectSwapError::UnsupportedVenue { .. } => "UnsupportedVenue",
            DirectSwapError::PoolUndecodable { .. } => "PoolUndecodable",
            DirectSwapError::PairNotInPool { .. } => "PairNotInPool",
            DirectSwapError::PoolNotTradable { .. } => "PoolNotTradable",
            DirectSwapError::InsufficientLiquidity { .. } => "InsufficientLiquidity",
            DirectSwapError::InvalidRequest { .. } => "InvalidRequest",
            DirectSwapError::AccountUnavailable { .. } => "AccountUnavailable",
            DirectSwapError::NodeUnavailable { .. } => "NodeUnavailable",
            DirectSwapError::QuoteMath { .. } => "QuoteMath",
            DirectSwapError::Build { .. } => "Build",
            DirectSwapError::SimulationRejected { .. } => "SimulationRejected",
            DirectSwapError::TransactionTooLarge { .. } => "TransactionTooLarge",
            DirectSwapError::SimulationUnavailable { .. } => "SimulationUnavailable",
            DirectSwapError::SubmitFailed { .. } => "SubmitFailed",
            DirectSwapError::BlockhashExpired { .. } => "BlockhashExpired",
            DirectSwapError::ConfirmationTimeout { .. } => "ConfirmationTimeout",
            DirectSwapError::TransactionFailed { .. } => "TransactionFailed",
            DirectSwapError::OutputNotReceived { .. } => "OutputNotReceived",
            DirectSwapError::InsufficientBalance { .. } => "InsufficientBalance",
            DirectSwapError::MarketMoved { .. } => "MarketMoved",
        }
    }

    /// The name a table row stands for, so coverage of every variant of every
    /// swap vocabulary can be asserted.
    fn row_name(error: &Error) -> String {
        match error {
            Error::Swaps(SwapExecutionError::NotSubmitted { reason, .. }) => {
                format!("NotSubmitted::{}", reason_name(reason))
            }
            Error::Swaps(SwapExecutionError::CompletedAmountOutOfRange { .. }) => {
                "CompletedAmountOutOfRange".to_owned()
            }
            Error::Solana(Solana::DirectSwap(direct)) => {
                format!("DirectSwap::{}", direct_name(direct))
            }
            Error::Solana(solana) => format!("Solana::{}", solana_name(solana)),
            other => format!("Other::{other}"),
        }
    }

    /// Every variant of every swap failure vocabulary, against the one question
    /// the fallback chain asks. A new variant does not compile without a name
    /// above, and the coverage assertion fails until it has a row here.
    #[test]
    fn every_swap_failure_is_classified_for_fallback_by_type() {
        let pool = crate::chains::solana::solana_sdk::pubkey::Pubkey::new_unique();
        let direct = |error: DirectSwapError| Error::Solana(Solana::DirectSwap(error));
        let sig = || "sig".to_owned();
        let rows: Vec<(Error, bool)> = vec![
            (
                not_submitted(NotSubmittedReason::BuildUnavailable(
                    crate::errors::NetworkError::RateLimited {
                        endpoint: "jupiter/swap".to_owned(),
                        retry_after_ms: None,
                    },
                )),
                true,
            ),
            (
                not_submitted(NotSubmittedReason::BuildUnusable {
                    detail: "undecodable".to_owned(),
                }),
                true,
            ),
            (too_large(), true),
            (
                not_submitted(NotSubmittedReason::UnsupportedFormat { version: 1 }),
                true,
            ),
            (
                not_submitted(NotSubmittedReason::RequestRejected {
                    detail: "-32602".to_owned(),
                }),
                true,
            ),
            (
                not_submitted(NotSubmittedReason::SimulationFailed {
                    detail: "custom 6001".to_owned(),
                }),
                true,
            ),
            (
                not_submitted(NotSubmittedReason::CostExceeded {
                    extra_raw: 13_045_440,
                    venue: "HumidiFi".to_owned(),
                    venue_address: "9H6tua7jkLhdm3w8BvgpTn5LZNU7g4ZynDmCiNN3q6Rp".to_owned(),
                }),
                true,
            ),
            (
                Error::Swaps(SwapExecutionError::CompletedAmountOutOfRange {
                    signature: sig(),
                    input_amount: 1u64.into(),
                    output_amount: 1u64.into(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::Execution(
                    crate::chains::ExecutionFailure::ConfirmationTimeout {
                        reference: sig(),
                        waited_ms: 60_000,
                    },
                )),
                false,
            ),
            (
                Error::Solana(Solana::Execution(
                    crate::chains::ExecutionFailure::NotFound { reference: sig() },
                )),
                false,
            ),
            (
                Error::Solana(Solana::Execution(
                    crate::chains::ExecutionFailure::IndexingDelay { reference: sig() },
                )),
                false,
            ),
            // Settled at `Confirmed`: the chain ran it and it failed, so it
            // moved nothing and can never land again.
            (
                Error::Solana(Solana::Execution(
                    crate::chains::ExecutionFailure::Reverted {
                        reference: sig(),
                        detail: "custom program error: 0x1771".to_owned(),
                    },
                )),
                true,
            ),
            (
                Error::Solana(Solana::Execution(
                    crate::chains::ExecutionFailure::Expired {
                        reference: sig(),
                        last_valid_block_height: 1,
                        current_block_height: 2,
                    },
                )),
                true,
            ),
            (
                Error::Solana(Solana::InvalidAddress {
                    kind: "mint",
                    value: "x".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::InvalidKeypair {
                    detail: "short".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::KeypairUnavailable {
                    detail: "locked".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::SecureStorage(
                    crate::secure_storage::Error::MachineIdentity {
                        detail: "none".to_owned(),
                    },
                )),
                false,
            ),
            (
                Error::Solana(Solana::Rpc {
                    operation: "sendTransaction",
                    detail: "reset".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::RpcFailure {
                    operation: "sendTransaction",
                    source: crate::rpc::RpcError::ProviderError {
                        code: -32602,
                        message: "too large".to_owned(),
                        data: None,
                    },
                }),
                false,
            ),
            (
                Error::Solana(Solana::AccountNotFound {
                    address: "x".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::Decode {
                    payload: "quote",
                    detail: "bad".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::InvalidPool {
                    reason: "no SOL leg".to_owned(),
                }),
                false,
            ),
            (
                Error::Solana(Solana::InstructionBuild {
                    instruction: "swap",
                    detail: "bad".to_owned(),
                }),
                false,
            ),
            // A wallet transaction refused before or at its send never
            // reached the chain.
            (
                Error::Solana(Solana::NotSent(NotSubmittedReason::RequestRejected {
                    detail: "malformed".to_owned(),
                })),
                true,
            ),
            (
                direct(DirectSwapError::UnsupportedVenue { program: pool }),
                false,
            ),
            (
                direct(DirectSwapError::PoolUndecodable {
                    pool,
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::PairNotInPool {
                    pool,
                    input_mint: pool,
                    output_mint: pool,
                }),
                false,
            ),
            (
                direct(DirectSwapError::PoolNotTradable {
                    pool,
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::InsufficientLiquidity {
                    pool,
                    amount_in: 1,
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::InvalidRequest {
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::AccountUnavailable {
                    address: pool,
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::NodeUnavailable {
                    operation: "getLatestBlockhash",
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::QuoteMath {
                    detail: String::new(),
                }),
                false,
            ),
            (
                direct(DirectSwapError::Build {
                    detail: String::new(),
                }),
                true,
            ),
            (
                direct(DirectSwapError::SimulationRejected {
                    detail: String::new(),
                    logs: vec![],
                }),
                true,
            ),
            (
                direct(DirectSwapError::TransactionTooLarge {
                    bytes: 1240,
                    limit: 1232,
                }),
                true,
            ),
            (
                direct(DirectSwapError::SimulationUnavailable {
                    detail: String::new(),
                }),
                true,
            ),
            // Only a node's refusal of the send request itself: every other
            // send outcome is settled by its signature instead.
            (
                direct(DirectSwapError::SubmitFailed {
                    detail: String::new(),
                }),
                true,
            ),
            (
                direct(DirectSwapError::BlockhashExpired {
                    signature: sig(),
                    last_valid_block_height: 1,
                    current_block_height: 2,
                }),
                true,
            ),
            (
                direct(DirectSwapError::ConfirmationTimeout {
                    signature: sig(),
                    waited_ms: 1,
                }),
                false,
            ),
            (
                direct(DirectSwapError::TransactionFailed {
                    signature: sig(),
                    detail: String::new(),
                }),
                true,
            ),
            (
                direct(DirectSwapError::OutputNotReceived {
                    signature: sig(),
                    expected_minimum: 2,
                    received: 1,
                }),
                false,
            ),
            (
                direct(DirectSwapError::InsufficientBalance {
                    mint: pool,
                    required: 2,
                    available: 1,
                }),
                false,
            ),
            (
                direct(DirectSwapError::MarketMoved {
                    pool,
                    accepted_min_net_out: 2,
                    fresh_expected_net_out: 1,
                }),
                true,
            ),
            // Outside the swap vocabularies nothing is proven, whatever the
            // RPC code says.
            (
                Error::Rpc(crate::rpc::RpcError::ProviderError {
                    code: -32602,
                    message: "base64 encoded VersionedTransaction too large".to_owned(),
                    data: None,
                }),
                false,
            ),
            (
                Error::Rpc(crate::rpc::RpcError::Network {
                    message: "reset".to_owned(),
                    is_timeout: true,
                }),
                false,
            ),
            (
                Error::Network(crate::errors::NetworkError::RequestFailed {
                    endpoint: "jupiter/swap".to_owned(),
                    detail: "reset".to_owned(),
                }),
                false,
            ),
        ];

        let covered: std::collections::BTreeSet<String> =
            rows.iter().map(|(error, _)| row_name(error)).collect();
        let reasons = 7;
        let execution_variants = 5;
        let solana_outside_direct = 11 + execution_variants;
        let direct_variants = 20;
        let swap_rows = covered
            .iter()
            .filter(|name| !name.starts_with("Other::"))
            .count();
        assert_eq!(
            swap_rows,
            reasons + 1 + solana_outside_direct + direct_variants,
            "every swap failure variant needs a row: {covered:?}"
        );

        for (error, safe) in rows {
            assert_eq!(is_fallback_safe(&error), safe, "{}", row_name(&error));
            match failed_swap(&error) {
                FailedSwap::Resendable => assert!(safe, "{}", row_name(&error)),
                FailedSwap::Reconcile { .. } | FailedSwap::Unresolved => {
                    assert!(!safe, "{}", row_name(&error))
                }
            }
        }
    }

    #[test]
    fn a_requote_must_reach_the_floor_the_trade_was_accepted_at() {
        let request = request();
        let accepted = quote_for(&request); // expects 1_000, guarantees 950
        let mut fresh = quote_for(&request);
        for (expected, holds) in [(1_200u64, true), (950, true), (949, false)] {
            fresh.output_amount = expected.into();
            assert_eq!(holds_accepted_floor(&accepted, &fresh), holds, "{expected}");
        }
    }

    /// How a [`ChainRouter`] fails or succeeds when asked to execute.
    #[derive(Clone, Copy)]
    enum Execution {
        Lands,
        TooLarge,
        Unproven,
    }

    /// A router that quotes 1_000 and executes as told, recording each call.
    struct ChainRouter {
        id: &'static str,
        priority: u8,
        execution: Execution,
        executed: Arc<std::sync::Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl SwapRouter for ChainRouter {
        fn id(&self) -> &'static str {
            self.id
        }
        fn name(&self) -> &'static str {
            self.id
        }
        fn is_enabled(&self) -> bool {
            true
        }
        fn priority(&self) -> u8 {
            self.priority
        }
        fn chain(&self) -> ChainId {
            ChainId::Solana
        }
        async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
            let mut quote = quote_for(request);
            quote.router_id = self.id.to_owned();
            quote.router_name = self.id.to_owned();
            Ok(quote)
        }
        async fn execute_swap(&self, _token: &Token, _quote: &Quote) -> crate::Result<SwapResult> {
            Err(crate::Error::internal_error("the wallet signer is used"))
        }
        async fn execute_swap_for_wallet(
            &self,
            quote: &Quote,
            _wallet_id: i64,
        ) -> crate::Result<SwapResult> {
            self.executed.lock().unwrap().push(self.id);
            match self.execution {
                Execution::Lands => Ok(SwapResult {
                    success: true,
                    router_id: self.id.to_owned(),
                    router_name: self.id.to_owned(),
                    transaction_signature: format!("sig-{}", self.id),
                    input_amount: quote.input_amount,
                    output_amount: quote.output_amount,
                    price_impact_pct: quote.price_impact_pct,
                    fee_lamports: 0,
                    execution_time_ms: 1,
                    effective_price_sol: None,
                }),
                Execution::TooLarge => Err(too_large()),
                Execution::Unproven => Err(Error::Rpc(crate::rpc::RpcError::Network {
                    message: "connection reset during sendTransaction".to_owned(),
                    is_timeout: true,
                })),
            }
        }
    }

    fn chain(primary: Execution) -> (RouterRegistry, Arc<std::sync::Mutex<Vec<&'static str>>>) {
        crate::config::utils::install_default_config();
        let executed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let router = |id, priority, execution| {
            Arc::new(ChainRouter {
                id,
                priority,
                execution,
                executed: executed.clone(),
            }) as Arc<dyn SwapRouter>
        };
        let registry = RouterRegistry::new(vec![
            router("jupiter", 0, primary),
            router("direct", 1, Execution::Lands),
        ]);
        (registry, executed)
    }

    fn primary_quote() -> Quote {
        let mut quote = quote_for(&request());
        quote.input_mint = "TokenMintIn11111111111111111111111111111111".to_owned();
        quote.router_id = "jupiter".to_owned();
        quote.router_name = "jupiter".to_owned();
        quote
    }

    /// The defect this chain exists for: the primary route's transaction is too
    /// large, which provably sent nothing, so the next router executes the
    /// trade and the result names the route that actually ran.
    #[tokio::test]
    async fn a_too_large_refusal_falls_through_to_the_next_router() {
        let (registry, executed) = chain(Execution::TooLarge);
        let (quote, result) = execute_with_fallback_on(
            &registry,
            SwapSigner::Wallet(7),
            primary_quote(),
            SwapAmountLimit::Unrestricted,
            Fallback::AnyRouter,
        )
        .await
        .expect("the fallback router executes");
        assert_eq!(*executed.lock().unwrap(), vec!["jupiter", "direct"]);
        assert_eq!(quote.router_id, "direct");
        assert_eq!(result.router_id, "direct");
        assert_eq!(result.transaction_signature, "sig-direct");
    }

    /// A failure that cannot prove it was never sent stops the chain: a second
    /// router could execute the same trade again.
    #[tokio::test]
    async fn an_unproven_send_failure_never_falls_back() {
        let (registry, executed) = chain(Execution::Unproven);
        let error = execute_with_fallback_on(
            &registry,
            SwapSigner::Wallet(7),
            primary_quote(),
            SwapAmountLimit::Unrestricted,
            Fallback::AnyRouter,
        )
        .await
        .expect_err("nothing else may execute");
        assert!(matches!(error, Error::Rpc(_)));
        assert_eq!(*executed.lock().unwrap(), vec!["jupiter"]);
    }

    /// A caller who named a router gets that router or its refusal.
    #[tokio::test]
    async fn a_named_router_is_never_re_routed() {
        let (registry, executed) = chain(Execution::TooLarge);
        let error = execute_with_fallback_on(
            &registry,
            SwapSigner::Wallet(7),
            primary_quote(),
            SwapAmountLimit::Unrestricted,
            Fallback::SameRouter,
        )
        .await
        .expect_err("only the named router was asked");
        assert!(is_fallback_safe(&error));
        assert_eq!(*executed.lock().unwrap(), vec!["jupiter"]);
    }
}
