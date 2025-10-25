#![allow(warnings)]

//! Verify transaction swap analysis against a Solscan CSV export.
//!
//! This tool parses a Solscan DeFi activities export, reprocesses each
//! transaction with the ScreenerBot transaction pipeline, and compares the
//! derived swap metrics (amounts, mints, router detection, etc.) against the
//! CSV expectations. It surfaces mismatches for deeper debugging.

use std::{collections::HashMap, path::PathBuf, str::FromStr};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use colored::*;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

use screenerbot::{
    arguments::set_cmd_args,
    constants::SOL_MINT,
    events,
    transactions::{
        database::init_transaction_database,
        processor::TransactionProcessor,
        types::{SwapPnLInfo, TokenSwapInfo, Transaction},
    },
};

#[derive(Parser, Debug)]
#[command(
    name = "verify_transactions_from_csv",
    about = "Cross-check Solscan swap exports with ScreenerBot transaction analysis"
)]
struct Args {
    /// Path to the Solscan DeFi activities CSV export (required for batch mode)
    #[arg(long, value_name = "PATH")]
    csv: Option<PathBuf>,

    /// Maximum number of rows to process (default: all)
    #[arg(long, value_name = "N")]
    limit: Option<usize>,

    /// Only take the last N swap rows from the CSV after filtering
    #[arg(long, value_name = "N")]
    tail: Option<usize>,

    /// Only display mismatched transactions (default: also show summary of matches)
    #[arg(long)]
    mismatches_only: bool,

    /// Print verbose details for mismatched transactions
    #[arg(long)]
    verbose: bool,

    /// Enable debug logging for transactions processing
    #[arg(long)]
    debug_transactions: bool,

    /// Check only a specific transaction signature
    #[arg(long, value_name = "SIGNATURE")]
    signature: Option<String>,

    /// Read raw transactions from cache only (no RPC fetch). Fast and deterministic.
    #[arg(long)]
    cache_only: bool,

    /// Force refresh raw transactions from RPC even when cache exists.
    #[arg(long)]
    force_refresh: bool,

    /// Inspect a single transaction by signature (bypasses CSV processing)
    #[arg(long, value_name = "SIGNATURE")]
    single: Option<String>,

    /// Wallet public key to use for single-transaction analysis (required with --single)
    #[arg(long, value_name = "PUBKEY")]
    wallet: Option<String>,

    /// Print full raw transaction JSON when using --single
    #[arg(long)]
    raw: bool,

    /// Only show mismatches where difference is greater than this percentage (e.g., 1.0 for 1%)
    #[arg(long, value_name = "PERCENT", default_value = "0.0")]
    min_mismatch_percent: f64,

    /// Optional path to write per-transaction verification results CSV for pattern analysis
    /// If provided, a CSV with columns
    /// signature,calculated_amount,verified_amount,percentage_diff,router,match_status
    /// will be written. Amounts are in SOL units (UI), percentage is signed.
    #[arg(long, value_name = "PATH")]
    output_results_csv: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Clone)]
struct CsvSwapRow {
    #[serde(rename = "Signature")]
    signature: String,

    #[serde(rename = "Block Time")]
    block_time: Option<i64>,

    #[serde(rename = "Human Time")]
    human_time: Option<String>,

    #[serde(rename = "Action")]
    action: String,

    #[serde(rename = "From")]
    wallet: String,

    #[serde(rename = "Token1")]
    token1: String,

    #[serde(rename = "Amount1")]
    amount1: String,

    #[serde(rename = "TokenDecimals1")]
    token_decimals1: Option<u32>,

    #[serde(rename = "Token2")]
    token2: String,

    #[serde(rename = "Amount2")]
    amount2: String,

    #[serde(rename = "TokenDecimals2")]
    token_decimals2: Option<u32>,

    #[serde(rename = "Value")]
    value: Option<String>,

    #[serde(rename = "Platforms")]
    platform: Option<String>,

    #[serde(rename = "Sources")]
    source: Option<String>,

    #[serde(rename = "Token1Multiplier")]
    token1_multiplier: Option<String>,

    #[serde(rename = "Token2Multiplier")]
    token2_multiplier: Option<String>,
}

#[derive(Debug, Default)]
struct VerificationStats {
    total_rows: usize,
    processed: usize,
    matched: usize,
    mismatched: usize,
    processing_failures: usize,
    skipped: usize,
}

