// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Jupiter Router Implementation
//!
//! Referral fees (0.5%) are the project's revenue and work WITHOUT an API key:
//! by default we hit the free `lite-api.jup.ag`. An API key is optional and only
//! raises rate limits (switches to `api.jup.ag`) — it does NOT change fees.
//!
//! The free endpoint rate-limits aggressively (HTTP 429), so quote/swap requests
//! retry with exponential backoff (see `jupiter_send_with_retry`) instead of
//! failing the trade. The `/swap` call only BUILDS an unsigned transaction, so
//! retrying it is safe (on-chain submission happens later via RPC).
//! Docs: https://developers.jup.ag/docs/swap/add-fees-to-swap

use super::http::RouterHttpFailure;
use crate::chains::solana::constants::{SOL_MINT, USDC_MINT};
use crate::chains::solana::solana_sdk::signature::{Keypair, Signer};
use crate::config::with_config;
use crate::errors::NetworkError;
use crate::logger::{self, LogTag};
use crate::swaps::error::{NotSubmittedReason, QuoteError, QuoteResult, SwapExecutionError};
use crate::swaps::router::SwapRouter;
use crate::swaps::types::{Quote, QuoteRequest, SwapResult};
use crate::tokens::Token;
use crate::{Error, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ============================================================================
// CREDENTIALS & FEE CONSTANTS
// API key: user-configurable (rate limiting only, from portal.jup.ag)
// Fee params: HARDCODED - NOT configurable (revenue source)
// ============================================================================

/// Jupiter API base URLs
/// api.jup.ag requires API key (free key from portal.jup.ag, or paid tier)
/// lite-api.jup.ag works without API key (free fallback, deprecation date TBA)
const JUPITER_API_BASE: &str = "https://api.jup.ag";
const JUPITER_API_BASE_FREE: &str = "https://lite-api.jup.ag";

/// The Jupiter host and API key for one request, read from config once: the
/// keyed host when an API key is set, the free host otherwise.
fn api_endpoint() -> (&'static str, Option<String>) {
    let key = with_config(|cfg| cfg.chains.solana.swaps.jupiter.api_key.clone());
    if key.is_empty() {
        (JUPITER_API_BASE_FREE, None)
    } else {
        (JUPITER_API_BASE, Some(key))
    }
}

/// HARDCODED REFERRAL FEE: 0.5% (50 basis points)
/// This fee is MANDATORY and CANNOT be changed by users
/// Revenue share: 80% to ScreenerBot, 20% to Jupiter
const REFERRAL_FEE_BPS: u16 = 50;

/// Referral token accounts for fee collection (must be initialized token accounts)
/// These receive fees based on the output mint of the swap
const REFERRAL_TOKEN_ACCOUNT_WSOL: &str = "9yiZThTzanryu3mg1VVu6Qy4HiqKhydCAUqcasLHPxWB";
const REFERRAL_TOKEN_ACCOUNT_USDC: &str = "3kmcF3DFGFRKXeC5v5AMzwpsdj2Uc3Z7a5KrojtWv2GW";

// ============================================================================
// API TYPES
// ============================================================================

#[derive(Debug, Serialize)]
struct JupiterQuoteRequest {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    amount: String,
    #[serde(rename = "slippageBps")]
    slippage_bps: u16,
    #[serde(rename = "swapMode", skip_serializing_if = "Option::is_none")]
    swap_mode: Option<String>,
    /// Platform fee in basis points - applied to output amount
    #[serde(rename = "platformFeeBps", skip_serializing_if = "Option::is_none")]
    platform_fee_bps: Option<u16>,
    /// Comma-separated list of DEX labels to exclude from routing
    #[serde(rename = "excludeDexes", skip_serializing_if = "Option::is_none")]
    exclude_dexes: Option<String>,
    /// Instruction version. "V2" is REQUIRED to collect the platform/referral fee
    /// on Token2022 tokens (otherwise the swap fails with IncorrectTokenProgramID
    /// 0x177e). It's a safe superset for standard SPL too, so we always send it.
    #[serde(rename = "instructionVersion", skip_serializing_if = "Option::is_none")]
    instruction_version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JupiterQuoteResponse {
    #[serde(rename = "inputMint")]
    input_mint: String,
    #[serde(rename = "inAmount")]
    in_amount: String,
    #[serde(rename = "outputMint")]
    output_mint: String,
    #[serde(rename = "outAmount")]
    out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    other_amount_threshold: String,
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    /// Present only because the quote request sets `platformFeeBps`. Jupiter
    /// sizes it on the leg its own routing takes the fee from, which is not
    /// necessarily the leg our `feeAccount` collects -- see
    /// [`JupiterRouter::platform_fee_lamports`].
    #[serde(rename = "platformFee")]
    platform_fee: Option<JupiterPlatformFee>,
    #[serde(rename = "routePlan")]
    route_plan: Vec<RoutePlanStep>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JupiterPlatformFee {
    amount: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RoutePlanStep {
    #[serde(rename = "swapInfo")]
    swap_info: SwapInfo,
}

#[derive(Debug, Deserialize, Serialize)]
struct SwapInfo {
    #[serde(rename = "ammKey")]
    amm_key: String,
    label: Option<String>,
}

/// Nominal compute units used to ESTIMATE a Jupiter swap's priority fee before
/// the transaction is built. Jupiter sizes the real limit when
/// `dynamicComputeUnitLimit` is on; this only fills the advisory estimate the
/// router comparison and the paper simulator use.
const JUPITER_ESTIMATED_COMPUTE_UNITS: u64 = 300_000;

impl JupiterRouter {
    /// The priority fee a Jupiter swap is expected to pay, in lamports.
    pub fn estimated_priority_fee_lamports() -> u64 {
        JUPITER_ESTIMATED_COMPUTE_UNITS
            .saturating_mul(with_config(|cfg| {
                cfg.chains.solana.swaps.jupiter.priority_fee_micro_lamports
            }))
            .div_ceil(1_000_000)
    }

    /// One signature plus the expected priority fee.
    pub fn estimated_network_fee_lamports() -> u64 {
        crate::chains::solana::swaps::direct::compute::BASE_SIGNATURE_FEE_LAMPORTS
            .saturating_add(Self::estimated_priority_fee_lamports())
    }
}

#[derive(Debug, Serialize)]
struct JupiterSwapRequest {
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "quoteResponse")]
    quote_response: serde_json::Value,
    #[serde(
        rename = "dynamicComputeUnitLimit",
        skip_serializing_if = "Option::is_none"
    )]
    dynamic_compute_unit_limit: Option<bool>,
    #[serde(
        rename = "computeUnitPriceMicroLamports",
        skip_serializing_if = "Option::is_none"
    )]
    compute_unit_price_micro_lamports: Option<u64>,
    #[serde(rename = "platformFeeBps", skip_serializing_if = "Option::is_none")]
    platform_fee_bps: Option<u16>,
    #[serde(rename = "feeAccount", skip_serializing_if = "Option::is_none")]
    fee_account: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
    /// The last block the transaction's blockhash is valid for. Lets the
    /// settle step prove a transaction that never landed dead instead of only
    /// timing out on it.
    #[serde(rename = "lastValidBlockHeight", default)]
    last_valid_block_height: Option<u64>,
    /// The compute-unit limit Jupiter wrote into the transaction. It falls back
    /// to the 1,400,000-unit ceiling when its own build simulation fails, which
    /// is why the pre-send gate tightens it from our own measurement.
    #[serde(rename = "computeUnitLimit", default)]
    compute_unit_limit: Option<u32>,
    /// The prioritization fee that limit costs at the requested unit price.
    #[serde(rename = "prioritizationFeeLamports", default)]
    prioritization_fee_lamports: Option<u64>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Max attempts for transient Jupiter API failures (HTTP 429 / 5xx / network).
