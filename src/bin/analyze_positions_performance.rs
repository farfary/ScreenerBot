use clap::{ Arg, Command };
use chrono::{ DateTime, Duration as ChronoDuration, Utc };
use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{ log, LogTag };
use screenerbot::positions::{
    get_db_closed_positions,
    get_db_open_positions,
    initialize_positions_database,
    Position,
};
use screenerbot::tokens::{
    get_latest_ohlcv,
    get_ohlcv_service_clone,
    get_security_analyzer,
    init_ohlcv_service,
    OhlcvDataPoint,
};
use screenerbot::pools::{ init_pool_service, stop_pool_service, set_debug_token_override };
use std::sync::Arc;
use std::fs::{ self, OpenOptions };
use std::io::Write;
use std::path::{ Path, PathBuf };

// CSV export (lightweight, single dependency)
use csv::WriterBuilder;
use tokio::sync::Notify;

/// Analyze positions performance: open position failure patterns and missed profits on closed ones
/// This is a read-only diagnostic tool. It does not execute any trades.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Analyze Positions Performance")
        .about("Analyze open/closed positions for failure patterns and missed profit opportunities")
        .arg(
            Arg::new("closed-limit")
                .long("closed-limit")
                .value_name("N")
                .help("Number of most recent closed positions to analyze")
                .required(false)
                .default_value("20")
        )
        .arg(
            Arg::new("lookahead-mins")
                .long("lookahead-mins")
                .value_name("MIN")
                .help("Minutes after exit to search for post-exit peak for missed profit calc")
                .required(false)
                .default_value("180")
        )
        .arg(
            Arg::new("ohlcv-limit")
                .long("ohlcv-limit")
                .value_name("N")
                .help("1m OHLCV candles to fetch (recent history window)")
                .required(false)
                .default_value("2000")
        )
        .arg(
            Arg::new("export-csv-dir")
                .long("export-csv-dir")
                .value_name("PATH")
                .help("Directory to export CSV files (open/closed diagnostics)")
                .required(false)
        )
        .arg(
            Arg::new("csv-append")
                .long("csv-append")
                .help("Append to existing CSVs instead of overwriting")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("lookahead-all")
                .long("lookahead-all")
                .help("Also compute missed-profit for multiple windows [15,30,60,120,180,360]")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("force-fetch")
                .long("force-fetch")
                .help("Force fetch missing OHLCV data for better analysis")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("mint")
                .long("mint")
                .value_name("MINT")
                .help("Analyze a single token mint only")
                .required(false)
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Verbose per-position diagnostics")
                .action(clap::ArgAction::SetTrue)
        )
        // Pass-through debug flags so this binary can turn on module debug logs like others
        .arg(
            Arg::new("debug-ohlcv")
                .long("debug-ohlcv")
                .help("Enable detailed OHLCV system debug logs")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("debug-api")
                .long("debug-api")
                .help("Enable external API debug logs (DexScreener/GeckoTerminal/Raydium)")
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
                .help("Enable pool discovery debug logs (DexScreener/Gecko/Raydium)")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // forward args to bot's global args system for logging flags
    let args = std::env::args().collect::<Vec<String>>();
    set_cmd_args(args);

    let closed_limit: usize = matches
        .get_one::<String>("closed-limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(20);
    let lookahead_mins: i64 = matches
        .get_one::<String>("lookahead-mins")
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(180);
    let ohlcv_limit: u32 = matches
        .get_one::<String>("ohlcv-limit")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1000);
    let export_csv_dir: Option<PathBuf> = matches
        .get_one::<String>("export-csv-dir")
        .map(|s| PathBuf::from(s));
    let csv_append = matches.get_flag("csv-append");
    let lookahead_all = matches.get_flag("lookahead-all");
    let filter_mint = matches.get_one::<String>("mint").map(|s| s.to_string());
    let force_fetch = matches.get_flag("force-fetch");
    let verbose = matches.get_flag("verbose");

    log(LogTag::System, "INFO", "📊 Starting positions performance analysis");

    // Initialize positions database first (we need mints to configure pool service)
    if let Err(e) = initialize_positions_database().await {
        log(LogTag::Positions, "ERROR", &format!("Failed to initialize positions database: {}", e));
        return Err(format!("Positions database initialization failed: {}", e).into());
    }

    // Load positions from database (before starting pool service so we can focus discovery)
    let open_positions = match get_db_open_positions().await {
        Ok(mut v) => {
            if let Some(mint) = &filter_mint {
                v.retain(|p| &p.mint == mint);
            }
            v
        }
        Err(e) => {
            log(LogTag::Positions, "ERROR", &format!("Failed to load open positions: {}", e));
            Vec::new()
        }
    };

    // Load closed positions before starting pool service so we can target discovery
    let mut closed_positions = match get_db_closed_positions().await {
        Ok(mut v) => {
            if let Some(mint) = &filter_mint {
                v.retain(|p| &p.mint == mint);
            }
            // most recent first by exit_time
            v.sort_by(|a, b| b.exit_time.cmp(&a.exit_time));
            v.truncate(closed_limit);
            v
        }
        Err(e) => {
            log(LogTag::Positions, "ERROR", &format!("Failed to load closed positions: {}", e));
            Vec::new()
        }
    };

    // Configure pool discovery to focus on tokens we care about (open + closed + filter)
    let mut tokens_to_monitor: std::collections::HashSet<String> = std::collections::HashSet::new();
    for p in &open_positions {
        tokens_to_monitor.insert(p.mint.clone());
    }
    for p in &closed_positions {
        tokens_to_monitor.insert(p.mint.clone());
    }
    if let Some(m) = &filter_mint {
        tokens_to_monitor.insert(m.clone());
    }
    if !tokens_to_monitor.is_empty() {
        set_debug_token_override(Some(tokens_to_monitor.iter().cloned().collect()));
    }

    // Start the pool service (DexScreener discovery enabled) so OHLCV can resolve pool addresses
    let shutdown_pools = Arc::new(Notify::new());
    if let Err(e) = init_pool_service(shutdown_pools.clone()).await {
        log(LogTag::PoolService, "ERROR", &format!("Failed to start pool service: {}", e));
        return Err(format!("Pool service start failed: {}", e).into());
    }

    // Give discovery a brief moment to fetch pools and compute prices
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Initialize OHLCV service for missed profit analysis (uses pool service to resolve pool addresses)
    if let Err(e) = init_ohlcv_service().await {
        log(LogTag::Ohlcv, "WARN", &format!("OHLCV service initialization failed: {}", e));
        log(LogTag::Ohlcv, "WARN", "Missed profit analysis will be unavailable");
    }

    // Warm up OHLCV service to ensure DB initialized
    if let Err(e) = get_ohlcv_service_clone().await {
        log(LogTag::Ohlcv, "WARN", &format!("OHLCV service not available: {}", e));
    }
    // closed_positions already loaded above

    // Print context
    println!(
        "\n🔄 Open positions: {} | ✅ Closed positions (analyzed): {}",
        open_positions.len(),
        closed_positions.len()
    );

    // Analyze open positions: find failure patterns
    analyze_open_positions(
        &open_positions,
        ohlcv_limit,
        force_fetch,
        verbose,
        export_csv_dir.as_deref(),
        csv_append
    ).await;

    // Analyze closed positions: compute missed profit
    analyze_closed_positions(
        &mut closed_positions,
        lookahead_mins,
        ohlcv_limit,
        force_fetch,
        verbose,
        export_csv_dir.as_deref(),
        csv_append,
        lookahead_all
    ).await;

    // Gracefully stop pool service before exit to clean up background tasks
    let stop_res = stop_pool_service(3).await;
    if let Err(e) = stop_res {
        log(LogTag::PoolService, "WARN", &format!("Pool service stop warning: {}", e));
    }

    Ok(())
}

