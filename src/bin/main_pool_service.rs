use clap::Parser;
use screenerbot::logger::{log, LogTag};
use screenerbot::pool_service;
use screenerbot::pool_discovery;

/// Pool Service Tool - Initialize and manage pool services
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Show help information
    #[arg(long, short)]
    help: bool,

    /// Token address to initialize from
    #[arg(long)]
    from_token_address: Option<String>,

    /// Pool address to initialize from
    #[arg(long)]
    from_pool_address: Option<String>,
}

fn main() {
    let args = Args::parse();

    // Show help if requested
    if args.help {
        println!("Pool Service Tool");
        println!();
        println!("This tool initializes the pool service and related components.");
        println!();
        println!("Usage:");
        println!("  main_pool_service [OPTIONS]");
        println!();
        println!("Options:");
        println!("  -h, --help                    Show this help message");
        println!("      --from-token-address      Token address to initialize from");
        println!("      --from-pool-address       Pool address to initialize from");
        println!();
        println!("The tool will:");
        println!("  1. Initialize the file logging system");
        println!("  2. Initialize the pool service");
        println!("  3. Log initialization progress");
        println!("  4. Process token/pool addresses if provided");
        return;
    }

    // Initialize logger
    screenerbot::logger::init_file_logging();

    log(LogTag::System, "INIT", "🚀 Starting Pool Service Tool");

    // Initialize pool service
    pool_service::init_pool_service();
    // Initialize pool DB service for persistence used by discovery
    if let Err(e) = screenerbot::pool_db::init_pool_db_service() {
        log(LogTag::System, "DB_INIT_WARN", &format!("Pool DB init warning: {}", e));
    }
    log(LogTag::System, "INIT", "🏊 Pool Service initialized");

    // Process token address if provided
    if let Some(token_address) = &args.from_token_address {
        log(
            LogTag::System,
            "TOKEN_ADDRESS",
            &format!("🎯 Processing token address: {}", token_address),
        );

        // Initialize discovery and run triple-API discovery for this token
        let token = token_address.clone();
        let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
        rt.block_on(async move {
            let discovery = pool_discovery::init_pool_discovery();

            match discovery.discover_pools_batch(&[token.clone()]).await {
                Ok(map) => {
                    if let Some(pools) = map.get(&token) {
                        log(
                            LogTag::System,
                            "DISCOVERY_RESULT",
                            &format!(
                                "✅ Discovered {} TOKEN/SOL pools for {}",
                                pools.len(),
                                &token[..8]
                            ),
                        );
                        // Log top 3 by liquidity
                        let mut pools_sorted = pools.clone();
                        pools_sorted.sort_by(|a, b| b.liquidity_usd.partial_cmp(&a.liquidity_usd).unwrap_or(std::cmp::Ordering::Equal));
                        for (i, p) in pools_sorted.iter().take(3).enumerate() {
                            log(
                                LogTag::System,
                                "POOL",
                                &format!(
                                    "#{} {} liq=${:.2} vol24h=${:.2} dex={}",
                                    i + 1,
                                    &p.pair_address[..8],
                                    p.liquidity_usd,
                                    p.volume_24h,
                                    p.dex_id
                                ),
                            );
                        }
                    } else {
                        log(
                            LogTag::System,
                            "DISCOVERY_EMPTY",
                            &format!("⚪ No pools discovered for {}", &token[..8]),
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "DISCOVERY_ERROR",
                        &format!("❌ Discovery error for {}: {}", &token[..8], e),
                    );
                }
            }
        });
    }

    // Process pool address if provided
    if let Some(pool_address) = &args.from_pool_address {
        log(
            LogTag::System,
            "POOL_ADDRESS",
            &format!("🏊 Processing pool address: {}", pool_address),
        );
        // TODO: Add pool address processing logic here
    }

    log(
        LogTag::System,
        "SUCCESS",
        "✅ Pool Service initialized successfully",
    );
    log(LogTag::System, "INFO", "Pool Service Tool is ready");
}
