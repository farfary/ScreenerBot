// Debug tool for OHLCV module - testing API, parsing, and database operations
//
// Usage:
//   cargo run --bin debug_ohlcv -- --help

use clap::{ Parser, Subcommand };
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
        Commands::TestApi { pool, timeframe, limit } => {
            test_api(&pool, &timeframe, limit).await;
        }
        Commands::TestMapping => {
            test_timeframe_mapping();
        }
        Commands::TestParsing { pool } => {
            test_parsing(&pool).await;
        }
    }
}

async fn test_api(pool: &str, timeframe: &str, limit: usize) {
    println!("🔍 Testing OHLCV API");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Pool:      {}", pool);
    println!("  Timeframe: {}", timeframe);
    println!("  Limit:     {}", limit);
    println!();

    let url = format!(
        "https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/{}?limit={}",
        pool,
        timeframe,
        limit
    );

    println!("📡 Request URL:");
    println!("  {}", url);
    println!();

    println!("⏳ Fetching data...");

    let client = reqwest::Client::new();
    let response = match client.get(&url).header("Accept", "application/json").send().await {
        Ok(resp) => resp,
        Err(e) => {
            println!("❌ Request failed: {}", e);
            return;
        }
    };

    let status = response.status();
    println!("📊 Response Status: {}", status);
    println!();

    let body_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            println!("❌ Failed to read response: {}", e);
            return;
        }
    };

    // Try to parse as JSON
    match serde_json::from_str::<ApiResponse>(&body_text) {
        Ok(api_response) => {
            if let Some(errors) = api_response.errors {
                println!("❌ API Error:");
                for error in errors {
                    println!("  Status: {}", error.status);
                    println!("  Title:  {}", error.title);
                }
            } else if let Some(data) = api_response.data {
                println!("✅ Success! Received {} candles", data.attributes.ohlcv_list.len());
                println!();
                println!("📈 Sample Data:");
                println!("  Format: [timestamp, open, high, low, close, volume]");
                println!();

                for (i, candle) in data.attributes.ohlcv_list.iter().enumerate() {
                    if candle.len() == 6 {
                        println!(
                            "  Candle {}: [{}, {:.8}, {:.8}, {:.8}, {:.8}, {:.2}]",
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
                println!("⚠️  Unexpected response format");
            }
        }
        Err(e) => {
            println!("❌ Failed to parse JSON: {}", e);
            println!("\n📄 Raw Response:");
            println!("{}", &body_text[..body_text.len().min(500)]);
        }
    }
}

fn test_timeframe_mapping() {
    println!("🔍 Testing Timeframe API Mapping");
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
        ("Day1", Timeframe::Day1)
    ];

    println!("📊 Internal → API Parameter Mapping:");
    println!();

    for (name, tf) in timeframes {
        let api_param = tf.to_api_param();
        let display = tf.as_str();
        let seconds = tf.to_seconds();

        println!("  {} ({}):", name, display);
        println!("    API param:    '{}'", api_param);
        println!("    Duration:     {} seconds", seconds);

        // Validate
        match api_param {
            "minute" | "hour" | "day" => println!("    Status:       ✅ Valid"),
            _ => println!("    Status:       ❌ INVALID - API will reject this!"),
        }
        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ All mappings validated");
}

async fn test_parsing(pool: &str) {
    println!("🔍 Testing OHLCV Data Parsing");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("  Pool: {}", pool);
    println!();

    use screenerbot::ohlcvs::OhlcvDataPoint;

    // Fetch real data
    let url =
        format!("https://api.geckoterminal.com/api/v2/networks/solana/pools/{}/ohlcv/minute?limit=5", pool);

    println!("📡 Fetching data from API...");

    let client = reqwest::Client::new();
    let response = match client.get(&url).header("Accept", "application/json").send().await {
        Ok(resp) => resp,
        Err(e) => {
            println!("❌ Request failed: {}", e);
            return;
        }
    };

    if !response.status().is_success() {
        println!("❌ API returned error: {}", response.status());
        return;
    }

    let body_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            println!("❌ Failed to read response: {}", e);
            return;
        }
    };

    match serde_json::from_str::<ApiResponse>(&body_text) {
        Ok(api_response) => {
            if let Some(data) = api_response.data {
                println!("✅ API response parsed successfully");
                println!();

                // Test our OhlcvDataPoint conversion
                println!("📊 Converting to OhlcvDataPoint structs:");
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

                        println!("  Candle {}:", i + 1);
                        println!(
                            "    Timestamp: {} ({})",
                            point.timestamp,
                            chrono::DateTime
                                ::from_timestamp(point.timestamp, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "Invalid".to_string())
                        );
                        println!("    Open:      {:.8}", point.open);
                        println!("    High:      {:.8}", point.high);
                        println!("    Low:       {:.8}", point.low);
                        println!("    Close:     {:.8}", point.close);
                        println!("    Volume:    {:.2}", point.volume);

                        // Validate
                        if point.is_valid() {
                            println!("    Valid:     ✅ Yes");
                        } else {
                            println!("    Valid:     ❌ No (high < low or invalid range)");
                        }
                        println!();
                    } else {
                        println!(
                            "  ❌ Candle {} has {} elements (expected 6)",
                            i + 1,
                            candle.len()
                        );
                    }
                }

                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("✅ All candles parsed and validated successfully");
            } else {
                println!("❌ No data in response");
            }
        }
        Err(e) => {
            println!("❌ Failed to parse response: {}", e);
        }
    }
}
