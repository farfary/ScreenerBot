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
    analyze_open_positions(&open_positions, ohlcv_limit, force_fetch, verbose).await;

    // Analyze closed positions: compute missed profit
    analyze_closed_positions(
        &mut closed_positions,
        lookahead_mins,
        ohlcv_limit,
        force_fetch,
        verbose
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
    verbose: bool
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

        // Try OHLCV to evaluate early dump after entry
        let mut had_early_dump_30m = false;
        let mut had_early_dump_10m = false;
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
                    had_early_dump_10m = dd10 <= -30.0; // 30%+ drop in first 10m
                    had_early_dump_30m = dd30 <= -50.0; // 50%+ drop in first 30m
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
                                had_early_dump_10m = dd10 <= -30.0;
                                had_early_dump_30m = dd30 <= -50.0;
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
    verbose: bool
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
                    println!("- ⚠️  {} | mint {} | No candle near exit time", p.symbol, mint);
                }
                continue;
            }
        };

        let window_end = exit_time + ChronoDuration::minutes(lookahead_mins);
        let (max_high, max_high_time) = max_high_after(&candles, exit_time, window_end);
        if max_high <= 0.0 {
            continue;
        }

        let missed_pct = ((max_high - exit_candle.close) / exit_candle.close) * 100.0;
        analyzed += 1;
        sum_missed_pct += missed_pct;
        if missed_pct >= 50.0 {
            count_large_miss_50 += 1;
        }

        if verbose {
            let dt = max_high_time.map(|t| t.to_rfc3339()).unwrap_or_else(|| "n/a".to_string());
            println!(
                "- {} | mint {} | exit_close={:.10} SOL | post-peak={:.10} SOL (+{:.2}% at {})",
                p.symbol,
                mint,
                exit_candle.close,
                max_high,
                missed_pct,
                dt
            );
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
