//! # Position Balance Validation and Auto-Closure Tool
//!
//! This utility performs comprehensive position validation by:
//! 1. Loading all open positions from positions.json
//! 2. Checking actual wallet token balances for each position
//! 3. Automatically closing positions with zero balances (sold outside the bot)
//! 4. Reporting discrepancies between stored and actual token amounts
//! 5. Creating backup of positions.json before any modifications
//!
//! ## Usage
//! ```bash
//! # Preview what would be updated (analysis only)
//! cargo run --bin tool_positions_update -- --dry-run
//!
//! # Update positions with zero balances automatically
//! cargo run --bin tool_positions_update -- --force
//!
//! # Verbose output with detailed logging
//! cargo run --bin tool_positions_update -- --verbose
//!
//! # Help and usage information
//! cargo run --bin tool_positions_update -- --help
//! ```
//!
//! ## Safety Features
//! - Creates automatic backup of positions.json before modifications
//! - Only closes positions with confirmed zero wallet balances
//! - Validates position data integrity
//! - Provides detailed analysis reporting
//! - Graceful error handling for RPC failures
//! - Cross-references stored token amounts with actual wallet balances
//!
//! ## Configuration
//! Requires `configs.json` with wallet private key and RPC endpoints.
//! Reads and updates `positions.json` for position tracking.
//!
//! ## Use Cases
//! - Fix positions after tokens were sold outside the bot
//! - Validate position integrity after system crashes
//! - Clean up orphaned position records
//! - Audit actual vs tracked token balances

use screenerbot::{
    logger::{ log, LogTag, init_file_logging },
    utils::{ get_wallet_address, get_token_balance },
    positions::{ get_open_positions, Position },
    utils::save_positions_to_file,
    global::{ read_configs, POSITIONS_FILE },
    rpc::init_rpc_client,
};
use std::{ env, fs };
use chrono::Utc;
use colored::Colorize;

#[derive(Debug)]
struct PositionAnalysis {
    mint: String,
    symbol: String,
    stored_amount: u64,
    actual_balance: u64,
    needs_closure: bool,
    error: Option<String>,
}

/// Show help information
fn show_help() {
    println!("🔧 {} {}", "Position Balance Validation Tool".bold().blue(), "v1.0".dimmed());
    println!("========================================");
    println!();
    println!("📋 {}", "Purpose:".bold());
    println!("  Validate and fix positions with mismatched token balances");
    println!();
    println!("🎯 {}", "Usage:".bold());
    println!("  cargo run --bin tool_positions_update -- [OPTIONS]");
    println!();
    println!("⚙️  {}", "Options:".bold());
    println!("  {}              Analysis mode - no modifications", "--dry-run".green());
    println!("  {}               Update positions with zero balances", "--force".yellow());
    println!("  {}             Detailed output and logging", "--verbose".cyan());
    println!("  {}                Show this help message", "--help".blue());
    println!();
    println!("📊 {}", "Examples:".bold());
    println!(
        "  {}  # Analysis only",
        "cargo run --bin tool_positions_update -- --dry-run".dimmed()
    );
    println!(
        "  {}   # Update positions",
        "cargo run --bin tool_positions_update -- --force".dimmed()
    );
    println!(
        "  {} # Detailed analysis",
        "cargo run --bin tool_positions_update -- --verbose".dimmed()
    );
    println!();
    println!("⚠️  {}", "Safety:".bold().red());
    println!("  🛡️  Automatic backup of positions.json before changes");
    println!("  🔍 Only closes positions with confirmed zero balances");
    println!("  📊 Detailed validation and error reporting");
    println!();
    println!("📁 {}", "Files:".bold());
    println!("  📄 Input:  {}", POSITIONS_FILE.cyan());
    println!("  💾 Backup: {}", "positions_backup_YYYYMMDD_HHMMSS.json".cyan());
}

/// Create backup of positions.json
fn create_positions_backup() -> Result<String, String> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = format!("data/positions_backup_{}.json", timestamp);

    if let Err(e) = fs::copy(POSITIONS_FILE, &backup_path) {
        return Err(format!("Failed to create backup: {}", e));
    }

    log(LogTag::System, "BACKUP", &format!("✅ Created backup: {}", backup_path));
    Ok(backup_path)
}