/// The free lite-api.jup.ag has tight rate limits, so we retry with backoff
/// rather than failing the trade outright.
const JUPITER_MAX_ATTEMPTS: u32 = 4;

/// Ceiling on a SINGLE Jupiter HTTP call (quote or swap-build).
///
/// Both calls happen BEFORE anything is signed or submitted, so cutting one short
/// is always safe — the worst case is a failed trade, never a duplicate one. The
/// bound is deliberately tighter than the shared default in `crate::net`: a swap
/// is a live, price-sensitive decision, and a quote that takes longer than this is
/// already stale. Without it a stalled socket parked the whole sell indefinitely.
const JUPITER_HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// Send a Jupiter HTTP request over the shared router transport.
async fn jupiter_send_with_retry<F>(
    label: &str,
    build: F,
) -> std::result::Result<String, RouterHttpFailure>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    super::http::send_with_retry("Jupiter", label, JUPITER_MAX_ATTEMPTS, build).await
}

/// The referral fee account (WSOL/USDC) for a swap pair. We always trade
/// against SOL or USDC, so one side always matches; the output side is
/// preferred because the fee is taken from the output. The fee rides that
/// standard SPL side, which works for Token2022 tokens too when the quote uses
/// `instructionVersion=V2`.
/// Docs: developers.jup.ag/docs/swap/add-fees-to-swap
fn referral_fee_account(input_mint: &str, output_mint: &str) -> Option<String> {
    // Check output mint first (preferred - fee taken from output)
    if output_mint == SOL_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_string());
    }
    if output_mint == USDC_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_string());
    }

    // Check input mint (for buy swaps where output is a token)
    if input_mint == SOL_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_string());
    }
    if input_mint == USDC_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_string());
    }

    // Neither side is SOL/USDC (shouldn't happen in our trading flow)
    None
}

