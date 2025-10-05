#![allow(warnings)]

//! Comprehensive transaction pattern analyzer.
//!
//! This tool deep-dives into transaction structure to find patterns in:
//! - Fee collection by different DEXs and platforms
//! - SOL and token transfer patterns
//! - Platform-specific behavior
//! - Balance changes mapping to actual value flows
//!
//! Use this to understand how different platforms charge fees and move tokens,
//! so we can accurately extract swap amounts without hardcoding percentages.

use std::{ collections::HashMap, str::FromStr };

use anyhow::{ anyhow, Context, Result };
use clap::Parser;
use colored::*;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;

use screenerbot::{
    arguments::set_cmd_args,
    events,
    pools::swap::types::constants::{ WSOL_MINT },
    transactions::{
        database::init_transaction_database,
        processor::TransactionProcessor,
        types::{ Transaction },
    },
};

// Constants
const SOL_MINT: &str = "11111111111111111111111111111111"; // Native SOL

#[derive(Parser, Debug)]
#[command(
    name = "analyze_transaction_patterns",
    about = "Deep analysis of transaction patterns to understand fee structures and token flows"
)]
struct Args {
    /// Transaction signature to analyze
    #[arg(long, value_name = "SIGNATURE")]
    signature: String,

    /// Wallet public key involved in the transaction
    #[arg(long, value_name = "PUBKEY")]
    wallet: String,

    /// Show raw instruction data and logs
    #[arg(long)]
    raw: bool,

    /// Focus on balance changes analysis
    #[arg(long)]
    balance_focus: bool,

    /// Focus on instruction data analysis
    #[arg(long)]
    instruction_focus: bool,

    /// Show detailed program interactions
    #[arg(long)]
    program_details: bool,

    /// Analyze multiple transactions from CSV for pattern discovery
    #[arg(long, value_name = "PATH")]
    csv: Option<String>,

    /// Only analyze transactions involving specific router/platform
    #[arg(long, value_name = "ROUTER")]
    router_filter: Option<String>,
}

#[derive(Debug, Clone)]
struct ProgramInteraction {
    program_id: String,
    instruction_count: usize,
    accounts_involved: Vec<String>,
    data_size: Option<usize>,
}

#[derive(Debug, Clone)]
struct BalanceChangeDetail {
    account: String,
    mint: String,
    pre_balance: u64,
    post_balance: u64,
    change: i64,
    ui_change: f64,
    decimals: u32,
    is_wallet: bool,
    account_type: String, // "wallet", "ata", "program", "pool", "unknown"
}

#[derive(Debug, Clone)]
struct TransferPattern {
    from_account: String,
    to_account: String,
    amount: u64,
    mint: String,
    transfer_type: String, // "fee", "swap", "rent", "tip", "unknown"
    associated_program: Option<String>,
}

#[derive(Debug)]
struct TransactionAnalysis {
    signature: String,
    wallet: String,
    programs_used: Vec<ProgramInteraction>,
    balance_changes: Vec<BalanceChangeDetail>,
    transfer_patterns: Vec<TransferPattern>,
    sol_fee_total: f64,
    token_fees: HashMap<String, f64>,
    potential_platform_fees: Vec<TransferPattern>,
    ata_rent_changes: Vec<BalanceChangeDetail>,
    classification: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Set command line arguments for the global system
    let mut cmd_args = vec!["analyze_transaction_patterns".to_string()];
    set_cmd_args(cmd_args);

    // Initialize events system
    if let Err(e) = events::init().await {
        eprintln!("Warning: Events system not initialized: {}", e);
    }

    init_transaction_database().await.map_err(|e|
        anyhow!("Failed to initialize transactions database: {}", e)
    )?;

    let wallet_pubkey = Pubkey::from_str(&args.wallet).context("Invalid wallet pubkey")?;

    // Use the existing transaction processor infrastructure
    let processor = TransactionProcessor::new(wallet_pubkey);

    // Process the transaction to get all analysis data
    let transaction = processor
        .process_transaction(&args.signature).await
        .map_err(|e| anyhow!("Failed to process transaction {}: {}", args.signature, e))?;

    println!(
        "{}",
        format!("=== Transaction Pattern Analysis: {} ===", args.signature).bold().cyan()
    );
    println!("Wallet: {}", args.wallet.green());

    if let Some(slot) = transaction.slot {
        println!("Slot: {}", slot);
    }

    if let Some(block_time) = transaction.block_time {
        println!("Block Time: {}", block_time);
    }

    // Perform comprehensive analysis
    let analysis = analyze_transaction(&transaction, &wallet_pubkey, &args).await?;

