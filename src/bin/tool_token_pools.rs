use screenerbot::{
    config::RpcConfig,
    marketdata::MarketData,
    pairs::PoolAnalyzer,
    rpc::RpcManager,
    discovery::DiscoveryDatabase,
};

use anyhow::{ Context, Result };
use solana_sdk::pubkey::Pubkey;
use std::{ str::FromStr, sync::Arc };
use log::{ info, warn };
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DexScreenerToken {
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "dexId")]
    dex_id: String,
    url: String,
    #[serde(rename = "pairAddress")]
    pair_address: String,
    labels: Option<Vec<String>>,
    #[serde(rename = "baseToken")]
    base_token: TokenInfo,
    #[serde(rename = "quoteToken")]
    quote_token: TokenInfo,
    #[serde(rename = "priceNative")]
    price_native: String,
    #[serde(rename = "priceUsd")]
    price_usd: String,
    liquidity: DexScreenerLiquidity,
    volume: DexScreenerVolume,
    #[serde(rename = "priceChange")]
    price_change: DexScreenerPriceChange,
    #[serde(rename = "pairCreatedAt")]
    pair_created_at: Option<u64>,
    fdv: Option<f64>,
    #[serde(rename = "marketCap")]
    market_cap: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    address: String,
    name: String,
    symbol: String,
}

#[derive(Debug, Deserialize)]
struct DexScreenerLiquidity {
    usd: f64,
    base: f64,
    quote: f64,
}

#[derive(Debug, Deserialize)]
struct DexScreenerVolume {
    h24: f64,
    h6: f64,
    h1: f64,
    m5: f64,
}

#[derive(Debug, Deserialize)]
struct DexScreenerPriceChange {
    h24: Option<f64>,
    h6: Option<f64>,
    h1: Option<f64>,
    m5: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    // Token address to search for
    let target_token = "83kGGSggYGP2ZEEyvX54SkZR1kFn84RgGCDyptbDbonk";

    println!("🔍 Finding all pools for token: {}", target_token);
    println!("{}", "=".repeat(80));

    // Setup components
    // Load config
    let config_path = std::env
        ::var("SCREENERBOT_CONFIG")
        .unwrap_or_else(|_| "configs.json".to_string());
    let config = screenerbot::config::Config::load(&config_path)?;
    let rpc_manager = setup_rpc_manager_from_config(
        &config.rpc_url,
        &config.rpc_fallbacks,
        config.rpc.clone()
    )?;
    let market_data = setup_market_data().await?;
    let pool_analyzer = PoolAnalyzer::new(rpc_manager.clone());
    let http_client = Client::new();

    // Method 1: Use DexScreener Tokens API (correct endpoint)
    println!("\n🌐 Method 1: Searching via DexScreener Tokens API");
    println!("{}", "-".repeat(50));

    match search_via_dexscreener_tokens_api(&http_client, target_token).await {
        Ok(count) => println!("✅ Found {} pools via DexScreener", count),
        Err(e) => warn!("❌ DexScreener search failed: {}", e),
    }

    // Method 2: Use Gecko Terminal API via market data
    println!("\n🦎 Method 2: Searching via Gecko Terminal API");
    println!("{}", "-".repeat(50));

    match search_via_gecko_terminal(&market_data, target_token).await {
        Ok(count) => println!("✅ Found {} pools via Gecko Terminal", count),
        Err(e) => warn!("❌ Gecko Terminal search failed: {}", e),
    }

    // Method 3: Direct on-chain scanning (most comprehensive but slower)
    println!("\n⛓️  Method 3: Direct on-chain pool scanning");
    println!("{}", "-".repeat(50));

    match search_via_onchain_scan(&rpc_manager, &pool_analyzer, target_token).await {
        Ok(count) => println!("✅ Found {} pools via on-chain scanning", count),
        Err(e) => warn!("❌ On-chain scanning failed: {}", e),
    }

    println!("\n🎯 Pool search completed!");

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

