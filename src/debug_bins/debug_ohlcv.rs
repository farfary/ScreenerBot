// Debug tool for OHLCV module - testing API, parsing, and database operations
//
// Usage:
// cargo run --bin debug_ohlcv -- --help

use clap::{Parser, Subcommand};
use reqwest;
use serde::Deserialize;
use serde_json;

#[derive(Parser)]
#[command(name = "debug_ohlcv")]
#[command(about = "Debug tool for OHLCV module", long_about = None)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  /// Test API connectivity and response parsing
  TestApi {
    /// Pool address to test
    #[arg(short, long)]
    pool: String,

    /// Timeframe (minute, hour, day)
    #[arg(short, long, default_value = "minute")]
    timeframe: String,

    /// Number of candles to fetch
    #[arg(short, long, default_value = "5")]
    limit: usize,
  },

  /// Test our timeframe mapping logic
  TestMapping,

  /// Validate OHLCV data parsing
  TestParsing {
    /// Pool address to test
    #[arg(short, long)]
    pool: String,
  },

  /// Test OHLCV service initialization
  TestInit,

  /// Test fetching OHLCV data from database
  TestFetch {
    /// Token mint address
    #[arg(short, long)]
    mint: String,

    /// Timeframe (1m, 5m, 15m, 1h, 4h, 12h, 1d)
    #[arg(short, long, default_value = "5m")]
    timeframe: String,

    /// Number of candles to fetch
    #[arg(short, long, default_value = "10")]
    limit: usize,
  },

  /// Test discovering available pools for a token
  TestPools {
    /// Token mint address
    #[arg(short, long)]
    mint: String,
  },

  /// Test monitoring functions (add/remove/status)
  TestMonitor {
    /// Token mint address
    #[arg(short, long)]
    mint: String,

    /// Action: add, remove, or status
    #[arg(short, long, default_value = "status")]
    action: String,
  },

  /// Test getting OHLCV metrics
  TestMetrics,

  /// End-to-end workflow test: Discover pool and fetch live data
  TestWorkflow {
    /// Pool address to test
    #[arg(short, long)]
    pool: String,

    /// Timeframe
    #[arg(short, long, default_value = "5m")]
    timeframe: String,

    /// Number of candles
    #[arg(short, long, default_value = "5")]
    limit: usize,
  },
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
  data: Option<ApiData>,
  errors: Option<Vec<ApiError>>,
}

#[derive(Deserialize, Debug)]
struct ApiData {
  attributes: ApiAttributes,
}

#[derive(Deserialize, Debug)]
struct ApiAttributes {
  ohlcv_list: Vec<Vec<f64>>,
}

#[derive(Deserialize, Debug)]
struct ApiError {
  status: String,
  title: String,
}

#[tokio::main]
async fn main() {
  let cli = Cli::parse();

  match cli.command {
    Commands::TestApi {
      pool,
      timeframe,
      limit,
    } => {
      test_api(&pool, &timeframe, limit).await;
    }
    Commands::TestMapping => {
      test_timeframe_mapping();
    }
    Commands::TestParsing { pool } => {
      test_parsing(&pool).await;
    }
    Commands::TestInit => {
      test_init().await;
    }
    Commands::TestFetch {
      mint,
      timeframe,
      limit,
    } => {
      test_fetch(&mint, &timeframe, limit).await;
    }
    Commands::TestPools { mint } => {
      test_pools(&mint).await;
    }
    Commands::TestMonitor { mint, action } => {
      test_monitor(&mint, &action).await;
    }
    Commands::TestMetrics => {
      test_metrics().await;
    }
    Commands::TestWorkflow {
      pool,
      timeframe,
      limit,
    } => {
      test_workflow(&pool, &timeframe, limit).await;
    }
  }
}

