use screenerbot::config::Config;
use screenerbot::swap::dex::{ JupiterSwap, GmgnSwap };
use screenerbot::swap::executor::SwapExecutor;
use screenerbot::swap::types::*;
use screenerbot::rpc_manager::RpcManager;
use anyhow::Result;
use std::time::Instant;
use std::sync::Arc;
use solana_sdk::signature::{ Keypair, Signer };
use log::{ info, warn, error };

const TEST_TOKENS: &[(&str, &str)] = &[
    ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"),
    ("WIF", "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm"),
    ("JUP", "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN"),
];

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    println!("🚀 Starting REAL transaction execution testing...\n");
    println!("⚠️  WARNING: This will execute REAL transactions on Solana mainnet!");
    println!("💰 Make sure you have enough SOL in your wallet for transactions and fees.\n");

    // Load configuration
    let config = Config::load("configs.json").map_err(|e| {
        anyhow::anyhow!("Failed to load config: {}", e)
    })?;

    // Initialize RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(
            vec![config.rpc_url.clone()].into_iter().chain(config.rpc_fallbacks.clone()).collect()
        )?
    );

    // Create keypair from private key
    let keypair = Keypair::from_base58_string(&config.main_wallet_private);
    let wallet_pubkey = keypair.pubkey().to_string();

    println!("📍 Using wallet: {}", wallet_pubkey);

    // Check wallet balance before starting
    match check_wallet_balance(&rpc_manager, &wallet_pubkey).await {
        Ok(balance_sol) => {
            println!("💰 Current SOL balance: {:.6} SOL\n", balance_sol);
            if balance_sol < 0.01 {
                println!(
                    "⚠️  WARNING: Low SOL balance! You may not have enough for transactions and fees."
                );
            }
        }
        Err(e) => {
            println!("⚠️  Could not check wallet balance: {}", e);
        }
    }

    // Initialize DEX clients
    let jupiter = JupiterSwap::new(config.swap.jupiter.clone());
    let gmgn = GmgnSwap::new(config.swap.gmgn.clone());

    // Initialize swap executor
    let executor = SwapExecutor::new(rpc_manager.clone(), keypair);

    // Test amount: 0.001 SOL = 1,000,000 lamports
    let test_amount = 1_000_000u64;
    let slippage_bps = 50; // 0.5% slippage for lowest cost

    println!(
        "💰 Test amount: {} lamports ({:.6} SOL)",
        test_amount,
        (test_amount as f64) / 1_000_000_000.0
    );
    println!("🎯 Slippage: {}bps ({}%)", slippage_bps, (slippage_bps as f64) / 100.0);
    println!("🔒 Anti-MEV: disabled (for lowest cost)\n");

    println!("🚀 Starting automatic test execution...\n");

    // Test 1: Small SOL to BONK swap (and back)
    println!("=== TEST 1: REAL SOL → BONK → SOL ROUND TRIP ===");
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

    // SOL to BONK
    let bonk_amount = execute_real_swap(
        &jupiter,
        &gmgn,
        &executor,
        SOL_MINT,
        bonk_mint,
        "SOL",
        "BONK",
        test_amount,
        slippage_bps,
        &wallet_pubkey
    ).await?;

    if let Some(bonk_received) = bonk_amount {
        println!("✅ Received {} BONK tokens", bonk_received);

        // Wait a moment for settlement
        println!("⏳ Waiting 5 seconds for settlement...");
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // BONK back to SOL
        println!("\n--- Converting BONK back to SOL ---");
        execute_real_swap(
            &jupiter,
            &gmgn,
            &executor,
            bonk_mint,
            SOL_MINT,
            "BONK",
            "SOL",
            bonk_received,
            slippage_bps,
            &wallet_pubkey
        ).await?;
    } else {
        println!("❌ First swap failed, skipping round trip");
    }

    // Test 2: Compare execution times between DEXes
    println!("\n=== TEST 2: DEX PERFORMANCE COMPARISON ===");
    compare_dex_performance(&jupiter, &gmgn, &executor, &wallet_pubkey).await?;

    // Final balance check
    println!("\n=== FINAL BALANCE CHECK ===");
    match check_wallet_balance(&rpc_manager, &wallet_pubkey).await {
        Ok(final_balance) => {
            println!("💰 Final SOL balance: {:.6} SOL", final_balance);
        }
        Err(e) => {
            println!("⚠️  Could not check final balance: {}", e);
        }
    }

    println!("\n✅ Real transaction testing completed!");
    Ok(())
}

async fn execute_real_swap(
    jupiter: &JupiterSwap,
    gmgn: &GmgnSwap,
    executor: &SwapExecutor,
    input_mint: &str,
    output_mint: &str,
    input_symbol: &str,
    output_symbol: &str,
    amount: u64,
    slippage_bps: u32,
    wallet_pubkey: &str
) -> Result<Option<u64>> {
    println!("🔄 Executing {} → {} swap...", input_symbol, output_symbol);

    let request = SwapRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        amount,
        slippage_bps,
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: None,
        is_anti_mev: false,
    };

    // Try Jupiter first
    if jupiter.is_enabled() {
        println!("🪐 Attempting with Jupiter...");
        match get_quote_and_execute(jupiter, executor, &request, "Jupiter").await {
            Ok(result) => {
                if result.success {
                    println!("✅ Jupiter execution successful!");
                    print_swap_result(&result);
                    return Ok(Some(result.output_amount));
                } else {
                    println!("❌ Jupiter execution failed: {:?}", result.error);
                }
            }
            Err(e) => {
                println!("❌ Jupiter failed: {}", e);
            }
        }
    }

    // Try GMGN as fallback
    if gmgn.is_enabled() {
        println!("🎯 Attempting with GMGN...");
        match get_quote_and_execute(gmgn, executor, &request, "GMGN").await {
            Ok(result) => {
                if result.success {
                    println!("✅ GMGN execution successful!");
                    print_swap_result(&result);
                    return Ok(Some(result.output_amount));
                } else {
                    println!("❌ GMGN execution failed: {:?}", result.error);
                }
            }
            Err(e) => {
                println!("❌ GMGN failed: {}", e);
            }
        }
    }

    println!("❌ Both DEXes failed for this swap");
    Ok(None)
}