    // Print analysis results
    print_analysis_summary(&analysis, &args);

    if args.raw {
        print_raw_data(&transaction);
    }

    Ok(())
}

async fn analyze_transaction(
    transaction: &Transaction,
    wallet_pubkey: &Pubkey,
    args: &Args
) -> Result<TransactionAnalysis> {
    let mut analysis = TransactionAnalysis {
        signature: transaction.signature.clone(),
        wallet: wallet_pubkey.to_string(),
        programs_used: Vec::new(),
        balance_changes: Vec::new(),
        transfer_patterns: Vec::new(),
        sol_fee_total: 0.0,
        token_fees: HashMap::new(),
        potential_platform_fees: Vec::new(),
        ata_rent_changes: Vec::new(),
        classification: "Unknown".to_string(),
    };

    // 1. Analyze program interactions from instructions
    analysis.programs_used = analyze_program_interactions(&transaction.instructions);

    // 2. Analyze balance changes
    analysis.balance_changes = analyze_balance_changes(transaction, wallet_pubkey)?;

    // 3. Extract transfer patterns from balance changes
    analysis.transfer_patterns = extract_transfer_patterns(&analysis.balance_changes);

    // 4. Classify transaction and detect fees
    classify_and_extract_fees(&mut analysis, transaction);

    // 5. Detect ATA rent operations
    extract_ata_rent_operations(&mut analysis);

    // 6. Find potential platform fees
    find_platform_fees(&mut analysis);

    Ok(analysis)
}

fn analyze_program_interactions(
    instructions: &[screenerbot::transactions::types::InstructionInfo]
) -> Vec<ProgramInteraction> {
    let mut program_counts: HashMap<String, ProgramInteraction> = HashMap::new();

    for instruction in instructions {
        let entry = program_counts
            .entry(instruction.program_id.clone())
            .or_insert(ProgramInteraction {
                program_id: instruction.program_id.clone(),
                instruction_count: 0,
                accounts_involved: Vec::new(),
                data_size: instruction.data.as_ref().map(|d| d.len()),
            });

        entry.instruction_count += 1;
        entry.accounts_involved.extend(instruction.accounts.clone());
    }

    program_counts.into_values().collect()
}

fn analyze_balance_changes(
    transaction: &Transaction,
    wallet_pubkey: &Pubkey
) -> Result<Vec<BalanceChangeDetail>> {
    let mut changes = Vec::new();

    // Process SOL balance changes
    for sol_change in &transaction.sol_balance_changes {
        let is_wallet = sol_change.account.to_string() == wallet_pubkey.to_string();
        let account_type = classify_account_type(
            &sol_change.account.to_string(),
            SOL_MINT,
            is_wallet
        );

        changes.push(BalanceChangeDetail {
            account: sol_change.account.to_string(),
            mint: SOL_MINT.to_string(),
            pre_balance: (sol_change.pre_balance * 1_000_000_000.0) as u64, // Convert to lamports
            post_balance: (sol_change.post_balance * 1_000_000_000.0) as u64, // Convert to lamports
            change: (sol_change.change * 1_000_000_000.0) as i64, // Convert to lamports
            ui_change: sol_change.change,
            decimals: 9,
            is_wallet,
            account_type,
        });
    }

    // Process token balance changes
    for token_change in &transaction.token_balance_changes {
        // For token balance changes, we need to determine the account from the mint
        // Since TokenBalanceChange doesn't have account/owner fields, we'll use placeholder values
        let account_placeholder = format!("TokenAccount-{}", &token_change.mint[..8]);
        let is_wallet = false; // We can't determine this without account info
        let account_type = classify_account_type(
            &account_placeholder,
            &token_change.mint,
            is_wallet
        );

        let pre_bal = token_change.pre_balance.unwrap_or(0.0);
        let post_bal = token_change.post_balance.unwrap_or(0.0);
        let change = post_bal - pre_bal;

        changes.push(BalanceChangeDetail {
            account: account_placeholder,
            mint: token_change.mint.clone(),
            pre_balance: (pre_bal * (10f64).powi(token_change.decimals as i32)) as u64,
            post_balance: (post_bal * (10f64).powi(token_change.decimals as i32)) as u64,
            change: (change * (10f64).powi(token_change.decimals as i32)) as i64,
            ui_change: change,
            decimals: token_change.decimals as u32,
            is_wallet,
            account_type,
        });
    }

    Ok(changes)
}

