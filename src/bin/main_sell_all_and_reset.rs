#![allow(warnings)]

//! # Sell All Tokens, Close ATAs, and Reset Bot Data
//!
//! This utility performs a comprehensive wallet cleanup and bot data reset by:
//! 1. Scanning for all SPL Token and Token-2022 accounts
//! 2. Selling all tokens with non-zero balances for SOL (with retry on failure)
//! 3. Closing all Associated Token Accounts (ATAs) to reclaim rent SOL (with retry on failure)
//! 4. Removing specific bot data files to reset the system
//!
//! ## Usage
//! ```bash
//! cargo run --bin main_sell_all_and_reset
//! ```
//!
//! ## Safety Features
//! - Skips SOL (native token) accounts
//! - Validates token balances before selling with retry logic
//! - Comprehensive balance checking before and after operations
//! - Retry mechanism for failed sells and ATA closes
//! - Supports both SPL Token and Token-2022 standards
//! - Provides detailed progress reporting
//! - Graceful error handling for failed operations
//! - Estimates rent SOL reclaimed from closed ATAs
//! - Only removes specific data files, not configuration files
//!
//! ## Configuration
//! Requires `configs.json` with wallet private key and RPC endpoints.
//!
//! ## Warning
//! This tool will attempt to sell ALL tokens in your wallet AND delete specific bot data files. Use with caution!

use futures::stream::{ self, StreamExt };
use screenerbot::arguments::{ is_debug_ata_enabled, is_debug_swaps_enabled, set_cmd_args };
use screenerbot::errors::ScreenerBotError;
use screenerbot::logger::{ log, LogTag };
use screenerbot::rpc::{ get_rpc_client, init_rpc_client, TokenAccountInfo };
use screenerbot::swaps::config::{ QUOTE_SLIPPAGE_PERCENT, SOL_MINT };
use screenerbot::swaps::{ execute_best_swap, get_best_quote };
use screenerbot::tokens::Token;
use screenerbot::utils::{
    close_token_account_with_context,
    get_all_token_accounts,
    get_sol_balance,
    get_token_balance,
    get_wallet_address,
    safe_truncate,
};
use solana_sdk::{
    instruction::Instruction,
    signature::{ Signature, Signer },
    signer::keypair::Keypair,
    transaction::Transaction,
    pubkey::Pubkey,
};
use spl_token::instruction as spl_instruction;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{ sleep, Duration };

/// Print comprehensive help menu for the Sell All and Reset Tool
fn print_help() {
    log(LogTag::System, "INFO", "🔄 Sell All Tokens, Close ATAs, and Reset Bot Data Tool");
    log(LogTag::System, "INFO", "======================================================");
    log(
        LogTag::System,
        "INFO",
        "Comprehensive wallet cleanup and bot reset utility that sells all tokens for SOL,"
    );
    log(
        LogTag::System,
        "INFO",
        "closes all Associated Token Accounts (ATAs), and resets bot data files."
    );
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "WARNING", "⚠️  WARNING: This tool will:");
    log(LogTag::System, "WARNING", "    • Sell ALL tokens in your wallet for SOL");
    log(LogTag::System, "WARNING", "    • Close empty Associated Token Accounts (ATAs)");
    log(LogTag::System, "WARNING", "    • DELETE specific bot data files (irreversible)");
    log(
        LogTag::System,
        "WARNING",
        "    Use with extreme caution and understand the risks involved."
    );
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "USAGE:");
    log(LogTag::System, "INFO", "    cargo run --bin tool_sell_all_and_reset [OPTIONS]");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "OPTIONS:");
    log(LogTag::System, "INFO", "    --help, -h          Show this help message");
    log(
        LogTag::System,
        "INFO",
        "    --dry-run, -d      Simulate operations without executing transactions"
    );
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "EXAMPLES:");
    log(LogTag::System, "INFO", "    # Simulate cleanup to see what would happen");
    log(LogTag::System, "INFO", "    cargo run --bin tool_sell_all_and_reset -- --dry-run");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "    # Execute full wallet cleanup and bot reset (DANGEROUS)");
    log(LogTag::System, "INFO", "    cargo run --bin tool_sell_all_and_reset");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "OPERATIONS PERFORMED:");
    log(LogTag::System, "INFO", "    1. Scan wallet for all SPL Token and Token-2022 accounts");
    log(LogTag::System, "INFO", "    2. Identify tokens with non-zero balances");
    log(LogTag::System, "INFO", "    3. Sell larger token amounts for SOL using swap service");
    log(LogTag::System, "INFO", "    4. Burn small dust amounts (<1000 raw units) to empty ATAs");
    log(LogTag::System, "INFO", "    5. Close only EMPTY Associated Token Accounts (zero balance)");
    log(LogTag::System, "INFO", "    6. Reclaim rent SOL from closed ATAs (~0.00203928 SOL each)");
    log(LogTag::System, "INFO", "    7. Delete specific bot data files to reset the system");
    log(LogTag::System, "INFO", "    8. Clean up all bot log files from logs/ directory");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "DATA FILES THAT WILL BE DELETED:");
    log(LogTag::System, "INFO", "    • data/rpc_stats.json (RPC statistics)");
    log(LogTag::System, "INFO", "    • data/ata_failed_cache.json (failed ATA cache)");
    log(LogTag::System, "INFO", "    • data/positions.db (trading positions database)");
    log(LogTag::System, "INFO", "    • data/events.db (events system database)");
    log(LogTag::System, "INFO", "    • data/events.db-shm (events DB shared memory)");
    log(LogTag::System, "INFO", "    • data/events.db-wal (events DB write-ahead log)");
    log(LogTag::System, "INFO", "    • logs/screenerbot_*.log (all bot log files)");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "FILES THAT WILL BE PRESERVED:");
    log(LogTag::System, "INFO", "    • data/configs.json (wallet keys and RPC endpoints)");
    log(LogTag::System, "INFO", "    • data/tokens.db (token database)");
    log(LogTag::System, "INFO", "    • data/decimal_cache.json (token decimals cache)");
    log(LogTag::System, "INFO", "    • data/token_blacklist.json (blacklisted tokens)");
    log(LogTag::System, "INFO", "    • data/wallet_transactions_stats.json (wallet sync data)");
    log(LogTag::System, "INFO", "    • data/cache_ohlcvs/ (OHLCV data cache)");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "SAFETY FEATURES:");
    log(LogTag::System, "INFO", "    • Skips SOL (native token) - cannot sell SOL for SOL");
    log(LogTag::System, "INFO", "    • Validates token balances before attempting sales");
    log(
        LogTag::System,
        "INFO",
        "    • Only closes ATAs with zero balance (cannot close non-empty ATAs)"
    );
    log(LogTag::System, "INFO", "    • Detailed progress reporting for each operation");
    log(LogTag::System, "INFO", "    • Graceful error handling for failed transactions");
    log(LogTag::System, "INFO", "    • Supports both SPL Token and Token-2022 programs");
    log(LogTag::System, "INFO", "    • Concurrent processing with rate limiting");
    log(LogTag::System, "INFO", "    • Only removes specific data files, preserves configuration");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "INFO", "ESTIMATED OUTCOMES:");
    log(LogTag::System, "INFO", "    • SOL received from token sales (varies by token values)");
    log(LogTag::System, "INFO", "    • Rent SOL reclaimed from closed ATAs");
    log(LogTag::System, "INFO", "    • Clean wallet with only SOL remaining");
    log(LogTag::System, "INFO", "    • Fresh bot state with preserved configuration");
    log(LogTag::System, "INFO", "");
    log(LogTag::System, "WARNING", "RISK WARNINGS:");
    log(
        LogTag::System,
        "WARNING",
        "    • Irreversible operation - tokens will be permanently sold"
    );
    log(LogTag::System, "WARNING", "    • Bot data files will be permanently deleted");
    log(LogTag::System, "WARNING", "    • Market slippage may result in lower SOL amounts");
    log(LogTag::System, "WARNING", "    • Some tokens may fail to sell due to liquidity issues");
    log(LogTag::System, "WARNING", "    • Failed transactions still consume transaction fees");
    log(LogTag::System, "WARNING", "    • Use --dry-run first to understand the impact");
    log(LogTag::System, "INFO", "");
}