async fn get_quote_and_execute<T>(
    dex: &T,
    executor: &SwapExecutor,
    request: &SwapRequest,
    dex_name: &str
) -> Result<SwapResult>
    where T: DexSwap + Send + Sync
{
    let start_time = Instant::now();

    // Get quote
    let route = dex.get_quote(request).await?;
    let quote_time = start_time.elapsed();

    println!("  📈 Quote: {} → {} (in {:?})", route.in_amount, route.out_amount, quote_time);
    println!("  💥 Price impact: {}%", route.price_impact_pct);

    // Get transaction
    let swap_tx = dex.get_swap_transaction(&route, &request.user_public_key).await?;
    let tx_prep_time = start_time.elapsed();

    println!("  🔗 Transaction prepared (in {:?})", tx_prep_time);

    // Execute transaction
    println!("  🚀 Executing transaction...");
    let result = executor.execute_swap(&swap_tx, &route).await?;

    Ok(result)
}

async fn compare_dex_performance(
    jupiter: &JupiterSwap,
    gmgn: &GmgnSwap,
    executor: &SwapExecutor,
    wallet_pubkey: &str
) -> Result<()> {
    println!("Comparing quote speeds (no execution)...");

    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: 1_000_000, // 0.001 SOL
        slippage_bps: 50,
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: None,
        is_anti_mev: false,
    };

    // Test Jupiter speed
    if jupiter.is_enabled() {
        let start = Instant::now();
        match jupiter.get_quote(&request).await {
            Ok(route) => {
                let duration = start.elapsed();
                println!("🪐 Jupiter quote: {} USDC in {:?}", route.out_amount, duration);
            }
            Err(e) => {
                println!("🪐 Jupiter quote failed: {}", e);
            }
        }
    }

    // Test GMGN speed
    if gmgn.is_enabled() {
        let start = Instant::now();
        match gmgn.get_quote(&request).await {
            Ok(route) => {
                let duration = start.elapsed();
                println!("🎯 GMGN quote: {} USDC in {:?}", route.out_amount, duration);
            }
            Err(e) => {
                println!("🎯 GMGN quote failed: {}", e);
            }
        }
    }

    Ok(())
}

async fn check_wallet_balance(rpc_manager: &Arc<RpcManager>, wallet_pubkey: &str) -> Result<f64> {
    let pubkey: solana_sdk::pubkey::Pubkey = wallet_pubkey.parse()?;
    let balance_lamports = rpc_manager.get_balance(&pubkey).await?;
    Ok((balance_lamports as f64) / 1_000_000_000.0)
}

fn print_swap_result(result: &SwapResult) {
    println!("📊 Swap Result:");
    println!("  🎯 DEX: {}", result.dex_used);
    println!("  💰 Input: {} lamports", result.input_amount);
    println!("  💰 Output: {} lamports", result.output_amount);
    println!("  💥 Price Impact: {:.6}%", result.price_impact);
    println!("  🎯 Slippage: {:.3}%", result.slippage * 100.0);
    println!("  💸 Fee: {} lamports", result.fee_lamports);
    println!("  ⏱️  Execution Time: {}ms", result.execution_time_ms);
    if let Some(signature) = &result.signature {
        println!("  🔗 Signature: {}", signature);
        println!("  🌐 Explorer: https://solscan.io/tx/{}", signature);
    }
    if let Some(block_height) = result.block_height {
        println!("  📦 Block Height: {}", block_height);
    }
}

// Trait to abstract DEX operations
trait DexSwap {
    async fn get_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError>;
    async fn get_swap_transaction(
        &self,
        route: &SwapRoute,
        user_public_key: &str
    ) -> Result<SwapTransaction, SwapError>;
    fn is_enabled(&self) -> bool;
}

impl DexSwap for JupiterSwap {
    async fn get_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        self.get_quote(request).await
    }

    async fn get_swap_transaction(
        &self,
        route: &SwapRoute,
        user_public_key: &str
    ) -> Result<SwapTransaction, SwapError> {
        self.get_swap_transaction(route, user_public_key).await
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled()
    }
}

impl DexSwap for GmgnSwap {
    async fn get_quote(&self, request: &SwapRequest) -> Result<SwapRoute, SwapError> {
        self.get_quote(request).await
    }

    async fn get_swap_transaction(
        &self,
        route: &SwapRoute,
        user_public_key: &str
    ) -> Result<SwapTransaction, SwapError> {
        self.get_swap_transaction(route, user_public_key).await
    }

    fn is_enabled(&self) -> bool {
        self.is_enabled()
    }
}
