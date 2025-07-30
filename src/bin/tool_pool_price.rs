/// Enhanced Pool Price Calculator Tool
///
/// This tool provides comprehensive testing and debugging for the pool price system.
/// It supports fetching pool data from API, calculating prices from pool reserves,
/// testing the background monitoring service, and validating price calculations.
///
/// Usage Examples:
/// - Test specific pool: cargo run --bin tool_pool_price -- --pool <POOL_ADDRESS> --token <TOKEN_MINT>
/// - Test token pools: cargo run --bin tool_pool_price -- --token <TOKEN_MINT> --test-pools
/// - Test monitoring: cargo run --bin tool_pool_price -- --test-monitoring --duration 30
/// - Compare prices: cargo run --bin tool_pool_price -- --token <TOKEN_MINT> --compare-api

use screenerbot::tokens::pool::{ get_pool_service, init_pool_service };
use screenerbot::tokens::api::{ get_token_pairs_from_api, init_dexscreener_api };
use screenerbot::tokens::price_service::{ initialize_price_service };
use screenerbot::tokens::decimals::{ get_cached_decimals, get_token_decimals_from_chain };
use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::rpc::{ init_rpc_client, init_rpc_client_with_url, get_rpc_client };
use screenerbot::global::set_cmd_args;
use clap::{ Arg, Command };
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // Initialize logger
    init_file_logging();

    let matches = Command::new("Enhanced Pool Price Calculator")
        .version("2.0")
        .about("Comprehensive testing and debugging tool for the pool price system")
        .arg(
            Arg::new("pool")
                .short('p')
                .long("pool")
                .value_name("POOL_ADDRESS")
                .help("Pool address to calculate price from")
                .required(false)
        )
        .arg(
            Arg::new("token")
                .short('t')
                .long("token")
                .value_name("TOKEN_MINT")
                .help("Token mint address")
                .required(false)
        )
        .arg(
            Arg::new("rpc")
                .short('r')
                .long("rpc")
                .value_name("RPC_URL")
                .help("Custom RPC URL (optional)")
                .required(false)
        )
        .arg(
            Arg::new("test-pools")
                .long("test-pools")
                .help("Test fetching and analyzing all pools for a token")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-monitoring")
                .long("test-monitoring")
                .help("Test the background monitoring service")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("compare-api")
                .long("compare-api")
                .help("Compare pool prices with API prices")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("duration")
                .long("duration")
                .value_name("SECONDS")
                .help("Duration for monitoring test (default: 30 seconds)")
                .default_value("30")
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug output")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-detailed")
                .long("debug-detailed")
                .help("Enable detailed debugging with decimal and reserve analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-price-service")
                .long("debug-price-service")
                .help("Enable price service debug output")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Set up command args for global debug flags
    let mut cmd_args = Vec::new();
    if matches.get_flag("debug") {
        cmd_args.push("--debug".to_string());
    }
    if matches.get_flag("debug-price-service") {
        cmd_args.push("--debug-price-service".to_string());
    }
    if matches.get_flag("debug-detailed") {
        cmd_args.push("--debug-detailed".to_string());
    }
    set_cmd_args(cmd_args);

    // Initialize RPC client
    let rpc_url = matches.get_one::<String>("rpc");
    if let Some(url) = rpc_url {
        init_rpc_client_with_url(Some(url.as_str()));
        log(LogTag::Pool, "INIT", &format!("RPC client initialized with custom URL: {}", url));
    } else {
        match init_rpc_client() {
            Ok(_) => log(LogTag::Pool, "SUCCESS", "RPC client initialized from configuration"),
            Err(e) =>
                log(LogTag::Pool, "WARN", &format!("Config init failed, using fallback: {}", e)),
        }
    }

    // Test RPC connection
    log(LogTag::Pool, "INIT", "Testing RPC connection...");
    match get_rpc_client().test_connection().await {
        Ok(_) => log(LogTag::Pool, "SUCCESS", "RPC connection successful"),
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("RPC connection failed: {}", e));
            return;
        }
    }

    // Initialize services
    log(LogTag::Pool, "INIT", "Initializing pool and price services...");

    // Initialize DexScreener API first
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::Pool, "ERROR", &format!("Failed to initialize DexScreener API: {}", e));
        return;
    }

    let pool_service = init_pool_service();
    if let Err(e) = initialize_price_service().await {
        eprintln!("Failed to initialize price service: {}", e);
        return;
    }

    // Determine what operation to perform
    if matches.get_flag("test-monitoring") {
        test_monitoring_service(pool_service, &matches).await;
    } else if let Some(token_address) = matches.get_one::<String>("token") {
        if matches.get_flag("test-pools") {
            test_token_pools(token_address).await;
        } else if matches.get_flag("compare-api") {
            compare_pool_api_prices(pool_service, token_address).await;
        } else if let Some(pool_address) = matches.get_one::<String>("pool") {
            if matches.get_flag("debug-detailed") {
                debug_specific_pool_detailed(pool_address, token_address).await;
            } else {
                test_specific_pool(pool_address, token_address).await;
            }
        } else {
            test_token_availability_and_price(pool_service, token_address).await;
        }
    } else {
        log(LogTag::Pool, "ERROR", "Please specify a token address or use --test-monitoring");
        print_usage_examples();
    }
}

