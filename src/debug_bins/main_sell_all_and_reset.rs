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
//! Requires `config.toml` with wallet private key and RPC endpoints.
//!
//! ## Warning
//! This tool will attempt to sell ALL tokens in your wallet AND delete specific bot data files. Use with caution!

use futures::stream::{self, StreamExt};
use screenerbot::arguments::set_cmd_args;
use screenerbot::config::with_config;
use screenerbot::constants::SOL_MINT;
use screenerbot::errors::ScreenerBotError;
use screenerbot::logger::{self as logger, LogTag};
use screenerbot::rpc::{get_rpc_client, init_rpc_client, TokenAccountInfo};
use screenerbot::swaps::{execute_best_swap, get_best_quote};
use screenerbot::tokens::Token;
use screenerbot::utils::{
    close_token_account_with_context, get_all_token_accounts, get_sol_balance, get_token_balance,
    get_wallet_address,
};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Signature, Signer},
    signer::keypair::Keypair,
    transaction::Transaction,
};
use spl_token::instruction as spl_instruction;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

/// Print comprehensive help menu for the Sell All and Reset Tool
fn print_help() {
    logger::info(
        LogTag::System,
        "🔄 Sell All Tokens, Close ATAs, and Reset Bot Data Tool",
    );
    logger::info(
        LogTag::System,
        "======================================================",
    );
    logger::info(
        LogTag::System,
        "Comprehensive wallet cleanup and bot reset utility that sells all tokens for SOL,",
    );
    logger::info(
        LogTag::System,
        "closes all Associated Token Accounts (ATAs), and resets bot data files.",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "⚠️  WARNING: This tool will:");
    logger::info(
        LogTag::System,
        "    • Sell ALL tokens in your wallet for SOL",
    );
    logger::info(
        LogTag::System,
        "    • Close empty Associated Token Accounts (ATAs)",
    );
    logger::info(
        LogTag::System,
        "    • DELETE specific bot data files (irreversible)",
    );
    logger::info(
        LogTag::System,
        "    Use with extreme caution and understand the risks involved.",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "USAGE:");
    logger::info(
        LogTag::System,
        "    cargo run --bin tool_sell_all_and_reset [OPTIONS]",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "OPTIONS:");
    logger::info(
        LogTag::System,
        "    --help, -h          Show this help message",
    );
    logger::info(
        LogTag::System,
        "    --dry-run, -d      Simulate operations without executing transactions",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "EXAMPLES:");
    logger::info(
        LogTag::System,
        "    # Simulate cleanup to see what would happen",
    );
    logger::info(
        LogTag::System,
        "    cargo run --bin tool_sell_all_and_reset -- --dry-run",
    );
    logger::info(LogTag::System, "");
    logger::info(
        LogTag::System,
        "    # Execute full wallet cleanup and bot reset (DANGEROUS)",
    );
    logger::info(
        LogTag::System,
        "    cargo run --bin tool_sell_all_and_reset",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "OPERATIONS PERFORMED:");
    logger::info(
        LogTag::System,
        "    1. Scan wallet for all SPL Token and Token-2022 accounts",
    );
    logger::info(
        LogTag::System,
        "    2. Identify tokens with non-zero balances",
    );
    logger::info(
        LogTag::System,
        "    3. Sell larger token amounts for SOL using swap service",
    );
    logger::info(
        LogTag::System,
        "    4. Burn small dust amounts (<1000 raw units) to empty ATAs",
    );
    logger::info(
        LogTag::System,
        "    5. Close only EMPTY Associated Token Accounts (zero balance)",
    );
    logger::info(
        LogTag::System,
        "    6. Reclaim rent SOL from closed ATAs (~0.00203928 SOL each)",
    );
    logger::info(
        LogTag::System,
        "    7. Delete specific bot data files to reset the system",
    );
    logger::info(
        LogTag::System,
        "    8. Clean up all bot log files from logs/ directory",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "DATA FILES THAT WILL BE DELETED:");
    logger::info(LogTag::System, "    • data/rpc_stats.json (RPC statistics)");
    logger::info(
        LogTag::System,
        "    • data/ata_failed_cache.json (failed ATA cache)",
    );
    logger::info(
        LogTag::System,
        "    • data/positions.db (trading positions database)",
    );
    logger::info(
        LogTag::System,
        "    • data/events.db (events system database)",
    );
    logger::info(
        LogTag::System,
        "    • data/events.db-shm (events DB shared memory)",
    );
    logger::info(
        LogTag::System,
        "    • data/events.db-wal (events DB write-ahead log)",
    );
    logger::info(
        LogTag::System,
        "    • logs/screenerbot_*.log (all bot log files)",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "FILES THAT WILL BE PRESERVED:");
    logger::info(
        LogTag::System,
        "    • data/config.toml (wallet keys and RPC endpoints)",
    );
    logger::info(LogTag::System, "    • data/tokens.db (token database)");
    logger::info(
        LogTag::System,
        "    • data/decimal_cache.json (token decimals cache)",
    );
    logger::info(
        LogTag::System,
        "    • data/token_blacklist.json (blacklisted tokens)",
    );
    logger::info(
        LogTag::System,
        "    • data/wallet_transactions_stats.json (wallet sync data)",
    );
    logger::info(
        LogTag::System,
        "    • data/cache_ohlcvs/ (OHLCV data cache)",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "SAFETY FEATURES:");
    logger::info(
        LogTag::System,
        "    • Skips SOL (native token) - cannot sell SOL for SOL",
    );
    logger::info(
        LogTag::System,
        "    • Validates token balances before attempting sales",
    );
    logger::info(
        LogTag::System,
        "    • Only closes ATAs with zero balance (cannot close non-empty ATAs)",
    );
    logger::info(
        LogTag::System,
        "    • Detailed progress reporting for each operation",
    );
    logger::info(
        LogTag::System,
        "    • Graceful error handling for failed transactions",
    );
    logger::info(
        LogTag::System,
        "    • Supports both SPL Token and Token-2022 programs",
    );
    logger::info(
        LogTag::System,
        "    • Concurrent processing with rate limiting",
    );
    logger::info(
        LogTag::System,
        "    • Only removes specific data files, preserves configuration",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "ESTIMATED OUTCOMES:");
    logger::info(
        LogTag::System,
        "    • SOL received from token sales (varies by token values)",
    );
    logger::info(LogTag::System, "    • Rent SOL reclaimed from closed ATAs");
    logger::info(LogTag::System, "    • Clean wallet with only SOL remaining");
    logger::info(
        LogTag::System,
        "    • Fresh bot state with preserved configuration",
    );
    logger::info(LogTag::System, "");
    logger::info(LogTag::System, "RISK WARNINGS:");
    logger::info(
        LogTag::System,
        "    • Irreversible operation - tokens will be permanently sold",
    );
    logger::info(
        LogTag::System,
        "    • Bot data files will be permanently deleted",
    );
    logger::info(
        LogTag::System,
        "    • Market slippage may result in lower SOL amounts",
    );
    logger::info(
        LogTag::System,
        "    • Some tokens may fail to sell due to liquidity issues",
    );
    logger::info(
        LogTag::System,
        "    • Failed transactions still consume transaction fees",
    );
    logger::info(
        LogTag::System,
        "    • Use --dry-run first to understand the impact",
    );
    logger::info(LogTag::System, "");
}

