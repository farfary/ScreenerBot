use chrono::{DateTime, Utc};
use clap::{Arg, Command};
use rusqlite;
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{self as logger, LogTag};
use screenerbot::ohlcvs::{get_ohlcv_data, Timeframe};
use screenerbot::pools::{init_pool_service, set_debug_token_override, stop_pool_service};
use screenerbot::positions::{get_db_open_positions, initialize_positions_database};
use std::sync::Arc;
use tokio::sync::Notify;

/// Debug tool to fetch and store 7-day price history for open positions
/// Initializes DexScreener, GeckoTerminal, and OHLCV services and validates price calculations
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Debug Fetch Position Prices")
        .about(
            "Initialize all services and fetch 7-day price history for open positions with validation"
        )
        .arg(
            Arg::new("days")
                .long("days")
                .value_name("N")
                .help("Number of days of price history to fetch")
                .required(false)
                .default_value("7")
        )
        .arg(
            Arg::new("validate-only")
                .long("validate-only")
                .help("Only validate existing data, don't fetch new data")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-ohlcv")
                .long("debug-ohlcv")
                .help("Enable detailed OHLCV system debug logs")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-api")
                .long("debug-api")
                .help("Enable external API debug logs (DexScreener/GeckoTerminal)")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-pool-service")
                .long("debug-pool-service")
                .help("Enable pool service supervisor debug logs")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-pool-discovery")
                .long("debug-pool-discovery")
                .help("Enable pool discovery debug logs")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Forward args to bot's global args system for logging flags
    let args = std::env::args().collect::<Vec<String>>();
    set_cmd_args(args);

    let days: u32 = matches
        .get_one::<String>("days")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(7);

    let validate_only = matches.get_flag("validate-only");

    logger::info(
        LogTag::System,
        "🚀 Starting comprehensive price fetching tool for open positions",
    );
    println!("🚀 Debug Tool: Fetch Position Prices");
    println!("📅 Days to fetch: {}", days);
    println!("🔍 Validate only: {}", validate_only);
    println!();

    // Step 1: Initialize databases
    logger::info(LogTag::System, "📊 Initializing databases...");

    if let Err(e) = initialize_positions_database().await {
        logger::info(
            LogTag::Positions,
            &format!("Failed to initialize positions database: {}", e),
        );
        return Err(format!("Positions database initialization failed: {}", e).into());
    }
    println!("✅ Positions database initialized");

    // TODO: OHLCV service initialization handled by ServiceManager in new system
    println!("⚠️  OHLCV functionality temporarily disabled - needs rewrite for new system");

    // Step 2: Load open positions
    logger::info(LogTag::System, "📋 Loading open positions...");
    let open_positions = match get_db_open_positions().await {
        Ok(positions) => positions,
        Err(e) => {
            logger::info(
                LogTag::Positions,
                &format!("Failed to load open positions: {}", e),
            );
            return Err(format!("Failed to load open positions: {}", e).into());
        }
    };

    if open_positions.is_empty() {
        println!("ℹ️  No open positions found. Nothing to fetch.");
        return Ok(());
    }

    println!("📊 Found {} open positions:", open_positions.len());
    for (i, pos) in open_positions.iter().enumerate() {
        println!(
            "  {}. {} ({}) - Entry: {} SOL @ {}",
            i + 1,
            pos.symbol,
            pos.mint,
            pos.entry_price,
            pos.entry_time
        );
    }
    println!();

    // Step 3: Configure token monitoring
    let tokens_to_monitor: Vec<String> = open_positions.iter().map(|p| p.mint.clone()).collect();
    set_debug_token_override(Some(tokens_to_monitor.clone()));

    // Step 4: Initialize pool service
    logger::info(LogTag::System, "🏊 Starting pool service...");
    let shutdown_pools = Arc::new(Notify::new());
    if let Err(e) = init_pool_service(shutdown_pools.clone()).await {
        logger::info(
            LogTag::PoolService,
            &format!("Failed to start pool service: {}", e),
        );
        return Err(format!("Pool service start failed: {}", e).into());
    }
    println!("✅ Pool service started");

    // Give discovery time to fetch pools
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 5: OHLCV service is managed by the main runtime
    logger::info(
        LogTag::System,
        "📈 Skipping OHLCV service startup (handled by main runtime). Operating in read-only mode.",
    );
    println!("⚠️  OHLCV service startup is handled by the main bot. Continuing in read-only mode.");

    if !validate_only {
        // Step 7: Fetch OHLCV data for each position
        logger::info(LogTag::System, "📊 Fetching OHLCV data for positions...");
        println!("📊 Fetching OHLCV data for each position...");

        let candles_per_day = 1440; // 1-minute candles
        let total_candles = days * candles_per_day;

        for (i, position) in open_positions.iter().enumerate() {
            println!(
                "\n🔄 [{}/{}] Processing: {} ({})",
                i + 1,
                open_positions.len(),
                position.symbol,
                position.mint
            );

            // Use OHLCV data access API
            match get_ohlcv_data(
                &position.mint,
                Timeframe::Minute1,
                None,
                total_candles as usize,
                None,
                None,
            )
            .await
            {
                Ok(data) => {
                    println!("  ✅ Retrieved {} OHLCV data points", data.len());
                }
                Err(e) => {
                    println!("  ❌ Failed: {}", e);
                    logger::info(
                        LogTag::Ohlcv,
                        &format!("Failed to fetch OHLCV for {}: {}", position.mint, e),
                    );
                }
            }

            // Rate limiting - be nice to APIs
            if i < open_positions.len() - 1 {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }

    // Step 8: Validate price calculations
    logger::info(LogTag::System, "🔍 Validating price calculations...");
    println!("\n🔍 Validating price calculations...");

    for position in &open_positions {
        println!("\n📊 Validating: {} ({})", position.symbol, position.mint);

        match validate_position_prices(&position.mint, &position.entry_time).await {
            Ok(validation) => {
                println!(
                    "  💰 Entry price: {} SOL (position) vs {} SOL (OHLCV)",
                    position.entry_price, validation.ohlcv_price_at_entry
                );

                if let Some(current) = validation.current_price {
                    println!("  📈 Current price: {} SOL", current);
                    let change_pct =
                        ((current - position.entry_price) / position.entry_price) * 100.0;
                    println!("  📊 P&L: {:.2}%", change_pct);
                }

                println!("  ✅ Price data validation: {}", validation.status);
            }
            Err(e) => {
                println!("  ❌ Validation failed: {}", e);
            }
        }
    }

    // Step 9: Summary report
    println!("\n📋 Summary Report");
    println!("================");

    // Get database stats
    let conn = rusqlite::Connection::open("data/ohlcvs.db")
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let ohlcv_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ohlcv_data", [], |row| row.get(0))
        .unwrap_or(0);

    let sol_price_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sol_prices", [], |row| row.get(0))
        .unwrap_or(0);

    println!("📊 OHLCV data points stored: {}", ohlcv_count);
    println!("💰 SOL price points stored: {}", sol_price_count);
    println!("🎯 Positions analyzed: {}", open_positions.len());

    // Gracefully stop pool service
    logger::info(LogTag::System, "🛑 Shutting down services...");
    if let Err(e) = stop_pool_service(3).await {
        logger::info(
            LogTag::PoolService,
            &format!("Pool service stop warning: {}", e),
        );
    }
    println!("✅ Services stopped gracefully");

    logger::info(
        LogTag::System,
        "🎉 Price fetching and validation completed successfully",
    );
    println!("\n🎉 Price fetching and validation completed!");

    Ok(())
}

#[derive(Debug)]
struct PriceValidation {
    ohlcv_price_at_entry: f64,
    current_price: Option<f64>,
    status: String,
}

async fn validate_position_prices(
    mint: &str,
    entry_time: &DateTime<Utc>,
) -> Result<PriceValidation, String> {
    // Use raw SQL to get SOL prices directly
    let conn = rusqlite::Connection::open("data/ohlcvs.db")
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT timestamp, close FROM ohlcv_data WHERE mint = ? ORDER BY timestamp DESC LIMIT 2000"
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let rows = stmt
        .query_map(rusqlite::params![mint], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, f64>(1)?))
        })
        .map_err(|e| format!("Failed to query SOL prices: {}", e))?;

    let mut price_data: Vec<(i64, f64)> = Vec::new();
    for row_result in rows {
        match row_result {
            Ok((timestamp, price)) => price_data.push((timestamp, price)),
            Err(e) => logger::info(LogTag::Ohlcv, &format!("Failed to parse row: {}", e)),
        }
    }

    if price_data.is_empty() {
        return Err("No SOL OHLCV data available for validation".to_string());
    }

    // Find OHLCV point closest to entry time
    let entry_timestamp = entry_time.timestamp();
    let entry_point = price_data.iter().min_by_key(|(timestamp, _)| {
        let diff = *timestamp - entry_timestamp;
        diff.abs()
    });

    let ohlcv_price_at_entry = match entry_point {
        Some((_, price)) => *price,
        None => {
            return Err("No OHLCV data point found near entry time".to_string());
        }
    };

    // Get current price (most recent OHLCV point)
    let current_price = price_data.first().map(|(_, price)| *price);

    // Validate data quality
    let status = if price_data.len() >= 1000 {
        "Good - sufficient historical data"
    } else if price_data.len() >= 500 {
        "Fair - limited historical data"
    } else {
        "Poor - insufficient historical data"
    };

    Ok(PriceValidation {
        ohlcv_price_at_entry,
        current_price,
        status: status.to_string(),
    })
}
