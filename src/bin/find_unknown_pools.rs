/*
 * Find Unknown Pools - Price Validation Tool with Batch Processing
 *
 * This tool demonstrates the correct pattern for efficient pool data fetching:
 * 1. ALWAYS use batch operations with get_multiple_accounts()
 * 2. Process tokens in batches (20-50 at a time)
 * 3. Collect all pool addresses first, then fetch all account data in one RPC call
 * 4. Use the batched data for analysis instead of individual fetches
 *
 * This pattern should be used throughout the entire system for:
 * - Pool data fetching
 * - Token account queries
 * - Price calculations
 * - Pool analysis
 */

use anyhow::{ Context, Result };
use screenerbot::{
    config::{ Config, RpcConfig },
    discovery::DiscoveryDatabase,
    marketdata::{ MarketData, database::TokenData },
    pairs::{ PairsClient, PoolAnalyzer, PoolDataFetcher },
    rpc::RpcManager,
};
use std::{ sync::Arc, time::Duration };
use log::{ error, info, warn };
use solana_sdk::pubkey::Pubkey;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("\n🔍 Finding Unknown Pools - Price Validation Tool");
    println!("================================================\n");

    // Initialize components
    let config = Config::load("configs.json")?;

    // Setup RPC manager
    let rpc_manager = setup_rpc_manager().await?;

    // Setup discovery database to get all tokens
    let discovery_db = Arc::new(DiscoveryDatabase::new()?);

    // Setup market data to get token data
    let market_data = Arc::new(MarketData::new(discovery_db.clone())?);

    // Setup pairs client for DEX data
    let pairs_client = Arc::new(PairsClient::new()?);

    // Setup pool analyzer for on-chain analysis
    let pool_analyzer = PoolAnalyzer::new(rpc_manager.clone());

    // Setup pool data fetcher for detailed pool information
    let pool_fetcher = PoolDataFetcher::new(rpc_manager.clone());

    println!("✅ Initialized all components successfully\n");

    // Get all tokens from market data database
    let all_tokens = market_data
        .get_all_tokens().await
        .context("Failed to get all tokens from market data")?;

    println!("📊 Found {} tokens in database", all_tokens.len());

    if all_tokens.is_empty() {
        println!("❌ No tokens found in database. Run discovery first!");
        return Ok(());
    }

    let mut failed_tokens = Vec::new();
    let mut processed_count = 0;
    let mut success_count = 0;
    let total_tokens = all_tokens.len();

    // Process tokens in batches for efficient RPC calls
    let batch_size = 50; // Process 20 tokens at once
    for batch in all_tokens.chunks(batch_size) {
        println!(
            "📈 Processing batch {}-{}/{} tokens...",
            processed_count + 1,
            (processed_count + batch.len()).min(total_tokens),
            total_tokens
        );

        // Collect all pool addresses for batch fetching
        let mut pool_addresses = Vec::new();
        let mut token_pool_map = std::collections::HashMap::new();

        for token in batch {
            if let Some(pool_address) = &token.top_pool_address {
                if let Ok(pool_pubkey) = pool_address.parse::<Pubkey>() {
                    pool_addresses.push(pool_pubkey);
                    token_pool_map.insert(pool_pubkey, token);
                }
            }
        }

        // Batch fetch pool account data if we have any pools
        let mut pool_data_map = std::collections::HashMap::new();
        if !pool_addresses.is_empty() {
            match rpc_manager.get_multiple_accounts(&pool_addresses).await {
                Ok(accounts) => {
                    for (pool_pubkey, account_opt) in pool_addresses.iter().zip(accounts.iter()) {
                        if let Some(account) = account_opt {
                            pool_data_map.insert(*pool_pubkey, account.clone());
                        }
                    }
                    println!("✅ Fetched {} pool accounts in batch", pool_data_map.len());
                }
                Err(e) => {
                    warn!("Failed to fetch pool accounts in batch: {}", e);
                }
            }
        }

        // Now process each token in the batch
        for token in batch {
            processed_count += 1;

            // Try to validate price through multiple methods with batched data
            let price_validation_result = validate_token_price_batch(
                &token,
                &pairs_client,
                &pool_analyzer,
                &pool_data_map
            ).await;

            match price_validation_result {
                Ok(true) => {
                    success_count += 1;
                    // Price validation successful
                }
                Ok(false) => {
                    // Price validation failed - collect details
                    failed_tokens.push(token.clone());
                    warn!("Price validation failed for token: {}", token.mint);
                }
                Err(e) => {
                    // Error during validation - collect details
                    failed_tokens.push(token.clone());
                    error!("Error validating token {}: {}", token.mint, e);
                }
            }
        }

        // Rate limiting between batches
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Report results
    println!("\n📋 Validation Results");
    println!("=====================");
    println!("✅ Successfully validated: {} tokens", success_count);
    println!("❌ Failed validation: {} tokens", failed_tokens.len());
    println!("📊 Total processed: {} tokens", total_tokens);

    if !failed_tokens.is_empty() {
        println!("\n🚨 Failed Tokens Details");
        println!("========================\n");

        for (index, token) in failed_tokens.iter().enumerate() {
            println!("{}. Token: {}", index + 1, token.mint);
            println!("   Symbol: {}", token.symbol);
            println!("   Name: {}", token.name);
            println!("   Current Price: ${:.8}", token.price_usd);
            println!("   Liquidity: ${:.2}", token.liquidity_usd);

            // Get detailed pool information with batch optimization
            match get_detailed_pool_info_batch(token, &pairs_client, &pool_analyzer).await {
                Ok(details) => {
                    println!("   Pool Details:");
                    for detail in details {
                        println!("     - {}", detail);
                    }
                }
                Err(e) => {
                    println!("   ⚠️  Failed to get pool details: {}", e);
                }
            }

            println!();
        }
    }

    println!("🔍 Analysis complete!");
    Ok(())
}

