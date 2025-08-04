//! # Sell Non-Position Tokens Utility
//!
//! This utility performs targeted cleanup by:
//! 1. Loading all open positions from positions.json
//! 2. Scanning wallet for all token accounts with balances > 0
//! 3. Identifying tokens that are NOT part of open positions
//! 4. Selling all non-position tokens for SOL
//! 5. Closing their Associated Token Accounts (ATAs) to reclaim rent SOL
//!
//! ## Usage
//! ```bash
//! # Preview what would be sold (dry run)
//! cargo run --bin tool_sell_non_positions -- --dry-run
//!
//! # Actually sell non-position tokens
//! cargo run --bin tool_sell_non_positions
//!
//! # Verbose output with detailed logging
//! cargo run --bin tool_sell_non_positions -- --verbose
//!
//! # Force sell even if some validations fail
//! cargo run --bin tool_sell_non_positions -- --force
//! ```
//!
//! ## Safety Features
//! - Only sells tokens NOT in open positions
//! - Skips SOL (native token) completely
//! - Validates token balances before selling
//! - Provides detailed progress reporting with position cross-reference
//! - Graceful error handling for failed operations
//! - Estimates rent SOL reclaimed from closed ATAs
//!
//! ## Configuration
//! Requires `configs.json` with wallet private key and RPC endpoints.
//! Reads open positions from `positions.json`.
//!
//! ## Use Cases
//! - Clean up tokens acquired outside the trading bot
//! - Remove dust tokens that didn't get properly tracked
//! - Reclaim rent SOL from forgotten token accounts
//! - Prepare wallet for fresh trading cycles

use screenerbot::{
    logger::{log, LogTag, init_file_logging},
    wallet::{
        get_wallet_address, 
        sell_token, 
        close_single_ata,
        get_all_token_accounts,
    },
    positions::get_open_positions,
    tokens::{
        api::{init_dexscreener_api, get_token_from_mint_global_api},
        Token,
    },
    rpc::init_rpc_client,
};
use std::{env, collections::HashSet};
use std::sync::Arc;
use tokio::sync::Semaphore;
use futures::stream::{self, StreamExt};

/// SOL token mint address (native Solana)
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Token account information with position tracking
#[derive(Debug, Clone)]
struct TokenAccountInfo {
    pub mint: String,
    pub balance: u64,
    pub ui_amount: f64, // Calculated from balance and decimals
    pub is_in_position: bool,
    pub position_symbol: Option<String>,
}