/// Get list of data files to be removed during reset
fn get_data_files_to_remove() -> Vec<std::path::PathBuf> {
    use screenerbot::paths;

    let mut files = Vec::new();

    // Cache files
    files.push(paths::get_rpc_stats_path());
    files.push(paths::get_ata_failed_cache_path());

    // Database files with WAL and SHM
    files.extend(paths::get_db_with_wal_files(paths::get_positions_db_path()));
    files.extend(paths::get_db_with_wal_files(paths::get_events_db_path()));

    // Uncomment to also remove:
    // files.extend(paths::get_db_with_wal_files(paths::get_transactions_db_path()));
    // files.extend(paths::get_db_with_wal_files(paths::get_wallet_db_path()));
    // files.extend(paths::get_db_with_wal_files(paths::get_ohlcvs_db_path()));
    // files.extend(paths::get_db_with_wal_files(paths::get_pools_db_path()));
    // files.extend(paths::get_db_with_wal_files(paths::get_strategies_db_path()));

    files
}

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
    dry_run: bool,
) -> (usize, usize, HashSet<String>) {
    logger::info(
        LogTag::System,
        &format!(
            "Starting dust token burning for {} accounts{}",
            token_accounts.len(),
            if dry_run { " (DRY RUN)" } else { "" }
        ),
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
                logger::info(
                    LogTag::System,
                    &format!(
                        "Found dust token for burning: {} ({} raw units)",
                        &account.mint, account.balance
                    ),
                );
                return true;
            }

            false
        })
        .collect();

    logger::info(
        LogTag::System,
        &format!(
            "Filtered {} burnable accounts from {} total (burn threshold: {} units)",
            burnable_accounts.len(),
            token_accounts.len(),
            BURN_THRESHOLD_RAW_UNITS
        ),
    );

    if burnable_accounts.is_empty() {
        return (0, 0, HashSet::new());
    }

    if dry_run {
        logger::info(
            LogTag::System,
            &format!("Would burn {} dust tokens", burnable_accounts.len()),
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
        logger::info(
            LogTag::System,
            &format!(
                "Attempting to burn {} raw units of {}",
                account.balance, &account.mint
            ),
        );

        // For simplicity, we'll use a "burn" by transferring to the null address
        // This is not a real burn but achieves the same goal of emptying the ATA
        match burn_single_token_amount(account).await {
            Ok(signature) => {
                logger::info(
                    LogTag::System,
                    &format!(
                        "Successfully burned dust amount for {}. TX: {}",
                        &account.mint, signature
                    ),
                );
                successful_burns += 1;
                successfully_burned_mints.insert(account.mint.clone());
            }
            Err(error_msg) => {
                logger::info(
                    LogTag::System,
                    &format!(
                        "Failed to burn dust amount for {}: {}",
                        &account.mint, error_msg
                    ),
                );
                failed_burns += 1;
            }
        }
    }

    logger::info(
        LogTag::System,
        &format!(
            "Burn completed: {} success, {} failed",
            successful_burns, failed_burns
        ),
    );

    (successful_burns, failed_burns, successfully_burned_mints)
}