/// How long a fetched program-id-to-label map is trusted. Venues are added to
/// the aggregator over time, so the map cannot be a one-shot constant, but it
/// changes far too rarely to fetch per swap.
const VENUE_LABEL_TTL: Duration = Duration::from_secs(6 * 60 * 60);

/// Jupiter's own program-id-to-label map, fetched lazily and cached.
static VENUE_LABELS: std::sync::LazyLock<
    tokio::sync::RwLock<Option<(Instant, std::collections::HashMap<String, String>)>>,
> = std::sync::LazyLock::new(|| tokio::sync::RwLock::new(None));

/// The label `excludeDexes` uses for the venue running at `program_id`.
///
/// Exclusion speaks the aggregator's vocabulary, not ours, and the vocabulary
/// is the aggregator's to define — so it is ASKED for, never hardcoded. That is
/// what lets a refusal name a venue this code has never heard of (any venue the
/// route picked) and still be answerable by a re-quote that avoids it.
/// A map we cannot fetch simply yields `None`: the caller then falls back to
/// another router instead of retrying blind.
pub(crate) async fn venue_label_for_program(program_id: &str) -> Option<String> {
    if let Some((fetched, labels)) = VENUE_LABELS.read().await.as_ref() {
        if fetched.elapsed() < VENUE_LABEL_TTL {
            return labels.get(program_id).cloned();
        }
    }

    let (api_base, api_key) = api_endpoint();
    let url = format!("{api_base}/swap/v1/program-id-to-label");
    let mut request = crate::net::client().get(&url).timeout(JUPITER_HTTP_TIMEOUT);
    if let Some(key) = api_key {
        request = request.header("x-api-key", key);
    }
    let labels: std::collections::HashMap<String, String> = match request.send().await {
        Ok(response) => match response.json().await {
            Ok(labels) => labels,
            Err(e) => {
                logger::warning(
                    LogTag::Swap,
                    &format!("Jupiter venue label map could not be read: {e}"),
                );
                return None;
            }
        },
        Err(e) => {
            logger::warning(
                LogTag::Swap,
                &format!("Jupiter venue label map unavailable: {e}"),
            );
            return None;
        }
    };

    let label = labels.get(program_id).cloned();
    *VENUE_LABELS.write().await = Some((Instant::now(), labels));
    label
}

// ============================================================================
// JUPITER ROUTER
// ============================================================================

pub struct JupiterRouter {
    client: Client,
}

impl JupiterRouter {
    /// Create a new Jupiter swap router instance
    pub fn new() -> Self {
        Self {
            client: crate::net::client(),
        }
    }

    /// The platform fee in WSOL lamports, when that is what it provably is.
    ///
    /// The quote sizes the fee on the OUTPUT leg for an ExactIn swap, so the
    /// number is lamports exactly when the output mint is wrapped SOL -- a sell.
    /// On a buy the same field is denominated in the token being bought while
    /// our fee account collects SOL, and no honest conversion exists here, so
    /// the dialog is told the rate and not a fabricated amount.
    fn platform_fee_lamports(response: &JupiterQuoteResponse) -> Option<u64> {
        if response.output_mint != SOL_MINT {
            return None;
        }
        response
            .platform_fee
            .as_ref()
            .and_then(|fee| fee.amount.parse::<u64>().ok())
    }