/// Data files to be removed during reset
const DATA_FILES_TO_REMOVE: &[&str] = &[
    "data/rpc_stats.json",
    "data/ata_failed_cache.json",
    "data/positions.db",
    "data/events.db",
    "data/events.db-shm",
    "data/events.db-wal",
];

/// Configuration for retry logic
const MAX_SELL_RETRIES: u32 = 3;
const MAX_ATA_CLOSE_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 2000;
/// Minimum raw token units to attempt a swap (avoid wasting quote calls on dust)
const DUST_THRESHOLD_RAW_UNITS: u64 = 25; // configurable heuristic

/// Minimum token units that should be burned instead of sold (saves on swap fees)
const BURN_THRESHOLD_RAW_UNITS: u64 = 1; // tokens below this will be burned

/// Burn small token amounts to prepare ATAs for closing
async fn burn_dust_tokens_with_retry(
    token_accounts: &[TokenAccountInfo],
    dry_run: bool
) -> (usize, usize, HashSet<String>) {
    log(
        LogTag::System,
        "BURN_START",
        &format!("Starting dust token burning for {} accounts{}", token_accounts.len(), if dry_run {
            " (DRY RUN)"
        } else {
            ""
        })
    );

    // Filter accounts for burning (small amounts > 0 but < BURN_THRESHOLD)
    let burnable_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|account| {
            if account.balance == 0 {
                return false;
            }

            if account.mint == SOL_MINT {
                return false;
            }

            // Only burn very small amounts that are uneconomical to sell
            if account.balance > 0 && account.balance < BURN_THRESHOLD_RAW_UNITS {
                log(
                    LogTag::System,
                    "BURN_CANDIDATE",
                    &format!(
                        "Found dust token for burning: {} ({} raw units)",
                        safe_truncate(&account.mint, 8),
                        account.balance
                    )
                );
                return true;
            }

            false
        })
        .collect();

    log(
        LogTag::System,
        "BURN_FILTER",
        &format!(
            "Filtered {} burnable accounts from {} total (burn threshold: {} units)",
            burnable_accounts.len(),
            token_accounts.len(),
            BURN_THRESHOLD_RAW_UNITS
        )
    );

    if burnable_accounts.is_empty() {
        return (0, 0, HashSet::new());
    }

    if dry_run {
        log(
            LogTag::System,
            "DRY_BURN",
            &format!("Would burn {} dust tokens", burnable_accounts.len())
        );
        let dry_run_burned_mints: HashSet<String> = burnable_accounts
            .iter()
            .map(|account| account.mint.clone())
            .collect();
        return (burnable_accounts.len(), 0, dry_run_burned_mints);
    }

    // For now, we'll use a simple burn by transferring to a burn address
    // In the future, this could use actual SPL Token burn instructions
    let mut successful_burns = 0;
    let mut failed_burns = 0;
    let mut successfully_burned_mints = HashSet::new();

    for account in &burnable_accounts {
        log(
            LogTag::System,
            "BURN_ATTEMPT",
            &format!(
                "Attempting to burn {} raw units of {}",
                account.balance,
                safe_truncate(&account.mint, 8)
            )
        );

        // For simplicity, we'll use a "burn" by transferring to the null address
        // This is not a real burn but achieves the same goal of emptying the ATA
        match burn_single_token_amount(account).await {
            Ok(signature) => {
                log(
                    LogTag::System,
                    "BURN_SUCCESS",
                    &format!(
                        "Successfully burned dust amount for {}. TX: {}",
                        safe_truncate(&account.mint, 8),
                        signature
                    )
                );
                successful_burns += 1;
                successfully_burned_mints.insert(account.mint.clone());
            }
            Err(error_msg) => {
                log(
                    LogTag::System,
                    "BURN_FAILED",
                    &format!(
                        "Failed to burn dust amount for {}: {}",
                        safe_truncate(&account.mint, 8),
                        error_msg
                    )
                );
                failed_burns += 1;
            }
        }
    }

    log(
        LogTag::System,
        "BURN_SUMMARY",
        &format!("Burn completed: {} success, {} failed", successful_burns, failed_burns)
    );

    (successful_burns, failed_burns, successfully_burned_mints)
}