async fn analyze_open_positions(
    open_positions: &[Position],
    ohlcv_limit: u32,
    force_fetch: bool,
    verbose: bool,
    export_dir: Option<&Path>,
    csv_append: bool
) {
    if open_positions.is_empty() {
        println!("\n🟢 No open positions to analyze.");
        return;
    }

    println!("\n📉 Open Positions Failure Analysis:");
    println!("----------------------------------");

    let mut total = 0usize;
    let mut fail_count = 0usize;

    // Aggregates for similarity patterns
    let mut can_mint_count = 0usize;
    let mut can_freeze_count = 0usize;
    let mut lp_unlocked_count = 0usize;
    let mut low_holders_count = 0usize;

    // Buckets for early dump detection
    let mut early_dump_30m = 0usize;
    let mut early_dump_10m = 0usize;

    // Prepare CSV if requested
    let mut open_writer: Option<csv::Writer<Box<dyn std::io::Write>>> = None;
    if let Some(dir) = export_dir {
        if let Err(e) = fs::create_dir_all(dir) {
            log(LogTag::System, "WARN", &format!("Failed to create export dir {:?}: {}", dir, e));
        } else {
            let path = dir.join("open_positions_diagnostics.csv");
            match
                create_csv_writer_with_header(
                    &path,
                    csv_append,
                    &[
                        "mint",
                        "symbol",
                        "entry_time",
                        "current_time",
                        "entry_price_sol",
                        "current_price_sol",
                        "pnl_pct",
                        "age_min",
                        "mdd_10m_pct",
                        "mdd_30m_pct",
                        "mru_10m_pct",
                        "mru_30m_pct",
                        "trend_5m_pct_per_min",
                        "trend_15m_pct_per_min",
                        "trend_60m_pct_per_min",
                        "price_fresh_secs",
                        "mint_auth_disabled",
                        "freeze_auth_disabled",
                        "lp_is_safe",
                        "holder_count",
                        "early_dump_10m",
                        "early_dump_30m",
                        "v_recovery_candidate",
                    ]
                )
            {
                Ok(w) => {
                    open_writer = Some(w);
                }
                Err(e) =>
                    log(LogTag::System, "WARN", &format!("CSV open failed {:?}: {}", path, e)),
            }
        }
    }

    for p in open_positions {
        total += 1;
        let mint = &p.mint;

        // Get cached/basic security info if available
        let mut can_mint = false;
        let mut can_freeze = false;
        let mut lp_locked = true; // assume safe unless known otherwise
        let mut holder_count: u32 = 0;

        {
            let analyzer = get_security_analyzer();
            match analyzer.database.get_security_info(mint) {
                Ok(Some(info)) => {
                    can_mint = !info.mint_authority_disabled;
                    can_freeze = !info.freeze_authority_disabled;
                    lp_locked = info.lp_is_safe;
                    holder_count = info.holder_count;
                }
                _ => {}
            }
        }

        // Try OHLCV to evaluate early dump after entry and compute metrics
        let mut had_early_dump_30m = false;
        let mut had_early_dump_10m = false;
        let mut mdd_10m = 0.0f64;
        let mut mdd_30m = 0.0f64;
        let mut mru_10m = 0.0f64;
        let mut mru_30m = 0.0f64;
        let mut trend_5m: Option<f64> = None;
        let mut trend_15m: Option<f64> = None;
        let mut trend_60m: Option<f64> = None;
        let mut price_fresh_secs: Option<i64> = None;
        match get_latest_ohlcv(mint, ohlcv_limit).await {
            Ok(candles) if !candles.is_empty() => {
                if let Some(entry_candle) = nearest_at_or_before(&candles, p.entry_time) {
                    let t10 = p.entry_time + ChronoDuration::minutes(10);
                    let t30 = p.entry_time + ChronoDuration::minutes(30);
                    let dd10 = percent_drawdown_from(
                        &candles,
                        p.entry_time,
                        t10,
                        entry_candle.close
                    );
                    let dd30 = percent_drawdown_from(
                        &candles,
                        p.entry_time,
                        t30,
                        entry_candle.close
                    );
                    mdd_10m = dd10;
                    mdd_30m = dd30;
                    mru_10m = percent_runup_from(&candles, p.entry_time, t10, entry_candle.close);
                    mru_30m = percent_runup_from(&candles, p.entry_time, t30, entry_candle.close);
                    had_early_dump_10m = dd10 <= -30.0; // 30%+ drop in first 10m
                    had_early_dump_30m = dd30 <= -50.0; // 50%+ drop in first 30m

                    let now = Utc::now();
                    price_fresh_secs = candles.last().map(|c| now.timestamp() - c.timestamp);
                    trend_5m = slope_pct_per_min(
                        &candles,
                        p.entry_time,
                        p.entry_time + ChronoDuration::minutes(5)
                    );
                    trend_15m = slope_pct_per_min(
                        &candles,
                        p.entry_time,
                        p.entry_time + ChronoDuration::minutes(15)
                    );
                    trend_60m = slope_pct_per_min(
                        &candles,
                        p.entry_time,
                        p.entry_time + ChronoDuration::minutes(60)
                    );
                }
            }
            Err(_) if force_fetch => {
                // Force fetch: try to add token to watch list and fetch new data
                if let Ok(ohlcv_service) = get_ohlcv_service_clone().await {
                    log(
                        LogTag::Ohlcv,
                        "FORCE_FETCH",
                        &format!("🔄 Force fetching OHLCV for {}", mint)
                    );
                    ohlcv_service.add_to_watch_list(mint, true).await;

                    // Wait a moment for background fetch, then retry
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    if let Ok(candles) = get_latest_ohlcv(mint, ohlcv_limit).await {
                        if !candles.is_empty() {
                            if
                                let Some(entry_candle) = nearest_at_or_before(
                                    &candles,
                                    p.entry_time
                                )
                            {
                                let t10 = p.entry_time + ChronoDuration::minutes(10);
                                let t30 = p.entry_time + ChronoDuration::minutes(30);
                                let dd10 = percent_drawdown_from(
                                    &candles,
                                    p.entry_time,
                                    t10,
                                    entry_candle.close
                                );
                                let dd30 = percent_drawdown_from(
                                    &candles,
                                    p.entry_time,
                                    t30,
                                    entry_candle.close
                                );
                                mdd_10m = dd10;
                                mdd_30m = dd30;
                                mru_10m = percent_runup_from(
                                    &candles,
                                    p.entry_time,
                                    t10,
                                    entry_candle.close
                                );
                                mru_30m = percent_runup_from(
                                    &candles,
                                    p.entry_time,
                                    t30,
                                    entry_candle.close
                                );
                                had_early_dump_10m = dd10 <= -30.0;
                                had_early_dump_30m = dd30 <= -50.0;

                                let now = Utc::now();
                                price_fresh_secs = candles
                                    .last()
                                    .map(|c| now.timestamp() - c.timestamp);
                                trend_5m = slope_pct_per_min(
                                    &candles,
                                    p.entry_time,
                                    p.entry_time + ChronoDuration::minutes(5)
                                );
                                trend_15m = slope_pct_per_min(
                                    &candles,
                                    p.entry_time,
                                    p.entry_time + ChronoDuration::minutes(15)
                                );
                                trend_60m = slope_pct_per_min(
                                    &candles,
                                    p.entry_time,
                                    p.entry_time + ChronoDuration::minutes(60)
                                );
                            }
                        }
                    }
                }
            }
            _ => {} // No data available and not force fetching
        }

        // Heuristic: consider it a failed open if current_price is far below entry_price
        let mut is_failed = false;
        if let Some(cur) = p.current_price {
            let entry = p.entry_price;
            if entry > 0.0 {
                let pnl_pct = ((cur - entry) / entry) * 100.0;
                is_failed = pnl_pct <= -35.0;
            }
        } else {
            // Fall back: if early big dump detected, consider failed
            is_failed = had_early_dump_30m || had_early_dump_10m;
        }

        if is_failed {
            fail_count += 1;
            if can_mint {
                can_mint_count += 1;
            }
            if can_freeze {
                can_freeze_count += 1;
            }
            if !lp_locked {
                lp_unlocked_count += 1;
            }
            if holder_count < 50 {
                low_holders_count += 1;
            }
            if had_early_dump_30m {
                early_dump_30m += 1;
            }
            if had_early_dump_10m {
                early_dump_10m += 1;
            }
        }

        if verbose && is_failed {
            println!(
                "- ❌ {} | mint {} | entry {} | cur {:?} | can_mint={} can_freeze={} lp_locked={} holders={} early_dump10={} early_dump30={}",
                p.symbol,
                mint,
                p.entry_price,
                p.current_price,
                can_mint,
                can_freeze,
                lp_locked,
                holder_count,
                had_early_dump_10m,
                had_early_dump_30m
            );
        }

        // CSV row
        if let Some(w) = open_writer.as_mut() {
            let now = Utc::now();
            let age_min = (now - p.entry_time).num_minutes();
            let pnl_pct = p.current_price
                .map(|cur| ((cur - p.entry_price) / p.entry_price) * 100.0)
                .unwrap_or(0.0);
            let v_recovery = mdd_30m <= -30.0 && trend_60m.unwrap_or(0.0) > 0.0;
            let _ = w.write_record(
                &[
                    mint,
                    p.symbol.as_str(),
                    &p.entry_time.to_rfc3339(),
                    &now.to_rfc3339(),
                    &format!("{:.10}", p.entry_price),
                    &p.current_price
                        .map(|v| format!("{:.10}", v))
                        .unwrap_or_else(|| "".to_string()),
                    &format!("{:.2}", pnl_pct),
                    &age_min.to_string(),
                    &format!("{:.2}", mdd_10m),
                    &format!("{:.2}", mdd_30m),
                    &format!("{:.2}", mru_10m),
                    &format!("{:.2}", mru_30m),
                    &trend_5m.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    &trend_15m.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    &trend_60m.map(|v| format!("{:.4}", v)).unwrap_or_default(),
                    &price_fresh_secs.map(|s| s.to_string()).unwrap_or_default(),
                    &(!can_mint).to_string(),
                    &(!can_freeze).to_string(),
                    &lp_locked.to_string(),
                    &holder_count.to_string(),
                    &had_early_dump_10m.to_string(),
                    &had_early_dump_30m.to_string(),
                    &v_recovery.to_string(),
                ]
            );
        }
    }

    // Summary
    if total > 0 {
        println!("\nSummary (Open Failing Positions):");
        println!(
            "- Total open: {} | Estimated failing: {} ({:.1}%)",
            total,
            fail_count,
            ((fail_count as f64) / (total as f64)) * 100.0
        );
        if fail_count > 0 {
            println!(
                "- Security flags among failed: can_mint={} | can_freeze={} | lp_unlocked={} | low_holders(<50)={}",
                can_mint_count,
                can_freeze_count,
                lp_unlocked_count,
                low_holders_count
            );
            println!("- Early dumps: 10m {} | 30m {}", early_dump_10m, early_dump_30m);
        }
    }
}

