/// GMGN Router - Self-contained implementation with direct API integration
use crate::config::with_config;
use crate::constants::SOL_MINT;
use crate::errors::ScreenerBotError;
use crate::logger::{self, LogTag};
use crate::rpc::RpcClientMethods;
use crate::swaps::router::{Quote, QuoteRequest, SwapResult, SwapRouter};
use crate::swaps::types::deserialize_optional_string_or_number;
use crate::tokens::Token;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;

// ============================================================================
// GMGN-SPECIFIC TYPES
// ============================================================================

/// Quote information from GMGN API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapQuote {
    #[serde(rename = "inputMint")]
    pub input_mint: String,
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outputMint")]
    pub output_mint: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "otherAmountThreshold")]
    pub other_amount_threshold: String,
    #[serde(rename = "inDecimals")]
    pub in_decimals: u8,
    #[serde(rename = "outDecimals")]
    pub out_decimals: u8,
    #[serde(rename = "swapMode")]
    pub swap_mode: String,
    #[serde(rename = "slippageBps")]
    pub slippage_bps: String,
    #[serde(rename = "platformFee")]
    pub platform_fee: Option<String>,
    #[serde(rename = "priceImpactPct")]
    pub price_impact_pct: String,
    #[serde(rename = "routePlan")]
    pub route_plan: serde_json::Value,
    #[serde(rename = "contextSlot")]
    pub context_slot: Option<u64>,
    #[serde(rename = "timeTaken")]
    pub time_taken: f64,
}

/// Raw transaction data from GMGN API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RawTransaction {
    #[serde(rename = "swapTransaction")]
    pub swap_transaction: String,
    #[serde(rename = "lastValidBlockHeight")]
    pub last_valid_block_height: u64,
    #[serde(rename = "prioritizationFeeLamports")]
    pub prioritization_fee_lamports: u64,
    #[serde(rename = "recentBlockhash")]
    pub recent_blockhash: String,
    pub version: Option<String>,
}

/// Complete swap response data from GMGN API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SwapData {
    pub quote: SwapQuote,
    pub raw_tx: RawTransaction,
    pub amount_in_usd: Option<String>,
    pub amount_out_usd: Option<String>,
    pub jito_order_id: Option<String>,
    #[serde(deserialize_with = "deserialize_optional_string_or_number")]
    pub sol_cost: Option<String>,
}

/// GMGN API response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct GMGNApiResponse {
    pub code: i32,
    pub msg: String,
    pub tid: Option<String>,
    pub data: Option<SwapData>,
}

// ============================================================================
// CONSTANTS
// ============================================================================

const GMGN_QUOTE_API: &str = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route";
const QUOTE_TIMEOUT_SECS: u64 = 15;
const RETRY_ATTEMPTS: usize = 3;

pub struct GmgnRouter {
    client: Client,
}