/// Results of the cleanup operation
#[derive(Debug)]
struct CleanupResults {
    pub total_tokens_found: usize,
    pub tokens_in_positions: usize,
    pub tokens_to_sell: usize,
    pub successful_sells: usize,
    pub failed_sells: usize,
    pub successful_ata_closes: usize,
    pub failed_ata_closes: usize,
    pub estimated_rent_reclaimed: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger first
    init_file_logging();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string()) || args.contains(&"-d".to_string());
    let verbose = args.contains(&"--verbose".to_string()) || args.contains(&"-v".to_string());
    let force = args.contains(&"--force".to_string()) || args.contains(&"-f".to_string());

    log(LogTag::System, "INFO", "🎯 NON-POSITION TOKENS CLEANUP UTILITY");
    log(LogTag::System, "INFO", "==========================================");

    if dry_run {
        log(LogTag::System, "MODE", "🔍 DRY RUN MODE - No actual transactions will be made");
    }
    if verbose {
        log(LogTag::System, "MODE", "📝 VERBOSE MODE - Detailed logging enabled");
    }
    if force {
        log(LogTag::System, "MODE", "⚠️ FORCE MODE - Will attempt operations even with warnings");
    }

    log(LogTag::System, "INFO", "This tool will:");
    log(LogTag::System, "INFO", "  1. Load open positions from positions.json");
    log(LogTag::System, "INFO", "  2. Scan wallet for all token accounts");
    log(LogTag::System, "INFO", "  3. Identify tokens NOT in open positions");
    if !dry_run {
        log(LogTag::System, "INFO", "  4. Sell all non-position tokens for SOL");
        log(LogTag::System, "INFO", "  5. Close ATAs to reclaim rent SOL");
    } else {
        log(LogTag::System, "INFO", "  4. Show what tokens would be sold");
        log(LogTag::System, "INFO", "  5. Show estimated rent SOL reclaim");
    }

    // Initialize required systems
    log(LogTag::System, "INIT", "🔧 Initializing systems...");
    
    // Initialize RPC client
    if let Err(e) = init_rpc_client() {
        log(LogTag::System, "ERROR", &format!("Failed to initialize RPC client: {}", e));
        return Err(e.into());
    }

    // Initialize DexScreener API for token metadata
    if let Err(e) = init_dexscreener_api().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize DexScreener API: {}", e));
        return Err(e.into());
    }

    log(LogTag::System, "SUCCESS", "✅ Systems initialized successfully");

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(e.into());
        }
    };

    log(LogTag::System, "WALLET", &format!("Processing wallet: {}", wallet_address));

    // Step 1: Load open positions
    log(LogTag::System, "POSITIONS", "📊 Loading open positions from positions.json...");
    let open_positions = get_open_positions();
    let open_position_mints: HashSet<String> = open_positions
        .iter()
        .map(|pos| pos.mint.clone())
        .collect();

    log(
        LogTag::System,
        "POSITIONS", 
        &format!("Found {} open positions:", open_positions.len())
    );

    if verbose {
        for position in &open_positions {
            log(
                LogTag::System,
                "POSITION",
                &format!(
                    "  📍 {} ({}) - Mint: {}...{}",
                    position.symbol,
                    position.name,
                    &position.mint[..8],
                    &position.mint[position.mint.len()-8..]
                )
            );
        }
    }

    // Step 2: Get all token accounts from wallet
    log(LogTag::System, "SCAN", "🔍 Scanning wallet for all token accounts...");
    let raw_token_accounts = match get_all_token_accounts(&wallet_address).await {
        Ok(accounts) => accounts,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get token accounts: {}", e));
            return Err(e.into());
        }
    };

    if raw_token_accounts.is_empty() {
        log(LogTag::System, "INFO", "✅ No token accounts found - wallet is clean!");
        return Ok(());
    }

    log(
        LogTag::System,
        "SCAN",
        &format!("Found {} total token accounts", raw_token_accounts.len())
    );

    // Step 3: Classify tokens and filter out SOL and zero balances
    log(LogTag::System, "CLASSIFY", "🏷️ Classifying tokens by position status...");
    let mut classified_tokens = Vec::new();

    for account in raw_token_accounts {
        // Skip SOL mint
        if account.mint == SOL_MINT {
            if verbose {
                log(LogTag::System, "SKIP", "Skipping SOL (native token)");
            }
            continue;
        }

        // Skip zero balance accounts
        if account.balance == 0 {
            if verbose {
                log(LogTag::System, "SKIP", &format!("Skipping {} (zero balance)", &account.mint[..8]));
            }
            continue;
        }

        // Check if this token is in an open position
        let is_in_position = open_position_mints.contains(&account.mint);
        let position_symbol = if is_in_position {
            open_positions
                .iter()
                .find(|pos| pos.mint == account.mint)
                .map(|pos| pos.symbol.clone())
        } else {
            None
        };

        let token_info = TokenAccountInfo {
            mint: account.mint.clone(),
            balance: account.balance,
            ui_amount: account.balance as f64 / 1_000_000_000.0, // Assume 9 decimals as default
            is_in_position,
            position_symbol: position_symbol.clone(),
        };

        classified_tokens.push(token_info.clone());

        if verbose {
            let status = if is_in_position { "IN POSITION" } else { "NON-POSITION" };
            let symbol = position_symbol.as_deref().unwrap_or("Unknown");
            log(
                LogTag::System,
                "CLASSIFY",
                &format!(
                    "  {} {} ({}) - Balance: {:.6} - Status: {}",
                    if is_in_position { "📍" } else { "🎯" },
                    symbol,
                    &account.mint[..8],
                    token_info.ui_amount,
                    status
                )
            );
        }
    }

    // Step 4: Separate tokens by position status
    let tokens_in_positions: Vec<_> = classified_tokens
        .iter()
        .filter(|token| token.is_in_position)
        .collect();

    let tokens_to_sell: Vec<_> = classified_tokens
        .iter()
        .filter(|token| !token.is_in_position)
        .collect();

    log(
        LogTag::System,
        "SUMMARY",
        &format!(
            "📊 CLASSIFICATION SUMMARY:\n  Total tokens with balance: {}\n  Tokens in open positions: {} (will be preserved)\n  Non-position tokens: {} (will be sold)",
            classified_tokens.len(),
            tokens_in_positions.len(),
            tokens_to_sell.len()
        )
    );

    if tokens_to_sell.is_empty() {
        log(LogTag::System, "SUCCESS", "✅ No non-position tokens found - all tokens are properly tracked!");
        return Ok(());
    }

    // Step 5: Display what will be sold
    log(LogTag::System, "TARGETS", "🎯 NON-POSITION TOKENS TO BE SOLD:");
    for (i, token) in tokens_to_sell.iter().enumerate() {
        log(
            LogTag::System,
            "TARGET",
            &format!(
                "  {}. Mint: {}...{} | Balance: {:.6} tokens | Raw: {}",
                i + 1,
                &token.mint[..8],
                &token.mint[token.mint.len()-8..],
                token.ui_amount,
                token.balance
            )
        );
    }

    // Calculate estimated rent reclaim
    let estimated_rent_reclaim = (tokens_to_sell.len() as f64) * 0.00203928; // ~0.002 SOL per ATA
    log(
        LogTag::System,
        "RENT",
        &format!(
            "💰 Estimated rent SOL reclaim: {:.6} SOL ({} ATAs × 0.00203928 SOL)",
            estimated_rent_reclaim,
            tokens_to_sell.len()
        )
    );

    if dry_run {
        log(LogTag::System, "DRY_RUN", "🔍 DRY RUN COMPLETE - No transactions were executed");
        log(LogTag::System, "DRY_RUN", "Run without --dry-run to execute the cleanup");
        return Ok(());
    }

    // Confirmation prompt for safety
    if !force {
        log(
            LogTag::System,
            "CONFIRM",
            &format!(
                "⚠️ READY TO SELL {} NON-POSITION TOKENS - Add --force flag to proceed",
                tokens_to_sell.len()
            )
        );
        return Ok(());
    }

    // Step 6: Execute token sales
    log(
        LogTag::System,
        "SELL_START",
        &format!("🚀 Starting sales for {} non-position tokens...", tokens_to_sell.len())
    );

    let mut results = CleanupResults {
        total_tokens_found: classified_tokens.len(),
        tokens_in_positions: tokens_in_positions.len(),
        tokens_to_sell: tokens_to_sell.len(),
        successful_sells: 0,
        failed_sells: 0,
        successful_ata_closes: 0,
        failed_ata_closes: 0,
        estimated_rent_reclaimed: estimated_rent_reclaim,
    };

    // Store the count for use in closures
    let total_sell_count = tokens_to_sell.len();

    // Process selling with 3 concurrent tasks
    let sell_semaphore = Arc::new(Semaphore::new(3));
    let sell_results: Vec<_> = stream::iter(tokens_to_sell.iter().enumerate())
        .map(|(i, token_info)| {
            let semaphore = sell_semaphore.clone();
            let token_info = (*token_info).clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                sell_non_position_token(i + 1, &token_info, total_sell_count).await
            }
        })
        .buffer_unordered(3)
        .collect()
        .await;

    // Count successful and failed sells
    results.successful_sells = sell_results.iter().filter(|&success| *success).count();
    results.failed_sells = sell_results.iter().filter(|&success| !*success).count();

    log(
        LogTag::System,
        "SELL_COMPLETE",
        &format!(
            "📈 SELLING COMPLETE: {} successful, {} failed",
            results.successful_sells,
            results.failed_sells
        )
    );

    // Step 7: Close ATAs for successfully sold tokens
    log(LogTag::System, "ATA_START", "🧹 Starting ATA cleanup...");

    // Get indices of successful sells for ATA closing
    let successful_sell_indices: Vec<_> = sell_results
        .iter()
        .enumerate()
        .filter_map(|(i, &success)| if success { Some(i) } else { None })
        .collect();

    let close_semaphore = Arc::new(Semaphore::new(3));
    let close_results: Vec<_> = stream::iter(successful_sell_indices.iter())
        .map(|&i| {
            let semaphore = close_semaphore.clone();
            let token_info = tokens_to_sell[i].clone();
            async move {
                let _permit = semaphore.acquire().await.unwrap();
                close_ata_for_token(i + 1, &token_info).await
            }
        })
        .buffer_unordered(3)
        .collect()
        .await;

    // Count successful and failed ATA closes
    results.successful_ata_closes = close_results.iter().filter(|&success| *success).count();
    results.failed_ata_closes = close_results.iter().filter(|&success| !*success).count();

    log(
        LogTag::System,
        "ATA_COMPLETE",
        &format!(
            "🧹 ATA CLEANUP COMPLETE: {} successful, {} failed",
            results.successful_ata_closes,
            results.failed_ata_closes
        )
    );

    // Step 8: Final summary
    print_final_summary(&results);

    log(LogTag::System, "SUCCESS", "🎉 Non-position token cleanup completed!");

    Ok(())
}