#[derive(Debug)]
struct ComparisonOutcome {
    signature: String,
    matched: bool,
    issues: Vec<String>,
    swap_info: Option<TokenSwapInfo>,
    pnl_info: Option<SwapPnLInfo>,
    csv_row: CsvSwapRow,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set command line arguments for the global system including debug flags
    let mut cmd_args = vec!["verify_transactions_from_csv".to_string()];
    if args.debug_transactions {
        cmd_args.push("--debug-transactions".to_string());
    }
    if args.cache_only {
        cmd_args.push("--cache-only".to_string());
    }
    if args.force_refresh {
        cmd_args.push("--force-refresh".to_string());
    }
    set_cmd_args(cmd_args);

    // Initialize events system to enable structured recording from transaction processing
    if let Err(e) = events::init().await {
        eprintln!(
            "{}",
            format!(
                "[WARN] Events system not initialized (continuing without persistent events): {}",
                e
            )
        );
    } else {
        // Spawn background maintenance (non-blocking)
        events::start_maintenance_task().await;
    }

    // Single-transaction inspection mode
    if let Some(sig) = args.single.clone() {
        let wallet_str = args
            .wallet
            .clone()
            .ok_or_else(|| anyhow!("--wallet <PUBKEY> is required with --single"))?;
        let wallet_pk = Pubkey::from_str(&wallet_str).context("Invalid --wallet pubkey")?;
        init_transaction_database()
            .await
            .map_err(|e| anyhow!("Failed to initialize transactions database: {}", e))?;
        let processor = TransactionProcessor::new_with_cache_options(
            wallet_pk,
            args.cache_only,
            args.force_refresh,
        );

        let tx = processor
            .process_transaction(&sig)
            .await
            .map_err(|e| anyhow!("Processing failed for {}: {}", sig, e))?;

        println!("Single-transaction analysis for {}", sig.bold());
        println!("Wallet: {}", wallet_str);
        println!(
            "Status: {:?} success={} fee_lamports={:?}",
            tx.status, tx.success, tx.fee_lamports
        );
        if let Some(slot) = tx.slot {
            println!("Slot: {}", slot);
        }
        if let Some(bt) = tx.block_time {
            println!("BlockTime: {}", bt);
        }

        // Print swap info if available
        if let Some(ref swap) = tx.token_swap_info {
            println!(
                "Swap: type={} router={} input_mint={} output_mint={} input_ui={:.9} output_ui={:.9} (raw in={} out={})",
                swap.swap_type,
                swap.router,
                swap.input_mint,
                swap.output_mint,
                swap.input_ui_amount,
                swap.output_ui_amount,
                swap.input_amount,
                swap.output_amount
            );
        } else {
            println!("Swap: <none>");
        }
        if let Some(ref pnl) = tx.swap_pnl_info {
            println!(
                "PnL: swap_type={} sol_amount={:.9} token_amount={:.9} fees={:.9} ata_rents={:.9}",
                pnl.swap_type, pnl.sol_amount, pnl.token_amount, pnl.fees_paid_sol, pnl.ata_rents
            );
        }

        // Instruction summary
        if !tx.instructions.is_empty() {
            println!("Instructions ({}):", tx.instructions.len());
            for (i, inst) in tx.instructions.iter().enumerate() {
                println!(
                    "  {:>3}. program={} type={} accounts={}{}",
                    i,
                    inst.program_id,
                    inst.instruction_type,
                    inst.accounts.len(),
                    inst.data.as_ref().map(|_| " data").unwrap_or("")
                );
            }
        }

        // Optional raw JSON dump
        if args.raw {
            if let Some(raw) = tx.raw_transaction_data {
                println!("\nRaw transaction JSON:");
                println!("{}", serde_json::to_string_pretty(&raw)?);
            } else {
                println!("No raw transaction data cached.");
            }
        }

        return Ok(());
    }

    let csv_path = args.csv.clone().ok_or_else(|| {
        anyhow!("--csv <PATH> is required for batch verification (omit it when using --single)")
    })?;
    let rows = load_csv(&csv_path)?;
    if rows.is_empty() {
        println!("{}", "No rows found in CSV".yellow());
        return Ok(());
    }

