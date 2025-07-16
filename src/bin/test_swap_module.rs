use anyhow::Result;
use screenerbot::{
    config::Config,
    rpc::RpcManager,
    swap::{SwapManager, SwapProvider, create_swap_request},
};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
};
use std::{str::FromStr, sync::Arc, time::Instant};

// Well-known token mints
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

#[derive(Debug)]
struct SwapComparison {
    provider: SwapProvider,
    sol_to_usdc_signature: Option<Signature>,
    usdc_received: u64,
    usdc_to_sol_signature: Option<Signature>, 
    sol_received: u64,
    total_execution_time_ms: u64,
    error: Option<String>,
}

impl SwapComparison {
    fn new(provider: SwapProvider) -> Self {
        Self {
            provider,
            sol_to_usdc_signature: None,
            usdc_received: 0,
            usdc_to_sol_signature: None,
            sol_received: 0,
            total_execution_time_ms: 0,
            error: None,
        }
    }

    fn calculate_performance_percentage(&self, initial_amount: u64) -> f64 {
        if self.sol_received == 0 {
            return -100.0; // Total loss
        }
        let difference = self.sol_received as f64 - initial_amount as f64;
        (difference / initial_amount as f64) * 100.0
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("🚀 Starting Swap Module Test");
    println!("Testing 0.01 SOL → USDC → SOL roundtrip with Jupiter and GMGN");
    println!("{}", "=".repeat(80));

    // Load configuration
    let config = Config::load("configs.json")?;
    println!("✅ Configuration loaded");

    // Create RPC manager
    let rpc_manager = Arc::new(RpcManager::new(
        config.rpc_url.clone(),
        config.rpc_fallbacks.clone(),
        config.rpc.clone(),
    )?);
    println!("✅ RPC Manager initialized");

    // Create swap manager
    let swap_manager = SwapManager::new(config.swap.clone(), rpc_manager.clone());
    println!("✅ Swap Manager initialized");

    // Parse wallet keypair
    let keypair_bytes = bs58::decode(&config.main_wallet_private)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Failed to decode private key: {}", e))?;
    let keypair = Keypair::try_from(&keypair_bytes[..])?;
    println!("✅ Wallet loaded: {}", keypair.pubkey());

    // Test amount: 0.01 SOL (10,000,000 lamports) - enough to get meaningful USDC amount
    let sol_amount = 10_000_000u64; // 0.01 SOL in lamports
    
    println!("\n💰 Initial amount: {} lamports (0.01 SOL)", sol_amount);

    // Parse mint addresses
    let sol_mint = Pubkey::from_str(SOL_MINT)?;
    let usdc_mint = Pubkey::from_str(USDC_MINT)?;

    // Test with Jupiter
    println!("\n🪐 Testing Jupiter...");
    let jupiter_result = test_roundtrip_swap(
        &swap_manager,
        &keypair,
        sol_mint,
        usdc_mint,
        sol_amount,
        SwapProvider::Jupiter,
    ).await;

    // Test with GMGN
    println!("\n🎯 Testing GMGN...");
    let gmgn_result = test_roundtrip_swap(
        &swap_manager,
        &keypair,
        sol_mint,
        usdc_mint,
        sol_amount,
        SwapProvider::Gmgn,
    ).await;

    // Compare results
    println!("\n📊 COMPARISON RESULTS");
    println!("{}", "=".repeat(80));
    
    display_swap_result("Jupiter", &jupiter_result, sol_amount);
    display_swap_result("GMGN", &gmgn_result, sol_amount);

    // Determine the best provider
    determine_best_provider(&jupiter_result, &gmgn_result, sol_amount);

    Ok(())
}

async fn test_roundtrip_swap(
    swap_manager: &SwapManager,
    keypair: &Keypair,
    sol_mint: Pubkey,
    usdc_mint: Pubkey,
    sol_amount: u64,
    provider: SwapProvider,
) -> SwapComparison {
    let mut result = SwapComparison::new(provider);
    let start_time = Instant::now();

    // Step 1: SOL -> USDC
    println!("  Step 1: {} lamports SOL → USDC", sol_amount);
    
    let sol_to_usdc_request = create_swap_request(
        sol_mint,
        usdc_mint,
        sol_amount,
        keypair.pubkey(),
        Some(50), // 0.5% slippage
        Some(provider),
    );

    match swap_manager.get_all_quotes(&sol_to_usdc_request).await.get(&provider) {
        Some(Ok(quote)) => {
            println!("    Quote: {} USDC expected", quote.out_amount);
            result.usdc_received = quote.out_amount;
            
            match swap_manager.execute_swap(&sol_to_usdc_request, quote, keypair).await {
                Ok(execution_result) => {
                    result.sol_to_usdc_signature = Some(execution_result.signature);
                    result.usdc_received = execution_result.output_amount;
                    println!("    ✅ Swap executed: {}", execution_result.signature);
                    println!("    💰 Received: {} USDC", result.usdc_received);
                    
                    // Wait a moment for the transaction to settle
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    
                    // Step 2: USDC -> SOL
                    println!("  Step 2: {} USDC → SOL", result.usdc_received);
                    
                    let usdc_to_sol_request = create_swap_request(
                        usdc_mint,
                        sol_mint,
                        result.usdc_received,
                        keypair.pubkey(),
                        Some(50), // 0.5% slippage
                        Some(provider),
                    );

                    match swap_manager.get_all_quotes(&usdc_to_sol_request).await.get(&provider) {
                        Some(Ok(quote2)) => {
                            println!("    Quote: {} lamports SOL expected", quote2.out_amount);
                            
                            match swap_manager.execute_swap(&usdc_to_sol_request, quote2, keypair).await {
                                Ok(execution_result2) => {
                                    result.usdc_to_sol_signature = Some(execution_result2.signature);
                                    result.sol_received = execution_result2.output_amount;
                                    println!("    ✅ Swap executed: {}", execution_result2.signature);
                                    println!("    💰 Received: {} lamports SOL", result.sol_received);
                                }
                                Err(e) => {
                                    result.error = Some(format!("USDC->SOL swap execution failed: {}", e));
                                    println!("    ❌ Swap execution failed: {}", e);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            result.error = Some(format!("USDC->SOL quote failed: {}", e));
                            println!("    ❌ Quote failed: {}", e);
                        }
                        None => {
                            result.error = Some("No quote available for USDC->SOL".to_string());
                            println!("    ❌ No quote available");
                        }
                    }
                }
                Err(e) => {
                    result.error = Some(format!("SOL->USDC swap execution failed: {}", e));
                    println!("    ❌ Swap execution failed: {}", e);
                }
            }
        }
        Some(Err(e)) => {
            result.error = Some(format!("SOL->USDC quote failed: {}", e));
            println!("    ❌ Quote failed: {}", e);
        }
        None => {
            result.error = Some("No quote available for SOL->USDC".to_string());
            println!("    ❌ No quote available");
        }
    }

    result.total_execution_time_ms = start_time.elapsed().as_millis() as u64;
    result
}

fn display_swap_result(provider_name: &str, result: &SwapComparison, initial_amount: u64) {
    println!("\n{} Results:", provider_name);
    println!("  Provider: {}", result.provider);
    
    if let Some(error) = &result.error {
        println!("  ❌ Error: {}", error);
        return;
    }
    
    println!("  SOL → USDC:");
    if let Some(sig) = result.sol_to_usdc_signature {
        println!("    Signature: {}", sig);
    }
    println!("    USDC Received: {}", result.usdc_received);
    
    println!("  USDC → SOL:");
    if let Some(sig) = result.usdc_to_sol_signature {
        println!("    Signature: {}", sig);
    }
    println!("    SOL Received: {} lamports", result.sol_received);
    
    let performance = result.calculate_performance_percentage(initial_amount);
    if performance >= 0.0 {
        let gain_amount = result.sol_received - initial_amount;
        println!("  💰 Gain: {:.4}% ({} lamports)", performance, gain_amount);
    } else {
        let loss_amount = initial_amount - result.sol_received;
        println!("  �� Loss: {:.4}% ({} lamports)", -performance, loss_amount);
    }
    println!("  ⏱️  Total Time: {} ms", result.total_execution_time_ms);
}

fn determine_best_provider(jupiter: &SwapComparison, gmgn: &SwapComparison, initial_amount: u64) {
    println!("\n🏆 BEST PROVIDER ANALYSIS");
    println!("{}", "=".repeat(50));
    
    let jupiter_has_error = jupiter.error.is_some();
    let gmgn_has_error = gmgn.error.is_some();
    
    if jupiter_has_error && gmgn_has_error {
        println!("❌ Both providers failed");
        return;
    }
    
    if jupiter_has_error {
        println!("🎯 Winner: GMGN (Jupiter failed)");
        return;
    }
    
    if gmgn_has_error {
        println!("🪐 Winner: Jupiter (GMGN failed)");
        return;
    }
    
    // Compare based on final SOL amount received
    if jupiter.sol_received > gmgn.sol_received {
        let difference = jupiter.sol_received - gmgn.sol_received;
        let pct_diff = (difference as f64 / initial_amount as f64) * 100.0;
        println!("🪐 Winner: Jupiter");
        println!("   Better by: {} lamports ({:.4}%)", difference, pct_diff);
    } else if gmgn.sol_received > jupiter.sol_received {
        let difference = gmgn.sol_received - jupiter.sol_received;
        let pct_diff = (difference as f64 / initial_amount as f64) * 100.0;
        println!("🎯 Winner: GMGN");
        println!("   Better by: {} lamports ({:.4}%)", difference, pct_diff);
    } else {
        println!("🤝 Tie: Both providers performed equally");
    }
    
    // Show performance metrics
    println!("\nPerformance Comparison:");
    let jupiter_perf = jupiter.calculate_performance_percentage(initial_amount);
    let gmgn_perf = gmgn.calculate_performance_percentage(initial_amount);
    
    if jupiter_perf >= 0.0 {
        println!("  Jupiter: {:.4}% gain, {} ms", jupiter_perf, jupiter.total_execution_time_ms);
    } else {
        println!("  Jupiter: {:.4}% loss, {} ms", -jupiter_perf, jupiter.total_execution_time_ms);
    }
    
    if gmgn_perf >= 0.0 {
        println!("  GMGN:    {:.4}% gain, {} ms", gmgn_perf, gmgn.total_execution_time_ms);
    } else {
        println!("  GMGN:    {:.4}% loss, {} ms", -gmgn_perf, gmgn.total_execution_time_ms);
    }
}
