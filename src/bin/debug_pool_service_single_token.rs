/// Debug tool for monitoring a single token with the full pool service
///
/// This tool starts the complete pool service but configures it to monitor
/// only one specific token, bypassing database discovery.

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::pools::{ start_pool_service, stop_pool_service, set_debug_token_override, get_pool_price };
use screenerbot::pools::types::ProgramKind;
use screenerbot::tokens::dexscreener::{ get_token_price_from_global_api, init_dexscreener_api };
use screenerbot::rpc::get_rpc_client;
use screenerbot::logger::{ log, LogTag };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ interval, Duration };

#[derive(Parser, Debug)]
#[command(name = "debug_pool_service_single_token", about = "Monitor a single token with pool service")]
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
        Err(_) => return ("INVALID_ADDRESS".to_string(), "unknown".to_string()),
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Set up command line arguments
    let cmd_args = vec!["debug_pool_service_single_token".to_string()];
    set_cmd_args(cmd_args);
    
    log(LogTag::PoolService, "START", &format!("Starting pool service for single token: {}", args.token));
    
    // Initialize DexScreener API
    log(LogTag::PoolService, "INIT", "Initializing DexScreener API...");
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::PoolService, "ERROR", &format!("Failed to initialize DexScreener API: {}", e));
        return Err(e.into());
    }
    
    // Set debug override to monitor only our target token
    set_debug_token_override(Some(vec![args.token.clone()]));
    
    // Start the pool service
    start_pool_service().await?;
    
    log(LogTag::PoolService, "SUCCESS", "Pool service started");
    log(LogTag::PoolService, "INFO", &format!("Monitoring token: {}", args.token));
    log(LogTag::PoolService, "INFO", &format!("Will run for {} seconds, checking every {} seconds", args.duration, args.interval));
    log(LogTag::PoolService, "INFO", "Starting price monitoring...");
    
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
    
    // Price monitoring loop
    let mut price_interval = interval(Duration::from_secs(args.interval));
    let mut price_count = 0;
    
    loop {
        tokio::select! {
            _ = price_interval.tick() => {
                // Get calculated price from pool service
                let pool_price = get_pool_price(&args.token);
                
                // Get DexScreener API price for comparison
                let api_price = get_token_price_from_global_api(&args.token).await;
                
                match pool_price {
                    Some(price) => {
                        price_count += 1;
                        
                        let price_comparison = match api_price {
                            Some(api_val) => format!("| API: {:.12} SOL", api_val),
                            None => "| API: unavailable".to_string(),
                        };
                        
                        // Get program information for this pool
                        let (program_id, program_name) = get_pool_program_info(&price.pool_address).await;
                        let program_display = if program_id.len() > 8 && !program_id.starts_with("INVALID") && !program_id.starts_with("FETCH") {
                            format!("{} ({}...{})", program_name, &program_id[..8], &program_id[program_id.len()-8..])
                        } else {
                            program_name
                        };
                        
                        log(
                            LogTag::PoolService, 
                            "PRICE", 
                            &format!("[{}] Pool: {:.12} SOL {} | Confidence: {:.2}", 
                                price_count, price.price_sol, price_comparison, price.confidence)
                        );
                        log(
                            LogTag::PoolService, 
                            "POOL_INFO", 
                            &format!("    Pool: {} | Program: {}", price.pool_address, program_display)
                        );
                        log(
                            LogTag::PoolService, 
                            "RESERVES", 
                            &format!("    Reserves: {:.6} SOL / {:.6} tokens", price.sol_reserves, price.token_reserves)
                        );
                    }
                    None => {
                        price_count += 1;
                        let api_only = match api_price {
                            Some(api_val) => format!("API only: {:.12} SOL", api_val),
                            None => "No price available".to_string(),
                        };
                        log(LogTag::PoolService, "NO_PRICE", &format!("[{}] Pool: unavailable | {}", price_count, api_only));
                    }
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
    log(LogTag::PoolService, "STATS", &format!("Total price updates received: {}", price_count));
    
    Ok(())
}
