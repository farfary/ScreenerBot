//! Raptor Router Implementation
//!
//! Raptor is Solana Tracker's aggregator: 20+ DEXes, up to four hops, no API key
//! and no published rate limit. It is quoted alongside Jupiter and the direct
//! pool engine, and the comparison layer takes whichever returns the most.
//!
//! It ships DISABLED. The hosted API is served from a beta host with no stated
//! availability guarantee, so turning it on is the owner's decision, not a
//! default.
//!
//! # The fee, and why the output convention matters
//!
//! Raptor reports `amountOut` GROSS of the platform fee when the fee is taken
//! from the output leg, and `minAmountOut` net of it. Jupiter reports BOTH net,
//! and the direct engine reports its own net figure deliberately (see
//! `direct_pool.rs`). `swaps::operations::best_quote_on` compares routers on
//! `Quote::output_amount` alone, so handing it Raptor's raw `amountOut` would
//! have let Raptor win every comparison by a platform fee that does not exist —
//! 50 bps of phantom output on every trade. [`RaptorRouter::net_output`]
//! converts to the net convention the other two routers already speak.
//!
//! Which leg the fee rides on follows `revenue::fee_reference_for_pair`, the
//! same rule the other routers use: the output side when it is SOL or USDC (a
//! sell), otherwise the input side (a buy). Raptor's `feeFromInput` expresses
//! that second case directly, which means a BUY collects its fee in SOL and the
//! lamports figure we report for it is real rather than denominated in the token
//! being bought.
//!
//! Docs: https://docs.solanatracker.io/raptor/overview

use super::http::{send_with_retry, RouterHttpFailure};
use crate::chains::solana::constants::SOL_MINT;
use crate::chains::solana::rpc::RpcClientMethods;
use crate::chains::solana::swaps::revenue::{
    fee_reference_for_mint, fee_reference_for_pair, platform_fee_amount, PLATFORM_FEE_BPS,
};
use crate::config::with_config;
use crate::errors::NetworkError;
use crate::logger::{self, LogTag};
use crate::swaps::error::{QuoteError, QuoteResult};
use crate::swaps::router::SwapRouter;
use crate::swaps::types::{Quote, QuoteRequest, SwapMode, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ============================================================================
// ENDPOINT & LIMITS
// ============================================================================

/// Raptor's hosted API. No key, no account, free during the public beta.
const RAPTOR_API_BASE: &str = "https://raptor-beta.solanatracker.io";

/// Max attempts for transient Raptor failures (HTTP 429 / 5xx / network).
const RAPTOR_MAX_ATTEMPTS: u32 = 3;

/// Ceiling on a SINGLE Raptor HTTP call (quote or swap-build).
///
/// Both happen before anything is signed, so cutting one short can only fail a
/// trade, never duplicate one. Tighter than Jupiter's because Raptor is the
/// optional router: a slow second opinion must not hold up a decision the
/// primary router could already serve.
const RAPTOR_HTTP_TIMEOUT: Duration = Duration::from_secs(10);

/// Nominal compute-unit limit used to ESTIMATE the network fee at quote time.
///
/// Raptor sizes the real limit when it builds the transaction, and the built
/// response reports the exact prioritization fee. This constant only fills the
/// quote's advisory estimate, which the trade dialog shows before a route is
/// chosen; nothing is charged against it.
const RAPTOR_ESTIMATED_COMPUTE_UNITS: u64 = 300_000;

// ============================================================================
// API TYPES
// ============================================================================

/// One leg of Raptor's route.
#[derive(Debug, Deserialize, Serialize)]
struct RaptorRouteStep {
    dex: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RaptorQuoteResponse {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "amountIn")]
    amount_in: String,
    /// GROSS of the platform fee when the fee rides on the output leg; already
    /// net when `feeFromInput` was set. Never used directly — see
    /// [`RaptorRouter::net_output`].
    #[serde(rename = "amountOut")]
    amount_out: String,
    /// Raptor's own floor, net of BOTH slippage and the platform fee. This is
    /// what the built instruction enforces on chain, so it is taken as-is.
    #[serde(rename = "minAmountOut")]
    min_amount_out: String,
    /// Percent, in the same unit as Jupiter's `priceImpactPct`.
    #[serde(rename = "priceImpact")]
    price_impact: Option<f64>,
    #[serde(rename = "routePlan", default)]
    route_plan: Vec<RaptorRouteStep>,
}

