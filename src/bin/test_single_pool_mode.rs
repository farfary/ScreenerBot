//! Test tool to verify single pool mode functionality

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::pools::service::is_single_pool_mode_enabled;
use screenerbot::pools::discovery::PoolDiscovery;
use screenerbot::logger::{log, LogTag};

#[derive(Parser, Debug)]
#[command(name = "test_single_pool_mode", about = "Test single pool mode filtering")]
struct Args {
    /// Token address to test
    #[arg(short, long)]
    token: String,
}

/// Test pool filtering logic similar to the main service
fn filter_pools_for_single_mode(mut pools: Vec<screenerbot::pools::types::PoolDescriptor>) -> Vec<screenerbot::pools::types::PoolDescriptor> {
    if is_single_pool_mode_enabled() && !pools.is_empty() {
        // Sort pools by liquidity (highest first) and take only the first one
        pools.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));
        let highest_liquidity_pool = pools.into_iter().next().unwrap();
        vec![highest_liquidity_pool]
    } else {
        pools
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    set_cmd_args(args.clone());

    log(LogTag::PoolService, "INFO", "Testing single pool mode functionality");
    
    // Show current configuration
    if is_single_pool_mode_enabled() {
        log(LogTag::PoolService, "INFO", "Single pool mode: ENABLED");
    } else {
        log(LogTag::PoolService, "INFO", "Single pool mode: DISABLED");
    }

    // Test pool discovery and filtering
    let discovery = PoolDiscovery::new();
    let pools = discovery.discover_pools_for_token(&args.token).await;
    
    log(
        LogTag::PoolService,
        "INFO",
        &format!("Discovered {} pools for token {}", pools.len(), &args.token[..8])
    );

    // Apply filtering
    let filtered_pools = filter_pools_for_single_mode(pools);
    
    log(
        LogTag::PoolService,
        "INFO",
        &format!("After filtering: {} pools", filtered_pools.len())
    );

    // Show details of filtered pools
    for (i, pool) in filtered_pools.iter().enumerate() {
        log(
            LogTag::PoolService,
            "INFO",
            &format!(
                "Pool {}: {} | {} | ${:.2} liquidity",
                i + 1,
                &pool.pool_id.to_string()[..8],
                pool.program_kind.display_name(),
                pool.liquidity_usd
            )
        );
    }

    log(LogTag::PoolService, "SUCCESS", "Test completed");
}
