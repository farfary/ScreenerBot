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

use crate::chains::solana::constants::{SOL_MINT, USDC_MINT};
use crate::chains::solana::rpc::RpcClientMethods;
use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::swaps::router::SwapRouter;
use crate::swaps::types::{Quote, QuoteRequest, SwapMode, SwapResult};
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

/// Get the Jupiter API key from config, if set.
/// Returns None if empty/unset.
fn get_api_key() -> Option<String> {
    let key = with_config(|cfg| cfg.swaps.jupiter.api_key.clone());
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

/// Get the correct Jupiter API base URL.
/// Uses api.jup.ag when API key is configured, lite-api.jup.ag as free fallback.
fn get_api_base() -> &'static str {
    let key = with_config(|cfg| cfg.swaps.jupiter.api_key.clone());
    if key.is_empty() {
        JUPITER_API_BASE_FREE
    } else {
        JUPITER_API_BASE
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
    #[serde(rename = "priceImpactPct")]
    price_impact_pct: String,
    #[serde(rename = "routePlan")]
    route_plan: Vec<RoutePlanStep>,
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
        rename = "prioritizationFeeLamports",
        skip_serializing_if = "Option::is_none"
    )]
    prioritization_fee_lamports: Option<u64>,
    #[serde(rename = "platformFeeBps", skip_serializing_if = "Option::is_none")]
    platform_fee_bps: Option<u16>,
    #[serde(rename = "feeAccount", skip_serializing_if = "Option::is_none")]
    fee_account: Option<String>,
}

#[derive(Debug, Deserialize)]
struct JupiterSwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
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

/// Compute a backoff delay. Honors a server `Retry-After` (seconds) when present,
/// otherwise exponential (~0.4s, 0.8s, 1.6s, 3.2s) capped at 4s, plus small jitter.
fn jupiter_backoff_delay(attempt: u32, retry_after_secs: Option<u64>) -> Duration {
    if let Some(secs) = retry_after_secs {
        return Duration::from_millis(secs.clamp(1, 5) * 1000);
    }
    let exp = 400u64.saturating_mul(1u64 << attempt.saturating_sub(1).min(4));
    let capped = exp.min(4000);
    let jitter = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.subsec_nanos() as u64) % 200)
        .unwrap_or(0);
    Duration::from_millis(capped + jitter)
}

/// Send a Jupiter HTTP request with retry+backoff on transient failures.
/// `build` is invoked fresh per attempt (a RequestBuilder is single-use).
/// Returns the success body text on 2xx, or a domain Error. Retries only on
/// HTTP 429, 5xx, and network/transport errors — never on 4xx (e.g. 400 = no
/// route), which are deterministic and surfaced immediately to the caller.
async fn jupiter_send_with_retry<F>(label: &str, build: F) -> Result<String>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        match build().send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return resp.text().await.map_err(|e| {
                        Error::network_error(format!(
                            "Failed to read Jupiter {label} response: {e}"
                        ))
                    });
                }
                let is_transient = status.as_u16() == 429 || status.is_server_error();
                let retry_after = resp
                    .headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.trim().parse::<u64>().ok());
                let body = resp.text().await.unwrap_or_else(|_| "Unknown".to_owned());
                if is_transient && attempt < JUPITER_MAX_ATTEMPTS {
                    let delay = jupiter_backoff_delay(attempt, retry_after);
                    logger::warning(
                        LogTag::Swap,
                        &format!(
                            "Jupiter {label} transient {} (attempt {}/{}), retrying in {}ms",
                            status,
                            attempt,
                            JUPITER_MAX_ATTEMPTS,
                            delay.as_millis()
                        ),
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(Error::api_error(format!(
                    "Jupiter {label} failed ({status}): {body}"
                )));
            }
            Err(e) => {
                if attempt < JUPITER_MAX_ATTEMPTS {
                    let delay = jupiter_backoff_delay(attempt, None);
                    logger::warning(
                        LogTag::Swap,
                        &format!(
                            "Jupiter {label} network error (attempt {}/{}): {} - retrying in {}ms",
                            attempt,
                            JUPITER_MAX_ATTEMPTS,
                            e,
                            delay.as_millis()
                        ),
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
                return Err(Error::network_error(format!(
                    "Jupiter {label} request failed after {attempt} attempts: {e}"
                )));
            }
        }
    }
}