/// Burn a single token's small amount using SPL Token burn instruction
async fn burn_single_token_amount(account: &TokenAccountInfo) -> Result<String, String> {
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;
    let rpc_client = get_rpc_client();

    logger::info(
        LogTag::System,
        &format!(
            "Starting burn of {} raw units for token {}",
            account.balance, &account.mint
        ),
    );

    // Get the associated token account address
    let ata_address = match rpc_client
        .get_associated_token_account(&wallet_address, &account.mint)
        .await
    {
        Ok(addr) => addr,
        Err(e) => {
            return Err(format!("Failed to get ATA address: {}", e));
        }
    };

    // Parse addresses
    let wallet_pubkey =
        Pubkey::from_str(&wallet_address).map_err(|e| format!("Invalid wallet address: {}", e))?;
    let mint_pubkey =
        Pubkey::from_str(&account.mint).map_err(|e| format!("Invalid mint address: {}", e))?;
    let ata_pubkey =
        Pubkey::from_str(&ata_address).map_err(|e| format!("Invalid ATA address: {}", e))?;

    // Get wallet keypair from config
    let wallet_keypair = screenerbot::config::get_wallet_keypair()
        .map_err(|e| format!("Failed to load wallet keypair: {}", e))?;

    // Create burn instruction
    let burn_instruction = spl_instruction::burn(
        &spl_token::id(),  // Token program ID
        &ata_pubkey,       // Source account (ATA)
        &mint_pubkey,      // Mint
        &wallet_pubkey,    // Authority (wallet)
        &[&wallet_pubkey], // Signers
        account.balance,   // Amount to burn
    )
    .map_err(|e| format!("Failed to create burn instruction: {}", e))?;

    logger::info(
        LogTag::System,
        &format!("Created burn instruction for {} tokens", account.balance),
    );

    // Get recent blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .map_err(|e| format!("Failed to get recent blockhash: {}", e))?;

    // Create and sign transaction
    let transaction = Transaction::new_signed_with_payer(
        &[burn_instruction],
        Some(&wallet_pubkey),
        &[&wallet_keypair],
        recent_blockhash,
    );

    logger::info(LogTag::System, "Sending burn transaction to network");

    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_signed_transaction(&transaction)
        .await
        .map_err(|e| format!("Failed to send burn transaction: {}", e))?;

    logger::info(
        LogTag::System,
        &format!("Burn transaction confirmed: {}", signature),
    );

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
        "--debug-swaps",  // Detailed swap + quote lifecycle
        "--debug-ata",    // ATA close + balance checks
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

    logger::info(LogTag::System, "WALLET CLEANUP AND BOT RESET UTILITY");
    logger::info(LogTag::System, "====================================");

    if dry_run {
        logger::info(
            LogTag::System,
            "DRY RUN MODE - No actual transactions or file deletions will be made",
        );
    }

    logger::info(LogTag::System, "This tool will:");
    logger::info(
        LogTag::System,
        "  - Scan for all token accounts (SPL & Token-2022)",
    );
    if !dry_run {
        logger::info(
            LogTag::System,
            "  - Sell larger token amounts for SOL with retry logic",
        );
        logger::info(LogTag::System, "  - Burn small dust amounts to empty ATAs");
        logger::info(
            LogTag::System,
            "  - Close empty Associated Token Accounts (ATAs) with retry logic",
        );
        logger::info(LogTag::System, "  - Reclaim rent SOL from closed ATAs");
        logger::info(
            LogTag::System,
            "  - Delete specific bot data files to reset the system",
        );
    } else {
        logger::info(LogTag::System, "  - Show what tokens would be sold");
        logger::info(LogTag::System, "  - Show what empty ATAs would be closed");
        logger::info(
            LogTag::System,
            "  - Estimate rent SOL that would be reclaimed",
        );
        logger::info(LogTag::System, "  - Show what data files would be deleted");
    }

    logger::info(
        LogTag::System,
        &format!(
            "Starting comprehensive wallet cleanup and bot reset{}",
            if dry_run { " (DRY RUN)" } else { "" }
        ),
    );

    // Initialize configuration
    logger::info(LogTag::System, "Loading configuration...");
    if let Err(e) = screenerbot::config::load_config() {
        logger::info(
            LogTag::System,
            &format!("Failed to load configuration: {}", e),
        );
        std::process::exit(1);
    }

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            logger::info(
                LogTag::System,
                &format!("Failed to get wallet address: {}", e),
            );
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    logger::info(
        LogTag::System,
        &format!("Processing wallet: {}", wallet_address),
    );

    // Step 1: Initialize RPC and get initial SOL balance
    logger::info(
        LogTag::System,
        "Initializing RPC client and checking initial balances...",
    );
    init_rpc_client()?;

    // Check initial SOL balance
    let initial_sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => {
            logger::info(
                LogTag::System,
                &format!("Initial SOL balance: {:.6} SOL", balance),
            );
            balance
        }
        Err(e) => {
            logger::info(
                LogTag::System,
                &format!("Failed to get initial SOL balance: {}", e),
            );
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    // Step 2: Get all token accounts using centralized RPC client
    logger::info(
        LogTag::System,
        "Scanning for all token accounts (SPL Token and Token-2022)...",
    );

    let token_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            logger::info(
                LogTag::System,
                &format!("Failed to get token accounts: {}", e),
            );
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    if token_accounts.is_empty() {
        logger::info(
            LogTag::System,
            "No token accounts found - wallet is already clean!",
        );
    } else {
        logger::info(
            LogTag::System,
            &format!("Found {} token accounts:", token_accounts.len()),
        );
        for account in &token_accounts {
            let token_program = if account.is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            };
            logger::info(
                LogTag::System,
                &format!(
                    "  {} ({}): {} raw units - Mint: {}",
                    &account.mint, token_program, account.balance, account.mint
                ),
            );
        }
    }

    // Check initial comprehensive balance
    if let Err(e) = check_comprehensive_balance(&wallet_address, "INITIAL").await {
        logger::info(
            LogTag::System,
            &format!("Initial balance check failed: {}", e),
        );
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
            logger::info(
                LogTag::System,
                &format!("Post-selling balance check failed: {}", e),
            );
        }
    }

    // Step 2.5: Wait for swap transactions to be processed before ATA closing
    if successful_sells > 0 && !dry_run {
        const SWAP_CONFIRMATION_DELAY_SECONDS: u64 = 10;
        logger::info(
            LogTag::System,
            &format!(
                "Waiting {}s for {} swap transactions to be confirmed before ATA closing...",
                SWAP_CONFIRMATION_DELAY_SECONDS, successful_sells
            ),
        );

        logger::info(
            LogTag::System,
            &format!(
                "⏳ RESET_WAIT_CONFIRMATION: delaying {}s for blockchain confirmation",
                SWAP_CONFIRMATION_DELAY_SECONDS
            ),
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(
            SWAP_CONFIRMATION_DELAY_SECONDS,
        ))
        .await;

        logger::info(
            LogTag::System,
            "Swap confirmation wait completed, proceeding with dust burning and ATA closing",
        );
    }

    // Step 3.5: Burn any remaining dust tokens before ATA closing
    let (successful_burns, failed_burns, successfully_burned_mints) = if !token_accounts.is_empty()
    {
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
            dry_run,
        )
        .await
    } else {
        (0, 0)
    };

    // Check balance after ATA closing
    if successful_closes > 0 || failed_closes > 0 {
        if let Err(e) = check_comprehensive_balance(&wallet_address, "AFTER_ATA_CLOSING").await {
            logger::info(
                LogTag::System,
                &format!("Post-ATA-closing balance check failed: {}", e),
            );
        }
    }

    // Step 4: Remove specified data files
    logger::info(
        LogTag::System,
        &format!(
            "Starting data file cleanup{}",
            if dry_run { " (DRY RUN)" } else { "" }
        ),
    );

    let mut files_removed = 0;
    let mut files_not_found = 0;
    let mut files_failed = 0;

    let data_files = get_data_files_to_remove();
    for file_path in &data_files {
        if dry_run {
            if file_path.exists() {
                logger::info(
                    LogTag::System,
                    &format!("Would remove file: {}", file_path.display()),
                );
                files_removed += 1;
            } else {
                logger::info(
                    LogTag::System,
                    &format!("File not found (would skip): {}", file_path.display()),
                );
                files_not_found += 1;
            }
        } else {
            if file_path.exists() {
                match fs::remove_file(file_path) {
                    Ok(()) => {
                        logger::info(
                            LogTag::System,
                            &format!("Successfully removed file: {}", file_path.display()),
                        );
                        files_removed += 1;
                    }
                    Err(e) => {
                        logger::info(
                            LogTag::System,
                            &format!("Failed to remove file {}: {}", file_path.display(), e),
                        );
                        files_failed += 1;
                    }
                }
            } else {
                logger::info(
                    LogTag::System,
                    &format!("File not found (skipping): {}", file_path.display()),
                );
                files_not_found += 1;
            }
        }
    }

    logger::info(
        LogTag::System,
        &format!(
            "File cleanup completed: {} removed, {} not found, {} failed",
            files_removed, files_not_found, files_failed
        ),
    );

    // Log file cleanup
    logger::info(
        LogTag::System,
        &format!(
            "Starting log file cleanup{}",
            if dry_run { " (DRY RUN)" } else { "" }
        ),
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
                                logger::info(
                                    LogTag::System,
                                    &format!("Would remove log file: {}", path.display()),
                                );
                                log_files_removed += 1;
                            } else {
                                match fs::remove_file(&path) {
                                    Ok(()) => {
                                        logger::info(
                                            LogTag::System,
                                            &format!(
                                                "Successfully removed log file: {}",
                                                path.display()
                                            ),
                                        );
                                        log_files_removed += 1;
                                    }
                                    Err(e) => {
                                        logger::info(
                                            LogTag::System,
                                            &format!(
                                                "Failed to remove log file {}: {}",
                                                path.display(),
                                                e
                                            ),
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
        logger::info(LogTag::System, "Logs directory not found or not accessible");
    }

    logger::info(
        LogTag::System,
        &format!(
            "Log cleanup completed: {} removed, {} failed",
            log_files_removed, log_files_failed
        ),
    );

    // Step 5: Final summary and cleanup report
    logger::info(LogTag::System, &format!("Final cleanup and reset report:"));

    if !token_accounts.is_empty() {
        logger::info(
            LogTag::System,
            &format!("Token accounts found: {}", token_accounts.len()),
        );
    }

    if dry_run {
        if !token_accounts.is_empty() {
            logger::info(
                LogTag::System,
                &format!(
                    "Would Sell - Success: {} | Failed: {}",
                    successful_sells, failed_sells
                ),
            );
            logger::info(
                LogTag::System,
                &format!(
                    "Would Burn - Success: {} | Failed: {}",
                    successful_burns, failed_burns
                ),
            );
            logger::info(
                LogTag::System,
                &format!(
                    "Would Close ATAs - Success: {} | Failed: {}",
                    successful_closes, failed_closes
                ),
            );
        }
        logger::info(
            LogTag::System,
            &format!(
                "Would Remove Files - Success: {} | Not Found: {} | Failed: {}",
                files_removed, files_not_found, files_failed
            ),
        );
    } else {
        if !token_accounts.is_empty() {
            logger::info(
                LogTag::System,
                &format!(
                    "Sales - Success: {} | Failed: {}",
                    successful_sells, failed_sells
                ),
            );
            logger::info(
                LogTag::System,
                &format!(
                    "Burns - Success: {} | Failed: {}",
                    successful_burns, failed_burns
                ),
            );
            logger::info(
                LogTag::System,
                &format!(
                    "ATA Closes - Success: {} | Failed: {}",
                    successful_closes, failed_closes
                ),
            );
        }
        logger::info(
            LogTag::System,
            &format!(
                "File Removals - Success: {} | Not Found: {} | Failed: {}",
                files_removed, files_not_found, files_failed
            ),
        );
    }

    if failed_sells > 0 {
        logger::info(
            LogTag::System,
            &format!("Found {} failed sells", failed_sells),
        );
    }

    if failed_burns > 0 {
        logger::info(
            LogTag::System,
            &format!("Found {} failed burns", failed_burns),
        );
    }

    if failed_closes > 0 {
        logger::info(
            LogTag::System,
            &format!("Found {} failed ATA closes", failed_closes),
        );
    }

    if files_failed > 0 {
        logger::info(
            LogTag::System,
            &format!("Found {} failed file removals", files_failed),
        );
    }

    let estimated_rent_reclaimed = (successful_closes as f64) * 0.00203928; // ~0.002 SOL per ATA
    if estimated_rent_reclaimed > 0.0 {
        if dry_run {
            logger::info(
                LogTag::System,
                &format!("Would reclaim {:.6} SOL in rent", estimated_rent_reclaimed),
            );
        } else {
            logger::info(
                LogTag::System,
                &format!("Reclaimed {:.6} SOL in rent", estimated_rent_reclaimed),
            );
        }
    }

    // Calculate expected vs actual counts, accounting for SOL being skipped
    let sol_accounts = token_accounts.iter().filter(|a| a.mint == SOL_MINT).count();
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
            logger::info(
                LogTag::System,
                "All operations would succeed - dry run complete",
            );
        } else {
            logger::info(
                LogTag::System,
                "All tokens sold/burned, ATAs closed, and data files removed successfully",
            );
        }
    } else {
        if dry_run {
            logger::info(
        LogTag::System,
                &format!(
                    "Dry run completed with issues: {} sell failures, {} burn failures, {} close failures, {} file removal failures",
                    failed_sells,
                    failed_burns,
                    failed_closes,
                    files_failed
                )
            );
        } else {
            logger::info(
        LogTag::System,
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
        logger::info(
            LogTag::System,
            &format!("Final balance check failed: {}", e),
        );
    }

    // Calculate and display final SOL balance change
    let final_sol_balance = match get_sol_balance(&wallet_address).await {
        Ok(balance) => {
            let sol_change = balance - initial_sol_balance;
            if sol_change > 0.0 {
                logger::info(
                    LogTag::System,
                    &format!(
                        "SOL balance increased by {:.6} SOL (from {:.6} to {:.6})",
                        sol_change, initial_sol_balance, balance
                    ),
                );
            } else if sol_change < 0.0 {
                logger::info(
                    LogTag::System,
                    &format!(
                        "SOL balance decreased by {:.6} SOL (from {:.6} to {:.6})",
                        -sol_change, initial_sol_balance, balance
                    ),
                );
            } else {
                logger::info(
                    LogTag::System,
                    &format!("SOL balance unchanged: {:.6} SOL", balance),
                );
            }
            balance
        }
        Err(e) => {
            logger::info(
                LogTag::System,
                &format!("Failed to get final SOL balance: {}", e),
            );
            initial_sol_balance
        }
    };

    if dry_run {
        logger::info(
            LogTag::System,
            "To execute for real, run without --dry-run flag",
        );
    } else {
        logger::info(
            LogTag::System,
            "Bot has been reset to fresh state with clean wallet",
        );
    }

    logger::info(
        LogTag::System,
        &format!(
            "Tool execution finished: {} token accounts processed, {} data files processed",
            token_accounts.len(),
            get_data_files_to_remove().len()
        ),
    );

    Ok(())
}

/// Sell all tokens with retry mechanism
async fn sell_all_tokens_with_retry(
    token_accounts: &[TokenAccountInfo],
    dry_run: bool,
) -> (usize, usize, HashSet<String>) {
    logger::info(
        LogTag::System,
        &format!(
            "Starting token sales for {} accounts{}",
            token_accounts.len(),
            if dry_run { " (DRY RUN)" } else { "" }
        ),
    );

    // Filter accounts for selling (skip zero balance and SOL)
    let sellable_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|account| {
            if account.balance == 0 {
                logger::info(
                    LogTag::System,
                    &format!("Skipping zero balance token: {}", &account.mint),
                );
                return false;
            }

            if account.balance < DUST_THRESHOLD_RAW_UNITS {
                logger::info(
                    LogTag::System,
                    &format!(
                        "Skipping dust balance (<{} raw units) token: {} ({} units)",
                        DUST_THRESHOLD_RAW_UNITS, &account.mint, account.balance
                    ),
                );
                return false;
            }

            if account.mint == SOL_MINT {
                logger::info(LogTag::System, "Skipping SOL (native token)");
                return false;
            }

            true
        })
        .collect();

    logger::info(
        LogTag::System,
        &format!(
            "Filtered {} sellable accounts from {} total (dust threshold: {} units)",
            sellable_accounts.len(),
            token_accounts.len(),
            DUST_THRESHOLD_RAW_UNITS
        ),
    );

    if sellable_accounts.is_empty() {
        return (0, 0, HashSet::new());
    }

    // Process selling with 3 concurrent tasks
    let sell_semaphore = Arc::new(Semaphore::new(3));
    let sell_results: Vec<_> = stream::iter(sellable_accounts.iter())
        .map(|account| {
            let semaphore = sell_semaphore.clone();
            let account = (*account).clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                sell_single_token_with_retry(&account, dry_run).await
            }
        })
        .buffer_unordered(3) // Process up to 3 concurrent sells
        .collect()
        .await;

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

    logger::info(
        LogTag::System,
        &format!(
            "Sales completed: {} success, {} failed",
            successful_sells, failed_sells
        ),
    );

    (successful_sells, failed_sells, successfully_sold_mints)
}

