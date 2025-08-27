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

use screenerbot::tokens::pool::{ get_pool_service, init_pool_service, initialize_price_service };
use screenerbot::tokens::dexscreener::{ get_token_pairs_from_api, init_dexscreener_api };
use screenerbot::tokens::decimals::{ get_cached_decimals, get_token_decimals_from_chain };
use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::rpc::{ init_rpc_client, get_rpc_client };
use screenerbot::arguments::set_cmd_args;
use clap::{ Arg, Command };
use std::time::Duration;
use std::str::FromStr;
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
        .arg(
            Arg::new("debug-pool-prices")
                .long("debug-pool-prices")
                .help("Enable pool price calculation debug output")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-direct")
                .long("test-direct")
                .help("Test pool directly via blockchain decoder (bypasses API discovery)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("list-pools")
                .long("list-pools")
                .help("List all available pools for a token with detailed information")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("test-pool-direct")
                .long("test-pool-direct")
                .help("Test a specific pool address directly (requires --pool)")
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
    if matches.get_flag("debug-pool-prices") {
        cmd_args.push("--debug-pool-prices".to_string());
    }
    set_cmd_args(cmd_args);

    // Initialize RPC client from configuration
    match init_rpc_client() {
        Ok(_) => log(LogTag::Pool, "SUCCESS", "RPC client initialized from configuration"),
        Err(e) => {
            eprintln!("Failed to initialize RPC client: {}", e);
            std::process::exit(1);
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
    } else if let Some(pool_address) = matches.get_one::<String>("pool") {
        if matches.get_flag("test-pool-direct") {
            test_pool_address_direct(pool_address).await;
        } else if let Some(token_address) = matches.get_one::<String>("token") {
            if matches.get_flag("test-direct") {
                test_pool_direct(pool_address, token_address).await;
            } else if matches.get_flag("debug-detailed") {
                debug_specific_pool_detailed(pool_address, token_address).await;
            } else {
                test_specific_pool(pool_address, token_address).await;
            }
        } else {
            log(
                LogTag::Pool,
                "ERROR",
                "Pool address specified but no token address provided. Use --token <TOKEN_MINT>"
            );
            print_usage_examples();
        }
    } else if let Some(token_address) = matches.get_one::<String>("token") {
        if matches.get_flag("list-pools") {
            list_all_pools_for_token(token_address).await;
        } else if matches.get_flag("test-pools") {
            test_token_pools(token_address).await;
        } else if matches.get_flag("compare-api") {
            compare_pool_api_prices(pool_service, token_address).await;
        } else {
            test_token_availability_and_price(pool_service, token_address).await;
        }
    } else {
        log(LogTag::Pool, "ERROR", "Please specify a token address or use --test-monitoring");
        print_usage_examples();
    }
}

/// List all available pools for a token with detailed information
async fn list_all_pools_for_token(token_address: &str) {
    log(LogTag::Pool, "LIST_POOLS", &format!("🔍 Listing all pools for token: {}", token_address));

    match get_token_pairs_from_api(token_address).await {
        Ok(pairs) => {
            if pairs.is_empty() {
                log(LogTag::Pool, "NO_POOLS", "❌ No pools found for this token");
                return;
            }

            log(LogTag::Pool, "SUCCESS", &format!("✅ Found {} pools for token", pairs.len()));

            // Sort pools by liquidity (highest first)
            let mut sorted_pairs = pairs.clone();
            sorted_pairs.sort_by(|a, b| {
                let a_liquidity = a.liquidity
                    .as_ref()
                    .map(|l| l.usd)
                    .unwrap_or(0.0);
                let b_liquidity = b.liquidity
                    .as_ref()
                    .map(|l| l.usd)
                    .unwrap_or(0.0);
                b_liquidity.partial_cmp(&a_liquidity).unwrap_or(std::cmp::Ordering::Equal)
            });

            println!("
📊 POOL LIST (sorted by liquidity):");
            println!("================================================================");

            for (i, pair) in sorted_pairs.iter().enumerate() {
                let liquidity = pair.liquidity
                    .as_ref()
                    .map(|l| l.usd)
                    .unwrap_or(0.0);
                let volume_24h = pair.volume.h24.unwrap_or(0.0);
                let price_usd = pair.price_usd.as_deref().unwrap_or("N/A");

                println!("{}. Pool Address: {}", i + 1, pair.pair_address);
                println!(
                    "   DEX:           {} ({})",
                    get_dex_display_name(&pair.dex_id),
                    pair.dex_id
                );
                println!("   Liquidity:     ${:.2}", liquidity);
                println!("   Volume 24h:    ${:.2}", volume_24h);
                println!("   Price USD:     ${}", price_usd);
                println!(
                    "   Base Token:    {} ({})",
                    pair.base_token.symbol,
                    pair.base_token.address
                );
                println!(
                    "   Quote Token:   {} ({})",
                    pair.quote_token.symbol,
                    pair.quote_token.address
                );

                // Try to identify pool program type
                let pool_type = identify_pool_program_type(&pair.dex_id, &pair.pair_address).await;
                println!("   Pool Type:     {}", pool_type);
                println!("   ─────────────────────────────────────────────────────────────");
            }

            // Show summary statistics
            let total_liquidity: f64 = sorted_pairs
                .iter()
                .map(|p|
                    p.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0)
                )
                .sum();
            let total_volume: f64 = sorted_pairs
                .iter()
                .map(|p| p.volume.h24.unwrap_or(0.0))
                .sum();

            println!("
📈 SUMMARY:");
            println!("Total Pools:     {}", sorted_pairs.len());
            println!("Total Liquidity: ${:.2}", total_liquidity);
            println!("Total Volume:    ${:.2}", total_volume);

            // Show DEX distribution
            let mut dex_counts = std::collections::HashMap::new();
            for pair in &sorted_pairs {
                *dex_counts.entry(pair.dex_id.clone()).or_insert(0) += 1;
            }

            println!("
🏪 DEX DISTRIBUTION:");
            for (dex, count) in dex_counts {
                println!("  {}: {} pools", get_dex_display_name(&dex), count);
            }
        }
        Err(e) => {
            log(LogTag::Pool, "ERROR", &format!("❌ Failed to fetch pools: {}", e));
        }
    }
}

/// Test a specific pool address directly without needing token address
async fn test_pool_address_direct(pool_address: &str) {
    log(
        LogTag::Pool,
        "DIRECT_POOL_TEST",
        &format!("🎯 Testing pool address directly: {}", pool_address)
    );

    // First, dump the raw pool data for analysis
    dump_pool_hex_data(pool_address).await;

    // Try to determine what type of pool this is by fetching the account
    {
        let mut calculator = screenerbot::tokens::pool::PoolPriceCalculator::new();
        calculator.enable_debug();

        // Get raw pool data to analyze
        match calculator.get_raw_pool_data(pool_address).await {
            Ok(Some(data)) => {
                log(
                    LogTag::Pool,
                    "POOL_DATA",
                    &format!("✅ Retrieved pool data: {} bytes", data.len())
                );

                // Try to identify the pool program type from the account owner
                match
                    get_rpc_client().get_account(
                        &solana_sdk::pubkey::Pubkey::from_str(pool_address).unwrap()
                    ).await
                {
                    Ok(account) => {
                        let program_id = account.owner.to_string();
                        log(
                            LogTag::Pool,
                            "PROGRAM_ID",
                            &format!("Pool owned by program: {}", program_id)
                        );

                        let pool_type = match program_id.as_str() {
                            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium Legacy AMM",
                            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => "Pump.fun AMM",
                            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => "Raydium CPMM",
                            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM",
                            "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv" => "Meteora DAMM v2",
                            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Orca CAMM",
                            "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB" => "Moonshot",
                            _ => &format!("Unknown ({})", program_id),
                        };

                        log(LogTag::Pool, "POOL_TYPE", &format!("🏷️  Pool Type: {}", pool_type));

                        // Try to decode specific pool type
                        match program_id.as_str() {
                            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => {
                                test_pump_fun_pool_direct(pool_address, &data).await;
                            }
                            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
                                test_raydium_legacy_pool_direct(pool_address, &data).await;
                            }
                            "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => {
                                test_raydium_cpmm_pool_direct(pool_address, &data).await;
                            }
                            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => {
                                test_meteora_dlmm_pool_direct(pool_address, &data).await;
                            }
                            "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv" => {
                                test_meteora_damm_pool_direct(pool_address, &data).await;
                            }
                            _ => {
                                log(
                                    LogTag::Pool,
                                    "UNSUPPORTED",
                                    &format!("❌ Unsupported pool type: {}", pool_type)
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Pool,
                            "ERROR",
                            &format!("❌ Failed to get account info: {}", e)
                        );
                    }
                }
            }
            Ok(None) => {
                log(LogTag::Pool, "NOT_FOUND", "❌ Pool account not found");
            }
            Err(e) => {
                log(LogTag::Pool, "ERROR", &format!("❌ Failed to fetch pool data: {}", e));
            }
        }
    }
}

/// Get display name for DEX
fn get_dex_display_name(dex_id: &str) -> &str {
    match dex_id {
        "pumpswap" => "Pump.fun",
        "raydium" => "Raydium CPMM",
        "meteora" => "Meteora",
        "orca" => "Orca",
        "moonshot" => "Moonshot",
        _ => dex_id,
    }
}

/// Identify pool program type from DEX ID and pool address
async fn identify_pool_program_type(dex_id: &str, pool_address: &str) -> String {
    // Try to get the account to check program ID
    if let Ok(pubkey) = solana_sdk::pubkey::Pubkey::from_str(pool_address) {
        if let Ok(account) = get_rpc_client().get_account(&pubkey).await {
            let program_id = account.owner.to_string();
            return match program_id.as_str() {
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium Legacy AMM".to_string(),
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => "Pump.fun AMM".to_string(),
                "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => "Raydium CPMM".to_string(),
                "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM".to_string(),
                "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv" => "Meteora DAMM v2".to_string(),
                "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Orca CAMM".to_string(),
                "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB" => "Moonshot".to_string(),
                _ => format!("Unknown ({}) - {}", dex_id, program_id),
            };
        }
    }

    // Fallback to DEX ID mapping
    (
        match dex_id {
            "pumpswap" => "Pump.fun AMM (assumed)",
            "raydium" => "Raydium (assumed)",
            "meteora" => "Meteora (assumed)",
            "orca" => "Orca (assumed)",
            "moonshot" => "Moonshot (assumed)",
            _ => dex_id,
        }
    ).to_string()
}

/// Test Pump.fun pool directly
async fn test_pump_fun_pool_direct(pool_address: &str, data: &[u8]) {
    log(LogTag::Pool, "PUMP_TEST", &format!("🚀 Testing Pump.fun pool: {}", pool_address));

    // Pump.fun pool structure (300 bytes):
    // Offsets based on our successful implementation
    if data.len() >= 300 {
        // Extract reserves at known offsets
        if data.len() >= 152 {
            let token_reserve_bytes = &data[136..144];
            let sol_reserve_bytes = &data[144..152];

            let token_reserve = u64::from_le_bytes(
                token_reserve_bytes.try_into().unwrap_or([0; 8])
            );
            let sol_reserve = u64::from_le_bytes(sol_reserve_bytes.try_into().unwrap_or([0; 8]));

            log(
                LogTag::Pool,
                "PUMP_RESERVES",
                &format!("Token Reserve: {}, SOL Reserve: {}", token_reserve, sol_reserve)
            );

            // Calculate price (SOL per token)
            if token_reserve > 0 && sol_reserve > 0 {
                let price_sol =
                    (sol_reserve as f64) /
                    (10_f64).powi(9) /
                    ((token_reserve as f64) / (10_f64).powi(6));
                log(
                    LogTag::Pool,
                    "PUMP_PRICE",
                    &format!("✅ Calculated Price: {:.12} SOL per token", price_sol)
                );
            }
        }
    } else {
        log(
            LogTag::Pool,
            "PUMP_ERROR",
            &format!("❌ Invalid pool data size: {} bytes (expected 300)", data.len())
        );
    }
}

/// Test Raydium Legacy AMM pool directly
async fn test_raydium_legacy_pool_direct(pool_address: &str, data: &[u8]) {
    log(
        LogTag::Pool,
        "RAYDIUM_LEGACY",
        &format!("🔄 Testing Raydium Legacy AMM pool: {}", pool_address)
    );

    // Raydium Legacy AMM structure analysis
    log(LogTag::Pool, "RAYDIUM_DATA", &format!("Pool data size: {} bytes", data.len()));

    // Try to extract reserves using known Raydium Legacy offsets
    // These offsets would need to be determined from Raydium documentation or reverse engineering
    if data.len() >= 700 {
        // Estimated size for Raydium Legacy pools
        log(LogTag::Pool, "RAYDIUM_ANALYSIS", "Analyzing Raydium Legacy pool structure...");

        // Look for potential reserve values (64-bit integers)
        let mut potential_reserves = Vec::new();

        for i in (0..data.len().saturating_sub(8)).step_by(8) {
            if let Ok(value_bytes) = data[i..i + 8].try_into() {
                let value = u64::from_le_bytes(value_bytes);
                // Look for values that could be token reserves (reasonable range)
                if value > 1_000_000 && value < 1_000_000_000_000_000_000 {
                    potential_reserves.push((i, value));
                }
            }
        }

        log(
            LogTag::Pool,
            "RAYDIUM_RESERVES",
            &format!("Found {} potential reserve values:", potential_reserves.len())
        );
        for (offset, value) in &potential_reserves {
            log(LogTag::Pool, "RESERVE_CANDIDATE", &format!("  Offset {}: {}", offset, value));
        }

        // Try to find vault addresses (32-byte pubkeys) - but don't fetch them to avoid hanging
        log(LogTag::Pool, "VAULT_SEARCH", "🔍 Looking for potential vault addresses in pool data");

        // Search for known pubkeys in the data instead of making RPC calls
        search_for_known_pubkeys_in_data(data);

        // Try direct reserve extraction based on hex analysis
        if let Some((reserve_a, reserve_b)) = extract_reserves_from_raydium_legacy(data) {
            log(
                LogTag::Pool,
                "RESERVES_EXTRACTED",
                &format!("🎯 Extracted reserves: A={}, B={}", reserve_a, reserve_b)
            );

            // Calculate potential price (assuming one is SOL, other is token)
            let sol_decimals = 9;
            let token_decimals = 6; // Common for pump.fun tokens

            let price_a_to_b =
                (reserve_b as f64) /
                (10_f64).powi(sol_decimals) /
                ((reserve_a as f64) / (10_f64).powi(token_decimals));
            let price_b_to_a =
                (reserve_a as f64) /
                (10_f64).powi(sol_decimals) /
                ((reserve_b as f64) / (10_f64).powi(token_decimals));

            log(
                LogTag::Pool,
                "PRICE_CALC",
                &format!(
                    "💰 Potential prices: A→B: {:.12} SOL, B→A: {:.12} SOL",
                    price_a_to_b,
                    price_b_to_a
                )
            );
        } else {
            log(LogTag::Pool, "NO_RESERVES", "❌ Could not extract reserves from pool data");
        }
    } else {
        log(
            LogTag::Pool,
            "RAYDIUM_ERROR",
            &format!("❌ Unexpected pool data size: {} bytes", data.len())
        );
    }
}

/// Test Raydium CPMM pool directly
async fn test_raydium_cpmm_pool_direct(pool_address: &str, data: &[u8]) {
    log(LogTag::Pool, "RAYDIUM_CPMM", &format!("🔄 Testing Raydium CPMM pool: {}", pool_address));

    // Raydium CPMM structure - we have working decoder for this
    log(LogTag::Pool, "CPMM_DATA", &format!("Pool data size: {} bytes", data.len()));

    // Use our existing CPMM decoder (constructor now infallible)
    let mut calculator = screenerbot::tokens::pool::PoolPriceCalculator::new();
    calculator.enable_debug();
    log(LogTag::Pool, "CPMM_DECODE", "Using existing Raydium CPMM decoder...");
}

/// Test Meteora DLMM pool directly
async fn test_meteora_dlmm_pool_direct(pool_address: &str, data: &[u8]) {
    log(LogTag::Pool, "METEORA_DLMM", &format!("⚡ Testing Meteora DLMM pool: {}", pool_address));

    log(LogTag::Pool, "DLMM_DATA", &format!("Pool data size: {} bytes", data.len()));

    // Meteora DLMM has complex structure - this would need detailed analysis
    if data.len() > 100 {
        log(LogTag::Pool, "DLMM_ANALYSIS", "Meteora DLMM pools have complex bin-based structure");
        log(
            LogTag::Pool,
            "DLMM_NOTE",
            "This pool type requires specialized bin analysis - using existing decoder"
        );
    }
}

/// Test Meteora DAMM v2 pool directly
async fn test_meteora_damm_pool_direct(pool_address: &str, data: &[u8]) {
    log(
        LogTag::Pool,
        "METEORA_DAMM",
        &format!("⚡ Testing Meteora DAMM v2 pool: {}", pool_address)
    );

    log(LogTag::Pool, "DAMM_DATA", &format!("Pool data size: {} bytes", data.len()));

    // DAMM v2 should have reserve information
    if data.len() >= 200 {
        log(LogTag::Pool, "DAMM_ANALYSIS", "Analyzing DAMM v2 structure - using existing decoder");
    }
}

/// Search for known pubkeys in pool data without making RPC calls
fn search_for_known_pubkeys_in_data(data: &[u8]) {
    log(LogTag::Pool, "SEARCH_PUBKEYS", "🔍 Searching for known pubkeys in pool data...");

    // Known important pubkeys to look for
    let known_pubkeys = vec![
        ("So11111111111111111111111111111111111111112", "SOL mint"),
        ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "SPL Token Program"),
        ("11111111111111111111111111111111", "System Program")
    ];

    for (pubkey_str, name) in known_pubkeys {
        if let Ok(pubkey) = solana_sdk::pubkey::Pubkey::from_str(pubkey_str) {
            let pubkey_bytes = pubkey.to_bytes();

            // Search for this pubkey in the data
            for i in 0..data.len().saturating_sub(32) {
                if &data[i..i + 32] == pubkey_bytes {
                    log(
                        LogTag::Pool,
                        "FOUND_PUBKEY",
                        &format!("✅ Found {} at offset {}: {}", name, i, pubkey_str)
                    );
                }
            }
        }
    }

    // Also look for the specific token mint in our test
    let test_token = "9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump";
    if let Ok(pubkey) = solana_sdk::pubkey::Pubkey::from_str(test_token) {
        let pubkey_bytes = pubkey.to_bytes();

        for i in 0..data.len().saturating_sub(32) {
            if &data[i..i + 32] == pubkey_bytes {
                log(
                    LogTag::Pool,
                    "FOUND_TOKEN",
                    &format!("✅ Found test token mint at offset {}: {}", i, test_token)
                );
            }
        }
    }
}

/// Extract reserves from Raydium Legacy pool data based on analysis
fn extract_reserves_from_raydium_legacy(data: &[u8]) -> Option<(u64, u64)> {
    // Based on our hex analysis, let's try the most promising offsets
    // From the analysis, we saw potential reserves at offsets 208 and 216

    if data.len() < 224 {
        return None;
    }

    // Try offset 208 and 216 (these showed promising values)
    if
        let (Ok(reserve_a_bytes), Ok(reserve_b_bytes)) = (
            data[208..216].try_into(),
            data[216..224].try_into(),
        )
    {
        let reserve_a = u64::from_le_bytes(reserve_a_bytes);
        let reserve_b = u64::from_le_bytes(reserve_b_bytes);

        // Validate that these look like reasonable reserves
        if
            reserve_a > 1_000 &&
            reserve_b > 1_000 &&
            reserve_a < 1_000_000_000_000_000 &&
            reserve_b < 1_000_000_000_000_000
        {
            return Some((reserve_a, reserve_b));
        }
    }

    // If that doesn't work, try other promising offsets from our analysis
    let promising_offsets = vec![
        (256, 264), // Alternative offsets
        (288, 296),
        (312, 320)
    ];

    for (offset_a, offset_b) in promising_offsets {
        if data.len() >= offset_b + 8 {
            if
                let (Ok(reserve_a_bytes), Ok(reserve_b_bytes)) = (
                    data[offset_a..offset_a + 8].try_into(),
                    data[offset_b..offset_b + 8].try_into(),
                )
            {
                let reserve_a = u64::from_le_bytes(reserve_a_bytes);
                let reserve_b = u64::from_le_bytes(reserve_b_bytes);

                if
                    reserve_a > 1_000 &&
                    reserve_b > 1_000 &&
                    reserve_a < 1_000_000_000_000_000 &&
                    reserve_b < 1_000_000_000_000_000
                {
                    return Some((reserve_a, reserve_b));
                }
            }
        }
    }

    None
}

/// Test monitoring service
async fn test_monitoring_service(
    pool_service: &'static screenerbot::tokens::pool::PoolPriceService,
    matches: &clap::ArgMatches
) {
    log(LogTag::Pool, "TEST", "Starting monitoring service test...");

    let duration: u64 = matches.get_one::<String>("duration").unwrap().parse().unwrap_or(30);

    // Start monitoring
    pool_service.start_monitoring().await;

    log(
        LogTag::Pool,
        "INFO",
        &format!("Monitoring for {} seconds (priority tokens sourced from price service)...", duration)
    );

    // Monitor for specified duration
    let mut elapsed = 0u64;
    while elapsed < duration {
        sleep(Duration::from_secs(5)).await;
        elapsed += 5;

        let (pool_cache_size, price_cache_size, availability_cache_size) =
            pool_service.get_cache_stats().await;

        log(
            LogTag::Pool,
            "STATUS",
            &format!(
                "Elapsed: {}s, Caches: pool={}, price={}, availability={}",
                elapsed,
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
                        pair.dex_id, // Keep API dex_id for debugging tool output
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
                        best_pool.dex_id // Keep API dex_id for debugging tool output
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

    // Get API price (this returns SOL price) - use the blocking version to ensure we get the price
    let api_price_sol = screenerbot::tokens
        ::get_price(token_address, Some(screenerbot::tokens::PriceOptions::api_only()), false).await
        .and_then(|r| r.best_sol_price());
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

/// Test pool directly via blockchain decoder (bypasses API discovery)
async fn test_pool_direct(pool_address: &str, token_address: &str) {
    log(
        LogTag::Pool,
        "DIRECT_TEST",
        &format!("Testing direct pool calculation: {} for token: {}", pool_address, token_address)
    );

    // First, dump the raw pool data for analysis
    dump_pool_hex_data(pool_address).await;

    let pool_service = get_pool_service();

    // Test direct pool price calculation
    match pool_service.get_pool_price_direct(pool_address, token_address, None).await {
        Some(pool_result) => {
            log(
                LogTag::Pool,
                "DIRECT_SUCCESS",
                &format!(
                    "✅ Direct pool calculation successful:\n\
                    - Pool Address: {}\n\
                    - Pool Type: {}\n\
                    - Price SOL: {:.12}\n\
                    - Source: {}\n\
                    - Calculated At: {}",
                    pool_result.pool_address,
                    pool_result.pool_type.as_deref().unwrap_or("Unknown"),
                    pool_result.price_sol.unwrap_or(0.0),
                    pool_result.source,
                    pool_result.calculated_at.format("%Y-%m-%d %H:%M:%S%.3f UTC")
                )
            );

            // Compare with API-based calculation if available
            log(LogTag::Pool, "COMPARISON", "Comparing with API-based calculation...");
            match pool_service.get_pool_price(token_address, None).await {
                Some(api_result) => {
                    if
                        let (Some(direct_price), Some(api_price)) = (
                            pool_result.price_sol,
                            api_result.price_sol,
                        )
                    {
                        let price_diff = direct_price - api_price;
                        let price_diff_percent = if api_price != 0.0 {
                            ((direct_price - api_price) / api_price) * 100.0
                        } else {
                            0.0
                        };

                        log(
                            LogTag::Pool,
                            "PRICE_DIFF",
                            &format!(
                                "📊 Price Comparison:\n\
                                - Direct: {:.12} SOL\n\
                                - API-based: {:.12} SOL\n\
                                - Difference: {:.12} SOL ({:.4}%)",
                                direct_price,
                                api_price,
                                price_diff,
                                price_diff_percent
                            )
                        );
                    }
                }
                None => {
                    log(LogTag::Pool, "COMPARISON", "❌ API-based calculation failed");
                }
            }
        }
        None => {
            log(LogTag::Pool, "DIRECT_ERROR", "❌ Direct pool calculation failed");
        }
    }
}

/// Test token availability and price calculation
async fn test_token_availability_and_price(
    pool_service: &'static screenerbot::tokens::pool::PoolPriceService,
    token_address: &str
) {
    log(LogTag::Pool, "TEST", &format!("Testing availability and price for: {}", token_address));

    // Enable debug mode if pool prices debug flag is set
    use screenerbot::global::is_debug_pool_prices_enabled;
    if is_debug_pool_prices_enabled() {
        log(
            LogTag::Pool,
            "DEBUG",
            "Pool price debugging enabled - will test with direct calculator"
        );
        let mut calculator = screenerbot::tokens::pool::PoolPriceCalculator::new();
        calculator.enable_debug();
        let has_pools = pool_service.check_token_availability(token_address).await;
        log(LogTag::Pool, "AVAILABILITY", &format!("Has pools: {}", has_pools));
        if has_pools {
            if
                let Ok(pairs) =
                    screenerbot::tokens::dexscreener::get_token_pairs_from_api(token_address).await
            {
                for pair in pairs {
                    if pair.dex_id == "pumpswap" {
                        log(
                            LogTag::Pool,
                            "PUMP_TEST",
                            &format!(
                                "Testing pump.fun pool: {} ({})",
                                pair.pair_address,
                                pair.dex_id
                            )
                        );
                        match
                            calculator.calculate_token_price(
                                &pair.pair_address,
                                token_address
                            ).await
                        {
                            Ok(Some(price_info)) => {
                                log(
                                    LogTag::Pool,
                                    "PUMP_SUCCESS",
                                    &format!(
                                        "Pump.fun price calculated: {:.12} SOL",
                                        price_info.price_sol
                                    )
                                );
                            }
                            Ok(None) => {
                                log(
                                    LogTag::Pool,
                                    "PUMP_NONE",
                                    "Pump.fun calculation returned None"
                                );
                            }
                            Err(e) => {
                                log(
                                    LogTag::Pool,
                                    "PUMP_ERROR",
                                    &format!("Pump.fun calculation failed: {}", e)
                                );
                            }
                        }
                        break;
                    }
                }
            }
        }
    } else {
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
                            "Pool price calculated: {:.12} SOL from {} ({})",
                            pool_result.price_sol.unwrap_or(0.0),
                            pool_result.pool_address,
                            pool_result.pool_type.as_ref().unwrap_or(&"Unknown Pool".to_string())
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
                        pool.dex_id, // Keep API dex_id for debugging tool output
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

    println!("\n🔍 LIST ALL POOLS:");
    println!("   cargo run --bin tool_pool_price -- --token <TOKEN_MINT> --list-pools");

    println!("\n🎯 TEST SPECIFIC POOL DIRECTLY:");
    println!("   cargo run --bin tool_pool_price -- --pool <POOL_ADDRESS> --test-pool-direct");

    println!("\n🧪 TEST RAYDIUM LEGACY AMM:");
    println!(
        "   cargo run --bin tool_pool_price -- --pool Bzc9NZfMqkXR6fz1DBph7BDf9BroyEf6pnzESP7v5iiw --test-pool-direct --debug"
    );

    println!("\n🚀 TEST PUMP.FUN POOL:");
    println!(
        "   cargo run --bin tool_pool_price -- --pool 35TqQMeiRwEbK6FR5qiPwastuAAvo32VjnULJpxVSxUK --test-pool-direct --debug"
    );

    println!("\n1. Test specific token pools:");
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

    println!("\n8. Test direct pool calculation (bypasses API discovery):");
    println!(
        "   cargo run --bin tool_pool_price -- --pool <POOL_ADDRESS> --token <TOKEN_MINT> --test-direct"
    );

    println!("\n9. Test Meteora DLMM pool (example with provided data):");
    println!(
        "   cargo run --bin tool_pool_price -- --pool 3PB3eohmv5g7rUtnr1k7vJkM1RLu1X8h4knryfxB1DuZ --token 9tqjeRS1swj36Ee5C1iGiwAxjQJNGAVCzaTLwFY8bonk --test-direct --debug"
    );

    println!("\n10. Full comparison test (API vs Direct pool calculation):");
    println!(
        "   cargo run --bin tool_pool_price -- --pool <POOL_ADDRESS> --token <TOKEN_MINT> --test-direct --debug-detailed"
    );
}

/// Dump raw pool account data as hex for analysis
async fn dump_pool_hex_data(pool_address: &str) {
    log(
        LogTag::Pool,
        "HEX_DUMP",
        &format!("🔍 Dumping raw pool data for analysis: {}", pool_address)
    );

    // Create calculator to get raw pool data
    {
        let mut calculator = screenerbot::tokens::pool::PoolPriceCalculator::new();
        calculator.enable_debug();

        match calculator.get_raw_pool_data(pool_address).await {
            Ok(Some(data)) => {
                log(
                    LogTag::Pool,
                    "HEX_DATA",
                    &format!("📊 Pool data length: {} bytes", data.len())
                );

                // Dump hex data in 16-byte rows with offset and ASCII
                for (i, chunk) in data.chunks(16).enumerate() {
                    let offset = i * 16;
                    let hex_part: String = chunk
                        .iter()
                        .map(|byte| format!("{:02x}", byte))
                        .collect::<Vec<String>>()
                        .join(" ");

                    let ascii_part: String = chunk
                        .iter()
                        .map(|&byte| {
                            if byte >= 32 && byte <= 126 { byte as char } else { '.' }
                        })
                        .collect();

                    log(
                        LogTag::Pool,
                        "HEX_ROW",
                        &format!("{:04x}: {:<48} |{}|", offset, hex_part, ascii_part)
                    );
                }

                // Look for the specific pubkeys in the data
                search_pubkeys_in_data(&data, pool_address).await;
            }
            Ok(None) => {
                log(LogTag::Pool, "HEX_ERROR", "❌ Pool account not found");
            }
            Err(e) => {
                log(LogTag::Pool, "HEX_ERROR", &format!("❌ Failed to fetch pool data: {}", e));
            }
        }
    }
}

/// Search for known pubkeys in the pool data to find their offsets
async fn search_pubkeys_in_data(data: &[u8], pool_address: &str) {
    // Known pubkeys from the provided structure
    let expected_pubkeys = vec![
        ("token_x_mint", "9tqjeRS1swj36Ee5C1iGiwAxjQJNGAVCzaTLwFY8bonk"),
        ("token_y_mint", "So11111111111111111111111111111111111111112"),
        ("reserve_x", "DTxrnmwcN9FDRhgwMANoaKxjeBLuffq8cQ1PuJ7BGUDW"),
        ("reserve_y", "FttuDshzg3NLyEadJg1LThsH9UFTp1ebUbfugfmW9yBu"),
        ("oracle", "7k15A8Qy2wgZRRJwgpQKaUtdCJLjfz7ByGcUH8BcwDXq"),
        ("base_key", "2RA1EnEVxWP8TQZhFt2nXuVcrQetFQUgYyGsUBTWUNpR"),
        ("creator", "3edfkoVJeU4AzWGjXRyNrsgsyns5FepTK4Je8QU9qbwi")
    ];

    log(
        LogTag::Pool,
        "SEARCH_PUBKEYS",
        &format!("🔍 Searching for known pubkeys in pool {} data...", pool_address)
    );

    for (name, pubkey_str) in expected_pubkeys {
        // Convert pubkey string to bytes
        if let Ok(pubkey) = solana_sdk::pubkey::Pubkey::from_str(pubkey_str) {
            let pubkey_bytes = pubkey.to_bytes();

            // Search for this pubkey in the data
            for i in 0..data.len().saturating_sub(31) {
                if data[i..i + 32] == pubkey_bytes {
                    log(
                        LogTag::Pool,
                        "FOUND_PUBKEY",
                        &format!("✅ Found {} at offset {}: {}", name, i, pubkey_str)
                    );
                    break;
                }
            }
        }
    }
}
