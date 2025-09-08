/// Debug tool for monitoring price changes in the biggest pool of a single token
///
/// This tool starts the complete pool service but configures it to monitor
/// only one specific token. It identifies the biggest pool by liquidity and
/// only logs when price changes occur, providing efficient change-based monitoring.

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::pools::{
    start_pool_service,
    stop_pool_service,
    set_debug_token_override,
    get_pool_price,
};
use screenerbot::pools::types::ProgramKind;
use screenerbot::pools::discovery::PoolDiscovery;
use screenerbot::pools::utils::is_stablecoin_mint;
use screenerbot::tokens::dexscreener::{ init_dexscreener_api, get_global_dexscreener_api };
use screenerbot::rpc::get_rpc_client;
use screenerbot::logger::{ log, LogTag };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ interval, Duration };

#[derive(Parser, Debug)]
#[command(
    name = "debug_pool_service_single_token",
    about = "Monitor a single token with pool service"
)]
struct Args {
    /// Token mint address to monitor
    #[arg(short, long)]
    token: String,

    /// Monitoring interval in seconds
    #[arg(short, long, default_value = "5")]
    interval: u64,

    /// Duration to run in seconds
    #[arg(short, long, default_value = "60")]
    duration: u64,
}

/// Get program type from pool address by fetching the account owner
async fn get_pool_program_info(pool_address: &str) -> (String, String) {
    let pool_pubkey = match Pubkey::from_str(pool_address) {
        Ok(pubkey) => pubkey,
        Err(_) => {
            return ("INVALID_ADDRESS".to_string(), "unknown".to_string());
        }
    };

    let rpc_client = get_rpc_client();
    match rpc_client.get_account(&pool_pubkey).await {
        Ok(account) => {
            let program_id = account.owner.to_string();
            let program_kind = ProgramKind::from_program_id(&program_id);
            (program_id, program_kind.display_name().to_string())
        }
        Err(_) => ("FETCH_ERROR".to_string(), "unknown".to_string()),
    }
}

