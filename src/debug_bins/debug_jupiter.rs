/// Debug tool for Jupiter API
///
/// Tests Jupiter token discovery endpoints.
///
/// Usage:
/// cargo run --bin debug_jupiter # Test all endpoints with default params
/// cargo run --bin debug_jupiter --recent # Test recent tokens
/// cargo run --bin debug_jupiter --organic # Test top organic score (24h)
/// cargo run --bin debug_jupiter --traded # Test top traded (24h)
/// cargo run --bin debug_jupiter --trending # Test top trending (24h)
/// cargo run --bin debug_jupiter --all # Test all endpoints
use clap::Parser;
use screenerbot::apis::jupiter::JupiterClient;

#[derive(Parser, Debug)]
#[command(name = "debug_jupiter")]
#[command(about = "Test Jupiter API endpoints")]
struct Args {
  /// Test recent tokens endpoint
  #[arg(long)]
  recent: bool,

  /// Test top organic score endpoint
  #[arg(long)]
  organic: bool,

  /// Test top traded endpoint
  #[arg(long)]
  traded: bool,

  /// Test top trending endpoint
  #[arg(long)]
  trending: bool,

  /// Test all endpoints
  #[arg(long)]
  all: bool,

  /// Interval for top endpoints (5m, 1h, 6h, 24h)
  #[arg(long, default_value = "24h")]
  interval: String,

  /// Limit results
  #[arg(long, default_value = "10")]
  limit: usize,
}

fn print_separator() {
  println!("\n{}", "=".repeat(80));
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  println!("Jupiter API Debug Tool");
  println!("{}", "=".repeat(80));

  // Create Jupiter client with hardcoded timeout
  let client = JupiterClient::new(true).expect("Failed to create Jupiter client");

  // Decide what to test
  let test_all = args.all || (!args.recent && !args.organic && !args.traded && !args.trending);

  if args.recent || test_all {
    test_recent(&client).await;
  }

  if args.organic || test_all {
    test_top_organic(&client, &args.interval, args.limit).await;
  }

  if args.traded || test_all {
    test_top_traded(&client, &args.interval, args.limit).await;
  }

  if args.trending || test_all {
    test_top_trending(&client, &args.interval, args.limit).await;
  }

  // Print stats
  print_separator();
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

  print_separator();
  println!("\nTest completed!");
}

async fn test_recent(client: &JupiterClient) {
  println!("\n[TEST] Fetching recent tokens from /tokens/v2/recent");
  print_separator();

  match client.fetch_recent_tokens().await {
    Ok(tokens) => {
 println!("Successfully fetched {} recent tokens", tokens.len());
      println!("\n[TOP 10 RECENT TOKENS]");
      for (i, token) in tokens.iter().take(10).enumerate() {
        println!(
 "{}. {} ({}) - {}",
          i + 1,
          token.symbol,
          token.name,
          token.id
        );
        println!(
 "Decimals: {}, Holders: {}",
          token.decimals,
          token.holder_count.unwrap_or(0)
        );
        if let Some(score) = token.organic_score {
          println!(
 "Organic Score: {:.2} ({})",
            score,
            token.organic_score_label.as_deref().unwrap_or("N/A")
          );
        }
        if let Some(verified) = token.is_verified {
          if verified {
 println!("Verified");
          }
        }
      }
      if tokens.len() > 10 {
 println!("... and {} more tokens", tokens.len() - 10);
      }
    }
    Err(e) => {
 println!("Failed to fetch recent tokens: {}", e);
    }
  }
}

