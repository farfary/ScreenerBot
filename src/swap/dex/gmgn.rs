use crate::swap::types::*;
use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

pub struct GmgnSwap {
    config: GmgnConfig,
    client: Client,
}

impl GmgnSwap {
    pub fn new(config: GmgnConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    pub async fn get_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        if !self.config.enabled {
            return Err(SwapError::DexNotAvailable("GMGN is disabled".to_string()));
        }

        // Updated GMGN API URL structure
        let url = format!(
            "{}/defi/router/v1/sol/tx/get_swap_route",
            self.config.base_url.trim_end_matches('/')
        );

        // Convert slippage from bps to percentage and store in variables to avoid temporary value issues
        let slippage_percent = (request.slippage_bps as f64) / 100.0;
        let amount_str = request.amount.to_string();
        let slippage_str = slippage_percent.to_string();

        // Prepare fee string if needed
        let fee_str = if self.config.referral_fee_bps > 0 {
            let fee_sol = ((self.config.referral_fee_bps as f64) / 10000.0) * 0.001; // Convert bps to SOL
            Some(fee_sol.to_string())
        } else {
            None
        };

        let mut params = vec![
            ("token_in_address", request.input_mint.as_str()),
            ("token_out_address", request.output_mint.as_str()),
            ("in_amount", amount_str.as_str()),
            ("from_address", &request.user_public_key),
            ("slippage", slippage_str.as_str()),
            ("swap_mode", "ExactIn")
        ];

        // Add anti-MEV parameter if enabled
        if request.is_anti_mev {
            params.push(("is_anti_mev", "true"));
            params.push(("fee", "0.002")); // Minimum fee for anti-MEV
        } else {
            params.push(("is_anti_mev", "false"));
            // Add referral fee if configured, otherwise use minimal fee for non-anti-MEV
            if let Some(ref fee) = fee_str {
                params.push(("fee", fee.as_str()));
            } else {
                params.push(("fee", "0.0001")); // Minimal fee for cheapest swap
            }
        }

        let response = self.client
            .get(&url)
            .query(&params)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(format!("GMGN API error: {}", error_text)));
        }

        let quote_response: GmgnApiResponse = response
            .json().await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        if quote_response.code != 0 {
            return Err(
                SwapError::ApiError(
                    format!(
                        "GMGN API error: {} (code: {})",
                        quote_response.msg,
                        quote_response.code
                    )
                )
            );
        }

        if quote_response.data.quote.is_none() {
            return Err(SwapError::ApiError("GMGN API returned no quote data".to_string()));
        }

        self.parse_gmgn_quote(&quote_response.data, request)
    }

    pub async fn get_swap_transaction(
        &self,
        route: &SwapRoute,
        user_public_key: &str
    ) -> Result<SwapTransaction, SwapError> {
        // With the new GMGN API, the swap transaction is already included in the quote response
        // We need to re-request with the user's public key if it wasn't provided in the original request

        let url = format!(
            "{}/defi/router/v1/sol/tx/get_swap_route",
            self.config.base_url.trim_end_matches('/')
        );

        let slippage_percent = (route.slippage_bps as f64) / 100.0;
        let slippage_str = slippage_percent.to_string();

        // Prepare fee string if needed
        let fee_str = if self.config.referral_fee_bps > 0 {
            let fee_sol = ((self.config.referral_fee_bps as f64) / 10000.0) * 0.001; // Convert bps to SOL
            Some(fee_sol.to_string())
        } else {
            None
        };

        let mut params = vec![
            ("token_in_address", route.input_mint.as_str()),
            ("token_out_address", route.output_mint.as_str()),
            ("in_amount", &route.in_amount),
            ("from_address", user_public_key),
            ("slippage", slippage_str.as_str()),
            ("swap_mode", route.swap_mode.as_str())
        ];

        // Add anti-MEV parameter based on route configuration
        params.push(("is_anti_mev", "false")); // We want lowest cost, so no anti-MEV

        // Add fee parameter (required by GMGN API)
        if let Some(ref fee) = fee_str {
            params.push(("fee", fee.as_str()));
        } else {
            params.push(("fee", "0.0001")); // Minimal fee for cheapest swap
        }

        let response = self.client
            .get(&url)
            .query(&params)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(format!("GMGN swap API error: {}", error_text)));
        }

        let api_response: GmgnApiResponse = response
            .json().await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        if api_response.code != 0 {
            return Err(
                SwapError::ApiError(
                    format!("GMGN API error: {} (code: {})", api_response.msg, api_response.code)
                )
            );
        }

        let raw_tx = api_response.data.raw_tx
            .as_ref()
            .ok_or_else(||
                SwapError::ApiError("Missing raw_tx data in GMGN response".to_string())
            )?;

        Ok(SwapTransaction {
            swap_transaction: raw_tx.swap_transaction.clone(),
            last_valid_block_height: raw_tx.last_valid_block_height,
            priority_fee_info: None, // GMGN doesn't provide this in the same format
        })
    }

    fn parse_gmgn_quote(
        &self,
        quote_data: &GmgnApiData,
        request: &SwapRequest
    ) -> Result<SwapRoute, SwapError> {
        let quote = quote_data.quote
            .as_ref()
            .ok_or_else(|| SwapError::ApiError("Missing quote data in GMGN response".to_string()))?;

        // Create a basic route plan since GMGN doesn't provide detailed routing info
        let route_plan = vec![RoutePlan {
            swap_info: SwapInfo {
                amm_key: "gmgn_pool".to_string(),
                label: "GMGN".to_string(),
                input_mint: quote.input_mint.clone(),
                output_mint: quote.output_mint.clone(),
                in_amount: quote.in_amount.clone(),
                out_amount: quote.out_amount.clone(),
                fee_amount: "0".to_string(),
                fee_mint: quote.input_mint.clone(),
            },
            percent: 100,
        }];

        let platform_fee = if self.config.referral_fee_bps > 0 {
            Some(PlatformFee {
                amount: "0".to_string(), // GMGN calculates this differently
                fee_bps: self.config.referral_fee_bps,
            })
        } else {
            None
        };

        Ok(SwapRoute {
            dex: DexType::Gmgn,
            input_mint: quote.input_mint.clone(),
            output_mint: quote.output_mint.clone(),
            in_amount: quote.in_amount.clone(),
            out_amount: quote.out_amount.clone(),
            other_amount_threshold: quote.other_amount_threshold.clone(),
            swap_mode: quote.swap_mode.clone(),
            slippage_bps: quote.slippage_bps,
            platform_fee,
            price_impact_pct: quote.price_impact_pct.clone(),
            route_plan,
            context_slot: quote.context_slot,
            time_taken: quote.time_taken,
        })
    }

    pub async fn get_price(&self, input_mint: &str, output_mint: &str) -> Result<f64, SwapError> {
        // GMGN doesn't have a separate price endpoint, so we'll use a small quote to get price
        let small_amount = if input_mint == SOL_MINT { 1_000_000 } else { 1_000_000 }; // 0.001 SOL or equivalent

        let dummy_pubkey = "11111111111111111111111111111112"; // System program ID as dummy

        let url = format!(
            "{}/defi/router/v1/sol/tx/get_swap_route",
            self.config.base_url.trim_end_matches('/')
        );

        let params = [
            ("token_in_address", input_mint),
            ("token_out_address", output_mint),
            ("in_amount", &small_amount.to_string()),
            ("from_address", dummy_pubkey),
            ("slippage", "0.5"),
            ("swap_mode", "ExactIn"),
            ("is_anti_mev", "false"),
            ("fee", "0.0001"), // Minimal fee required by GMGN API
        ];

        let response = self.client
            .get(&url)
            .query(&params)
            .send().await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(format!("GMGN price API error: {}", error_text)));
        }

        let api_response: GmgnApiResponse = response
            .json().await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        if api_response.code != 0 {
            return Err(
                SwapError::ApiError(
                    format!("GMGN API error: {} (code: {})", api_response.msg, api_response.code)
                )
            );
        }

        let quote = api_response.data.quote
            .as_ref()
            .ok_or_else(|| SwapError::ApiError("Missing quote data in GMGN response".to_string()))?;

        // Calculate price from the quote
        let in_amount: f64 = quote.in_amount
            .parse()
            .map_err(|_| SwapError::SerializationError("Invalid input amount".to_string()))?;
        let out_amount: f64 = quote.out_amount
            .parse()
            .map_err(|_| SwapError::SerializationError("Invalid output amount".to_string()))?;

        if in_amount > 0.0 {
            Ok(out_amount / in_amount)
        } else {
            Err(SwapError::SerializationError("Invalid input amount".to_string()))
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn supports_anti_mev(&self) -> bool {
        true // GMGN supports anti-MEV features
    }

    pub fn get_supported_networks(&self) -> Vec<String> {
        vec!["solana".to_string()]
    }
}