    /// Build route plan summary from Jupiter response
    fn build_route_plan(route_plan: &[RoutePlanStep]) -> String {
        // An empty path is not a claim about the venue. Naming it "Direct" read
        // as our own Direct Pool router in the trade dialog; an empty string
        // simply hides the path row and leaves the router name to speak.
        if route_plan.is_empty() {
            return String::new();
        }

        let labels: Vec<String> = route_plan
            .iter()
            .map(|step| {
                step.swap_info
                    .label
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_owned())
            })
            .collect();

        labels.join(" → ")
    }
}

#[async_trait]
impl SwapRouter for JupiterRouter {
    fn id(&self) -> &'static str {
        "jupiter"
    }

    fn name(&self) -> &'static str {
        "Jupiter"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.chains.solana.swaps.jupiter.enabled)
    }

    fn priority(&self) -> u8 {
        0 // Highest priority (primary router)
    }

    fn chain(&self) -> crate::chains::ChainId {
        crate::chains::ChainId::Solana
    }

    async fn get_quote(&self, request: &QuoteRequest) -> QuoteResult<Quote> {
        let input_amount =
            u64::try_from(request.input_amount).map_err(|_| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: "input amount exceeds the Solana u64 limit".to_owned(),
            })?;
        self.accept_own_chain(request)
            .map_err(|e| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: e.to_string(),
            })?;
        // Mark a swap as in flight so background Jupiter pollers (price, token
        // discovery, health) defer and don't steal the shared rate budget.
        let _swap_guard = crate::chains::solana::apis::jupiter::throttle::swap_guard();

        let slippage_bps = ((request.slippage_pct * 100.0).round() as u16).max(1);

        // Always collect the referral platform fee — this is the project's revenue.
        // `instructionVersion=V2` (set below) lets Jupiter collect fees even on
        // Token2022 tokens (the fee is taken on the SOL/USDC side, which is always
        // standard SPL), so we no longer skip fees for Token2022.
        let quote_req = JupiterQuoteRequest {
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            amount: input_amount.to_string(),
            slippage_bps,
            swap_mode: Some(request.swap_mode.as_str().to_owned()),
            platform_fee_bps: Some(REFERRAL_FEE_BPS),
            exclude_dexes: request.exclude_dexes.as_ref().map(|d| d.join(",")),
            instruction_version: Some("V2".to_owned()),
        };

        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter quote request: {} {} → {} (slippage: {}bps, fee: {}bps, V2)",
                input_amount,
                request.input_mint,
                request.output_mint,
                slippage_bps,
                REFERRAL_FEE_BPS
            ),
        );

        // Send quote request (with API key header if configured), retrying on
        // transient rate-limit / network failures. Raw response text is kept so
        // ALL fields are preserved for the later swap request.
        let (api_base, api_key) = api_endpoint();
        let url = format!("{api_base}/swap/v1/quote");
        let response_text = jupiter_send_with_retry("quote", || {
            let mut req = self.client.get(&url);
            if let Some(ref key) = api_key {
                req = req.header("x-api-key", key.clone());
            }
            req.query(&quote_req).timeout(JUPITER_HTTP_TIMEOUT)
        })
        .await
        .map_err(|failure| {
            let router = self.name().to_owned();
            match failure.status {
                Some(429) => QuoteError::RateLimited {
                    router,
                    retry_after: failure.retry_after,
                },
                Some(status) if (400..500).contains(&status) => {
                    // Jupiter reports the reason as a stable machine `errorCode`;
                    // fall back to the raw body only when the shape is unexpected.
                    let code = serde_json::from_str::<serde_json::Value>(&failure.body)
                        .ok()
                        .and_then(|v| {
                            v.get("errorCode")
                                .and_then(|c| c.as_str())
                                .map(str::to_owned)
                        })
                        .unwrap_or_default()
                        .to_ascii_uppercase();
                    let body_lower = failure.body.to_lowercase();

                    if code == "TOKEN_NOT_TRADABLE" || body_lower.contains("not tradable") {
                        QuoteError::NotTradable {
                            router,
                            detail: failure.body,
                        }
                    } else if code == "COULD_NOT_FIND_ANY_ROUTE"
                        || body_lower.contains("could not find any route")
                        || body_lower.contains("no route")
                        || body_lower.contains("no routes")
                    {
                        QuoteError::NoRoute {
                            router,
                            detail: failure.body,
                        }
                    } else {
                        // A 4xx we do not recognise is a request WE got wrong, not
                        // a verdict on the token — it must never retire one.
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
        })?;

        // Parse into our limited struct just to extract key values
        let quote_response: JupiterQuoteResponse =
            serde_json::from_str(&response_text).map_err(|e| QuoteError::Unavailable {
                router: self.name().to_owned(),
                detail: format!("quote parse failed: {e}"),
            })?;

        let output_amount =
            quote_response
                .out_amount
                .parse::<u64>()
                .map_err(|e| QuoteError::RouterRejected {
                    router: self.name().to_owned(),
                    detail: format!("invalid output amount '{}': {e}", quote_response.out_amount),
                })?;

        // Price impact drives the confirm-time gate and the dialog's warning, so
        // an unparseable value must fail the quote. It used to default to 0.0,
        // which showed the user a perfect fill and traded on it.
        let price_impact = quote_response
            .price_impact_pct
            .parse::<f64>()
            .ok()
            .filter(|impact| impact.is_finite() && *impact >= 0.0)
            .ok_or_else(|| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: format!(
                    "unusable price impact '{}'",
                    quote_response.price_impact_pct
                ),
            })?;

        // Jupiter's own floor, after slippage and its fee. Deriving one from
        // `outAmount` here would disagree with the transaction it builds.
        let minimum_output_amount = quote_response
            .other_amount_threshold
            .parse::<u64>()
            .map_err(|e| QuoteError::RouterRejected {
                router: self.name().to_owned(),
                detail: format!(
                    "invalid otherAmountThreshold '{}': {e}",
                    quote_response.other_amount_threshold
                ),
            })?;

        let route_plan = Self::build_route_plan(&quote_response.route_plan);

        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter quote: {} output, {:.4}% impact, route: {}",
                output_amount, price_impact, route_plan
            ),
        );

        // CRITICAL: Store the raw JSON response as execution_data
        // This preserves ALL fields (inputMint, outputMint, etc.) that the swap endpoint needs
        let execution_data = response_text.into_bytes();

        Ok(Quote {
            chain: request.chain,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: input_amount.into(),
            output_amount: output_amount.into(),
            minimum_output_amount: minimum_output_amount.into(),
            price_impact_pct: price_impact,
            platform_fee_lamports: Self::platform_fee_lamports(&quote_response),
            // One signature plus the prioritization fee this router asks for at
            // swap-build time -- the two components the wallet actually pays.
            estimated_network_fee_lamports: Some(Self::estimated_network_fee_lamports()),
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
        super::checked_quote_amounts(quote)?;
        let keypair = crate::chains::solana::accounts::configured_keypair()?;
        self.build_and_submit(quote, &keypair).await
    }

    async fn execute_swap_for_wallet(&self, quote: &Quote, wallet_id: i64) -> Result<SwapResult> {
        self.accept_own_quote(quote)?;
        super::checked_quote_amounts(quote)?;
        let keypair = crate::chains::solana::accounts::keypair_for_wallet(wallet_id).await?;
        self.build_and_submit(quote, &keypair).await
    }
}