async fn test_top_organic(client: &JupiterClient, interval: &str, limit: usize) {
  println!(
    "\n[TEST] Fetching top organic score tokens from /tokens/v2/toporganicscore/{}",
    interval
  );
  print_separator();

  match client.fetch_top_organic_score(interval, Some(limit)).await {
    Ok(tokens) => {
      println!(
 "Successfully fetched {} top organic score tokens",
        tokens.len()
      );
      println!(
        "\n[TOP {} ORGANIC SCORE TOKENS ({})]",
        limit.min(10),
        interval
      );
      for (i, token) in tokens.iter().take(10).enumerate() {
        println!(
 "{}. {} ({}) - {}",
          i + 1,
          token.symbol,
          token.name,
          token.id
        );
        println!(
 "Organic Score: {:.2} ({})",
          token.organic_score.unwrap_or(0.0),
          token.organic_score_label.as_deref().unwrap_or("N/A")
        );
        println!(
 "Holders: {}, Verified: {}",
          token.holder_count.unwrap_or(0),
          token.is_verified.unwrap_or(false)
        );
        if let Some(price) = token.usd_price {
          println!(
 "Price: ${:.6}, MCap: ${:.2}M",
            price,
            token.mcap.unwrap_or(0.0) / 1_000_000.0
          );
        }
      }
      if tokens.len() > 10 {
 println!("... and {} more tokens", tokens.len() - 10);
      }
    }
    Err(e) => {
 println!("Failed to fetch top organic score tokens: {}", e);
    }
  }
}

async fn test_top_traded(client: &JupiterClient, interval: &str, limit: usize) {
  println!(
    "\n[TEST] Fetching top traded tokens from /tokens/v2/toptraded/{}",
    interval
  );
  print_separator();

  match client.fetch_top_traded(interval, Some(limit)).await {
    Ok(tokens) => {
 println!("Successfully fetched {} top traded tokens", tokens.len());
      println!("\n[TOP {} TRADED TOKENS ({})]", limit.min(10), interval);
      for (i, token) in tokens.iter().take(10).enumerate() {
        println!(
 "{}. {} ({}) - {}",
          i + 1,
          token.symbol,
          token.name,
          token.id
        );
        if let (Some(price), Some(liquidity)) = (token.usd_price, token.liquidity) {
          println!(
 "Price: ${:.6}, Liquidity: ${:.2}M",
            price,
            liquidity / 1_000_000.0
          );
        }
        if let Some(stats24h) = &token.stats24h {
          if let (Some(buy_vol), Some(sell_vol)) =
            (stats24h.buy_volume, stats24h.sell_volume)
          {
            println!(
 "24h Volume: Buy ${:.2}M / Sell ${:.2}M",
              buy_vol / 1_000_000.0,
              sell_vol / 1_000_000.0
            );
          }
        }
 println!("Verified: {}", token.is_verified.unwrap_or(false));
      }
      if tokens.len() > 10 {
 println!("... and {} more tokens", tokens.len() - 10);
      }
    }
    Err(e) => {
 println!("Failed to fetch top traded tokens: {}", e);
    }
  }
}

async fn test_top_trending(client: &JupiterClient, interval: &str, limit: usize) {
  println!(
    "\n[TEST] Fetching top trending tokens from /tokens/v2/toptrending/{}",
    interval
  );
  print_separator();

  match client.fetch_top_trending(interval, Some(limit)).await {
    Ok(tokens) => {
      println!(
 "Successfully fetched {} top trending tokens",
        tokens.len()
      );
      println!("\n[TOP {} TRENDING TOKENS ({})]", limit.min(10), interval);
      for (i, token) in tokens.iter().take(10).enumerate() {
        println!(
 "{}. {} ({}) - {}",
          i + 1,
          token.symbol,
          token.name,
          token.id
        );
        println!(
 "Holders: {}, Verified: {}",
          token.holder_count.unwrap_or(0),
          token.is_verified.unwrap_or(false)
        );
        if let Some(stats24h) = &token.stats24h {
          if let Some(holder_change) = stats24h.holder_change {
 println!("24h Holder Change: {:+}", holder_change);
          }
          if let Some(price_change) = stats24h.price_change {
 println!("24h Price Change: {:+.2}%", price_change);
          }
        }
        if let (Some(price), Some(mcap)) = (token.usd_price, token.mcap) {
          println!(
 "Price: ${:.6}, MCap: ${:.2}M",
            price,
            mcap / 1_000_000.0
          );
        }
      }
      if tokens.len() > 10 {
 println!("... and {} more tokens", tokens.len() - 10);
      }
    }
    Err(e) => {
 println!("Failed to fetch top trending tokens: {}", e);
    }
  }
}
