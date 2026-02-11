/// Jupiter Router Implementation
/// Uses api.jup.ag with referral fees for revenue and optional user API key for rate limits
use crate::config::with_config;
use crate::errors::ScreenerBotError;
use crate::logger::{self, LogTag};
use crate::rpc::RpcClientMethods;
use crate::swaps::router::{Quote, QuoteRequest, SwapMode, SwapResult, SwapRouter};
use crate::tokens::decimals::is_token_2022;
use crate::tokens::Token;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

// ============================================================================
// CREDENTIALS & FEE CONSTANTS
// API key: user-configurable (rate limiting only, from portal.jup.ag)
// Fee params: HARDCODED - NOT configurable (revenue source)
// ============================================================================

/// Jupiter API base URL (NEW - migrated from lite-api.jup.ag)
const JUPITER_API_BASE: &str = "https://api.jup.ag";

/// Default Jupiter API key used when user hasn't configured their own
/// User can set their own key in config for higher rate limits (portal.jup.ag)
/// This does NOT affect fee collection — fees use platformFeeBps + feeAccount
/// NOTE: This placeholder will cause API failures if config is not set - this is intentional
const DEFAULT_JUPITER_API_KEY: &str = "YOUR_JUPITER_API_KEY";

/// Get the Jupiter API key (from config or default)
fn get_api_key() -> String {
    let key = with_config(|cfg| cfg.swaps.jupiter.api_key.clone());
    if key.is_empty() {
        DEFAULT_JUPITER_API_KEY.to_string()
    } else {
        key
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

/// WSOL and USDC mint addresses for fee account selection
const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

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

/// Get the referral token account for a swap based on input or output mint
/// Since we always trade against SOL or USDC, one side will always match
/// Fee is taken from the output side, but Jupiter handles routing internally
fn get_referral_token_account_for_swap(input_mint: &str, output_mint: &str) -> Option<String> {
    // Check output mint first (preferred - fee taken from output)
    if output_mint == WSOL_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_string());
    }
    if output_mint == USDC_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_string());
    }

    // Check input mint (for buy swaps where output is a token)
    if input_mint == WSOL_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_WSOL.to_string());
    }
    if input_mint == USDC_MINT {
        return Some(REFERRAL_TOKEN_ACCOUNT_USDC.to_string());
    }

    // Neither side is SOL/USDC (shouldn't happen in our trading flow)
    None
}

// ============================================================================
// JUPITER ROUTER
// ============================================================================

pub struct JupiterRouter {
    client: Client,
}

