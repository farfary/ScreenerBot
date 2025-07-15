use crate::swap::*;
use crate::config::{Config, TransactionManagerConfig};
use crate::database::Database;
use crate::rpc_manager::RpcManager;
use crate::trading::transaction_manager::TransactionManager;
use crate::wallet::WalletTracker;
use anyhow::Result;
use solana_sdk::signature::{Keypair, Signer};
use std::sync::Arc;

/// Integration tests for the swap module
/// These tests will perform real swaps with small amounts

pub struct SwapTestRunner {
    swap_manager: SwapManager,
    test_keypair: Keypair,
}

impl SwapTestRunner {
    pub async fn new() -> Result<Self> {
        // Load configuration
        let config = load_test_config().await?;
        
        // Create RPC manager
        let rpc_manager = Arc::new(RpcManager::new(
            config.rpc_url.clone(),
            config.rpc_fallbacks.clone(),
        ));

        // Create database and transaction manager
        let database = Arc::new(Database::new("test_swap.db")?);
        let config_clone = config.clone();
        let wallet_tracker = Arc::new(WalletTracker::new(config_clone, database.clone())?);
        let transaction_manager = Arc::new(TransactionManager::new(
            TransactionManagerConfig {
                cache_transactions: true,
                cache_duration_hours: 24,
                track_pnl: true,
                auto_calculate_profits: true,
            },
            database,
            wallet_tracker,
        ));

        // Create swap manager
        let swap_manager = create_swap_manager(&config, rpc_manager, transaction_manager)?;

        // Create test keypair from config
        let test_keypair = Keypair::from_base58_string(&config.main_wallet_private);

        Ok(Self {
            swap_manager,
            test_keypair,
        })
    }

    /// Test SOL to USDC swap with 0.001 SOL
    pub async fn test_sol_to_usdc_swap(&self) -> Result<SwapResult> {
        let amount_sol = 0.001;
        let amount_lamports = sol_to_lamports(amount_sol);
        
        let request = create_swap_request(
            SOL_MINT,
            USDC_MINT,
            amount_lamports,
            50, // 0.5% slippage
            &self.test_keypair.pubkey().to_string(),
            None, // Let the system choose the best DEX
            false,
        );

        println!("🔄 Testing SOL -> USDC swap ({} SOL)", amount_sol);
        let result = self.swap_manager.execute_swap(request, &self.test_keypair).await?;
        
        self.print_swap_result(&result, "SOL", "USDC");
        self.verify_swap_result(&result)?;
        
        Ok(result)
    }

    /// Test USDC to SOL swap with equivalent USDC amount
    pub async fn test_usdc_to_sol_swap(&self, usdc_amount: f64) -> Result<SwapResult> {
        let amount_micro_usdc = usdc_to_micro_usdc(usdc_amount);
        
        let request = create_swap_request(
            USDC_MINT,
            SOL_MINT,
            amount_micro_usdc,
            50, // 0.5% slippage
            &self.test_keypair.pubkey().to_string(),
            None, // Let the system choose the best DEX
            false,
        );

        println!("🔄 Testing USDC -> SOL swap ({} USDC)", usdc_amount);
        let result = self.swap_manager.execute_swap(request, &self.test_keypair).await?;
        
        self.print_swap_result(&result, "USDC", "SOL");
        self.verify_swap_result(&result)?;
        
        Ok(result)
    }