/// Sell a single token with retry mechanism
async fn sell_single_token_with_retry(
    account: &TokenAccountInfo,
    dry_run: bool,
) -> (String, bool, Option<String>) {
    if dry_run {
        logger::info(
            LogTag::System,
            &format!(
                "Would sell {} raw units of {}",
                account.balance, &account.mint
            ),
        );
        return (account.mint.clone(), true, None);
    }

    logger::info(
        LogTag::System,
        &format!(
            "Starting sell for token: {} ({} raw units)",
            &account.mint, account.balance
        ),
    );

    // Try to sell with retries
    for attempt in 1..=MAX_SELL_RETRIES {
        logger::info(
            LogTag::System,
            &format!(
                "Sell attempt {} of {} for token {}",
                attempt, MAX_SELL_RETRIES, &account.mint
            ),
        );

        match attempt_single_sell(account).await {
            Ok(success_msg) => {
                logger::info(LogTag::System, &success_msg);
                return (account.mint.clone(), true, None);
            }
            Err(error_msg) => {
                logger::info(
                    LogTag::System,
                    &format!(
                        "Sell attempt {} failed for {}: {}",
                        attempt, &account.mint, error_msg
                    ),
                );

                // Detect terminal no-route / no-liquidity conditions to avoid useless retries
                let lower_err = error_msg.to_lowercase();
                let is_terminal_no_route = lower_err.contains("no route")
                    || lower_err.contains("no routers available for quote")
                    || lower_err.contains("could not find any route");
                if is_terminal_no_route {
                    logger::info(
        LogTag::System,
                        &format!(
                            "Detected terminal no-route condition for {} – aborting further retries",
                            &account.mint
                        )
                    );
                    return (account.mint.clone(), false, Some(error_msg));
                }

                if attempt < MAX_SELL_RETRIES {
                    let delay = Duration::from_millis(RETRY_DELAY_MS * (attempt as u64));
                    logger::info(
                        LogTag::System,
                        &format!("Waiting {}ms before retry...", delay.as_millis()),
                    );
                    sleep(delay).await;
                } else {
                    logger::info(
                        LogTag::System,
                        &format!(
                            "All sell attempts failed for {}: {}",
                            &account.mint, error_msg
                        ),
                    );
                    return (account.mint.clone(), false, Some(error_msg));
                }
            }
        }
    }

    (
        account.mint.clone(),
        false,
        Some("Max retries exceeded".to_string()),
    )
}

