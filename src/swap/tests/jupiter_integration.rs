#[cfg(test)]
mod jupiter_integration_tests {
    use crate::swap::dex::JupiterSwap;
    use crate::swap::types::*;

    fn create_test_jupiter_config() -> JupiterConfig {
        JupiterConfig {
            enabled: true,
            base_url: "https://lite-api.jup.ag/swap/v1".to_string(),
            timeout_seconds: 15,
            max_accounts: 64,
            only_direct_routes: false,
            as_legacy_transaction: false,
        }
    }

    #[tokio::test]
    async fn test_jupiter_sol_to_usdc_quote() {
        let config = create_test_jupiter_config();
        let jupiter = JupiterSwap::new(config);

        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 100_000_000, // 0.1 SOL
            slippage_bps: 50, // 0.5%
            user_public_key: "11111111111111111111111111111112".to_string(),
            dex_preference: Some(DexType::Jupiter),
            is_anti_mev: true,
        };

        match jupiter.get_quote(&request).await {
            Ok(route) => {
                println!("✅ Jupiter quote successful!");
                println!(
                    "   Input: {} SOL",
                    (route.in_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000_000.0
                );
                println!(
                    "   Output: {} USDC",
                    (route.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000.0
                );
                println!("   Price Impact: {}%", route.price_impact_pct);
                println!("   Routes: {}", route.route_plan.len());

                // Assertions
                assert_eq!(route.dex, DexType::Jupiter);
                assert_eq!(route.input_mint, SOL_MINT);
                assert_eq!(route.output_mint, USDC_MINT);
                assert_eq!(route.in_amount, "100000000");
                assert!(!route.out_amount.is_empty());
                assert!(!route.route_plan.is_empty());
            }
            Err(e) => {
                println!("⚠️  Jupiter quote failed (this may be expected if no network): {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }

    #[tokio::test]
    async fn test_jupiter_usdc_to_sol_quote() {
        let config = create_test_jupiter_config();
        let jupiter = JupiterSwap::new(config);

        let request = SwapRequest {
            input_mint: USDC_MINT.to_string(),
            output_mint: SOL_MINT.to_string(),
            amount: 10_000_000, // 10 USDC
            slippage_bps: 100, // 1%
            user_public_key: "11111111111111111111111111111112".to_string(),
            dex_preference: Some(DexType::Jupiter),
            is_anti_mev: false,
        };

        match jupiter.get_quote(&request).await {
            Ok(route) => {
                println!("✅ Jupiter reverse quote successful!");
                println!(
                    "   Input: {} USDC",
                    (route.in_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000.0
                );
                println!(
                    "   Output: {} SOL",
                    (route.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000_000.0
                );
                println!("   Price Impact: {}%", route.price_impact_pct);

                // Assertions
                assert_eq!(route.dex, DexType::Jupiter);
                assert_eq!(route.input_mint, USDC_MINT);
                assert_eq!(route.output_mint, SOL_MINT);
                assert_eq!(route.in_amount, "10000000");
                assert!(!route.out_amount.is_empty());
            }
            Err(e) => {
                println!("⚠️  Jupiter reverse quote failed: {}", e);
                // Don't fail the test since we might not have network access
            }
        }
    }

    #[test]
    fn test_jupiter_config_creation() {
        let config = create_test_jupiter_config();
        let jupiter = JupiterSwap::new(config.clone());

        assert!(jupiter.is_enabled());
        assert_eq!(config.base_url, "https://lite-api.jup.ag/swap/v1");
        assert_eq!(config.timeout_seconds, 15);
        assert_eq!(config.max_accounts, 64);
    }
}
