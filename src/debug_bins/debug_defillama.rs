use clap::Parser;
use screenerbot::apis::defillama::DefiLlamaClient;
use std::error::Error;

#[derive(Parser)]
#[command(name = "debug_defillama")]
#[command(about = "Debug tool for DeFiLlama API", long_about = None)]
struct Args {
    /// Test protocols endpoint
    #[arg(short, long)]
    protocols: bool,

    /// Test token price endpoint with a specific mint address
    #[arg(short = 't', long)]
    token: Option<String>,

    /// Test all endpoints
    #[arg(short, long)]
    all: bool,

    /// Show detailed output
    #[arg(short, long)]
    verbose: bool,

    /// Maximum number of results to display
    #[arg(short, long, default_value = "10")]
    limit: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    println!("DeFiLlama API Debug Tool\n");
    println!("{}", "=".repeat(80));

    // Create DeFiLlama client with hardcoded timeout
    let client = DefiLlamaClient::new(true).expect("Failed to create DeFiLlama client");

    // Decide what to test
    let test_protocols = args.all || args.protocols;
    let test_price = args.all || args.token.is_some();

    // Test protocols
    if test_protocols {
        println!("\n[TEST] Fetching DeFi protocols\n");
        println!("{}", "=".repeat(80));

        match client.fetch_protocols().await {
            Ok(protocols) => {
                let solana_protocols =
                    DefiLlamaClient::extract_solana_addresses_with_names(&protocols);

                println!("✓ Successfully fetched {} total protocols", protocols.len());
                println!("✓ Found {} Solana protocols", solana_protocols.len());

                println!("\n[FIRST {} SOLANA PROTOCOLS]", args.limit);
                for (i, (name, mint)) in solana_protocols.iter().enumerate().take(args.limit) {
                    println!("  {}. {} - {}", i + 1, name, mint);
                }

                if solana_protocols.len() > args.limit {
                    println!(
                        "  ... and {} more Solana protocols",
                        solana_protocols.len() - args.limit
                    );
                }

                if args.verbose {
                    println!("\n[DETAILED PROTOCOL INFO (first {})]", args.limit);
                    for protocol in protocols.iter().take(args.limit) {
                        println!("\n  Protocol: {}", protocol.name);
                        println!("    Symbol: {}", protocol.symbol);
                        if let Some(chain) = &protocol.chain {
                            println!("    Chain: {}", chain);
                        }
                        if let Some(tvl) = protocol.tvl {
                            println!("    TVL: ${:.2}M", tvl / 1_000_000.0);
                        }
                        if let Some(url) = &protocol.url {
                            println!("    URL: {}", url);
                        }
                    }
                }
            }
            Err(e) => {
                println!("✗ Failed to fetch protocols: {}", e);
            }
        }
    }

    // Test token price
    if test_price {
        let mint = args
            .token
            .unwrap_or_else(|| "So11111111111111111111111111111111111111112".to_string()); // Default to SOL

        println!("\n[TEST] Fetching price for token: {}\n", mint);
        println!("{}", "=".repeat(80));

        match client.fetch_token_price(&mint).await {
            Ok(price) => {
                println!("✓ Successfully fetched price data");
                println!("\n[PRICE INFORMATION]");
                println!("  Token: {}", mint);
                println!("  Price: ${:.9}", price);
            }
            Err(e) => {
                println!("✗ Failed to fetch token price: {}", e);
            }
        }
    }

    // Print stats
    println!("\n{}", "=".repeat(80));
    let stats = client.get_stats().await;
    println!("\n[API STATS]");
    println!("  Total Requests: {}", stats.total_requests);
    println!("  Successful: {}", stats.successful_requests);
    println!("  Failed: {}", stats.failed_requests);
    println!("  Cache Hits: {}", stats.cache_hits);
    println!("  Cache Misses: {}", stats.cache_misses);
    println!(
        "  Avg Response Time: {:.2}ms",
        stats.average_response_time_ms
    );

    println!("\n{}", "=".repeat(80));
    println!("\nTest completed!");

    // Show usage hint if no tests were run
    if !test_protocols && !test_price {
        println!("\nNo tests specified. Use --protocols, --token <MINT>, or --all");
        println!("Example: cargo run --bin debug_defillama -- --all");
        println!("Example: cargo run --bin debug_defillama -- --token So11111111111111111111111111111111111111112");
    }

    Ok(())
}
