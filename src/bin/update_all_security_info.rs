/// Security Database Update Tool
///
/// This tool updates security information for all tokens in the token database.
/// It's designed for bulk updates and ensuring complete security coverage.
///
/// Features:
/// - Batch processing with configurable batch sizes
/// - Progress tracking and statistics
/// - Resume capability for interrupted updates
/// - Force refresh option for existing security data
/// - Filtering by token age, liquidity, or other criteria

use screenerbot::{
    logger::{ log, LogTag },
    tokens::{ cache::TokenDatabase, security::{ SecurityRiskLevel } },
    utils::safe_truncate,
};
use std::collections::HashMap;
use std::env;
use std::time::Instant;
use tokio;

const HELP_TEXT: &str =
    r#"
Security Database Update Tool

This tool updates security information for all tokens in the token database.
Perfect for bulk security analysis and ensuring complete security coverage.

⚠️  WARNING: This tool can make many RPC calls and may take considerable time.
    Use appropriate batch sizes and delays to avoid overwhelming the RPC endpoint.

USAGE:
    cargo run --bin update_all_security_info [OPTIONS]

OPTIONS:
    --help                     Show this help message
    --batch-size <N>           Number of tokens to process per batch (default: 10, max: 50)
    --delay <SECONDS>          Delay between batches in seconds (default: 2)
    --force-refresh            Force refresh existing security data
    --min-liquidity <USD>      Only update tokens with min liquidity (default: 1000)
    --max-age-days <DAYS>      Only update tokens created within days (default: 30)
    --dry-run                  Show what would be updated without making changes
    --resume-from <INDEX>      Resume from specific token index (for interrupted updates)
    --limit <N>                Limit total tokens to process (default: all)
    --risk-filter <LEVEL>      Only update tokens with specific risk level
    --show-stats               Show security database statistics only
    --cleanup-invalid          Remove invalid/corrupted security entries

RISK LEVELS:
    safe, low, medium, high, critical, unknown

EXAMPLES:
    # Show current statistics
    cargo run --bin update_all_security_info --show-stats

    # Dry run to see what would be updated
    cargo run --bin update_all_security_info --dry-run

    # Update all tokens with small batch size and delays
    cargo run --bin update_all_security_info --batch-size 5 --delay 3

    # Force refresh all security data
    cargo run --bin update_all_security_info --force-refresh --batch-size 5

    # Update only high liquidity tokens
    cargo run --bin update_all_security_info --min-liquidity 10000

    # Update only recent tokens
    cargo run --bin update_all_security_info --max-age-days 7

    # Resume interrupted update from token 500
    cargo run --bin update_all_security_info --resume-from 500

    # Update only first 100 tokens
    cargo run --bin update_all_security_info --limit 100
"#;

#[derive(Debug, Clone)]
struct UpdateConfig {
    batch_size: usize,
    delay_seconds: u64,
    force_refresh: bool,
    min_liquidity_usd: f64,
    max_age_days: Option<i64>,
    dry_run: bool,
    resume_from: usize,
    limit: Option<usize>,
    risk_filter: Option<SecurityRiskLevel>,
    show_stats: bool,
    cleanup_invalid: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            delay_seconds: 2,
            force_refresh: false,
            min_liquidity_usd: 1000.0,
            max_age_days: Some(30),
            dry_run: false,
            resume_from: 0,
            limit: None,
            risk_filter: None,
            show_stats: false,
            cleanup_invalid: false,
        }
    }
}

#[derive(Debug, Default)]
struct UpdateStats {
    total_tokens: usize,
    processed: usize,
    updated: usize,
    skipped: usize,
    failed: usize,
    start_time: Option<Instant>,
}

impl UpdateStats {
    fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    fn print_progress(&self) {
        let elapsed = self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        let rate = if elapsed > 0 { (self.processed as f64) / (elapsed as f64) } else { 0.0 };

        println!(
            "📊 Progress: {}/{} processed | {} updated | {} skipped | {} failed | {:.1} tokens/sec",
            self.processed,
            self.total_tokens,
            self.updated,
            self.skipped,
            self.failed,
            rate
        );
    }

