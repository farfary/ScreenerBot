use screenerbot::config::Config;
use screenerbot::swap::dex::{ JupiterSwap, GmgnSwap };
use screenerbot::swap::executor::SwapExecutor;
use screenerbot::swap::types::*;
use screenerbot::rpc_manager::RpcManager;
use anyhow::Result;
use std::time::Instant;
use std::sync::Arc;
use solana_sdk::signature::{ Keypair, Signer };

/// Dual DEX comparison structure
#[derive(Debug, Clone)]
pub struct QuoteComparison {
    pub jupiter_quote: Option<SwapRoute>,
    pub gmgn_quote: Option<SwapRoute>,
    pub jupiter_time_ms: u64,
    pub gmgn_time_ms: u64,
    pub jupiter_error: Option<String>,
    pub gmgn_error: Option<String>,
}

/// Execution result for both DEXes
#[derive(Debug, Clone)]
pub struct DualExecutionResult {
    pub jupiter_result: Option<SwapResult>,
    pub gmgn_result: Option<SwapResult>,
    pub jupiter_execution_time_ms: u64,
    pub gmgn_execution_time_ms: u64,
    pub jupiter_error: Option<String>,
    pub gmgn_error: Option<String>,
}

/// Comprehensive swap module that handles both GMGN and Jupiter
pub struct DualDexSwapManager {
    jupiter: JupiterSwap,
    gmgn: GmgnSwap,
    executor: SwapExecutor,
    rpc_manager: Arc<RpcManager>,
    keypair: Keypair,
}

impl DualDexSwapManager {
    pub fn new(config: &Config, rpc_manager: Arc<RpcManager>, keypair: Keypair) -> Self {
        let jupiter = JupiterSwap::new(config.swap.jupiter.clone());
        let gmgn = GmgnSwap::new(config.swap.gmgn.clone());
        let executor = SwapExecutor::new(rpc_manager.clone(), keypair.insecure_clone());

        Self {
            jupiter,
            gmgn,
            executor,
            rpc_manager,
            keypair,
        }
    }