/// Attempt to sell a single token
async fn attempt_single_sell(account: &TokenAccountInfo) -> Result<String, String> {
    let wallet_address = get_wallet_address().map_err(|e| e.to_string())?;

    // Double-check balance before selling
    let actual_balance = get_token_balance(&wallet_address, &account.mint)
        .await
        .map_err(|e| format!("Failed to get token balance: {}", e))?;

    if actual_balance == 0 {
        return Err("Token balance is zero, cannot sell".to_string());
    }

    if actual_balance != account.balance {
        logger::info(
            LogTag::System,
            &format!(
                "Balance mismatch for {}: expected {}, actual {}",
                &account.mint, account.balance, actual_balance
            ),
        );
    }

    // Create minimal Token struct for the sell operation
    let now = chrono::Utc::now();
    let token = Token {
        // Core identity
        mint: account.mint.clone(),
        symbol: format!("TOKEN_{}", &account.mint[..8]),
        name: format!("Unknown Token {}", &account.mint[..8]),
        decimals: 9, // Default decimals, actual value doesn't matter for liquidation

        // Optional metadata
        description: None,
        image_url: None,
        header_image_url: None,
        supply: None,

        // Data source configuration
        data_source: screenerbot::tokens::types::DataSource::Unknown,
        first_discovered_at: now,
        blockchain_created_at: None,
        metadata_last_fetched_at: now,
        decimals_last_fetched_at: now,
        market_data_last_fetched_at: now,
        security_data_last_fetched_at: None,
        pool_price_last_calculated_at: now,
        pool_price_last_used_pool: None,

        // Price information (zeros for liquidation)
        price_usd: 0.0,
        price_sol: 0.0,
        price_native: "0".to_string(),
        price_change_m5: None,
        price_change_h1: None,
        price_change_h6: None,
        price_change_h24: None,

        // Market metrics
        market_cap: None,
        fdv: None,
        liquidity_usd: None,

        // Volume data
        volume_m5: None,
        volume_h1: None,
        volume_h6: None,
        volume_h24: None,
        pool_count: None,
        reserve_in_usd: None,

        // Transaction activity
        txns_m5_buys: None,
        txns_m5_sells: None,
        txns_h1_buys: None,
        txns_h1_sells: None,
        txns_h6_buys: None,
        txns_h6_sells: None,
        txns_h24_buys: None,
        txns_h24_sells: None,

        // Social & links
        websites: vec![],
        socials: vec![],

        // Security information
        mint_authority: None,
        freeze_authority: None,
        security_score: None,
        is_rugged: false,
        token_type: None,
        graph_insiders_detected: None,
        lp_provider_count: None,
        security_risks: vec![],
        total_holders: None,
        top_holders: vec![],
        creator_balance_pct: None,
        transfer_fee_pct: None,
        transfer_fee_max_amount: None,
        transfer_fee_authority: None,

        // Bot-specific state
        is_blacklisted: false,
        priority: screenerbot::tokens::priorities::Priority::Background,
    };

    // Get quote and execute swap
    let quote_slippage = with_config(|cfg| cfg.swaps.slippage.quote_default_pct);
    let best_quote = get_best_quote_legacy(
        &token.mint,
        SOL_MINT,
        actual_balance,
        &wallet_address,
        quote_slippage,
        "ExactIn", // ExactIn mode: sell exact token amount, receive variable SOL
    )
    .await
    .map_err(|e| format!("Failed to get quote: {}", e))?;

    let swap_result = execute_best_swap_legacy(&token, &token.mint, SOL_MINT, actual_balance, best_quote)
        .await
        .map_err(|e| format!("Failed to execute swap: {}", e))?;

    if swap_result.success {
        let sol_amount = swap_result
            .output_amount
            .parse::<u64>()
            .map(|lamports| (lamports as f64) / 1_000_000_000.0)
            .unwrap_or(0.0);

        Ok(format!(
            "Successfully sold {} for {:.6} SOL",
            &account.mint, sol_amount
        ))
    } else {
        Err(swap_result
            .error
            .unwrap_or_else(|| "Unknown swap error".to_string()))
    }
}

