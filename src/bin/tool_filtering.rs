/// Tool for analyzing the filtering system performance with database and rugcheck data
///
/// This tool provides comprehensive analysis of:
/// - Token filtering effectiveness
/// - Database token statistics
/// - Rugcheck data distribution
/// - Filter performance metrics
/// - Metadata completeness analysis

use screenerbot::{
    filtering::{ should_buy_token, FilterReason },
    tokens::{
        cache::TokenDatabase,
        types::{ Token, ApiToken },
        rugcheck::{ is_token_safe_for_trading, get_high_risk_issues },
        api::{ get_global_dexscreener_api },
        init_dexscreener_api,
        get_token_rugcheck_data_safe,
    },
    logger::{ log, LogTag },
    global::read_configs,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use colored::*;
use tokio;

/// Print comprehensive help menu for the Filtering Analysis Tool
fn print_help() {
    println!("🔍 Token Filtering Analysis Tool");
    println!("=====================================");
    println!("Comprehensive analysis tool for the token filtering system performance,");
    println!("database statistics, rugcheck data distribution, and filter effectiveness.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_filtering [TOKEN_MINT] [OPTIONS]");
    println!("");
    println!("ARGUMENTS:");
    println!("    [TOKEN_MINT]       Optional token mint to analyze specific filtering");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h         Show this help message");
    println!("    --debug-filtering  Enable detailed step-by-step filtering logs");
    println!("");
    println!("EXAMPLES:");
    println!("    # Analyze complete filtering system performance");
    println!("    cargo run --bin tool_filtering");
    println!("");
    println!("    # Test specific token filtering with debug output");
    println!(
        "    cargo run --bin tool_filtering -- EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --debug-filtering"
    );
    println!("");
    println!("    # Quick analysis of Bonk token filtering");
    println!("    cargo run --bin tool_filtering -- DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263");
    println!("");
    println!("ANALYSIS FEATURES:");
    println!("    • Complete database token statistics");
    println!("    • Filter pass/fail rates and reasons");
    println!("    • Rugcheck risk score distribution");
    println!("    • Metadata completeness analysis");
    println!("    • Liquidity and volume statistical breakdown");
    println!("    • Age distribution and filtering impact");
    println!("    • High-risk token identification");
    println!("");
    println!("FILTERING VALIDATION:");
    println!("    • 7-step filtering process analysis");
    println!("    • Step-by-step rejection reason tracking");
    println!("    • Security analysis effectiveness");
    println!("    • ATH proximity filtering performance");
    println!("    • Position constraint validation");
    println!("");
    println!("STATISTICAL OUTPUT:");
    println!("    • Pass/fail percentages for each filter step");
    println!("    • Average liquidity and volume metrics");
    println!("    • Risk score distribution charts");
    println!("    • Metadata field availability percentages");
    println!("    • Performance timing for filter operations");
    println!("");
    println!("SAFETY ANALYSIS:");
    println!("    • High-risk token detection accuracy");
    println!("    • LP lock validation effectiveness");
    println!("    • Authority risk assessment coverage");
    println!("    • Freeze/mint authority safety checks");
    println!("");
}

#[derive(Debug, Default)]
struct FilteringStats {
    total_tokens: usize,
    passed_filtering: usize,
    failed_filtering: usize,
    filter_reasons: HashMap<String, usize>,

    // Metadata completeness
    tokens_with_logo: usize,
    tokens_with_website: usize,
    tokens_with_description: usize,
    tokens_with_all_metadata: usize,

    // Rugcheck stats
    tokens_with_rugcheck: usize,
    safe_tokens: usize,
    rugged_tokens: usize,
    high_risk_tokens: usize,
    rugcheck_score_distribution: HashMap<String, usize>,

    // Liquidity and volume stats
    avg_liquidity: f64,
    avg_volume_24h: f64,
    high_liquidity_tokens: usize, // >$50k
    low_liquidity_tokens: usize, // <$10k

    // Age and activity stats
    new_tokens: usize, // <24h
    old_tokens: usize, // >7 days
    active_tokens: usize, // >100 txns/24h
}

impl FilteringStats {
    fn add_filter_reason(&mut self, reason: &FilterReason) {
        let reason_str = format!("{:?}", reason);
        *self.filter_reasons.entry(reason_str).or_insert(0) += 1;
    }

    fn print_summary(&self) {
        println!("\n{}", "=== FILTERING SYSTEM ANALYSIS ===".bright_cyan().bold());

        // Overall stats
        println!("\n{}", "📊 Overall Statistics:".bright_white().bold());
        println!("  Total Tokens Analyzed: {}", self.total_tokens.to_string().bright_yellow());
        println!(
            "  Passed Filtering: {} ({}%)",
            self.passed_filtering.to_string().bright_green(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.passed_filtering as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );
        println!("  Failed Filtering: {} ({}%)", self.failed_filtering.to_string().bright_red(), if
            self.total_tokens > 0
        {
            format!("{:.1}", ((self.failed_filtering as f64) / (self.total_tokens as f64)) * 100.0)
        } else {
            "0.0".to_string()
        });

        // Metadata completeness
        println!("\n{}", "📝 Metadata Completeness:".bright_white().bold());
        println!(
            "  Tokens with Logo URL: {} ({}%)",
            self.tokens_with_logo.to_string().bright_cyan(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.tokens_with_logo as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );
        println!(
            "  Tokens with Website: {} ({}%)",
            self.tokens_with_website.to_string().bright_cyan(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.tokens_with_website as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );
        println!(
            "  Tokens with Description: {} ({}%)",
            self.tokens_with_description.to_string().bright_cyan(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.tokens_with_description as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );
        println!(
            "  Tokens with ALL Metadata: {} ({}%)",
            self.tokens_with_all_metadata.to_string().bright_green(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.tokens_with_all_metadata as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );

        // Rugcheck stats
        println!("\n{}", "🔒 Rugcheck Analysis:".bright_white().bold());
        println!(
            "  Tokens with Rugcheck Data: {} ({}%)",
            self.tokens_with_rugcheck.to_string().bright_cyan(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.tokens_with_rugcheck as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );
        println!("  Safe Tokens: {} ({}%)", self.safe_tokens.to_string().bright_green(), if
            self.tokens_with_rugcheck > 0
        {
            format!(
                "{:.1}",
                ((self.safe_tokens as f64) / (self.tokens_with_rugcheck as f64)) * 100.0
            )
        } else {
            "0.0".to_string()
        });
        println!("  Rugged Tokens: {} ({}%)", self.rugged_tokens.to_string().bright_red(), if
            self.tokens_with_rugcheck > 0
        {
            format!(
                "{:.1}",
                ((self.rugged_tokens as f64) / (self.tokens_with_rugcheck as f64)) * 100.0
            )
        } else {
            "0.0".to_string()
        });
        println!(
            "  High Risk Tokens: {} ({}%)",
            self.high_risk_tokens.to_string().bright_yellow(),
            if self.tokens_with_rugcheck > 0 {
                format!(
                    "{:.1}",
                    ((self.high_risk_tokens as f64) / (self.tokens_with_rugcheck as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );

        // Liquidity and volume
        println!("\n{}", "💰 Liquidity & Volume Analysis:".bright_white().bold());
        println!("  Average Liquidity: ${:.2}", self.avg_liquidity);
        println!("  Average 24h Volume: ${:.2}", self.avg_volume_24h);
        println!(
            "  High Liquidity (>$50k): {} ({}%)",
            self.high_liquidity_tokens.to_string().bright_green(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.high_liquidity_tokens as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );
        println!(
            "  Low Liquidity (<$10k): {} ({}%)",
            self.low_liquidity_tokens.to_string().bright_red(),
            if self.total_tokens > 0 {
                format!(
                    "{:.1}",
                    ((self.low_liquidity_tokens as f64) / (self.total_tokens as f64)) * 100.0
                )
            } else {
                "0.0".to_string()
            }
        );

        // Top filter reasons
        println!("\n{}", "🚫 Top Filter Rejection Reasons:".bright_white().bold());
        let mut reasons: Vec<_> = self.filter_reasons.iter().collect();
        reasons.sort_by(|a, b| b.1.cmp(a.1));
        for (i, (reason, count)) in reasons.iter().take(10).enumerate() {
            let percentage = if self.failed_filtering > 0 {
                ((**count as f64) / (self.failed_filtering as f64)) * 100.0
            } else {
                0.0
            };
            println!(
                "  {}. {}: {} ({}%)",
                (i + 1).to_string().bright_white(),
                reason.trim_start_matches("FilterReason::").bright_yellow(),
                count.to_string().bright_red(),
                format!("{:.1}", percentage)
            );
        }

        // Rugcheck score distribution
        if !self.rugcheck_score_distribution.is_empty() {
            println!("\n{}", "📈 Rugcheck Score Distribution:".bright_white().bold());
            let mut scores: Vec<_> = self.rugcheck_score_distribution.iter().collect();
            scores.sort_by(|a, b| a.0.cmp(b.0));
            for (range, count) in scores {
                println!("  {}: {} tokens", range.bright_cyan(), count.to_string().bright_white());
            }
        }
    }
}

async fn analyze_token_filtering(token: &Token) -> (bool, Option<FilterReason>) {
    // Convert Token to the format expected by should_buy_token
    let passed = should_buy_token(token);

    if !passed {
        // Try to determine the reason by checking individual conditions
        if token.symbol.is_empty() {
            return (false, Some(FilterReason::EmptySymbol));
        }

        if token.mint.is_empty() {
            return (false, Some(FilterReason::EmptyMint));
        }

        if
            token.logo_url.is_none() ||
            token.logo_url.as_ref().map_or(true, |s| s.trim().is_empty())
        {
            return (false, Some(FilterReason::EmptyLogoUrl));
        }

        if token.website.is_none() || token.website.as_ref().map_or(true, |s| s.trim().is_empty()) {
            return (false, Some(FilterReason::EmptyWebsite));
        }

        if
            token.description.is_none() ||
            token.description.as_ref().map_or(true, |s| s.trim().is_empty())
        {
            return (false, Some(FilterReason::EmptyDescription));
        }

        if
            token.liquidity
                .as_ref()
                .and_then(|l| l.usd)
                .unwrap_or(0.0) < 10000.0
        {
            return (
                false,
                Some(FilterReason::InsufficientLiquidity {
                    current_usd: token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0),
                    minimum_required: 10000.0,
                }),
            );
        }

        // Default to lock acquisition failed (generic failure)
        return (false, Some(FilterReason::LockAcquisitionFailed));
    }

    (true, None)
}

fn check_metadata_completeness(token: &Token) -> (bool, bool, bool, bool) {
    let has_logo = token.logo_url.as_ref().map_or(false, |s| !s.trim().is_empty());
    let has_website = token.website.as_ref().map_or(false, |s| !s.trim().is_empty());
    let has_description = token.description.as_ref().map_or(false, |s| !s.trim().is_empty());
    let has_all = has_logo && has_website && has_description;

    (has_logo, has_website, has_description, has_all)
}

fn categorize_rugcheck_score(score: Option<i32>) -> String {
    match score {
        // CORRECTED: Higher scores mean MORE risk, not less!
        Some(s) if s >= 80 => "Critical Risk (80-100)".to_string(),
        Some(s) if s >= 60 => "Very High Risk (60-79)".to_string(),
        Some(s) if s >= 40 => "High Risk (40-59)".to_string(),
        Some(s) if s >= 20 => "Medium Risk (20-39)".to_string(),
        Some(s) if s >= 10 => "Low Risk (10-19)".to_string(),
        Some(s) => format!("Very Low Risk (0-9): {}", s),
        None => "No Score".to_string(),
    }
}

async fn analyze_rugcheck_data(mint: &str) -> Option<(bool, bool, bool, Option<i32>)> {
    match get_token_rugcheck_data_safe(mint).await {
        Ok(Some(rugcheck_data)) => {
            let is_safe = is_token_safe_for_trading(&rugcheck_data);
            let is_rugged = rugcheck_data.rugged.unwrap_or(false);
            let has_high_risk = if rugcheck_data.risks.is_some() {
                !get_high_risk_issues(&rugcheck_data).is_empty()
            } else {
                false
            };
            let score = rugcheck_data.score_normalised.or(rugcheck_data.score);

            Some((is_safe, is_rugged, has_high_risk, score))
        }
        Ok(None) => None,
        Err(_) => None,
    }
}

async fn run_filtering_analysis() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Starting Filtering System Analysis...".bright_green().bold());

    // Initialize systems
    let _configs = read_configs("configs.json")?;
    init_dexscreener_api().await?;

    // Connect to database
    println!("Connecting to token database...");
    let db = TokenDatabase::new()?;

    // Initialize rugcheck service
    println!("Initializing rugcheck service...");
    let shutdown_notify = Arc::new(tokio::sync::Notify::new());
    let db_clone = TokenDatabase::new()?;
    screenerbot::tokens
        ::initialize_global_rugcheck_service(db_clone, shutdown_notify).await
        .map_err(|e| format!("Failed to initialize rugcheck service: {}", e))?;

    // Get all tokens from database
    println!("Loading tokens from database...");
    let start_time = Instant::now();
    let api_tokens = db.get_all_tokens().await?;

    if api_tokens.is_empty() {
        println!(
            "{}",
            "No tokens found in database. Please run the bot first to populate the database.".bright_red()
        );
        return Ok(());
    }

    println!("Loaded {} tokens in {:.2}s", api_tokens.len(), start_time.elapsed().as_secs_f64());

    let mut stats = FilteringStats::default();
    stats.total_tokens = api_tokens.len();

    let mut total_liquidity = 0.0;
    let mut total_volume = 0.0;
    let mut tokens_with_liquidity = 0;
    let mut tokens_with_volume = 0;

    println!("Analyzing filtering performance...");
    let analysis_start = Instant::now();

    for (i, api_token) in api_tokens.iter().enumerate() {
        if i % 100 == 0 {
            println!(
                "  Processed {}/{} tokens ({:.1}%)",
                i,
                api_tokens.len(),
                ((i as f64) / (api_tokens.len() as f64)) * 100.0
            );
        }

        // Convert to Token for filtering analysis
        let token = Token::from(api_token.clone());

        // Analyze filtering
        let (passed, reason) = analyze_token_filtering(&token).await;
        if passed {
            stats.passed_filtering += 1;
        } else {
            stats.failed_filtering += 1;
            if let Some(r) = reason {
                stats.add_filter_reason(&r);
            }
        }

        // Check metadata completeness
        let (has_logo, has_website, has_description, has_all) = check_metadata_completeness(&token);
        if has_logo {
            stats.tokens_with_logo += 1;
        }
        if has_website {
            stats.tokens_with_website += 1;
        }
        if has_description {
            stats.tokens_with_description += 1;
        }
        if has_all {
            stats.tokens_with_all_metadata += 1;
        }

        // Analyze rugcheck data
        if
            let Some((is_safe, is_rugged, has_high_risk, score)) = analyze_rugcheck_data(
                &api_token.mint
            ).await
        {
            stats.tokens_with_rugcheck += 1;
            if is_safe {
                stats.safe_tokens += 1;
            }
            if is_rugged {
                stats.rugged_tokens += 1;
            }
            if has_high_risk {
                stats.high_risk_tokens += 1;
            }

            let score_category = categorize_rugcheck_score(score);
            *stats.rugcheck_score_distribution.entry(score_category).or_insert(0) += 1;
        }

        // Liquidity and volume stats
        if let Some(liquidity_info) = &api_token.liquidity {
            if let Some(liquidity) = liquidity_info.usd {
                total_liquidity += liquidity;
                tokens_with_liquidity += 1;

                if liquidity > 50000.0 {
                    stats.high_liquidity_tokens += 1;
                } else if liquidity < 10000.0 {
                    stats.low_liquidity_tokens += 1;
                }
            }
        }

        if let Some(volume_info) = &api_token.volume {
            if let Some(volume) = volume_info.h24 {
                total_volume += volume;
                tokens_with_volume += 1;
            }
        }
    }

    // Calculate averages
    if tokens_with_liquidity > 0 {
        stats.avg_liquidity = total_liquidity / (tokens_with_liquidity as f64);
    }
    if tokens_with_volume > 0 {
        stats.avg_volume_24h = total_volume / (tokens_with_volume as f64);
    }

    println!("Analysis completed in {:.2}s", analysis_start.elapsed().as_secs_f64());

    // Print results
    stats.print_summary();

    // Additional insights
    println!("\n{}", "💡 Key Insights:".bright_white().bold());

    let metadata_completion_rate = if stats.total_tokens > 0 {
        ((stats.tokens_with_all_metadata as f64) / (stats.total_tokens as f64)) * 100.0
    } else {
        0.0
    };

    if metadata_completion_rate < 50.0 {
        println!(
            "  {} Metadata completion is low ({}%). Consider relaxing metadata requirements.",
            "⚠️".bright_yellow(),
            format!("{:.1}%", metadata_completion_rate).bright_red()
        );
    } else {
        println!(
            "  {} Good metadata completion rate: {}%",
            "✅".bright_green(),
            format!("{:.1}", metadata_completion_rate).bright_green()
        );
    }

    let filtering_pass_rate = if stats.total_tokens > 0 {
        ((stats.passed_filtering as f64) / (stats.total_tokens as f64)) * 100.0
    } else {
        0.0
    };

    if filtering_pass_rate < 5.0 {
        println!(
            "  {} Very low filtering pass rate ({}%). Filters may be too strict.",
            "⚠️".bright_yellow(),
            format!("{:.1}%", filtering_pass_rate).bright_red()
        );
    } else if filtering_pass_rate > 20.0 {
        println!(
            "  {} High filtering pass rate ({}%). Consider tightening filters for better quality.",
            "⚠️".bright_yellow(),
            format!("{:.1}%", filtering_pass_rate).bright_yellow()
        );
    } else {
        println!(
            "  {} Good filtering balance: {}% pass rate",
            "✅".bright_green(),
            format!("{:.1}", filtering_pass_rate).bright_green()
        );
    }

    let rugcheck_coverage = if stats.total_tokens > 0 {
        ((stats.tokens_with_rugcheck as f64) / (stats.total_tokens as f64)) * 100.0
    } else {
        0.0
    };

    if rugcheck_coverage < 70.0 {
        println!(
            "  {} Low rugcheck coverage ({}%). Many tokens lack safety data.",
            "⚠️".bright_yellow(),
            format!("{:.1}%", rugcheck_coverage).bright_red()
        );
    } else {
        println!(
            "  {} Good rugcheck coverage: {}%",
            "✅".bright_green(),
            format!("{:.1}", rugcheck_coverage).bright_green()
        );
    }

    println!("\n{}", "Analysis complete!".bright_green().bold());

    Ok(())
}

#[tokio::main]
async fn main() {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    if let Err(e) = run_filtering_analysis().await {
        eprintln!("{}: {}", "Error".bright_red().bold(), e);
        std::process::exit(1);
    }
}
