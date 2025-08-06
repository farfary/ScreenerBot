//! # Sell All Tokens and Close ATAs Utility
//!
//! This utility performs a comprehensive wallet cleanup by:
//! 1. Scanning for all SPL Token and Token-2022 accounts
//! 2. Selling all tokens with non-zero balances for SOL
//! 3. Closing all Associated Token Accounts (ATAs) to reclaim rent SOL
//!
//! ## Usage
//! ```bash
//! cargo run --bin sell_all_and_close_atas
//! ```
//!
//! ## Safety Features
//! - Skips SOL (native token) accounts
//! - Validates token balances before selling
//! - Provides detailed progress reporting
//! - Graceful error handling for failed operations
//! - Estimates rent SOL reclaimed from closed ATAs
//!
//! ## Configuration
//! Requires `configs.json` with wallet private key and RPC endpoints.
//!
//! ## Warning
//! This tool will attempt to sell ALL tokens in your wallet. Use with caution!

use screenerbot::global::{ read_configs };
use screenerbot::tokens::{ Token };
use screenerbot::logger::{ log, LogTag };
use screenerbot::utils::{ get_wallet_address, close_token_account };
use screenerbot::swaps::sell_token;
use screenerbot::rpc::SwapError;
use reqwest;
use serde_json;
use std::env;
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::stream::{ self, StreamExt };

/// Print comprehensive help menu for the Sell All and Close ATAs Tool
fn print_help() {
    println!("💰 Sell All Tokens and Close ATAs Tool");
    println!("=====================================");
    println!("Comprehensive wallet cleanup utility that sells all tokens for SOL and");
    println!("closes all Associated Token Accounts (ATAs) to reclaim rent SOL.");
    println!("");
    println!("⚠️  WARNING: This tool will attempt to sell ALL tokens in your wallet!");
    println!("    Use with extreme caution and understand the risks involved.");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_sell_all_and_close_atas [OPTIONS]");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h          Show this help message");
    println!("    --dry-run, -d      Simulate operations without executing transactions");
    println!("");
    println!("EXAMPLES:");
    println!("    # Simulate cleanup to see what would happen");
    println!("    cargo run --bin tool_sell_all_and_close_atas -- --dry-run");
    println!("");
    println!("    # Execute full wallet cleanup (DANGEROUS)");
    println!("    cargo run --bin tool_sell_all_and_close_atas");
    println!("");
    println!("OPERATIONS PERFORMED:");
    println!("    1. Scan wallet for all SPL Token and Token-2022 accounts");
    println!("    2. Identify tokens with non-zero balances");
    println!("    3. Sell all tokens for SOL using GMGN swap service");
    println!("    4. Close all Associated Token Accounts (empty and non-empty)");
    println!("    5. Reclaim rent SOL from closed ATAs (~0.00203928 SOL each)");
    println!("");
    println!("SAFETY FEATURES:");
    println!("    • Skips SOL (native token) - cannot sell SOL for SOL");
    println!("    • Validates token balances before attempting sales");
    println!("    • Detailed progress reporting for each operation");
    println!("    • Graceful error handling for failed transactions");
    println!("    • Supports both SPL Token and Token-2022 programs");
    println!("    • Concurrent processing with rate limiting");
    println!("");
    println!("ESTIMATED OUTCOMES:");
    println!("    • SOL received from token sales (varies by token values)");
    println!("    • Rent SOL reclaimed from closed ATAs");
    println!("    • Clean wallet with only SOL remaining");
    println!("");
    println!("RISK WARNINGS:");
    println!("    • Irreversible operation - tokens will be permanently sold");
    println!("    • Market slippage may result in lower SOL amounts");
    println!("    • Some tokens may fail to sell due to liquidity issues");
    println!("    • Failed transactions still consume transaction fees");
    println!("    • Use --dry-run first to understand the impact");
    println!("");
}

/// SOL token mint address (native Solana)
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Token account information from Solana RPC
#[derive(Debug, Clone)]
struct TokenAccount {
    pub mint: String,
    pub balance: u64,
    pub ui_amount: f64,
}