impl GmgnRouter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn fetch_gmgn_quote_internal(
        &self,
        input_mint: &str,
        output_mint: &str,
        input_amount: u64,
        from_address: &str,
        slippage: f64,
        swap_mode: &str,
    ) -> Result<SwapData, ScreenerBotError> {
        if let Some(unhealthy) =
            crate::connectivity::check_endpoints_healthy(&["internet", "rpc"]).await
        {
            return Err(ScreenerBotError::connectivity_error(format!(
                "Cannot fetch GMGN quote - Unhealthy endpoints: {}",
                unhealthy
            )));
        }

        let gmgn_fee_sol = with_config(|cfg| cfg.swaps.gmgn.fee_sol);
        let gmgn_anti_mev = with_config(|cfg| cfg.swaps.gmgn.anti_mev);
        let gmgn_partner = with_config(|cfg| cfg.swaps.gmgn.partner.clone());

        let url = format!(
            "{}?token_in_address={}&token_out_address={}&in_amount={}&from_address={}&slippage={}&swap_mode={}&fee={}&is_anti_mev={}&partner={}",
            GMGN_QUOTE_API,
            input_mint,
            output_mint,
            input_amount,
            from_address,
            slippage,
            swap_mode,
            gmgn_fee_sol,
            gmgn_anti_mev,
            gmgn_partner
        );

        logger::debug(
            LogTag::Swap,
            &format!(
                "GMGN quote: {} {} → {} (slippage: {}%)",
                input_amount,
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
                slippage
            ),
        );

        let mut last_error = None;

        for attempt in 1..=RETRY_ATTEMPTS {
            match self
                .client
                .get(&url)
                .timeout(tokio::time::Duration::from_secs(QUOTE_TIMEOUT_SECS))
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        let response_text = match response.text().await {
                            Ok(t) => t,
                            Err(e) => {
                                last_error = Some(ScreenerBotError::invalid_response(format!(
                                    "Failed to get response text: {}",
                                    e
                                )));
                                continue;
                            }
                        };

                        if let Ok(value) = serde_json::from_str::<Value>(&response_text) {
                            let code_opt = value.get("code").and_then(|c| c.as_i64());
                            let msg_opt = value.get("msg").and_then(|m| m.as_str()).unwrap_or("");

                            if let Some(code) = code_opt {
                                if code != 0 {
                                    if code == 40000402 || msg_opt.contains("no route") {
                                        logger::debug(
                                            LogTag::Swap,
                                            &format!(
                                                "GMGN no route for pair (code {}): {}",
                                                code, msg_opt
                                            ),
                                        );
                                        return Err(ScreenerBotError::api_error(format!(
                                            "GMGN no route: {} (code {})",
                                            msg_opt, code
                                        )));
                                    } else {
                                        last_error = Some(ScreenerBotError::api_error(format!(
                                            "GMGN API error: {} - {}",
                                            code, msg_opt
                                        )));
                                        continue;
                                    }
                                }
                            }
                        }

                        match serde_json::from_str::<GMGNApiResponse>(&response_text) {
                            Ok(api_response) => {
                                if api_response.code == 0 {
                                    if let Some(data) = api_response.data {
                                        logger::debug(
                                            LogTag::Swap,
                                            &format!(
                                                "GMGN quote: {} → {} (impact: {}%)",
                                                data.quote.in_amount,
                                                data.quote.out_amount,
                                                data.quote.price_impact_pct
                                            ),
                                        );
                                        return Ok(data);
                                    } else {
                                        last_error = Some(ScreenerBotError::invalid_response(
                                            "GMGN API returned empty data".to_string(),
                                        ));
                                    }
                                } else {
                                    last_error = Some(ScreenerBotError::api_error(format!(
                                        "GMGN API error: {} - {}",
                                        api_response.code, api_response.msg
                                    )));
                                }
                            }
                            Err(e) => {
                                last_error = Some(ScreenerBotError::invalid_response(format!(
                                    "GMGN API JSON parse error: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        last_error = Some(ScreenerBotError::api_error(format!(
                            "GMGN API HTTP error: {}",
                            response.status()
                        )));
                    }
                }
                Err(e) => {
                    last_error = Some(ScreenerBotError::network_error(e.to_string()));
                }
            }

            if attempt < RETRY_ATTEMPTS {
                let delay = tokio::time::Duration::from_millis(1000 * (attempt as u64));
                tokio::time::sleep(delay).await;
            }
        }

        Err(last_error.unwrap_or_else(|| {
            ScreenerBotError::api_error("All GMGN retry attempts failed".to_string())
        }))
    }

    async fn execute_gmgn_swap_internal(
        &self,
        token: &Token,
        input_mint: &str,
        output_mint: &str,
        swap_data: SwapData,
    ) -> Result<String, ScreenerBotError> {
        if let Some(unhealthy) =
            crate::connectivity::check_endpoints_healthy(&["internet", "rpc"]).await
        {
            return Err(ScreenerBotError::connectivity_error(format!(
                "Cannot send GMGN transaction - Unhealthy endpoints: {}",
                unhealthy
            )));
        }

        logger::debug(
            LogTag::Swap,
            &format!(
                "GMGN swap for {} ({})",
                token.symbol,
                if input_mint == SOL_MINT {
                    "buy"
                } else {
                    "sell"
                }
            ),
        );

        let rpc_client = crate::rpc::get_rpc_client();
        let signature = rpc_client
            .sign_send_and_confirm_transaction_simple(&swap_data.raw_tx.swap_transaction)
            .await?;

        let sig_str = signature.to_string();
        logger::info(
            LogTag::Swap,
            &format!("GMGN swap confirmed: {}", &sig_str[..8]),
        );

        // Parse amounts with proper error handling
        let in_amount = swap_data
            .quote
            .in_amount
            .parse::<u64>()
            .unwrap_or_else(|e| {
                logger::warning(
                    LogTag::Swap,
                    &format!(
                        "Failed to parse in_amount '{}': {}",
                        swap_data.quote.in_amount, e
                    ),
                );
                0
            });

        let out_amount = swap_data
            .quote
            .out_amount
            .parse::<u64>()
            .unwrap_or_else(|e| {
                logger::warning(
                    LogTag::Swap,
                    &format!(
                        "Failed to parse out_amount '{}': {}",
                        swap_data.quote.out_amount, e
                    ),
                );
                0
            });

        crate::events::record_swap_event(
            &sig_str,
            input_mint,
            output_mint,
            in_amount,
            out_amount,
            true,
            None,
        )
        .await;

        Ok(sig_str)
    }
}