impl JupiterRouter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Build route plan summary from Jupiter response
    fn build_route_plan(route_plan: &[RoutePlanStep]) -> String {
        if route_plan.is_empty() {
            return "Direct".to_string();
        }

        let labels: Vec<String> = route_plan
            .iter()
            .map(|step| {
                step.swap_info
                    .label
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string())
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

    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError> {
        let slippage_bps = ((request.slippage_pct * 100.0).round() as u16).max(1);

        // Check if either token is Token2022 - Jupiter cannot collect fees on Token2022
        // Error 0x177e (6014) = IncorrectTokenProgramID when trying to collect fees
        // TODO: Monitor Jupiter API updates for Token2022 fee support in the future
        let input_is_token_2022 = is_token_2022(&request.input_mint).await;
        let output_is_token_2022 = is_token_2022(&request.output_mint).await;
        let skip_fees = input_is_token_2022 || output_is_token_2022;

        let platform_fee_bps = if skip_fees {
            logger::info(
                LogTag::Swap,
                &format!(
                    "Skipping Jupiter platform fee for Token2022 swap: input_2022={}, output_2022={}, input={}, output={}",
                    input_is_token_2022, output_is_token_2022, request.input_mint, request.output_mint
                ),
            );
            None
        } else {
            Some(REFERRAL_FEE_BPS)
        };

        let quote_req = JupiterQuoteRequest {
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            amount: request.input_amount.to_string(),
            slippage_bps,
            swap_mode: Some(request.swap_mode.as_str().to_string()),
            platform_fee_bps,
        };

        logger::debug(
            LogTag::Swap,
            &format!(
                "Jupiter quote request: {} {} → {} (slippage: {}bps, fee: {}bps)",
                request.input_amount,
                request.input_mint,
                request.output_mint,
                slippage_bps,
                platform_fee_bps.unwrap_or(0)
            ),
        );

        // Send quote request with API key
        let url = format!("{}/swap/v1/quote", JUPITER_API_BASE);
        let response = self
            .client
            .get(&url)
            .header("x-api-key", get_api_key())
            .query(&quote_req)
            .send()
            .await
            .map_err(|e| {
                ScreenerBotError::network_error(format!("Jupiter quote request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown".to_string());
            return Err(ScreenerBotError::api_error(format!(
                "Jupiter quote failed ({}): {}",
                status, error_text
            )));
        }

        // Get raw response text first - we need to preserve ALL fields for the swap request
        let response_text = response.text().await.map_err(|e| {
            ScreenerBotError::network_error(format!("Failed to read Jupiter response: {}", e))
        })?;

        // Parse into our limited struct just to extract key values
        let quote_response: JupiterQuoteResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                ScreenerBotError::parse_error(format!("Jupiter quote parse failed: {}", e))
            })?;

        let output_amount = quote_response
            .out_amount
            .parse::<u64>()
            .map_err(|e| ScreenerBotError::parse_error(format!("Invalid output amount: {}", e)))?;

        let price_impact = quote_response
            .price_impact_pct
            .parse::<f64>()
            .unwrap_or_else(|_| {
                logger::warning(
                    LogTag::Swap,
                    &format!("Jupiter: Failed to parse price_impact_pct '{}', defaulting to 0.0", quote_response.price_impact_pct),
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

    async fn execute_swap(
        &self,
        _token: &Token,
        quote: &Quote,
    ) -> Result<SwapResult, ScreenerBotError> {
        let start = Instant::now();

        // Deserialize quote response
        let quote_response: serde_json::Value = serde_json::from_slice(&quote.execution_data)
            .map_err(|e| {
                ScreenerBotError::internal_error(format!("Quote deserialization failed: {}", e))
            })?;

        // Check if either token is Token2022 - Jupiter cannot collect fees on Token2022
        // Skip fee account for Token2022 tokens to avoid IncorrectTokenProgramID error
        // TODO: Optimize by passing Token2022 status from quote to avoid duplicate RPC calls
        let input_is_token_2022 = is_token_2022(&quote.input_mint).await;
        let output_is_token_2022 = is_token_2022(&quote.output_mint).await;
        let skip_fees = input_is_token_2022 || output_is_token_2022;

        // Get the referral token account - check both input and output mints
        // Since we always trade against SOL or USDC, one side will always match
        // Skip fee account for Token2022 tokens to avoid IncorrectTokenProgramID error
        let fee_account = if skip_fees {
            logger::debug(
                LogTag::Swap,
                &format!(
                    "Skipping feeAccount for Token2022 swap: input={}, output={}",
                    quote.input_mint, quote.output_mint
                ),
            );
            None
        } else {
            get_referral_token_account_for_swap(&quote.input_mint, &quote.output_mint)
        };

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

        // Get swap transaction
        let url = format!("{}/swap/v1/swap", JUPITER_API_BASE);
        let response = self
            .client
            .post(&url)
            .header("x-api-key", get_api_key())
            .header("Content-Type", "application/json")
            .json(&swap_req)
            .send()
            .await
            .map_err(|e| {
                ScreenerBotError::network_error(format!("Jupiter swap request failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown".to_string());
            return Err(ScreenerBotError::api_error(format!(
                "Jupiter swap failed ({}): {}",
                status, error_text
            )));
        }

        let swap_response: JupiterSwapResponse = response.json().await.map_err(|e| {
            ScreenerBotError::parse_error(format!("Jupiter swap response parse failed: {}", e))
        })?;

        // Transaction is already base64 encoded, send it directly
        let rpc_client = crate::rpc::get_rpc_client();
        let signature = rpc_client
            .sign_send_and_confirm_transaction_simple(&swap_response.swap_transaction)
            .await
            .map_err(|e| {
                ScreenerBotError::network_error(format!("Transaction send failed: {}", e))
            })?;

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
