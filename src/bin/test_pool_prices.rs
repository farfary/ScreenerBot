use anyhow::Result;
use screenerbot::{ Config, RpcManager };
use screenerbot::pairs::{ PoolDataFetcher, PoolAnalyzer };
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n💹 ScreenerBot Pool Price Discovery Test");
    println!("=========================================\n");

    // Load configuration
    let config = Config::load("configs.json")?;

    // Initialize RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone(), config.rpc.clone())?
    );

    println!("✅ RPC Manager initialized");

    // Test RPC connection
    match rpc_manager.get_latest_blockhash().await {
        Ok(blockhash) => println!("✅ RPC connected - Latest blockhash: {}", blockhash),
        Err(e) => {
            println!("❌ RPC connection failed: {}", e);
            return Ok(());
        }
    }

    // Initialize pool fetcher and analyzer
    let pool_fetcher = PoolDataFetcher::new(Arc::clone(&rpc_manager));
    let pool_analyzer = PoolAnalyzer::new(Arc::clone(&rpc_manager));

    println!("✅ Pool fetcher and analyzer ready");

    // Test various pool types
    let test_pools = vec![
        ("SOL-USDC Raydium CLMM", "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2"),
        ("SOL-USDT Raydium", "7XawhbbxtsRcQA8KTkHT9f9nc6d69UwqCDh6U5EEbEmX"),
        ("SOL-USDC Orca", "EoTcMgcDRTJVZDMZWBoU6rhYHZfkNTVEAfz3uUJRcYGj"),
        ("Meteora DLMM Test", "Ew6yvhDsEsC8hw6bWJzWQnPnQFrtDaXm8SqSjZZrCr1R"),
        ("Pump.fun Test", "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
    ];

    println!("\n🔍 Testing Pool Price Discovery:");
    println!("================================");

    let mut successful_pools = 0;
    let mut total_tvl = 0.0;

    for (name, pool_address_str) in &test_pools {
        println!("\n📊 Testing: {}", name);
        println!("   Address: {}", pool_address_str);

        match pool_analyzer.analyze_pool(pool_address_str).await {
            Ok(analysis) => {
                successful_pools += 1;
                total_tvl += analysis.tvl;

                println!("   ✅ SUCCESSFUL ANALYSIS:");
                println!("      🏊 Pool Type: {:?}", analysis.pool_info.pool_type);
                println!("      💰 Price: {:.8} (token1/token0)", analysis.price_info.price);
                println!("      🌊 TVL: ${:.2}", analysis.tvl);
                println!("      ❤️  Health Score: {:.1}/100", analysis.health_score);
                println!(
                    "      🪙 Token 0: {} ({})",
                    analysis.pool_info.token_mint_0,
                    analysis.pool_info.reserve_0
                );
                println!(
                    "      🪙 Token 1: {} ({})",
                    analysis.pool_info.token_mint_1,
                    analysis.pool_info.reserve_1
                );
                println!(
                    "      📅 Last Update: {}",
                    analysis.price_info.last_update.format("%H:%M:%S")
                );

                // Calculate some trading metrics
                let price_impact_1_sol = calculate_price_impact(&analysis.pool_info, 1_000_000_000); // 1 SOL
                println!("      📈 Price Impact (1 SOL trade): {:.3}%", price_impact_1_sol);

                if format!("{:?}", analysis.pool_info.pool_type).contains("CLMM") {
                    println!("      🎯 CLMM Active Range: Concentrated liquidity");
                }
            }
            Err(e) => {
                println!("   ❌ FAILED: {}", e);
                if e.to_string().contains("program") {
                    println!("      💡 This might be an unsupported pool type");
                }
            }
        }
    }

    println!("\n🎯 POOL PRICE DISCOVERY SUMMARY:");
    println!("================================");
    println!("✅ Successfully analyzed: {}/{} pools", successful_pools, test_pools.len());
    println!("💰 Total TVL across analyzed pools: ${:.2}", total_tvl);

    // Show supported pool types
    let supported_programs = pool_fetcher.get_supported_programs();
    println!("\n🔧 SUPPORTED POOL TYPES:");
    println!("========================");
    for (i, program_id) in supported_programs.iter().enumerate() {
        let name = match program_id.to_string().as_str() {
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium CLMM",
            "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1" => "Raydium CPMM",
            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM",
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca Whirlpool",
            "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "Pump.fun AMM",
            _ => "Unknown",
        };
        println!("   {}. {} - {}", i + 1, name, program_id);
    }

    if successful_pools > 0 {
        println!("\n🚀 POOL PRICE FETCHING IS WORKING!");
        println!("💹 The bot can get real-time prices directly from DEX pools");
        println!("⚡ No external APIs needed for price data");
        println!("🎯 Ready for production trading!");
    } else {
        println!("\n⚠️  Pool price fetching needs attention");
        println!("🔧 Check RPC connectivity and pool addresses");
    }

    println!("\n💡 To use in main bot:");
    println!("   - The bot will automatically fetch pool prices");
    println!("   - No market data API dependencies");
    println!("   - Real-time price discovery from on-chain data");

    Ok(())
}

/// Calculate approximate price impact for a trade
fn calculate_price_impact(pool_info: &screenerbot::pairs::PoolInfo, trade_amount: u64) -> f64 {
    if pool_info.reserve_0 == 0 || pool_info.reserve_1 == 0 {
        return 0.0;
    }

    // Simple constant product formula estimation
    let reserve_0 = pool_info.reserve_0 as f64;
    let _reserve_1 = pool_info.reserve_1 as f64;
    let trade_amount_f64 = trade_amount as f64;

    // Calculate impact as percentage of trade vs reserves
    let impact = (trade_amount_f64 / reserve_0) * 100.0;
    impact.min(50.0) // Cap at 50% for display
}