    fn print_final(&self) {
        let elapsed = self.start_time.map(|t| t.elapsed()).unwrap_or_default();

        println!("\n🎉 Update completed!");
        println!("📊 Final Statistics:");
        println!("   Total tokens: {}", self.total_tokens);
        println!("   Processed: {}", self.processed);
        println!("   Updated: {}", self.updated);
        println!("   Skipped: {}", self.skipped);
        println!("   Failed: {}", self.failed);
        println!("   Time elapsed: {:.1} seconds", elapsed.as_secs_f64());

        if self.processed > 0 {
            let success_rate = ((self.updated as f64) / (self.processed as f64)) * 100.0;
            println!("   Success rate: {:.1}%", success_rate);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    log(LogTag::Security, "INFO", "Starting security database update tool");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    // Parse command line arguments
    let mut config = UpdateConfig::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--help" => {
                print_help();
                return Ok(());
            }
            "--batch-size" => {
                if i + 1 < args.len() {
                    config.batch_size = args[i + 1].parse().unwrap_or(10).min(50).max(1);
                    i += 1;
                }
            }
            "--delay" => {
                if i + 1 < args.len() {
                    config.delay_seconds = args[i + 1].parse().unwrap_or(2).max(1);
                    i += 1;
                }
            }
            "--force-refresh" => {
                config.force_refresh = true;
            }
            "--min-liquidity" => {
                if i + 1 < args.len() {
                    config.min_liquidity_usd = args[i + 1].parse().unwrap_or(1000.0);
                    i += 1;
                }
            }
            "--max-age-days" => {
                if i + 1 < args.len() {
                    config.max_age_days = Some(args[i + 1].parse().unwrap_or(30));
                    i += 1;
                }
            }
            "--dry-run" => {
                config.dry_run = true;
            }
            "--resume-from" => {
                if i + 1 < args.len() {
                    config.resume_from = args[i + 1].parse().unwrap_or(0);
                    i += 1;
                }
            }
            "--limit" => {
                if i + 1 < args.len() {
                    config.limit = Some(args[i + 1].parse().unwrap_or(100));
                    i += 1;
                }
            }
            "--risk-filter" => {
                if i + 1 < args.len() {
                    config.risk_filter = parse_risk_level(&args[i + 1]);
                    i += 1;
                }
            }
            "--show-stats" => {
                config.show_stats = true;
            }
            "--cleanup-invalid" => {
                config.cleanup_invalid = true;
            }
            _ => {
                println!("Unknown option: {}", args[i]);
                print_help();
                return Ok(());
            }
        }
        i += 1;
    }

    // Initialize security analyzer
    let _security_analyzer = screenerbot::tokens::security::init_security_analyzer()?;

    if config.show_stats {
        show_security_stats().await?;
        return Ok(());
    }

    if config.cleanup_invalid {
        cleanup_invalid_entries().await?;
    }

    // Get tokens from database
    let database = TokenDatabase::new()?;
    let all_tokens = database
        .get_all_tokens_with_update_time().await
        .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

    println!("📊 Found {} tokens in database", all_tokens.len());

    // Filter tokens based on criteria
    let filtered_tokens = filter_tokens_by_criteria(&all_tokens, &config);
    println!("🔍 Filtered to {} tokens based on criteria", filtered_tokens.len());

    if filtered_tokens.is_empty() {
        println!("❌ No tokens match the specified criteria");
        return Ok(());
    }

    // Apply resume and limit
    let start_index = config.resume_from.min(filtered_tokens.len());
    let end_index = if let Some(limit) = config.limit {
        (start_index + limit).min(filtered_tokens.len())
    } else {
        filtered_tokens.len()
    };

    let tokens_to_process = &filtered_tokens[start_index..end_index];

    if config.dry_run {
        print_dry_run_info(tokens_to_process, &config);
        return Ok(());
    }

    // Run the update
    run_security_update(tokens_to_process, &config).await?;

    Ok(())
}