fn classify_account_type(account: &str, mint: &str, is_wallet: bool) -> String {
    if is_wallet {
        return "wallet".to_string();
    }

    // Known program IDs
    let known_programs = [
        "11111111111111111111111111111111", // System Program
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // SPL Token Program
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", // Associated Token Program
        "ComputeBudget111111111111111111111111111111", // Compute Budget Program
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4", // Jupiter v6
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB", // Jupiter v4
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P", // Pump.fun
        "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA", // Pump.fun AMM
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM v4
        "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // Raydium AMM v5
    ];

    if known_programs.contains(&account) {
        return "program".to_string();
    }

    // Check if it might be an ATA (Associated Token Account)
    // ATAs are typically 44 characters and not system programs
    if account.len() == 44 && mint != SOL_MINT {
        return "ata".to_string();
    }

    // Could be a pool or other contract account
    "unknown".to_string()
}

fn extract_transfer_patterns(balance_changes: &[BalanceChangeDetail]) -> Vec<TransferPattern> {
    let mut patterns = Vec::new();

    // Group by mint to find transfers
    let mut by_mint: HashMap<String, Vec<&BalanceChangeDetail>> = HashMap::new();
    for change in balance_changes {
        by_mint.entry(change.mint.clone()).or_default().push(change);
    }

    // For each mint, find who sent and who received
    for (mint, changes) in by_mint {
        let mut senders = Vec::new();
        let mut receivers = Vec::new();

        for change in changes {
            if change.change < 0 {
                senders.push(change);
            } else if change.change > 0 {
                receivers.push(change);
            }
        }

        // Create transfer patterns
        for sender in &senders {
            for receiver in &receivers {
                // Try to match amounts (allowing for small discrepancies due to fees)
                let sent_amount = -sender.change as u64;
                let received_amount = receiver.change as u64;

                if
                    sent_amount >= received_amount &&
                    sent_amount - received_amount < sent_amount / 100
                {
                    // Likely a direct transfer with possible fee
                    patterns.push(TransferPattern {
                        from_account: sender.account.clone(),
                        to_account: receiver.account.clone(),
                        amount: received_amount,
                        mint: mint.clone(),
                        transfer_type: classify_transfer_type(sender, receiver),
                        associated_program: None, // Will be filled later if we can determine
                    });
                }
            }
        }
    }

    patterns
}

fn classify_transfer_type(sender: &BalanceChangeDetail, receiver: &BalanceChangeDetail) -> String {
    // ATA rent detection (exactly 2,039,280 lamports for SOL)
    if sender.mint == SOL_MINT && -sender.change == 2_039_280 {
        return "ata_rent".to_string();
    }

    // If sender is wallet and receiver is program/unknown, likely a fee or swap
    if sender.is_wallet && !receiver.is_wallet {
        if receiver.account_type == "program" {
            return "fee".to_string();
        } else {
            return "swap_out".to_string();
        }
    }

    // If sender is program/unknown and receiver is wallet, likely swap proceeds
    if !sender.is_wallet && receiver.is_wallet {
        return "swap_in".to_string();
    }

    // Program to program transfers might be platform fees
    if !sender.is_wallet && !receiver.is_wallet {
        return "platform_fee".to_string();
    }

    "unknown".to_string()
}

fn classify_and_extract_fees(analysis: &mut TransactionAnalysis, transaction: &Transaction) {
    analysis.sol_fee_total = transaction.fee_sol;

    // Use the transaction processor's classification
    analysis.classification = format!(
        "{:?} ({:?})",
        transaction.transaction_type,
        transaction.direction
    );
}

fn extract_ata_rent_operations(analysis: &mut TransactionAnalysis) {
    const ATA_RENT_LAMPORTS: i64 = 2_039_280;

    for change in &analysis.balance_changes {
        if
            change.mint == SOL_MINT &&
            (change.change == ATA_RENT_LAMPORTS || change.change == -ATA_RENT_LAMPORTS)
        {
            analysis.ata_rent_changes.push(change.clone());
        }
    }
}

fn find_platform_fees(analysis: &mut TransactionAnalysis) {
    // Look for transfers that go to known platform fee collectors or unusual patterns
    for pattern in &analysis.transfer_patterns {
        if
            pattern.transfer_type == "platform_fee" ||
            (pattern.transfer_type == "fee" && !pattern.from_account.ends_with("wallet"))
        {
            analysis.potential_platform_fees.push(pattern.clone());
        }
    }
}

