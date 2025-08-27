use std::time::Duration;
use tokio::time::sleep;
use chrono::Utc;
use screenerbot::tokens::pool::{ get_pool_service, get_price, PriceOptions };
use screenerbot::tokens;
use screenerbot::logger::init_file_logging;
use screenerbot::tokens::decimals::get_cached_decimals;

/// Test token address to monitor
const TEST_TOKEN: &str = "A8C3xuqscfmyLrte3VmTqrAq8kgMASius9AFNANwpump";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Price Monitor for: {}", TEST_TOKEN);
    println!("============================================");

    // Initialize logging
    init_file_logging();
    println!("✅ Logging initialized");

    // Initialize token database
    let _token_database = match tokens::TokenDatabase::new() {
        Ok(db) => {
            println!("✅ Token database initialized");
            db
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize token database: {}", e);
            return Err(e.into());
        }
    };

    // Initialize price service
    println!("🚀 Initializing price service...");
    if let Err(e) = tokens::initialize_price_service().await {
        eprintln!("❌ Failed to initialize price service: {}", e);
        return Err(e.into());
    }
    println!("✅ Price service initialized");

    // Initialize DexScreener API
    println!("🚀 Initializing DexScreener API...");
    if let Err(e) = tokens::init_dexscreener_api().await {
        eprintln!("⚠️ Failed to initialize DexScreener API: {}", e);
        println!("🔄 Continuing anyway - will use cached data if available");
    } else {
        println!("✅ DexScreener API initialized");
    }

    // Initialize the pool service
    let pool_service = get_pool_service();

    // Start monitoring if not active
    if !pool_service.is_monitoring_active().await {
        println!("🚀 Starting pool monitoring...");
        pool_service.start_monitoring().await;
        sleep(Duration::from_secs(3)).await;
    }

    let mut last_price: Option<f64> = None;
    let mut iteration = 0;

    println!("📊 Starting price monitoring (10s intervals)...");
    println!();

    loop {
        iteration += 1;
        let timestamp = Utc::now();

        println!("🔄 Check #{} - {}", iteration, timestamp.format("%H:%M:%S"));

        // Check actual token decimals from the system
        let token_decimals = get_cached_decimals(TEST_TOKEN).unwrap_or(6);
        let sol_decimals = 9; // SOL is always 9 decimals
        println!("   🔢 Token decimals: {} | SOL decimals: {}", token_decimals, sol_decimals);

        // Try comprehensive price first with debug pool calculation enabled and force fresh
        let mut price_options = PriceOptions::comprehensive();
        price_options.force_refresh = true; // Force fresh data to see real market changes

        match get_price(TEST_TOKEN, Some(price_options), true).await {
            // Enable debug mode
            Some(result) => {
                println!("   🔍 DEBUG: Full result = {:?}", result);

                // Use API price when available (more accurate), fallback to pool price
                let best_price = result.api_price_sol
                    .or(result.pool_price_sol)
                    .or(result.price_sol);

                if let Some(price_sol) = best_price {
                    // Check for price change
                    if let Some(last) = last_price {
                        let change_pct = ((price_sol - last) / last) * 100.0;
                        let change_icon = if change_pct > 0.0 {
                            "📈"
                        } else if change_pct < 0.0 {
                            "📉"
                        } else {
                            "➡️"
                        };

                        println!(
                            "   💰 Price: {:.9} SOL {} ({:+.2}%)",
                            price_sol,
                            change_icon,
                            change_pct
                        );

                        if change_pct.abs() >= 1.0 {
                            println!("   🚨 SIGNIFICANT CHANGE: {:+.2}%", change_pct);
                        }
                    } else {
                        println!("   💰 Price: {:.9} SOL (initial)", price_sol);
                    }

                    // Show price comparison if both available
                    if let (Some(api), Some(pool)) = (result.api_price_sol, result.pool_price_sol) {
                        let ratio = pool / api;
                        println!(
                            "   ⚖️  API: {:.9} SOL | Pool: {:.9} SOL (ratio: {:.1}x)",
                            api,
                            pool,
                            ratio
                        );
                    }

                    last_price = Some(price_sol);

                    // Show additional details
                    if let Some(usd_price) = result.price_usd {
                        println!("   💵 USD: ${:.6}", usd_price);
                    }

                    println!("   📊 Source: {}", result.source);

                    if let Some(pool_addr) = &result.pool_address {
                        println!(
                            "   🏊 Pool: {}...{}",
                            &pool_addr[..8],
                            &pool_addr[pool_addr.len() - 8..]
                        );
                    }

                    if let Some(pool_type) = &result.pool_type {
                        println!("   🔧 Pool Type: {}", pool_type);
                    }

                    if let Some(dex_id) = &result.dex_id {
                        println!("   📋 DEX: {}", dex_id);
                    }

                    if let Some(liquidity) = result.liquidity_usd {
                        println!("   💧 Liquidity: ${:.0}", liquidity);
                    }
                } else {
                    println!("   ❌ No price available");
                }
            }
            None => {
                println!("   ❌ Failed to get price");
            }
        }

        // Show availability status
        let availability = pool_service.check_token_availability(TEST_TOKEN).await;
        println!("   🎯 Pools available: {}", if availability { "✅" } else { "❌" });

        // Show history count with more detail
        let history = pool_service.get_recent_price_history(TEST_TOKEN).await;
        if !history.is_empty() {
            println!("   📈 Recent history entries: {}", history.len());
            if history.len() > 1 {
                // Show last few entries for debugging
                let last_entries: Vec<_> = history.iter().take(3).collect();
                for (i, entry) in last_entries.iter().enumerate() {
                    println!(
                        "       [{}] {}: {:.9} SOL",
                        i + 1,
                        entry.0.format("%H:%M:%S"),
                        entry.1
                    );
                }
            }
        } else {
            println!("   📈 No price history available");
        }

        // Check pools info cache specifically
        if let Some(pools_info) = pool_service.get_cached_pools_infos(TEST_TOKEN).await {
            println!("   🏊 Cached pools count: {}", pools_info.len());
            for (i, pool) in pools_info.iter().take(2).enumerate() {
                println!(
                    "       Pool {}: {} ({}) - ${:.0}",
                    i + 1,
                    pool.pair_address[..8].to_string() + "...",
                    pool.dex_id,
                    pool.liquidity_usd
                );
            }
        } else {
            println!("   🏊 No cached pools info");
        }

        println!();

        // Show stats every 10 iterations
        if iteration % 10 == 0 {
            let stats = pool_service.get_enhanced_stats().await;
            println!("📊 SERVICE STATS (after {} checks):", iteration);
            println!("   Success rate: {:.1}%", stats.get_success_rate() * 100.0);
            println!("   Cache hit rate: {:.1}%", stats.get_cache_hit_rate() * 100.0);
            println!("   Total requests: {}", stats.total_price_requests);
            println!();
        }

        // Wait 5 seconds (shorter interval to catch micro-changes)
        sleep(Duration::from_secs(5)).await;
    }
}