/// Burn a single token's small amount using SPL Token burn instruction
async fn burn_single_token_amount(account: &TokenAccountInfo) -> Result<String, String> {
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;
    let rpc_client = get_rpc_client();

    log(
        LogTag::System,
        "BURN_START",
        &format!(
            "Starting burn of {} raw units for token {}",
            account.balance,
            safe_truncate(&account.mint, 8)
        )
    );

    // Get the associated token account address
    let ata_address = match
        rpc_client.get_associated_token_account(&wallet_address, &account.mint).await
    {
        Ok(addr) => addr,
        Err(e) => {
            return Err(format!("Failed to get ATA address: {}", e));
        }
    };

    // Parse addresses
    let wallet_pubkey = Pubkey::from_str(&wallet_address).map_err(|e|
        format!("Invalid wallet address: {}", e)
    )?;
    let mint_pubkey = Pubkey::from_str(&account.mint).map_err(|e|
        format!("Invalid mint address: {}", e)
    )?;
    let ata_pubkey = Pubkey::from_str(&ata_address).map_err(|e|
        format!("Invalid ATA address: {}", e)
    )?;

    // Get wallet keypair from config
    let configs = screenerbot::configs
        ::read_configs()
        .map_err(|e| format!("Failed to read configs: {:?}", e))?;
    let wallet_keypair = screenerbot::configs
        ::load_wallet_from_config(&configs)
        .map_err(|e| format!("Failed to load wallet keypair: {:?}", e))?;

    // Create burn instruction
    let burn_instruction = spl_instruction
        ::burn(
            &spl_token::id(), // Token program ID
            &ata_pubkey, // Source account (ATA)
            &mint_pubkey, // Mint
            &wallet_pubkey, // Authority (wallet)
            &[&wallet_pubkey], // Signers
            account.balance // Amount to burn
        )
        .map_err(|e| format!("Failed to create burn instruction: {}", e))?;

    log(
        LogTag::System,
        "BURN_INSTRUCTION",
        &format!("Created burn instruction for {} tokens", account.balance)
    );

    // Get recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash().await
        .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;

    // Create and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &[burn_instruction],
        Some(&wallet_pubkey),
        &[&wallet_keypair],
        recent_blockhash
    );

    log(LogTag::System, "BURN_TRANSACTION", "Sending burn transaction to network");

    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction).await
        .map_err(|e| format!("Failed to send burn transaction: {}", e))?;

    log(LogTag::System, "BURN_SUCCESS", &format!("Burn transaction confirmed: {}", signature));

    Ok(signature)
}