/// Sells a single non-position token
async fn sell_non_position_token(
    index: usize,
    token_info: &TokenAccountInfo,
    total_count: usize,
) -> bool {
    log(
        LogTag::Wallet,
        "SELL",
        &format!(
            "🔄 [{}/{}] Selling token {}...{} (Balance: {:.6})",
            index,
            total_count,
            &token_info.mint[..8],
            &token_info.mint[token_info.mint.len()-8..],
            token_info.ui_amount
        )
    );

    // Try to get token metadata for better logging
    let token = match get_token_from_mint_global_api(&token_info.mint).await {
        Ok(Some(token)) => token,
        Ok(None) | Err(_) => {
            // Create a minimal token struct if API call fails
            Token {
                mint: token_info.mint.clone(),
                symbol: format!("UNKNOWN_{}", &token_info.mint[..8]),
                name: "Unknown Token".to_string(),
                chain: "solana".to_string(),
                logo_url: None,
                coingecko_id: None,
                website: None,
                description: None,
                tags: Vec::new(),
                is_verified: false,
                created_at: None,
                price_dexscreener_sol: None,
                price_dexscreener_usd: None,
                price_pool_sol: None,
                price_pool_usd: None,
                dex_id: None,
                pair_address: None,
                pair_url: None,
                labels: Vec::new(),
                fdv: None,
                market_cap: None,
                txns: None,
                volume: None,
                price_change: None,
                liquidity: None,
                info: None,
                boosts: None,
            }
        }
    };

    // Attempt to sell all tokens in wallet (the sell_token function already does this)
    match sell_token(&token, token_info.balance, None).await {
        Ok(swap_result) => {
            if swap_result.success {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!(
                        "✅ [{}/{}] Successfully sold {} ({}) for SOL",
                        index,
                        total_count,
                        token.symbol,
                        &token_info.mint[..8]
                    )
                );
                true
            } else {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!(
                        "❌ [{}/{}] Swap failed for {} ({}): {}",
                        index,
                        total_count,
                        token.symbol,
                        &token_info.mint[..8],
                        swap_result.error.unwrap_or_else(|| "Unknown error".to_string())
                    )
                );
                false
            }
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!(
                    "❌ [{}/{}] Failed to sell {} ({}): {}",
                    index,
                    total_count,
                    token.symbol,
                    &token_info.mint[..8],
                    e
                )
            );
            false
        }
    }
}

