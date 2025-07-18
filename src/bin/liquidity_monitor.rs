use anyhow::Result;
use clap::{ Parser, Subcommand };
use screenerbot::{ Config, MarketData };
use screenerbot::discovery::DiscoveryDatabase;
use screenerbot::marketdata::{ MarketDatabase, LiquidityHistory };
use screenerbot::rug_detection::{ RugDetectionEngine, RugDetectionConfig };
use std::sync::Arc;
use chrono::{ DateTime, Utc, Duration };
use serde_json;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(name = "liquidity_monitor")]
#[command(about = "ScreenerBot Liquidity Analysis and Monitoring Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Monitor liquidity changes for a specific token
    Watch {
        /// Token address to monitor
        #[arg(value_name = "TOKEN_ADDRESS")]
        token_address: String,

        /// Monitoring interval in seconds
        #[arg(short, long, default_value = "60")]
        interval: u64,

        /// Alert threshold for percentage drop
        #[arg(short, long, default_value = "20.0")]
        alert_threshold: f64,

        /// Duration to monitor in minutes (0 = indefinite)
        #[arg(short, long, default_value = "0")]
        duration: u64,
    },

    /// Analyze liquidity patterns across all tokens
    Analyze {
        /// Hours back to analyze
        #[arg(short = 'H', long, default_value = "24")]
        hours: i64,

        /// Minimum liquidity to consider
        #[arg(short, long, default_value = "1000.0")]
        min_liquidity: f64,

        /// Show top N tokens by liquidity drop
        #[arg(short, long, default_value = "10")]
        top: usize,

        /// Export results to JSON
        #[arg(short, long)]
        export: Option<String>,
    },

    /// Generate liquidity cliff detection report
    Report {
        /// Hours back to analyze
        #[arg(short = 'H', long, default_value = "24")]
        hours: i64,

        /// Minimum drop percentage to report
        #[arg(short, long, default_value = "50.0")]
        min_drop: f64,

        /// Minimum peak liquidity to consider
        #[arg(short, long, default_value = "10000.0")]
        min_peak: f64,

        /// Output format: text, json, csv
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Backfill liquidity history from current token data
    Backfill {
        /// Force backfill even if data exists
        #[arg(short, long)]
        force: bool,

        /// Limit number of tokens to process
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Show liquidity statistics
    Stats {
        /// Time window in hours
        #[arg(short = 'H', long, default_value = "24")]
        hours: i64,

        /// Show detailed breakdown
        #[arg(short, long)]
        detailed: bool,
    },
}

#[derive(Debug, serde::Serialize)]
struct LiquidityAnalysis {
    token_address: String,
    current_liquidity: f64,
    peak_liquidity: f64,
    drop_percentage: f64,
    drop_duration_hours: f64,
    risk_level: String,
    last_updated: DateTime<Utc>,
}

#[derive(Debug, serde::Serialize)]
struct CliffReport {
    detected_at: DateTime<Utc>,
    token_address: String,
    before_liquidity: f64,
    after_liquidity: f64,
    drop_percentage: f64,
    time_span_minutes: i64,
    severity: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    // Load configuration
    let config = Config::load("configs.json")?;

    // Initialize database connections
    let discovery_db = Arc::new(DiscoveryDatabase::new()?);
    let market_data = Arc::new(MarketData::new(discovery_db)?);
    let market_db = market_data.get_database();

    match cli.command {
        Commands::Watch { token_address, interval, alert_threshold, duration } => {
            watch_token(&market_db, &token_address, interval, alert_threshold, duration).await?;
        }

        Commands::Analyze { hours, min_liquidity, top, export } => {
            analyze_liquidity_patterns(&market_db, hours, min_liquidity, top, export).await?;
        }

        Commands::Report { hours, min_drop, min_peak, format } => {
            generate_cliff_report(&market_db, hours, min_drop, min_peak, &format).await?;
        }

        Commands::Backfill { force, limit } => {
            backfill_liquidity_history(&market_db, force, limit).await?;
        }

        Commands::Stats { hours, detailed } => {
            show_liquidity_stats(&market_db, hours, detailed).await?;
        }
    }

    Ok(())
}

async fn watch_token(
    db: &MarketDatabase,
    token_address: &str,
    interval: u64,
    alert_threshold: f64,
    duration: u64
) -> Result<()> {
    println!("👀 Monitoring liquidity for: {}", token_address);
    println!("   Interval: {}s", interval);
    println!("   Alert threshold: {:.1}%", alert_threshold);
    if duration > 0 {
        println!("   Duration: {}m", duration);
    } else {
        println!("   Duration: indefinite (Ctrl+C to stop)");
    }
    println!();

    let start_time = Utc::now();
    let end_time = if duration > 0 {
        Some(start_time + Duration::minutes(duration as i64))
    } else {
        None
    };

    let mut previous_liquidity: Option<f64> = None;
    let mut peak_liquidity = 0.0;

    loop {
        // Check if duration exceeded
        if let Some(end) = end_time {
            if Utc::now() > end {
                println!("⏰ Monitoring duration completed");
                break;
            }
        }

        // Get current token data
        match db.get_token(token_address)? {
            Some(token_data) => {
                let current_liquidity = token_data.liquidity_sol;
                let timestamp = Utc::now();

                // Update peak
                if current_liquidity > peak_liquidity {
                    peak_liquidity = current_liquidity;
                }

                // Calculate changes
                let change_from_peak: f64 = if peak_liquidity > 0.0 {
                    ((peak_liquidity - current_liquidity) / peak_liquidity) * 100.0
                } else {
                    0.0
                };

                let change_from_previous = if let Some(prev) = previous_liquidity {
                    if prev > 0.0 { ((prev - current_liquidity) / prev) * 100.0 } else { 0.0 }
                } else {
                    0.0
                };

                // Display current status
                print!("{} | ${:.2} ", timestamp.format("%H:%M:%S"), current_liquidity);

                if change_from_peak.abs() > 0.1 {
                    if change_from_peak > 0.0 {
                        print!("📉 -{:.1}% from peak ", change_from_peak);
                    } else {
                        print!("📈 +{:.1}% from peak ", change_from_peak.abs());
                    }
                }

                if previous_liquidity.is_some() && change_from_previous.abs() > 0.1 {
                    if change_from_previous > 0.0 {
                        print!("(#{:.1}%)", change_from_previous);
                    } else {
                        print!("(+{:.1}%)", change_from_previous.abs());
                    }
                }

                // Check for alerts
                if change_from_peak >= alert_threshold {
                    print!(" 🚨 ALERT: {:.1}% DROP!", change_from_peak);
                } else if change_from_peak >= alert_threshold / 2.0 {
                    print!(" ⚠️  Warning: {:.1}% drop", change_from_peak);
                }

                println!();

                // Record to history
                if
                    let Err(e) = db.record_liquidity_history(
                        token_address,
                        current_liquidity,
                        "monitor"
                    )
                {
                    eprintln!("Warning: Failed to record history: {}", e);
                }

                previous_liquidity = Some(current_liquidity);
            }
            None => {
                println!("{} | ❌ Token data not found", Utc::now().format("%H:%M:%S"));
            }
        }

        // Wait for next interval
        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }

    // Final summary
    println!();
    println!("📊 Monitoring Summary:");
    println!("   Peak liquidity: ${:.2}", peak_liquidity);
    if let Some(final_liquidity) = previous_liquidity {
        println!("   Final liquidity: ${:.2}", final_liquidity);
        let total_change = if peak_liquidity > 0.0 {
            ((peak_liquidity - final_liquidity) / peak_liquidity) * 100.0
        } else {
            0.0
        };
        println!("   Total change: {:.1}%", -total_change);
    }

    Ok(())
}

async fn analyze_liquidity_patterns(
    db: &MarketDatabase,
    hours: i64,
    min_liquidity: f64,
    top: usize,
    export: Option<String>
) -> Result<()> {
    println!("🔍 Analyzing liquidity patterns...");
    println!("   Time window: {} hours", hours);
    println!("   Minimum liquidity: ${:.2}", min_liquidity);
    println!();

    let active_tokens = db.get_active_tokens()?;
    let mut analyses = Vec::new();

    for token_address in active_tokens {
        // Get current token data
        let token_data = match db.get_token(&token_address)? {
            Some(data) => data,
            None => {
                continue;
            }
        };

        // Skip tokens below minimum liquidity
        if token_data.liquidity_sol < min_liquidity {
            continue;
        }

        // Get liquidity history
        let history = db.get_liquidity_history(&token_address, hours)?;
        if history.is_empty() {
            continue;
        }

        // Calculate peak and drop
        let liquidities: Vec<f64> = history
            .iter()
            .map(|h| h.liquidity_sol)
            .collect();
        let current = token_data.liquidity_sol;
        let peak = liquidities.iter().fold(current, |a, &b| a.max(b));

        let drop_percentage = if peak > 0.0 { ((peak - current) / peak) * 100.0 } else { 0.0 };

        // Determine risk level
        let risk_level = (
            if drop_percentage >= 80.0 {
                "CRITICAL"
            } else if drop_percentage >= 50.0 {
                "HIGH"
            } else if drop_percentage >= 20.0 {
                "MEDIUM"
            } else {
                "LOW"
            }
        ).to_string();

        // Calculate drop duration (simplified)
        let drop_duration_hours = if history.len() > 1 {
            let earliest = history.last().unwrap().timestamp;
            let latest = history.first().unwrap().timestamp;
            (latest - earliest).num_hours() as f64
        } else {
            0.0
        };

        analyses.push(LiquidityAnalysis {
            token_address,
            current_liquidity: current,
            peak_liquidity: peak,
            drop_percentage,
            drop_duration_hours,
            risk_level,
            last_updated: Utc::now(),
        });
    }

    // Sort by drop percentage (highest first)
    analyses.sort_by(|a, b| b.drop_percentage.partial_cmp(&a.drop_percentage).unwrap());

    // Display top results
    println!("🏆 Top {} Liquidity Drops:", top);
    println!("┌─────────────┬──────────────┬──────────────┬──────────┬──────────────┐");
    println!("│ Token       │ Current      │ Peak         │ Drop %   │ Risk Level   │");
    println!("├─────────────┼──────────────┼──────────────┼──────────┼──────────────┤");

    for analysis in analyses.iter().take(top) {
        println!(
            "│ {:<11} │ ${:<11.0} │ ${:<11.0} │ {:<8.1}% │ {:<12} │",
            &analysis.token_address[..11],
            analysis.current_liquidity,
            analysis.peak_liquidity,
            analysis.drop_percentage,
            analysis.risk_level
        );
    }
    println!("└─────────────┴──────────────┴──────────────┴──────────┴──────────────┘");

    // Export if requested
    if let Some(filename) = export {
        let json_data = serde_json::to_string_pretty(&analyses)?;
        std::fs::write(&filename, json_data)?;
        println!("📁 Results exported to: {}", filename);
    }

    // Summary statistics
    let critical_count = analyses
        .iter()
        .filter(|a| a.risk_level == "CRITICAL")
        .count();
    let high_count = analyses
        .iter()
        .filter(|a| a.risk_level == "HIGH")
        .count();
    let medium_count = analyses
        .iter()
        .filter(|a| a.risk_level == "MEDIUM")
        .count();

    println!();
    println!("📊 Risk Distribution:");
    println!("   🚨 Critical: {} tokens", critical_count);
    println!("   ⚠️  High: {} tokens", high_count);
    println!("   📊 Medium: {} tokens", medium_count);
    println!("   ✅ Low: {} tokens", analyses.len() - critical_count - high_count - medium_count);

    Ok(())
}

async fn generate_cliff_report(
    db: &MarketDatabase,
    hours: i64,
    min_drop: f64,
    min_peak: f64,
    format: &str
) -> Result<()> {
    println!("📋 Generating liquidity cliff report...");
    println!("   Time window: {} hours", hours);
    println!("   Minimum drop: {:.1}%", min_drop);
    println!("   Minimum peak: ${:.2}", min_peak);
    println!();

    let active_tokens = db.get_active_tokens()?;
    let mut cliffs = Vec::new();

    for token_address in active_tokens {
        let history = db.get_liquidity_history(&token_address, hours)?;
        if history.len() < 2 {
            continue;
        }

        // Look for significant drops between consecutive entries
        for window in history.windows(2) {
            let before = &window[1]; // Earlier timestamp (history is sorted desc)
            let after = &window[0]; // Later timestamp

            if before.liquidity_sol < min_peak {
                continue;
            }

            let drop_percentage = if before.liquidity_sol > 0.0 {
                ((before.liquidity_sol - after.liquidity_sol) / before.liquidity_sol) * 100.0
            } else {
                0.0
            };

            if drop_percentage >= min_drop {
                let time_span = (after.timestamp - before.timestamp).num_minutes();

                let severity = (
                    if drop_percentage >= 90.0 {
                        "EXTREME"
                    } else if drop_percentage >= 80.0 {
                        "SEVERE"
                    } else if drop_percentage >= 70.0 {
                        "HIGH"
                    } else {
                        "MODERATE"
                    }
                ).to_string();

                cliffs.push(CliffReport {
                    detected_at: after.timestamp,
                    token_address: token_address.clone(),
                    before_liquidity: before.liquidity_sol,
                    after_liquidity: after.liquidity_sol,
                    drop_percentage,
                    time_span_minutes: time_span,
                    severity,
                });
            }
        }
    }

    // Sort by severity and drop percentage
    cliffs.sort_by(|a, b| { b.drop_percentage.partial_cmp(&a.drop_percentage).unwrap() });

    // Output in requested format
    match format.to_lowercase().as_str() {
        "json" => {
            let json_data = serde_json::to_string_pretty(&cliffs)?;
            println!("{}", json_data);
        }
        "csv" => {
            println!(
                "timestamp,token_address,before_liquidity,after_liquidity,drop_percentage,time_span_minutes,severity"
            );
            for cliff in cliffs {
                println!(
                    "{},{},{:.2},{:.2},{:.1},{},{}",
                    cliff.detected_at.to_rfc3339(),
                    cliff.token_address,
                    cliff.before_liquidity,
                    cliff.after_liquidity,
                    cliff.drop_percentage,
                    cliff.time_span_minutes,
                    cliff.severity
                );
            }
        }
        _ => {
            println!("🪨 Liquidity Cliff Detection Report");
            println!("Found {} significant liquidity drops:", cliffs.len());
            println!();

            for (i, cliff) in cliffs.iter().enumerate() {
                println!("{}. {} [{}]", i + 1, cliff.token_address, cliff.severity);
                println!("   Time: {}", cliff.detected_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!(
                    "   Drop: ${:.2} → ${:.2} ({:.1}% in {}m)",
                    cliff.before_liquidity,
                    cliff.after_liquidity,
                    cliff.drop_percentage,
                    cliff.time_span_minutes
                );
                println!();
            }

            if cliffs.is_empty() {
                println!(
                    "✅ No significant liquidity cliffs detected in the specified time window."
                );
            }
        }
    }

    Ok(())
}

async fn backfill_liquidity_history(
    db: &MarketDatabase,
    force: bool,
    limit: Option<usize>
) -> Result<()> {
    println!("🔄 Backfilling liquidity history...");

    let active_tokens = db.get_active_tokens()?;
    let tokens_to_process = if let Some(lim) = limit {
        active_tokens.into_iter().take(lim).collect()
    } else {
        active_tokens
    };

    let mut processed = 0;
    let mut backfilled = 0;

    for token_address in tokens_to_process {
        // Check if history already exists
        if !force {
            let existing_history = db.get_liquidity_history(&token_address, 24)?;
            if !existing_history.is_empty() {
                processed += 1;
                continue;
            }
        }

        // Get current token data
        if let Some(token_data) = db.get_token(&token_address)? {
            // Record current liquidity as historical entry
            db.record_liquidity_history(&token_address, token_data.liquidity_sol, "backfill")?;

            backfilled += 1;
            print!(".");

            if backfilled % 50 == 0 {
                println!(" {} tokens backfilled", backfilled);
            }
        }

        processed += 1;

        // Small delay to avoid overwhelming the database
        if processed % 100 == 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    println!();
    println!("✅ Backfill complete:");
    println!("   Tokens processed: {}", processed);
    println!("   History entries added: {}", backfilled);

    Ok(())
}

async fn show_liquidity_stats(db: &MarketDatabase, hours: i64, detailed: bool) -> Result<()> {
    println!("📊 Liquidity Statistics ({}h window)", hours);

    let active_tokens = db.get_active_tokens()?;
    let mut total_current_liquidity = 0.0;
    let mut tokens_with_history = 0;
    let mut total_history_entries = 0;

    for token_address in &active_tokens {
        if let Some(token_data) = db.get_token(token_address)? {
            total_current_liquidity += token_data.liquidity_sol;
        }

        let history = db.get_liquidity_history(token_address, hours)?;
        if !history.is_empty() {
            tokens_with_history += 1;
            total_history_entries += history.len();
        }
    }

    println!();
    println!("🔢 Overview:");
    println!("   Active tokens: {}", active_tokens.len());
    println!("   Total current liquidity: ${:.2}", total_current_liquidity);
    println!("   Tokens with history: {}", tokens_with_history);
    println!("   Total history entries: {}", total_history_entries);

    if tokens_with_history > 0 {
        let avg_entries_per_token = (total_history_entries as f64) / (tokens_with_history as f64);
        println!("   Average entries per token: {:.1}", avg_entries_per_token);
    }

    if detailed {
        println!();
        println!("📈 Detailed Breakdown:");

        // Liquidity ranges
        let mut ranges = [0; 6]; // <1K, 1K-10K, 10K-100K, 100K-1M, 1M-10M, >10M

        for token_address in &active_tokens {
            if let Some(token_data) = db.get_token(token_address)? {
                let liquidity = token_data.liquidity_sol;
                let range_index = if liquidity < 1_000.0 {
                    0
                } else if liquidity < 10_000.0 {
                    1
                } else if liquidity < 100_000.0 {
                    2
                } else if liquidity < 1_000_000.0 {
                    3
                } else if liquidity < 10_000_000.0 {
                    4
                } else {
                    5
                };
                ranges[range_index] += 1;
            }
        }

        let range_labels = [
            "< $1K",
            "$1K - $10K",
            "$10K - $100K",
            "$100K - $1M",
            "$1M - $10M",
            "> $10M",
        ];

        for (i, &count) in ranges.iter().enumerate() {
            if count > 0 {
                println!("   {}: {} tokens", range_labels[i], count);
            }
        }
    }

    Ok(())
}