async fn test_api(pool: &str, timeframe: &str, limit: usize) {
 println!("Testing OHLCV API");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Pool: {}", pool);
 println!("Timeframe: {}", timeframe);
 println!("Limit: {}", limit);
  println!();

  let url = format!(
    "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}?limit={}",
    pool, timeframe, limit
  );

 println!("Request URL:");
 println!("{}", url);
  println!();

 println!("Fetching data...");

  let client = reqwest::Client::new();
  let response = match client
    .get(&url)
    .header("Accept", "application/json")
    .send()
    .await
  {
    Ok(resp) => resp,
    Err(e) => {
 println!("Request failed: {}", e);
      return;
    }
  };

  let status = response.status();
 println!("Response Status: {}", status);
  println!();

  let body_text = match response.text().await {
    Ok(text) => text,
    Err(e) => {
 println!("Failed to read response: {}", e);
      return;
    }
  };

  // Try to parse as JSON
  match serde_json::from_str::<ApiResponse>(&body_text) {
    Ok(api_response) => {
      if let Some(errors) = api_response.errors {
 println!("API Error:");
        for error in errors {
 println!("Status: {}", error.status);
 println!("Title: {}", error.title);
        }
      } else if let Some(data) = api_response.data {
        println!(
 "Success! Received {} candles",
          data.attributes.ohlcv_list.len()
        );
        println!();
 println!("Sample Data:");
 println!("Format: [timestamp, open, high, low, close, volume]");
        println!();

        for (i, candle) in data.attributes.ohlcv_list.iter().enumerate() {
          if candle.len() == 6 {
            println!(
 "Candle {}: [{}, {:.8}, {:.8}, {:.8}, {:.8}, {:.2}]",
              i + 1,
              candle[0] as i64,
              candle[1],
              candle[2],
              candle[3],
              candle[4],
              candle[5]
            );
          }
        }
      } else {
 println!("Unexpected response format");
      }
    }
    Err(e) => {
 println!("Failed to parse JSON: {}", e);
      println!("\n Raw Response:");
      println!("{}", &body_text[..body_text.len().min(500)]);
    }
  }
}

fn test_timeframe_mapping() {
 println!("Testing Timeframe API Mapping");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
  println!();

  use screenerbot::ohlcvs::Timeframe;

  let timeframes = vec![
    ("Minute1", Timeframe::Minute1),
    ("Minute5", Timeframe::Minute5),
    ("Minute15", Timeframe::Minute15),
    ("Hour1", Timeframe::Hour1),
    ("Hour4", Timeframe::Hour4),
    ("Hour12", Timeframe::Hour12),
    ("Day1", Timeframe::Day1),
  ];

 println!("Internal → API Parameter Mapping:");
  println!();

  for (name, tf) in timeframes {
    let api_param = tf.to_api_param();
    let display = tf.as_str();
    let seconds = tf.to_seconds();

 println!("{} ({}):", name, display);
 println!("API param: '{}'", api_param);
 println!("Duration: {} seconds", seconds);

    // Validate
    match api_param {
 "minute"| "hour"| "day"=> println!("Status: Valid"),
 _ => println!("Status: INVALID - API will reject this!"),
    }
    println!();
  }

  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("All mappings validated");
}