    info!("✅ RPC manager initialized");
    Ok(rpc_manager)
}
fn setup_rpc_manager_from_config(
    primary_url: &str,
    fallback_urls: &[String],
    rpc_config: RpcConfig
) -> Result<Arc<RpcManager>> {
    info!("🔗 Setting up RPC manager from config...");
    let fallback_urls = if fallback_urls.is_empty() {
        vec![
            "https://solana-api.projectserum.com".to_string(),
            "https://api.mainnet-beta.solana.com".to_string()
        ]
    } else {
        fallback_urls.to_vec()
    };
    let rpc_manager = Arc::new(
        RpcManager::new(primary_url.to_string(), fallback_urls, rpc_config)?
    );
    info!("✅ RPC manager initialized");
    Ok(rpc_manager)
}

async fn setup_market_data() -> Result<MarketData> {
    info!("📊 Setting up market data module...");

    // Create a discovery database (required for MarketData)
    let discovery_db = Arc::new(DiscoveryDatabase::new()?);
    let market_data = MarketData::new(discovery_db)?;

    info!("✅ Market data module initialized");
    Ok(market_data)
}

async fn search_via_dexscreener_tokens_api(client: &Client, token_address: &str) -> Result<usize> {
    println!("� Fetching pools from DexScreener Tokens API...");

    let url = format!("https://api.dexscreener.com/tokens/v1/solana/{}", token_address);

    let response = client
        .get(&url)
        .send().await
        .context("Failed to send request to DexScreener API")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(
            anyhow::anyhow!("DexScreener API request failed with status {}: {}", status, error_text)
        );
    }

    let pools: Vec<DexScreenerToken> = response
        .json().await
        .context("Failed to parse JSON response from DexScreener API")?;

    if pools.is_empty() {
        println!("❌ No pools found on DexScreener");
        return Ok(0);
    }

    println!("✅ Found {} pools from DexScreener:", pools.len());

    // Display statistics
    let total_liquidity: f64 = pools
        .iter()
        .map(|p| p.liquidity.as_ref().map_or(0.0, |l| l.usd))
        .sum();
    let total_volume_24h: f64 = pools
        .iter()
        .map(|p| p.volume.h24)
        .sum();

    println!("📊 Summary Statistics:");
    println!("   Total Liquidity: ${:.2}", total_liquidity);
    println!("   Total 24h Volume: ${:.2}", total_volume_24h);

    // Show all pools with details
    println!("\n🏊 Pool Details:");
    for (i, pool) in pools.iter().enumerate() {
        let price = pool.price_usd.parse::<f64>().unwrap_or(0.0);
        let price_change_24h = pool.price_change.h24.unwrap_or(0.0);
        let labels = pool.labels
            .as_ref()
            .map(|l| l.join(", "))
            .unwrap_or_else(|| "N/A".to_string());

        println!("   {}. {} Pool ({})", i + 1, pool.dex_id.to_uppercase(), pool.pair_address);
        println!("      Pair: {}/{}", pool.base_token.symbol, pool.quote_token.symbol);
        println!("      Labels: {}", labels);
        println!("      Price: ${:.8} ({:+.2}% 24h)", price, price_change_24h);
        let liquidity_usd = pool.liquidity.as_ref().map_or(0.0, |l| l.usd);
        println!("      Liquidity: ${:.2}", liquidity_usd);
        println!("      24h Volume: ${:.2}", pool.volume.h24);
        if let Some(market_cap) = pool.market_cap {
            println!("      Market Cap: ${:.2}", market_cap);
        }
        if let Some(fdv) = pool.fdv {
            println!("      FDV: ${:.2}", fdv);
        }
        println!("      URL: {}", pool.url);
        println!();
    }

    // Show pools by DEX
    let mut dex_counts = std::collections::HashMap::new();
    for pool in &pools {
        *dex_counts.entry(&pool.dex_id).or_insert(0) += 1;
    }

    println!("🏢 Pools by DEX:");
    let mut dex_list: Vec<_> = dex_counts.iter().collect();
    dex_list.sort_by(|a, b| b.1.cmp(a.1));
    for (dex, count) in dex_list {
        println!("   {}: {} pools", dex.to_uppercase(), count);
    }

    Ok(pools.len())
}

