use clap::{ Arg, Command };
use chrono::{ DateTime, Duration as ChronoDuration, Utc };
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{ log, LogTag };
use screenerbot::positions::{ get_db_open_positions, initialize_positions_database };
use screenerbot::tokens::{ init_ohlcv_service };
use screenerbot::pools::{ init_pool_service, stop_pool_service, set_debug_token_override };
use screenerbot::tokens::geckoterminal::{
    get_ohlcv_data_from_geckoterminal,
    get_token_pools_from_geckoterminal,
};
use screenerbot::tokens::ohlcv_db::{
    get_ohlcv_database,
    init_ohlcv_database,
    DbSolPriceDataPoint,
    DbOhlcvDataPoint,
};
use std::sync::Arc;
use tokio::sync::Notify;
use rusqlite;

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

    log(LogTag::System, "INFO", "🚀 Starting comprehensive price fetching tool for open positions");
    println!("🚀 Debug Tool: Fetch Position Prices");
    println!("📅 Days to fetch: {}", days);
    println!("🔍 Validate only: {}", validate_only);
    println!();

    // Step 1: Initialize databases
    log(LogTag::System, "INFO", "📊 Initializing databases...");

    if let Err(e) = initialize_positions_database().await {
        log(LogTag::Positions, "ERROR", &format!("Failed to initialize positions database: {}", e));
        return Err(format!("Positions database initialization failed: {}", e).into());
    }
    println!("✅ Positions database initialized");

    if let Err(e) = init_ohlcv_database() {
        log(LogTag::Ohlcv, "ERROR", &format!("Failed to initialize OHLCV database: {}", e));
        return Err(format!("OHLCV database initialization failed: {}", e).into());
    }
    println!("✅ OHLCV database initialized");

    // Step 2: Load open positions
    log(LogTag::System, "INFO", "📋 Loading open positions...");
    let open_positions = match get_db_open_positions().await {
        Ok(positions) => positions,
        Err(e) => {
            log(LogTag::Positions, "ERROR", &format!("Failed to load open positions: {}", e));
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
    let tokens_to_monitor: Vec<String> = open_positions
        .iter()
        .map(|p| p.mint.clone())
        .collect();
    set_debug_token_override(Some(tokens_to_monitor.clone()));

    // Step 4: Initialize pool service
    log(LogTag::System, "INFO", "🏊 Starting pool service...");
    let shutdown_pools = Arc::new(Notify::new());
    if let Err(e) = init_pool_service(shutdown_pools.clone()).await {
        log(LogTag::PoolService, "ERROR", &format!("Failed to start pool service: {}", e));
        return Err(format!("Pool service start failed: {}", e).into());
    }
    println!("✅ Pool service started");

    // Give discovery time to fetch pools
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 5: Initialize OHLCV service
    log(LogTag::System, "INFO", "📈 Starting OHLCV service...");
    if let Err(e) = init_ohlcv_service().await {
        log(LogTag::Ohlcv, "ERROR", &format!("OHLCV service initialization failed: {}", e));
        return Err(format!("OHLCV service failed: {}", e).into());
    }
    println!("✅ OHLCV service started");

    // Step 6: Ensure SOL price history is available
    log(LogTag::System, "INFO", "💰 Ensuring SOL price history...");
    let result = ensure_sol_price_history(days).await;
    match result {
        Ok(count) => {
            println!("✅ SOL price history available: {} data points", count);
        }
        Err(e) => {
            log(LogTag::Ohlcv, "ERROR", &format!("Failed to ensure SOL price history: {}", e));
            println!("❌ Failed to ensure SOL price history: {}", e);
        }
    }

    if !validate_only {
        // Step 7: Fetch OHLCV data for each position
        log(LogTag::System, "INFO", "📊 Fetching OHLCV data for positions...");
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

            match fetch_and_store_position_data(&position.mint, total_candles).await {
                Ok(count) => {
                    println!("  ✅ Stored {} OHLCV data points", count);
                }
                Err(e) => {
                    println!("  ❌ Failed: {}", e);
                    log(
                        LogTag::Ohlcv,
                        "ERROR",
                        &format!("Failed to fetch OHLCV for {}: {}", position.mint, e)
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
    log(LogTag::System, "INFO", "🔍 Validating price calculations...");
    println!("\n🔍 Validating price calculations...");

    for position in &open_positions {
        println!("\n📊 Validating: {} ({})", position.symbol, position.mint);

        match validate_position_prices(&position.mint, &position.entry_time).await {
            Ok(validation) => {
                println!(
                    "  💰 Entry price: {} SOL (position) vs {} SOL (OHLCV)",
                    position.entry_price,
                    validation.ohlcv_price_at_entry
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
    let conn = rusqlite::Connection
        ::open("data/ohlcvs.db")
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
    log(LogTag::System, "INFO", "🛑 Shutting down services...");
    if let Err(e) = stop_pool_service(3).await {
        log(LogTag::PoolService, "WARN", &format!("Pool service stop warning: {}", e));
    }
    println!("✅ Services stopped gracefully");

    log(LogTag::System, "INFO", "🎉 Price fetching and validation completed successfully");
    println!("\n🎉 Price fetching and validation completed!");

    Ok(())
}

#[derive(Debug)]
struct PriceValidation {
    ohlcv_price_at_entry: f64,
    current_price: Option<f64>,
    status: String,
}

async fn ensure_sol_price_history(days: u32) -> Result<usize, String> {
    // Calculate time range
    let now = Utc::now();
    let start_time = now - ChronoDuration::days(days as i64);

    // Use raw database connection to check SOL price coverage
    let conn = rusqlite::Connection
        ::open("data/ohlcvs.db")
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let existing_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sol_prices WHERE timestamp >= ? AND timestamp <= ?",
            rusqlite::params![start_time.timestamp(), now.timestamp()],
            |row| row.get(0)
        )
        .map_err(|e| format!("Failed to count existing SOL prices: {}", e))?;

    log(
        LogTag::Ohlcv,
        "INFO",
        &format!("Found {} existing SOL price points for {} days", existing_count, days)
    );

    // If we have good coverage (at least 80% of expected data points), return
    let expected_points = days * 24; // roughly hourly data points
    if (existing_count as u32) >= (expected_points * 80) / 100 {
        return Ok(existing_count as usize);
    }

    // Need to fetch more SOL price data - use the SOL major pool
    let sol_pool_address = "Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"; // Major SOL/USDC pool
    log(
        LogTag::Ohlcv,
        "INFO",
        &format!("Fetching SOL price history from pool: {}", sol_pool_address)
    );

    // Fetch SOL price OHLCV data
    let limit = days * 24 * 60; // 1-minute candles for the period
    match get_ohlcv_data_from_geckoterminal(sol_pool_address, limit).await {
        Ok(ohlcv_data) => {
            log(
                LogTag::Ohlcv,
                "INFO",
                &format!("Fetched {} SOL OHLCV data points", ohlcv_data.len())
            );

            // Convert OHLCV to SOL price points (close price as SOL/USD rate)
            let mut sol_prices = Vec::new();
            for point in ohlcv_data {
                sol_prices.push(DbSolPriceDataPoint {
                    timestamp: point.timestamp,
                    price_usd: point.close, // SOL/USD close price
                    source: "geckoterminal_sol_pool".to_string(),
                    created_at: now,
                });
            }

            // Store SOL prices using database connection
            let db = get_ohlcv_database().map_err(|e| format!("Database access failed: {}", e))?;
            if let Err(e) = db.store_sol_prices(&sol_prices) {
                return Err(format!("Failed to store SOL prices: {}", e));
            }

            log(LogTag::Ohlcv, "INFO", &format!("Stored {} SOL price points", sol_prices.len()));
            Ok(sol_prices.len())
        }
        Err(e) => { Err(format!("Failed to fetch SOL price history: {}", e)) }
    }
}

async fn fetch_and_store_position_data(mint: &str, limit: u32) -> Result<usize, String> {
    // Try to get pool address for token
    let pools = get_token_pools_from_geckoterminal(mint).await.map_err(|e|
        format!("Failed to get pools for token: {}", e)
    )?;

    if pools.is_empty() {
        return Err("No pools found for token".to_string());
    }

    // Use the first pool with highest liquidity
    let pool_address = &pools[0].pool_address;

    log(
        LogTag::Ohlcv,
        "INFO",
        &format!("Fetching OHLCV for mint {} using pool {}", mint, pool_address)
    );

    // Fetch OHLCV data from GeckoTerminal
    let ohlcv_data = get_ohlcv_data_from_geckoterminal(pool_address, limit).await.map_err(|e|
        format!("Failed to fetch OHLCV data: {}", e)
    )?;

    if ohlcv_data.is_empty() {
        return Err("No OHLCV data returned".to_string());
    }

    log(
        LogTag::Ohlcv,
        "INFO",
        &format!("Fetched {} OHLCV data points, converting to SOL denomination", ohlcv_data.len())
    );

    // Get database connection
    let db = get_ohlcv_database().map_err(|e| format!("Database access failed: {}", e))?;

    // Convert USD OHLCV to SOL OHLCV
    let mut sol_ohlcv_data = Vec::new();
    let mut conversion_failures = 0;

    for point in ohlcv_data {
        // Get SOL price at this timestamp (60 second tolerance)
        match db.get_sol_price_at_timestamp(point.timestamp, 60) {
            Ok(Some(sol_rate)) => {
                // Convert USD prices to SOL prices
                let sol_point = DbOhlcvDataPoint {
                    id: None,
                    mint: mint.to_string(),
                    pool_address: pool_address.to_string(),
                    timestamp: point.timestamp,
                    open_sol: point.open / sol_rate,
                    high_sol: point.high / sol_rate,
                    low_sol: point.low / sol_rate,
                    close_sol: point.close / sol_rate,
                    volume_sol: point.volume / sol_rate,
                    sol_usd_rate: sol_rate,
                    created_at: Utc::now(),
                };
                sol_ohlcv_data.push(sol_point);
            }
            Ok(None) => {
                conversion_failures += 1;
                log(
                    LogTag::Ohlcv,
                    "WARN",
                    &format!("No SOL price available for timestamp {}", point.timestamp)
                );
            }
            Err(e) => {
                conversion_failures += 1;
                log(
                    LogTag::Ohlcv,
                    "ERROR",
                    &format!("Failed to get SOL price for timestamp {}: {}", point.timestamp, e)
                );
            }
        }
    }

    if conversion_failures > 0 {
        log(
            LogTag::Ohlcv,
            "WARN",
            &format!("Failed to convert {} OHLCV points due to missing SOL prices", conversion_failures)
        );
    }

    if sol_ohlcv_data.is_empty() {
        return Err("No OHLCV data could be converted to SOL denomination".to_string());
    }

    // Store SOL OHLCV data
    db
        .store_sol_ohlcv_data(mint, pool_address, &sol_ohlcv_data)
        .map_err(|e| format!("Failed to store SOL OHLCV data: {}", e))?;

    log(
        LogTag::Ohlcv,
        "INFO",
        &format!(
            "Successfully stored {} SOL OHLCV data points for mint {}",
            sol_ohlcv_data.len(),
            mint
        )
    );

    Ok(sol_ohlcv_data.len())
}

async fn validate_position_prices(
    mint: &str,
    entry_time: &DateTime<Utc>
) -> Result<PriceValidation, String> {
    // Use raw SQL to get SOL prices directly
    let conn = rusqlite::Connection
        ::open("data/ohlcvs.db")
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let mut stmt = conn
        .prepare(
            "SELECT timestamp, close_sol FROM ohlcv_data WHERE mint = ? ORDER BY timestamp DESC LIMIT 2000"
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
            Err(e) => log(LogTag::Ohlcv, "WARN", &format!("Failed to parse row: {}", e)),
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