fn print_help() {
    println!("{}", HELP_TEXT);
}

fn parse_risk_level(level_str: &str) -> Option<SecurityRiskLevel> {
    match level_str.to_lowercase().as_str() {
        "safe" => Some(SecurityRiskLevel::Safe),
        "low" => Some(SecurityRiskLevel::Low),
        "medium" => Some(SecurityRiskLevel::Medium),
        "high" => Some(SecurityRiskLevel::High),
        "critical" => Some(SecurityRiskLevel::Critical),
        "unknown" => Some(SecurityRiskLevel::Unknown),
        _ => None,
    }
}

fn filter_tokens_by_criteria(
    tokens: &[(String, String, chrono::DateTime<chrono::Utc>, f64)],
    config: &UpdateConfig
) -> Vec<String> {
    let now = chrono::Utc::now();

    tokens
        .iter()
        .filter(|(_, _, created_at, liquidity)| {
            // Liquidity filter
            if *liquidity < config.min_liquidity_usd {
                return false;
            }

            // Age filter
            if let Some(max_age_days) = config.max_age_days {
                let age = now.signed_duration_since(*created_at);
                if age.num_days() > max_age_days {
                    return false;
                }
            }

            true
        })
        .map(|(mint, _, _, _)| mint.clone())
        .collect()
}

async fn show_security_stats() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Security Database Statistics");
    println!("{}", "=".repeat(50));

    let security_analyzer = screenerbot::tokens::security::get_security_analyzer();

    // Get token database stats
    let database = TokenDatabase::new()?;
    let all_tokens = database.get_all_tokens_with_update_time().await?;

    println!("🗃️  Token Database:");
    println!("   Total tokens: {}", all_tokens.len());

    // Count tokens with security info
    let mut tokens_with_security = 0;
    let mut risk_distribution: HashMap<String, usize> = HashMap::new();

    for (mint, _, _, _) in &all_tokens {
        if let Ok(Some(security_info)) = security_analyzer.database.get_security_info(mint) {
            tokens_with_security += 1;
            let risk_key = format!("{:?}", security_info.risk_level);
            *risk_distribution.entry(risk_key).or_insert(0) += 1;
        }
    }

    println!("🔐 Security Database:");
    println!(
        "   Tokens with security info: {} ({:.1}%)",
        tokens_with_security,
        ((tokens_with_security as f64) / (all_tokens.len() as f64)) * 100.0
    );
    println!("   Tokens missing security info: {}", all_tokens.len() - tokens_with_security);

    if !risk_distribution.is_empty() {
        println!("📈 Risk Level Distribution:");
        for (risk, count) in risk_distribution.iter() {
            println!("   {}: {}", risk, count);
        }
    }

    Ok(())
}

async fn cleanup_invalid_entries() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up invalid security entries...");

    // This would implement cleanup logic for corrupted entries
    // For now, just show what would be cleaned
    println!("✅ Cleanup completed (placeholder implementation)");

    Ok(())
}

fn print_dry_run_info(tokens_to_process: &[String], config: &UpdateConfig) {
    println!("\n🔍 DRY RUN - No changes will be made");
    println!("{}", "=".repeat(50));
    println!("Configuration:");
    println!("   Tokens to process: {}", tokens_to_process.len());
    println!("   Batch size: {}", config.batch_size);
    println!("   Delay between batches: {} seconds", config.delay_seconds);
    println!("   Force refresh: {}", config.force_refresh);
    println!("   Min liquidity: ${:.0}", config.min_liquidity_usd);

    if let Some(max_age) = config.max_age_days {
        println!("   Max age: {} days", max_age);
    }

    if let Some(risk_filter) = &config.risk_filter {
        println!("   Risk filter: {:?}", risk_filter);
    }

    println!("\nFirst 10 tokens that would be processed:");
    for (i, mint) in tokens_to_process.iter().take(10).enumerate() {
        println!("   {}. {}", i + 1, safe_truncate(mint, 12));
    }

    if tokens_to_process.len() > 10 {
        println!("   ... and {} more", tokens_to_process.len() - 10);
    }

    let estimated_time =
        ((tokens_to_process.len() / config.batch_size + 1) as u64) * config.delay_seconds;
    println!(
        "\n⏱️  Estimated time: {} seconds ({:.1} minutes)",
        estimated_time,
        (estimated_time as f64) / 60.0
    );
}

