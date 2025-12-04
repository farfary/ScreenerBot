use clap::Parser;
use screenerbot::apis::coingecko::CoinGeckoClient;
use std::error::Error;

#[derive(Parser)]
#[command(name = "debug_coingecko")]
#[command(about = "Debug tool for CoinGecko API", long_about = None)]
struct Args {
  /// Show detailed output for each coin
  #[arg(short, long)]
  verbose: bool,

  /// Maximum number of results to display
  #[arg(short, long, default_value = "10")]
  limit: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let args = Args::parse();

  println!("CoinGecko API Debug Tool\n");
  println!("{}", "=".repeat(80));

  // Create CoinGecko client with hardcoded timeout
  let client = CoinGeckoClient::new(true).expect("Failed to create CoinGecko client");

  // Test coins list
  println!("\n[TEST] Fetching coins list with platforms\n");
  println!("{}", "=".repeat(80));

  match client.fetch_coins_list().await {
    Ok(coins) => {
      let solana_coins = CoinGeckoClient::extract_solana_addresses_with_names(&coins);

 println!("Successfully fetched {} total coins", coins.len());
 println!("Found {} Solana tokens", solana_coins.len());

      if args.verbose {
        println!("\n[SOLANA TOKENS (showing first {})]", args.limit);
        for (i, (name, mint)) in solana_coins.iter().enumerate().take(args.limit) {
 println!("{}. {} - {}", i + 1, name, mint);
        }

        if solana_coins.len() > args.limit {
          println!(
 "... and {} more Solana tokens",
            solana_coins.len() - args.limit
          );
        }
      } else {
        println!("\n[FIRST {} SOLANA TOKENS]", args.limit);
        for (i, (name, mint)) in solana_coins.iter().enumerate().take(args.limit) {
 println!("{}. {} - {}", i + 1, name, mint);
        }
      }
    }
    Err(e) => {
 println!("Failed to fetch coins list: {}", e);
    }
  }

  // Print stats
  println!("\n{}", "=".repeat(80));
  let stats = client.get_stats().await;
  println!("\n[API STATS]");
 println!("Total Requests: {}", stats.total_requests);
 println!("Successful: {}", stats.successful_requests);
 println!("Failed: {}", stats.failed_requests);
 println!("Cache Hits: {}", stats.cache_hits);
 println!("Cache Misses: {}", stats.cache_misses);
  println!(
 "Avg Response Time: {:.2}ms",
    stats.average_response_time_ms
  );

  println!("\n{}", "=".repeat(80));
  println!("\nTest completed!");

  Ok(())
}