async fn test_parsing(pool: &str) {
 println!("Testing OHLCV Data Parsing");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Pool: {}", pool);
  println!();

  use screenerbot::ohlcvs::OhlcvDataPoint;

  // Fetch real data
  let url = format!(
    "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/minute?limit=5",
    pool
  );

 println!("Fetching data from API...");

  let client = reqwest::Client::new();
  let response = match client
    .get(&url)
    .header("Accept", "application/json")
    .send()
    .await
  {
    Ok(resp) => resp,
    Err(e) => {
 println!("Request failed: {}", e);
      return;
    }
  };

  if !response.status().is_success() {
 println!("API returned error: {}", response.status());
    return;
  }

  let body_text = match response.text().await {
    Ok(text) => text,
    Err(e) => {
 println!("Failed to read response: {}", e);
      return;
    }
  };

  match serde_json::from_str::<ApiResponse>(&body_text) {
    Ok(api_response) => {
      if let Some(data) = api_response.data {
 println!("API response parsed successfully");
        println!();

        // Test our OhlcvDataPoint conversion
 println!("Converting to OhlcvDataPoint structs:");
        println!();

        for (i, candle) in data.attributes.ohlcv_list.iter().enumerate() {
          if candle.len() == 6 {
            let point = OhlcvDataPoint {
              timestamp: candle[0] as i64,
              open: candle[1],
              high: candle[2],
              low: candle[3],
              close: candle[4],
              volume: candle[5],
            };

 println!("Candle {}:", i + 1);
            println!(
 "Timestamp: {} ({})",
              point.timestamp,
              chrono::DateTime::from_timestamp(point.timestamp, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Invalid".to_string())
            );
 println!("Open: {:.8}", point.open);
 println!("High: {:.8}", point.high);
 println!("Low: {:.8}", point.low);
 println!("Close: {:.8}", point.close);
 println!("Volume: {:.2}", point.volume);

            // Validate
            if point.is_valid() {
 println!("Valid: Yes");
            } else {
 println!("Valid: No (high < low or invalid range)");
            }
            println!();
          } else {
            println!(
 "Candle {} has {} elements (expected 6)",
              i + 1,
              candle.len()
            );
          }
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("All candles parsed and validated successfully");
      } else {
 println!("No data in response");
      }
    }
    Err(e) => {
 println!("Failed to parse response: {}", e);
    }
  }
}

/// Test OHLCV service initialization
async fn test_init() {
  use screenerbot::ohlcvs;

 println!("Testing OHLCV Service Initialization");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
  println!();

 println!("Initializing OHLCV service...");
 println!("(Service auto-initializes on first use)");

  // Get metrics - this will initialize the service
  let metrics = ohlcvs::get_metrics().await;

 println!("OHLCV service initialized successfully");
  println!();

 println!("Initial Metrics:");
 println!("Monitored tokens: {}", metrics.tokens_monitored);
 println!("Total pools tracked: {}", metrics.pools_tracked);
  println!(
 "API calls/min: {:.2}",
    metrics.api_calls_per_minute
  );
  println!(
 "Cache hit rate: {:.1}%",
    metrics.cache_hit_rate * 100.0
  );
  println!(
 "Avg fetch latency: {:.2} ms",
    metrics.average_fetch_latency_ms
  );
 println!("Gaps detected: {}", metrics.gaps_detected);
 println!("Gaps filled: {}", metrics.gaps_filled);
 println!("Data points stored: {}", metrics.data_points_stored);

  println!();
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Test fetching OHLCV data from database
async fn test_fetch(mint: &str, timeframe_str: &str, limit: usize) {
  use screenerbot::ohlcvs::{self, Timeframe};

 println!("Testing OHLCV Data Fetch");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Mint: {}", mint);
 println!("Timeframe: {}", timeframe_str);
 println!("Limit: {}", limit);
  println!();

  // Parse timeframe
  let timeframe = match timeframe_str {
 "1m"=> Timeframe::Minute1,
 "5m"=> Timeframe::Minute5,
 "15m"=> Timeframe::Minute15,
 "1h"=> Timeframe::Hour1,
 "4h"=> Timeframe::Hour4,
 "12h"=> Timeframe::Hour12,
 "1d"=> Timeframe::Day1,
    _ => {
 println!("Invalid timeframe: {}", timeframe_str);
 println!("Valid options: 1m, 5m, 15m, 1h, 4h, 12h, 1d");
      return;
    }
  };

  // Fetch data (service auto-initializes)
 println!("Fetching data from database...");
  match ohlcvs::get_ohlcv_data(mint, timeframe, None, limit, None, None).await {
    Ok(data) => {
      if data.is_empty() {
 println!("No data found in database");
        println!();
 println!("Tip: Check if:");
 println!("1. Token is being monitored (use test-monitor)");
 println!("2. OHLCV service has fetched data (use test-pools)");
 println!("3. Token has liquidity pools on GeckoTerminal");
      } else {
 println!("Fetched {} candles", data.len());
        println!();

        for (i, point) in data.iter().enumerate().take(5) {
 println!("Candle {}:", i + 1);

          // Format timestamp as readable date
          let dt = chrono::DateTime::from_timestamp(point.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Invalid timestamp".to_string());

 println!("Timestamp: {} ({})", point.timestamp, dt);
 println!("Open: ${:.8}", point.open);
 println!("High: ${:.8}", point.high);
 println!("Low: ${:.8}", point.low);
 println!("Close: ${:.8}", point.close);
 println!("Volume: ${:.2}", point.volume);
          println!(
 "Valid: {}",
 if point.is_valid() { ""} else { ""}
          );
          println!();
        }

        if data.len() > 5 {
 println!("... and {} more candles", data.len() - 5);
          println!();
        }
      }
    }
    Err(e) => {
 println!("Failed to fetch data: {}", e);
    }
  }

  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Test discovering available pools for a token
async fn test_pools(mint: &str) {
  use screenerbot::ohlcvs;

 println!("Testing Pool Discovery");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Mint: {}", mint);
  println!();

  // Get available pools (service auto-initializes)
 println!("Discovering pools...");
  match ohlcvs::get_available_pools(mint).await {
    Ok(pools) => {
      if pools.is_empty() {
 println!("No pools found on GeckoTerminal");
        println!();
 println!("This could mean:");
 println!("1. Token doesn't have liquidity pools");
 println!("2. Pools aren't indexed by GeckoTerminal yet");
 println!("3. Mint address is incorrect");
      } else {
 println!("Found {} pool(s)", pools.len());
        println!();

        for (i, pool) in pools.iter().enumerate() {
 println!("Pool {}:", i + 1);
 println!("Address: {}", pool.address);
 println!("DEX: {}", pool.dex);
 println!("Liquidity: ${:.2}", pool.liquidity);
          println!(
 "Default: {}",
 if pool.is_default { ""} else { ""}
          );
          println!(
 "Healthy: {}",
 if pool.is_healthy { ""} else { ""}
          );
          if let Some(last_fetch) = pool.last_successful_fetch {
 println!("Last Fetch: {}", last_fetch.format("%Y-%m-%d %H:%M:%S"));
          }
          println!();
        }
      }
    }
    Err(e) => {
 println!("Failed to discover pools: {}", e);
    }
  }

  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Test monitoring functions (add/remove/status)
async fn test_monitor(mint: &str, action: &str) {
  use screenerbot::ohlcvs::{self, Priority};

 println!("Testing Token Monitoring");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Mint: {}", mint);
 println!("Action: {}", action);
  println!();

  // Service auto-initializes on first use
  match action {
 "add"=> {
 println!("Adding token to monitoring (High priority)...");
      match ohlcvs::add_token_monitoring(mint, Priority::High).await {
        Ok(_) => {
 println!("Token added to monitoring");
          println!();
 println!("The OHLCV service will now:");
 println!("1. Discover available pools");
 println!("2. Start fetching data every cycle");
 println!("3. Fill any data gaps automatically");
        }
        Err(e) => {
 println!("Failed to add monitoring: {}", e);
        }
      }
    }
 "remove"=> {
 println!("Removing token from monitoring...");
      match ohlcvs::remove_token_monitoring(mint).await {
        Ok(_) => {
 println!("Token removed from monitoring");
          println!();
 println!("Existing data remains in database");
        }
        Err(e) => {
 println!("Failed to remove monitoring: {}", e);
        }
      }
    }
 "status"=> {
 println!("Checking monitoring status...");
      match ohlcvs::has_data(mint).await {
        Ok(has_data) => {
          if has_data {
 println!("Token has data in database");

            // Check for gaps in each timeframe
            println!();
 println!("Checking data gaps:");

            for tf_str in ["1m", "5m", "15m", "1h", "4h", "12h", "1d"] {
              let timeframe = match tf_str {
 "1m"=> screenerbot::ohlcvs::Timeframe::Minute1,
 "5m"=> screenerbot::ohlcvs::Timeframe::Minute5,
 "15m"=> screenerbot::ohlcvs::Timeframe::Minute15,
 "1h"=> screenerbot::ohlcvs::Timeframe::Hour1,
 "4h"=> screenerbot::ohlcvs::Timeframe::Hour4,
 "12h"=> screenerbot::ohlcvs::Timeframe::Hour12,
 "1d"=> screenerbot::ohlcvs::Timeframe::Day1,
                _ => {
                  continue;
                }
              };

              match ohlcvs::get_data_gaps(mint, timeframe).await {
                Ok(gaps) => {
                  if gaps.is_empty() {
 println!("{} - No gaps", tf_str);
                  } else {
 println!("{} - {} gap(s) found", tf_str, gaps.len());
                  }
                }
                Err(_) => {
 println!("{} - Unable to check", tf_str);
                }
              }
            }
          } else {
 println!("No data found for this token");
            println!();
 println!("Use 'test-monitor --action add'to start monitoring");
          }
        }
        Err(e) => {
 println!("Failed to check status: {}", e);
        }
      }
    }
    _ => {
 println!("Invalid action: {}", action);
 println!("Valid options: add, remove, status");
    }
  }

  println!();
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Test getting OHLCV metrics
async fn test_metrics() {
  use screenerbot::ohlcvs;

 println!("Testing OHLCV Metrics");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
  println!();

  // Get metrics (service auto-initializes)
 println!("Fetching metrics...");
  let metrics = ohlcvs::get_metrics().await;

 println!("Metrics retrieved");
  println!();
 println!("OHLCV Service Metrics:");
 println!("Monitored Tokens: {}", metrics.tokens_monitored);
 println!("Pools Tracked: {}", metrics.pools_tracked);
  println!(
 "API Calls/min: {:.2}",
    metrics.api_calls_per_minute
  );
  println!(
 "Cache Hit Rate: {:.1}%",
    metrics.cache_hit_rate * 100.0
  );
  println!(
 "Avg Fetch Latency: {:.2} ms",
    metrics.average_fetch_latency_ms
  );
 println!("Gaps Detected: {}", metrics.gaps_detected);
 println!("Gaps Filled: {}", metrics.gaps_filled);
 println!("Data Points Stored: {}", metrics.data_points_stored);
  println!(
 "Database Size: {:.2} MB",
    metrics.database_size_mb
  );

  if let Some(oldest) = metrics.oldest_data_timestamp {
    println!(
 "Oldest Data: {}",
      oldest.format("%Y-%m-%d %H:%M:%S")
    );
  }

  println!();
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// End-to-end workflow test: Fetch data directly from API and parse
async fn test_workflow(pool: &str, timeframe_str: &str, limit: usize) {
  use screenerbot::ohlcvs::{OhlcvDataPoint, Timeframe};

 println!("Testing End-to-End OHLCV Workflow");
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Pool: {}", pool);
 println!("Timeframe: {}", timeframe_str);
 println!("Limit: {}", limit);
  println!();

  // Parse timeframe for mapping
  let timeframe = match timeframe_str {
 "1m"=> Timeframe::Minute1,
 "5m"=> Timeframe::Minute5,
 "15m"=> Timeframe::Minute15,
 "1h"=> Timeframe::Hour1,
 "4h"=> Timeframe::Hour4,
 "12h"=> Timeframe::Hour12,
 "1d"=> Timeframe::Day1,
    _ => {
 println!("Invalid timeframe: {}", timeframe_str);
      return;
    }
  };

  // Get API parameter
  let api_param = timeframe.to_api_param();
 println!("Timeframe Mapping:");
 println!("Internal: {:?}", timeframe);
 println!("API param: {}", api_param);
  println!();

  // Fetch from API
 println!("Fetching data from GeckoTerminal API...");
  let url = format!(
    "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}?limit={}",
    pool, api_param, limit
  );

  let client = reqwest::Client::new();
  match client.get(&url).send().await {
    Ok(response) => {
      let status = response.status();
      println!(
 "Status: {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("")
      );

      if status.is_success() {
        match response.json::<ApiResponse>().await {
          Ok(api_response) => {
            if let Some(data) = api_response.data {
              let candles = data.attributes.ohlcv_list;
 println!("Received {} candles from API", candles.len());
              println!();

              // Parse into OhlcvDataPoint
 println!("Converting to OhlcvDataPoint structs...");
              let mut success_count = 0;
              let mut error_count = 0;

              for (i, candle) in candles.iter().enumerate().take(limit) {
                if candle.len() == 6 {
                  let point = OhlcvDataPoint {
                    timestamp: candle[0] as i64,
                    open: candle[1],
                    high: candle[2],
                    low: candle[3],
                    close: candle[4],
                    volume: candle[5],
                  };

                  if point.is_valid() {
                    if i < 3 {
                      let dt = chrono::DateTime::from_timestamp(
                        point.timestamp,
                        0,
                      )
                      .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                      .unwrap_or_else(|| "Invalid".to_string());

 println!("Candle {}: ", i + 1);
 println!("Time: {} ({})", point.timestamp, dt);
                      println!(
 "O/H/L/C: ${:.8} / ${:.8} / ${:.8} / ${:.8}",
                        point.open, point.high, point.low, point.close
                      );
 println!("Volume: ${:.2}", point.volume);
                      println!();
                    }
                    success_count += 1;
                  } else {
                    if i < 3 {
                      println!(
 "Candle {}: Invalid (high < low or bad range)",
                        i + 1
                      );
                      println!();
                    }
                    error_count += 1;
                  }
                } else {
                  error_count += 1;
                }
              }

              if candles.len() > 3 {
 println!("... and {} more candles", candles.len() - 3);
                println!();
              }

              println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
 println!("Workflow Summary:");
 println!("API Response: Success");
 println!("Total Candles: {}", candles.len());
 println!("Valid Candles: {} ", success_count);
 println!("Invalid Candles: {} ", error_count);
 println!("Timeframe Map: {} → {}", timeframe_str, api_param);
              println!();

              if error_count == 0 {
 println!("All candles parsed and validated successfully!");
              } else {
 println!("Some candles failed validation");
              }
            } else {
 println!("No data in API response");
            }
          }
          Err(e) => {
 println!("Failed to parse JSON: {}", e);
          }
        }
      } else {
 println!("API returned error: {}", status);
      }
    }
    Err(e) => {
 println!("Failed to fetch from API: {}", e);
    }
  }

  println!();
  println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