    // Optionally tail the last N rows globally (after filtering by action)
    let rows = if let Some(n) = args.tail {
        if n >= rows.len() {
            rows
        } else {
            let start = rows.len() - n;
            rows[start..].to_vec()
        }
    } else {
        rows
    };

    init_transaction_database()
        .await
        .map_err(|e| anyhow!("Failed to initialize transactions database: {}", e))?;

    println!(
        "{}",
        format!(
            "Loaded {} swap rows across {} wallet(s)",
            rows.len(),
            rows.iter()
                .map(|r| &r.wallet)
                .collect::<std::collections::HashSet<_>>()
                .len()
        )
        .bold()
    );

    let mut stats = VerificationStats::default();
    stats.total_rows = rows.len();

    let grouped = group_rows_by_wallet(rows);

    let mut outcomes = Vec::new();

    // Prepare optional CSV writer for results
    let mut csv_writer: Option<csv::Writer<std::fs::File>> = None;
    if let Some(path) = args.output_results_csv.as_ref() {
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!(
                    "{}",
                    format!(
                        "[WARN] Could not create parent dir {}: {}",
                        parent.display(),
                        e
                    )
                );
            }
        }
        match std::fs::File::create(path) {
            Ok(f) => {
                let mut wtr = csv::WriterBuilder::new().has_headers(true).from_writer(f);
                // Header expected by analyze_mismatch_patterns.rs
                let _ = wtr.write_record([
                    "signature",
                    "calculated_amount",
                    "verified_amount",
                    "percentage_diff",
                    "router",
                    "match_status",
                ]);
                csv_writer = Some(wtr);
                println!(
                    "{}",
                    format!("Writing verification results to {}", path.display())
                );
            }
            Err(e) => {
                eprintln!(
                    "{}",
                    format!(
                        "[WARN] Could not create results CSV at {}: {} (continuing without)",
                        path.display(),
                        e
                    )
                );
            }
        }
    }

    for (wallet, wallet_rows) in grouped {
        if args.limit.is_some() && stats.processed >= args.limit.unwrap() {
            break;
        }

        let wallet_pubkey = Pubkey::from_str(&wallet).context("Invalid wallet pubkey in CSV")?;
        let processor = TransactionProcessor::new_with_cache_options(
            wallet_pubkey,
            args.cache_only,
            args.force_refresh,
        );

        for row in wallet_rows {
            if args.limit.is_some() && stats.processed >= args.limit.unwrap() {
                break;
            }

            // Skip if signature filter is specified and doesn't match
            if let Some(ref target_sig) = args.signature {
                if row.signature != *target_sig {
                    continue;
                }
            }

            stats.processed += 1;

            match processor.process_transaction(&row.signature).await {
                Ok(transaction) => {
                    let outcome =
                        compare_row_with_transaction(&row, &transaction, args.min_mismatch_percent);
                    if outcome.matched {
                        stats.matched += 1;
                        if !args.mismatches_only {
                            println!(
                                "{}",
                                format!("{} {}", "MATCH".green().bold(), &row.signature)
                            );
                        }
                    } else {
                        stats.mismatched += 1;
                        println!(
                            "{}",
                            format!(
                                "{} {}: {}",
                                "MISMATCH".red().bold(),
                                &row.signature,
                                outcome.issues.join("; ")
                            )
                        );

                        // Print a compact details line even without --verbose for faster triage
                        print_mismatch_compact(&outcome);

                        if args.verbose {
                            print_verbose_details(&outcome);
                        }
                    }

                    outcomes.push(outcome);

                    // Optionally write per-row CSV record for analyzer, only when mismatched
                    if let Some(ref mut wtr) = csv_writer {
                        let last = outcomes.last().unwrap();
                        if !last.matched {
                            if let Err(e) = write_results_csv_record(wtr, last) {
                                eprintln!("[WARN] Failed to write results CSV record: {}", e);
                            }
                        }
                    }
                }
                Err(err) => {
                    stats.processing_failures += 1;
                    println!(
                        "{}",
                        format!(
                            "{} {}: {}",
                            "PROCESSING_ERROR".red().bold(),
                            &row.signature,
                            err
                        )
                    );

                    // Even on processing error, try to log a CSV row with expected amounts from CSV and zero calculated
                    if let Some(ref mut wtr) = csv_writer {
                        if let Err(e) = write_results_csv_record_from_error(wtr, &row) {
                            eprintln!("[WARN] Failed to write error results CSV record: {}", e);
                        }
                    }
                }
            }
        }
    }

    println!("\n{}", "Verification summary".bold());
    println!("  Total rows: {}", stats.total_rows);
    println!("  Processed : {}", stats.processed);
    println!("  Matched   : {}", stats.matched.to_string().green());
    println!("  Mismatched: {}", stats.mismatched.to_string().red());
    println!(
        "  Failures  : {}",
        if stats.processing_failures == 0 {
            stats.processing_failures.to_string().green()
        } else {
            stats.processing_failures.to_string().red()
        }
    );

    if args.min_mismatch_percent > 0.0 {
        println!(
            "  Min mismatch threshold: {}%",
            args.min_mismatch_percent.to_string().cyan()
        );
    }

    if stats.mismatched > 0 {
        println!(
            "\n{}",
            "Use --verbose for detailed breakdowns or investigate the reported mismatches".yellow()
        );
    }

    // Flush CSV writer if used
    if let Some(mut wtr) = csv_writer {
        if let Err(e) = wtr.flush() {
            eprintln!("[WARN] Failed to flush results CSV: {}", e);
        }
    }

    Ok(())
}

