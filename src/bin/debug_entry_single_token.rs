/// Debug tool for testing entry::should_buy on a single token
///
/// - Starts the pool service focused on one token
/// - Periodically fetches PriceResult and calls entry::should_buy
/// - Prints decision, confidence, and reason; logs detailed guards when --debug-entry is enabled
/// - Safe: does NOT open positions or run the trader loop
///
/// Usage examples:
/// cargo run --bin debug_entry_single_token -- --token <MINT> --duration 60 --interval 5 --debug-entry --debug-pool-service

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{ log, LogTag };
use screenerbot::pools::{
    start_pool_service,
    stop_pool_service,
    set_debug_token_override,
    get_pool_price,
};
use screenerbot::tokens::decimals::get_token_decimals_from_chain;
use screenerbot::tokens::dexscreener::{ init_dexscreener_api };

#[derive(Parser, Debug)]
#[command(
    name = "debug_entry_single_token",
    about = "Test entry::should_buy for one token using live pool prices"
)]
struct Args {
    /// Token mint address to test
    #[arg(short, long)]
    token: String,

    /// Seconds between checks
    #[arg(short, long, default_value = "5")]
    interval: u64,

    /// Total run duration in seconds
    #[arg(short, long, default_value = "60")]
    duration: u64,

    /// Enable entry module debug logging
    #[arg(long)]
    debug_entry: bool,

    /// Enable pool service debug logging
    #[arg(long)]
    debug_pool_service: bool,

    /// Enable pool calculator debug logging
    #[arg(long)]
    debug_pool_calculator: bool,

    /// Enable pool fetcher debug logging
    #[arg(long)]
    debug_pool_fetcher: bool,

    /// Enable pool decoders debug logging
    #[arg(long)]
    debug_pool_decoders: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Wire up global args for logging gates
    let mut cmd_args = vec!["debug_entry_single_token".to_string()];
    if args.debug_entry {
        cmd_args.push("--debug-entry".to_string());
    }
    if args.debug_pool_service {
        cmd_args.push("--debug-pool-service".to_string());
    }
    if args.debug_pool_calculator {
        cmd_args.push("--debug-pool-calculator".to_string());
    }
    if args.debug_pool_fetcher {
        cmd_args.push("--debug-pool-fetcher".to_string());
    }
    if args.debug_pool_decoders {
        cmd_args.push("--debug-pool-decoders".to_string());
    }
    set_cmd_args(cmd_args.clone());

    log(LogTag::Entry, "START", &format!("Starting entry test for token {}", args.token));

    // Initialize external APIs used by discovery/fetchers
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::Entry, "ERROR", &format!("DexScreener init failed: {}", e));
    }

    // Ensure decimals are cached to allow any token math elsewhere if needed
    match get_token_decimals_from_chain(&args.token).await {
        Ok(dec) => log(LogTag::Entry, "INIT", &format!("Token decimals: {}", dec)),
        Err(e) => log(LogTag::Entry, "WARN", &format!("Failed to fetch decimals: {}", e)),
    }

    // Focus everything on our single token
    set_debug_token_override(Some(vec![args.token.clone()]));

    // Start pool service (single-pool mode for debug override)
    start_pool_service().await?;
    log(LogTag::Entry, "READY", &format!("Pool service started, running for {}s", args.duration));

    // Periodic checks
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(args.interval));
    let stop_at = std::time::Instant::now() + std::time::Duration::from_secs(args.duration);

    while std::time::Instant::now() < stop_at {
        tick.tick().await;

        if let Some(price) = get_pool_price(&args.token) {
            let (approved, confidence, reason) = screenerbot::entry::should_buy(&price).await;

            // Log a concise decision line (full details gated behind --debug-entry within entry.rs)
            log(
                LogTag::Entry,
                "ENTRY_TEST",
                &format!(
                    "mint={} price={:.12} SOL should_buy={} conf={:.1}% reason={}",
                    price.mint,
                    price.price_sol,
                    approved,
                    confidence,
                    reason
                )
            );
        } else {
            log(LogTag::Entry, "WAIT", "Price not yet available from pool service");
        }
    }

    // Gracefully stop the pool service with a small timeout window
    stop_pool_service(5).await?;
    log(LogTag::Entry, "DONE", "Stopped pool service and exiting");
    Ok(())
}