#[derive(Debug, Serialize)]
struct RaptorSwapRequest {
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "quoteResponse")]
    quote_response: serde_json::Value,
    #[serde(rename = "feeAccount", skip_serializing_if = "Option::is_none")]
    fee_account: Option<String>,
    #[serde(rename = "feeBps")]
    fee_bps: u16,
    #[serde(rename = "feeFromInput")]
    fee_from_input: bool,
    #[serde(rename = "txVersion")]
    tx_version: &'static str,
    #[serde(rename = "wrapUnwrapSol")]
    wrap_unwrap_sol: bool,
    #[serde(rename = "computeUnitPriceMicroLamports")]
    compute_unit_price_micro_lamports: u64,
}

#[derive(Debug, Deserialize)]
struct RaptorSwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
}

// ============================================================================
// FEE PLACEMENT
// ============================================================================

/// Where this pair's platform fee rides, and into which account.
///
/// Mirrors `revenue::fee_reference_for_pair`: the output leg when it is a
/// reference mint, otherwise the input leg. `from_input` is the flag Raptor
/// needs to express the second case.
#[derive(Debug, Clone, Copy)]
struct FeePlacement {
    account: Option<&'static str>,
    from_input: bool,
    bps: u16,
}

impl FeePlacement {
    fn resolve(input_mint: &str, output_mint: &str) -> Self {
        match fee_reference_for_pair(input_mint, output_mint) {
            Some(reference) => Self {
                account: Some(reference.account),
                // The output leg is preferred; falling back to the input leg is
                // exactly the case Raptor calls `feeFromInput`.
                from_input: fee_reference_for_mint(output_mint).is_none(),
                bps: PLATFORM_FEE_BPS,
            },
            // Neither leg is SOL or USDC, so there is no account that can
            // receive a fee. Charging one anyway would send it to an address
            // that cannot hold it. Our trading flow always has a reference mint
            // on one side, so this is a defensive branch, not a routine one.
            None => Self {
                account: None,
                from_input: false,
                bps: 0,
            },
        }
    }
}

// ============================================================================
// RAPTOR ROUTER
// ============================================================================

pub struct RaptorRouter {
    client: Client,
}

impl RaptorRouter {
    /// Create a new Raptor swap router instance.
    pub fn new() -> Self {
        Self {
            client: crate::net::client(),
        }
    }

    /// What the WALLET actually keeps, in the same convention Jupiter and the
    /// direct engine already report.
    ///
    /// With the fee on the output leg Raptor's `amountOut` is gross, so the fee
    /// is subtracted here. With `feeFromInput` the fee was already taken out of
    /// the input before routing, so `amountOut` is what arrives and is returned
    /// unchanged. Getting this backwards inflates the quote by the fee and wins
    /// comparisons it should lose.
    fn net_output(amount_out: u64, placement: FeePlacement) -> u64 {
        if placement.bps == 0 || placement.from_input {
            amount_out
        } else {
            amount_out.saturating_sub(platform_fee_amount(amount_out))
        }
    }

    /// The platform fee in WSOL lamports, when that is provably what it is.
    ///
    /// A sell collects on the output leg, which is lamports when the output mint
    /// is wrapped SOL. A buy collects on the INPUT leg, which is lamports when
    /// the input mint is wrapped SOL — so unlike the Jupiter path, a buy can
    /// report a real figure here instead of only a rate.
    fn platform_fee_lamports(
        input_amount: u64,
        amount_out: u64,
        input_mint: &str,
        output_mint: &str,
        placement: FeePlacement,
    ) -> Option<u64> {
        if placement.bps == 0 {
            return None;
        }
        if placement.from_input {
            (input_mint == SOL_MINT).then(|| platform_fee_amount(input_amount))
        } else {
            (output_mint == SOL_MINT).then(|| platform_fee_amount(amount_out))
        }
    }

    /// Build a route summary from Raptor's plan.
    ///
    /// An empty path is not a claim about the venue — it leaves the row hidden
    /// and lets the router name speak, matching the Jupiter adapter.
    fn build_route_plan(route_plan: &[RaptorRouteStep]) -> String {
        if route_plan.is_empty() {
            return String::new();
        }
        route_plan
            .iter()
            .map(|step| step.dex.clone().unwrap_or_else(|| "Unknown".to_owned()))
            .collect::<Vec<_>>()
            .join(" → ")
    }