/// Main function to sell all tokens and close all ATAs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Check for help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    let dry_run = args.contains(&"--dry-run".to_string()) || args.contains(&"-d".to_string());

    log(LogTag::System, "INFO", "WALLET CLEANUP UTILITY");
    log(LogTag::System, "INFO", "========================");

    if dry_run {
        log(LogTag::System, "INFO", "DRY RUN MODE - No actual transactions will be made");
    }

    log(LogTag::System, "INFO", "This tool will:");
    log(LogTag::System, "INFO", "  - Scan for all token accounts (SPL & Token-2022)");
    if !dry_run {
        log(LogTag::System, "INFO", "  - Sell ALL tokens for SOL");
        log(LogTag::System, "INFO", "  - Close all Associated Token Accounts (ATAs)");
        log(LogTag::System, "INFO", "  - Reclaim rent SOL from closed ATAs");
    } else {
        log(LogTag::System, "INFO", "  - Show what tokens would be sold");
        log(LogTag::System, "INFO", "  - Show what ATAs would be closed");
        log(LogTag::System, "INFO", "  - Estimate rent SOL that would be reclaimed");
    }

    log(
        LogTag::System,
        "INFO",
        &format!("Starting comprehensive wallet cleanup{}", if dry_run { " (DRY RUN)" } else { "" })
    );

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    log(LogTag::System, "WALLET", &format!("Processing wallet: {}", wallet_address));

    // Step 1: Get all token accounts (both regular SPL and Token-2022)
    log(LogTag::System, "INFO", "Scanning for SPL Token accounts...");
    let mut token_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get SPL token accounts: {}", e));
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    log(LogTag::System, "INFO", "Scanning for Token-2022 accounts...");
    match get_token_2022_accounts(&wallet_address).await {
        Ok(mut token_2022_accounts) => {
            if !token_2022_accounts.is_empty() {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("Found {} Token-2022 accounts", token_2022_accounts.len())
                );
                token_accounts.append(&mut token_2022_accounts);
            }
        }
        Err(e) => {
            log(LogTag::System, "WARNING", &format!("Could not scan Token-2022 accounts: {}", e));
        }
    }

    if token_accounts.is_empty() {
        log(LogTag::System, "INFO", "No token accounts found - wallet is already clean!");
        return Ok(());
    }

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

    // Step 2: Sell all tokens with balances > 0
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
                match sell_token(&token, account.balance, None).await {
                    Ok(swap_result) => {
                        if swap_result.success {
                            // Calculate SOL output from output_amount string
                            let sol_output = swap_result.output_amount
                                .parse::<u64>()
                                .map(|lamports| (lamports as f64) / 1_000_000_000.0) // Convert lamports to SOL
                                .unwrap_or(0.0);

                            log(
                                LogTag::System,
                                "SELL_SUCCESS",
                                &format!(
                                    "Successfully sold {} for {:.6} SOL",
                                    account.mint,
                                    sol_output
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

    log(
        LogTag::System,
        "SELL_SUMMARY",
        &format!("Sales completed: {} success, {} failed", successful_sells, failed_sells)
    );

    // Step 3: Close all ATAs
    log(
        LogTag::System,
        "ATA_START",
        &format!("Starting ATA cleanup for {} accounts{}", token_accounts.len(), if dry_run {
            " (DRY RUN)"
        } else {
            ""
        })
    );

    // Filter accounts for ATA closing (skip SOL)
    let closable_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|account| {
            if account.mint == SOL_MINT {
                log(LogTag::System, "SKIP_SOL_ATA", "Skipping SOL account for ATA closing");
                return false;
            }
            true
        })
        .collect();

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
    let close_semaphore = Arc::new(Semaphore::new(3));
    let close_results: Vec<_> = stream
        ::iter(closable_accounts.iter())
        .map(|account| {
            let semaphore = close_semaphore.clone();
            let account = (*account).clone();
            let wallet_address = wallet_address.clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();

                if dry_run {
                    log(
                        LogTag::System,
                        "DRY_ATA",
                        &format!("Would close ATA for token: {}", account.mint)
                    );
                    return (account, true, Some("DRY_RUN_TX".to_string()));
                }

                log(
                    LogTag::System,
                    "ATA_START",
                    &format!("Starting ATA close for token: {}", account.mint)
                );

                match close_token_account(&account.mint, &wallet_address).await {
                    Ok(signature) => {
                        log(
                            LogTag::System,
                            "ATA_SUCCESS",
                            &format!(
                                "Successfully closed ATA for {}. TX: {}",
                                account.mint,
                                signature
                            )
                        );
                        (account, true, Some(signature))
                    }
                    Err(e) => {
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

    // Step 4: Final summary and cleanup report
    log(
        LogTag::System,
        "FINAL_REPORT",
        &format!("Final cleanup report: {} accounts found", token_accounts.len())
    );
    if dry_run {
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
    } else {
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

    if failed_sells > 0 {
        log(LogTag::System, "FAILED_SELLS", &format!("Found {} failed sells", failed_sells));
        for (account, success, error) in &sell_results {
            if !*success {
                let error_msg = error.as_deref().unwrap_or("Unknown error");
                log(
                    LogTag::System,
                    "SELL_FAIL_DETAIL",
                    &format!("Failed sell for {}: {}", account.mint, error_msg)
                );
            }
        }
    }

    if failed_closes > 0 {
        log(LogTag::System, "FAILED_CLOSES", &format!("Found {} failed ATA closes", failed_closes));
        for (account, success, _) in &close_results {
            if !*success {
                log(
                    LogTag::System,
                    "CLOSE_FAIL_DETAIL",
                    &format!("Failed ATA close for {}", account.mint)
                );
            }
        }
    }

    let estimated_rent_reclaimed = (successful_closes as f64) * 0.00203928; // ~0.002 SOL per ATA
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

    // Calculate expected vs actual counts, accounting for SOL being skipped
    let sol_accounts = token_accounts
        .iter()
        .filter(|a| a.mint == SOL_MINT)
        .count();
    let expected_operations = token_accounts.len() - sol_accounts;

    if successful_sells == expected_operations && successful_closes == expected_operations {
        // All operations successful
        if dry_run {
            log(
                LogTag::System,
                "DRY_RUN_COMPLETE",
                "All operations would succeed - dry run complete"
            );
        } else {
            log(LogTag::System, "CLEANUP_COMPLETE", "All tokens sold and ATAs closed successfully");
        }
    } else {
        if dry_run {
            log(
                LogTag::System,
                "DRY_RUN_ISSUES",
                &format!(
                    "Dry run completed with issues: {} sell failures, {} close failures",
                    failed_sells,
                    failed_closes
                )
            );
        } else {
            log(
                LogTag::System,
                "CLEANUP_ISSUES",
                &format!(
                    "Cleanup completed with issues: {} sell failures, {} close failures",
                    failed_sells,
                    failed_closes
                )
            );
        }
    }

    if dry_run {
        log(LogTag::System, "DRY_RUN_HINT", "To execute for real, run without --dry-run flag");
    }

    log(
        LogTag::System,
        "TOOL_COMPLETE",
        &format!("Tool execution finished: {} total accounts processed", token_accounts.len())
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