/// Get the referral token account for a swap based on input or output mint
/// Since we always trade against SOL or USDC, one side will always match
/// Fee is taken from the output side, but Jupiter handles routing internally
fn get_referral_token_account_for_swap(input_mint: &str, output_mint: &str) -> Option<String> {
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

/// Resolve the referral fee account (WSOL/USDC) for a swap pair. We always trade
/// against SOL/USDC, so the fee is taken on that (standard SPL) side — works for
/// Token2022 tokens too when the quote uses `instructionVersion=V2`. Shared by the
/// main router AND the multi-wallet tool executor so BOTH collect referral revenue.
/// Docs: developers.jup.ag/docs/swap/add-fees-to-swap
pub(crate) fn referral_fee_account(input_mint: &str, output_mint: &str) -> Option<String> {
    get_referral_token_account_for_swap(input_mint, output_mint)
}

/// Build, sign and submit a Jupiter swap transaction with a caller-supplied
/// keypair (rather than the main wallet). Used by the multi-wallet tool
/// executor, which trades from wallets other than the configured main wallet
/// but must still collect the same referral fee via `referral_fee_account`.
pub(crate) async fn execute_with_keypair(
    quote: &Quote,
    keypair: &crate::chains::solana::solana_sdk::signature::Keypair,
) -> std::result::Result<String, String> {
    use crate::chains::solana::solana_sdk::signer::Signer;

    let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
        .map_err(|e| format!("Quote deserialization failed: {e}"))?;

    let fee_account = referral_fee_account(&quote.input_mint, &quote.output_mint);

    let swap_req = JupiterSwapRequest {
        user_public_key: keypair.pubkey().to_string(),
        quote_response,
        dynamic_compute_unit_limit: Some(true),
        prioritization_fee_lamports: Some(with_config(|cfg| {
            cfg.swaps.jupiter.default_priority_fee
        })),
        platform_fee_bps: None, // Already set in quote request
        fee_account,
    };

    // Mark a swap in flight so background Jupiter pollers defer to it.
    let _swap_guard = crate::apis::jupiter::throttle::swap_guard();

    let api_base = get_api_base();
    let url = format!("{api_base}/swap/v1/swap");
    let api_key = get_api_key();
    let client = crate::net::client();
    let response_text = jupiter_send_with_retry("swap", || {
        let mut req = client.post(&url);
        if let Some(ref key) = api_key {
            req = req.header("x-api-key", key.clone());
        }
        req.header("Content-Type", "application/json")
            .json(&swap_req)
            .timeout(JUPITER_HTTP_TIMEOUT)
    })
    .await
    .map_err(|e| e.to_string())?;

    let swap_response: JupiterSwapResponse = serde_json::from_str(&response_text)
        .map_err(|e| format!("Jupiter swap response parse failed: {e}"))?;

    let rpc_client = crate::chains::solana::rpc::get_rpc_client();
    let signature = rpc_client
        .sign_send_and_confirm_with_keypair(&swap_response.swap_transaction, keypair)
        .await
        .map_err(|e| format!("Transaction failed: {e}"))?;

    Ok(signature.to_string())
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

    /// Build route plan summary from Jupiter response
    fn build_route_plan(route_plan: &[RoutePlanStep]) -> String {
        if route_plan.is_empty() {
            return "Direct".to_owned();
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
        with_config(|cfg| cfg.swaps.jupiter.enabled)
    }

    fn priority(&self) -> u8 {
        0 // Highest priority (primary router)
    }

    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote> {
        // Mark a swap as in flight so background Jupiter pollers (price, token
        // discovery, health) defer and don't steal the shared rate budget.
        let _swap_guard = crate::apis::jupiter::throttle::swap_guard();

        let slippage_bps = ((request.slippage_pct * 100.0).round() as u16).max(1);

        // Always collect the referral platform fee — this is the project's revenue.
        // `instructionVersion=V2` (set below) lets Jupiter collect fees even on
        // Token2022 tokens (the fee is taken on the SOL/USDC side, which is always
        // standard SPL), so we no longer skip fees for Token2022.
        let quote_req = JupiterQuoteRequest {
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            amount: request.input_amount.to_string(),
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
                request.input_amount,
                request.input_mint,
                request.output_mint,
                slippage_bps,
                REFERRAL_FEE_BPS
            ),
        );

        // Send quote request (with API key header if configured), retrying on
        // transient rate-limit / network failures. Raw response text is kept so
        // ALL fields are preserved for the later swap request.
        let api_base = get_api_base();
        let url = format!("{api_base}/swap/v1/quote");
        let api_key = get_api_key();
        let response_text = jupiter_send_with_retry("quote", || {
            let mut req = self.client.get(&url);
            if let Some(ref key) = api_key {
                req = req.header("x-api-key", key.clone());
            }
            req.query(&quote_req).timeout(JUPITER_HTTP_TIMEOUT)
        })
        .await?;

        // Parse into our limited struct just to extract key values
        let quote_response: JupiterQuoteResponse = serde_json::from_str(&response_text)
            .map_err(|e| Error::parse_error(format!("Jupiter quote parse failed: {e}")))?;

        let output_amount = quote_response
            .out_amount
            .parse::<u64>()
            .map_err(|e| Error::parse_error(format!("Invalid output amount: {e}")))?;

        let price_impact = quote_response
            .price_impact_pct
            .parse::<f64>()
            .unwrap_or_else(|_| {
                logger::warning(
                    LogTag::Swap,
                    &format!(
                        "Jupiter: Failed to parse price_impact_pct '{}', defaulting to 0.0",
                        quote_response.price_impact_pct
                    ),
                );
                0.0
            });

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
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount,
            price_impact_pct: price_impact,
            fee_lamports: 0, // Fee taken from output via referral system
            slippage_bps,
            route_plan,
            swap_mode: request.swap_mode,
            wallet_address: request.wallet_address.clone(),
            execution_data,
        })
    }

    async fn execute_swap(&self, _token: &Token, quote: &Quote) -> Result<SwapResult> {
        // Keep background Jupiter pollers deferred while the swap transaction is
        // being built (see throttle module).
        let _swap_guard = crate::apis::jupiter::throttle::swap_guard();

        let start = Instant::now();

        // Deserialize quote response
        let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
            .map_err(|e| Error::internal_error(format!("Quote deserialization failed: {e}")))?;

        // Resolve the referral fee account (WSOL/USDC side; works for Token2022 too
        // because the quote uses instructionVersion=V2).
        let fee_account = referral_fee_account(&quote.input_mint, &quote.output_mint);

        let swap_req = JupiterSwapRequest {
            user_public_key: quote.wallet_address.clone(),
            quote_response,
            dynamic_compute_unit_limit: Some(with_config(|cfg| {
                cfg.swaps.jupiter.dynamic_compute_unit_limit
            })),
            prioritization_fee_lamports: Some(with_config(|cfg| {
                cfg.swaps.jupiter.default_priority_fee
            })),
            platform_fee_bps: None, // Already set in quote request
            fee_account: fee_account.clone(),
        };

        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter swap request: user={}, feeAccount={}",
                swap_req.user_public_key,
                fee_account.as_deref().unwrap_or("none")
            ),
        );

        // Get swap transaction (with API key header if configured), retrying on
        // transient failures. This only BUILDS an unsigned transaction, so retry
        // is safe — on-chain submission happens afterwards via RPC.
        let api_base = get_api_base();
        let url = format!("{api_base}/swap/v1/swap");
        let api_key = get_api_key();
        let response_text = jupiter_send_with_retry("swap", || {
            let mut req = self.client.post(&url);
            if let Some(ref key) = api_key {
                req = req.header("x-api-key", key.clone());
            }
            req.header("Content-Type", "application/json")
                .json(&swap_req)
                .timeout(JUPITER_HTTP_TIMEOUT)
        })
        .await?;

        let swap_response: JupiterSwapResponse = serde_json::from_str(&response_text)
            .map_err(|e| Error::parse_error(format!("Jupiter swap response parse failed: {e}")))?;

        // Transaction is already base64 encoded, send it directly
        let rpc_client = crate::chains::solana::rpc::get_rpc_client();
        // Propagate the send/confirm error UNCHANGED. Wrapping it used to prepend
        // "Transaction send failed: ", which corrupted the submitted-but-unconfirmed
        // marker that `swaps::unconfirmed_swap_signature` reads out of the message —
        // the caller then stored a garbage string as the exit signature and
        // verification could never reconcile the sell that actually landed.
        let signature = rpc_client
            .sign_send_and_confirm_transaction_simple(&swap_response.swap_transaction)
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
            fee_lamports: 0,
            execution_time_ms: elapsed.as_millis() as u64,
            effective_price_sol: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn referral_account_prefers_the_output_mint_then_falls_back_to_input() {
        let other_mint = "SomeRandomTokenMintAddress11111111111111111";

        // Output is SOL/USDC: fee taken from output.
        assert_eq!(
            get_referral_token_account_for_swap(other_mint, SOL_MINT),
            Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_owned())
        );
        assert_eq!(
            get_referral_token_account_for_swap(other_mint, USDC_MINT),
            Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_owned())
        );
        // Output is a token, but input is SOL/USDC: fee taken from input side.
        assert_eq!(
            get_referral_token_account_for_swap(SOL_MINT, other_mint),
            Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_owned())
        );
        assert_eq!(
            get_referral_token_account_for_swap(USDC_MINT, other_mint),
            Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_owned())
        );
        // Neither leg is SOL/USDC: no fee account, matches "shouldn't happen" path.
        assert_eq!(
            get_referral_token_account_for_swap(other_mint, "AnotherTokenMint111111111111"),
            None
        );
    }

    #[test]
    fn backoff_delay_honors_retry_after_and_otherwise_grows_exponentially_with_a_cap() {
        assert_eq!(
            jupiter_backoff_delay(1, Some(2)),
            Duration::from_millis(2000)
        );
        // Clamped into [1, 5] seconds even for an extreme server value.
        assert_eq!(
            jupiter_backoff_delay(1, Some(999)),
            Duration::from_millis(5000)
        );

        let d1 = jupiter_backoff_delay(1, None);
        let d2 = jupiter_backoff_delay(2, None);
        let d4 = jupiter_backoff_delay(4, None);
        assert!(
            d1 <= d2 && d2 <= d4,
            "delay must not shrink as attempts grow"
        );
        assert!(
            d4 <= Duration::from_millis(4000 + 200),
            "delay is capped near 4s plus jitter, got {d4:?}"
        );
    }

    #[test]
    fn quote_response_json_decodes_and_bad_price_impact_is_recoverable() {
        let json = r#"{
            "inputMint": "So11111111111111111111111111111111111111112",
            "inAmount": "1000000000",
            "outputMint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "outAmount": "123456789",
            "priceImpactPct": "0.42",
            "routePlan": [
                {"swapInfo": {"ammKey": "Amm11111111111111111111111111111111111111", "label": "Raydium"}}
            ]
        }"#;
        let parsed: JupiterQuoteResponse =
            serde_json::from_str(json).expect("well-formed Jupiter quote JSON must decode");
        assert_eq!(parsed.out_amount.parse::<u64>().unwrap(), 123_456_789);
        assert_eq!(parsed.price_impact_pct.parse::<f64>().unwrap(), 0.42);
        assert_eq!(parsed.route_plan.len(), 1);
        assert_eq!(
            parsed.route_plan[0].swap_info.label.as_deref(),
            Some("Raydium")
        );

        // A malformed priceImpactPct must not fail the whole quote — callers
        // fall back to 0.0 rather than propagate the parse error.
        let bad_impact = json.replace("\"0.42\"", "\"not-a-number\"");
        let parsed: JupiterQuoteResponse = serde_json::from_str(&bad_impact).unwrap();
        assert_eq!(parsed.price_impact_pct.parse::<f64>().unwrap_or(0.0), 0.0);
    }

    #[test]
    fn quote_response_rejects_malformed_json() {
        let result: std::result::Result<JupiterQuoteResponse, _> =
            serde_json::from_str("{\"not\": \"a valid quote\"}");
        assert!(result.is_err());
    }
}