/// Close all ATAs with retry mechanism
async fn close_all_atas_with_retry(
    token_accounts: &[TokenAccountInfo],
    successfully_sold_mints: &HashSet<String>,
    successfully_burned_mints: &HashSet<String>,
    wallet_address: &str,
    dry_run: bool,
) -> (usize, usize) {
    logger::info(
        LogTag::System,
        &format!(
            "Starting ATA cleanup for {} accounts{}",
            token_accounts.len(),
            if dry_run { " (DRY RUN)" } else { "" }
        ),
    );

    // Filter accounts for ATA closing (skip SOL and non-zero balances unless burned)
    let closable_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|account| {
            if account.mint == SOL_MINT {
                logger::info(LogTag::System, "Skipping SOL account for ATA closing");
                return false;
            }

            // Check if the token was successfully burned or sold
            let was_burned = successfully_burned_mints.contains(&account.mint);
            let was_sold = successfully_sold_mints.contains(&account.mint);

            if account.balance > 0 && !was_burned && !was_sold {
                logger::info(
        LogTag::System,
                    &format!(
                        "Skipping ATA close for {} - still has {} tokens (cannot close non-empty ATA)",
                        &account.mint,
                        account.balance
                    )
                );
                return false;
            }

            if was_burned {
                logger::info(
        LogTag::System,
                    &format!(
                        "Including ATA for {} - tokens were burned (should now be empty)",
                        &account.mint
                    )
                );
            }

            true
        })
        .collect();

    logger::info(
        LogTag::System,
        &format!(
            "Filtered {} closable accounts from {} total (only empty ATAs can be closed)",
            closable_accounts.len(),
            token_accounts.len()
        ),
    );

    if closable_accounts.is_empty() {
        return (0, 0);
    }

    // Process ATA closing with 3 concurrent tasks
    let close_semaphore = Arc::new(Semaphore::new(3));
    let close_results: Vec<_> = stream::iter(closable_accounts.iter())
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
                    dry_run,
                )
                .await
            }
        })
        .buffer_unordered(3) // Process up to 3 concurrent ATA closes
        .collect()
        .await;

    let successful_closes = close_results
        .iter()
        .filter(|(_, success, _)| *success)
        .count();
    let failed_closes = close_results
        .iter()
        .filter(|(_, success, _)| !*success)
        .count();

    logger::info(
        LogTag::System,
        &format!(
            "ATA cleanup completed: {} success, {} failed",
            successful_closes, failed_closes
        ),
    );

    (successful_closes, failed_closes)
}