async fn analyze_closed_positions(
    closed_positions: &mut [Position],
    lookahead_mins: i64,
    ohlcv_limit: u32,
    force_fetch: bool,
    verbose: bool,
    export_dir: Option<&Path>,
    csv_append: bool,
    lookahead_all: bool
) {
    if closed_positions.is_empty() {
        println!("\n📗 No recent closed positions to analyze.");
        return;
    }

    println!("\n📈 Recently Closed: Missed Profit Analysis (lookahead {}m)", lookahead_mins);
    println!("----------------------------------------------------------");

    let mut total = 0usize;
    let mut analyzed = 0usize;
    let mut sum_missed_pct = 0f64;
    let mut count_large_miss_50 = 0usize;

    // Prepare CSV if requested
    let mut closed_writer: Option<csv::Writer<Box<dyn std::io::Write>>> = None;
    let windows: Vec<i64> = if lookahead_all {
        vec![15, 30, 60, 120, 180, 360]
    } else {
        vec![lookahead_mins]
    };
    if let Some(dir) = export_dir {
        if let Err(e) = fs::create_dir_all(dir) {
            log(LogTag::System, "WARN", &format!("Failed to create export dir {:?}: {}", dir, e));
        } else {
            let path = dir.join("closed_positions_missed_profit.csv");
            // Build headers dynamically for windows
            let mut headers = vec![
                "mint",
                "symbol",
                "entry_time",
                "exit_time",
                "entry_price_sol",
                "exit_price_sol",
                "exit_pnl_pct",
                "time_in_position_min",
                "pre_exit_trend_15m_pct_per_min",
                "pre_exit_vol_15m_pct"
            ];
            for w in &windows {
                headers.push(Box::leak(format!("missed_peak_pct_{}m", w).into_boxed_str()));
                headers.push(Box::leak(format!("time_to_peak_min_{}m", w).into_boxed_str()));
            }
            headers.extend_from_slice(
                &["holder_count", "lp_is_safe", "mint_auth_disabled", "freeze_auth_disabled"]
            );

            match create_csv_writer_with_header(&path, csv_append, &headers) {
                Ok(w) => {
                    closed_writer = Some(w);
                }
                Err(e) =>
                    log(LogTag::System, "WARN", &format!("CSV open failed {:?}: {}", path, e)),
            }
        }
    }

    for p in closed_positions.iter() {
        total += 1;
        let mint = &p.mint;
        if p.exit_time.is_none() {
            continue;
        }
        let exit_time = p.exit_time.unwrap();

        // Retrieve OHLCV series and compute prices at exit and max high after exit (SOL-denominated)
        let candles = match get_latest_ohlcv(mint, ohlcv_limit).await {
            Ok(v) if !v.is_empty() => v,
            Err(e) if force_fetch => {
                // Force fetch: try to add token to watch list and fetch new data
                if verbose {
                    println!("- 🔄 {} | mint {} | Force fetching OHLCV: {}", p.symbol, mint, e);
                }
                if let Ok(ohlcv_service) = get_ohlcv_service_clone().await {
                    log(
                        LogTag::Ohlcv,
                        "FORCE_FETCH",
                        &format!("🔄 Force fetching OHLCV for {}", mint)
                    );
                    ohlcv_service.add_to_watch_list(mint, false).await;

                    // Wait for background fetch
                    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    match get_latest_ohlcv(mint, ohlcv_limit).await {
                        Ok(v) if !v.is_empty() => {
                            if verbose {
                                println!(
                                    "- ✅ {} | mint {} | Successfully fetched {} OHLCV points",
                                    p.symbol,
                                    mint,
                                    v.len()
                                );
                            }
                            v
                        }
                        _ => {
                            if verbose {
                                println!(
                                    "- ⚠️  {} | mint {} | OHLCV still unavailable after force fetch",
                                    p.symbol,
                                    mint
                                );
                            }
                            continue;
                        }
                    }
                } else {
                    if verbose {
                        println!("- ⚠️  {} | mint {} | OHLCV unavailable: {}", p.symbol, mint, e);
                    }
                    continue;
                }
            }
            Err(e) => {
                if verbose {
                    println!("- ⚠️  {} | mint {} | OHLCV unavailable: {}", p.symbol, mint, e);
                }
                continue;
            }
            Ok(_) => {
                if verbose {
                    println!("- ⚠️  {} | mint {} | No OHLCV data points found", p.symbol, mint);
                }
                continue;
            }
        };

        let exit_candle = match nearest_at_or_before(&candles, exit_time) {
            Some(c) => c,
            None => {
                if verbose {
                    let earliest_candle_time = candles
                        .first()
                        .map(|c|
                            DateTime::<Utc>
                                ::from_timestamp(c.timestamp, 0)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_else(|| "invalid".to_string())
                        )
                        .unwrap_or_else(|| "none".to_string());
                    println!(
                        "- ⚠️  {} | mint {} | No candle near exit time {} (earliest OHLCV: {})",
                        p.symbol,
                        mint,
                        exit_time.to_rfc3339(),
                        earliest_candle_time
                    );
                }
                continue;
            }
        };

        // Multi-window computation
        let mut row_dynamic: Vec<String> = Vec::new();
        let entry_price = p.entry_price;
        let exit_pnl_pct = if entry_price > 0.0 {
            ((exit_candle.close - entry_price) / entry_price) * 100.0
        } else {
            0.0
        };
        let time_in_pos = p.exit_time.map(|et| (et - p.entry_time).num_minutes()).unwrap_or(0);

        // Pre-exit metrics (15m)
        let pre_start = exit_time - ChronoDuration::minutes(15);
        let pre_trend = slope_pct_per_min(&candles, pre_start, exit_time).unwrap_or(0.0);
        let pre_vol = realized_volatility_pct(&candles, pre_start, exit_time).unwrap_or(0.0);

        // Aggregate summary uses primary lookahead (first in windows)
        let primary_window = *windows.first().unwrap_or(&lookahead_mins);
        let (max_high_primary, _) = max_high_after(
            &candles,
            exit_time,
            exit_time + ChronoDuration::minutes(primary_window)
        );
        if max_high_primary <= 0.0 {
            continue;
        }
        let missed_pct_primary =
            ((max_high_primary - exit_candle.close) / exit_candle.close) * 100.0;
        analyzed += 1;
        sum_missed_pct += missed_pct_primary;
        if missed_pct_primary >= 50.0 {
            count_large_miss_50 += 1;
        }

        if verbose {
            println!(
                "- {} | mint {} | exit_close={:.10} SOL | missed_peak(+{}m) +{:.2}%",
                p.symbol,
                mint,
                exit_candle.close,
                primary_window,
                missed_pct_primary
            );
        }

        // CSV row if requested
        if let Some(w) = closed_writer.as_mut() {
            // Security data for row
            let mut can_mint = false;
            let mut can_freeze = false;
            let mut lp_locked = true;
            let mut holder_count: u32 = 0;
            {
                let analyzer = get_security_analyzer();
                if let Ok(Some(info)) = analyzer.database.get_security_info(mint) {
                    can_mint = !info.mint_authority_disabled;
                    can_freeze = !info.freeze_authority_disabled;
                    lp_locked = info.lp_is_safe;
                    holder_count = info.holder_count;
                }
            }

            let mut record: Vec<String> = vec![
                mint.clone(),
                p.symbol.clone(),
                p.entry_time.to_rfc3339(),
                exit_time.to_rfc3339(),
                format!("{:.10}", entry_price),
                format!("{:.10}", exit_candle.close),
                format!("{:.2}", exit_pnl_pct),
                time_in_pos.to_string(),
                format!("{:.4}", pre_trend),
                format!("{:.4}", pre_vol)
            ];
            for wmin in &windows {
                let (mx, mxt) = max_high_after(
                    &candles,
                    exit_time,
                    exit_time + ChronoDuration::minutes(*wmin)
                );
                if mx > 0.0 {
                    let miss = ((mx - exit_candle.close) / exit_candle.close) * 100.0;
                    let ttp = mxt
                        .map(|t| (t - exit_time).num_minutes().to_string())
                        .unwrap_or_default();
                    record.push(format!("{:.2}", miss));
                    record.push(ttp);
                } else {
                    record.push(String::new());
                    record.push(String::new());
                }
            }
            record.push(holder_count.to_string());
            record.push(lp_locked.to_string());
            record.push((!can_mint).to_string());
            record.push((!can_freeze).to_string());

            let _ = w.write_record(&record);
        }
    }

    if analyzed > 0 {
        println!("\nSummary (Closed Missed Profit):");
        println!(
            "- Analyzed: {} of {} | Avg missed: +{:.2}% | Big misses (>=50%): {}",
            analyzed,
            total,
            sum_missed_pct / (analyzed as f64),
            count_large_miss_50
        );
    }

    // Optional: top offenders sorted by missed_pct (requires recomputation). Keep light for now.
}

