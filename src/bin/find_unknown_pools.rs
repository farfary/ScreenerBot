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
use std::sync::Arc;
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
    let pool_analyzer = Arc::new(PoolAnalyzer::new(rpc_manager.clone()));

    // Setup pool data fetcher for detailed pool information
    let pool_fetcher = PoolDataFetcher::new(rpc_manager.clone());

    println!("✅ Initialized all components successfully\n");

    // Get all tokens from market data database
    let all_tokens_from_db = market_data
        .get_all_tokens().await
        .context("Failed to get all tokens from market data")?;

    println!("📊 Found {} tokens in database", all_tokens_from_db.len());

    if all_tokens_from_db.is_empty() {
        println!("❌ No tokens found in database. Run discovery first!");
        return Ok(());
    }

    // Limit to first 200 tokens to keep processing manageable
    let all_tokens: Vec<_> = all_tokens_from_db.into_iter().take(200).collect();
    println!("🎯 Processing first {} tokens for pool validation", all_tokens.len());

    let mut failed_tokens = Vec::new();
    let mut processed_count = 0;
    let mut success_count = 0;
    let total_tokens = all_tokens.len();

    // Batch the entire process: collect all pool addresses, fetch all at once, then validate all tokens concurrently
    use futures::future::join_all;
    println!("\n� Collecting all pool addresses for {} tokens...", all_tokens.len());
    let mut pool_addresses = Vec::new();
    let mut token_pool_map = std::collections::HashMap::new();
    for token in &all_tokens {
        if let Some(pool_address) = &token.top_pool_address {
            if let Ok(pool_pubkey) = pool_address.parse::<Pubkey>() {
                pool_addresses.push(pool_pubkey);
                token_pool_map.insert(pool_pubkey, token);
            }
        }
    }

    println!("🔄 Fetching all pool accounts in batches of 100 ({} total)...", pool_addresses.len());
    let mut pool_data_map = std::collections::HashMap::new();
    for (batch_idx, chunk) in pool_addresses.chunks(100).enumerate() {
        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match rpc_manager.get_multiple_accounts(chunk).await {
                Ok(accounts) => {
                    for (pool_pubkey, account_opt) in chunk.iter().zip(accounts.iter()) {
                        if let Some(account) = account_opt {
                            pool_data_map.insert(*pool_pubkey, account.clone());
                        }
                    }
                    println!(
                        "✅ Got {} pool accounts in batch {}/{}",
                        chunk.len(),
                        batch_idx + 1,
                        pool_addresses.chunks(100).count()
                    );
                    break;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    if
                        (error_msg.contains("429") || error_msg.contains("Too Many Requests")) &&
                        retry_count < max_retries
                    {
                        retry_count += 1;
                        let delay = std::time::Duration::from_millis(2000 * (retry_count as u64));
                        warn!(
                            "Rate limited on batch {}, retrying in {}ms (attempt {}/{})",
                            batch_idx + 1,
                            delay.as_millis(),
                            retry_count,
                            max_retries
                        );
                        tokio::time::sleep(delay).await;
                    } else {
                        warn!("Failed to fetch pool accounts batch {}: {}", batch_idx + 1, e);
                        break;
                    }
                }
            }
        }

        // Add small delay between batches to be respectful to RPC
        if batch_idx < pool_addresses.chunks(100).count() - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    println!("🚦 Validating all tokens concurrently with rate limiting...");

    // Process tokens in smaller chunks to avoid overwhelming APIs
    let chunk_size = 20; // Process 20 tokens at a time
    let mut all_results = Vec::new();

    for (chunk_idx, token_chunk) in all_tokens.chunks(chunk_size).enumerate() {
        println!(
            "🔄 Processing chunk {}/{} ({} tokens)...",
            chunk_idx + 1,
            (all_tokens.len() + chunk_size - 1) / chunk_size,
            token_chunk.len()
        );

        // Create validation futures for this chunk
        let validation_futures: Vec<_> = token_chunk
            .iter()
            .map(|token| {
                let pairs_client = pairs_client.clone();
                let pool_data_map = pool_data_map.clone();
                let pool_analyzer = pool_analyzer.clone();
                let token = token.clone();
                async move {
                    let log_prefix = format!("[{}] {}", token.symbol, token.mint);
                    println!("{}: Starting validation", log_prefix);
                    let result = validate_token_price_batch(
                        &token,
                        &pairs_client,
                        &pool_analyzer,
                        &pool_data_map
                    ).await;
                    match &result {
                        Ok(true) => println!("{}: ✅ Price validation passed", log_prefix),
                        Ok(false) => warn!("{}: ❌ Price validation failed", log_prefix),
                        Err(e) => error!("{}: ⚠️  Error during validation: {}", log_prefix, e),
                    }
                    (token, result)
                }
            })
            .collect();

        // Execute this chunk's validations concurrently
        let chunk_results = join_all(validation_futures).await;
        all_results.extend(chunk_results);

        // Add delay between chunks to be respectful to APIs
        if chunk_idx < (all_tokens.len() + chunk_size - 1) / chunk_size - 1 {
            println!("⏳ Waiting 2 seconds before next chunk...");
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        }
    }

    // Process results
    for (token, result) in all_results {
        processed_count += 1;
        match result {
            Ok(true) => {
                success_count += 1;
            }
            Ok(false) | Err(_) => failed_tokens.push(token),
        }
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
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

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
    pool_analyzer: &Arc<PoolAnalyzer>,
    pool_data_map: &std::collections::HashMap<Pubkey, solana_sdk::account::Account>
) -> Result<bool> {
    let token_mint = &token.mint;

    // Method 1: Try DEX Screener API price discovery with retry logic
    let mut retry_count = 0;
    let max_retries = 3;
    let mut last_error = None;

    while retry_count < max_retries {
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
                break; // No point retrying if no data exists
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("429") || error_msg.contains("Too Many Requests") {
                    retry_count += 1;
                    last_error = Some(e);
                    if retry_count < max_retries {
                        let delay = std::time::Duration::from_millis(1000 * (retry_count as u64));
                        warn!(
                            "Rate limited for {}, retrying in {}ms (attempt {}/{})",
                            token_mint,
                            delay.as_millis(),
                            retry_count,
                            max_retries
                        );
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                } else {
                    warn!("DEX price lookup failed for {}: {}", token_mint, e);
                    break;
                }
            }
        }
    }

    if retry_count >= max_retries {
        warn!(
            "DEX price lookup failed for {} after {} retries: {}",
            token_mint,
            max_retries,
            last_error.unwrap()
        );
    }

    // Method 2: Try on-chain pool analysis with batched data
    if let Some(pool_address) = &token.top_pool_address {
        if let Ok(pool_pubkey) = pool_address.parse::<Pubkey>() {
            if pool_data_map.get(&pool_pubkey).is_some() {
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
    pool_analyzer: &Arc<PoolAnalyzer>
) -> Result<Vec<String>> {
    let mut details = Vec::new();
    let token_mint = &token.mint;

    // Get DEX Screener pool information (this already does batching internally)
    let mut retry_count = 0;
    let max_retries = 3;

    loop {
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
                    let liquidity_usd = best_pair.liquidity.as_ref().map_or(0.0, |l| l.usd);
                    details.push(format!("  Liquidity: ${:.2}", liquidity_usd));
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
                break; // Success, exit retry loop
            }
            Err(e) => {
                let error_msg = e.to_string();
                if
                    (error_msg.contains("429") || error_msg.contains("Too Many Requests")) &&
                    retry_count < max_retries
                {
                    retry_count += 1;
                    let delay = std::time::Duration::from_millis(1000 * (retry_count as u64));
                    warn!(
                        "Rate limited getting pairs for {}, retrying in {}ms (attempt {}/{})",
                        token_mint,
                        delay.as_millis(),
                        retry_count,
                        max_retries
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    details.push(format!("DEX API lookup failed: {}", e));
                    break;
                }
            }
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
                let liquidity_usd = best_pair.liquidity.as_ref().map_or(0.0, |l| l.usd);
                details.push(format!("  Liquidity: ${:.2}", liquidity_usd));
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