impl JupiterRouter {
    /// Ask Jupiter to build the transaction for `quote`, addressed to `signer`,
    /// and hand it to the pre-send gate, which measures, simulates, signs,
    /// sends and confirms it. One path for the main wallet and the wallet
    /// tools, both collecting the referral fee.
    async fn build_and_submit(&self, quote: &Quote, signer: &Keypair) -> Result<SwapResult> {
        let start = Instant::now();
        let not_submitted = |reason| {
            Error::Swaps(SwapExecutionError::NotSubmitted {
                router: self.name().to_owned(),
                reason,
            })
        };

        let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
            .map_err(|e| {
                not_submitted(NotSubmittedReason::BuildUnusable {
                    detail: format!("the stored quote does not decode: {e}"),
                })
            })?;
        let fee_account = referral_fee_account(&quote.input_mint, &quote.output_mint);
        let swap_req = JupiterSwapRequest {
            user_public_key: signer.pubkey().to_string(),
            quote_response,
            dynamic_compute_unit_limit: Some(with_config(|cfg| {
                cfg.chains.solana.swaps.jupiter.dynamic_compute_unit_limit
            })),
            compute_unit_price_micro_lamports: Some(with_config(|cfg| {
                cfg.chains.solana.swaps.jupiter.priority_fee_micro_lamports
            })),
            platform_fee_bps: None, // Already set in quote request
            fee_account,
        };

        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter swap request: user={}, feeAccount={}",
                swap_req.user_public_key,
                swap_req.fee_account.as_deref().unwrap_or("none")
            ),
        );

        // Keep background Jupiter pollers deferred while the swap transaction is
        // being built (see throttle module).
        let _swap_guard = crate::chains::solana::apis::jupiter::throttle::swap_guard();

        // This only BUILDS an unsigned transaction, so retrying transient
        // failures is safe; submission happens afterwards through the gate.
        let (api_base, api_key) = api_endpoint();
        let url = format!("{api_base}/swap/v1/swap");
        let response_text = jupiter_send_with_retry("swap", || {
            let mut req = self.client.post(&url);
            if let Some(ref key) = api_key {
                req = req.header("x-api-key", key.clone());
            }
            req.header("Content-Type", "application/json")
                .json(&swap_req)
                .timeout(JUPITER_HTTP_TIMEOUT)
        })
        .await
        .map_err(|failure| {
            let endpoint = format!("jupiter/{}", failure.label);
            not_submitted(NotSubmittedReason::BuildUnavailable(match failure.status {
                Some(429) => NetworkError::RateLimited {
                    endpoint,
                    retry_after_ms: failure.retry_after.map(|d| d.as_millis() as u64),
                },
                Some(status) => NetworkError::HttpStatus {
                    endpoint,
                    status,
                    body: Some(failure.body),
                },
                None => NetworkError::RequestFailed {
                    endpoint,
                    detail: failure.body,
                },
            }))
        })?;

        let swap_response: JupiterSwapResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                not_submitted(NotSubmittedReason::BuildUnusable {
                    detail: format!("the swap response does not decode: {e}"),
                })
            })?;
        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter build: compute limit {:?} units, prioritization fee {:?} lamports",
                swap_response.compute_unit_limit, swap_response.prioritization_fee_lamports
            ),
        );

        // The send/confirm error is propagated UNCHANGED: a submitted but
        // unconfirmed swap carries its signature as a typed outcome, and
        // wrapping it would hide that from `swaps::unconfirmed_swap_signature`.
        let signature = crate::chains::solana::swaps::presend::submit_built_swap(
            self.name(),
            &swap_response.swap_transaction,
            swap_response.last_valid_block_height,
            quote,
            signer,
        )
        .await?;

        let elapsed = start.elapsed();
        logger::info(
            LogTag::Swap,
            &format!(
                "Jupiter swap executed: sig={}, time={:.2}s",
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
            // Only when the quote could establish it in lamports; a buy's fee is
            // collected in SOL but reported by Jupiter in the bought token.
            fee_lamports: quote.platform_fee_lamports.unwrap_or(0),
            execution_time_ms: elapsed.as_millis() as u64,
            effective_price_sol: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn oversized_input_is_rejected_before_router_io() {
        let request = QuoteRequest {
            chain: crate::chains::ChainId::Solana,
            input_mint: crate::chains::solana::constants::SOL_MINT.to_owned(),
            output_mint: crate::chains::solana::constants::USDC_MINT.to_owned(),
            input_amount: crate::chains::RawAmount::new(u128::from(u64::MAX) + 1),
            wallet_address: String::new(),
            slippage_pct: 1.0,
            swap_mode: crate::swaps::types::SwapMode::ExactIn,
            exclude_dexes: None,
        };
        let error = JupiterRouter::new().get_quote(&request).await.unwrap_err();
        assert!(matches!(error, QuoteError::RouterRejected { .. }));
        assert!(error.to_string().contains("Solana u64 limit"));
    }

    #[test]
    fn referral_account_prefers_the_output_mint_then_falls_back_to_input() {
        let other_mint = "SomeRandomTokenMintAddress11111111111111111";

        // Output is SOL/USDC: fee taken from output.
        assert_eq!(
            referral_fee_account(other_mint, SOL_MINT),
            Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_owned())
        );
        assert_eq!(
            referral_fee_account(other_mint, USDC_MINT),
            Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_owned())
        );
        // Output is a token, but input is SOL/USDC: fee taken from input side.
        assert_eq!(
            referral_fee_account(SOL_MINT, other_mint),
            Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_owned())
        );
        assert_eq!(
            referral_fee_account(USDC_MINT, other_mint),
            Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_owned())
        );
        // Neither leg is SOL/USDC: no fee account, matches "shouldn't happen" path.
        assert_eq!(
            referral_fee_account(other_mint, "AnotherTokenMint111111111111"),
            None
        );
    }

    /// `otherAmountThreshold` and `platformFee` are read off the same response
    /// as the output amount: the threshold is the floor Jupiter's own swap
    /// instruction enforces, and reconstructing either in our code would
    /// disagree with the transaction it builds.
    #[test]
    fn a_quote_response_carries_its_own_floor_and_fee() {
        let json = r#"{
            "inputMint": "So11111111111111111111111111111111111111112",
            "inAmount": "1000000000",
            "outputMint": "So11111111111111111111111111111111111111112",
            "outAmount": "123456789",
            "otherAmountThreshold": "122222222",
            "priceImpactPct": "0.42",
            "platformFee": {"amount": "617283"},
            "routePlan": [
                {"swapInfo": {"ammKey": "Amm11111111111111111111111111111111111111", "label": "Raydium"}}
            ]
        }"#;
        let parsed: JupiterQuoteResponse =
            serde_json::from_str(json).expect("well-formed Jupiter quote JSON must decode");
        assert_eq!(parsed.out_amount.parse::<u64>().unwrap(), 123_456_789);
        assert_eq!(
            parsed.other_amount_threshold.parse::<u64>().unwrap(),
            122_222_222
        );
        assert_eq!(parsed.price_impact_pct.parse::<f64>().unwrap(), 0.42);
        assert_eq!(parsed.route_plan.len(), 1);
        assert_eq!(
            parsed.route_plan[0].swap_info.label.as_deref(),
            Some("Raydium")
        );

        // The fee amount is lamports only when the output leg IS wrapped SOL.
        assert_eq!(
            JupiterRouter::platform_fee_lamports(&parsed),
            Some(617_283),
            "a sell collects the fee in SOL, so the amount is reportable"
        );

        // A buy prices the same field in the token being bought while our fee
        // account collects SOL: there is no honest lamports figure to report.
        let buy = json.replace(
            r#""outputMint": "So11111111111111111111111111111111111111112""#,
            r#""outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v""#,
        );
        let parsed: JupiterQuoteResponse = serde_json::from_str(&buy).unwrap();
        assert_eq!(JupiterRouter::platform_fee_lamports(&parsed), None);

        // A malformed priceImpactPct is not recoverable: it used to default to
        // 0.0, which showed a perfect fill and then traded on it.
        let bad_impact = json.replace("\"0.42\"", "\"not-a-number\"");
        let parsed: JupiterQuoteResponse = serde_json::from_str(&bad_impact).unwrap();
        assert!(parsed.price_impact_pct.parse::<f64>().is_err());
    }

    /// `/swap` reports the blockhash validity, limit and prioritization fee it
    /// built in; all are read, and a response without them still decodes.
    #[test]
    fn a_swap_response_keeps_jupiters_own_build_report() {
        let parsed: JupiterSwapResponse = serde_json::from_str(
            r#"{
                "swapTransaction": "AQAB",
                "lastValidBlockHeight": 1,
                "computeUnitLimit": 1400000,
                "prioritizationFeeLamports": 70000,
                "simulationError": {"errorCode": "FAILED_TO_SIMULATE_SWAP"}
            }"#,
        )
        .expect("a full /swap response decodes");
        assert_eq!(parsed.last_valid_block_height, Some(1));
        assert_eq!(parsed.compute_unit_limit, Some(1_400_000));
        assert_eq!(parsed.prioritization_fee_lamports, Some(70_000));

        let bare: JupiterSwapResponse =
            serde_json::from_str(r#"{"swapTransaction": "AQAB"}"#).expect("decodes");
        assert_eq!(bare.last_valid_block_height, None);
        assert_eq!(bare.compute_unit_limit, None);
        assert_eq!(bare.prioritization_fee_lamports, None);
    }

    #[test]
    fn quote_response_rejects_malformed_json() {
        let result: std::result::Result<JupiterQuoteResponse, _> =
            serde_json::from_str("{\"not\": \"a valid quote\"}");
        assert!(result.is_err());
    }
}
