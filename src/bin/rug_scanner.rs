use anyhow::Result;
use clap::{ Parser, Subcommand };
use screenerbot::{ Config, MarketData };
use screenerbot::discovery::DiscoveryDatabase;
use screenerbot::rug_detection::{ RugDetectionEngine, RugDetectionMonitor, RugAction };
use screenerbot::marketdata::{ MarketDatabase, TokenBlacklist };
use std::sync::Arc;
use chrono::{ DateTime, Utc };

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(name = "rug_scanner")]
#[command(about = "ScreenerBot Rug Detection and Blacklist Management Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a specific token for rug indicators
    Scan {
        /// Token address to scan
        #[arg(value_name = "TOKEN_ADDRESS")]
        token_address: String,

        /// Current liquidity in USD (if known)
        #[arg(short, long)]
        liquidity: Option<f64>,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Scan all active tokens for rug indicators
    ScanAll {
        /// Maximum number of tokens to scan
        #[arg(short, long, default_value = "100")]
        limit: usize,

        /// Only scan tokens with liquidity above this threshold
        #[arg(short = 't', long, default_value = "1000.0")]
        threshold: f64,

        /// Auto-blacklist detected rugs
        #[arg(short, long)]
        auto_blacklist: bool,
    },

    /// Manage token blacklist
    Blacklist {
        #[command(subcommand)]
        action: BlacklistAction,
    },

    /// Analyze liquidity history for a token
    Liquidity {
        /// Token address to analyze
        #[arg(value_name = "TOKEN_ADDRESS")]
        token_address: String,

        /// Hours back to analyze
        #[arg(short, long, default_value = "24")]
        hours: i64,

        /// Show detailed history
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show rug detection statistics
    Stats {
        /// Show detailed breakdown
        #[arg(short, long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum BlacklistAction {
    /// List all blacklisted tokens
    List {
        /// Show only recent entries
        #[arg(short, long)]
        recent: bool,

        /// Maximum entries to show
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Add a token to blacklist
    Add {
        /// Token address to blacklist
        #[arg(value_name = "TOKEN_ADDRESS")]
        token_address: String,

        /// Reason for blacklisting
        #[arg(short, long)]
        reason: String,

        /// Peak liquidity before rug (if known)
        #[arg(short, long)]
        peak_liquidity: Option<f64>,

        /// Final liquidity after rug (if known)
        #[arg(short, long)]
        final_liquidity: Option<f64>,
    },

    /// Remove a token from blacklist
    Remove {
        /// Token address to remove
        #[arg(value_name = "TOKEN_ADDRESS")]
        token_address: String,
    },

    /// Check if a token is blacklisted
    Check {
        /// Token address to check
        #[arg(value_name = "TOKEN_ADDRESS")]
        token_address: String,
    },
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

    // Initialize rug detection engine
    let rug_engine = Arc::new(
        RugDetectionEngine::new(market_db.clone(), config.trader.rug_detection.clone())
    );

    match cli.command {
        Commands::Scan { token_address, liquidity, verbose } => {
            scan_token(&rug_engine, &market_db, &token_address, liquidity, verbose).await?;
        }

        Commands::ScanAll { limit, threshold, auto_blacklist } => {
            scan_all_tokens(&rug_engine, &market_db, limit, threshold, auto_blacklist).await?;
        }

        Commands::Blacklist { action } => {
            handle_blacklist_action(&market_db, action).await?;
        }

        Commands::Liquidity { token_address, hours, detailed } => {
            analyze_liquidity(&market_db, &token_address, hours, detailed).await?;
        }

        Commands::Stats { detailed } => {
            show_stats(&market_db, detailed).await?;
        }
    }

    Ok(())
}

async fn scan_token(
    engine: &RugDetectionEngine,
    db: &MarketDatabase,
    token_address: &str,
    liquidity: Option<f64>,
    verbose: bool
) -> Result<()> {
    println!("🔍 Scanning token: {}", token_address);

    let current_liquidity = if let Some(liq) = liquidity {
        liq
    } else {
        // Try to get liquidity from database
        if let Some(token_data) = db.get_token(token_address)? {
            token_data.liquidity_sol
        } else {
            println!("⚠️  No token data found in database. Please provide liquidity with -l flag.");
            return Ok(());
        }
    };

    println!("💧 Current liquidity: ${:.2}", current_liquidity);

    match engine.analyze_token(token_address, current_liquidity).await {
        Ok(result) => {
            match result.recommended_action {
                RugAction::Blacklist => {
                    println!("🚨 RUG DETECTED! Recommended action: BLACKLIST");
                    println!("   Confidence: {:.1}%", result.confidence * 100.0);
                    println!("   Reasons:");
                    for reason in &result.reasons {
                        println!("     - {}", reason);
                    }
                }
                RugAction::SellImmediately => {
                    println!("🚨 CRITICAL RUG! Recommended action: SELL IMMEDIATELY");
                    println!("   Confidence: {:.1}%", result.confidence * 100.0);
                    println!("   Reasons:");
                    for reason in &result.reasons {
                        println!("     - {}", reason);
                    }
                }
                RugAction::Monitor => {
                    println!("⚠️  SUSPICIOUS - Recommended action: MONITOR");
                    println!("   Confidence: {:.1}%", result.confidence * 100.0);
                    println!("   Reasons:");
                    for reason in &result.reasons {
                        println!("     - {}", reason);
                    }
                }
                RugAction::Continue => {
                    println!("✅ Token appears safe - Continue trading");
                    if verbose {
                        println!("   Confidence: {:.1}%", result.confidence * 100.0);
                        if !result.reasons.is_empty() {
                            println!("   Notes:");
                            for reason in &result.reasons {
                                println!("     - {}", reason);
                            }
                        }
                    }
                }
            }

            if verbose {
                // Show additional analysis details
                println!("\n📊 Additional Details:");

                // Check if blacklisted
                if let Ok(true) = db.is_blacklisted(token_address) {
                    println!("   🚫 Token is already blacklisted");
                }

                // Show liquidity history if available
                if let Ok(history) = db.get_liquidity_history(token_address, 24) {
                    if !history.is_empty() {
                        println!("   📈 Recent liquidity history ({} entries):", history.len());
                        for (i, entry) in history.iter().take(5).enumerate() {
                            println!(
                                "     {}. ${:.2} at {}",
                                i + 1,
                                entry.liquidity_sol,
                                entry.timestamp.format("%H:%M:%S")
                            );
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Scan failed: {}", e);
        }
    }

    Ok(())
}

async fn scan_all_tokens(
    engine: &RugDetectionEngine,
    db: &MarketDatabase,
    limit: usize,
    threshold: f64,
    auto_blacklist: bool
) -> Result<()> {
    println!("🔍 Scanning all active tokens...");
    println!("   Limit: {} tokens", limit);
    println!("   Liquidity threshold: ${:.2}", threshold);
    println!("   Auto-blacklist: {}", auto_blacklist);
    println!();

    let active_tokens = db.get_active_tokens()?;
    let mut scanned = 0;
    let mut rugs_detected = 0;
    let mut blacklisted = 0;

    for token_address in active_tokens.iter().take(limit) {
        if scanned >= limit {
            break;
        }

        // Get token data for liquidity check
        let token_data = match db.get_token(token_address)? {
            Some(data) => data,
            None => {
                continue;
            }
        };

        // Skip tokens below threshold
        if token_data.liquidity_sol < threshold {
            continue;
        }

        print!("Scanning {} (${:.0})... ", &token_address[..8], token_data.liquidity_sol);

        match engine.analyze_token(token_address, token_data.liquidity_sol).await {
            Ok(result) => {
                match result.recommended_action {
                    RugAction::Blacklist | RugAction::SellImmediately => {
                        println!("🚨 RUG DETECTED! ({}%)", (result.confidence * 100.0) as u8);
                        rugs_detected += 1;

                        if auto_blacklist && !db.is_blacklisted(token_address)? {
                            let blacklist_entry = TokenBlacklist {
                                token_address: token_address.clone(),
                                reason: format!("Auto-scan detected: {:?}", result.reasons),
                                blacklisted_at: Utc::now(),
                                peak_liquidity: None,
                                final_liquidity: Some(token_data.liquidity_sol),
                                drop_percentage: None,
                            };

                            db.add_to_blacklist(&blacklist_entry)?;
                            println!("   🚫 Auto-blacklisted");
                            blacklisted += 1;
                        }
                    }
                    RugAction::Monitor => {
                        println!("⚠️  Suspicious ({}%)", (result.confidence * 100.0) as u8);
                    }
                    RugAction::Continue => {
                        println!("✅ Safe");
                    }
                }
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }

        scanned += 1;

        // Small delay to avoid overwhelming the system
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!();
    println!("📊 Scan Results:");
    println!("   Tokens scanned: {}", scanned);
    println!("   Rugs detected: {}", rugs_detected);
    println!("   Auto-blacklisted: {}", blacklisted);

    Ok(())
}

async fn handle_blacklist_action(db: &MarketDatabase, action: BlacklistAction) -> Result<()> {
    match action {
        BlacklistAction::List { recent, limit } => {
            println!("📋 Blacklisted Tokens:");

            // This is a simplified version - in a real implementation,
            // you'd add methods to MarketDatabase to query blacklist entries
            println!("   (Implementation would show {} blacklisted tokens)", limit);
            if recent {
                println!("   (Filtered to recent entries)");
            }
        }

        BlacklistAction::Add { token_address, reason, peak_liquidity, final_liquidity } => {
            let drop_percentage = if
                let (Some(peak), Some(final_liq)) = (peak_liquidity, final_liquidity)
            {
                Some(((peak - final_liq) / peak) * 100.0)
            } else {
                None
            };

            let blacklist_entry = TokenBlacklist {
                token_address: token_address.clone(),
                reason,
                blacklisted_at: Utc::now(),
                peak_liquidity,
                final_liquidity,
                drop_percentage,
            };

            db.add_to_blacklist(&blacklist_entry)?;
            println!("🚫 Added {} to blacklist", token_address);
        }

        BlacklistAction::Remove { token_address } => {
            db.remove_from_blacklist(&token_address)?;
            println!("✅ Removed {} from blacklist", token_address);
        }

        BlacklistAction::Check { token_address } => {
            if db.is_blacklisted(&token_address)? {
                if let Some(entry) = db.get_blacklist_entry(&token_address)? {
                    println!("🚫 {} is BLACKLISTED", token_address);
                    println!("   Reason: {}", entry.reason);
                    println!(
                        "   Blacklisted at: {}",
                        entry.blacklisted_at.format("%Y-%m-%d %H:%M:%S UTC")
                    );
                    if let Some(peak) = entry.peak_liquidity {
                        println!("   Peak liquidity: ${:.2}", peak);
                    }
                    if let Some(final_liq) = entry.final_liquidity {
                        println!("   Final liquidity: ${:.2}", final_liq);
                    }
                    if let Some(drop) = entry.drop_percentage {
                        println!("   Liquidity drop: {:.1}%", drop);
                    }
                }
            } else {
                println!("✅ {} is NOT blacklisted", token_address);
            }
        }
    }

    Ok(())
}

async fn analyze_liquidity(
    db: &MarketDatabase,
    token_address: &str,
    hours: i64,
    detailed: bool
) -> Result<()> {
    println!("💧 Liquidity Analysis for: {}", token_address);
    println!("   Time range: {} hours back", hours);

    let history = db.get_liquidity_history(token_address, hours)?;

    if history.is_empty() {
        println!("   ❌ No liquidity history found");
        return Ok(());
    }

    // Calculate statistics
    let liquidities: Vec<f64> = history
        .iter()
        .map(|h| h.liquidity_sol)
        .collect();
    let current = liquidities.first().unwrap_or(&0.0);
    let max = liquidities.iter().fold(0.0f64, |a, &b| a.max(b));
    let min = liquidities.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let avg = liquidities.iter().sum::<f64>() / (liquidities.len() as f64);

    let max_drop = if max > 0.0 { ((max - current) / max) * 100.0 } else { 0.0 };

    println!();
    println!("📊 Statistics:");
    println!("   Current: ${:.2}", current);
    println!("   Peak: ${:.2}", max);
    println!("   Lowest: ${:.2}", min);
    println!("   Average: ${:.2}", avg);
    println!("   Max drop from peak: {:.1}%", max_drop);
    println!("   Data points: {}", history.len());

    if max_drop > 50.0 {
        println!("   🚨 WARNING: Significant liquidity drop detected!");
    } else if max_drop > 20.0 {
        println!("   ⚠️  Moderate liquidity decline");
    }

    if detailed && history.len() > 0 {
        println!();
        println!("📈 Detailed History (most recent first):");
        for (i, entry) in history.iter().take(20).enumerate() {
            println!(
                "   {}. ${:.2} at {} ({})",
                i + 1,
                entry.liquidity_sol,
                entry.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                entry.source
            );
        }

        if history.len() > 20 {
            println!("   ... and {} more entries", history.len() - 20);
        }
    }

    Ok(())
}

async fn show_stats(db: &MarketDatabase, detailed: bool) -> Result<()> {
    println!("📊 Rug Detection Statistics");

    // Get basic counts
    let active_tokens = db.get_active_tokens()?.len();

    println!();
    println!("🔢 Database Stats:");
    println!("   Active tokens: {}", active_tokens);

    // This would be enhanced with actual blacklist count methods
    println!("   Blacklisted tokens: (implementation needed)");
    println!("   Rug events recorded: (implementation needed)");

    if detailed {
        println!();
        println!("🏃‍♂️ System Performance:");
        println!("   Average scan time: (implementation needed)");
        println!("   Last full scan: (implementation needed)");
        println!("   Detection accuracy: (implementation needed)");
    }

    Ok(())
}