/// Discover pools using proper discovery module and identify the biggest SOL pool by liquidity
async fn discover_and_identify_biggest_pool(
    token_address: &str
) -> Result<Option<(String, f64, String)>, String> {
    log(
        LogTag::PoolService,
        "DISCOVER_START",
        &format!("Discovering pools for token: {}", token_address)
    );

    // Use the proper discovery module which already filters for SOL-only pools
    let discovery = PoolDiscovery::new();
    let pool_descriptors = discovery.discover_pools_for_token(token_address).await;

    if pool_descriptors.is_empty() {
        log(LogTag::PoolService, "DISCOVER_ERROR", "No SOL-based pools found for token");
        return Ok(None);
    }

    log(
        LogTag::PoolService,
        "DISCOVER_SUCCESS",
        &format!("Found {} SOL-based pools for token {}", pool_descriptors.len(), token_address)
    );

    // Find the biggest pool by liquidity from the already-filtered SOL pools
    let mut biggest_pool = None;
    let mut highest_liquidity = -1.0;

    for descriptor in &pool_descriptors {
        let liquidity = descriptor.liquidity_usd;

        if liquidity > highest_liquidity {
            highest_liquidity = liquidity;
            biggest_pool = Some(descriptor);
        }
    }

    if let Some(pool) = biggest_pool {
        // Get program info for the biggest pool
        let (program_id, program_name) = get_pool_program_info(&pool.pool_id.to_string()).await;
        let program_display = format!("{} ({})", program_name, &program_id[..8]);

        log(
            LogTag::PoolService,
            "BIGGEST_POOL",
            &format!(
                "Selected biggest SOL pool: {} | {} | Liquidity: ${:.2}",
                &pool.pool_id.to_string()[..8],
                program_display,
                highest_liquidity
            )
        );

        // Log summary of all pools for reference
        log(
            LogTag::PoolService,
            "POOLS_SUMMARY",
            &format!(
                "All SOL pools: {} total, focusing on highest liquidity pool",
                pool_descriptors.len()
            )
        );

        Ok(Some((pool.pool_id.to_string(), highest_liquidity, program_display)))
    } else {
        log(LogTag::PoolService, "DISCOVER_ERROR", "No valid SOL pools found");
        Ok(None)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set up command line arguments with debug flags enabled
    let cmd_args = vec![
        "debug_pool_service_single_token".to_string(),
        "--debug-pool-calculator".to_string(),
        "--debug-pool-discovery".to_string(),
        "--debug-pool-service".to_string(),
        "--debug-pool-decoders".to_string()
    ];
    set_cmd_args(cmd_args);

    log(
        LogTag::PoolService,
        "START",
        &format!("Starting pool service for single token: {}", args.token)
    );

    // Initialize DexScreener API
    log(LogTag::PoolService, "INIT", "Initializing DexScreener API...");
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::PoolService, "ERROR", &format!("Failed to initialize DexScreener API: {}", e));
        return Err(e.into());
    }

    // Pre-fetch token decimals to ensure they're cached
    log(LogTag::PoolService, "INIT", "Pre-fetching token decimals...");

    // Early stablecoin validation - reject stablecoin tokens immediately
    if is_stablecoin_mint(&args.token) {
        log(
            LogTag::PoolService,
            "ERROR",
            &format!(
                "Token {} is a stablecoin (USDC/USDT) - not supported for price monitoring",
                &args.token[..8]
            )
        );
        return Err("Stablecoin tokens are not supported for price monitoring".into());
    }

    match screenerbot::tokens::decimals::get_token_decimals_from_chain(&args.token).await {
        Ok(decimals) => {
            log(
                LogTag::PoolService,
                "SUCCESS",
                &format!("Token decimals fetched: {} decimals", decimals)
            );
        }
        Err(e) => {
            log(LogTag::PoolService, "WARN", &format!("Failed to fetch token decimals: {}", e));
        }
    }

    // Set debug override to monitor only our target token
    set_debug_token_override(Some(vec![args.token.clone()]));

    // Start the pool service
    start_pool_service().await?;

    log(LogTag::PoolService, "SUCCESS", "Pool service started");
    log(LogTag::PoolService, "INFO", &format!("Monitoring token: {}", args.token));
    log(
        LogTag::PoolService,
        "INFO",
        &format!("Will run for {} seconds, checking every {} seconds", args.duration, args.interval)
    );

    // Discover pools and identify the biggest one
    let biggest_pool_info = match discover_and_identify_biggest_pool(&args.token).await {
        Ok(Some(info)) => {
            log(LogTag::PoolService, "SUCCESS", "Biggest pool identified successfully");
            Some(info)
        }
        Ok(None) => {
            log(LogTag::PoolService, "WARN", "No pools found, will monitor anyway");
            None
        }
        Err(e) => {
            log(LogTag::PoolService, "DISCOVER_FAILED", &format!("Pool discovery failed: {}", e));
            None
        }
    };

    log(LogTag::PoolService, "INFO", "Starting price change monitoring (biggest pool only)...");

    // Create shutdown notification for clean exit
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Set up signal handling for graceful shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        log(LogTag::PoolService, "SHUTDOWN", "Received Ctrl+C, shutting down...");
        shutdown_clone.notify_one();
    });

    // Set up timer for automatic shutdown
    let shutdown_timer = shutdown.clone();
    let run_duration = args.duration;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(run_duration)).await;
        log(LogTag::PoolService, "SHUTDOWN", "Time limit reached, shutting down...");
        shutdown_timer.notify_one();
    });

    // Price monitoring loop with change detection
    let mut price_interval = interval(Duration::from_secs(args.interval));
    let mut last_price: Option<f64> = None;
    let mut last_api_price: Option<f64> = None;
    let mut check_count = 0;

    loop {
        tokio::select! {
            _ = price_interval.tick() => {
                check_count += 1;
                
                // Get calculated price from pool service
                let pool_price = get_pool_price(&args.token);
                
                // Get DexScreener API price for comparison
                let api_price = {
                    if let Ok(api) = get_global_dexscreener_api().await {
                        if let Ok(mut guard) = tokio::time::timeout(std::time::Duration::from_millis(8000), api.lock()).await {
                            guard.get_price(&args.token).await
                        } else { None }
                    } else { None }
                };
                
                // Check if we have a price from the pool service
                if let Some(price) = &pool_price {
                    // Check if this price is from the biggest pool (if we identified one)
                    let is_biggest_pool = if let Some((biggest_pool_address, _, _)) = &biggest_pool_info {
                        price.pool_address == *biggest_pool_address
                    } else {
                        true // If no biggest pool identified, accept any pool
                    };
                    
                    if is_biggest_pool {
                        // Check for price changes
                        let price_changed = match last_price {
                            Some(last) => (price.price_sol - last).abs() > 0.000000001, // Very small threshold for SOL prices
                            None => true, // First price is always a "change"
                        };
                        
                        let api_price_changed = match (&api_price, &last_api_price) {
                            (Some(current), Some(last)) => (current - last).abs() > 0.000000001,
                            (Some(_), None) => true,
                            _ => false,
                        };
                        
                        if price_changed || api_price_changed {
                            let price_comparison = match api_price {
                                Some(api_val) => format!("| API: {:.12} SOL", api_val),
                                None => "| API: unavailable".to_string(),
                            };
                            
                            // Show price change information
                            let change_info = if let Some(last) = last_price {
                                let change = price.price_sol - last;
                                let change_pct = (change / last) * 100.0;
                                format!(" (Change: {:+.9} SOL, {:+.4}%)", change, change_pct)
                            } else {
                                " (Initial price)".to_string()
                            };
                            
                            // Get program information for this pool
                            let program_display = if let Some((_, _, program_info)) = &biggest_pool_info {
                                program_info.clone()
                            } else {
                                let (program_id, program_name) = get_pool_program_info(&price.pool_address).await;
                                if program_id.len() > 8 && !program_id.starts_with("INVALID") && !program_id.starts_with("FETCH") {
                                    format!("{} ({}...{})", program_name, &program_id[..8], &program_id[program_id.len()-8..])
                                } else {
                                    program_name
                                }
                            };
                            
                            log(
                                LogTag::PoolService, 
                                "PRICE_CHANGE", 
                                &format!("[{}] {:.12} SOL{} {}{} | Confidence: {:.2}", 
                                    check_count, 
                                    price.price_sol, 
                                    change_info, 
                                    price_comparison,
                                    match api_price { 
                                        Some(api) if api > 0.0 => {
                                            let diff_pct = ((price.price_sol - api) / api) * 100.0;
                                            format!(" | Diff vs API: {:+.2}%", diff_pct)
                                        },
                                        _ => String::new()
                                    },
                                    price.confidence)
                            );
                            log(
                                LogTag::PoolService, 
                                "POOL_INFO", 
                                &format!("    Biggest Pool: {} | Program: {}", price.pool_address, program_display)
                            );
                            log(
                                LogTag::PoolService, 
                                "RESERVES", 
                                &format!("    Reserves: {:.6} SOL / {:.6} tokens", price.sol_reserves, price.token_reserves)
                            );
                            
                            // Update last prices
                            last_price = Some(price.price_sol);
                            last_api_price = api_price;
                        } else {
                            // No change, just log a brief status every 10 checks
                            if check_count % 10 == 0 {
                                log(
                                    LogTag::PoolService, 
                                    "PRICE_STABLE", 
                                    &format!("[{}] Price stable: {:.12} SOL (no changes)", check_count, price.price_sol)
                                );
                            }
                        }
                    } else {
                        // Price is not from the biggest pool, ignore it
                        if check_count % 10 == 0 {
                            log(
                                LogTag::PoolService, 
                                "WRONG_POOL", 
                                &format!("[{}] Price from different pool ({}), waiting for biggest pool", check_count, &price.pool_address[..8])
                            );
                        }
                    }
                } else {
                    // No pool price available
                    match api_price {
                        Some(api_val) => {
                            // Check if API price changed
                            let api_changed = match last_api_price {
                                Some(last) => (api_val - last).abs() > 0.000000001,
                                None => true,
                            };
                            
                            if api_changed {
                                let change_info = if let Some(last) = last_api_price {
                                    let change = api_val - last;
                                    let change_pct = (change / last) * 100.0;
                                    format!(" (Change: {:+.9} SOL, {:+.4}%)", change, change_pct)
                                } else {
                                    " (Initial API price)".to_string()
                                };
                                
                                log(
                                    LogTag::PoolService, 
                                    "API_PRICE_CHANGE", 
                                    &format!("[{}] Pool unavailable | API: {:.12} SOL{}", check_count, api_val, change_info)
                                );
                                last_api_price = Some(api_val);
                            } else if check_count % 10 == 0 {
                                log(
                                    LogTag::PoolService, 
                                    "NO_POOL_PRICE", 
                                    &format!("[{}] Pool price unavailable | API stable: {:.12} SOL", check_count, api_val)
                                );
                            }
                        }
                        None => {
                            if check_count % 10 == 0 {
                                log(
                                    LogTag::PoolService, 
                                    "NO_PRICE", 
                                    &format!("[{}] No price available from pool or API", check_count)
                                );
                            }
                        }
                    };
                }
            }
            _ = shutdown.notified() => {
                break;
            }
        }
    }

    log(LogTag::PoolService, "STOP", "Stopping pool service...");

    // Stop the pool service
    stop_pool_service(10).await?;

    // Clear debug override
    set_debug_token_override(None);

    log(LogTag::PoolService, "SUCCESS", "Pool service stopped");
    log(LogTag::PoolService, "STATS", &format!("Total price checks performed: {}", check_count));

    Ok(())
}