fn nearest_at_or_before<'a>(
    candles: &'a [OhlcvDataPoint],
    ts: DateTime<Utc>
) -> Option<&'a OhlcvDataPoint> {
    let target = ts.timestamp();
    candles
        .iter()
        .filter(|c| c.timestamp <= target)
        .max_by_key(|c| c.timestamp)
}

fn percent_drawdown_from(
    candles: &[OhlcvDataPoint],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    ref_price: f64
) -> f64 {
    if ref_price <= 0.0 {
        return 0.0;
    }
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();
    let mut min_close = f64::MAX;
    for c in candles.iter() {
        if c.timestamp >= start_ts && c.timestamp <= end_ts {
            if c.close < min_close {
                min_close = c.close;
            }
        }
    }
    if min_close == f64::MAX {
        return 0.0;
    }
    ((min_close - ref_price) / ref_price) * 100.0
}

fn percent_runup_from(
    candles: &[OhlcvDataPoint],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    ref_price: f64
) -> f64 {
    if ref_price <= 0.0 {
        return 0.0;
    }
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();
    let mut max_close = 0.0f64;
    for c in candles.iter() {
        if c.timestamp >= start_ts && c.timestamp <= end_ts {
            if c.close > max_close {
                max_close = c.close;
            }
        }
    }
    if max_close == 0.0 {
        return 0.0;
    }
    ((max_close - ref_price) / ref_price) * 100.0
}