/// Get token balance with error handling
async fn get_safe_token_balance(wallet_address: &str, mint: &str) -> Result<u64, String> {
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => Ok(balance),
        Err(e) => Err(format!("RPC error: {}", e)),
    }
}

/// Analyze a single position
async fn analyze_position(position: &Position, wallet_address: &str) -> PositionAnalysis {
    let stored_amount = position.token_amount.unwrap_or(0);

    match get_safe_token_balance(wallet_address, &position.mint).await {
        Ok(actual_balance) => {
            let needs_closure = actual_balance == 0 && stored_amount > 0;

            PositionAnalysis {
                mint: position.mint.clone(),
                symbol: position.symbol.clone(),
                stored_amount,
                actual_balance,
                needs_closure,
                error: None,
            }
        }
        Err(error) => {
            PositionAnalysis {
                mint: position.mint.clone(),
                symbol: position.symbol.clone(),
                stored_amount,
                actual_balance: 0,
                needs_closure: false,
                error: Some(error),
            }
        }
    }
}

/// Format token amount with proper decimal display
fn format_token_amount(amount: u64) -> String {
    if amount == 0 {
        "0".to_string()
    } else if amount > 1_000_000_000 {
        format!("{:.2}B", (amount as f64) / 1_000_000_000.0)
    } else if amount > 1_000_000 {
        format!("{:.2}M", (amount as f64) / 1_000_000.0)
    } else if amount > 1_000 {
        format!("{:.2}K", (amount as f64) / 1_000.0)
    } else {
        amount.to_string()
    }
}

/// Print analysis summary
fn print_analysis_summary(analyses: &[PositionAnalysis], verbose: bool) {
    println!();
    println!(
        "📊 {} {}",
        "Position Analysis Summary".bold().blue(),
        format!("({} positions)", analyses.len()).dimmed()
    );
    println!("======================================");

    let mut closure_needed = 0;
    let mut balance_mismatches = 0;
    let mut errors = 0;

    for analysis in analyses {
        if analysis.error.is_some() {
            errors += 1;
        } else if analysis.needs_closure {
            closure_needed += 1;
        } else if analysis.stored_amount != analysis.actual_balance {
            balance_mismatches += 1;
        }

        if verbose || analysis.needs_closure || analysis.error.is_some() {
            println!();
            println!("🪙 {} ({})", analysis.symbol.bold(), analysis.mint.dimmed());

            if let Some(error) = &analysis.error {
                println!("  ❌ {}: {}", "Error".red(), error);
            } else {
                println!(
                    "  📦 {}: {}",
                    "Stored".cyan(),
                    format_token_amount(analysis.stored_amount)
                );
                println!(
                    "  💰 {}: {}",
                    "Actual".green(),
                    format_token_amount(analysis.actual_balance)
                );

                if analysis.needs_closure {
                    println!("  🔥 {}", "NEEDS CLOSURE - Zero balance detected".red().bold());
                } else if analysis.stored_amount != analysis.actual_balance {
                    println!("  ⚠️  {}", "Balance mismatch".yellow());
                }
            }
        }
    }

    println!();
    println!("📈 {}", "Summary Statistics".bold());
    println!("-------------------");
    println!("  📊 Total positions analyzed: {}", analyses.len().to_string().cyan());
    println!("  🔥 Positions needing closure: {}", closure_needed.to_string().red());
    println!("  ⚠️  Balance mismatches: {}", balance_mismatches.to_string().yellow());
    println!("  ❌ RPC errors: {}", errors.to_string().red());

    if closure_needed > 0 {
        println!();
        println!("💡 {} Use {} to apply closure fixes", "Tip:".bold().blue(), "--force".green());
    }
}