fn load_csv(csv_path: &PathBuf) -> Result<Vec<CsvSwapRow>> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(csv_path)
        .with_context(|| format!("Failed to open CSV at {}", csv_path.display()))?;

    let mut rows = Vec::new();

    for result in reader.deserialize::<CsvSwapRow>() {
        let row = result.with_context(|| "Failed to deserialize CSV row")?;

        if !row.action.to_uppercase().contains("SWAP") {
            continue;
        }

        rows.push(row);
    }

    Ok(rows)
}

fn group_rows_by_wallet(rows: Vec<CsvSwapRow>) -> HashMap<String, Vec<CsvSwapRow>> {
    let mut grouped: HashMap<String, Vec<CsvSwapRow>> = HashMap::new();
    for row in rows {
        grouped.entry(row.wallet.clone()).or_default().push(row);
    }
    grouped
}

fn compare_row_with_transaction(
    row: &CsvSwapRow,
    transaction: &Transaction,
    min_mismatch_percent: f64,
) -> ComparisonOutcome {
    let mut issues = Vec::new();

    if !transaction.success {
        issues.push(format!(
            "Transaction failed on-chain: {:?}",
            transaction.error_message
        ));
    }

    if transaction.swap_pnl_info.is_none() {
        issues.push("No swap PnL info present".to_string());
    }

    let swap_info = transaction.token_swap_info.clone();
    let pnl_info = transaction.swap_pnl_info.clone();

    if let Some(ref swap) = swap_info {
        if let Err(err) = verify_swap_amounts(row, swap, &pnl_info, min_mismatch_percent) {
            issues.push(err);
        }

        if let Err(err) = verify_swap_orientation(row, swap) {
            issues.push(err);
        }

        if let Err(err) = verify_router_detection(row, swap) {
            issues.push(err);
        }
    } else {
        issues.push("No token swap info available".to_string());
    }

    if let Some(ref pnl) = pnl_info {
        if pnl.swap_type != "Buy" && pnl.swap_type != "Sell" {
            issues.push(format!(
                "Unexpected swap type in PnL info: {}",
                pnl.swap_type
            ));
        }
    }

    ComparisonOutcome {
        signature: row.signature.clone(),
        matched: issues.is_empty(),
        issues,
        swap_info,
        pnl_info,
        csv_row: row.clone(),
    }
}