/// Closes ATA for a successfully sold token
async fn close_ata_for_token(_index: usize, token_info: &TokenAccountInfo) -> bool {
    log(
        LogTag::Wallet,
        "ATA",
        &format!(
            "🧹 Closing ATA for token {}...{}",
            &token_info.mint[..8],
            &token_info.mint[token_info.mint.len()-8..]
        )
    );

    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to get wallet address for ATA close: {}", e)
            );
            return false;
        }
    };

    match close_single_ata(&wallet_address, &token_info.mint).await {
        Ok(signature) => {
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!(
                    "✅ ATA closed for {} | TX: {}",
                    &token_info.mint[..8],
                    signature
                )
            );
            true
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "ERROR",
                &format!(
                    "❌ Failed to close ATA for {}: {}",
                    &token_info.mint[..8],
                    e
                )
            );
            false
        }
    }
}

/// Prints comprehensive final summary
fn print_final_summary(results: &CleanupResults) {
    log(LogTag::System, "SUMMARY", "");
    log(LogTag::System, "SUMMARY", "🎯 NON-POSITION TOKEN CLEANUP SUMMARY");
    log(LogTag::System, "SUMMARY", "==========================================");
    
    // Token analysis
    log(
        LogTag::System,
        "SUMMARY",
        &format!(
            "📊 TOKEN ANALYSIS:\n  Total tokens found: {}\n  Tokens in positions: {} (preserved)\n  Non-position tokens: {} (targeted for cleanup)",
            results.total_tokens_found,
            results.tokens_in_positions,
            results.tokens_to_sell
        )
    );

    // Selling results
    log(
        LogTag::System,
        "SUMMARY",
        &format!(
            "💰 SELLING RESULTS:\n  Successful sells: {}\n  Failed sells: {}\n  Success rate: {:.1}%",
            results.successful_sells,
            results.failed_sells,
            if results.tokens_to_sell > 0 {
                (results.successful_sells as f64 / results.tokens_to_sell as f64) * 100.0
            } else {
                0.0
            }
        )
    );

    // ATA cleanup results
    log(
        LogTag::System,
        "SUMMARY",
        &format!(
            "🧹 ATA CLEANUP RESULTS:\n  Successful closes: {}\n  Failed closes: {}\n  Success rate: {:.1}%",
            results.successful_ata_closes,
            results.failed_ata_closes,
            if results.successful_sells > 0 {
                (results.successful_ata_closes as f64 / results.successful_sells as f64) * 100.0
            } else {
                0.0
            }
        )
    );

    // Rent reclaimed
    let actual_rent_reclaimed = (results.successful_ata_closes as f64) * 0.00203928;
    log(
        LogTag::System,
        "SUMMARY",
        &format!(
            "💎 RENT SOL RECLAIMED: {:.6} SOL ({} ATAs closed)",
            actual_rent_reclaimed,
            results.successful_ata_closes
        )
    );

    // Overall status
    let overall_success = results.failed_sells == 0 && results.failed_ata_closes == 0;
    if overall_success {
        log(LogTag::System, "SUCCESS", "✅ Complete success! All non-position tokens cleaned up.");
    } else {
        log(
            LogTag::System,
            "PARTIAL",
            &format!(
                "⚠️ Partial success. {} total failures. Check logs for details.",
                results.failed_sells + results.failed_ata_closes
            )
        );
    }

    if results.tokens_in_positions > 0 {
        log(
            LogTag::System,
            "INFO",
            &format!(
                "📍 {} tokens remain in wallet as part of open positions (preserved)",
                results.tokens_in_positions
            )
        );
    }
}