    /// Raptor's price impact, refused only when it is missing or malformed.
    ///
    /// A reported `0.0` is accepted, exactly as Jupiter's is. Refusing it made
    /// identical small orders flip between quotable and refused from one second
    /// to the next, and applied a rule to one aggregator that the other is not
    /// held to. The on-chain protection is `minAmountOut`, not this figure.
    fn usable_price_impact(response: &RaptorQuoteResponse, router: &str) -> QuoteResult<f64> {
        let impact = response
            .price_impact
            .ok_or_else(|| QuoteError::RouterRejected {
                router: router.to_owned(),
                detail: "quote carried no price impact".to_owned(),
            })?;
        if !impact.is_finite() || impact < 0.0 {
            return Err(QuoteError::RouterRejected {
                router: router.to_owned(),
                detail: format!("unusable price impact {impact}"),
            });
        }
        Ok(impact)
    }

    /// Ask Raptor to build the transaction for an accepted quote.
    ///
    /// The fee placement is recomputed from the quote's own mints rather than
    /// carried across, so the built instruction can never disagree with what the
    /// quote was priced on.
    async fn build_transaction(&self, quote: &Quote, user_public_key: &str) -> Result<String> {
        let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
            .map_err(|e| Error::parse_error(format!("Quote deserialization failed: {e}")))?;
        let placement = FeePlacement::resolve(&quote.input_mint, &quote.output_mint);

        let swap_req = RaptorSwapRequest {
            user_public_key: user_public_key.to_owned(),
            quote_response,
            fee_account: placement.account.map(str::to_owned),
            fee_bps: placement.bps,
            fee_from_input: placement.from_input,
            tx_version: "V0",
            wrap_unwrap_sol: true,
            compute_unit_price_micro_lamports: with_config(|cfg| {
                cfg.swaps.raptor.priority_fee_micro_lamports
            }),
        };

        let url = format!("{RAPTOR_API_BASE}/swap");
        let response_text = send_with_retry("Raptor", "swap", RAPTOR_MAX_ATTEMPTS, || {
            self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&swap_req)
                .timeout(RAPTOR_HTTP_TIMEOUT)
        })
        .await
        .map_err(raptor_error)?;

        let swap_response: RaptorSwapResponse = serde_json::from_str(&response_text)
            .map_err(|e| Error::parse_error(format!("Raptor swap response parse failed: {e}")))?;
        Ok(swap_response.swap_transaction)
    }

    /// Check a built transaction before it is signed and sent.
    ///
    /// Raptor builds the transaction on its own host, so nothing on our side has
    /// checked its instructions or what they do with the wallet's lamports. The
    /// shared preflight in [`crate::chains::solana::swaps::cost_guard`] rejects
    /// both a transaction that would fail on chain and one that would spend SOL
    /// outside the trade — for free, before anything is signed.
    async fn preflight(transaction_base64: &str, quote: &Quote) -> Result<()> {
        crate::chains::solana::swaps::cost_guard::preflight("Raptor", transaction_base64, quote)
            .await
            .map_err(Into::into)
    }
}

