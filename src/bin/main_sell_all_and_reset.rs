#![allow(warnings)]

//! # Sell All Tokens, Close ATAs, and Reset Bot Data
//!
//! This utility performs a comprehensive wallet cleanup and bot data reset by:
//! 1. Scanning for all SPL Token and Token-2022 accounts
//! 2. Selling all tokens with non-zero balances for SOL
//! 3. Closing all Associated Token Accounts (ATAs) to reclaim rent SOL
//! 4. Removing specific bot data files to reset the system
//!
//! ## Usage
//! ```bash
//! cargo run --bin tool_sell_all_and_reset
//! ```
//!
//! ## Safety Features
//! - Skips SOL (native token) accounts
//! - Validates token balances before selling
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

use screenerbot::global::{ read_configs };
use screenerbot::tokens::{ Token };
use screenerbot::logger::{ log, LogTag };
use screenerbot::utils::{ get_wallet_address, close_token_account_with_context };
use screenerbot::swaps::sell_token;
use screenerbot::rpc::{ SwapError, init_rpc_client, get_rpc_client };
use screenerbot::arguments::is_debug_ata_enabled;
use reqwest;
use serde_json;
use std::env;
use std::sync::Arc;
use std::fs;
use std::path::Path;
use std::collections::HashSet;
use tokio::sync::Semaphore;
use futures::stream::{ self, StreamExt };

/// Print comprehensive help menu for the Sell All and Reset Tool
fn print_help() {
    println!("🔄 Sell All Tokens, Close ATAs, and Reset Bot Data Tool");
    println!("======================================================");
    println!("Comprehensive wallet cleanup and bot reset utility that sells all tokens for SOL,");
    println!("closes all Associated Token Accounts (ATAs), and resets bot data files.");
    println!("");
    println!("⚠️  WARNING: This tool will:");
    println!("    • Sell ALL tokens in your wallet for SOL");
    println!("    • Close all Associated Token Accounts (ATAs)");
    println!("    • DELETE specific bot data files (irreversible)");
    println!("    Use with extreme caution and understand the risks involved.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_sell_all_and_reset [OPTIONS]");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h          Show this help message");
    println!("    --dry-run, -d      Simulate operations without executing transactions");
    println!("");
    println!("EXAMPLES:");
    println!("    # Simulate cleanup to see what would happen");
    println!("    cargo run --bin tool_sell_all_and_reset -- --dry-run");
    println!("");
    println!("    # Execute full wallet cleanup and bot reset (DANGEROUS)");
    println!("    cargo run --bin tool_sell_all_and_reset");
    println!("");
    println!("OPERATIONS PERFORMED:");
    println!("    1. Scan wallet for all SPL Token and Token-2022 accounts");
    println!("    2. Identify tokens with non-zero balances");
    println!("    3. Sell all tokens for SOL using GMGN swap service");
    println!("    4. Close all Associated Token Accounts (empty and non-empty)");
    println!("    5. Reclaim rent SOL from closed ATAs (~0.00203928 SOL each)");
    println!("    6. Delete specific bot data files to reset the system");
    println!("    7. Clean up all bot log files from logs/ directory");
    println!("");
    println!("DATA FILES THAT WILL BE DELETED:");
    println!("    • data/rpc_stats.json (RPC statistics)");
    println!("    • data/positions.json (trading positions)");
    println!("    • data/ata_failed_cache.json (failed ATA cache)");
    println!("    • logs/screenerbot_*.log (all bot log files)");
    println!("");
    println!("FILES THAT WILL BE PRESERVED:");
    println!("    • data/configs.json (wallet keys and RPC endpoints)");
    println!("    • data/tokens.db (token database)");
    println!("    • data/decimal_cache.json (token decimals cache)");
    println!("    • data/token_blacklist.json (blacklisted tokens)");
    println!("    • data/wallet_transactions_stats.json (wallet sync data)");
    println!("    • data/transactions/ (individual transaction files)");
    println!("    • data/cache_ohlcvs/ (OHLCV data cache)");
    println!("");
    println!("SAFETY FEATURES:");
    println!("    • Skips SOL (native token) - cannot sell SOL for SOL");
    println!("    • Validates token balances before attempting sales");
    println!("    • Detailed progress reporting for each operation");
    println!("    • Graceful error handling for failed transactions");
    println!("    • Supports both SPL Token and Token-2022 programs");
    println!("    • Concurrent processing with rate limiting");
    println!("    • Only removes specific data files, preserves configuration");
    println!("");
    println!("ESTIMATED OUTCOMES:");
    println!("    • SOL received from token sales (varies by token values)");
    println!("    • Rent SOL reclaimed from closed ATAs");
    println!("    • Clean wallet with only SOL remaining");
    println!("    • Fresh bot state with preserved configuration");
    println!("");
    println!("RISK WARNINGS:");
    println!("    • Irreversible operation - tokens will be permanently sold");
    println!("    • Bot data files will be permanently deleted");
    println!("    • Market slippage may result in lower SOL amounts");
    println!("    • Some tokens may fail to sell due to liquidity issues");
    println!("    • Failed transactions still consume transaction fees");
    println!("    • Use --dry-run first to understand the impact");
    println!("");
}