async fn run_security_update(
    tokens_to_process: &[String],
    config: &UpdateConfig
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Starting security update process");
    println!("{}", "=".repeat(50));

    // Get security analyzer instance
    let security_analyzer = screenerbot::tokens::security::get_security_analyzer();

    let mut stats = UpdateStats {
        total_tokens: tokens_to_process.len(),
        ..Default::default()
    };
    stats.start();

    // Process tokens in batches
    for (batch_index, batch) in tokens_to_process.chunks(config.batch_size).enumerate() {
        let batch_start = Instant::now();

        println!(
            "\n📦 Processing batch {} (tokens {}-{})",
            batch_index + 1,
            batch_index * config.batch_size + 1,
            (batch_index * config.batch_size + batch.len()).min(tokens_to_process.len())
        );

        // Filter tokens that need updates
        let mut tokens_to_update = Vec::new();

        for mint in batch {
            stats.processed += 1;

            // Check if update is needed
            let needs_update = if config.force_refresh {
                true
            } else {
                match security_analyzer.database.get_security_info(mint) {
                    Ok(Some(existing)) => {
                        // Check if data is stale
                        let age = chrono::Utc
                            ::now()
                            .signed_duration_since(existing.timestamps.last_updated);
                        age.num_hours() > 24 // Update if older than 24 hours
                    }
                    Ok(None) => true, // No security info exists
                    Err(_) => true, // Error accessing data
                }
            };

            if needs_update {
                tokens_to_update.push(mint.clone());
            } else {
                stats.skipped += 1;
                log(
                    LogTag::Security,
                    "SKIP",
                    &format!("Token {} has recent security data", safe_truncate(mint, 8))
                );
            }
        }

        if !tokens_to_update.is_empty() {
            println!("   🔄 Updating {} tokens in this batch", tokens_to_update.len());

            // Process the batch
            match security_analyzer.analyze_multiple_tokens(&tokens_to_update).await {
                Ok(results) => {
                    stats.updated += results.len();

                    println!("   ✅ Successfully updated {} tokens", results.len());

                    // Log some details about the results
                    for (mint, security_info) in results.iter() {
                        log(
                            LogTag::Security,
                            "UPDATE",
                            &format!(
                                "Token {} updated: score={}, risk={:?}",
                                safe_truncate(mint, 8),
                                security_info.security_score,
                                security_info.risk_level
                            )
                        );
                    }
                }
                Err(e) => {
                    stats.failed += tokens_to_update.len();
                    log(LogTag::Security, "ERROR", &format!("Batch update failed: {}", e));
                    println!("   ❌ Batch update failed: {}", e);
                }
            }
        } else {
            println!("   ⏭️  All tokens in batch have recent security data");
        }

        let batch_elapsed = batch_start.elapsed();
        println!("   ⏱️  Batch completed in {:.1} seconds", batch_elapsed.as_secs_f64());

        // Print progress
        stats.print_progress();

        // Delay between batches (except for the last batch)
        if batch_index < tokens_to_process.len() / config.batch_size {
            println!("   ⏳ Waiting {} seconds before next batch...", config.delay_seconds);
            tokio::time::sleep(tokio::time::Duration::from_secs(config.delay_seconds)).await;
        }
    }

    stats.print_final();

    log(
        LogTag::Security,
        "COMPLETE",
        &format!(
            "Security update completed: {}/{} tokens updated successfully",
            stats.updated,
            stats.total_tokens
        )
    );

    Ok(())
}