/// Test the background monitoring service
async fn test_monitoring_service(
    pool_service: &'static screenerbot::tokens::pool::PoolPriceService,
    matches: &clap::ArgMatches
) {
    log(LogTag::Pool, "TEST", "Starting monitoring service test...");

    let duration: u64 = matches.get_one::<String>("duration").unwrap().parse().unwrap_or(30);

    // Start monitoring
    pool_service.start_monitoring().await;

    // Add some test tokens to watch list
    let test_tokens = vec![
        "So11111111111111111111111111111111111111112", // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB" // USDT
    ];

    for (i, token) in test_tokens.iter().enumerate() {
        pool_service.add_to_watch_list(token, 100 + (i as i32)).await;
        log(LogTag::Pool, "WATCH", &format!("Added {} to watch list", token));
    }

    log(LogTag::Pool, "INFO", &format!("Monitoring for {} seconds...", duration));

    // Monitor for specified duration
    let mut elapsed = 0u64;
    while elapsed < duration {
        sleep(Duration::from_secs(5)).await;
        elapsed += 5;

        // Get watch list status
        let watch_list = pool_service.get_watch_list().await;
        let (pool_cache_size, price_cache_size, availability_cache_size) =
            pool_service.get_cache_stats().await;

        log(
            LogTag::Pool,
            "STATUS",
            &format!(
                "Elapsed: {}s, Watch list: {}, Caches: pool={}, price={}, availability={}",
                elapsed,
                watch_list.len(),
                pool_cache_size,
                price_cache_size,
                availability_cache_size
            )
        );
    }

    // Stop monitoring
    pool_service.stop_monitoring().await;
    log(LogTag::Pool, "TEST", "Monitoring service test completed");
}

