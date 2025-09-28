//! Verify transaction swap analysis against a Solscan CSV export.
//!
//! This tool parses a Solscan DeFi activities export, reprocesses each
//! transaction with the ScreenerBot transaction pipeline, and compares the
//! derived swap metrics (amounts, mints, router detection, etc.) against the
//! CSV expectations. It surfaces mismatches for deeper debugging.

use std::{ collections::HashMap, path::PathBuf, str::FromStr };

use anyhow::{ anyhow, Context, Result };
use clap::Parser;
use colored::*;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

use screenerbot::{
    arguments::set_cmd_args,
    events,
    transactions::{
        database::init_transaction_database,
        processor::TransactionProcessor,
        types::{ SwapPnLInfo, TokenSwapInfo, Transaction },
        utils::{ WSOL_MINT },
    },
};

#[derive(Parser, Debug)]
#[command(
    name = "verify_transactions_from_csv",
    about = "Cross-check Solscan swap exports with ScreenerBot transaction analysis"
)]
struct Args {
    /// Path to the Solscan DeFi activities CSV export
    #[arg(long, value_name = "PATH")]
    csv: PathBuf,

    /// Maximum number of rows to process (default: all)
    #[arg(long, value_name = "N")]
    limit: Option<usize>,

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
    set_cmd_args(cmd_args);

    // Initialize events system to enable structured recording from transaction processing
    if let Err(e) = events::init().await {
        eprintln!(
            "{}",
            format!("[WARN] Events system not initialized (continuing without persistent events): {}", e)
        );
    } else {
        // Spawn background maintenance (non-blocking)
        events::start_maintenance_task().await;
    }

    let rows = load_csv(&args)?;
    if rows.is_empty() {
        println!("{}", "No rows found in CSV".yellow());
        return Ok(());
    }

    init_transaction_database().await.map_err(|e|
        anyhow!("Failed to initialize transactions database: {}", e)
    )?;

    println!(
        "{}",
        format!(
            "Loaded {} swap rows across {} wallet(s)",
            rows.len(),
            rows
                .iter()
                .map(|r| &r.wallet)
                .collect::<std::collections::HashSet<_>>()
                .len()
        ).bold()
    );

    let mut stats = VerificationStats::default();
    stats.total_rows = rows.len();

    let grouped = group_rows_by_wallet(rows);

    let mut outcomes = Vec::new();

    for (wallet, wallet_rows) in grouped {
        if args.limit.is_some() && stats.processed >= args.limit.unwrap() {
            break;
        }

        let wallet_pubkey = Pubkey::from_str(&wallet).context("Invalid wallet pubkey in CSV")?;
        let processor = TransactionProcessor::new(wallet_pubkey);

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
                    let outcome = compare_row_with_transaction(&row, &transaction);
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

                        if args.verbose {
                            print_verbose_details(&outcome);
                        }
                    }

                    outcomes.push(outcome);
                }
                Err(err) => {
                    stats.processing_failures += 1;
                    println!(
                        "{}",
                        format!("{} {}: {}", "PROCESSING_ERROR".red().bold(), &row.signature, err)
                    );
                }
            }
        }
    }

    println!("\n{}", "Verification summary".bold());
    println!("  Total rows: {}", stats.total_rows);
    println!("  Processed : {}", stats.processed);
    println!("  Matched   : {}", stats.matched.to_string().green());
    println!("  Mismatched: {}", stats.mismatched.to_string().red());
    println!("  Failures  : {}", if stats.processing_failures == 0 {
        stats.processing_failures.to_string().green()
    } else {
        stats.processing_failures.to_string().red()
    });

    if stats.mismatched > 0 {
        println!(
            "\n{}",
            "Use --verbose for detailed breakdowns or investigate the reported mismatches".yellow()
        );
    }

    Ok(())
}

