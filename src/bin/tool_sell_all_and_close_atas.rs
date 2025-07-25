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

use screenerbot::global::{ Token, read_configs };
use screenerbot::logger::{ log, LogTag };
use screenerbot::wallet::{ get_wallet_address, sell_token, close_token_account, SwapError };
use reqwest;
use serde_json;
use std::env;

/// SOL token mint address (native Solana)
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Token account information from Solana RPC
#[derive(Debug, Clone)]
struct TokenAccount {
    pub mint: String,
    pub balance: u64,
    pub decimals: u8,
    pub ui_amount: f64,
}

/// Main function to sell all tokens and close all ATAs
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string()) || args.contains(&"-d".to_string());

    println!("⚠️  WALLET CLEANUP UTILITY ⚠️");
    println!("===============================");

    if dry_run {
        println!("🔍 DRY RUN MODE - No actual transactions will be made");
    }

    println!("This tool will:");
    println!("  🔍 Scan for all token accounts (SPL & Token-2022)");
    if !dry_run {
        println!("  💰 Sell ALL tokens for SOL");
        println!("  🔒 Close all Associated Token Accounts (ATAs)");
        println!("  💎 Reclaim rent SOL from closed ATAs");
    } else {
        println!("  💰 Show what tokens would be sold");
        println!("  🔒 Show what ATAs would be closed");
        println!("  💎 Estimate rent SOL that would be reclaimed");
    }
    println!("");

    if !dry_run {
        println!("❗ WARNING: This will sell ALL tokens in your wallet!");
        println!("❗ Make sure you want to do this before continuing.");
        println!("❗ Add --dry-run flag to see what would happen without executing.");
        println!("");
        print!("Type 'YES' to confirm or anything else to cancel: ");

        use std::io::{ self, Write };
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim() != "YES" {
            println!("❌ Operation cancelled by user.");
            return Ok(());
        }
    }

    println!("🚀 Starting comprehensive wallet cleanup{}...", if dry_run {
        " (DRY RUN)"
    } else {
        ""
    });

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("❌ Failed to get wallet address: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    log(LogTag::System, "WALLET", &format!("Processing wallet: {}", wallet_address));

    // Step 1: Get all token accounts (both regular SPL and Token-2022)
    println!("🔍 Scanning for SPL Token accounts...");
    let mut token_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            eprintln!("❌ Failed to get SPL token accounts: {}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    println!("🔍 Scanning for Token-2022 accounts...");
    match get_token_2022_accounts(&wallet_address).await {
        Ok(mut token_2022_accounts) => {
            if !token_2022_accounts.is_empty() {
                println!("📊 Found {} Token-2022 accounts", token_2022_accounts.len());
                token_accounts.append(&mut token_2022_accounts);
            }
        }
        Err(e) => {
            println!("⚠️  Warning: Could not scan Token-2022 accounts: {}", e);
        }
    }

    if token_accounts.is_empty() {
        println!("✅ No token accounts found - wallet is already clean!");
        return Ok(());
    }

    println!("📊 Found {} token accounts:", token_accounts.len());
    for account in &token_accounts {
        println!(
            "  🪙 {} ({:.6} tokens) - Mint: {}",
            account.mint[..8].to_string() + "...",
            account.ui_amount,
            account.mint
        );
    }

    // Step 2: Sell all tokens with balances > 0
    let mut sell_results = Vec::new();
    let mut successful_sells = 0;
    let mut failed_sells = 0;

    println!("\n💰 Starting token sales{}...", if dry_run { " (DRY RUN)" } else { "" });

    for account in &token_accounts {
        if account.balance == 0 {
            println!("⏭️  Skipping {} - zero balance", account.mint[..8].to_string() + "...");
            continue;
        }

        // Skip SOL (native token) - can't sell SOL for SOL
        if account.mint == SOL_MINT {
            println!("⏭️  Skipping SOL (native token)");
            continue;
        }

        if dry_run {
            println!(
                "🔄 Would sell {:.6} tokens of {}",
                account.ui_amount,
                account.mint[..8].to_string() + "..."
            );
            sell_results.push((account.clone(), true, None));
            successful_sells += 1;
            continue;
        }

        println!(
            "🔄 Selling {:.6} tokens of {}...",
            account.ui_amount,
            account.mint[..8].to_string() + "..."
        );

        // Create a minimal Token struct for the sell operation
        let token = Token {
            mint: account.mint.clone(),
            symbol: format!("TOKEN_{}", &account.mint[..8]),
            name: format!("Unknown Token {}", &account.mint[..8]),
            decimals: account.decimals,
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
                    // Calculate SOL output from actual_output_change and decimals
                    let sol_output = if let Some(output_change) = swap_result.actual_output_change {
                        (output_change as f64) / 1_000_000_000.0 // Convert lamports to SOL
                    } else {
                        0.0
                    };

                    println!(
                        "✅ Successfully sold {} for {:.6} SOL",
                        account.mint[..8].to_string() + "...",
                        sol_output
                    );
                    sell_results.push((account.clone(), true, None));
                    successful_sells += 1;
                } else {
                    println!(
                        "❌ Sell failed for {}: {}",
                        account.mint[..8].to_string() + "...",
                        swap_result.error.as_deref().unwrap_or("Unknown error")
                    );
                    sell_results.push((account.clone(), false, swap_result.error.clone()));
                    failed_sells += 1;
                }
            }
            Err(e) => {
                println!("❌ Sell error for {}: {}", account.mint[..8].to_string() + "...", e);
                sell_results.push((account.clone(), false, Some(e.to_string())));
                failed_sells += 1;
            }
        }

        // Small delay between sales to avoid rate limiting
        if !dry_run {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }

    println!("\n📈 Sales Summary:");
    println!("  ✅ Successful sells: {}", successful_sells);
    println!("  ❌ Failed sells: {}", failed_sells);

    // Step 3: Close all ATAs
    println!("\n🔒 Starting ATA cleanup{}...", if dry_run { " (DRY RUN)" } else { "" });

    let mut close_results = Vec::new();
    let mut successful_closes = 0;
    let mut failed_closes = 0;

    for account in &token_accounts {
        // Skip SOL accounts
        if account.mint == SOL_MINT {
            continue;
        }

        if dry_run {
            println!("🔄 Would close ATA for {}", account.mint[..8].to_string() + "...");
            close_results.push((account.clone(), true, Some("DRY_RUN_TX".to_string())));
            successful_closes += 1;
            continue;
        }

        println!("🔄 Closing ATA for {}...", account.mint[..8].to_string() + "...");

        match close_token_account(&account.mint, &wallet_address).await {
            Ok(signature) => {
                println!(
                    "✅ Successfully closed ATA for {}. TX: {}",
                    account.mint[..8].to_string() + "...",
                    signature
                );
                close_results.push((account.clone(), true, Some(signature)));
                successful_closes += 1;
            }
            Err(e) => {
                println!(
                    "❌ Failed to close ATA for {}: {}",
                    account.mint[..8].to_string() + "...",
                    e
                );
                close_results.push((account.clone(), false, None));
                failed_closes += 1;
            }
        }

        // Small delay between ATA closes
        if !dry_run {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }

    println!("\n🔒 ATA Cleanup Summary:");
    println!("  ✅ Successful closes: {}", successful_closes);
    println!("  ❌ Failed closes: {}", failed_closes);

    // Step 4: Final summary and cleanup report
    println!("\n🎯 FINAL CLEANUP REPORT{}", if dry_run { " (DRY RUN)" } else { "" });
    println!("========================");
    println!("📊 Token Accounts Found: {}", token_accounts.len());
    if dry_run {
        println!("💰 Would Sell - Success: {} | Failed: {}", successful_sells, failed_sells);
        println!(
            "🔒 Would Close ATAs - Success: {} | Failed: {}",
            successful_closes,
            failed_closes
        );
    } else {
        println!("💰 Sales - Success: {} | Failed: {}", successful_sells, failed_sells);
        println!("🔒 ATA Closes - Success: {} | Failed: {}", successful_closes, failed_closes);
    }

    if failed_sells > 0 {
        println!("\n❌ FAILED SELLS:");
        for (account, success, error) in &sell_results {
            if !success {
                println!(
                    "  🪙 {} - {}",
                    account.mint[..8].to_string() + "...",
                    error.as_deref().unwrap_or("Unknown error")
                );
            }
        }
    }

    if failed_closes > 0 {
        println!("\n❌ FAILED ATA CLOSES:");
        for (account, success, _) in &close_results {
            if !success {
                println!("  🪙 {}", account.mint[..8].to_string() + "...");
            }
        }
    }

    let estimated_rent_reclaimed = (successful_closes as f64) * 0.00203928; // ~0.002 SOL per ATA
    if dry_run {
        println!(
            "\n💎 Estimated rent SOL that would be reclaimed: {:.6} SOL",
            estimated_rent_reclaimed
        );
    } else {
        println!("\n💎 Estimated rent SOL reclaimed: {:.6} SOL", estimated_rent_reclaimed);
    }

    if
        successful_sells == token_accounts.len() - 1 &&
        successful_closes == token_accounts.len() - 1
    {
        // -1 because we skip SOL accounts
        if dry_run {
            println!("\n🎉 DRY RUN COMPLETE! All tokens would be sold and ATAs closed.");
        } else {
            println!("\n🎉 WALLET CLEANUP COMPLETE! All tokens sold and ATAs closed.");
        }
    } else {
        if dry_run {
            println!("\n⚠️  Dry run completed with some potential failures. See details above.");
        } else {
            println!("\n⚠️  Cleanup completed with some failures. See details above.");
        }
    }

    if dry_run {
        println!("\n💡 To execute these operations for real, run without --dry-run flag.");
    }

    Ok(())
}

/// Gets all token accounts for the given wallet address
async fn get_all_token_accounts(wallet_address: &str) -> Result<Vec<TokenAccount>, SwapError> {
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

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

                                                    let decimals = token_amount
                                                        .and_then(|ta| ta.get("decimals"))
                                                        .and_then(|d| d.as_u64())
                                                        .unwrap_or(0) as u8;

                                                    let ui_amount = token_amount
                                                        .and_then(|ta| ta.get("uiAmount"))
                                                        .and_then(|ua| ua.as_f64())
                                                        .unwrap_or(0.0);

                                                    if !mint.is_empty() {
                                                        token_accounts.push(TokenAccount {
                                                            mint,
                                                            balance,
                                                            decimals,
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
    let configs = read_configs("configs.json").map_err(|e| SwapError::ConfigError(e.to_string()))?;

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
                                        let (Some(pubkey), Some(account_data)) = (
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

                                                    let decimals = token_amount
                                                        .and_then(|ta| ta.get("decimals"))
                                                        .and_then(|d| d.as_u64())
                                                        .unwrap_or(0) as u8;

                                                    let ui_amount = token_amount
                                                        .and_then(|ta| ta.get("uiAmount"))
                                                        .and_then(|ua| ua.as_f64())
                                                        .unwrap_or(0.0);

                                                    if !mint.is_empty() {
                                                        token_accounts.push(TokenAccount {
                                                            mint,
                                                            balance,
                                                            decimals,
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