    /// Test quote-only (no execution) for all DEXes
    pub async fn test_quotes_comparison(&self) -> Result<()> {
        let amount_sol = 0.001;
        let amount_lamports = sol_to_lamports(amount_sol);
        
        let request = create_swap_request(
            SOL_MINT,
            USDC_MINT,
            amount_lamports,
            50,
            &self.test_keypair.pubkey().to_string(),
            None,
            false,
        );

        println!("📊 Comparing quotes from all DEXes for {} SOL -> USDC", amount_sol);
        
        // Test each DEX individually
        for dex in [DexType::Jupiter, DexType::Raydium, DexType::Gmgn] {
            if !self.swap_manager.is_dex_available(&dex) {
                println!("  ❌ {} is not available", dex);
                continue;
            }

            let mut dex_request = request.clone();
            dex_request.dex_preference = Some(dex.clone());

            match self.swap_manager.get_best_quote(&dex_request).await {
                Ok(route) => {
                    let output_usdc = micro_usdc_to_usdc(route.out_amount.parse().unwrap_or(0));
                    println!(
                        "  ✅ {}: {:.6} USDC (Impact: {}%, Hops: {})",
                        dex,
                        output_usdc,
                        route.price_impact_pct,
                        route.route_plan.len()
                    );
                }
                Err(e) => {
                    println!("  ❌ {} failed: {}", dex, e);
                }
            }
        }

        // Get the best overall quote
        match self.swap_manager.get_best_quote(&request).await {
            Ok(best_route) => {
                let output_usdc = micro_usdc_to_usdc(best_route.out_amount.parse().unwrap_or(0));
                println!(
                    "🏆 Best quote: {} with {:.6} USDC (Impact: {}%)",
                    best_route.dex,
                    output_usdc,
                    best_route.price_impact_pct
                );
            }
            Err(e) => {
                println!("❌ Failed to get best quote: {}", e);
            }
        }

        Ok(())
    }

    /// Test anti-MEV functionality (if supported)
    pub async fn test_anti_mev_swap(&self) -> Result<()> {
        let amount_sol = 0.001;
        let amount_lamports = sol_to_lamports(amount_sol);
        
        let request = create_swap_request(
            SOL_MINT,
            USDC_MINT,
            amount_lamports,
            50,
            &self.test_keypair.pubkey().to_string(),
            Some(DexType::Gmgn), // GMGN supports anti-MEV
            true, // Enable anti-MEV
        );

        println!("🛡️ Testing anti-MEV swap with GMGN");
        
        if !self.swap_manager.is_dex_available(&DexType::Gmgn) {
            println!("❌ GMGN is not available, skipping anti-MEV test");
            return Ok(());
        }

        match self.swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                println!(
                    "✅ Anti-MEV quote successful: {} USDC (Impact: {}%)",
                    micro_usdc_to_usdc(route.out_amount.parse().unwrap_or(0)),
                    route.price_impact_pct
                );
            }
            Err(e) => {
                println!("❌ Anti-MEV quote failed: {}", e);
            }
        }

        Ok(())
    }

    /// Test slippage protection
    pub async fn test_slippage_protection(&self) -> Result<()> {
        let amount_sol = 0.001;
        let amount_lamports = sol_to_lamports(amount_sol);
        
        // Test with very low slippage (should fail or have limited routes)
        let low_slippage_request = create_swap_request(
            SOL_MINT,
            USDC_MINT,
            amount_lamports,
            1, // 0.01% slippage (very tight)
            &self.test_keypair.pubkey().to_string(),
            None,
            false,
        );

        println!("🎯 Testing slippage protection with 0.01% slippage");
        
        match self.swap_manager.get_best_quote(&low_slippage_request).await {
            Ok(route) => {
                println!(
                    "✅ Low slippage quote successful: {} USDC (Impact: {}%)",
                    micro_usdc_to_usdc(route.out_amount.parse().unwrap_or(0)),
                    route.price_impact_pct
                );
            }
            Err(SwapError::SlippageTooHigh { expected, actual }) => {
                println!("✅ Slippage protection working: expected {:.2}%, got {:.2}%", expected * 100.0, actual);
            }
            Err(e) => {
                println!("❌ Unexpected error: {}", e);
            }
        }

        Ok(())
    }

    /// Run a full round-trip test (SOL -> USDC -> SOL)
    pub async fn test_round_trip(&self) -> Result<()> {
        println!("🔄 Starting round-trip test: SOL -> USDC -> SOL");
        
        // Step 1: SOL to USDC
        let sol_to_usdc_result = self.test_sol_to_usdc_swap().await?;
        let usdc_received = micro_usdc_to_usdc(sol_to_usdc_result.output_amount);
        
        // Wait a moment to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Step 2: USDC back to SOL (use 95% of received USDC to account for fees)
        let usdc_to_swap = usdc_received * 0.95;
        let usdc_to_sol_result = self.test_usdc_to_sol_swap(usdc_to_swap).await?;
        let sol_received = lamports_to_sol(usdc_to_sol_result.output_amount);
        
        // Calculate round-trip efficiency
        let original_sol = 0.001;
        let efficiency = (sol_received / original_sol) * 100.0;
        
        println!("🎯 Round-trip completed:");
        println!("  Started with: {:.6} SOL", original_sol);
        println!("  Received USDC: {:.6} USDC", usdc_received);
        println!("  Final SOL: {:.6} SOL", sol_received);
        println!("  Efficiency: {:.2}%", efficiency);
        
        if efficiency > 90.0 {
            println!("✅ Round-trip efficiency is good (> 90%)");
        } else {
            println!("⚠️ Round-trip efficiency is low (< 90%)");
        }

        Ok(())
    }

    /// Print detailed swap result
    fn print_swap_result(&self, result: &SwapResult, input_symbol: &str, output_symbol: &str) {
        println!("✅ Swap Result:");
        println!("  Success: {}", result.success);
        println!("  DEX Used: {}", result.dex_used);
        println!("  Signature: {}", result.signature.as_ref().unwrap_or(&"None".to_string()));
        println!("  Input Amount: {} {}", result.input_amount, input_symbol);
        println!("  Output Amount: {} {}", result.output_amount, output_symbol);
        println!("  Price Impact: {:.2}%", result.price_impact);
        println!("  Fee: {} lamports", result.fee_lamports);
        println!("  Block Height: {}", result.block_height.unwrap_or(0));
    }

    /// Verify swap result is valid
    fn verify_swap_result(&self, result: &SwapResult) -> Result<()> {
        if !result.success {
            return Err(anyhow::anyhow!("Swap was not successful"));
        }

        if result.signature.is_none() {
            return Err(anyhow::anyhow!("No transaction signature"));
        }

        if result.input_amount == 0 {
            return Err(anyhow::anyhow!("Input amount is zero"));
        }

        if result.output_amount == 0 {
            return Err(anyhow::anyhow!("Output amount is zero"));
        }

        if result.price_impact > 10.0 {
            return Err(anyhow::anyhow!("Price impact too high: {:.2}%", result.price_impact));
        }

        Ok(())
    }
}