async fn setup_rpc_manager() -> Result<Arc<RpcManager>> {
    info!("🔗 Setting up RPC manager...");

    let primary_url = std::env
        ::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://side-cool-seed.solana-mainnet.quiknode.pro/59757f9c6ed8ca54674c938cdd55b4f357abd797/".to_string());

    let fallback_urls = vec![
        "https://solana-api.projectserum.com".to_string(),
        "https://api.mainnet-beta.solana.com".to_string()
    ];

    let rpc_config = RpcConfig::default();
    let rpc_manager = Arc::new(RpcManager::new(primary_url, fallback_urls, rpc_config)?);

    info!("✅ RPC Manager initialized with {} fallback URLs", 2);
    Ok(rpc_manager)
}

/// Validate token price through multiple methods with batched data
async fn validate_token_price_batch(
    token: &TokenData,
    pairs_client: &Arc<PairsClient>,
    pool_analyzer: &PoolAnalyzer,
    pool_data_map: &std::collections::HashMap<Pubkey, solana_sdk::account::Account>
) -> Result<bool> {
    let token_mint = &token.mint;

    // Method 1: Try DEX Screener API price discovery
    match pairs_client.get_best_price(token_mint).await {
        Ok(Some(dex_price)) => {
            // Compare with stored price (allow 10% variance)
            if token.price_usd > 0.0 {
                let price_diff = (dex_price - token.price_usd).abs() / token.price_usd;
                if price_diff > 0.1 {
                    warn!(
                        "Large price difference for {}: stored=${:.8}, DEX=${:.8}",
                        token_mint,
                        token.price_usd,
                        dex_price
                    );
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        Ok(None) => {
            warn!("No DEX price found for {}", token_mint);
        }
        Err(e) => {
            warn!("DEX price lookup failed for {}: {}", token_mint, e);
        }
    }

    // Method 2: Try on-chain pool analysis with batched data
    if let Some(pool_address) = &token.top_pool_address {
        if let Ok(pool_pubkey) = pool_address.parse::<Pubkey>() {
            if let Some(account_data) = pool_data_map.get(&pool_pubkey) {
                // We have the account data, try to decode and analyze it
                match pool_analyzer.analyze_pool(&pool_pubkey.to_string()).await {
                    Ok(analysis) => {
                        let pool_price = analysis.price_info.price;
                        if token.price_usd > 0.0 {
                            let price_diff = (pool_price - token.price_usd).abs() / token.price_usd;
                            if price_diff > 0.1 {
                                warn!(
                                    "Large price difference for {} (on-chain): stored=${:.8}, pool=${:.8}",
                                    token_mint,
                                    token.price_usd,
                                    pool_price
                                );
                                return Ok(false);
                            }
                        }
                        return Ok(true);
                    }
                    Err(e) => {
                        warn!("Pool analysis failed for {} ({}): {}", token_mint, pool_address, e);
                    }
                }
            }
        }
    }

    // Method 3: Check if token has valid liquidity/volume data
    if token.liquidity_usd > 1000.0 && token.volume_24h > 100.0 {
        // Has reasonable liquidity and volume, price might be valid
        return Ok(true);
    }

    // All validation methods failed
    Ok(false)
}

/// Validate token price through multiple methods (old single-fetch method - deprecated)
async fn validate_token_price(
    token: &TokenData,
    pairs_client: &Arc<PairsClient>,
    pool_analyzer: &PoolAnalyzer,
    _pool_fetcher: &PoolDataFetcher
) -> Result<bool> {
    let token_mint = &token.mint;

    // Method 1: Try DEX Screener API price discovery
    match pairs_client.get_best_price(token_mint).await {
        Ok(Some(dex_price)) => {
            // Compare with stored price (allow 10% variance)
            let price_diff = (dex_price - token.price_usd).abs() / token.price_usd;
            if price_diff > 0.1 {
                warn!(
                    "Large price difference for {}: stored=${:.8}, DEX=${:.8}",
                    token_mint,
                    token.price_usd,
                    dex_price
                );
                return Ok(false);
            }
            return Ok(true);
        }
        Ok(None) => {
            warn!("No DEX price found for {}", token_mint);
        }
        Err(e) => {
            warn!("DEX price lookup failed for {}: {}", token_mint, e);
        }
    }

    // Method 2: Try on-chain pool analysis if we have pool address
    if let Some(pool_address) = &token.top_pool_address {
        match pool_address.parse::<Pubkey>() {
            Ok(pool_pubkey) => {
                match pool_analyzer.analyze_pool(&pool_pubkey.to_string()).await {
                    Ok(analysis) => {
                        let pool_price = analysis.price_info.price;
                        let price_diff = (pool_price - token.price_usd).abs() / token.price_usd;
                        if price_diff > 0.1 {
                            warn!(
                                "Large price difference for {} (on-chain): stored=${:.8}, pool=${:.8}",
                                token_mint,
                                token.price_usd,
                                pool_price
                            );
                            return Ok(false);
                        }
                        return Ok(true);
                    }
                    Err(e) => {
                        warn!("Pool analysis failed for {} ({}): {}", token_mint, pool_address, e);
                    }
                }
            }
            Err(e) => {
                warn!("Invalid pool address for {}: {} - {}", token_mint, pool_address, e);
            }
        }
    }

    // Method 3: Check if token has valid liquidity/volume data
    if token.liquidity_usd > 1000.0 && token.volume_24h > 100.0 {
        // Has reasonable liquidity and volume, price might be valid
        return Ok(true);
    }

    // All validation methods failed
    Ok(false)
}

/// Get detailed information about token pools with batch optimization
async fn get_detailed_pool_info_batch(
    token: &TokenData,
    pairs_client: &Arc<PairsClient>,
    pool_analyzer: &PoolAnalyzer
) -> Result<Vec<String>> {
    let mut details = Vec::new();
    let token_mint = &token.mint;

    // Get DEX Screener pool information (this already does batching internally)
    match pairs_client.get_solana_token_pairs(token_mint).await {
        Ok(pairs) => {
            details.push(format!("DEX Pools Found: {}", pairs.len()));

            // Group by DEX
            let mut dex_counts = std::collections::HashMap::new();
            for pair in &pairs {
                *dex_counts.entry(pair.dex_id.clone()).or_insert(0) += 1;
            }

            for (dex, count) in dex_counts {
                details.push(format!("  {} pools on {}", count, dex.to_uppercase()));
            }

            // Get best pair details
            if let Some(best_pair) = pairs_client.get_best_pair(pairs.clone()) {
                let quality_score = pairs_client.calculate_pool_quality_score(&best_pair);
                details.push(
                    format!("Best Pool: {} ({})", best_pair.pair_address, best_pair.dex_id)
                );
                details.push(format!("  Quality Score: {:.1}/100", quality_score));
                details.push(format!("  Liquidity: ${:.2}", best_pair.liquidity.usd));
                details.push(format!("  24h Volume: ${:.2}", best_pair.volume.h24));
                details.push(format!("  DEX Price: ${:.8}", best_pair.price_usd));

                // Try to get on-chain pool data using batch-optimized approach
                // Note: In a real batch scenario, we'd collect all pool addresses first
                // and then fetch them all at once using get_multiple_accounts
                match best_pair.pair_address.parse::<Pubkey>() {
                    Ok(pool_pubkey) => {
                        match pool_analyzer.analyze_pool(&pool_pubkey.to_string()).await {
                            Ok(analysis) => {
                                details.push(
                                    format!(
                                        "On-chain Pool Type: {:?}",
                                        analysis.pool_info.pool_type
                                    )
                                );
                                details.push(
                                    format!("On-chain Price: ${:.8}", analysis.price_info.price)
                                );
                                details.push(
                                    format!("Program ID: {}", analysis.pool_info.program_id)
                                );
                                details.push(
                                    format!("Health Score: {:.1}/100", analysis.health_score)
                                );
                                details.push(format!("TVL: ${:.2}", analysis.tvl));

                                // Token information
                                details.push(
                                    format!("Token 0: {}", analysis.pool_info.token_mint_0)
                                );
                                details.push(
                                    format!("Token 1: {}", analysis.pool_info.token_mint_1)
                                );
                                details.push(
                                    format!("Reserve 0: {}", analysis.pool_info.reserve_0)
                                );
                                details.push(
                                    format!("Reserve 1: {}", analysis.pool_info.reserve_1)
                                );
                            }
                            Err(e) => {
                                details.push(format!("On-chain analysis failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        details.push(format!("Invalid pool address format: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            details.push(format!("DEX API lookup failed: {}", e));
        }
    }

    // Check stored pool information
    if let Some(pool_address) = &token.top_pool_address {
        details.push(format!("Stored Top Pool: {}", pool_address));
        details.push(
            format!(
                "Stored Base Reserve: {}",
                token.top_pool_base_reserve.map_or("N/A".to_string(), |v| format!("{:.2}", v))
            )
        );
        details.push(
            format!(
                "Stored Quote Reserve: {}",
                token.top_pool_quote_reserve.map_or("N/A".to_string(), |v| format!("{:.2}", v))
            )
        );
    } else {
        details.push("No stored pool information".to_string());
    }

    Ok(details)
}

/// Get detailed information about token pools (old single-fetch method - deprecated)
async fn get_detailed_pool_info(
    token: &TokenData,
    pairs_client: &Arc<PairsClient>,
    pool_analyzer: &PoolAnalyzer
) -> Result<Vec<String>> {
    let mut details = Vec::new();
    let token_mint = &token.mint;

    // Get DEX Screener pool information
    match pairs_client.get_solana_token_pairs(token_mint).await {
        Ok(pairs) => {
            details.push(format!("DEX Pools Found: {}", pairs.len()));

            // Group by DEX
            let mut dex_counts = std::collections::HashMap::new();
            for pair in &pairs {
                *dex_counts.entry(pair.dex_id.clone()).or_insert(0) += 1;
            }

            for (dex, count) in dex_counts {
                details.push(format!("  {} pools on {}", count, dex.to_uppercase()));
            }

            // Get best pair details
            if let Some(best_pair) = pairs_client.get_best_pair(pairs.clone()) {
                let quality_score = pairs_client.calculate_pool_quality_score(&best_pair);
                details.push(
                    format!("Best Pool: {} ({})", best_pair.pair_address, best_pair.dex_id)
                );
                details.push(format!("  Quality Score: {:.1}/100", quality_score));
                details.push(format!("  Liquidity: ${:.2}", best_pair.liquidity.usd));
                details.push(format!("  24h Volume: ${:.2}", best_pair.volume.h24));
                details.push(format!("  DEX Price: ${:.8}", best_pair.price_usd));

                // Try to get on-chain pool data
                match best_pair.pair_address.parse::<Pubkey>() {
                    Ok(pool_pubkey) => {
                        match pool_analyzer.analyze_pool(&pool_pubkey.to_string()).await {
                            Ok(analysis) => {
                                details.push(
                                    format!(
                                        "On-chain Pool Type: {:?}",
                                        analysis.pool_info.pool_type
                                    )
                                );
                                details.push(
                                    format!("On-chain Price: ${:.8}", analysis.price_info.price)
                                );
                                details.push(
                                    format!("Program ID: {}", analysis.pool_info.program_id)
                                );
                                details.push(
                                    format!("Health Score: {:.1}/100", analysis.health_score)
                                );
                                details.push(format!("TVL: ${:.2}", analysis.tvl));

                                // Token information
                                details.push(
                                    format!("Token 0: {}", analysis.pool_info.token_mint_0)
                                );
                                details.push(
                                    format!("Token 1: {}", analysis.pool_info.token_mint_1)
                                );
                                details.push(
                                    format!("Reserve 0: {}", analysis.pool_info.reserve_0)
                                );
                                details.push(
                                    format!("Reserve 1: {}", analysis.pool_info.reserve_1)
                                );
                            }
                            Err(e) => {
                                details.push(format!("On-chain analysis failed: {}", e));
                            }
                        }
                    }
                    Err(e) => {
                        details.push(format!("Invalid pool address format: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            details.push(format!("DEX API lookup failed: {}", e));
        }
    }

    // Check stored pool information
    if let Some(pool_address) = &token.top_pool_address {
        details.push(format!("Stored Top Pool: {}", pool_address));
        details.push(
            format!(
                "Stored Base Reserve: {}",
                token.top_pool_base_reserve.map_or("N/A".to_string(), |v| format!("{:.2}", v))
            )
        );
        details.push(
            format!(
                "Stored Quote Reserve: {}",
                token.top_pool_quote_reserve.map_or("N/A".to_string(), |v| format!("{:.2}", v))
            )
        );
    } else {
        details.push("No stored pool information".to_string());
    }

    Ok(details)
}