impl Default for RaptorRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SwapRouter for RaptorRouter {
    fn id(&self) -> &'static str {
        "raptor"
    }

    fn name(&self) -> &'static str {
        "Raptor"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.raptor.enabled)
    }

    fn priority(&self) -> u8 {
        2 // Behind Jupiter and the direct engine: newest and on a beta host.
    }

    fn chain(&self) -> crate::chains::ChainId {
        crate::chains::ChainId::Solana
    }

    async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
        self.accept_own_chain(request)
            .map_err(|e| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: e.to_string(),
            })?;

        // Raptor prices forward from the input only. This is a refusal by THIS
        // router, never a verdict on the pair, so it must not count towards the
        // no-route strikes that retire a token.
        if request.swap_mode != SwapMode::ExactIn {
            return Err(QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: format!(
                    "{:?} is not supported; Raptor prices ExactIn only",
                    request.swap_mode
                ),
            });
        }

        // A caller that excluded a DEX excluded it from EVERY router. Raptor's
        // `dexes` parameter is a filter list whose include/exclude sense is not
        // documented, and guessing it wrong would route a graduated-token retry
        // straight back through the venue that just rejected it. Declining is
        // the only safe answer until the parameter's meaning is established.
        if request
            .exclude_dexes
            .as_ref()
            .is_some_and(|excluded| !excluded.is_empty())
        {
            return Err(QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: "cannot honour a DEX exclusion, so it declines rather than \
                         risk routing through an excluded venue"
                    .to_owned(),
            });
        }

        let slippage_bps = ((request.slippage_pct * 100.0).round() as u16).max(1);
        let placement = FeePlacement::resolve(&request.input_mint, &request.output_mint);
        let max_hops = with_config(|cfg| cfg.swaps.raptor.max_hops).clamp(1, 4);

        let mut query: Vec<(&str, String)> = vec![
            ("inputMint", request.input_mint.clone()),
            ("outputMint", request.output_mint.clone()),
            ("amount", request.input_amount.to_string()),
            ("slippageBps", slippage_bps.to_string()),
            ("maxHops", max_hops.to_string()),
            ("feeBps", placement.bps.to_string()),
            ("feeFromInput", placement.from_input.to_string()),
        ];
        if let Some(account) = placement.account {
            query.push(("feeAccount", account.to_owned()));
        }

        let url = format!("{RAPTOR_API_BASE}/quote");
        let response_text = send_with_retry("Raptor", "quote", RAPTOR_MAX_ATTEMPTS, || {
            self.client
                .get(&url)
                .query(&query)
                .timeout(RAPTOR_HTTP_TIMEOUT)
        })
        .await
        .map_err(|f| raptor_quote_error(f, self.name()))?;

        let quote_response: RaptorQuoteResponse =
            serde_json::from_str(&response_text).map_err(|e| QuoteError::Unavailable {
                router: self.name().to_owned(),
                detail: format!("quote parse failed: {e}"),
            })?;

        let amount_out =
            quote_response
                .amount_out
                .parse::<u64>()
                .map_err(|e| QuoteError::RouterRejected {
                    router: self.name().to_owned(),
                    detail: format!("invalid output amount '{}': {e}", quote_response.amount_out),
                })?;

        // Raptor's floor already accounts for slippage AND the platform fee.
        // Deriving one here would disagree with the instruction it builds.
        let minimum_output_amount = quote_response.min_amount_out.parse::<u64>().map_err(|e| {
            QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: format!(
                    "invalid minimum output '{}': {e}",
                    quote_response.min_amount_out
                ),
            }
        })?;

        let price_impact = Self::usable_price_impact(&quote_response, self.name())?;
        let output_amount = Self::net_output(amount_out, placement);
        let route_plan = Self::build_route_plan(&quote_response.route_plan);

        logger::debug(
            LogTag::Swap,
            &format!(
                "Raptor quote: {output_amount} net output ({amount_out} gross), \
                 {price_impact:.4}% impact, fee {}bps{}, route: {route_plan}",
                placement.bps,
                if placement.from_input {
                    " from input"
                } else {
                    ""
                }
            ),
        );

        // Keep the RAW response as execution_data: the /swap endpoint takes the
        // whole quote object back, so every field must survive.
        let execution_data = response_text.into_bytes();

        Ok(Quote {
            chain: request.chain,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount,
            minimum_output_amount,
            price_impact_pct: price_impact,
            platform_fee_lamports: Self::platform_fee_lamports(
                request.input_amount,
                amount_out,
                &request.input_mint,
                &request.output_mint,
                placement,
            ),
            estimated_network_fee_lamports: Some(
                crate::chains::solana::swaps::direct::compute::BASE_SIGNATURE_FEE_LAMPORTS
                    .saturating_add(
                        RAPTOR_ESTIMATED_COMPUTE_UNITS
                            .saturating_mul(with_config(|cfg| {
                                cfg.swaps.raptor.priority_fee_micro_lamports
                            }))
                            .div_ceil(1_000_000),
                    ),
            ),
            slippage_bps,
            route_plan,
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            exclude_dexes: request.exclude_dexes.clone(),
            execution_data,
        })
    }

    async fn execute_swap(&self, _token: &Token, quote: &Quote) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        let start = Instant::now();

        let transaction = self.build_transaction(quote, &quote.wallet_address).await?;
        Self::preflight(&transaction, quote).await?;

        // Propagate the send/confirm error UNCHANGED so a submitted-but-
        // unconfirmed signature stays recoverable by
        // `swaps::unconfirmed_swap_signature`.
        let signature = crate::chains::solana::rpc::get_rpc_client()
            .sign_send_and_confirm_transaction_simple(&transaction)
            .await?;

        let elapsed = start.elapsed();
        logger::info(
            LogTag::Swap,
            &format!(
                "Raptor swap executed: sig={}, time={:.2}s",
                signature,
                elapsed.as_secs_f64()
            ),
        );

        Ok(SwapResult {
            success: true,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            transaction_signature: signature.to_string(),
            input_amount: quote.input_amount,
            output_amount: quote.output_amount,
            price_impact_pct: quote.price_impact_pct,
            fee_lamports: quote.platform_fee_lamports.unwrap_or(0),
            execution_time_ms: elapsed.as_millis() as u64,
            effective_price_sol: None,
        })
    }

    async fn execute_swap_for_wallet(&self, quote: &Quote, wallet_id: i64) -> Result<SwapResult> {
        use crate::chains::solana::solana_sdk::signer::Signer;

        self.accept_own_quote(quote)?;
        let start = Instant::now();

        let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
        let transaction = self
            .build_transaction(quote, &keypair.pubkey().to_string())
            .await?;
        Self::preflight(&transaction, quote).await?;
        let signature = crate::chains::solana::rpc::get_rpc_client()
            .sign_send_and_confirm_with_keypair(&transaction, &keypair)
            .await?;

        Ok(SwapResult {
            success: true,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            transaction_signature: signature.to_string(),
            input_amount: quote.input_amount,
            output_amount: quote.output_amount,
            price_impact_pct: quote.price_impact_pct,
            fee_lamports: quote.platform_fee_lamports.unwrap_or(0),
            execution_time_ms: start.elapsed().as_millis() as u64,
            effective_price_sol: None,
        })
    }
}