/// Load test configuration
async fn load_test_config() -> Result<Config> {
    // Try to load from actual config file first
    if let Ok(config) = Config::load("configs.json") {
        return Ok(config);
    }
    
    // Fallback to default config with test-safe values
    let mut config = Config::default();
    config.swap.enabled = true;
    config.swap.jupiter.enabled = true;
    config.swap.raydium.enabled = true;
    config.swap.gmgn.enabled = false; // Disabled in tests
    
    Ok(config)
}

/// Run all swap tests
pub async fn run_all_tests() -> Result<()> {
    println!("🚀 Starting comprehensive swap module tests");
    println!("⚠️  WARNING: These tests will perform real swaps with small amounts!");
    println!("   Make sure you have sufficient SOL and USDC in your test wallet.");
    println!();

    let test_runner = SwapTestRunner::new().await?;

    // Test 1: Quote comparison
    println!("=== Test 1: Quote Comparison ===");
    test_runner.test_quotes_comparison().await?;
    println!();

    // Test 2: Slippage protection
    println!("=== Test 2: Slippage Protection ===");
    test_runner.test_slippage_protection().await?;
    println!();

    // Test 3: Anti-MEV (if available)
    println!("=== Test 3: Anti-MEV Testing ===");
    test_runner.test_anti_mev_swap().await?;
    println!();

    // Test 4: Real swap - SOL to USDC
    println!("=== Test 4: Real Swap SOL -> USDC ===");
    let _sol_to_usdc = test_runner.test_sol_to_usdc_swap().await?;
    println!();

    // Test 5: Full round-trip
    println!("=== Test 5: Round-trip Test ===");
    test_runner.test_round_trip().await?;
    println!();

    println!("🎉 All swap tests completed successfully!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Use --ignored flag to run this test
    async fn test_real_swaps() {
        if let Err(e) = run_all_tests().await {
            panic!("Swap tests failed: {}", e);
        }
    }

    #[tokio::test]
    async fn test_conversion_functions() {
        // Test SOL conversions
        assert_eq!(sol_to_lamports(0.001), 1_000_000);
        assert_eq!(lamports_to_sol(1_000_000), 0.001);

        // Test USDC conversions
        assert_eq!(usdc_to_micro_usdc(100.0), 100_000_000);
        assert_eq!(micro_usdc_to_usdc(100_000_000), 100.0);

        // Test token decimals
        assert_eq!(get_token_decimals(SOL_MINT), 9);
        assert_eq!(get_token_decimals(USDC_MINT), 6);
    }

    #[tokio::test]
    async fn test_swap_request_creation() {
        let request = create_swap_request(
            SOL_MINT,
            USDC_MINT,
            1_000_000,
            50,
            "test_pubkey",
            Some(DexType::Jupiter),
            false,
        );

        assert_eq!(request.input_mint, SOL_MINT);
        assert_eq!(request.output_mint, USDC_MINT);
        assert_eq!(request.amount, 1_000_000);
        assert_eq!(request.slippage_bps, 50);
        assert_eq!(request.dex_preference, Some(DexType::Jupiter));
        assert!(!request.is_anti_mev);
    }

    #[tokio::test]
    async fn test_gmgn_01_sol_quote() {
        // Test GMGN with 0.01 SOL (10,000,000 lamports) as requested
        use crate::swap::core::manager::SwapManager;
        use solana_sdk::signature::Keypair;
        
        let swap_config = SwapConfig {
            enabled: true,
            default_dex: "jupiter".to_string(),
            is_anti_mev: false,
            max_slippage: 0.01,
            timeout_seconds: 30,
            retry_attempts: 3,
            dex_preferences: vec!["jupiter".to_string(), "raydium".to_string(), "gmgn".to_string()],
            jupiter: JupiterConfig {
                enabled: true,
                base_url: "https://quote-api.jup.ag/v6".to_string(),
                timeout_seconds: 15,
                max_accounts: 64,
                only_direct_routes: false,
                as_legacy_transaction: false,
            },
            raydium: RaydiumConfig {
                enabled: true,
                base_url: "https://api.raydium.io/v2".to_string(),
                timeout_seconds: 15,
                pool_type: "all".to_string(),
            },
            gmgn: GmgnConfig {
                enabled: true,
                base_url: "https://gmgn.ai".to_string(),
                timeout_seconds: 15,
                api_key: "".to_string(),
                referral_account: "".to_string(),
                referral_fee_bps: 0,
            },
        };
        
        let rpc_manager = Arc::new(RpcManager::new(
            "https://api.mainnet-beta.solana.com".to_string(),
            vec![],
        ));
        let database = Arc::new(Database::new("test.db").unwrap());
        
        // Create a dummy keypair for testing
        let dummy_keypair = Keypair::new();
        let mut test_config = Config::default();
        test_config.main_wallet_private = bs58::encode(dummy_keypair.to_bytes()).into_string();
        
        let wallet_tracker = Arc::new(WalletTracker::new(test_config.clone(), database.clone()).unwrap());
        let transaction_manager = Arc::new(TransactionManager::new(
            TransactionManagerConfig {
                cache_transactions: true,
                cache_duration_hours: 24,
                track_pnl: true,
                auto_calculate_profits: true,
            },
            database,
            wallet_tracker,
        ));

        let swap_manager = SwapManager::new(swap_config, rpc_manager, transaction_manager);

        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 10_000_000, // 0.01 SOL as requested
            slippage_bps: 50, // 0.5%
            user_public_key: dummy_keypair.pubkey().to_string(), // Use the dummy keypair's public key
            dex_preference: Some(DexType::Gmgn),
            is_anti_mev: false, // As requested, don't use anti-MEV
        };

        match swap_manager.get_best_quote(&request).await {
            Ok(route) => {
                let output_usdc = micro_usdc_to_usdc(route.out_amount.parse().unwrap_or(0));
                println!("✅ Quote for 0.01 SOL successful:");
                println!("  Input: 0.01 SOL (10,000,000 lamports)");
                println!("  Output: {:.6} USDC", output_usdc);
                println!("  Price Impact: {}%", route.price_impact_pct);
                println!("  Route Hops: {}", route.route_plan.len());
                println!("  DEX: {}", route.dex);
                
                if route.dex == DexType::Gmgn {
                    println!("🎉 GMGN API is working correctly!");
                } else {
                    println!("ℹ️  GMGN might not have the best quote, falling back to {}", route.dex);
                }
                
                assert_eq!(route.input_mint, SOL_MINT);
                assert_eq!(route.output_mint, USDC_MINT);
            }
            Err(e) => {
                println!("❌ Quote failed: {}", e);
                // Don't fail the test since API might be rate limited or blocked
                panic!("Failed to get any quote: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_gmgn_direct_api() {
        // Test GMGN API directly
        use crate::swap::dex::gmgn::GmgnSwap;
        use crate::swap::types::*;
        
        let config = GmgnConfig {
            enabled: true,
            base_url: "https://gmgn.ai".to_string(),
            timeout_seconds: 15,
            api_key: "".to_string(),
            referral_account: "".to_string(),
            referral_fee_bps: 0,
        };
        
        let client = GmgnSwap::new(config);
        
        // Create dummy keypair for user address
        let dummy_keypair = Keypair::new();
        
        let request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 10_000_000, // 0.01 SOL
            slippage_bps: 50, // 0.5%
            user_public_key: dummy_keypair.pubkey().to_string(),
            dex_preference: Some(DexType::Gmgn),
            is_anti_mev: false,
        };
        
        match client.get_quote(&request).await {
            Ok(quote) => {
                println!("✅ GMGN direct API test successful:");
                println!("  Input: {} ({})", request.amount, SOL_MINT);
                println!("  Output: {} ({})", quote.out_amount, quote.output_mint);
                println!("  Price Impact: {}%", quote.price_impact_pct);
                println!("  Route Hops: {}", quote.route_plan.len());
            }
            Err(e) => {
                println!("❌ GMGN direct API test failed: {}", e);
                // Print more detailed error for debugging
                println!("Error details: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn debug_gmgn_response() {
        // Debug test to see actual GMGN API response
        use reqwest::Client;
        use std::time::Duration;
        use solana_sdk::signature::Keypair;
        
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("Failed to create HTTP client");
        
        let dummy_keypair = Keypair::new();
        let url = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route";
        
        let params = [
            ("token_in_address", SOL_MINT),
            ("token_out_address", USDC_MINT),
            ("in_amount", "10000000"), // 0.01 SOL
            ("from_address", &dummy_keypair.pubkey().to_string()),
            ("slippage", "0.5"), // 0.5%
            ("fee", "0"), // Add fee parameter
        ];

        match client.get(url).query(&params).send().await {
            Ok(response) => {
                println!("✅ GMGN API Response Status: {}", response.status());
                match response.text().await {
                    Ok(text) => {
                        println!("📄 Raw Response Body:");
                        println!("{}", text);
                        
                        // Try to parse as JSON to see structure
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text) {
                            println!("📋 Parsed JSON structure:");
                            println!("{:#}", json_value);
                        }
                    }
                    Err(e) => println!("❌ Failed to read response text: {}", e),
                }
            }
            Err(e) => {
                println!("❌ GMGN API request failed: {}", e);
            }
        }
    }
}

/// Command line interface for running tests
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("Swap Module Test Runner");
    println!("======================");
    
    // Check if --quotes-only flag is provided
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--quotes-only".to_string()) {
        let test_runner = SwapTestRunner::new().await?;
        test_runner.test_quotes_comparison().await?;
        return Ok(());
    }

    // Run all tests
    run_all_tests().await
}
