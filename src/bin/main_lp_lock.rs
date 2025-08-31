/// LP Lock Detection Diagnostic Tool
///
/// This tool provides comprehensive testing and analysis of LP lock detection
/// for tokens on Solana. It supports various output formats and analysis modes.

use screenerbot::{
    errors::ScreenerBotError,
    logger::init_file_logging,
    rpc::init_rpc_client,
    tokens::lp_lock::{ check_lp_lock_status, check_multiple_lp_locks, LpLockStatus, LockPrograms },
    utils::safe_truncate,
};
use clap::{ Arg, Command };
use serde_json;
use std::process;

#[tokio::main]
async fn main() {
    // Initialize logging
    init_file_logging();

    let matches = Command::new("LP Lock Checker")
        .about("Analyze liquidity pool lock status for Solana tokens")
        .version("1.0.0")
        .arg(
            Arg::new("token-mint")
                .long("token-mint")
                .short('t')
                .value_name("MINT_ADDRESS")
                .help("Token mint address to check")
                .conflicts_with_all(&["batch-file", "list-programs"])
        )
        .arg(
            Arg::new("batch-file")
                .long("batch-file")
                .short('b')
                .value_name("FILE")
                .help("File containing token mint addresses (one per line)")
                .conflicts_with_all(&["token-mint", "list-programs"])
        )
        .arg(
            Arg::new("output-format")
                .long("output-format")
                .short('f')
                .value_name("FORMAT")
                .help("Output format: text, json, csv")
                .default_value("text")
                .value_parser(["text", "json", "csv"])
        )
        .arg(
            Arg::new("output-file")
                .long("output-file")
                .short('o')
                .value_name("FILE")
                .help("Output file (default: stdout)")
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("list-programs")
                .long("list-programs")
                .help("List all known lock programs")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with_all(&["token-mint", "batch-file"])
        )
        .arg(
            Arg::new("safe-only")
                .long("safe-only")
                .help("Only show tokens with safe LP lock status")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("risky-only")
                .long("risky-only")
                .help("Only show tokens with risky LP lock status")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("safe-only")
        )
        .get_matches();

    // Handle list-programs command
    if matches.get_flag("list-programs") {
        list_known_programs();
        return;
    }

    // Initialize RPC client
    if let Err(e) = init_rpc_client() {
        eprintln!("Failed to initialize RPC client: {}", e);
        process::exit(1);
    }

    let verbose = matches.get_flag("verbose");
    let output_format = matches.get_one::<String>("output-format").unwrap();
    let safe_only = matches.get_flag("safe-only");
    let risky_only = matches.get_flag("risky-only");

    if verbose {
        println!("🔍 LP Lock Checker - Starting analysis...");
        println!("📊 Output format: {}", output_format);
        if safe_only {
            println!("🟢 Filter: Safe tokens only");
        } else if risky_only {
            println!("🔴 Filter: Risky tokens only");
        }
        println!();
    }

    // Handle single token analysis
    if let Some(token_mint) = matches.get_one::<String>("token-mint") {
        match analyze_single_token(token_mint, verbose).await {
            Ok(analysis) => {
                // Apply filters
                if should_include_result(&analysis.status, safe_only, risky_only) {
                    output_single_result(
                        &analysis,
                        output_format,
                        matches.get_one::<String>("output-file")
                    );
                } else if verbose {
                    println!("Token filtered out based on safety criteria");
                }
            }
            Err(e) => {
                eprintln!("Error analyzing token {}: {}", safe_truncate(token_mint, 8), e);
                process::exit(1);
            }
        }
        return;
    }

    // Handle batch analysis
    if let Some(batch_file) = matches.get_one::<String>("batch-file") {
        match analyze_batch_file(batch_file, verbose, safe_only, risky_only).await {
            Ok(results) => {
                output_batch_results(
                    &results,
                    output_format,
                    matches.get_one::<String>("output-file")
                );
            }
            Err(e) => {
                eprintln!("Error in batch analysis: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // No valid command provided
    eprintln!("Error: Must provide either --token-mint, --batch-file, or --list-programs");
    eprintln!("Use --help for usage information");
    process::exit(1);
}

/// Analyze a single token
async fn analyze_single_token(
    token_mint: &str,
    verbose: bool
) -> Result<screenerbot::tokens::lp_lock::LpLockAnalysis, ScreenerBotError> {
    if verbose {
        println!("🔍 Analyzing token: {}", token_mint);
        println!("⏳ Checking LP lock status...");
    }

    let start_time = std::time::Instant::now();
    let analysis = check_lp_lock_status(token_mint).await?;
    let duration = start_time.elapsed();

    if verbose {
        println!("✅ Analysis completed in {:.2}s", duration.as_secs_f64());
        println!();
    }

    Ok(analysis)
}

/// Analyze tokens from a batch file
async fn analyze_batch_file(
    file_path: &str,
    verbose: bool,
    safe_only: bool,
    risky_only: bool
) -> Result<Vec<screenerbot::tokens::lp_lock::LpLockAnalysis>, String> {
    // Read token mints from file
    let content = std::fs
        ::read_to_string(file_path)
        .map_err(|e| format!("Failed to read batch file {}: {}", file_path, e))?;

    let token_mints: Vec<String> = content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();

    if token_mints.is_empty() {
        return Err("No valid token mints found in batch file".to_string());
    }

    if verbose {
        println!("📁 Loaded {} token mints from {}", token_mints.len(), file_path);
        println!("⏳ Starting batch analysis...");
        println!();
    }

    let start_time = std::time::Instant::now();

    match check_multiple_lp_locks(&token_mints).await {
        Ok(results) => {
            let duration = start_time.elapsed();

            // Apply filters
            let filtered_results: Vec<_> = results
                .into_iter()
                .filter(|analysis| should_include_result(&analysis.status, safe_only, risky_only))
                .collect();

            if verbose {
                println!("✅ Batch analysis completed in {:.2}s", duration.as_secs_f64());
                println!(
                    "📊 Results: {} tokens analyzed, {} match criteria",
                    token_mints.len(),
                    filtered_results.len()
                );
                println!();
            }

            Ok(filtered_results)
        }
        Err(e) => Err(format!("Batch analysis failed: {}", e)),
    }
}

/// Check if result should be included based on filters
fn should_include_result(status: &LpLockStatus, safe_only: bool, risky_only: bool) -> bool {
    if safe_only { status.is_safe() } else if risky_only { !status.is_safe() } else { true }
}

/// Output single result in specified format
fn output_single_result(
    analysis: &screenerbot::tokens::lp_lock::LpLockAnalysis,
    format: &str,
    output_file: Option<&String>
) {
    let content = match format {
        "json" => format_single_json(analysis),
        "csv" => format_single_csv(analysis),
        _ => format_single_text(analysis),
    };

    if let Some(file_path) = output_file {
        if let Err(e) = std::fs::write(file_path, content) {
            eprintln!("Failed to write to file {}: {}", file_path, e);
            process::exit(1);
        }
        println!("Results written to: {}", file_path);
    } else {
        print!("{}", content);
    }
}

/// Output batch results in specified format
fn output_batch_results(
    results: &[screenerbot::tokens::lp_lock::LpLockAnalysis],
    format: &str,
    output_file: Option<&String>
) {
    let content = match format {
        "json" => format_batch_json(results),
        "csv" => format_batch_csv(results),
        _ => format_batch_text(results),
    };

    if let Some(file_path) = output_file {
        if let Err(e) = std::fs::write(file_path, content) {
            eprintln!("Failed to write to file {}: {}", file_path, e);
            process::exit(1);
        }
        println!("Results written to: {}", file_path);
    } else {
        print!("{}", content);
    }
}

/// Format single result as text
fn format_single_text(analysis: &screenerbot::tokens::lp_lock::LpLockAnalysis) -> String {
    let mut output = String::new();

    output.push_str(&format!("🪙 Token: {}\n", analysis.token_mint));
    output.push_str(
        &format!(
            "🏊 Pool: {}\n",
            analysis.pool_address.as_ref().unwrap_or(&"Not found".to_string())
        )
    );
    output.push_str(
        &format!("🎫 LP Mint: {}\n", analysis.lp_mint.as_ref().unwrap_or(&"Not found".to_string()))
    );
    output.push_str(&format!("🔒 Status: {}\n", analysis.status.risk_level()));
    output.push_str(&format!("📝 Description: {}\n", analysis.status.description()));

    if let Some(pool_type) = &analysis.details.pool_type {
        output.push_str(&format!("🏗️  Pool Type: {}\n", pool_type));
    }

    if let Some(supply) = analysis.details.total_lp_supply {
        output.push_str(&format!("💰 LP Supply: {}\n", supply));
    }

    if analysis.details.locked_lp_amount > 0 {
        output.push_str(&format!("🔐 Locked Amount: {}\n", analysis.details.locked_lp_amount));
    }

    if analysis.details.creator_held_amount > 0 {
        output.push_str(&format!("👨‍💻 Creator Held: {}\n", analysis.details.creator_held_amount));
    }

    if !analysis.details.lock_programs.is_empty() {
        output.push_str(
            &format!("🛡️  Lock Programs: {}\n", analysis.details.lock_programs.join(", "))
        );
    }

    if let Some(ref authority) = analysis.details.lp_mint_authority {
        output.push_str(&format!("🔑 Mint Authority: {}\n", safe_truncate(authority, 12)));
    } else {
        output.push_str("🔑 Mint Authority: BURNED ✅\n");
    }

    if !analysis.details.notes.is_empty() {
        output.push_str("📋 Notes:\n");
        for note in &analysis.details.notes {
            output.push_str(&format!("   • {}\n", note));
        }
    }

    output.push_str(
        &format!("⏰ Analyzed: {}\n", analysis.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC"))
    );
    output.push('\n');

    output
}

/// Format single result as JSON
fn format_single_json(analysis: &screenerbot::tokens::lp_lock::LpLockAnalysis) -> String {
    match serde_json::to_string_pretty(analysis) {
        Ok(json) => json + "\n",
        Err(e) => format!("Error serializing to JSON: {}\n", e),
    }
}

/// Format single result as CSV
fn format_single_csv(analysis: &screenerbot::tokens::lp_lock::LpLockAnalysis) -> String {
    let mut output = String::new();

    // CSV header
    output.push_str(
        "token_mint,pool_address,lp_mint,status,risk_level,description,pool_type,lp_supply,locked_amount,creator_held,lock_programs,mint_authority_burned,analyzed_at\n"
    );

    // CSV data
    output.push_str(
        &format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            analysis.token_mint,
            analysis.pool_address.as_ref().unwrap_or(&"".to_string()),
            analysis.lp_mint.as_ref().unwrap_or(&"".to_string()),
            format!("{:?}", analysis.status),
            analysis.status.risk_level(),
            analysis.status.description(),
            analysis.details.pool_type.as_ref().unwrap_or(&"".to_string()),
            analysis.details.total_lp_supply.unwrap_or(0),
            analysis.details.locked_lp_amount,
            analysis.details.creator_held_amount,
            analysis.details.lock_programs.join(";"),
            analysis.details.lp_mint_authority.is_none(),
            analysis.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")
        )
    );

    output
}

/// Format batch results as text
fn format_batch_text(results: &[screenerbot::tokens::lp_lock::LpLockAnalysis]) -> String {
    let mut output = String::new();

    output.push_str(&format!("📊 LP Lock Analysis Results ({} tokens)\n", results.len()));
    output.push_str(&"═".repeat(60));
    output.push('\n');
    output.push('\n');

    // Summary stats
    let safe_count = results
        .iter()
        .filter(|r| r.status.is_safe())
        .count();
    let risky_count = results.len() - safe_count;

    output.push_str("📈 SUMMARY:\n");
    output.push_str(
        &format!(
            "🟢 Safe tokens: {} ({:.1}%)\n",
            safe_count,
            ((safe_count as f64) / (results.len() as f64)) * 100.0
        )
    );
    output.push_str(
        &format!(
            "🔴 Risky tokens: {} ({:.1}%)\n",
            risky_count,
            ((risky_count as f64) / (results.len() as f64)) * 100.0
        )
    );
    output.push('\n');

    // Status breakdown
    let mut status_counts = std::collections::HashMap::new();
    for result in results {
        let status_str = format!("{:?}", result.status);
        *status_counts.entry(status_str).or_insert(0) += 1;
    }

    output.push_str("📋 STATUS BREAKDOWN:\n");
    for (status, count) in status_counts {
        output.push_str(&format!("   {}: {}\n", status, count));
    }
    output.push('\n');

    // Individual results
    output.push_str("🔍 INDIVIDUAL RESULTS:\n");
    output.push_str(&"─".repeat(60));
    output.push('\n');

    for (i, result) in results.iter().enumerate() {
        output.push_str(
            &format!(
                "{}. {} {} - {}\n",
                i + 1,
                safe_truncate(&result.token_mint, 12),
                result.status.risk_level(),
                result.status.description()
            )
        );

        if let Some(pool_type) = &result.details.pool_type {
            output.push_str(
                &format!(
                    "   Pool: {} ({})\n",
                    result.pool_address
                        .as_ref()
                        .map(|p| safe_truncate(p, 12))
                        .unwrap_or("N/A"),
                    pool_type
                )
            );
        }

        if !result.details.lock_programs.is_empty() {
            output.push_str(
                &format!("   Lock Programs: {}\n", result.details.lock_programs.join(", "))
            );
        }

        output.push('\n');
    }

    output
}

/// Format batch results as JSON
fn format_batch_json(results: &[screenerbot::tokens::lp_lock::LpLockAnalysis]) -> String {
    let wrapper =
        serde_json::json!({
        "total_analyzed": results.len(),
        "analysis_timestamp": chrono::Utc::now(),
        "results": results
    });

    match serde_json::to_string_pretty(&wrapper) {
        Ok(json) => json + "\n",
        Err(e) => format!("Error serializing to JSON: {}\n", e),
    }
}

/// Format batch results as CSV
fn format_batch_csv(results: &[screenerbot::tokens::lp_lock::LpLockAnalysis]) -> String {
    let mut output = String::new();

    // CSV header
    output.push_str(
        "token_mint,pool_address,lp_mint,status,risk_level,description,pool_type,lp_supply,locked_amount,creator_held,lock_programs,mint_authority_burned,analyzed_at\n"
    );

    // CSV data
    for result in results {
        output.push_str(
            &format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                result.token_mint,
                result.pool_address.as_ref().unwrap_or(&"".to_string()),
                result.lp_mint.as_ref().unwrap_or(&"".to_string()),
                format!("{:?}", result.status),
                result.status.risk_level(),
                result.status.description(),
                result.details.pool_type.as_ref().unwrap_or(&"".to_string()),
                result.details.total_lp_supply.unwrap_or(0),
                result.details.locked_lp_amount,
                result.details.creator_held_amount,
                result.details.lock_programs.join(";"),
                result.details.lp_mint_authority.is_none(),
                result.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")
            )
        );
    }

    output
}

/// List all known lock programs
fn list_known_programs() {
    println!("🛡️  Known LP Lock Programs:");
    println!("{}", "═".repeat(60));
    println!();

    let programs = LockPrograms::known_programs();
    let mut sorted_programs: Vec<_> = programs.into_iter().collect();
    sorted_programs.sort_by(|a, b| a.1.cmp(b.1));

    for (address, name) in sorted_programs {
        println!("📦 {}", name);
        println!("   Address: {}", address);
        println!();
    }

    println!("Total: {} lock programs supported", LockPrograms::all_addresses().len());
}