fn verify_swap_orientation(row: &CsvSwapRow, swap: &TokenSwapInfo) -> Result<(), String> {
    let token1_sol = row.token1 == SOL_MINT;
    let token2_sol = row.token2 == SOL_MINT;

    let expected_swap_type = if token1_sol && !token2_sol {
        "sol_to_token"
    } else if !token1_sol && token2_sol {
        "token_to_sol"
    } else if token1_sol && token2_sol {
        return Err("CSV indicates SOL↔SOL swap, unsupported scenario".to_string());
    } else {
        "token_to_token"
    };

    if swap.swap_type != expected_swap_type {
        return Err(format!(
            "Swap type mismatch: expected {}, got {}",
            expected_swap_type, swap.swap_type
        ));
    }

    match expected_swap_type {
        "sol_to_token" => {
            if swap.input_mint != SOL_MINT {
                return Err(format!(
                    "Input mint mismatch for buy: expected WSOL, got {}",
                    swap.input_mint
                ));
            }
            if swap.output_mint != row.token2 {
                return Err(format!(
                    "Output mint mismatch: expected {}, got {}",
                    &row.token2, &swap.output_mint
                ));
            }
        }
        "token_to_sol" => {
            if swap.input_mint != row.token1 {
                return Err(format!(
                    "Input mint mismatch for sell: expected {}, got {}",
                    &row.token1, &swap.input_mint
                ));
            }
            if swap.output_mint != SOL_MINT {
                return Err(format!(
                    "Output mint mismatch for sell: expected WSOL, got {}",
                    &swap.output_mint
                ));
            }
        }
        "token_to_token" => {
            if swap.input_mint != row.token1 || swap.output_mint != row.token2 {
                return Err("Token-to-token mint mismatch".to_string());
            }
        }
        _ => {}
    }

    Ok(())
}

fn verify_swap_amounts(
    row: &CsvSwapRow,
    swap: &TokenSwapInfo,
    pnl_info: &Option<SwapPnLInfo>,
    min_mismatch_percent: f64,
) -> Result<(), String> {
    let decimals1 = decimals_or_default(&row.token1, row.token_decimals1)?;
    let decimals2 = decimals_or_default(&row.token2, row.token_decimals2)?;

    let expected_input_raw = parse_amount(&row.amount1, decimals1)
        .map_err(|e| format!("Failed to parse Token1 amount: {}", e))?;
    let expected_output_raw = parse_amount(&row.amount2, decimals2)
        .map_err(|e| format!("Failed to parse Token2 amount: {}", e))?;

    // Start with processor-reported raw amounts
    let mut actual_input_raw = swap.input_amount as i128;
    let mut actual_output_raw = swap.output_amount as i128;

    // Helper to compute diffs and percents
    let mut input_diff = ((expected_input_raw as i128) - actual_input_raw).abs();
    let mut output_diff = ((expected_output_raw as i128) - actual_output_raw).abs();
    let mut input_percent_diff = if expected_input_raw > 0 {
        ((input_diff as f64) / (expected_input_raw as f64)) * 100.0
    } else {
        0.0
    };
    let mut output_percent_diff = if expected_output_raw > 0 {
        ((output_diff as f64) / (expected_output_raw as f64)) * 100.0
    } else {
        0.0
    };

    // Router- and instruction-aware normalization: CSV often includes ATA rent in SOL legs,
    // while our swap amounts exclude it and surface it in PnL (ata_rents). When mismatching
    // on SOL-side amounts, try reconciling by adding rent back for comparison.
    if let Some(ref pnl) = pnl_info {
        let rent_lamports: i128 = ((pnl.ata_rents * 1_000_000_000.0).round() as i128).abs();
        if rent_lamports > 0 {
            // If selling token->SOL, CSV may include recovered rent in SOL out
            let is_token_to_sol = swap.swap_type == "token_to_sol" && row.token2 == SOL_MINT;
            // If buying SOL->token, CSV may include rent paid in SOL in
            let is_sol_to_token = swap.swap_type == "sol_to_token" && row.token1 == SOL_MINT;

            // Only attempt normalization when current diff would be flagged and adding rent could help
            let output_allowed = tolerance_for_decimals(decimals2) as i128;
            let input_allowed = tolerance_for_decimals(decimals1) as i128;

            if is_token_to_sol {
                // Try adding rent to actual output
                let alt_actual_output = actual_output_raw.saturating_add(rent_lamports);
                let alt_out_diff = ((expected_output_raw as i128) - alt_actual_output).abs();
                let alt_out_pct = if expected_output_raw > 0 {
                    ((alt_out_diff as f64) / (expected_output_raw as f64)) * 100.0
                } else {
                    0.0
                };
                // Accept the normalization if it resolves the mismatch under strict tolerances
                if alt_out_diff <= output_allowed || alt_out_pct < min_mismatch_percent {
                    actual_output_raw = alt_actual_output;
                    output_diff = alt_out_diff;
                    output_percent_diff = alt_out_pct;
                }
            } else if is_sol_to_token {
                // Try adding rent to actual input (SOL leg)
                let alt_actual_input = actual_input_raw.saturating_add(rent_lamports);
                let alt_in_diff = ((expected_input_raw as i128) - alt_actual_input).abs();
                let alt_in_pct = if expected_input_raw > 0 {
                    ((alt_in_diff as f64) / (expected_input_raw as f64)) * 100.0
                } else {
                    0.0
                };
                if alt_in_diff <= input_allowed || alt_in_pct < min_mismatch_percent {
                    actual_input_raw = alt_actual_input;
                    input_diff = alt_in_diff;
                    input_percent_diff = alt_in_pct;
                }
            }
        }
    }

    // Base tolerances by decimals (strict; no WSOL special-casing)
    let input_allowed = tolerance_for_decimals(decimals1);
    let output_allowed = tolerance_for_decimals(decimals2);

    if input_diff > (input_allowed as i128) && input_percent_diff >= min_mismatch_percent {
        return Err(format!(
            "Input amount mismatch: expected {} (raw {}), got {} (raw {}), diff {} ({:.2}%)",
            format_ui_amount(expected_input_raw, decimals1),
            expected_input_raw,
            format_ui_amount(actual_input_raw.max(0) as u128, decimals1),
            actual_input_raw,
            input_diff,
            input_percent_diff
        ));
    }

    if output_diff > (output_allowed as i128) && output_percent_diff >= min_mismatch_percent {
        return Err(format!(
            "Output amount mismatch: expected {} (raw {}), got {} (raw {}), diff {} ({:.2}%)",
            format_ui_amount(expected_output_raw, decimals2),
            expected_output_raw,
            format_ui_amount(actual_output_raw.max(0) as u128, decimals2),
            actual_output_raw,
            output_diff,
            output_percent_diff
        ));
    }

    Ok(())
}