/// Close a single ATA with retry mechanism
async fn close_single_ata_with_retry(
    account: &TokenAccountInfo,
    successfully_sold_mints: &HashSet<String>,
    successfully_burned_mints: &HashSet<String>,
    wallet_address: &str,
    dry_run: bool,
) -> (String, bool, Option<String>) {
    if dry_run {
        logger::info(
            LogTag::System,
            &format!("Would close ATA for token: {}", &account.mint),
        );
        return (account.mint.clone(), true, Some("DRY_RUN_TX".to_string()));
    }

    logger::info(
        LogTag::System,
        &format!("Starting ATA close for token: {}", &account.mint),
    );

    // Check if this token was recently sold or burned
    let recently_sold = successfully_sold_mints.contains(&account.mint);
    let recently_burned = successfully_burned_mints.contains(&account.mint);
    let recently_processed = recently_sold || recently_burned;

    // Try to close with retries
    for attempt in 1..=MAX_ATA_CLOSE_RETRIES {
        logger::info(
            LogTag::System,
            &format!(
                "ATA close attempt {} of {} for token {}",
                attempt, MAX_ATA_CLOSE_RETRIES, &account.mint
            ),
        );

        match attempt_single_ata_close(&account.mint, wallet_address, recently_processed).await {
            Ok(signature) => {
                logger::info(
                    LogTag::System,
                    &format!(
                        "Successfully closed ATA for {}. TX: {}",
                        &account.mint, signature
                    ),
                );
                return (account.mint.clone(), true, Some(signature));
            }
            Err(error_msg) => {
                logger::info(
                    LogTag::System,
                    &format!(
                        "ATA close attempt {} failed for {}: {}",
                        attempt, &account.mint, error_msg
                    ),
                );

                if attempt < MAX_ATA_CLOSE_RETRIES {
                    let delay = Duration::from_millis(RETRY_DELAY_MS * (attempt as u64));
                    logger::info(
                        LogTag::System,
                        &format!("Waiting {}ms before retry...", delay.as_millis()),
                    );
                    sleep(delay).await;
                } else {
                    logger::info(
                        LogTag::System,
                        &format!(
                            "All ATA close attempts failed for {}: {}",
                            &account.mint, error_msg
                        ),
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
    recently_sold: bool,
) -> Result<String, String> {
    // Verify the ATA actually exists before trying to close it
    let rpc_client = get_rpc_client();

    match rpc_client
        .get_associated_token_account(wallet_address, mint)
        .await
    {
        Ok(ata_address) => {
            // Double-check that the account still exists with fresh RPC data
            match rpc_client.is_token_account_token_2022(&ata_address).await {
                Ok(_) => {
                    // Account exists, proceed with closing
                    logger::info(
                        LogTag::System,
                        &format!(
                            "✅ ATA_VERIFIED: account {} exists, proceeding with close",
                            &ata_address
                        ),
                    );
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
    logger::info(
        LogTag::System,
        &format!("Checking comprehensive balance at stage: {}", stage),
    );

    // Check SOL balance
    match get_sol_balance(wallet_address).await {
        Ok(sol_balance) => {
            logger::info(
                LogTag::System,
                &format!("{}: SOL balance: {:.6} SOL", stage, sol_balance),
            );
        }
        Err(e) => {
            logger::info(
                LogTag::System,
                &format!("Failed to get SOL balance at {}: {}", stage, e),
            );
        }
    }

    // Check token accounts
    match get_all_token_accounts(wallet_address).await {
        Ok(token_accounts) => {
            let non_zero_accounts = token_accounts.iter().filter(|a| a.balance > 0).count();
            logger::info(
                LogTag::System,
                &format!(
                    "{}: Found {} token accounts, {} with non-zero balance",
                    stage,
                    token_accounts.len(),
                    non_zero_accounts
                ),
            );

            if !token_accounts.is_empty() {
                logger::debug(LogTag::System, &format!("Token accounts at {}:", stage));
                for account in token_accounts.iter().take(10) {
                    // Show max 10 to avoid spam
                    let token_program = if account.is_token_2022 {
                        "Token-2022"
                    } else {
                        "SPL Token"
                    };
                    logger::debug(
                        LogTag::System,
                        &format!(
                            "  {} ({}): {} raw units",
                            &account.mint, token_program, account.balance
                        ),
                    );
                }
                if token_accounts.len() > 10 {
                    logger::debug(
                        LogTag::System,
                        &format!("  ... and {} more accounts", token_accounts.len() - 10),
                    );
                }
            }
        }
        Err(e) => {
            logger::info(
                LogTag::System,
                &format!("Failed to get token accounts at {}: {}", stage, e),
            );
        }
    }

    Ok(())
}
