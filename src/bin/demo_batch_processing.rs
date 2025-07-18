/*
 * Batch Pool Processing Demo
 * 
 * This tool demonstrates the CORRECT w    println!("🔄 Demo 1: Wrong Way - Individual RPC Calls");
    println!("{}", "-".repeat(50)); to fetch and process pool data efficiently:
 * 
 * ✅ CORRECT PATTERNS:
 * 1. Use get_multiple_accounts() to fetch 20-100 pools at once
 * 2. Process tokens in batches, not one by one  
 * 3. Collect all addresses first, then batch fetch
 * 4. Use batched data for all subsequent operations
 * 5. Implement fallback to individual fetching only when batch fails
 * 
 * ❌ AVOID:
 * 1. Individual get_account() calls in loops
 * 2. Sequential processing without batching
 * 3. Repeated RPC calls for the same data
 * 4. No rate limiting between operations
 */

use anyhow::{ Context, Result };
use screenerbot::{ config::RpcConfig, pairs::{ PoolAnalyzer, PoolDataFetcher }, rpc::RpcManager };
use std::{ sync::Arc, time::Duration };
use log::{ info, warn };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("\n🚀 Batch Pool Processing Demo");
    println!("============================\n");

    // Setup RPC manager
    let rpc_manager = setup_rpc_manager().await?;

    // Setup pool components
    let pool_fetcher = PoolDataFetcher::new(rpc_manager.clone());
    let pool_analyzer = PoolAnalyzer::new(rpc_manager.clone());

    println!("✅ Initialized components\n");

    // Demo 1: Batch pool fetching vs individual fetching
    await_demo_batch_vs_individual(&pool_fetcher).await?;

    // Demo 2: Batch pool analysis
    await_demo_batch_analysis(&pool_analyzer, &rpc_manager).await?;

    // Demo 3: Real-world batch processing with DEX Screener data
    await_demo_real_world_batch(&pool_fetcher).await?;

    println!("🎯 All demos completed!");
    Ok(())
}

async fn setup_rpc_manager() -> Result<Arc<RpcManager>> {
    let primary_url = std::env
        ::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let fallback_urls = vec![
        "https://solana-api.projectserum.com".to_string(),
        "https://api.mainnet-beta.solana.com".to_string()
    ];

    let rpc_config = RpcConfig::default();
    let rpc_manager = Arc::new(RpcManager::new(primary_url, fallback_urls, rpc_config)?);

    info!("✅ RPC Manager initialized");
    Ok(rpc_manager)
}