fn verify_router_detection(row: &CsvSwapRow, swap: &TokenSwapInfo) -> Result<(), String> {
    let platform_hint = row
        .platform
        .as_ref()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if platform_hint.is_empty() {
        return Ok(());
    }

    let expected_router = if platform_hint.contains("jup") {
        "jupiter"
    } else if platform_hint.contains("raydium") {
        "raydium"
    } else if platform_hint.contains("orca") {
        "orca"
    } else if platform_hint.contains("pump") {
        "pumpfun"
    } else if platform_hint.contains("meteora") {
        "meteora"
    } else {
        return Ok(());
    };

    if swap.router != expected_router {
        return Err(format!(
            "Router mismatch: expected {}, detected {}",
            expected_router, swap.router
        ));
    }

    Ok(())
}

fn decimals_or_default(token: &str, decimals: Option<u32>) -> Result<u32, String> {
    if token == SOL_MINT {
        return Ok(9);
    }

    decimals.ok_or_else(|| {
        format!(
            "Missing decimals for token {} in CSV (required for verification)",
            token
        )
    })
}

fn parse_amount(raw: &str, decimals: u32) -> Result<u128, String> {
    if let Ok(value) = raw.parse::<u128>() {
        return Ok(value);
    }

    if let Ok(value_f64) = raw.parse::<f64>() {
        let scale = (10f64).powi(decimals as i32);
        let scaled = (value_f64 * scale).round();
        if scaled.is_finite() && scaled >= 0.0 {
            return Ok(scaled as u128);
        }
    }

    Err(format!("Could not parse '{}' as numeric amount", raw))
}

fn tolerance_for_decimals(decimals: u32) -> u64 {
    match decimals {
        0..=2 => 1,
        3..=6 => 10,
        7..=9 => 1_000,
        _ => 10_000,
    }
}

fn format_ui_amount(raw: u128, decimals: u32) -> String {
    if decimals == 0 {
        return raw.to_string();
    }

    let scale = (10f64).powi(decimals as i32);
    let value = (raw as f64) / scale;
    format!("{:.6}", value)
}

fn print_verbose_details(outcome: &ComparisonOutcome) {
    println!(
        "  CSV: token1={} amount1={} token2={} amount2={} action={}",
        &outcome.csv_row.token1,
        outcome.csv_row.amount1,
        &outcome.csv_row.token2,
        outcome.csv_row.amount2,
        outcome.csv_row.action
    );

    if let Some(ref swap) = outcome.swap_info {
        println!(
            "  Detected swap: type={} router={} input_mint={} output_mint={} input_ui={:.6} output_ui={:.6}",
            swap.swap_type,
            swap.router,
            &swap.input_mint,
            &swap.output_mint,
            swap.input_ui_amount,
            swap.output_ui_amount
        );
    } else {
        println!("  No swap info captured");
    }

    if let Some(ref pnl) = outcome.pnl_info {
        println!(
            "  PnL: swap_type={} sol_amount={:.6} token_amount={:.6} fees={:.6} ata_rents={:.6}",
            pnl.swap_type, pnl.sol_amount, pnl.token_amount, pnl.fees_paid_sol, pnl.ata_rents
        );
    }
}