/// Fold a failed Raptor call into the crate error channel.
fn raptor_error(failure: RouterHttpFailure) -> Error {
    match failure.status {
        Some(429) => Error::Network(NetworkError::RateLimited {
            endpoint: format!("raptor/{}", failure.label),
            retry_after_ms: failure.retry_after.map(|d| d.as_millis() as u64),
        }),
        Some(status) => Error::Network(NetworkError::HttpStatus {
            endpoint: format!("raptor/{}", failure.label),
            status,
            body: Some(failure.body),
        }),
        None => Error::Network(NetworkError::RequestFailed {
            endpoint: format!("raptor/{}", failure.label),
            detail: failure.body,
        }),
    }
}

/// Classify a failed Raptor call into the quote vocabulary.
///
/// This is the one place Raptor's wire format becomes our vocabulary. The rule
/// that matters is the same as everywhere else: only a verdict about the TOKEN
/// may become `NotTradable`/`NoRoute`, because those are what retire a mint. A
/// 4xx we do not recognise is a request WE got wrong and stays `Unavailable`.
fn raptor_quote_error(failure: RouterHttpFailure, router: &str) -> QuoteError {
    let router = router.to_owned();
    match failure.status {
        Some(429) => QuoteError::RateLimited {
            router,
            retry_after: failure.retry_after,
        },
        Some(status) if (400..500).contains(&status) => {
            let body_lower = failure.body.to_lowercase();
            if body_lower.contains("not tradable") || body_lower.contains("not tradeable") {
                QuoteError::NotTradable {
                    router,
                    detail: failure.body,
                }
            } else if body_lower.contains("no route")
                || body_lower.contains("no routes")
                || body_lower.contains("could not find any route")
                || body_lower.contains("insufficient liquidity")
            {
                QuoteError::NoRoute {
                    router,
                    detail: failure.body,
                }
            } else {
                QuoteError::Unavailable {
                    router,
                    detail: format!("HTTP {status}: {}", failure.body),
                }
            }
        }
        Some(status) => QuoteError::Unavailable {
            router,
            detail: format!("HTTP {status}: {}", failure.body),
        },
        None if failure.timed_out => QuoteError::Timeout { router },
        None => QuoteError::Unavailable {
            router,
            detail: failure.body,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::solana::constants::USDC_MINT;
    use crate::chains::solana::swaps::revenue::{FEE_TOKEN_ACCOUNT_USDC, FEE_TOKEN_ACCOUNT_WSOL};

    const TOKEN: &str = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump";

    #[test]
    fn the_router_identifies_itself_and_never_outranks_jupiter() {
        let router = RaptorRouter::new();
        assert_eq!(router.id(), "raptor");
        assert_eq!(router.name(), "Raptor");
        assert_eq!(router.chain(), crate::chains::ChainId::Solana);
        assert!(router.priority() > 0, "Jupiter stays the primary router");
    }

    /// A sell collects on the output leg; a buy falls back to the input leg,
    /// which is exactly what `feeFromInput` expresses.
    #[test]
    fn fee_placement_follows_the_shared_reference_rule() {
        let sell = FeePlacement::resolve(TOKEN, SOL_MINT);
        assert_eq!(sell.account, Some(FEE_TOKEN_ACCOUNT_WSOL));
        assert!(!sell.from_input, "a sell collects on the SOL output");
        assert_eq!(sell.bps, PLATFORM_FEE_BPS);

        let buy = FeePlacement::resolve(SOL_MINT, TOKEN);
        assert_eq!(buy.account, Some(FEE_TOKEN_ACCOUNT_WSOL));
        assert!(buy.from_input, "a buy has no collectable output leg");

        // Both legs are reference mints: the output side wins, as in revenue.rs.
        let both = FeePlacement::resolve(SOL_MINT, USDC_MINT);
        assert_eq!(both.account, Some(FEE_TOKEN_ACCOUNT_USDC));
        assert!(!both.from_input);

        // Neither leg can receive a fee, so none is charged rather than sent
        // somewhere it cannot be held.
        let unpayable =
            FeePlacement::resolve(TOKEN, "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm");
        assert_eq!(unpayable.account, None);
        assert_eq!(unpayable.bps, 0);
    }

    /// The regression this guards: Raptor reports `amountOut` gross when the fee
    /// rides on the output, while Jupiter and the direct engine both report net.
    /// Comparing a gross figure against those would win every route by 50 bps of
    /// output that the wallet never receives.
    #[test]
    fn a_gross_output_is_converted_to_the_net_convention_the_other_routers_use() {
        let sell = FeePlacement::resolve(TOKEN, SOL_MINT);
        // 1 SOL gross out, 0.5% fee taken from that output leg.
        assert_eq!(RaptorRouter::net_output(1_000_000_000, sell), 995_000_000);

        // With the fee taken from the input, `amountOut` is ALREADY what the
        // wallet keeps and must not be reduced a second time.
        let buy = FeePlacement::resolve(SOL_MINT, TOKEN);
        assert_eq!(RaptorRouter::net_output(1_000_000_000, buy), 1_000_000_000);

        // No fee charged, nothing to subtract.
        let unpayable =
            FeePlacement::resolve(TOKEN, "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm");
        assert_eq!(
            RaptorRouter::net_output(1_000_000_000, unpayable),
            1_000_000_000
        );
    }

    /// A buy's fee is taken from SOL, so unlike the Jupiter path it yields a
    /// real lamports figure instead of only a rate.
    #[test]
    fn the_fee_is_reported_in_lamports_on_both_legs_when_sol_is_the_paying_side() {
        let sell = FeePlacement::resolve(TOKEN, SOL_MINT);
        assert_eq!(
            RaptorRouter::platform_fee_lamports(500, 1_000_000_000, TOKEN, SOL_MINT, sell),
            Some(5_000_000),
            "a sell collects 0.5% of the SOL output"
        );

        let buy = FeePlacement::resolve(SOL_MINT, TOKEN);
        assert_eq!(
            RaptorRouter::platform_fee_lamports(1_000_000_000, 42, SOL_MINT, TOKEN, buy),
            Some(5_000_000),
            "a buy collects 0.5% of the SOL input"
        );

        // USDC pays the fee, so there is no honest lamports figure to report.
        let usdc_sell = FeePlacement::resolve(TOKEN, USDC_MINT);
        assert_eq!(
            RaptorRouter::platform_fee_lamports(500, 1_000_000, TOKEN, USDC_MINT, usdc_sell),
            None
        );
    }

    /// A missing or malformed impact is refused; a reported zero is accepted,
    /// the same as Jupiter's, so identical small orders stop flapping.
    #[test]
    fn a_malformed_price_impact_is_refused_and_zero_is_accepted() {
        let quote = |impact: Option<f64>| RaptorQuoteResponse {
            input_mint: SOL_MINT.to_owned(),
            output_mint: TOKEN.to_owned(),
            amount_in: "1000".to_owned(),
            amount_out: "1000".to_owned(),
            min_amount_out: "990".to_owned(),
            price_impact: impact,
            route_plan: vec![],
        };

        assert_eq!(
            RaptorRouter::usable_price_impact(&quote(Some(0.0)), "Raptor").unwrap(),
            0.0
        );

        for bad in [Some(-1.0), Some(f64::NAN), None] {
            assert!(
                matches!(
                    RaptorRouter::usable_price_impact(&quote(bad), "Raptor"),
                    Err(QuoteError::RouterRejected { .. })
                ),
                "price impact {bad:?} must not be traded on"
            );
        }

        // A real, small impact is fine — Raptor reports fractions of a basis
        // point for small orders rather than a hard zero.
        assert_eq!(
            RaptorRouter::usable_price_impact(&quote(Some(0.0017)), "Raptor").unwrap(),
            0.0017
        );
    }

    /// A quote response carries its own floor, and that floor is already net of
    /// both slippage and the fee. Reconstructing it would disagree with the
    /// transaction Raptor builds.
    #[test]
    fn a_quote_response_decodes_its_floor_and_route() {
        let json = r#"{
            "inputMint": "So11111111111111111111111111111111111111112",
            "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "amountIn": "1000000000",
            "amountOut": "101927853",
            "minAmountOut": "100911121",
            "feeAmount": "100001",
            "priceImpact": 0.0017859352247640843,
            "routePlan": [{"dex": "Raydium CP", "pool": "x", "percent": 100}]
        }"#;
        let parsed: RaptorQuoteResponse =
            serde_json::from_str(json).expect("well-formed Raptor quote JSON must decode");
        assert_eq!(parsed.amount_out.parse::<u64>().unwrap(), 101_927_853);
        assert_eq!(parsed.min_amount_out.parse::<u64>().unwrap(), 100_911_121);
        assert_eq!(
            RaptorRouter::build_route_plan(&parsed.route_plan),
            "Raydium CP"
        );

        // An empty plan states nothing rather than naming a venue.
        assert_eq!(RaptorRouter::build_route_plan(&[]), "");
    }

    #[test]
    fn quote_response_rejects_malformed_json() {
        assert!(serde_json::from_str::<RaptorQuoteResponse>("{\"not\": \"a quote\"}").is_err());
    }

    /// Only a verdict about the token may retire a mint; our own bad request
    /// must stay router-level.
    #[test]
    fn only_a_token_verdict_is_classified_as_one() {
        let failure = |status: u16, body: &str| RouterHttpFailure {
            label: "quote".to_owned(),
            status: Some(status),
            body: body.to_owned(),
            retry_after: None,
            timed_out: false,
        };

        assert!(matches!(
            raptor_quote_error(failure(400, "token is not tradable"), "Raptor"),
            QuoteError::NotTradable { .. }
        ));
        assert!(matches!(
            raptor_quote_error(failure(400, "no route found"), "Raptor"),
            QuoteError::NoRoute { .. }
        ));
        assert!(matches!(
            raptor_quote_error(failure(400, "maxHops must be between 1 and 4"), "Raptor"),
            QuoteError::Unavailable { .. }
        ));
        assert!(matches!(
            raptor_quote_error(failure(503, "upstream down"), "Raptor"),
            QuoteError::Unavailable { .. }
        ));
        assert!(matches!(
            raptor_quote_error(failure(429, "slow down"), "Raptor"),
            QuoteError::RateLimited { .. }
        ));
        assert!(matches!(
            raptor_quote_error(
                RouterHttpFailure {
                    label: "quote".to_owned(),
                    status: None,
                    body: "timed out".to_owned(),
                    retry_after: None,
                    timed_out: true,
                },
                "Raptor"
            ),
            QuoteError::Timeout { .. }
        ));
    }
}