    /// Get quotes from both DEXes and compare them
    pub async fn get_dual_quotes(&self, request: &SwapRequest) -> Result<QuoteComparison> {
        println!("📊 Getting quotes from both Jupiter and GMGN...");

        // Get Jupiter quote
        let jupiter_start = Instant::now();
        let (jupiter_quote, jupiter_error) = match self.jupiter.get_quote(request).await {
            Ok(quote) => (Some(quote), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let jupiter_time_ms = jupiter_start.elapsed().as_millis() as u64;

        // Get GMGN quote
        let gmgn_start = Instant::now();
        let (gmgn_quote, gmgn_error) = match self.gmgn.get_quote(request).await {
            Ok(quote) => (Some(quote), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let gmgn_time_ms = gmgn_start.elapsed().as_millis() as u64;

        Ok(QuoteComparison {
            jupiter_quote,
            gmgn_quote,
            jupiter_time_ms,
            gmgn_time_ms,
            jupiter_error,
            gmgn_error,
        })
    }

    /// Execute swaps on both DEXes for comparison
    pub async fn execute_dual_swaps(&self, request: &SwapRequest) -> Result<DualExecutionResult> {
        println!("🚀 Executing swaps on both Jupiter and GMGN...");

        // Execute Jupiter swap
        let jupiter_start = Instant::now();
        let (jupiter_result, jupiter_error) = match self.execute_jupiter_swap(request).await {
            Ok(result) => (Some(result), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let jupiter_execution_time_ms = jupiter_start.elapsed().as_millis() as u64;

        // Execute GMGN swap
        let gmgn_start = Instant::now();
        let (gmgn_result, gmgn_error) = match self.execute_gmgn_swap(request).await {
            Ok(result) => (Some(result), None),
            Err(e) => (None, Some(e.to_string())),
        };
        let gmgn_execution_time_ms = gmgn_start.elapsed().as_millis() as u64;

        Ok(DualExecutionResult {
            jupiter_result,
            gmgn_result,
            jupiter_execution_time_ms,
            gmgn_execution_time_ms,
            jupiter_error,
            gmgn_error,
        })
    }

    /// Execute swap specifically with Jupiter
    async fn execute_jupiter_swap(&self, request: &SwapRequest) -> Result<SwapResult> {
        println!("🪐 Executing Jupiter swap...");
        println!("  🔍 DEBUG: Starting Jupiter execution process");
        println!(
            "  📝 DEBUG: Request details - Input: {}, Output: {}, Amount: {}",
            request.input_mint,
            request.output_mint,
            request.amount
        );

        // Get quote first
        println!("  📊 DEBUG: Getting Jupiter quote...");
        let route = match self.jupiter.get_quote(request).await {
            Ok(route) => {
                println!("  ✅ DEBUG: Jupiter quote successful");
                route
            }
            Err(e) => {
                println!("  ❌ DEBUG: Jupiter quote failed: {}", e);
                return Err(e.into());
            }
        };

        println!(
            "  📈 Jupiter quote: {} → {} ({}% impact)",
            request.amount,
            route.out_amount,
            route.price_impact_pct
        );

        // Get swap transaction
        println!("  🔧 DEBUG: Getting Jupiter swap transaction...");
        println!("  🔑 DEBUG: Using wallet address: {}", request.user_public_key);
        let swap_transaction = match
            self.jupiter.get_swap_transaction(&route, &request.user_public_key).await
        {
            Ok(tx) => {
                println!("  ✅ DEBUG: Jupiter transaction prepared successfully");
                println!(
                    "  📋 DEBUG: Transaction data length: {} bytes",
                    tx.swap_transaction.len()
                );
                tx
            }
            Err(e) => {
                println!("  ❌ DEBUG: Jupiter transaction preparation failed: {}", e);
                return Err(e.into());
            }
        };
        println!("  🔗 Jupiter transaction prepared");

        // Execute transaction
        println!("  🚀 DEBUG: Executing Jupiter transaction...");
        println!("  🌐 DEBUG: RPC manager status check...");

        let result = match self.executor.execute_swap(&swap_transaction, &route).await {
            Ok(result) => {
                println!("  ✅ DEBUG: Jupiter execution completed successfully");
                result
            }
            Err(e) => {
                println!("  ❌ DEBUG: Jupiter execution failed with error: {}", e);
                println!("  🔍 DEBUG: Error type: {:?}", e);

                // Check if it's an RPC error specifically
                let error_string = e.to_string();
                if error_string.contains("RPC") || error_string.contains("endpoint") {
                    println!("  🌐 DEBUG: This appears to be an RPC connectivity issue");
                    println!("  🔧 DEBUG: Checking RPC endpoint health...");
                }

                return Err(e);
            }
        };

        println!("  ✅ Jupiter execution successful!");
        Ok(result)
    }

    /// Execute swap specifically with GMGN
    async fn execute_gmgn_swap(&self, request: &SwapRequest) -> Result<SwapResult> {
        println!("🎯 Executing GMGN swap...");
        println!("  🔍 DEBUG: Starting GMGN execution process");
        println!(
            "  📝 DEBUG: Request details - Input: {}, Output: {}, Amount: {}",
            request.input_mint,
            request.output_mint,
            request.amount
        );

        // Get quote first
        println!("  📊 DEBUG: Getting GMGN quote...");
        let route = match self.gmgn.get_quote(request).await {
            Ok(route) => {
                println!("  ✅ DEBUG: GMGN quote successful");
                route
            }
            Err(e) => {
                println!("  ❌ DEBUG: GMGN quote failed: {}", e);
                return Err(e.into());
            }
        };

        println!(
            "  📈 GMGN quote: {} → {} ({}% impact)",
            request.amount,
            route.out_amount,
            route.price_impact_pct
        );

        // Get swap transaction
        println!("  🔧 DEBUG: Getting GMGN swap transaction...");
        println!("  🔑 DEBUG: Using wallet address: {}", request.user_public_key);
        let swap_transaction = match
            self.gmgn.get_swap_transaction(&route, &request.user_public_key).await
        {
            Ok(tx) => {
                println!("  ✅ DEBUG: GMGN transaction prepared successfully");
                println!(
                    "  📋 DEBUG: Transaction data length: {} bytes",
                    tx.swap_transaction.len()
                );
                tx
            }
            Err(e) => {
                println!("  ❌ DEBUG: GMGN transaction preparation failed: {}", e);
                return Err(e.into());
            }
        };
        println!("  🔗 GMGN transaction prepared");

        // Execute transaction
        println!("  🚀 DEBUG: Executing GMGN transaction...");
        println!("  🌐 DEBUG: RPC manager status check...");

        let result = match self.executor.execute_swap(&swap_transaction, &route).await {
            Ok(result) => {
                println!("  ✅ DEBUG: GMGN execution completed successfully");
                result
            }
            Err(e) => {
                println!("  ❌ DEBUG: GMGN execution failed with error: {}", e);
                println!("  🔍 DEBUG: Error type: {:?}", e);

                // Check if it's an RPC error specifically
                let error_string = e.to_string();
                if error_string.contains("RPC") || error_string.contains("endpoint") {
                    println!("  🌐 DEBUG: This appears to be an RPC connectivity issue");
                    println!("  🔧 DEBUG: Checking RPC endpoint health...");
                }

                return Err(e);
            }
        };

        println!("  ✅ GMGN execution successful!");
        Ok(result)
    }

    /// Get wallet public key
    pub fn get_wallet_address(&self) -> String {
        self.keypair.pubkey().to_string()
    }

    /// Check if both DEXes are available
    pub async fn health_check(&self) -> (bool, bool) {
        println!("  🔍 DEBUG: Starting DEX health checks...");

        let test_request = SwapRequest {
            input_mint: SOL_MINT.to_string(),
            output_mint: USDC_MINT.to_string(),
            amount: 1000000, // 0.001 SOL
            slippage_bps: 50,
            user_public_key: self.get_wallet_address(),
            dex_preference: None,
            is_anti_mev: false,
        };

        println!("  🪐 DEBUG: Testing Jupiter health...");
        let jupiter_health = match self.jupiter.get_quote(&test_request).await {
            Ok(_) => {
                println!("  ✅ DEBUG: Jupiter health check passed");
                true
            }
            Err(e) => {
                println!("  ❌ DEBUG: Jupiter health check failed: {}", e);
                false
            }
        };

        println!("  🎯 DEBUG: Testing GMGN health...");
        let gmgn_health = match self.gmgn.get_quote(&test_request).await {
            Ok(_) => {
                println!("  ✅ DEBUG: GMGN health check passed");
                true
            }
            Err(e) => {
                println!("  ❌ DEBUG: GMGN health check failed: {}", e);
                false
            }
        };

        (jupiter_health, gmgn_health)
    }

    /// Check RPC endpoint health
    pub async fn check_rpc_health(&self) -> Result<()> {
        println!("🌐 DEBUG: Checking RPC endpoint health...");

        use solana_sdk::pubkey::Pubkey;
        use std::str::FromStr;

        let test_pubkey = Pubkey::from_str(&self.get_wallet_address())?;

        match self.rpc_manager.get_account(&test_pubkey).await {
            Ok(account) => {
                println!(
                    "  ✅ DEBUG: RPC health check passed - Account found with {} lamports",
                    account.lamports
                );
                Ok(())
            }
            Err(e) => {
                println!("  ❌ DEBUG: RPC health check failed: {}", e);
                Err(e)
            }
        }
    }
}

/// Format comparison results
fn print_quote_comparison(comparison: &QuoteComparison, request: &SwapRequest) {
    println!("\n=== QUOTE COMPARISON ===");
    println!("📝 Request: {} {} → {}", request.amount, request.input_mint, request.output_mint);

    // Jupiter results
    if let Some(ref jupiter) = comparison.jupiter_quote {
        println!("🪐 Jupiter:");
        println!("  📈 Output: {} tokens", jupiter.out_amount);
        println!("  💥 Price Impact: {}%", jupiter.price_impact_pct);
        println!("  ⏱️  Quote Time: {}ms", comparison.jupiter_time_ms);
    } else if let Some(ref error) = comparison.jupiter_error {
        println!("🪐 Jupiter: ❌ Error - {}", error);
    }

    // GMGN results
    if let Some(ref gmgn) = comparison.gmgn_quote {
        println!("🎯 GMGN:");
        println!("  📈 Output: {} tokens", gmgn.out_amount);
        println!("  💥 Price Impact: {}%", gmgn.price_impact_pct);
        println!("  ⏱️  Quote Time: {}ms", comparison.gmgn_time_ms);
    } else if let Some(ref error) = comparison.gmgn_error {
        println!("🎯 GMGN: ❌ Error - {}", error);
    }

    // Compare outputs if both succeeded
    if
        let (Some(ref jupiter), Some(ref gmgn)) = (
            &comparison.jupiter_quote,
            &comparison.gmgn_quote,
        )
    {
        let jupiter_amount: u64 = jupiter.out_amount.parse().unwrap_or(0);
        let gmgn_amount: u64 = gmgn.out_amount.parse().unwrap_or(0);

        if jupiter_amount > gmgn_amount {
            let diff = jupiter_amount - gmgn_amount;
            let percent = ((diff as f64) / (gmgn_amount as f64)) * 100.0;
            println!("🏆 Jupiter offers {:.2}% more tokens (+{} tokens)", percent, diff);
        } else if gmgn_amount > jupiter_amount {
            let diff = gmgn_amount - jupiter_amount;
            let percent = ((diff as f64) / (jupiter_amount as f64)) * 100.0;
            println!("🏆 GMGN offers {:.2}% more tokens (+{} tokens)", percent, diff);
        } else {
            println!("🤝 Both DEXes offer the same amount");
        }
    }
}

/// Check wallet balance
async fn check_wallet_balance(rpc_manager: &RpcManager, wallet_address: &str) -> Result<f64> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    let pubkey = Pubkey::from_str(wallet_address)?;
    let account = rpc_manager.get_account(&pubkey).await?;
    Ok((account.lamports as f64) / 1_000_000_000.0)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("🚀 Starting Dual DEX Swap Comparison Test...\n");
    println!("⚡ This test compares Jupiter and GMGN quotes and executions");
    println!("💰 Testing with small amounts on Solana mainnet\n");

    // Load configuration
    let config = Config::load("configs.json").map_err(|e| {
        anyhow::anyhow!("Failed to load config: {}", e)
    })?;

    // Initialize RPC manager
    let rpc_endpoints: Vec<String> = vec![config.rpc_url.clone()]
        .into_iter()
        .chain(config.rpc_fallbacks.clone())
        .collect();

    println!("🌐 DEBUG: Configured RPC endpoints:");
    for (i, endpoint) in rpc_endpoints.iter().enumerate() {
        println!("  {}. {}", i + 1, endpoint);
    }

    let rpc_manager = Arc::new(RpcManager::new(rpc_endpoints)?);

    // Create keypair from private key
    let keypair = Keypair::from_base58_string(&config.main_wallet_private);

    // Initialize dual DEX manager
    let dual_manager = DualDexSwapManager::new(&config, rpc_manager.clone(), keypair);
    let wallet_address = dual_manager.get_wallet_address();

    println!("📍 Using wallet: {}", wallet_address);

    // Check wallet balance
    match check_wallet_balance(&rpc_manager, &wallet_address).await {
        Ok(balance_sol) => {
            println!("💰 Current SOL balance: {:.6} SOL\n", balance_sol);
            if balance_sol < 0.01 {
                println!(
                    "⚠️  WARNING: Low SOL balance! You may not have enough for transactions and fees.\n"
                );
            }
        }
        Err(e) => {
            println!("⚠️  Could not check wallet balance: {}\n", e);
        }
    }

    // Health check
    println!("🔍 Checking DEX availability...");
    let (jupiter_health, gmgn_health) = dual_manager.health_check().await;
    println!("🪐 Jupiter: {}", if jupiter_health { "✅ Available" } else { "❌ Unavailable" });
    println!("🎯 GMGN: {}\n", if gmgn_health { "✅ Available" } else { "❌ Unavailable" });

    if !jupiter_health && !gmgn_health {
        println!("❌ Both DEXes are unavailable. Exiting.");
        return Ok(());
    }

    // Test parameters
    let test_amount = 1_000_000u64; // 0.001 SOL
    let slippage_bps = 50; // 0.5%

    println!(
        "💰 Test amount: {} lamports ({:.6} SOL)",
        test_amount,
        (test_amount as f64) / 1_000_000_000.0
    );
    println!("🎯 Slippage: {}bps ({:.2}%)\n", slippage_bps, (slippage_bps as f64) / 100.0);

    // Test 1: Quote Comparison - SOL to BONK
    println!("=== TEST 1: QUOTE COMPARISON (SOL → BONK) ===");
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

    let sol_to_bonk_request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: bonk_mint.to_string(),
        amount: test_amount,
        slippage_bps,
        user_public_key: wallet_address.clone(),
        dex_preference: None,
        is_anti_mev: false,
    };

    let sol_bonk_comparison = dual_manager.get_dual_quotes(&sol_to_bonk_request).await?;
    print_quote_comparison(&sol_bonk_comparison, &sol_to_bonk_request);

    println!("\n⏳ Waiting 3 seconds...\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test 2: Quote Comparison - SOL to USDC
    println!("=== TEST 2: QUOTE COMPARISON (SOL → USDC) ===");

    let sol_to_usdc_request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: test_amount,
        slippage_bps,
        user_public_key: wallet_address.clone(),
        dex_preference: None,
        is_anti_mev: false,
    };

    let sol_usdc_comparison = dual_manager.get_dual_quotes(&sol_to_usdc_request).await?;
    print_quote_comparison(&sol_usdc_comparison, &sol_to_usdc_request);

    println!("\n⏳ Waiting 3 seconds...\n");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Test 3: Execution Comparison (small amounts)
    println!("=== TEST 3: EXECUTION COMPARISON (REAL TRANSACTIONS) ===");
    println!("🚨 This will execute REAL transactions with 0.001 SOL each");

    // Check RPC health before executing transactions
    println!("🔍 DEBUG: Performing pre-execution RPC health check...");
    if let Err(e) = dual_manager.check_rpc_health().await {
        println!("⚠️  WARNING: RPC health check failed: {}", e);
        println!("🔧 This might cause execution failures. Continuing anyway...");
    }

    let exec_amount = 1_000_000u64; // 0.001 SOL for real execution
    let exec_request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: bonk_mint.to_string(),
        amount: exec_amount,
        slippage_bps,
        user_public_key: wallet_address.clone(),
        dex_preference: None,
        is_anti_mev: false,
    };

    // Execute on both DEXes
    let execution_results = dual_manager.execute_dual_swaps(&exec_request).await?;

    println!("\n📊 EXECUTION RESULTS:");
    if let Some(ref jupiter_result) = execution_results.jupiter_result {
        println!("🪐 Jupiter:");
        println!(
            "  ✅ Success - Signature: {}",
            jupiter_result.signature.as_ref().unwrap_or(&"N/A".to_string())
        );
        println!("  ⏱️  Execution Time: {}ms", execution_results.jupiter_execution_time_ms);
        println!(
            "  🔗 Explorer: https://solscan.io/tx/{}",
            jupiter_result.signature.as_ref().unwrap_or(&"".to_string())
        );
    } else if let Some(ref error) = execution_results.jupiter_error {
        println!("🪐 Jupiter: ❌ Failed - {}", error);
    }

    if let Some(ref gmgn_result) = execution_results.gmgn_result {
        println!("🎯 GMGN:");
        println!(
            "  ✅ Success - Signature: {}",
            gmgn_result.signature.as_ref().unwrap_or(&"N/A".to_string())
        );
        println!("  ⏱️  Execution Time: {}ms", execution_results.gmgn_execution_time_ms);
        println!(
            "  🔗 Explorer: https://solscan.io/tx/{}",
            gmgn_result.signature.as_ref().unwrap_or(&"".to_string())
        );
    } else if let Some(ref error) = execution_results.gmgn_error {
        println!("🎯 GMGN: ❌ Failed - {}", error);
    }

    // Final balance check
    println!("\n=== FINAL BALANCE CHECK ===");
    match check_wallet_balance(&rpc_manager, &wallet_address).await {
        Ok(balance_sol) => {
            println!("💰 Final SOL balance: {:.6} SOL", balance_sol);
        }
        Err(e) => {
            println!("⚠️  Could not check final balance: {}", e);
        }
    }

    println!("\n✅ Dual DEX comparison test completed!");
    Ok(())
}