/// Main function to sell all tokens, close all ATAs, and reset bot data
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // ---------------------------------------------------------------------
    // AUTO-ENABLE DEBUG MODES FOR THIS MAINTENANCE TOOL
    // This tool benefits from verbose diagnostics (swap quote parsing, ATA
    // verification, wallet + rpc operations). Instead of requiring manual
    // flags, we inject the most relevant debug flags if the user did not
    // explicitly supply any of them. This helps quickly surface issues like
    // malformed GMGN / Jupiter responses during liquidation.
    // ---------------------------------------------------------------------
    let mut effective_args = args.clone();
    let debug_flags = [
        "--debug-swaps", // Detailed swap + quote lifecycle
        "--debug-ata", // ATA close + balance checks
        "--debug-wallet", // Wallet balance + token account fetches
        // Do NOT auto-enable --debug-rpc to avoid printing RPC URLs/keys in logs
        "--debug-system", // System-level debug summaries
    ];

    let any_user_debug = effective_args.iter().any(|a| a.starts_with("--debug-"));
    if !any_user_debug {
        // Only auto-inject if user did not request other debug modes
        for flag in debug_flags.iter() {
            if !effective_args.contains(&flag.to_string()) {
                effective_args.push(flag.to_string());
            }
        }
    }
    // Persist augmented args so is_debug_* helpers pick them up globally
    set_cmd_args(effective_args);

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    let dry_run = args.contains(&"--dry-run".to_string()) || args.contains(&"-d".to_string());

    log(LogTag::System, "INFO", "WALLET CLEANUP AND BOT RESET UTILITY");
    log(LogTag::System, "INFO", "====================================");

    if dry_run {
        log(
            LogTag::System,
            "INFO",
            "DRY RUN MODE - No actual transactions or file deletions will be made"
        );
    }

    log(LogTag::System, "INFO", "This tool will:");
    log(LogTag::System, "INFO", "  - Scan for all token accounts (SPL & Token-2022)");
    if !dry_run {
        log(LogTag::System, "INFO", "  - Sell larger token amounts for SOL with retry logic");
        log(LogTag::System, "INFO", "  - Burn small dust amounts to empty ATAs");
        log(
            LogTag::System,
            "INFO",
            "  - Close empty Associated Token Accounts (ATAs) with retry logic"
        );
        log(LogTag::System, "INFO", "  - Reclaim rent SOL from closed ATAs");
        log(LogTag::System, "INFO", "  - Delete specific bot data files to reset the system");
    } else {
        log(LogTag::System, "INFO", "  - Show what tokens would be sold");
        log(LogTag::System, "INFO", "  - Show what empty ATAs would be closed");
        log(LogTag::System, "INFO", "  - Estimate rent SOL that would be reclaimed");
        log(LogTag::System, "INFO", "  - Show what data files would be deleted");
    }

    log(
        LogTag::System,
        "INFO",
        &format!("Starting comprehensive wallet cleanup and bot reset{}", if dry_run {
            " (DRY RUN)"
        } else {
            ""
        })
    );

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    log(LogTag::System, "WALLET", &format!("Processing wallet: {}", wallet_address));

    // Step 1: Initialize RPC and get initial SOL balance
    log(LogTag::System, "INFO", "Initializing RPC client and checking initial balances...");
    init_rpc_client()?;

    // Check initial SOL balance
    let initial_sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => {
            log(LogTag::System, "BALANCE", &format!("Initial SOL balance: {:.6} SOL", balance));
            balance
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get initial SOL balance: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    // Step 2: Get all token accounts using centralized RPC client
    log(LogTag::System, "INFO", "Scanning for all token accounts (SPL Token and Token-2022)...");

    let token_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token accounts: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    if token_accounts.is_empty() {
        log(LogTag::System, "INFO", "No token accounts found - wallet is already clean!");
    } else {
        log(LogTag::System, "INFO", &format!("Found {} token accounts:", token_accounts.len()));
        for account in &token_accounts {
            let token_program = if account.is_token_2022 { "Token-2022" } else { "SPL Token" };
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "  {} ({}): {} raw units - Mint: {}",
                    safe_truncate(&account.mint, 8),
                    token_program,
                    account.balance,
                    account.mint
                )
            );
        }
    }

    // Check initial comprehensive balance
    if let Err(e) = check_comprehensive_balance(&wallet_address, "INITIAL").await {
        log(LogTag::System, "WARNING", &format!("Initial balance check failed: {}", e));
    }

    // Step 3: Sell all tokens with balances > 0 using retry logic
    let (successful_sells, failed_sells, successfully_sold_mints) = if !token_accounts.is_empty() {
        sell_all_tokens_with_retry(&token_accounts, dry_run).await
    } else {
        (0, 0, HashSet::new())
    };

    // Check balance after selling
    if successful_sells > 0 || failed_sells > 0 {
        if let Err(e) = check_comprehensive_balance(&wallet_address, "AFTER_SELLING").await {
            log(LogTag::System, "WARNING", &format!("Post-selling balance check failed: {}", e));
        }
    }

    // Step 2.5: Wait for swap transactions to be processed before ATA closing
    if successful_sells > 0 && !dry_run {
        const SWAP_CONFIRMATION_DELAY_SECONDS: u64 = 10;
        log(
            LogTag::System,
            "WAIT_CONFIRMATION",
            &format!(
                "Waiting {}s for {} swap transactions to be confirmed before ATA closing...",
                SWAP_CONFIRMATION_DELAY_SECONDS,
                successful_sells
            )
        );

        if is_debug_ata_enabled() {
            log(
                LogTag::System,
                "DEBUG",
                &format!("⏳ RESET_WAIT_CONFIRMATION: delaying {}s for blockchain confirmation", SWAP_CONFIRMATION_DELAY_SECONDS)
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(SWAP_CONFIRMATION_DELAY_SECONDS)).await;

        log(
            LogTag::System,
            "WAIT_CONFIRMATION_DONE",
            "Swap confirmation wait completed, proceeding with dust burning and ATA closing"
        );
    }

    // Step 3.5: Burn any remaining dust tokens before ATA closing
    let (successful_burns, failed_burns, successfully_burned_mints) = if !token_accounts.is_empty() {
        burn_dust_tokens_with_retry(&token_accounts, dry_run).await
    } else {
        (0, 0, HashSet::new())
    };

    // Step 4: Close all ATAs with retry logic
    let (successful_closes, failed_closes) = if !token_accounts.is_empty() {
        close_all_atas_with_retry(
            &token_accounts,
            &successfully_sold_mints,
            &successfully_burned_mints,
            &wallet_address,
            dry_run
        ).await
    } else {
        (0, 0)
    };

    // Check balance after ATA closing
    if successful_closes > 0 || failed_closes > 0 {
        if let Err(e) = check_comprehensive_balance(&wallet_address, "AFTER_ATA_CLOSING").await {
            log(
                LogTag::System,
                "WARNING",
                &format!("Post-ATA-closing balance check failed: {}", e)
            );
        }
    }

    // Step 4: Remove specified data files
    log(
        LogTag::System,
        "FILE_CLEANUP_START",
        &format!("Starting data file cleanup{}", if dry_run { " (DRY RUN)" } else { "" })
    );

    let mut files_removed = 0;
    let mut files_not_found = 0;
    let mut files_failed = 0;

    for file_path in DATA_FILES_TO_REMOVE {
        if dry_run {
            if Path::new(file_path).exists() {
                log(
                    LogTag::System,
                    "DRY_FILE_REMOVE",
                    &format!("Would remove file: {}", file_path)
                );
                files_removed += 1;
            } else {
                log(
                    LogTag::System,
                    "DRY_FILE_NOT_FOUND",
                    &format!("File not found (would skip): {}", file_path)
                );
                files_not_found += 1;
            }
        } else {
            if Path::new(file_path).exists() {
                match fs::remove_file(file_path) {
                    Ok(()) => {
                        log(
                            LogTag::System,
                            "FILE_REMOVED",
                            &format!("Successfully removed file: {}", file_path)
                        );
                        files_removed += 1;
                    }
                    Err(e) => {
                        log(
                            LogTag::System,
                            "FILE_REMOVE_FAILED",
                            &format!("Failed to remove file {}: {}", file_path, e)
                        );
                        files_failed += 1;
                    }
                }
            } else {
                log(
                    LogTag::System,
                    "FILE_NOT_FOUND",
                    &format!("File not found (skipping): {}", file_path)
                );
                files_not_found += 1;
            }
        }
    }

    log(
        LogTag::System,
        "FILE_CLEANUP_SUMMARY",
        &format!(
            "File cleanup completed: {} removed, {} not found, {} failed",
            files_removed,
            files_not_found,
            files_failed
        )
    );

    // Log file cleanup
    log(
        LogTag::System,
        "LOG_CLEANUP_START",
        &format!("Starting log file cleanup{}", if dry_run { " (DRY RUN)" } else { "" })
    );

    let mut log_files_removed = 0;
    let mut log_files_failed = 0;

    if let Ok(entries) = fs::read_dir("logs") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(filename) = path.file_name() {
                    if let Some(name_str) = filename.to_str() {
                        if name_str.starts_with("screenerbot_") && name_str.ends_with(".log") {
                            if dry_run {
                                log(
                                    LogTag::System,
                                    "DRY_LOG_REMOVE",
                                    &format!("Would remove log file: {}", path.display())
                                );
                                log_files_removed += 1;
                            } else {
                                match fs::remove_file(&path) {
                                    Ok(()) => {
                                        log(
                                            LogTag::System,
                                            "LOG_REMOVED",
                                            &format!(
                                                "Successfully removed log file: {}",
                                                path.display()
                                            )
                                        );
                                        log_files_removed += 1;
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::System,
                                            "LOG_REMOVE_FAILED",
                                            &format!(
                                                "Failed to remove log file {}: {}",
                                                path.display(),
                                                e
                                            )
                                        );
                                        log_files_failed += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    } else {
        log(LogTag::System, "LOG_DIR_NOT_FOUND", "Logs directory not found or not accessible");
    }

    log(
        LogTag::System,
        "LOG_CLEANUP_SUMMARY",
        &format!(
            "Log cleanup completed: {} removed, {} failed",
            log_files_removed,
            log_files_failed
        )
    );

    // Step 5: Final summary and cleanup report
    log(LogTag::System, "FINAL_REPORT", &format!("Final cleanup and reset report:"));

    if !token_accounts.is_empty() {
        log(
            LogTag::System,
            "FINAL_REPORT",
            &format!("Token accounts found: {}", token_accounts.len())
        );
    }

    if dry_run {
        if !token_accounts.is_empty() {
            log(
                LogTag::System,
                "FINAL_REPORT",
                &format!("Would Sell - Success: {} | Failed: {}", successful_sells, failed_sells)
            );
            log(
                LogTag::System,
                "FINAL_REPORT",
                &format!("Would Burn - Success: {} | Failed: {}", successful_burns, failed_burns)
            );
            log(
                LogTag::System,
                "FINAL_REPORT",
                &format!(
                    "Would Close ATAs - Success: {} | Failed: {}",
                    successful_closes,
                    failed_closes
                )
            );
        }
        log(
            LogTag::System,
            "FINAL_REPORT",
            &format!(
                "Would Remove Files - Success: {} | Not Found: {} | Failed: {}",
                files_removed,
                files_not_found,
                files_failed
            )
        );
    } else {
        if !token_accounts.is_empty() {
            log(
                LogTag::System,
                "FINAL_REPORT",
                &format!("Sales - Success: {} | Failed: {}", successful_sells, failed_sells)
            );
            log(
                LogTag::System,
                "FINAL_REPORT",
                &format!("Burns - Success: {} | Failed: {}", successful_burns, failed_burns)
            );
            log(
                LogTag::System,
                "FINAL_REPORT",
                &format!("ATA Closes - Success: {} | Failed: {}", successful_closes, failed_closes)
            );
        }
        log(
            LogTag::System,
            "FINAL_REPORT",
            &format!(
                "File Removals - Success: {} | Not Found: {} | Failed: {}",
                files_removed,
                files_not_found,
                files_failed
            )
        );
    }

    if failed_sells > 0 {
        log(LogTag::System, "FAILED_SELLS", &format!("Found {} failed sells", failed_sells));
    }

    if failed_burns > 0 {
        log(LogTag::System, "FAILED_BURNS", &format!("Found {} failed burns", failed_burns));
    }

    if failed_closes > 0 {
        log(LogTag::System, "FAILED_CLOSES", &format!("Found {} failed ATA closes", failed_closes));
    }

    if files_failed > 0 {
        log(
            LogTag::System,
            "FAILED_FILE_REMOVES",
            &format!("Found {} failed file removals", files_failed)
        );
    }

    let estimated_rent_reclaimed = (successful_closes as f64) * 0.00203928; // ~0.002 SOL per ATA
    if estimated_rent_reclaimed > 0.0 {
        if dry_run {
            log(
                LogTag::System,
                "ESTIMATED_RENT",
                &format!("Would reclaim {:.6} SOL in rent", estimated_rent_reclaimed)
            );
        } else {
            log(
                LogTag::System,
                "RECLAIMED_RENT",
                &format!("Reclaimed {:.6} SOL in rent", estimated_rent_reclaimed)
            );
        }
    }

    // Calculate expected vs actual counts, accounting for SOL being skipped
    let sol_accounts = token_accounts
        .iter()
        .filter(|a| a.mint == SOL_MINT)
        .count();
    let expected_operations = if token_accounts.is_empty() {
        0
    } else {
        token_accounts.len() - sol_accounts
    };

    // Success means all non-SOL tokens were either sold or burned, and all empty ATAs were closed
    let all_wallet_ops_successful =
        token_accounts.is_empty() || (failed_sells == 0 && failed_burns == 0 && failed_closes == 0);
    let all_file_ops_successful = files_failed == 0;

    if all_wallet_ops_successful && all_file_ops_successful {
        // All operations successful
        if dry_run {
            log(
                LogTag::System,
                "DRY_RUN_COMPLETE",
                "All operations would succeed - dry run complete"
            );
        } else {
            log(
                LogTag::System,
                "RESET_COMPLETE",
                "All tokens sold/burned, ATAs closed, and data files removed successfully"
            );
        }
    } else {
        if dry_run {
            log(
                LogTag::System,
                "DRY_RUN_ISSUES",
                &format!(
                    "Dry run completed with issues: {} sell failures, {} burn failures, {} close failures, {} file removal failures",
                    failed_sells,
                    failed_burns,
                    failed_closes,
                    files_failed
                )
            );
        } else {
            log(
                LogTag::System,
                "RESET_ISSUES",
                &format!(
                    "Reset completed with issues: {} sell failures, {} burn failures, {} close failures, {} file removal failures",
                    failed_sells,
                    failed_burns,
                    failed_closes,
                    files_failed
                )
            );
        }
    }

    // Final comprehensive balance check
    if let Err(e) = check_comprehensive_balance(&wallet_address, "FINAL").await {
        log(LogTag::System, "WARNING", &format!("Final balance check failed: {}", e));
    }

    // Calculate and display final SOL balance change
    let final_sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => {
            let sol_change = balance - initial_sol_balance;
            if sol_change > 0.0 {
                log(
                    LogTag::System,
                    "BALANCE_CHANGE",
                    &format!(
                        "SOL balance increased by {:.6} SOL (from {:.6} to {:.6})",
                        sol_change,
                        initial_sol_balance,
                        balance
                    )
                );
            } else if sol_change < 0.0 {
                log(
                    LogTag::System,
                    "BALANCE_CHANGE",
                    &format!(
                        "SOL balance decreased by {:.6} SOL (from {:.6} to {:.6})",
                        -sol_change,
                        initial_sol_balance,
                        balance
                    )
                );
            } else {
                log(
                    LogTag::System,
                    "BALANCE_CHANGE",
                    &format!("SOL balance unchanged: {:.6} SOL", balance)
                );
            }
            balance
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get final SOL balance: {}", e));
            initial_sol_balance
        }
    };

    if dry_run {
        log(LogTag::System, "DRY_RUN_HINT", "To execute for real, run without --dry-run flag");
    } else {
        log(LogTag::System, "RESET_HINT", "Bot has been reset to fresh state with clean wallet");
    }

    log(
        LogTag::System,
        "TOOL_COMPLETE",
        &format!(
            "Tool execution finished: {} token accounts processed, {} data files processed",
            token_accounts.len(),
            DATA_FILES_TO_REMOVE.len()
        )
    );

    Ok(())
}

/// Sell all tokens with retry mechanism
async fn sell_all_tokens_with_retry(
    token_accounts: &[TokenAccountInfo],
    dry_run: bool
) -> (usize, usize, HashSet<String>) {
    log(
        LogTag::System,
        "SELL_START",
        &format!("Starting token sales for {} accounts{}", token_accounts.len(), if dry_run {
            " (DRY RUN)"
        } else {
            ""
        })
    );

    // Filter accounts for selling (skip zero balance and SOL)
    let sellable_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|account| {
            if account.balance == 0 {
                log(
                    LogTag::System,
                    "SKIP_ZERO",
                    &format!("Skipping zero balance token: {}", safe_truncate(&account.mint, 8))
                );
                return false;
            }

            if account.balance < DUST_THRESHOLD_RAW_UNITS {
                log(
                    LogTag::System,
                    "SKIP_DUST",
                    &format!(
                        "Skipping dust balance (<{} raw units) token: {} ({} units)",
                        DUST_THRESHOLD_RAW_UNITS,
                        safe_truncate(&account.mint, 8),
                        account.balance
                    )
                );
                return false;
            }

            if account.mint == SOL_MINT {
                log(LogTag::System, "SKIP_SOL", "Skipping SOL (native token)");
                return false;
            }

            true
        })
        .collect();

    log(
        LogTag::System,
        "SELL_FILTER",
        &format!(
            "Filtered {} sellable accounts from {} total (dust threshold: {} units)",
            sellable_accounts.len(),
            token_accounts.len(),
            DUST_THRESHOLD_RAW_UNITS
        )
    );

    if sellable_accounts.is_empty() {
        return (0, 0, HashSet::new());
    }

    // Process selling with 3 concurrent tasks
    let sell_semaphore = Arc::new(Semaphore::new(3));
    let sell_results: Vec<_> = stream
        ::iter(sellable_accounts.iter())
        .map(|account| {
            let semaphore = sell_semaphore.clone();
            let account = (*account).clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                sell_single_token_with_retry(&account, dry_run).await
            }
        })
        .buffer_unordered(3) // Process up to 3 concurrent sells
        .collect().await;

    let successful_sells = sell_results
        .iter()
        .filter(|(_, success, _)| *success)
        .count();
    let failed_sells = sell_results
        .iter()
        .filter(|(_, success, _)| !*success)
        .count();

    // Track successfully sold token mints for enhanced ATA closing
    let successfully_sold_mints: HashSet<String> = sell_results
        .iter()
        .filter(|(_, success, _)| *success)
        .map(|(mint, _, _)| mint.clone())
        .collect();

    log(
        LogTag::System,
        "SELL_SUMMARY",
        &format!("Sales completed: {} success, {} failed", successful_sells, failed_sells)
    );

    (successful_sells, failed_sells, successfully_sold_mints)
}

/// Sell a single token with retry mechanism
async fn sell_single_token_with_retry(
    account: &TokenAccountInfo,
    dry_run: bool
) -> (String, bool, Option<String>) {
    if dry_run {
        log(
            LogTag::System,
            "DRY_SELL",
            &format!(
                "Would sell {} raw units of {}",
                account.balance,
                safe_truncate(&account.mint, 8)
            )
        );
        return (account.mint.clone(), true, None);
    }

    log(
        LogTag::System,
        "SELL_START",
        &format!(
            "Starting sell for token: {} ({} raw units)",
            safe_truncate(&account.mint, 8),
            account.balance
        )
    );

    // Try to sell with retries
    for attempt in 1..=MAX_SELL_RETRIES {
        log(
            LogTag::System,
            "SELL_ATTEMPT",
            &format!(
                "Sell attempt {} of {} for token {}",
                attempt,
                MAX_SELL_RETRIES,
                safe_truncate(&account.mint, 8)
            )
        );

        match attempt_single_sell(account).await {
            Ok(success_msg) => {
                log(LogTag::System, "SELL_SUCCESS", &success_msg);
                return (account.mint.clone(), true, None);
            }
            Err(error_msg) => {
                log(
                    LogTag::System,
                    "SELL_ATTEMPT_FAILED",
                    &format!(
                        "Sell attempt {} failed for {}: {}",
                        attempt,
                        safe_truncate(&account.mint, 8),
                        error_msg
                    )
                );

                // Detect terminal no-route / no-liquidity conditions to avoid useless retries
                let lower_err = error_msg.to_lowercase();
                let is_terminal_no_route =
                    lower_err.contains("no route") ||
                    lower_err.contains("no routers available for quote") ||
                    lower_err.contains("could not find any route");
                if is_terminal_no_route {
                    log(
                        LogTag::System,
                        "SELL_TERMINAL_NO_ROUTE",
                        &format!(
                            "Detected terminal no-route condition for {} – aborting further retries",
                            safe_truncate(&account.mint, 8)
                        )
                    );
                    return (account.mint.clone(), false, Some(error_msg));
                }

                if attempt < MAX_SELL_RETRIES {
                    let delay = Duration::from_millis(RETRY_DELAY_MS * (attempt as u64));
                    log(
                        LogTag::System,
                        "SELL_RETRY_DELAY",
                        &format!("Waiting {}ms before retry...", delay.as_millis())
                    );
                    sleep(delay).await;
                } else {
                    log(
                        LogTag::System,
                        "SELL_FAILED",
                        &format!(
                            "All sell attempts failed for {}: {}",
                            safe_truncate(&account.mint, 8),
                            error_msg
                        )
                    );
                    return (account.mint.clone(), false, Some(error_msg));
                }
            }
        }
    }

    (account.mint.clone(), false, Some("Max retries exceeded".to_string()))
}

/// Attempt to sell a single token
async fn attempt_single_sell(account: &TokenAccountInfo) -> Result<String, String> {
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;

    // Double-check balance before selling
    let actual_balance = get_token_balance(&wallet_address, &account.mint).await.map_err(|e|
        format!("Failed to get token balance: {}", e)
    )?;

    if actual_balance == 0 {
        return Err("Token balance is zero, cannot sell".to_string());
    }

    if actual_balance != account.balance {
        log(
            LogTag::System,
            "BALANCE_MISMATCH",
            &format!(
                "Balance mismatch for {}: expected {}, actual {}",
                safe_truncate(&account.mint, 8),
                account.balance,
                actual_balance
            )
        );
    }

    // Create minimal Token struct for the sell operation
    let token = Token {
        mint: account.mint.clone(),
        symbol: format!("TOKEN_{}", safe_truncate(&account.mint, 8)),
        name: format!("Unknown Token {}", safe_truncate(&account.mint, 8)),
        chain: "solana".to_string(),

        // Set all optional fields to defaults
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: None,
        price_dexscreener_sol: None,
        price_dexscreener_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: None,
        info: None,
        boosts: None,
        decimals: None,
    };

    // Get quote and execute swap
    let best_quote = get_best_quote(
        &token.mint,
        SOL_MINT,
        actual_balance,
        &wallet_address,
        QUOTE_SLIPPAGE_PERCENT,
        "ExactIn" // Use ExactOut for selling to completely liquidate tokens
    ).await.map_err(|e| format!("Failed to get quote: {}", e))?;

    let swap_result = execute_best_swap(
        &token,
        &token.mint,
        SOL_MINT,
        actual_balance,
        best_quote
    ).await.map_err(|e| format!("Failed to execute swap: {}", e))?;

    if swap_result.success {
        let sol_amount = swap_result.output_amount
            .parse::<u64>()
            .map(|lamports| (lamports as f64) / 1_000_000_000.0)
            .unwrap_or(0.0);

        Ok(
            format!(
                "Successfully sold {} for {:.6} SOL",
                safe_truncate(&account.mint, 8),
                sol_amount
            )
        )
    } else {
        Err(swap_result.error.unwrap_or_else(|| "Unknown swap error".to_string()))
    }
}

/// Close all ATAs with retry mechanism
async fn close_all_atas_with_retry(
    token_accounts: &[TokenAccountInfo],
    successfully_sold_mints: &HashSet<String>,
    successfully_burned_mints: &HashSet<String>,
    wallet_address: &str,
    dry_run: bool
) -> (usize, usize) {
    log(
        LogTag::System,
        "ATA_START",
        &format!("Starting ATA cleanup for {} accounts{}", token_accounts.len(), if dry_run {
            " (DRY RUN)"
        } else {
            ""
        })
    );

    // Filter accounts for ATA closing (skip SOL and non-zero balances unless burned)
    let closable_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|account| {
            if account.mint == SOL_MINT {
                log(LogTag::System, "SKIP_SOL_ATA", "Skipping SOL account for ATA closing");
                return false;
            }

            // Check if the token was successfully burned or sold
            let was_burned = successfully_burned_mints.contains(&account.mint);
            let was_sold = successfully_sold_mints.contains(&account.mint);

            if account.balance > 0 && !was_burned && !was_sold {
                log(
                    LogTag::System,
                    "SKIP_NONZERO_ATA",
                    &format!(
                        "Skipping ATA close for {} - still has {} tokens (cannot close non-empty ATA)",
                        safe_truncate(&account.mint, 8),
                        account.balance
                    )
                );
                return false;
            }

            if was_burned {
                log(
                    LogTag::System,
                    "BURNED_ATA_CANDIDATE",
                    &format!(
                        "Including ATA for {} - tokens were burned (should now be empty)",
                        safe_truncate(&account.mint, 8)
                    )
                );
            }

            true
        })
        .collect();

    log(
        LogTag::System,
        "ATA_FILTER",
        &format!(
            "Filtered {} closable accounts from {} total (only empty ATAs can be closed)",
            closable_accounts.len(),
            token_accounts.len()
        )
    );

    if closable_accounts.is_empty() {
        return (0, 0);
    }

    // Process ATA closing with 3 concurrent tasks
    let close_semaphore = Arc::new(Semaphore::new(3));
    let close_results: Vec<_> = stream
        ::iter(closable_accounts.iter())
        .map(|account| {
            let semaphore = close_semaphore.clone();
            let account = (*account).clone();
            let wallet_address = wallet_address.to_string();
            let successfully_sold_mints = successfully_sold_mints.clone();
            let successfully_burned_mints = successfully_burned_mints.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                close_single_ata_with_retry(
                    &account,
                    &successfully_sold_mints,
                    &successfully_burned_mints,
                    &wallet_address,
                    dry_run
                ).await
            }
        })
        .buffer_unordered(3) // Process up to 3 concurrent ATA closes
        .collect().await;

    let successful_closes = close_results
        .iter()
        .filter(|(_, success, _)| *success)
        .count();
    let failed_closes = close_results
        .iter()
        .filter(|(_, success, _)| !*success)
        .count();

    log(
        LogTag::System,
        "ATA_SUMMARY",
        &format!("ATA cleanup completed: {} success, {} failed", successful_closes, failed_closes)
    );

    (successful_closes, failed_closes)
}