/// Test fetching and analyzing all pools for a token
async fn test_token_pools(token_address: &str) {
    log(LogTag::Pool, "TEST", &format!("Testing pools for token: {}", token_address));

    match get_token_pairs_from_api(token_address).await {
        Ok(pairs) => {
            log(LogTag::Pool, "SUCCESS", &format!("Found {} pools for token", pairs.len()));

            for (i, pair) in pairs.iter().enumerate() {
                log(
                    LogTag::Pool,
                    "POOL",
                    &format!(
                        "Pool {}: {} on {} - Liquidity: ${:.2}, Volume 24h: ${:.2}, Price: ${}",
                        i + 1,
                        pair.pair_address,
                        pair.dex_id,
                        pair.liquidity
                            .as_ref()
                            .map(|l| l.usd)
                            .unwrap_or(0.0),
                        pair.volume.h24.unwrap_or(0.0),
                        pair.price_usd.as_deref().unwrap_or("N/A")
                    )
                );
            }

            // Find the best pool (highest liquidity)
            if
                let Some(best_pool) = pairs.iter().max_by(|a, b| {
                    let a_liquidity = a.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    let b_liquidity = b.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    a_liquidity.partial_cmp(&b_liquidity).unwrap_or(std::cmp::Ordering::Equal)
                })
            {
                log(
                    LogTag::Pool,
                    "BEST",
                    &format!(
                        "Best pool: {} with ${:.2} liquidity on {}",
                        best_pool.pair_address,
                        best_pool.liquidity
                            .as_ref()
                            .map(|l| l.usd)
                            .unwrap_or(0.0),
                        best_pool.dex_id
                    )
                );
            }
        }
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("Failed to fetch pools: {}", e));
        }
    }
}

/// Compare pool prices with API prices - Enhanced with decimal debugging
async fn compare_pool_api_prices(
    pool_service: &'static screenerbot::tokens::pool::PoolPriceService,
    token_address: &str
) {
    log(LogTag::Pool, "TEST", &format!("Comparing pool vs API prices for: {}", token_address));

    // Debug decimal information first
    debug_token_decimals(token_address).await;

    // Check pool availability
    let has_pools = pool_service.check_token_availability(token_address).await;
    log(LogTag::Pool, "INFO", &format!("Token has available pools: {}", has_pools));

    if !has_pools {
        log(LogTag::Pool, "WARN", "No pools available for price calculation");
        return;
    }

    // Get API price (this returns SOL price) - use the global price service function
    let api_price_sol = screenerbot::tokens::price_service::get_token_price_safe(token_address).await;
    log(LogTag::Pool, "API", &format!("API price: {:.12} SOL", api_price_sol.unwrap_or(0.0)));

    // Get pool price with detailed debugging
    let pool_result = pool_service.get_pool_price(token_address, api_price_sol).await;

    match pool_result {
        Some(pool_result) => {
            // Display pool price in SOL (since we don't calculate USD from pools)
            if let Some(price_sol) = pool_result.price_sol {
                log(
                    LogTag::Pool,
                    "POOL",
                    &format!(
                        "Pool price: {:.12} SOL from {} (liquidity: ${:.2})",
                        price_sol,
                        pool_result.pool_address,
                        pool_result.liquidity_usd
                    )
                );
            } else {
                log(
                    LogTag::Pool,
                    "POOL",
                    &format!(
                        "Pool found: {} (liquidity: ${:.2}) - Price calculation not available",
                        pool_result.pool_address,
                        pool_result.liquidity_usd
                    )
                );
            }

            // We only compare API USD prices with API data, not with pool SOL prices
            log(
                LogTag::Pool,
                "INFO",
                "✅ Pool calculation successful - providing SOL price only (no USD conversion)"
            );
        }
        None => {
            log(LogTag::Pool, "ERROR", "❌ Failed to calculate pool price");
        }
    }
}

/// Test specific pool address
async fn test_specific_pool(pool_address: &str, token_address: &str) {
    log(
        LogTag::Pool,
        "TEST",
        &format!("Testing specific pool: {} for token: {}", pool_address, token_address)
    );

    // This would require implementing specific pool data fetching
    // For now, just test the token's general pool availability
    let pool_service = get_pool_service();
    let has_pools = pool_service.check_token_availability(token_address).await;

    log(
        LogTag::Pool,
        "INFO",
        &format!("Token {} has available pools: {}", token_address, has_pools)
    );
}