fn max_high_after(
    candles: &[OhlcvDataPoint],
    start: DateTime<Utc>,
    end: DateTime<Utc>
) -> (f64, Option<DateTime<Utc>>) {
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();
    let mut max_high = 0.0;
    let mut max_ts: Option<i64> = None;
    for c in candles.iter() {
        if c.timestamp >= start_ts && c.timestamp <= end_ts {
            if c.high > max_high {
                max_high = c.high;
                max_ts = Some(c.timestamp);
            }
        }
    }
    let max_dt = max_ts.and_then(|t| DateTime::<Utc>::from_timestamp(t, 0));
    (max_high, max_dt)
}

fn slope_pct_per_min(
    candles: &[OhlcvDataPoint],
    start: DateTime<Utc>,
    end: DateTime<Utc>
) -> Option<f64> {
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();
    let mut first: Option<&OhlcvDataPoint> = None;
    let mut last: Option<&OhlcvDataPoint> = None;
    for c in candles.iter() {
        if c.timestamp >= start_ts && c.timestamp <= end_ts {
            if first.is_none() {
                first = Some(c);
            }
            last = Some(c);
        }
    }
    match (first, last) {
        (Some(f), Some(l)) if f.close > 0.0 && l.timestamp > f.timestamp => {
            let mins = ((l.timestamp - f.timestamp) as f64) / 60.0;
            if mins <= 0.0 {
                return None;
            }
            let pct = ((l.close - f.close) / f.close) * 100.0;
            Some(pct / mins)
        }
        _ => None,
    }
}