/// Print a compact, single-block details view for mismatches to aid quick triage
fn print_mismatch_compact(outcome: &ComparisonOutcome) {
    let csv = &outcome.csv_row;

    // CSV platform hint and decimals
    let platform = csv.platform.as_ref().map(|s| s.as_str()).unwrap_or("-");

    // Always show CSV-side token+amount context
    let d1 = match decimals_or_default(&csv.token1, csv.token_decimals1) {
        Ok(v) => v,
        Err(_) => 0,
    };
    let d2 = match decimals_or_default(&csv.token2, csv.token_decimals2) {
        Ok(v) => v,
        Err(_) => 0,
    };

    // Calculate diffs if swap_info is present
    if let Some(ref swap) = outcome.swap_info {
        let exp_in_raw = parse_amount(&csv.amount1, d1).unwrap_or(0);
        let exp_out_raw = parse_amount(&csv.amount2, d2).unwrap_or(0);

        let act_in_raw = swap.input_amount as i128;
        let act_out_raw = swap.output_amount as i128;

        let in_diff = ((exp_in_raw as i128) - act_in_raw).abs();
        let out_diff = ((exp_out_raw as i128) - act_out_raw).abs();

        let in_pct = if exp_in_raw > 0 {
            ((in_diff as f64) / (exp_in_raw as f64)) * 100.0
        } else {
            0.0
        };
        let out_pct = if exp_out_raw > 0 {
            ((out_diff as f64) / (exp_out_raw as f64)) * 100.0
        } else {
            0.0
        };

        let exp_in_ui = format_ui_amount(exp_in_raw as u128, d1);
        let exp_out_ui = format_ui_amount(exp_out_raw as u128, d2);
        let act_in_ui = format_ui_amount(swap.input_amount as u128, d1);
        let act_out_ui = format_ui_amount(swap.output_amount as u128, d2);

        println!(
            "  Details: router={} type={} mints={} -> {} | csv_platform={} | csv_decimals={}/{}",
            swap.router, swap.swap_type, swap.input_mint, swap.output_mint, platform, d1, d2
        );
        println!(
            "           in: exp {} (raw {}) vs got {} (raw {}) [Δ {} ({:.2}%)]",
            exp_in_ui, exp_in_raw, act_in_ui, act_in_raw, in_diff, in_pct
        );
        println!(
            "           out: exp {} (raw {}) vs got {} (raw {}) [Δ {} ({:.2}%)]",
            exp_out_ui, exp_out_raw, act_out_ui, act_out_raw, out_diff, out_pct
        );
    } else {
        // No swap info captured; still provide CSV context for investigation
        println!(
            "  Details: router=<none> type=<none> mints={} -> {} | csv_platform={} | csv_decimals={}/{}",
            csv.token1,
            csv.token2,
            platform,
            d1,
            d2
        );
        println!(
            "           csv amounts: token1={} | token2={}",
            csv.amount1, csv.amount2
        );
    }
}