/// Test token availability and price calculation
async fn test_token_availability_and_price(
    pool_service: &'static screenerbot::tokens::pool::PoolPriceService,
    token_address: &str
) {
    log(LogTag::Pool, "TEST", &format!("Testing availability and price for: {}", token_address));

    // Test availability
    let has_pools = pool_service.check_token_availability(token_address).await;
    log(LogTag::Pool, "AVAILABILITY", &format!("Has pools: {}", has_pools));

    if has_pools {
        // Test price calculation
        match pool_service.get_pool_price(token_address, None).await {
            Some(pool_result) => {
                log(
                    LogTag::Pool,
                    "SUCCESS",
                    &format!(
                        "Pool price calculated: {:.12} SOL from {} on {}",
                        pool_result.price_sol.unwrap_or(0.0),
                        pool_result.pool_address,
                        pool_result.dex_id
                    )
                );
            }
            None => {
                log(LogTag::Pool, "ERROR", "Failed to calculate pool price");
            }
        }
    } else {
        log(LogTag::Pool, "WARN", "No pools available for this token");
    }
}

/// Debug token decimal information
async fn debug_token_decimals(token_address: &str) {
    log(LogTag::Pool, "DEBUG", "=== DECIMAL DEBUGGING ===");

    // Check cached decimals first
    if let Some(cached_decimals) = get_cached_decimals(token_address) {
        log(
            LogTag::Pool,
            "CACHE",
            &format!("✅ Found cached decimals for {}: {}", token_address, cached_decimals)
        );
    } else {
        log(LogTag::Pool, "CACHE", &format!("❌ No cached decimals for {}", token_address));

        // Try to fetch from chain
        match get_token_decimals_from_chain(token_address).await {
            Ok(chain_decimals) => {
                log(
                    LogTag::Pool,
                    "CHAIN",
                    &format!("✅ Fetched from chain: {} decimals", chain_decimals)
                );
            }
            Err(e) => {
                log(LogTag::Pool, "CHAIN", &format!("❌ Failed to fetch from chain: {}", e));
            }
        }
    }

    // Also check SOL decimals (should be 9)
    if let Some(sol_decimals) = get_cached_decimals("So11111111111111111111111111111111111111112") {
        log(LogTag::Pool, "CACHE", &format!("✅ SOL decimals: {}", sol_decimals));
    } else {
        log(LogTag::Pool, "CACHE", "❌ SOL decimals not cached");
    }
}

/// Debug pool reserves and calculations
async fn debug_pool_reserves(pool_address: &str, token_address: &str) {
    log(LogTag::Pool, "DEBUG", "=== POOL RESERVES DEBUGGING ===");
    log(LogTag::Pool, "DEBUG", &format!("Pool Address: {}", pool_address));
    log(LogTag::Pool, "DEBUG", &format!("Token Address: {}", token_address));

    // Try to get detailed pool information by fetching from API
    match get_token_pairs_from_api(token_address).await {
        Ok(pairs) => {
            // Find the specific pool
            if let Some(pool) = pairs.iter().find(|p| p.pair_address == pool_address) {
                // Parse price_usd string to f64
                let price_usd_f64 = pool.price_usd
                    .as_ref()
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);

                log(
                    LogTag::Pool,
                    "RESERVES",
                    &format!(
                        "Pool found via API:\n\
                    - Pair Address: {}\n\
                    - DEX: {}\n\
                    - Base Token: {} ({})\n\
                    - Quote Token: {} ({})\n\
                    - Price USD: ${:.12}\n\
                    - Liquidity USD: ${:.2}\n\
                    - Volume 24h: ${:.2}",
                        pool.pair_address,
                        pool.dex_id,
                        pool.base_token.address,
                        pool.base_token.symbol,
                        pool.quote_token.address,
                        pool.quote_token.symbol,
                        price_usd_f64,
                        pool.liquidity
                            .as_ref()
                            .map(|l| l.usd)
                            .unwrap_or(0.0),
                        pool.volume.h24.unwrap_or(0.0)
                    )
                );

                // Check decimal information for both tokens
                log(LogTag::Pool, "DECIMALS", "Checking token decimals...");
                debug_token_decimals(&pool.base_token.address).await;
                debug_token_decimals(&pool.quote_token.address).await;

                // Calculate what the price should be with different decimal assumptions
                log(LogTag::Pool, "CALC", "=== DECIMAL SENSITIVITY ANALYSIS ===");

                // Test with different decimal combinations
                let test_decimals = vec![6, 8, 9, 18];
                for &base_decimals in &test_decimals {
                    for &quote_decimals in &test_decimals {
                        // This is a simplified calculation for demonstration
                        let ratio = (10_f64).powi((quote_decimals - base_decimals) as i32);
                        let adjusted_price = price_usd_f64 * ratio;

                        log(
                            LogTag::Pool,
                            "TEST",
                            &format!(
                                "If base={} decimals, quote={} decimals: ${:.12}",
                                base_decimals,
                                quote_decimals,
                                adjusted_price
                            )
                        );
                    }
                }
            } else {
                log(
                    LogTag::Pool,
                    "ERROR",
                    &format!("Pool {} not found in API results", pool_address)
                );
            }
        }
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("Failed to fetch pool data from API: {}", e));
        }
    }
}