fn print_analysis_summary(analysis: &TransactionAnalysis, args: &Args) {
    println!("\n{}", "=== ANALYSIS SUMMARY ===".bold().yellow());
    println!("Classification: {}", analysis.classification.green());
    println!("Total SOL fees: {:.9} SOL", analysis.sol_fee_total);

    if !analysis.ata_rent_changes.is_empty() {
        println!("\n{}", "ATA Rent Operations:".bold());
        for rent in &analysis.ata_rent_changes {
            println!(
                "  {} {} {:.9} SOL",
                rent.account,
                if rent.change > 0 {
                    "received"
                } else {
                    "paid"
                },
                rent.ui_change.abs()
            );
        }
    }

    if args.program_details || args.instruction_focus {
        println!("\n{}", "Program Interactions:".bold());
        for program in &analysis.programs_used {
            println!(
                "  {} ({} instructions, {} accounts)",
                program.program_id.cyan(),
                program.instruction_count,
                program.accounts_involved.len()
            );

            // Identify known programs
            let program_name = match program.program_id.as_str() {
                "11111111111111111111111111111111" => "System Program",
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "SPL Token",
                "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => "Associated Token",
                "ComputeBudget111111111111111111111111111111" => "Compute Budget",
                "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter v6",
                "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "Jupiter v4",
                "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "Pump.fun",
                "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => "Pump.fun AMM",
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM v4",
                "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1" => "Raydium AMM v5",
                _ => "Unknown",
            };

            if program_name != "Unknown" {
                println!("    → {}", program_name.green());
            }
        }
    }

    if args.balance_focus {
        println!("\n{}", "Balance Changes:".bold());
        for change in &analysis.balance_changes {
            let change_str = if change.change > 0 {
                format!("+{:.9}", change.ui_change).green()
            } else {
                format!("{:.9}", change.ui_change).red()
            };

            let mint_display = if change.mint == SOL_MINT {
                "SOL".to_string()
            } else if change.mint == WSOL_MINT {
                "WSOL".to_string()
            } else {
                format!("{}...{}", &change.mint[..8], &change.mint[change.mint.len() - 8..])
            };

            println!(
                "  {} {} {} ({}{})",
                change.account_type.purple(),
                change_str,
                mint_display.cyan(),
                change.account,
                if change.is_wallet {
                    " [WALLET]".bold().to_string()
                } else {
                    "".to_string()
                }
            );
        }
    }

    if !analysis.transfer_patterns.is_empty() {
        println!("\n{}", "Transfer Patterns:".bold());
        for pattern in &analysis.transfer_patterns {
            let mint_display = if pattern.mint == SOL_MINT {
                "SOL".to_string()
            } else if pattern.mint == WSOL_MINT {
                "WSOL".to_string()
            } else {
                format!("{}...{}", &pattern.mint[..8], &pattern.mint[pattern.mint.len() - 8..])
            };

            let amount_ui =
                (pattern.amount as f64) /
                (10f64).powi(if pattern.mint == SOL_MINT { 9 } else { 6 });

            println!(
                "  {} {:.9} {} → {} ({})",
                pattern.transfer_type.yellow(),
                amount_ui,
                mint_display.cyan(),
                &pattern.to_account[..8],
                &pattern.from_account[..8]
            );
        }
    }

    if !analysis.potential_platform_fees.is_empty() {
        println!("\n{}", "Potential Platform Fees:".bold().red());
        for fee in &analysis.potential_platform_fees {
            let mint_display = if fee.mint == SOL_MINT {
                "SOL".to_string()
            } else {
                format!("{}...{}", &fee.mint[..8], &fee.mint[fee.mint.len() - 8..])
            };

            let amount_ui =
                (fee.amount as f64) / (10f64).powi(if fee.mint == SOL_MINT { 9 } else { 6 });

            println!(
                "  {} {:.9} {} (from {} to {})",
                fee.transfer_type.red(),
                amount_ui,
                mint_display,
                &fee.from_account[..12],
                &fee.to_account[..12]
            );
        }
    }
}

fn print_raw_data(transaction: &Transaction) {
    println!("\n{}", "=== RAW TRANSACTION DATA ===".bold().cyan());

    if let Some(ref raw_data) = transaction.raw_transaction_data {
        println!(
            "{}",
            serde_json::to_string_pretty(raw_data).unwrap_or("Error serializing".to_string())
        );
    } else {
        println!("No raw transaction data available");
    }

    // Also print log messages if available
    if !transaction.log_messages.is_empty() {
        println!("\n{}", "=== LOG MESSAGES ===".bold().cyan());
        for (i, log) in transaction.log_messages.iter().enumerate() {
            println!("{:3}: {}", i, log);
        }
    }
}