#[async_trait]
impl SwapRouter for GmgnRouter {
    fn id(&self) -> &'static str {
        "gmgn"
    }

    fn name(&self) -> &'static str {
        "GMGN"
    }

    fn is_enabled(&self) -> bool {
        with_config(|cfg| cfg.swaps.gmgn.enabled)
    }

    fn priority(&self) -> u8 {
        1
    }

    async fn get_quote(&self, request: &QuoteRequest) -> Result<Quote, ScreenerBotError> {
        let swap_data = self
            .fetch_gmgn_quote_internal(
                &request.input_mint,
                &request.output_mint,
                request.input_amount,
                &request.wallet_address,
                request.slippage_pct,
                request.swap_mode.as_str(),
            )
            .await?;

        let output_amount = swap_data.quote.out_amount.parse::<u64>().map_err(|e| {
            ScreenerBotError::parse_error(format!("Failed to parse output_amount: {}", e))
        })?;

        let price_impact = swap_data
            .quote
            .price_impact_pct
            .parse::<f64>()
            .unwrap_or(0.0);

        let execution_data = serde_json::to_vec(&swap_data).map_err(|e| {
            ScreenerBotError::internal_error(format!("Swap data serialization failed: {}", e))
        })?;

        Ok(Quote {
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            input_mint: request.input_mint.clone(),
            output_mint: request.output_mint.clone(),
            input_amount: request.input_amount,
            output_amount,
            price_impact_pct: price_impact,
            fee_lamports: swap_data.raw_tx.prioritization_fee_lamports,
            slippage_bps: (request.slippage_pct * 100.0) as u16,
            route_plan: "GMGN Anti-MEV".to_string(),
            wallet_address: request.wallet_address.clone(),
            swap_mode: request.swap_mode,
            execution_data,
        })
    }

    async fn execute_swap(
        &self,
        token: &Token,
        quote: &Quote,
    ) -> Result<SwapResult, ScreenerBotError> {
        let start = Instant::now();

        let swap_data: SwapData = serde_json::from_slice(&quote.execution_data).map_err(|e| {
            ScreenerBotError::internal_error(format!("Swap data deserialization failed: {}", e))
        })?;

        let signature = self
            .execute_gmgn_swap_internal(
                token,
                &quote.input_mint,
                &quote.output_mint,
                swap_data.clone(),
            )
            .await?;

        Ok(SwapResult {
            success: true,
            router_id: self.id().to_string(),
            router_name: self.name().to_string(),
            transaction_signature: signature,
            input_amount: quote.input_amount,
            output_amount: quote.output_amount,
            price_impact_pct: quote.price_impact_pct,
            fee_lamports: quote.fee_lamports,
            execution_time_ms: start.elapsed().as_millis() as u64,
            effective_price_sol: None,
        })
    }
}