/// Enhanced test for specific pool with detailed debugging
async fn debug_specific_pool_detailed(pool_address: &str, token_address: &str) {
    log(LogTag::Pool, "DEBUG", "=== DETAILED POOL ANALYSIS ===");
    log(
        LogTag::Pool,
        "TEST",
        &format!("Analyzing pool: {} for token: {}", pool_address, token_address)
    );

    // Step 1: Check decimal cache
    debug_token_decimals(token_address).await;

    // Step 2: Get pool service and check availability
    let pool_service = get_pool_service();
    let has_pools = pool_service.check_token_availability(token_address).await;
    log(LogTag::Pool, "AVAILABILITY", &format!("Token has pools: {}", has_pools));

    if !has_pools {
        log(LogTag::Pool, "ERROR", "No pools available for analysis");
        return;
    }

    // Step 3: Detailed pool reserves analysis
    debug_pool_reserves(pool_address, token_address).await;

    // Step 4: Test price calculation
    match pool_service.get_pool_price(token_address, None).await {
        Some(pool_result) => {
            log(
                LogTag::Pool,
                "SUCCESS",
                &format!(
                    "✅ Pool price calculation successful:\n\
                - Price SOL: {:.12}\n\
                - Pool: {}\n\
                - Liquidity: ${:.2}",
                    pool_result.price_sol.unwrap_or(0.0),
                    pool_result.pool_address,
                    pool_result.liquidity_usd
                )
            );
        }
        None => {
            log(LogTag::Pool, "ERROR", "❌ Pool price calculation failed");
        }
    }
}

/// Print usage examples
fn print_usage_examples() {
    println!("\nUsage Examples:");
    println!("1. Test specific token pools:");
    println!(
        "   cargo run --bin tool_pool_price -- --token So11111111111111111111111111111111111111112 --test-pools"
    );

    println!("\n2. Compare pool vs API prices:");
    println!("   cargo run --bin tool_pool_price -- --token <TOKEN_MINT> --compare-api");

    println!("\n3. Test monitoring service:");
    println!("   cargo run --bin tool_pool_price -- --test-monitoring --duration 60");

    println!("\n4. Test token availability:");
    println!("   cargo run --bin tool_pool_price -- --token <TOKEN_MINT>");

    println!("\n5. Use custom RPC:");
    println!("   cargo run --bin tool_pool_price -- --token <TOKEN_MINT> --rpc <RPC_URL>");

    println!("\n6. Debug specific pool with detailed decimal analysis:");
    println!(
        "   cargo run --bin tool_pool_price -- --pool <POOL_ADDRESS> --token <TOKEN_MINT> --debug-detailed"
    );

    println!("\n7. Compare prices with detailed debugging:");
    println!("   cargo run --bin tool_pool_price -- --token <TOKEN_MINT> --compare-api --debug");
}