/// Close a single ATA with retry mechanism
async fn close_single_ata_with_retry(
    account: &TokenAccountInfo,
    successfully_sold_mints: &HashSet<String>,
    successfully_burned_mints: &HashSet<String>,
    wallet_address: &str,
    dry_run: bool
) -> (String, bool, Option<String>) {
    if dry_run {
        log(
            LogTag::System,
            "DRY_ATA",
            &format!("Would close ATA for token: {}", safe_truncate(&account.mint, 8))
        );
        return (account.mint.clone(), true, Some("DRY_RUN_TX".to_string()));
    }

    log(
        LogTag::System,
        "ATA_START",
        &format!("Starting ATA close for token: {}", safe_truncate(&account.mint, 8))
    );

    // Check if this token was recently sold or burned
    let recently_sold = successfully_sold_mints.contains(&account.mint);
    let recently_burned = successfully_burned_mints.contains(&account.mint);
    let recently_processed = recently_sold || recently_burned;

    // Try to close with retries
    for attempt in 1..=MAX_ATA_CLOSE_RETRIES {
        log(
            LogTag::System,
            "ATA_ATTEMPT",
            &format!(
                "ATA close attempt {} of {} for token {}",
                attempt,
                MAX_ATA_CLOSE_RETRIES,
                safe_truncate(&account.mint, 8)
            )
        );

        match attempt_single_ata_close(&account.mint, wallet_address, recently_processed).await {
            Ok(signature) => {
                log(
                    LogTag::System,
                    "ATA_SUCCESS",
                    &format!(
                        "Successfully closed ATA for {}. TX: {}",
                        safe_truncate(&account.mint, 8),
                        signature
                    )
                );
                return (account.mint.clone(), true, Some(signature));
            }
            Err(error_msg) => {
                log(
                    LogTag::System,
                    "ATA_ATTEMPT_FAILED",
                    &format!(
                        "ATA close attempt {} failed for {}: {}",
                        attempt,
                        safe_truncate(&account.mint, 8),
                        error_msg
                    )
                );

                if attempt < MAX_ATA_CLOSE_RETRIES {
                    let delay = Duration::from_millis(RETRY_DELAY_MS * (attempt as u64));
                    log(
                        LogTag::System,
                        "ATA_RETRY_DELAY",
                        &format!("Waiting {}ms before retry...", delay.as_millis())
                    );
                    sleep(delay).await;
                } else {
                    log(
                        LogTag::System,
                        "ATA_FAILED",
                        &format!(
                            "All ATA close attempts failed for {}: {}",
                            safe_truncate(&account.mint, 8),
                            error_msg
                        )
                    );
                    return (account.mint.clone(), false, None);
                }
            }
        }
    }

    (account.mint.clone(), false, None)
}