async fn search_via_gecko_terminal(market_data: &MarketData, token_address: &str) -> Result<usize> {
    println!("🦎 Fetching data from Gecko Terminal...");

    // Try to get token data including pools
    match market_data.get_token_data(token_address).await? {
        Some(token_data) => {
            println!("✅ Token found in Gecko Terminal:");
            println!("   Name: {}", token_data.name);
            println!("   Symbol: {}", token_data.symbol);
            println!("   Price: ${:.8}", token_data.price_usd);
            println!("   Market Cap: ${:.2}", token_data.market_cap);
            println!("   24h Volume: ${:.2}", token_data.volume_24h);

            // Get pools for this token
            let pools = market_data.get_token_pools(token_address).await?;

            if pools.is_empty() {
                println!("❌ No pools found in Gecko Terminal");
                return Ok(0);
            }

            println!("\n🏊 Pools from Gecko Terminal ({} total):", pools.len());
            for (i, pool) in pools.iter().enumerate() {
                println!(
                    "   {}. {} - ${:.2} liquidity, ${:.2} 24h volume",
                    i + 1,
                    pool.pool_address,
                    pool.liquidity_usd,
                    pool.volume_24h
                );
            }

            Ok(pools.len())
        }
        None => {
            println!("❌ Token not found in Gecko Terminal");
            Ok(0)
        }
    }
}

async fn search_via_onchain_scan(
    rpc_manager: &Arc<RpcManager>,
    pool_analyzer: &PoolAnalyzer,
    token_address: &str
) -> Result<usize> {
    println!("⛓️  Scanning sample pools on-chain for token presence...");

    let token_pubkey = Pubkey::from_str(token_address).context("Invalid token address")?;

    println!("🔍 Target token: {}", token_pubkey);

    // For demonstration, we'll check a smaller set of well-known pools
    let sample_pools = vec![
        "8sLbNZoA1cfnvMJLPfp98ZLAnFSYCFApfJKMbiXNLwxj", // Raydium CLMM SOL/USDC
        "61R1ndXxvsWXXkWSyNkCxnzwd3zUNB8Q2ibmkiLPC8ht", // Raydium CLMM
        "LaT4mSXV2gPyjQsuCBZ7XmV1G7DEoToXVBf4pEvL6be" // The actual pool from API test
    ];

    println!("🔍 Checking {} sample pool addresses...", sample_pools.len());

    let mut found_pools = Vec::new();

    for pool_address in sample_pools {
        match check_pool_contains_token(rpc_manager, pool_address, &token_pubkey).await {
            Ok(contains_token) => {
                if contains_token {
                    println!("✅ Found pool containing token: {}", pool_address);

                    // Try to analyze the pool for more details
                    match pool_analyzer.analyze_pool(pool_address).await {
                        Ok(analysis) => {
                            println!("   📊 Pool Analysis:");
                            println!("      Type: {:?}", analysis.pool_info.pool_type);
                            println!("      Price: {:.8}", analysis.price_info.price);
                            println!("      TVL: {:.2}", analysis.tvl);
                            println!("      Health Score: {:.1}/100", analysis.health_score);
                        }
                        Err(e) => {
                            warn!("   ⚠️ Pool analysis failed: {}", e);
                        }
                    }

                    found_pools.push(pool_address.to_string());
                }
            }
            Err(_) => {
                // Expected - most pools won't contain our target token
            }
        }
    }

    if found_pools.is_empty() {
        println!("❌ No pools found containing target token in sample set");
        println!(
            "💡 Note: This is a limited scan. For comprehensive results, use the DexScreener API above"
        );
    } else {
        println!("✅ Found {} pools containing target token via on-chain scan", found_pools.len());
    }

    Ok(found_pools.len())
}

async fn check_pool_contains_token(
    rpc_manager: &Arc<RpcManager>,
    pool_address: &str,
    token_pubkey: &Pubkey
) -> Result<bool> {
    let pool_pubkey = Pubkey::from_str(pool_address)?;

    // Get the pool account
    let account = rpc_manager.get_account(&pool_pubkey).await?;

    // Quick check: scan the account data for the token address
    let token_bytes = token_pubkey.to_bytes();

    // Look for the token address in the pool data
    let data = &account.data;
    for window in data.windows(32) {
        if window == token_bytes {
            return Ok(true);
        }
    }

    Ok(false)
}