/// Compute SOL-leg expected and actual values (lamports) and signed percentage diff.
/// - Expected is taken from CSV SOL leg (Token1 or Token2 depending on WSOL position)
/// - Actual is taken from swap SOL leg amounts, with rent normalization applied using PnL
fn compute_sol_leg_diff(outcome: &ComparisonOutcome) -> Result<(i128, i128, f64), String> {
    let csv = &outcome.csv_row;

    // Determine which CSV side is SOL
    let token1_is_sol = csv.token1 == SOL_MINT;
    let token2_is_sol = csv.token2 == SOL_MINT;

    if !token1_is_sol && !token2_is_sol {
        return Err("CSV row does not include a SOL leg".to_string());
    }

    // Expected SOL in lamports from CSV (decimals always 9 for WSOL)
    let expected_sol_lamports: i128 = if token1_is_sol {
        parse_amount(&csv.amount1, 9)
            .map_err(|e| format!("Failed to parse CSV SOL amount1: {}", e))? as i128
    } else {
        parse_amount(&csv.amount2, 9)
            .map_err(|e| format!("Failed to parse CSV SOL amount2: {}", e))? as i128
    };

    // Actual SOL lamports from swap, or 0 if none
    let base_actual_sol_lamports: i128 = if let Some(ref swap) = outcome.swap_info {
        match swap.swap_type.as_str() {
            "sol_to_token" => swap.input_amount as i128,
            "token_to_sol" => swap.output_amount as i128,
            _ => 0,
        }
    } else {
        0
    };

    // Consider both with and without rent normalization; choose one that minimizes abs pct diff
    let mut candidates: Vec<i128> = vec![base_actual_sol_lamports];
    if let Some(ref pnl) = outcome.pnl_info {
        let rent_lamports: i128 = ((pnl.ata_rents * 1_000_000_000.0).round() as i128).abs();
        if rent_lamports > 0 {
            if let Some(ref swap) = outcome.swap_info {
                if swap.swap_type == "token_to_sol" && token2_is_sol {
                    candidates.push(base_actual_sol_lamports.saturating_add(rent_lamports));
                }
                if swap.swap_type == "sol_to_token" && token1_is_sol {
                    candidates.push(base_actual_sol_lamports.saturating_add(rent_lamports));
                }
            }
        }
    }

    // Pick the candidate that yields the smallest absolute percent difference
    let mut best_actual = base_actual_sol_lamports;
    let mut best_abs_pct = f64::INFINITY;
    for cand in candidates {
        let pct = if expected_sol_lamports != 0 {
            (((cand - expected_sol_lamports) as f64) / (expected_sol_lamports as f64)) * 100.0
        } else {
            0.0
        };
        let abs_pct = pct.abs();
        if abs_pct < best_abs_pct {
            best_abs_pct = abs_pct;
            best_actual = cand;
        }
    }

    let pct = if expected_sol_lamports != 0 {
        (((best_actual - expected_sol_lamports) as f64) / (expected_sol_lamports as f64)) * 100.0
    } else {
        0.0
    };

    Ok((best_actual, expected_sol_lamports, pct))
}

/// Write one CSV record for analyzer, amounts in SOL (UI)
fn write_results_csv_record(
    wtr: &mut csv::Writer<std::fs::File>,
    outcome: &ComparisonOutcome,
) -> Result<(), Box<dyn std::error::Error>> {
    let (actual_lamports, expected_lamports, pct) =
        compute_sol_leg_diff(outcome).unwrap_or((0, 0, 0.0));
    let actual_ui = (actual_lamports as f64) / 1_000_000_000.0;
    let expected_ui = (expected_lamports as f64) / 1_000_000_000.0;
    let router = outcome
        .swap_info
        .as_ref()
        .map(|s| s.router.clone())
        .unwrap_or_else(|| "-".to_string());
    let status = if outcome.matched { "MATCH" } else { "MISMATCH" };

    wtr.write_record(&[
        outcome.signature.as_str(),
        &format!("{:.9}", actual_ui),
        &format!("{:.9}", expected_ui),
        &format!("{:.6}", pct),
        router.as_str(),
        status,
    ])?;
    Ok(())
}

/// Minimal CSV record when processing errored—use CSV expected SOL and zero calculated
fn write_results_csv_record_from_error(
    wtr: &mut csv::Writer<std::fs::File>,
    row: &CsvSwapRow,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine SOL expected from CSV
    let token1_is_sol = row.token1 == SOL_MINT;
    let expected_sol_lamports: i128 = if token1_is_sol {
        parse_amount(&row.amount1, 9).unwrap_or(0) as i128
    } else {
        parse_amount(&row.amount2, 9).unwrap_or(0) as i128
    };
    let expected_ui = (expected_sol_lamports as f64) / 1_000_000_000.0;
    // Calculated is zero when processing fails
    let actual_ui = 0.0f64;
    let pct = if expected_sol_lamports != 0 {
        -100.0
    } else {
        0.0
    };

    wtr.write_record(&[
        row.signature.as_str(),
        &format!("{:.9}", actual_ui),
        &format!("{:.9}", expected_ui),
        &format!("{:.6}", pct),
        "-",
        "PROCESSING_ERROR",
    ])?;
    Ok(())
}