/// Apply position closures
async fn apply_closures(analyses: &[PositionAnalysis], verbose: bool) -> Result<(), String> {
    let closures_needed: Vec<_> = analyses
        .iter()
        .filter(|a| a.needs_closure)
        .collect();

    if closures_needed.is_empty() {
        println!("✅ {}", "No position closures needed".green());
        return Ok(());
    }

    println!();
    println!(
        "🔧 {} {} positions...",
        "Applying closures to".bold().yellow(),
        closures_needed.len()
    );

    // Create backup before modifications
    create_positions_backup()?;

    // Get current positions
    let mut open_positions = get_open_positions();
    let mut successful_closures = 0;
    let mut failed_closures = 0;

    for analysis in &closures_needed {
        if verbose {
            println!("  🔥 Closing position: {} ({})", analysis.symbol, analysis.mint.dimmed());
        }

        // Find the position in the list
        if let Some(position) = open_positions.iter_mut().find(|p| p.mint == analysis.mint) {
            // Mark position as closed with zero exit price (manual closure)
            position.exit_price = Some(0.0);
            position.exit_time = Some(Utc::now());
            position.effective_exit_price = Some(0.0);
            position.sol_received = Some(0.0);

            log(
                LogTag::System,
                "CLOSURE",
                &format!("✅ Marked position closed: {}", analysis.symbol)
            );
            successful_closures += 1;
        } else {
            log(LogTag::System, "ERROR", &format!("❌ Position not found: {}", analysis.symbol));
            failed_closures += 1;
        }
    }

    // Save positions after all closures
    if successful_closures > 0 {
        save_positions_to_file(&open_positions);
        log(LogTag::System, "SAVE", "✅ Positions saved successfully");
    }

    println!();
    println!("📈 {} {}", "Closure Results".bold().green(), "Complete".dimmed());
    println!("  ✅ Successful closures: {}", successful_closures.to_string().green());
    println!("  ❌ Failed closures: {}", failed_closures.to_string().red());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());
    let force = args.contains(&"--force".to_string());
    let verbose = args.contains(&"--verbose".to_string());
    let help = args.contains(&"--help".to_string()) || args.contains(&"-h".to_string());

    if help {
        show_help();
        return Ok(());
    }

    // Initialize logging
    init_file_logging();
    log(LogTag::System, "START", "🔧 Position Balance Validation Tool");

    // Initialize services
    println!("🔧 {}", "Initializing services...".blue());

    // Load configurations
    let _configs = read_configs()?;

    // Initialize RPC client
    init_rpc_client()?;

    // Get wallet address
    let wallet_address = get_wallet_address()?;
    log(LogTag::System, "WALLET", &format!("📍 Wallet: {}", wallet_address.dimmed()));

    // Load open positions
    println!("📂 {}", "Loading open positions...".blue());
    let open_positions = get_open_positions();

    if open_positions.is_empty() {
        println!("✅ {}", "No open positions found - nothing to validate".green());
        return Ok(());
    }

    println!("📊 Found {} open positions to analyze", open_positions.len().to_string().cyan());

    // Analyze each position
    println!("🔍 {}", "Analyzing position balances...".blue());
    let mut analyses = Vec::new();

    for (i, position) in open_positions.iter().enumerate() {
        if verbose {
            println!(
                "  [{}/{}] Checking {} ({})",
                (i + 1).to_string().cyan(),
                open_positions.len().to_string().cyan(),
                position.symbol.bold(),
                position.mint.dimmed()
            );
        }

        let analysis = analyze_position(position, &wallet_address).await;
        analyses.push(analysis);
    }

    // Print analysis results
    print_analysis_summary(&analyses, verbose);

    // Apply fixes if requested
    if force && !dry_run {
        println!();
        println!("🚀 {}", "Applying position fixes...".bold().yellow());
        apply_closures(&analyses, verbose).await?;
    } else if dry_run {
        println!();
        println!(
            "🔍 {} (use {} to apply fixes)",
            "Analysis complete".bold().blue(),
            "--force".green()
        );
    } else {
        println!();
        println!(
            "💡 {} Use {} to apply automatic closures",
            "Tip:".bold().blue(),
            "--force".green()
        );
    }

    log(LogTag::System, "COMPLETE", "✅ Position validation completed");
    Ok(())
}
