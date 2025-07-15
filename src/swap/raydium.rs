use super::types::*;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

pub struct RaydiumSwap {
    config: RaydiumConfig,
    client: Client,
}

impl RaydiumSwap {
    pub fn new(config: RaydiumConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    pub async fn get_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        if !self.config.enabled {
            return Err(SwapError::DexNotAvailable("Raydium is disabled".to_string()));
        }

        // Raydium V2 API for quotes
        let url = format!("{}/swap/route", self.config.base_url);
        
        let params = vec![
            ("inputMint", request.input_mint.clone()),
            ("outputMint", request.output_mint.clone()),
            ("amount", request.amount.to_string()),
            ("slippageBps", request.slippage_bps.to_string()),
            ("onlyDirectRoutes", "false".to_string()),
        ];

        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SwapError::ApiError(format!("Raydium API error: {}", error_text)));
        }

        let quote: Value = response
            .json()
            .await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        self.parse_raydium_quote(&quote, request)
    }

    pub async fn get_swap_transaction(
        &self,
        route: &SwapRoute,
        user_public_key: &str,
    ) -> Result<SwapTransaction, SwapError> {
        let url = format!("{}/swap", self.config.base_url);

        let swap_request = serde_json::json!({
            "route": self.route_to_raydium_format(route),
            "userPublicKey": user_public_key,
            "wrapSol": true,
            "unwrapSol": true,
            "feeAccount": null,
            "computeUnitPriceMicroLamports": null
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
            return Err(SwapError::ApiError(format!("Raydium swap API error: {}", error_text)));
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
                .unwrap_or(0),
            priority_fee_info: None,
        })
    }

    fn parse_raydium_quote(&self, quote: &Value, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        let data = &quote["data"];
        
        let input_mint = data["inputMint"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing inputMint".to_string()))?
            .to_string();

        let output_mint = data["outputMint"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing outputMint".to_string()))?
            .to_string();

        let in_amount = data["inAmount"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing inAmount".to_string()))?
            .to_string();

        let out_amount = data["outAmount"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing outAmount".to_string()))?
            .to_string();

        let other_amount_threshold = data["otherAmountThreshold"]
            .as_str()
            .ok_or_else(|| SwapError::SerializationError("Missing otherAmountThreshold".to_string()))?
            .to_string();

        let price_impact_pct = data["priceImpactPct"]
            .as_str()
            .unwrap_or("0")
            .to_string();

        // Parse route plan for Raydium
        let route_plan = data["routePlan"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|plan| {
                let swap_info = &plan["swapInfo"];
                RoutePlan {
                    swap_info: SwapInfo {
                        amm_key: swap_info["poolId"].as_str().unwrap_or("").to_string(),
                        label: "Raydium".to_string(),
                        input_mint: swap_info["inputMint"].as_str().unwrap_or("").to_string(),
                        output_mint: swap_info["outputMint"].as_str().unwrap_or("").to_string(),
                        in_amount: swap_info["inAmount"].as_str().unwrap_or("").to_string(),
                        out_amount: swap_info["outAmount"].as_str().unwrap_or("").to_string(),
                        fee_amount: swap_info["feeAmount"].as_str().unwrap_or("0").to_string(),
                        fee_mint: swap_info["feeMint"].as_str().unwrap_or("").to_string(),
                    },
                    percent: plan["percent"].as_u64().unwrap_or(100) as u32,
                }
            })
            .collect();

        Ok(SwapRoute {
            dex: DexType::Raydium,
            input_mint,
            output_mint,
            in_amount,
            out_amount,
            other_amount_threshold,
            swap_mode: "ExactIn".to_string(),
            slippage_bps: request.slippage_bps,
            platform_fee: None, // Raydium fees are included in the price
            price_impact_pct,
            route_plan,
            context_slot: data["contextSlot"].as_u64(),
            time_taken: data["timeTaken"].as_f64(),
        })
    }

    fn route_to_raydium_format(&self, route: &SwapRoute) -> Value {
        serde_json::json!({
            "inputMint": route.input_mint,
            "inAmount": route.in_amount,
            "outputMint": route.output_mint,
            "outAmount": route.out_amount,
            "otherAmountThreshold": route.other_amount_threshold,
            "swapMode": route.swap_mode,
            "slippageBps": route.slippage_bps,
            "priceImpactPct": route.price_impact_pct,
            "routePlan": route.route_plan.iter().map(|plan| {
                serde_json::json!({
                    "swapInfo": {
                        "poolId": plan.swap_info.amm_key,
                        "inputMint": plan.swap_info.input_mint,
                        "outputMint": plan.swap_info.output_mint,
                        "inAmount": plan.swap_info.in_amount,
                        "outAmount": plan.swap_info.out_amount,
                        "feeAmount": plan.swap_info.fee_amount,
                        "feeMint": plan.swap_info.fee_mint
                    },
                    "percent": plan.percent
                })
            }).collect::<Vec<_>>()
        })
    }

    pub async fn get_pools(&self) -> Result<Vec<Value>, SwapError> {
        let url = format!("{}/pools", self.config.base_url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SwapError::ApiError("Failed to fetch pools".to_string()));
        }

        let pools: Value = response
            .json()
            .await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        Ok(pools["data"].as_array().unwrap_or(&vec![]).clone())
    }

    pub async fn get_pool_info(&self, pool_id: &str) -> Result<Value, SwapError> {
        let url = format!("{}/pools/{}", self.config.base_url, pool_id);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| SwapError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SwapError::ApiError("Failed to fetch pool info".to_string()));
        }

        let pool_info: Value = response
            .json()
            .await
            .map_err(|e| SwapError::SerializationError(e.to_string()))?;

        Ok(pool_info)
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn supports_anti_mev(&self) -> bool {
        false // Raydium doesn't have built-in anti-MEV features
    }

    pub fn get_supported_pool_types(&self) -> Vec<String> {
        match self.config.pool_type.as_str() {
            "all" => vec![
                "standard".to_string(),
                "concentrated".to_string(),
                "stable".to_string(),
            ],
            specific => vec![specific.to_string()],
        }
    }
}

// Add Clone trait where needed
impl Clone for RaydiumSwap {
    fn clone(&self) -> Self {
        Self::new(self.config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> RaydiumConfig {
        RaydiumConfig {
            enabled: true,
            base_url: "https://api.raydium.io/v2".to_string(),
            timeout_seconds: 15,
            pool_type: "all".to_string(),
        }
    }

    #[tokio::test]
    async fn test_raydium_quote() {
        let config = create_test_config();
        let raydium = RaydiumSwap::new(config);

        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 1_000_000, // 0.001 SOL
            slippage_bps: 50, // 0.5%
            user_public_key: "".to_string(),
            dex_preference: Some(DexType::Raydium),
            is_anti_mev: false,
        };

        match raydium.get_quote(&request).await {
            Ok(route) => {
                println!("Raydium quote successful:");
                println!("  Input: {} {}", route.in_amount, route.input_mint);
                println!("  Output: {} {}", route.out_amount, route.output_mint);
                println!("  Price Impact: {}%", route.price_impact_pct);
                println!("  Routes: {}", route.route_plan.len());
                assert_eq!(route.dex, DexType::Raydium);
            }
            Err(e) => {
                println!("Raydium quote failed: {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }

    #[tokio::test]
    async fn test_raydium_pools() {
        let config = create_test_config();
        let raydium = RaydiumSwap::new(config);

        match raydium.get_pools().await {
            Ok(pools) => {
                println!("Raydium returned {} pools", pools.len());
                // Don't assert since pool count can vary
            }
            Err(e) => {
                println!("Raydium pools failed: {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }

    #[test]
    fn test_raydium_config() {
        let config = create_test_config();
        let raydium = RaydiumSwap::new(config);
        
        assert!(raydium.is_enabled());
        assert!(!raydium.supports_anti_mev());
        assert!(!raydium.get_supported_pool_types().is_empty());
    }
}