fn load_csv(args: &Args) -> Result<Vec<CsvSwapRow>> {
    let mut reader = csv::ReaderBuilder
        ::new()
        .has_headers(true)
        .from_path(&args.csv)
        .with_context(|| format!("Failed to open CSV at {}", args.csv.display()))?;

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

fn compare_row_with_transaction(row: &CsvSwapRow, transaction: &Transaction) -> ComparisonOutcome {
    let mut issues = Vec::new();

    if !transaction.success {
        issues.push(format!("Transaction failed on-chain: {:?}", transaction.error_message));
    }

    if transaction.swap_pnl_info.is_none() {
        issues.push("No swap PnL info present".to_string());
    }

    let swap_info = transaction.token_swap_info.clone();
    let pnl_info = transaction.swap_pnl_info.clone();

    if let Some(ref swap) = swap_info {
        if let Err(err) = verify_swap_amounts(row, swap) {
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
            issues.push(format!("Unexpected swap type in PnL info: {}", pnl.swap_type));
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
    let token1_sol = row.token1 == WSOL_MINT;
    let token2_sol = row.token2 == WSOL_MINT;

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
        return Err(
            format!("Swap type mismatch: expected {}, got {}", expected_swap_type, swap.swap_type)
        );
    }

    match expected_swap_type {
        "sol_to_token" => {
            if swap.input_mint != WSOL_MINT {
                return Err(
                    format!("Input mint mismatch for buy: expected WSOL, got {}", swap.input_mint)
                );
            }
            if swap.output_mint != row.token2 {
                return Err(
                    format!(
                        "Output mint mismatch: expected {}, got {}",
                        &row.token2,
                        &swap.output_mint
                    )
                );
            }
        }
        "token_to_sol" => {
            if swap.input_mint != row.token1 {
                return Err(
                    format!(
                        "Input mint mismatch for sell: expected {}, got {}",
                        &row.token1,
                        &swap.input_mint
                    )
                );
            }
            if swap.output_mint != WSOL_MINT {
                return Err(
                    format!(
                        "Output mint mismatch for sell: expected WSOL, got {}",
                        &swap.output_mint
                    )
                );
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

fn verify_swap_amounts(row: &CsvSwapRow, swap: &TokenSwapInfo) -> Result<(), String> {
    let decimals1 = decimals_or_default(&row.token1, row.token_decimals1)?;
    let decimals2 = decimals_or_default(&row.token2, row.token_decimals2)?;

    let expected_input_raw = parse_amount(&row.amount1, decimals1).map_err(|e|
        format!("Failed to parse Token1 amount: {}", e)
    )?;
    let expected_output_raw = parse_amount(&row.amount2, decimals2).map_err(|e|
        format!("Failed to parse Token2 amount: {}", e)
    )?;

    let actual_input_raw = swap.input_amount as i128;
    let actual_output_raw = swap.output_amount as i128;

    let input_diff = ((expected_input_raw as i128) - actual_input_raw).abs();
    let output_diff = ((expected_output_raw as i128) - actual_output_raw).abs();

    let input_allowed = tolerance_for_decimals(decimals1);
    let output_allowed = tolerance_for_decimals(decimals2);

    if input_diff > (input_allowed as i128) {
        return Err(
            format!(
                "Input amount mismatch: expected {} (raw {}), got {} (raw {}), diff {}",
                format_ui_amount(expected_input_raw, decimals1),
                expected_input_raw,
                format_ui_amount(actual_input_raw as u128, decimals1),
                actual_input_raw,
                input_diff
            )
        );
    }

    if output_diff > (output_allowed as i128) {
        return Err(
            format!(
                "Output amount mismatch: expected {} (raw {}), got {} (raw {}), diff {}",
                format_ui_amount(expected_output_raw, decimals2),
                expected_output_raw,
                format_ui_amount(actual_output_raw as u128, decimals2),
                actual_output_raw,
                output_diff
            )
        );
    }

    Ok(())
}

fn verify_router_detection(row: &CsvSwapRow, swap: &TokenSwapInfo) -> Result<(), String> {
    let platform_hint = row.platform
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
        return Err(
            format!("Router mismatch: expected {}, detected {}", expected_router, swap.router)
        );
    }

    Ok(())
}

fn decimals_or_default(token: &str, decimals: Option<u32>) -> Result<u32, String> {
    if token == WSOL_MINT {
        return Ok(9);
    }

    decimals.ok_or_else(|| {
        format!("Missing decimals for token {} in CSV (required for verification)", token)
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
            pnl.swap_type,
            pnl.sol_amount,
            pnl.token_amount,
            pnl.fees_paid_sol,
            pnl.ata_rents
        );
    }
}
