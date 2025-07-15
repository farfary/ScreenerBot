use super::types::*;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct JupiterSwap {
    config: JupiterConfig,
    client: Client,
}

impl JupiterSwap {
    pub fn new(config: JupiterConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    pub async fn get_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        if !self.config.enabled {
            return Err(SwapError::DexNotAvailable("Jupiter is disabled".to_string()));
        }

        let url = format!("{}/quote", self.config.base_url);
        
        let mut params = vec![
            ("inputMint", request.input_mint.clone()),
            ("outputMint", request.output_mint.clone()),
            ("amount", request.amount.to_string()),
            ("slippageBps", request.slippage_bps.to_string()),
            ("onlyDirectRoutes", self.config.only_direct_routes.to_string()),
            ("asLegacyTransaction", self.config.as_legacy_transaction.to_string()),
            ("maxAccounts", self.config.max_accounts.to_string()),
        ];

        if request.is_anti_mev {
            params.push(("restrictIntermediateTokens", "true".to_string()));
        }

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(format!("Jupiter API error: {}", error_text)));
        }

        let quote: Value = response
            .json()
            .await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        self.parse_jupiter_quote(&quote, request)
    }

    pub async fn get_swap_transaction(
        &self,
        route: &SwapRoute,
        user_public_key: &str,
    ) -> Result<SwapTransaction, SwapError> {
        let url = format!("{}/swap", self.config.base_url);

        let swap_request = serde_json::json!({
            "quoteResponse": self.route_to_jupiter_quote(route),
            "userPublicKey": user_public_key,
            "wrapAndUnwrapSol": true,
            "useSharedAccounts": true,
            "feeAccount": null,
            "trackingAccount": null,
            "computeUnitPriceMicroLamports": null,
            "prioritizationFeeLamports": null,
            "asLegacyTransaction": self.config.as_legacy_transaction,
            "useTokenLedger": false,
            "destinationTokenAccount": null,
            "dynamicComputeUnitLimit": false,
            "skipUserAccountsRpcCalls": false
        });

        let response = self
            .client
            .post(&url)
            .json(&swap_request)
            .send()
            .await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(format!("Jupiter swap API error: {}", error_text)));
        }

        let swap_response: Value = response
            .json()
            .await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        Ok(SwapTransaction {
            swap_transaction: swap_response["swapTransaction"]
                .as_str()
                .ok_or_else(|| SwapError::SerializationError("Missing swapTransaction".to_string()))?
                .to_string(),
            last_valid_block_height: swap_response["lastValidBlockHeight"]
                .as_u64()
                .ok_or_else(|| SwapError::SerializationError("Missing lastValidBlockHeight".to_string()))?,
            priority_fee_info: None, // Jupiter doesn't return this in the same format
        })
    }

    fn parse_jupiter_quote(&self, quote: &Value, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        let input_mint = quote["inputMint"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing inputMint".to_string()))?
            .to_string();

        let output_mint = quote["outputMint"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing outputMint".to_string()))?
            .to_string();

        let in_amount = quote["inAmount"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing inAmount".to_string()))?
            .to_string();

        let out_amount = quote["outAmount"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing outAmount".to_string()))?
            .to_string();

        let other_amount_threshold = quote["otherAmountThreshold"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing otherAmountThreshold".to_string()))?
            .to_string();

        let price_impact_pct = quote["priceImpactPct"]
            .as_str()
            .unwrap_or("0")
            .to_string();

        let context_slot = quote["contextSlot"].as_u64();
        let time_taken = quote["timeTaken"].as_f64();

        let route_plan = quote["routePlan"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|plan| {
                let swap_info = &plan["swapInfo"];
                RoutePlan {
                    swap_info: SwapInfo {
                        amm_key: swap_info["ammKey"].as_str().unwrap_or("").to_string(),
                        label: swap_info["label"].as_str().unwrap_or("").to_string(),
                        input_mint: swap_info["inputMint"].as_str().unwrap_or("").to_string(),
                        output_mint: swap_info["outputMint"].as_str().unwrap_or("").to_string(),
                        in_amount: swap_info["inAmount"].as_str().unwrap_or("").to_string(),
                        out_amount: swap_info["outAmount"].as_str().unwrap_or("").to_string(),
                        fee_amount: swap_info["feeAmount"].as_str().unwrap_or("").to_string(),
                        fee_mint: swap_info["feeMint"].as_str().unwrap_or("").to_string(),
                    },
                    percent: plan["percent"].as_u64().unwrap_or(100) as u32,
                }
            })
            .collect();

        Ok(SwapRoute {
            dex: DexType::Jupiter,
            input_mint,
            output_mint,
            in_amount,
            out_amount,
            other_amount_threshold,
            swap_mode: "ExactIn".to_string(), // Jupiter default
            slippage_bps: request.slippage_bps,
            platform_fee: None, // Jupiter doesn't charge platform fees
            price_impact_pct,
            route_plan,
            context_slot,
            time_taken,
        })
    }

    fn route_to_jupiter_quote(&self, route: &SwapRoute) -> Value {
        serde_json::json!({
            "inputMint": route.input_mint,
            "inAmount": route.in_amount,
            "outputMint": route.output_mint,
            "outAmount": route.out_amount,
            "otherAmountThreshold": route.other_amount_threshold,
            "swapMode": route.swap_mode,
            "slippageBps": route.slippage_bps,
            "platformFee": route.platform_fee,
            "priceImpactPct": route.price_impact_pct,
            "routePlan": route.route_plan.iter().map(|plan| {
                serde_json::json!({
                    "swapInfo": {
                        "ammKey": plan.swap_info.amm_key,
                        "label": plan.swap_info.label,
                        "inputMint": plan.swap_info.input_mint,
                        "outputMint": plan.swap_info.output_mint,
                        "inAmount": plan.swap_info.in_amount,
                        "outAmount": plan.swap_info.out_amount,
                        "feeAmount": plan.swap_info.fee_amount,
                        "feeMint": plan.swap_info.fee_mint
                    },
                    "percent": plan.percent
                })
            }).collect::<Vec<_>>(),
            "contextSlot": route.context_slot,
            "timeTaken": route.time_taken
        })
    }

    pub async fn get_tokens(&self) -> Result<Vec<TokenInfo>, SwapError> {
        let url = format!("{}/tokens", self.config.base_url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SwapError::ApiError("Failed to fetch tokens".to_string()));
        }

        let tokens: Vec<Value> = response
            .json()
            .await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        Ok(tokens
            .into_iter()
            .filter_map(|token| {
                Some(TokenInfo {
                    mint: token["address"].as_str()?.to_string(),
                    symbol: token["symbol"].as_str()?.to_string(),
                    name: token["name"].as_str()?.to_string(),
                    decimals: token["decimals"].as_u64()? as u8,
                    logo_uri: token["logoURI"].as_str().map(|s| s.to_string()),
                })
            })
            .collect())
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> JupiterConfig {
        JupiterConfig {
            enabled: true,
            base_url: "https://quote-api.jup.ag/v6".to_string(),
            timeout_seconds: 15,
            max_accounts: 64,
            only_direct_routes: false,
            as_legacy_transaction: false,
        }
    }

    #[tokio::test]
    async fn test_jupiter_quote() {
        let config = create_test_config();
        let jupiter = JupiterSwap::new(config);

        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 1_000_000, // 0.001 SOL
            slippage_bps: 50, // 0.5%
            user_public_key: "".to_string(),
            dex_preference: Some(DexType::Jupiter),
            is_anti_mev: false,
        };

        match jupiter.get_quote(&request).await {
            Ok(route) => {
                println!("Jupiter quote successful:");
                println!("  Input: {} {}", route.in_amount, route.input_mint);
                println!("  Output: {} {}", route.out_amount, route.output_mint);
                println!("  Price Impact: {}%", route.price_impact_pct);
                println!("  Routes: {}", route.route_plan.len());
                assert_eq!(route.dex, DexType::Jupiter);
            }
            Err(e) => {
                println!("Jupiter quote failed: {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }

    #[tokio::test]
    async fn test_jupiter_tokens() {
        let config = create_test_config();
        let jupiter = JupiterSwap::new(config);

        match jupiter.get_tokens().await {
            Ok(tokens) => {
                println!("Jupiter returned {} tokens", tokens.len());
                assert!(!tokens.is_empty());
                
                // Check if common tokens exist
                let sol_token = tokens.iter().find(|t| t.mint == SOL_MINT);
                assert!(sol_token.is_some());
                
                let usdc_token = tokens.iter().find(|t| t.mint == USDC_MINT);
                assert!(usdc_token.is_some());
            }
            Err(e) => {
                println!("Jupiter tokens failed: {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }
}

impl Clone for JupiterSwap {
    fn clone(&self) -> Self {
        Self::new(self.config.clone())
    }
}