// Add Clone trait where needed
impl Clone for GmgnSwap {
    fn clone(&self) -> Self {
        Self::new(self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> GmgnConfig {
        GmgnConfig {
            enabled: true,
            base_url: "https://gmgn.ai".to_string(),
            timeout_seconds: 15,
            referral_fee_bps: 0,
        }
    }

    #[tokio::test]
    async fn test_gmgn_quote() {
        let config = create_test_config();
        let gmgn = GmgnSwap::new(config);

        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 10_000_000, // 0.01 SOL (as requested)
            slippage_bps: 50, // 0.5%
            user_public_key: "11111111111111111111111111111112".to_string(), // Dummy public key
            dex_preference: Some(DexType::Gmgn),
            is_anti_mev: false, // As requested, don't use anti-MEV
        };

        match gmgn.get_quote(&request).await {
            Ok(route) => {
                println!("GMGN quote successful:");
                println!("  Input: {} {}", route.in_amount, route.input_mint);
                println!("  Output: {} {}", route.out_amount, route.output_mint);
                println!("  Price Impact: {}%", route.price_impact_pct);
                println!("  Routes: {}", route.route_plan.len());
                assert_eq!(route.dex, DexType::Gmgn);
            }
            Err(e) => {
                println!("GMGN quote failed: {}", e);
                // Don't fail the test since we might not have API access
            }
        }
    }

    #[tokio::test]
    async fn test_gmgn_price() {
        let config = create_test_config();
        let gmgn = GmgnSwap::new(config);

        match gmgn.get_price(SOL_MINT, USDC_MINT).await {
            Ok(price) => {
                println!("GMGN price successful: {} USDC per SOL", price);
                assert!(price > 0.0);
            }
            Err(e) => {
                println!("GMGN price failed: {}", e);
                // Don't fail the test since we might not have API access
            }
        }
    }

    #[test]
    fn test_gmgn_config() {
        let config = create_test_config();
        let gmgn = GmgnSwap::new(config);

        assert!(gmgn.is_enabled());
        assert!(gmgn.supports_anti_mev());
        assert!(!gmgn.get_supported_networks().is_empty());
    }
}