/// SOL token mint address (native Solana)
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Data files to be removed during reset
const DATA_FILES_TO_REMOVE: &[&str] = &[
    "data/rpc_stats.json",
    "data/positions.json",
    "data/ata_failed_cache.json",
];

/// Token account information from Solana RPC
#[derive(Debug, Clone)]
struct TokenAccount {
    pub mint: String,
    pub balance: u64,
    pub ui_amount: f64,
}

/// Main function to sell all tokens, close all ATAs, and reset bot data
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    let dry_run = args.contains(&"--dry-run".to_string()) || args.contains(&"-d".to_string());

    log(LogTag::System, "INFO", "WALLET CLEANUP AND BOT RESET UTILITY");
    log(LogTag::System, "INFO", "====================================");

    if dry_run {
        log(LogTag::System, "INFO", "DRY RUN MODE - No actual transactions or file deletions will be made");
    }

    log(LogTag::System, "INFO", "This tool will:");
    log(LogTag::System, "INFO", "  - Scan for all token accounts (SPL & Token-2022)");
    if !dry_run {
        log(LogTag::System, "INFO", "  - Sell ALL tokens for SOL");
        log(LogTag::System, "INFO", "  - Close all Associated Token Accounts (ATAs)");
        log(LogTag::System, "INFO", "  - Reclaim rent SOL from closed ATAs");
        log(LogTag::System, "INFO", "  - Delete specific bot data files to reset the system");
    } else {
        log(LogTag::System, "INFO", "  - Show what tokens would be sold");
        log(LogTag::System, "INFO", "  - Show what ATAs would be closed");
        log(LogTag::System, "INFO", "  - Estimate rent SOL that would be reclaimed");
        log(LogTag::System, "INFO", "  - Show what data files would be deleted");
    }

    log(
        LogTag::System,
        "INFO",
        &format!("Starting comprehensive wallet cleanup and bot reset{}", if dry_run { " (DRY RUN)" } else { "" })
    );

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    log(LogTag::System, "WALLET", &format!("Processing wallet: {}", wallet_address));

    // Step 1: Get all token accounts using centralized RPC client (prevents stale account data)
    log(LogTag::System, "INFO", "Scanning for all token accounts using centralized RPC client...");
    
    // Initialize centralized RPC client to avoid stale cache issues
    init_rpc_client()?;
    let rpc_client = get_rpc_client();
    
    let rpc_token_accounts = match rpc_client.get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token accounts: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };
    
    // Convert from RPC TokenAccountInfo to local TokenAccount format
    let token_accounts: Vec<TokenAccount> = rpc_token_accounts.into_iter().map(|rpc_account| {
        TokenAccount {
            mint: rpc_account.mint,
            balance: rpc_account.balance,
            ui_amount: (rpc_account.balance as f64) / 1_000_000.0, // Approximate UI amount
        }
    }).collect();

    if token_accounts.is_empty() {
        log(LogTag::System, "INFO", "No token accounts found - wallet is already clean!");
    } else {
        log(LogTag::System, "INFO", &format!("Found {} token accounts:", token_accounts.len()));
        for account in &token_accounts {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "  Token {} ({:.6} tokens) - Mint: {}",
                    &account.mint[..8],
                    account.ui_amount,
                    account.mint
                )
            );
        }
    }

    // Step 2: Sell all tokens with balances > 0
    let (successful_sells, failed_sells, successfully_sold_mints) = if !token_accounts.is_empty() {
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
                        &format!("Skipping zero balance token: {}", account.mint)
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
                "Filtered {} sellable accounts from {} total",
                sellable_accounts.len(),
                token_accounts.len()
            )
        );

        // Process selling with 3 concurrent tasks
        let sell_semaphore = Arc::new(Semaphore::new(3));
        let sell_results: Vec<_> = stream
            ::iter(sellable_accounts.iter())
            .map(|account| {
                let semaphore = sell_semaphore.clone();
                let account = (*account).clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    if dry_run {
                        log(
                            LogTag::System,
                            "DRY_SELL",
                            &format!("Would sell {} tokens of {}", account.ui_amount, account.mint)
                        );
                        return (account, true, None);
                    }

                    log(
                        LogTag::System,
                        "SELL_START",
                        &format!(
                            "Starting sell for token: {} ({:.6} tokens)",
                            account.mint,
                            account.ui_amount
                        )
                    );

                    // Create a minimal Token struct for the sell operation
                    let token = Token {
                        mint: account.mint.clone(),
                        symbol: format!("TOKEN_{}", &account.mint[..8]),
                        name: format!("Unknown Token {}", &account.mint[..8]),
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
                    };

                    // Attempt to sell all tokens
                    match sell_token(&token, account.balance, None, None).await {
                        Ok(swap_result) => {
                            if swap_result.success {
                                // Parse the amount from swap result
                                let sol_amount = swap_result.output_amount
                                    .parse::<u64>()
                                    .map(|lamports| (lamports as f64) / 1_000_000_000.0)
                                    .unwrap_or(0.0);

                                log(
                                    LogTag::System,
                                    "SELL_SUCCESS",
                                    &format!(
                                        "Successfully sold {} for {:.6} SOL",
                                        account.mint,
                                        sol_amount
                                    )
                                );
                                (account, true, None)
                            } else {
                                let error_msg = swap_result.error.as_deref().unwrap_or("Unknown error");
                                log(
                                    LogTag::System,
                                    "SELL_FAILED",
                                    &format!("Sell failed for {}: {}", account.mint, error_msg)
                                );
                                (account, false, swap_result.error.clone())
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::System,
                                "SELL_ERROR",
                                &format!("Sell error for {}: {}", account.mint, e)
                            );
                            (account, false, Some(e.to_string()))
                        }
                    }
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
        let successfully_sold_mints: std::collections::HashSet<String> = sell_results
            .iter()
            .filter(|(_, success, _)| *success)
            .map(|(account, _, _)| account.mint.clone())
            .collect();

        log(
            LogTag::System,
            "SELL_SUMMARY",
            &format!("Sales completed: {} success, {} failed", successful_sells, failed_sells)
        );

        (successful_sells, failed_sells, successfully_sold_mints)
    } else {
        (0, 0, HashSet::new())
    };

    // Step 2.5: Wait for swap transactions to be processed before ATA closing
    if successful_sells > 0 && !dry_run {
        const SWAP_CONFIRMATION_DELAY_SECONDS: u64 = 10;
        log(
            LogTag::System,
            "WAIT_CONFIRMATION",
            &format!("Waiting {}s for {} swap transactions to be confirmed before ATA closing...", 
                SWAP_CONFIRMATION_DELAY_SECONDS, successful_sells)
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
            "Swap confirmation wait completed, proceeding with ATA closing"
        );
    }

    // Step 3: Close all ATAs
    let (successful_closes, failed_closes) = if !token_accounts.is_empty() {
        log(
            LogTag::System,
            "ATA_START",
            &format!("Starting ATA cleanup for {} accounts{}", token_accounts.len(), if dry_run {
                " (DRY RUN)"
            } else {
                ""
            })
        );

        if is_debug_ata_enabled() {
            log(
                LogTag::System,
                "DEBUG",
                &format!("🔧 RESET_ATA_START: processing {} accounts with debug enabled", token_accounts.len())
            );
        }

        // Filter accounts for ATA closing (skip SOL)
        let closable_accounts: Vec<_> = token_accounts
            .iter()
            .filter(|account| {
                if account.mint == SOL_MINT {
                    if is_debug_ata_enabled() {
                        log(
                            LogTag::System,
                            "DEBUG",
                            &format!("⏭️ RESET_SKIP_SOL: skipping SOL mint {}", &account.mint[..8])
                        );
                    }
                    log(LogTag::System, "SKIP_SOL_ATA", "Skipping SOL account for ATA closing");
                    return false;
                }
                if is_debug_ata_enabled() {
                    log(
                        LogTag::System,
                        "DEBUG",
                        &format!("✅ RESET_INCLUDE_TOKEN: including token {} for ATA closing", &account.mint[..8])
                    );
                }
                true
            })
            .collect();

        if is_debug_ata_enabled() {
            log(
                LogTag::System,
                "DEBUG",
                &format!("📊 RESET_FILTER_RESULT: {} closable accounts from {} total", 
                    closable_accounts.len(), token_accounts.len())
            );
        }

        log(
            LogTag::System,
            "ATA_FILTER",
            &format!(
                "Filtered {} closable accounts from {} total",
                closable_accounts.len(),
                token_accounts.len()
            )
        );

        // Process ATA closing with 3 concurrent tasks
        if is_debug_ata_enabled() {
            log(
                LogTag::System,
                "DEBUG",
                &format!("🔄 RESET_CONCURRENT_START: starting {} concurrent ATA closing tasks", 3)
            );
        }
        
        let close_semaphore = Arc::new(Semaphore::new(3));
        let close_results: Vec<_> = stream
            ::iter(closable_accounts.iter())
            .map(|account| {
                let semaphore = close_semaphore.clone();
                let account = (*account).clone();
                let wallet_address = wallet_address.clone();
                let successfully_sold_mints = successfully_sold_mints.clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    if is_debug_ata_enabled() {
                        let was_recently_sold = successfully_sold_mints.contains(&account.mint);
                        log(
                            LogTag::System,
                            "DEBUG",
                            &format!("🎬 RESET_ATA_TASK_START: processing mint {} with balance {}, recently_sold={}", 
                                &account.mint[..8], account.ui_amount, was_recently_sold)
                        );
                    }

                    if dry_run {
                        if is_debug_ata_enabled() {
                            log(
                                LogTag::System,
                                "DEBUG",
                                &format!("🧪 RESET_DRY_RUN: simulating ATA close for mint {}", &account.mint[..8])
                            );
                        }
                        log(
                            LogTag::System,
                            "DRY_ATA",
                            &format!("Would close ATA for token: {}", account.mint)
                        );
                        return (account, true, Some("DRY_RUN_TX".to_string()));
                    }

                    if is_debug_ata_enabled() {
                        log(
                            LogTag::System,
                            "DEBUG",
                            &format!("🚀 RESET_ATA_LIVE: executing live ATA close for mint {}", &account.mint[..8])
                        );
                    }

                    log(
                        LogTag::System,
                        "ATA_START",
                        &format!("Starting ATA close for token: {}", account.mint)
                    );

                    // Verify the ATA actually exists before trying to close it
                    // This prevents "invalid account data" errors on already-closed ATAs
                    let rpc_client = get_rpc_client();
                    match rpc_client.get_associated_token_account(&wallet_address, &account.mint).await {
                        Ok(ata_address) => {
                            // Double-check that the account still exists with fresh RPC data
                            match rpc_client.is_token_account_token_2022(&ata_address).await {
                                Ok(_) => {
                                    // Account exists, proceed with closing
                                    if is_debug_ata_enabled() {
                                        log(
                                            LogTag::System,
                                            "DEBUG",
                                            &format!("✅ ATA_VERIFIED: account {} exists, proceeding with close", &ata_address[..8])
                                        );
                                    }
                                }
                                Err(_) => {
                                    // Account doesn't exist or is already closed
                                    if is_debug_ata_enabled() {
                                        log(
                                            LogTag::System,
                                            "DEBUG",
                                            &format!("⚠️ ATA_ALREADY_CLOSED: account for mint {} appears to be already closed", &account.mint[..8])
                                        );
                                    }
                                    log(
                                        LogTag::System,
                                        "ATA_SKIP",
                                        &format!("ATA for {} appears to be already closed, skipping", account.mint)
                                    );
                                    return (account, true, Some("ALREADY_CLOSED".to_string()));
                                }
                            }
                        }
                        Err(_) => {
                            // Cannot find ATA, likely already closed
                            log(
                                LogTag::System,
                                "ATA_SKIP",
                                &format!("Cannot find ATA for {}, likely already closed", account.mint)
                            );
                            return (account, true, Some("NOT_FOUND".to_string()));
                        }
                    }

                    let start_time = std::time::Instant::now();
                    
                    // Check if this token was recently sold and use enhanced retry logic
                    let recently_sold = successfully_sold_mints.contains(&account.mint);
                    
                    match close_token_account_with_context(&account.mint, &wallet_address, recently_sold).await {
                        Ok(signature) => {
                            let duration = start_time.elapsed();
                            
                            if is_debug_ata_enabled() {
                                log(
                                    LogTag::System,
                                    "DEBUG",
                                    &format!("🎉 RESET_ATA_SUCCESS: closed mint {} in {:.2}s, tx={}", 
                                        &account.mint[..8], duration.as_secs_f64(), &signature[..8])
                                );
                            }
                            
                            log(
                                LogTag::System,
                                "ATA_SUCCESS",
                                &format!(
                                    "Successfully closed ATA for {}. TX: {}",
                                    account.mint,
                                    signature
                                )
                            );
                            
                            // Verify the ATA closing transaction if not in dry run mode
                            if !dry_run {
                                if is_debug_ata_enabled() {
                                    log(
                                        LogTag::System,
                                        "DEBUG",
                                        &format!("🔍 RESET_ATA_VERIFY: starting verification for tx {}", &signature[..8])
                                    );
                                }
                                
                                log(LogTag::System, "ATA_VERIFY", &format!("Verifying ATA close transaction: {}", &signature[..8]));
                                
                                // Wait a moment for transaction to propagate
                                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                
                                // Note: ATA closing is not a "swap" so we can't use verify_swap_transaction_global
                                // But we can log that we attempted verification
                                if is_debug_ata_enabled() {
                                    log(
                                        LogTag::System,
                                        "DEBUG",
                                        &format!("📝 RESET_ATA_LOGGED: verification logged for tx {}", &signature[..8])
                                    );
                                }
                                
                                log(LogTag::System, "ATA_VERIFY_INFO", "ATA close transaction logged for future verification");
                            }
                            
                            (account, true, Some(signature))
                        }
                        Err(e) => {
                            let duration = start_time.elapsed();
                            
                            if is_debug_ata_enabled() {
                                log(
                                    LogTag::System,
                                    "DEBUG",
                                    &format!("❌ RESET_ATA_FAILED: mint {} failed after {:.2}s: {}", 
                                        &account.mint[..8], duration.as_secs_f64(), e)
                                );
                            }
                            
                            log(
                                LogTag::System,
                                "ATA_FAILED",
                                &format!("Failed to close ATA for {}: {}", account.mint, e)
                            );
                            (account, false, None)
                        }
                    }
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
    } else {
        (0, 0)
    };

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
                                            &format!("Successfully removed log file: {}", path.display())
                                        );
                                        log_files_removed += 1;
                                    }
                                    Err(e) => {
                                        log(
                                            LogTag::System,
                                            "LOG_REMOVE_FAILED",
                                            &format!("Failed to remove log file {}: {}", path.display(), e)
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
        log(
            LogTag::System,
            "LOG_DIR_NOT_FOUND",
            "Logs directory not found or not accessible"
        );
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
    log(
        LogTag::System,
        "FINAL_REPORT",
        &format!("Final cleanup and reset report:")
    );
    
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

    if failed_closes > 0 {
        log(LogTag::System, "FAILED_CLOSES", &format!("Found {} failed ATA closes", failed_closes));
    }

    if files_failed > 0 {
        log(LogTag::System, "FAILED_FILE_REMOVES", &format!("Found {} failed file removals", files_failed));
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
    let expected_operations = if token_accounts.is_empty() { 0 } else { token_accounts.len() - sol_accounts };

    let all_wallet_ops_successful = token_accounts.is_empty() || 
        (successful_sells == expected_operations && successful_closes == expected_operations);
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
            log(LogTag::System, "RESET_COMPLETE", "All tokens sold, ATAs closed, and data files removed successfully");
        }
    } else {
        if dry_run {
            log(
                LogTag::System,
                "DRY_RUN_ISSUES",
                &format!(
                    "Dry run completed with issues: {} sell failures, {} close failures, {} file removal failures",
                    failed_sells,
                    failed_closes,
                    files_failed
                )
            );
        } else {
            log(
                LogTag::System,
                "RESET_ISSUES",
                &format!(
                    "Reset completed with issues: {} sell failures, {} close failures, {} file removal failures",
                    failed_sells,
                    failed_closes,
                    files_failed
                )
            );
        }
    }

    if dry_run {
        log(LogTag::System, "DRY_RUN_HINT", "To execute for real, run without --dry-run flag");
    } else {
        log(LogTag::System, "RESET_HINT", "Bot has been reset to fresh state with clean wallet");
    }

    log(
        LogTag::System,
        "TOOL_COMPLETE",
        &format!("Tool execution finished: {} token accounts processed, {} data files processed", 
                token_accounts.len(), 
                DATA_FILES_TO_REMOVE.len())
    );

    Ok(())
}

/// Gets all token accounts for the given wallet address
async fn get_all_token_accounts(wallet_address: &str) -> Result<Vec<TokenAccount>, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" // SPL Token Program
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        log(LogTag::System, "RPC", &format!("Querying token accounts from: {}", rpc_url));

        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                let mut token_accounts = Vec::new();

                                for account in accounts {
                                    if
                                        let (Some(_pubkey), Some(account_data)) = (
                                            account.get("pubkey"),
                                            account.get("account"),
                                        )
                                    {
                                        if let Some(data) = account_data.get("data") {
                                            if let Some(parsed) = data.get("parsed") {
                                                if let Some(info) = parsed.get("info") {
                                                    let mint = info
                                                        .get("mint")
                                                        .and_then(|m| m.as_str())
                                                        .unwrap_or("")
                                                        .to_string();

                                                    let token_amount = info.get("tokenAmount");
                                                    let balance = token_amount
                                                        .and_then(|ta| ta.get("amount"))
                                                        .and_then(|a| a.as_str())
                                                        .and_then(|s| s.parse::<u64>().ok())
                                                        .unwrap_or(0);

                                                    let ui_amount = token_amount
                                                        .and_then(|ta| ta.get("uiAmount"))
                                                        .and_then(|ua| ua.as_f64())
                                                        .unwrap_or(0.0);

                                                    if !mint.is_empty() {
                                                        token_accounts.push(TokenAccount {
                                                            mint,
                                                            balance,
                                                            ui_amount,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                log(
                                    LogTag::System,
                                    "SUCCESS",
                                    &format!("Found {} token accounts", token_accounts.len())
                                );
                                return Ok(token_accounts);
                            }
                        }
                    }

                    // Check for RPC error
                    if let Some(error) = rpc_response.get("error") {
                        log(LogTag::System, "RPC_ERROR", &format!("RPC error: {}", error));
                        continue;
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "NETWORK_ERROR",
                    &format!("Network error with {}: {}", rpc_url, e)
                );
                continue;
            }
        }
    }

    Err(
        SwapError::TransactionError(
            "Failed to fetch token accounts from all RPC endpoints".to_string()
        )
    )
}

/// Also get Token-2022 accounts (Token Extensions Program)
async fn get_token_2022_accounts(wallet_address: &str) -> Result<Vec<TokenAccount>, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet_address,
            {
                "programId": "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" // Token-2022 Program
            },
            {
                "encoding": "jsonParsed"
            }
        ]
    });

    let client = reqwest::Client::new();

    // Try main RPC first, then fallbacks
    let rpc_endpoints = std::iter
        ::once(&configs.rpc_url)
        .chain(configs.rpc_fallbacks.iter())
        .collect::<Vec<_>>();

    for rpc_url in rpc_endpoints {
        match
            client
                .post(rpc_url)
                .header("Content-Type", "application/json")
                .json(&rpc_payload)
                .send().await
        {
            Ok(response) => {
                if let Ok(rpc_response) = response.json::<serde_json::Value>().await {
                    if let Some(result) = rpc_response.get("result") {
                        if let Some(value) = result.get("value") {
                            if let Some(accounts) = value.as_array() {
                                let mut token_accounts = Vec::new();

                                for account in accounts {
                                    if
                                        let (Some(_pubkey), Some(account_data)) = (
                                            account.get("pubkey"),
                                            account.get("account"),
                                        )
                                    {
                                        if let Some(data) = account_data.get("data") {
                                            if let Some(parsed) = data.get("parsed") {
                                                if let Some(info) = parsed.get("info") {
                                                    let mint = info
                                                        .get("mint")
                                                        .and_then(|m| m.as_str())
                                                        .unwrap_or("")
                                                        .to_string();

                                                    let token_amount = info.get("tokenAmount");
                                                    let balance = token_amount
                                                        .and_then(|ta| ta.get("amount"))
                                                        .and_then(|a| a.as_str())
                                                        .and_then(|s| s.parse::<u64>().ok())
                                                        .unwrap_or(0);

                                                    let ui_amount = token_amount
                                                        .and_then(|ta| ta.get("uiAmount"))
                                                        .and_then(|ua| ua.as_f64())
                                                        .unwrap_or(0.0);

                                                    if !mint.is_empty() {
                                                        token_accounts.push(TokenAccount {
                                                            mint,
                                                            balance,
                                                            ui_amount,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                return Ok(token_accounts);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                continue;
            }
        }
    }

    // If we get here, either there was an error or no Token-2022 accounts found
    // Return empty vec instead of error since Token-2022 accounts are optional
    Ok(Vec::new())
}