async fn await_demo_batch_vs_individual(pool_fetcher: &PoolDataFetcher) -> Result<()> {
    println!("📊 Demo 1: Batch vs Individual Pool Fetching");
    println!("{}", "-".repeat(50));

    // Sample pool addresses (well-known Solana pools)
    let pool_addresses = vec![
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2", // SOL/USDC Raydium
        "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm", // SOL/USDC Orca
        "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj", // SOL/USDT Raydium
        "85HrPbJtrZYeUFNjFqCWfTdyWnDVjrJ3C8VGMd8Fy8h8", // RAY/SOL
        "6a1CsrpeZubDjEJE9s1CMVheB6HWM5d7m1cj2jkhyXhj" // SAMO/SOL
    ];

    let pubkeys: Result<Vec<Pubkey>, _> = pool_addresses
        .iter()
        .map(|addr| Pubkey::from_str(addr))
        .collect();

    let pubkeys = pubkeys.context("Failed to parse pool addresses")?;

    println!("📈 Testing with {} pool addresses", pubkeys.len());

    // Method 1: Individual fetching (SLOW - DON'T DO THIS)
    println!("\n❌ Method 1: Individual fetching (inefficient)");
    let start_time = std::time::Instant::now();
    let mut individual_results = Vec::new();

    for (i, pool_address) in pubkeys.iter().enumerate() {
        match pool_fetcher.fetch_pool_data(pool_address).await {
            Ok(pool_info) => {
                individual_results.push(pool_info);
                println!("   Fetched pool {}/{}: {}", i + 1, pubkeys.len(), pool_address);
            }
            Err(e) => {
                warn!("Failed to fetch pool {}: {}", pool_address, e);
            }
        }

        // Rate limiting
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    let individual_duration = start_time.elapsed();
    println!("   ⏱️  Individual fetching took: {:?}", individual_duration);
    println!("   ✅ Successfully fetched: {} pools", individual_results.len());

    // Method 2: Batch fetching (FAST - CORRECT WAY)
    println!("\n✅ Method 2: Batch fetching (efficient)");
    let start_time = std::time::Instant::now();

    let batch_results = pool_fetcher.fetch_multiple_pools(&pubkeys).await?;

    let batch_duration = start_time.elapsed();
    println!("   ⏱️  Batch fetching took: {:?}", batch_duration);
    println!("   ✅ Successfully fetched: {} pools", batch_results.len());

    // Performance comparison
    let speedup = (individual_duration.as_millis() as f64) / (batch_duration.as_millis() as f64);
    println!("\n🚀 Performance Results:");
    println!("   Individual: {:?} ({} pools)", individual_duration, individual_results.len());
    println!("   Batch:      {:?} ({} pools)", batch_duration, batch_results.len());
    println!("   Speedup:    {:.1}x faster with batch processing!", speedup);

    Ok(())
}

async fn await_demo_batch_analysis(
    pool_analyzer: &PoolAnalyzer,
    rpc_manager: &Arc<RpcManager>
) -> Result<()> {
    println!("\n\n📊 Demo 2: Batch Pool Analysis");
    println!("{}", "-".repeat(50));

    // Sample pools for analysis
    let pool_addresses = vec![
        "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2", // SOL/USDC Raydium
        "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm", // SOL/USDC Orca
        "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj" // SOL/USDT Raydium
    ];

    let pubkeys: Result<Vec<Pubkey>, _> = pool_addresses
        .iter()
        .map(|addr| Pubkey::from_str(addr))
        .collect();

    let pubkeys = pubkeys.context("Failed to parse pool addresses")?;

    println!("🔍 Analyzing {} pools with batch account fetching", pubkeys.len());

    // Fetch all account data in one batch call
    let start_time = std::time::Instant::now();

    match rpc_manager.get_multiple_accounts(&pubkeys).await {
        Ok(accounts) => {
            let fetch_duration = start_time.elapsed();
            println!("✅ Fetched {} account data in {:?}", accounts.len(), fetch_duration);

            // Now analyze each pool using the batched data
            for (i, (pool_address, account_opt)) in pubkeys
                .iter()
                .zip(accounts.iter())
                .enumerate() {
                if let Some(_account) = account_opt {
                    // Analyze the pool (this still does individual RPC calls for vault balances)
                    match pool_analyzer.analyze_pool(&pool_address.to_string()).await {
                        Ok(analysis) => {
                            println!(
                                "   {}. Pool {}: {:?}",
                                i + 1,
                                pool_address,
                                analysis.pool_info.pool_type
                            );
                            println!("      Price: ${:.6}", analysis.price_info.price);
                            println!("      TVL: ${:.2}", analysis.tvl);
                            println!("      Health Score: {:.1}/100", analysis.health_score);
                        }
                        Err(e) => {
                            warn!("Failed to analyze pool {}: {}", pool_address, e);
                        }
                    }
                } else {
                    warn!("No account data for pool {}", pool_address);
                }
            }
        }
        Err(e) => {
            warn!("Failed to fetch pool accounts in batch: {}", e);
        }
    }

    Ok(())
}

async fn await_demo_real_world_batch(pool_fetcher: &PoolDataFetcher) -> Result<()> {
    println!("\n\n📊 Demo 3: Real-world Batch Processing Pattern");
    println!("{}", "-".repeat(50));

    // Simulate getting pools from DEX Screener API for multiple tokens
    let token_mints = vec![
        "So11111111111111111111111111111111111111112", // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" // USDT
    ];

    println!("🔍 Processing {} tokens to find all their pools", token_mints.len());

    // Step 1: Collect all unique pool addresses from all tokens
    let mut all_pool_addresses = std::collections::HashSet::new();
    let mut token_pool_map = std::collections::HashMap::new();

    for token_mint in &token_mints {
        // In a real scenario, this would come from DEX Screener API
        // For demo, we'll use some known pools
        let sample_pools = match *token_mint {
            "So11111111111111111111111111111111111111112" =>
                vec![
                    "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
                    "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm",
                    "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj"
                ],
            "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" =>
                vec![
                    "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2",
                    "7qbRF6YsyGuLUVs6Y1q64bdVrfe4ZcUUz1JRdoVNUJnm"
                ],
            _ => vec!["8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj"],
        };

        let pool_pubkeys: Result<Vec<Pubkey>, _> = sample_pools
            .iter()
            .map(|addr| Pubkey::from_str(addr))
            .collect();

        if let Ok(pubkeys) = pool_pubkeys {
            for pubkey in pubkeys {
                all_pool_addresses.insert(pubkey);
                token_pool_map.entry(token_mint.to_string()).or_insert_with(Vec::new).push(pubkey);
            }
        }
    }

    let unique_pools: Vec<Pubkey> = all_pool_addresses.into_iter().collect();
    println!("📊 Found {} unique pools across all tokens", unique_pools.len());

    // Step 2: Batch fetch ALL pool data at once
    println!("🚀 Batch fetching all pool data...");
    let start_time = std::time::Instant::now();

    let all_pool_infos = pool_fetcher.fetch_multiple_pools(&unique_pools).await?;

    let batch_duration = start_time.elapsed();
    println!("✅ Fetched {} pools in {:?}", all_pool_infos.len(), batch_duration);

    // Step 3: Use the batched data for analysis per token
    for token_mint in &token_mints {
        if let Some(pool_addresses) = token_pool_map.get(*token_mint) {
            println!("\n📈 Token {}: {} pools", token_mint, pool_addresses.len());

            let mut best_pool = None;
            let mut best_tvl = 0.0;

            for pool_address in pool_addresses {
                // Find the pool info in our batched results
                if
                    let Some(pool_info) = all_pool_infos
                        .iter()
                        .find(|p| p.pool_address == *pool_address)
                {
                    // Calculate TVL (simplified)
                    let tvl = (pool_info.reserve_0 as f64) + (pool_info.reserve_1 as f64);

                    println!("   Pool {}: TVL ~{:.0}", pool_address, tvl);

                    if tvl > best_tvl {
                        best_tvl = tvl;
                        best_pool = Some(pool_info);
                    }
                }
            }

            if let Some(pool) = best_pool {
                println!("   🏆 Best pool: {} (TVL: {:.0})", pool.pool_address, best_tvl);
            }
        }
    }

    println!("\n💡 Key Takeaways:");
    println!("   ✅ Batched {} unique pools in one operation", unique_pools.len());
    println!("   ✅ Used batched data for analysis of {} tokens", token_mints.len());
    println!("   ✅ Avoided {} individual RPC calls", unique_pools.len());
    println!("   ✅ Total time: {:?}", batch_duration);

    Ok(())
}