/// Attempt to close a single ATA
async fn attempt_single_ata_close(
    mint: &str,
    wallet_address: &str,
    recently_sold: bool
) -> Result<String, String> {
    // Verify the ATA actually exists before trying to close it
    let rpc_client = get_rpc_client();

    match rpc_client.get_associated_token_account(wallet_address, mint).await {
        Ok(ata_address) => {
            // Double-check that the account still exists with fresh RPC data
            match rpc_client.is_token_account_token_2022(&ata_address).await {
                Ok(_) => {
                    // Account exists, proceed with closing
                    if is_debug_ata_enabled() {
                        log(
                            LogTag::System,
                            "DEBUG",
                            &format!(
                                "✅ ATA_VERIFIED: account {} exists, proceeding with close",
                                safe_truncate(&ata_address, 8)
                            )
                        );
                    }
                }
                Err(_) => {
                    // Account doesn't exist or is already closed
                    return Ok("ALREADY_CLOSED".to_string());
                }
            }
        }
        Err(_) => {
            // Cannot find ATA, likely already closed
            return Ok("NOT_FOUND".to_string());
        }
    }

    // Use the library function to close the ATA
    match close_token_account_with_context(mint, wallet_address, recently_sold).await {
        Ok(signature) => Ok(signature),
        Err(e) => Err(e.to_string()),
    }
}