fn realized_volatility_pct(
    candles: &[OhlcvDataPoint],
    start: DateTime<Utc>,
    end: DateTime<Utc>
) -> Option<f64> {
    let start_ts = start.timestamp();
    let end_ts = end.timestamp();
    let mut prev: Option<f64> = None;
    let mut rets: Vec<f64> = Vec::new();
    for c in candles.iter() {
        if c.timestamp >= start_ts && c.timestamp <= end_ts {
            if let Some(p) = prev {
                if p > 0.0 {
                    let r = (c.close / p - 1.0) * 100.0; // percent return
                    rets.push(r);
                }
            }
            prev = Some(c.close);
        }
    }
    if rets.len() < 2 {
        return None;
    }
    let mean: f64 = rets.iter().sum::<f64>() / (rets.len() as f64);
    let var: f64 =
        rets
            .iter()
            .map(|r| (r - mean).powi(2))
            .sum::<f64>() / ((rets.len() - 1) as f64);
    Some(var.sqrt())
}

fn create_csv_writer_with_header(
    path: &Path,
    append: bool,
    headers: &[&str]
) -> Result<csv::Writer<Box<dyn std::io::Write>>, String> {
    let file_exists = path.exists();
    let mut file = OpenOptions::new()
        .create(true)
        .append(append)
        .write(true)
        .truncate(!append)
        .open(path)
        .map_err(|e| format!("open csv: {}", e))?;
    let need_header = if append {
        match file.metadata() {
            Ok(m) => m.len() == 0,
            Err(_) => true,
        }
    } else {
        true
    };
    // csv writer
    let mut w = WriterBuilder::new()
        .has_headers(false)
        .from_writer(Box::new(file) as Box<dyn std::io::Write>);
    if need_header {
        w.write_record(headers).map_err(|e| format!("csv header: {}", e))?;
    }
    Ok(w)
}
