/// Debug tool for monitoring a single token with the full pool service
///
/// This tool starts the complete pool service but configures it to monitor
/// only one specific token, bypassing database discovery.

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::pools::{ start_pool_service, stop_pool_service, set_debug_token_override, get_pool_price };
use screenerbot::tokens::dexscreener::{ get_token_price_from_global_api, init_dexscreener_api };
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ interval, Duration };

#[derive(Parser, Debug)]
#[command(name = "debug_pool_service_single_token", about = "Monitor a single token with pool service")]
struct Args {
    /// Token mint to monitor
    #[arg(long)]
    token: String,
    
    /// Enable debug logging for pool service
    #[arg(long, default_value_t = false)]
    debug_pool_service: bool,
    
    /// Enable debug logging for pool calculator
    #[arg(long, default_value_t = false)]
    debug_pool_calculator: bool,
    
    /// Run duration in seconds (default: 60)
    #[arg(long, default_value_t = 60)]
    duration: u64,
    
    /// Price check interval in seconds (default: 5)
    #[arg(long, default_value_t = 5)]
    interval: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Set up command line arguments for debug flags
    let mut cmd_args = vec!["debug_pool_service_single_token".to_string()];
    if args.debug_pool_service {
        cmd_args.push("--debug-pool-service".to_string());
    }
    if args.debug_pool_calculator {
        cmd_args.push("--debug-pool-calculator".to_string());
    }
    set_cmd_args(cmd_args);
    
    println!("🚀 Starting pool service for single token: {}", args.token);
    
    // Initialize DexScreener API
    println!("🔌 Initializing DexScreener API...");
    if let Err(e) = init_dexscreener_api().await {
        eprintln!("❌ Failed to initialize DexScreener API: {}", e);
        return Err(e.into());
    }
    
    // Set debug override to monitor only our target token
    set_debug_token_override(Some(vec![args.token.clone()]));
    
    // Start the pool service
    start_pool_service().await?;
    
    println!("✅ Pool service started");
    println!("🔍 Monitoring token: {}", args.token);
    println!("⏱️  Will run for {} seconds, checking every {} seconds", args.duration, args.interval);
    println!("📊 Price updates:");
    
    // Create shutdown notification for clean exit
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();
    
    // Set up signal handling for graceful shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("\n🛑 Received Ctrl+C, shutting down...");
        shutdown_clone.notify_one();
    });
    
    // Set up timer for automatic shutdown
    let shutdown_timer = shutdown.clone();
    let run_duration = args.duration;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(run_duration)).await;
        println!("\n⏰ Time limit reached, shutting down...");
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
                        
                        println!(
                            "[{}] 💰 Pool: {:.12} SOL {} | Confidence: {:.2} | Pool: {} | Reserves: {:.6} SOL / {:.6} tokens",
                            price_count,
                            price.price_sol,
                            price_comparison,
                            price.confidence,
                            price.pool_address,
                            price.sol_reserves,
                            price.token_reserves
                        );
                    }
                    None => {
                        price_count += 1;
                        let api_only = match api_price {
                            Some(api_val) => format!("API only: {:.12} SOL", api_val),
                            None => "No price available".to_string(),
                        };
                        println!("[{}] ❌ Pool: unavailable | {}", price_count, api_only);
                    }
                }
            }
            _ = shutdown.notified() => {
                break;
            }
        }
    }
    
    println!("\n🔄 Stopping pool service...");
    
    // Stop the pool service
    stop_pool_service(10).await?;
    
    // Clear debug override
    set_debug_token_override(None);
    
    println!("✅ Pool service stopped");
    println!("📈 Total price updates received: {}", price_count);
    
    Ok(())
}