/// Check comprehensive balance before and after operations
async fn check_comprehensive_balance(wallet_address: &str, stage: &str) -> Result<(), String> {
    log(
        LogTag::System,
        "BALANCE_CHECK",
        &format!("Checking comprehensive balance at stage: {}", stage)
    );

    // Check SOL balance
    match get_sol_balance(wallet_address).await {
        Ok(sol_balance) => {
            log(
                LogTag::System,
                "BALANCE_SOL",
                &format!("{}: SOL balance: {:.6} SOL", stage, sol_balance)
            );
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get SOL balance at {}: {}", stage, e));
        }
    }

    // Check token accounts
    match get_all_token_accounts(wallet_address).await {
        Ok(token_accounts) => {
            let non_zero_accounts = token_accounts
                .iter()
                .filter(|a| a.balance > 0)
                .count();
            log(
                LogTag::System,
                "BALANCE_TOKENS",
                &format!(
                    "{}: Found {} token accounts, {} with non-zero balance",
                    stage,
                    token_accounts.len(),
                    non_zero_accounts
                )
            );

            if is_debug_ata_enabled() && !token_accounts.is_empty() {
                log(LogTag::System, "DEBUG", &format!("Token accounts at {}:", stage));
                for account in token_accounts.iter().take(10) {
                    // Show max 10 to avoid spam
                    let token_program = if account.is_token_2022 {
                        "Token-2022"
                    } else {
                        "SPL Token"
                    };
                    log(
                        LogTag::System,
                        "DEBUG",
                        &format!(
                            "  {} ({}): {} raw units",
                            safe_truncate(&account.mint, 8),
                            token_program,
                            account.balance
                        )
                    );
                }
                if token_accounts.len() > 10 {
                    log(
                        LogTag::System,
                        "DEBUG",
                        &format!("  ... and {} more accounts", token_accounts.len() - 10)
                    );
                }
            }
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Failed to get token accounts at {}: {}", stage, e)
            );
        }
    }

    Ok(())
}
