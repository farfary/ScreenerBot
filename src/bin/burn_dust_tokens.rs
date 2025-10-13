/// Dust Token Burner Tool
///
/// This tool scans the wallet for dust tokens (very small amounts) and burns them
/// by swapping to SOL using ExactOut mode to ensure complete liquidation.
/// After burning, it closes empty ATAs to reclaim rent (~0.002 SOL per ATA).
///
/// Features:
/// - Detects dust tokens based on configurable thresholds
/// - Uses ExactOut swap mode for complete liquidation
/// - Automatically closes empty ATAs after burning
/// - Safe operation with confirmation prompts
/// - Detailed logging and reporting
/// - Dry-run mode for testing
use screenerbot::{
    constants::SOL_MINT,
    logger::{init_file_logging, log, LogTag},
    rpc::TokenAccountInfo,
    swaps::{execute_best_swap, get_best_quote},
    tokens::{get_token_decimals, Token},
    utils::{close_single_ata, get_all_token_accounts, get_wallet_address},
};
use tokio;

/// Configuration for dust detection
const DUST_THRESHOLD_UI: f64 = 0.001; // UI amount threshold (e.g., 0.001 tokens)
const DUST_THRESHOLD_VALUE_SOL: f64 = 0.0001; // Value threshold in SOL (e.g., $0.01 worth)
const MIN_SWAP_AMOUNT_LAMPORTS: u64 = 1000; // Minimum raw token amount to attempt swap
const SLIPPAGE_PERCENT: f64 = 10.0; // Higher slippage for dust tokens
const BATCH_SIZE: usize = 5; // Process tokens in batches to avoid overwhelming RPC

/// Statistics for the burn operation
#[derive(Debug, Default)]
struct BurnStats {
    total_tokens_found: usize,
    dust_tokens_detected: usize,
    tokens_burned: usize,
    atas_closed: usize,
    total_rent_reclaimed: f64,
    total_sol_received: f64,
    errors: Vec<String>,
}

/// Dust token information
#[derive(Debug, Clone)]
struct DustToken {
    mint: String,
    balance: u64,
    balance_ui: f64,
    decimals: u8,
    estimated_value_sol: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    log(
        LogTag::Swap,
        "BURN_DUST_START",
        "🔥 Starting dust token burn tool...",
    );

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.contains(&"--dry-run".to_string());
    let force = args.contains(&"--force".to_string());
    let help = args.contains(&"--help".to_string()) || args.contains(&"-h".to_string());

    if help {
        print_help();
        return Ok(());
    }

    if dry_run {
        log(
            LogTag::Swap,
            "DRY_RUN",
            "📋 Running in DRY RUN mode - no actual transactions will be sent",
        );
    }

    // Get wallet address
    let wallet_address =
        get_wallet_address().map_err(|e| format!("Failed to get wallet address: {}", e))?;

    log(
        LogTag::Swap,
        "WALLET",
        &format!("🏦 Wallet address: {}", wallet_address),
    );

    // Step 1: Scan for all token accounts
    let mut stats = BurnStats::default();

    log(
        LogTag::Swap,
        "SCAN_START",
        "🔍 Scanning wallet for token accounts...",
    );
    let token_accounts = get_all_token_accounts(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    stats.total_tokens_found = token_accounts.len();
    log(
        LogTag::Swap,
        "SCAN_COMPLETE",
        &format!("📊 Found {} token accounts", stats.total_tokens_found),
    );

    if token_accounts.is_empty() {
        log(
            LogTag::Swap,
            "NO_TOKENS",
            "✅ No token accounts found - wallet is clean",
        );
        return Ok(());
    }

    // Step 2: Identify dust tokens
    log(
        LogTag::Swap,
        "DUST_DETECTION",
        "🧹 Analyzing tokens for dust detection...",
    );
    let dust_tokens = identify_dust_tokens(token_accounts).await?;
    stats.dust_tokens_detected = dust_tokens.len();

    if dust_tokens.is_empty() {
        log(
            LogTag::Swap,
            "NO_DUST",
            "✅ No dust tokens detected - wallet is optimized",
        );
        return Ok(());
    }

    // Display dust tokens found
    log(
        LogTag::Swap,
        "DUST_FOUND",
        &format!("🗑️ Detected {} dust tokens:", dust_tokens.len()),
    );
    for (i, dust) in dust_tokens.iter().enumerate() {
        log(
            LogTag::Swap,
            "DUST_TOKEN",
            &format!(
                "  {}. {} - {} tokens ({}), Value: {:.6} SOL",
                i + 1,
                &dust.mint[..8],
                dust.balance_ui,
                dust.balance,
                dust.estimated_value_sol.unwrap_or(0.0)
            ),
        );
    }

    // Confirmation prompt (unless force or dry-run)
    if !force && !dry_run {
        if !confirm_burn_operation(&dust_tokens) {
            log(LogTag::Swap, "CANCELLED", "❌ Operation cancelled by user");
            return Ok(());
        }
    }

    // Step 3: Burn dust tokens
    if dry_run {
        log(
            LogTag::Swap,
            "DRY_RUN_COMPLETE",
            &format!(
                "📋 Dry run complete - would burn {} dust tokens",
                dust_tokens.len()
            ),
        );
        print_dust_summary(&dust_tokens);
        return Ok(());
    }

    log(
        LogTag::Swap,
        "BURN_START",
        "🔥 Starting dust token burn operation...",
    );
    burn_dust_tokens(&dust_tokens, &wallet_address, &mut stats).await?;

    // Step 4: Final cleanup - close any remaining empty ATAs
    log(
        LogTag::Swap,
        "CLEANUP_START",
        "🧹 Performing final ATA cleanup...",
    );
    cleanup_empty_atas(&wallet_address, &mut stats).await?;

    // Step 5: Print final report
    print_final_report(&stats);

    log(
        LogTag::Swap,
        "BURN_COMPLETE",
        "✅ Dust token burn operation completed successfully",
    );
    Ok(())
}

/// Identify dust tokens based on configured thresholds
async fn identify_dust_tokens(
    token_accounts: Vec<TokenAccountInfo>,
) -> Result<Vec<DustToken>, Box<dyn std::error::Error>> {
    let mut dust_tokens = Vec::new();

    for account in token_accounts {
        // Skip SOL (should not appear in token accounts anyway)
        if account.mint == SOL_MINT {
            continue;
        }

        // Skip if balance is zero (will be handled by ATA cleanup)
        if account.balance == 0 {
            continue;
        }

        // Skip if balance is too small to even attempt a swap
        if account.balance < MIN_SWAP_AMOUNT_LAMPORTS {
            continue;
        }

        // Get decimals for UI calculation
        let decimals = match get_token_decimals(&account.mint).await {
            Some(dec) => dec,
            None => {
                log(
                    LogTag::Swap,
                    "DECIMALS_MISSING",
                    &format!(
                        "⚠️ No decimals cached for token {}, skipping",
                        &account.mint[..8]
                    ),
                );
                continue;
            }
        };

        // Calculate UI amount
        let balance_ui = (account.balance as f64) / (10_f64).powi(decimals as i32);

        // Check if it's dust based on UI amount threshold
        let is_dust_by_amount = balance_ui <= DUST_THRESHOLD_UI;

        // TODO: Could add price checking here to determine SOL value
        // For now, we'll use amount-based detection only
        let estimated_value_sol = None;

        if is_dust_by_amount {
            dust_tokens.push(DustToken {
                mint: account.mint.clone(),
                balance: account.balance,
                balance_ui,
                decimals,
                estimated_value_sol,
            });
        }
    }

    Ok(dust_tokens)
}

/// Burn dust tokens using ExactOut swaps
async fn burn_dust_tokens(
    dust_tokens: &[DustToken],
    wallet_address: &str,
    stats: &mut BurnStats,
) -> Result<(), Box<dyn std::error::Error>> {
    for (i, dust) in dust_tokens.iter().enumerate() {
        log(
            LogTag::Swap,
            "BURN_TOKEN",
            &format!(
                "🔥 Burning token {}/{}: {} ({} tokens)",
                i + 1,
                dust_tokens.len(),
                &dust.mint[..8],
                dust.balance_ui
            ),
        );

        // Create token object for swap
        let token = Token {
            mint: dust.mint.clone(),
            symbol: format!("DUST_{}", &dust.mint[..8]),
            name: format!("Dust Token {}", &dust.mint[..8]),
            chain: "solana".to_string(),
            decimals: Some(dust.decimals),
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
            last_updated: chrono::Utc::now(),
        };

        // Get quote using ExactOut mode for complete liquidation
        match get_best_quote(
            &dust.mint,       // input_mint (token)
            SOL_MINT,         // output_mint (SOL)
            dust.balance,     // input_amount (all tokens)
            wallet_address,   // from_address
            SLIPPAGE_PERCENT, // slippage
            "ExactOut",       // swap_mode - use ExactOut for complete liquidation
        )
        .await
        {
            Ok(quote) => {
                log(
                    LogTag::Swap,
                    "QUOTE_SUCCESS",
                    &format!(
                        "💱 Quote obtained: {} lamports SOL, impact: {:.2}%",
                        quote.output_amount, quote.price_impact_pct
                    ),
                );

                // Execute the swap
                match execute_best_swap(&token, &dust.mint, SOL_MINT, dust.balance, quote).await {
                    Ok(swap_result) => {
                        if swap_result.success {
                            let sol_received =
                                (swap_result.output_amount.parse::<u64>().unwrap_or(0) as f64)
                                    / 1_000_000_000.0;
                            stats.tokens_burned += 1;
                            stats.total_sol_received += sol_received;

                            log(
                                LogTag::Swap,
                                "BURN_SUCCESS",
                                &format!(
                                    "✅ Burned {} tokens, received {:.6} SOL - Tx: {}",
                                    dust.balance_ui,
                                    sol_received,
                                    swap_result.transaction_signature.unwrap_or_default()
                                ),
                            );
                        } else {
                            let error_msg = format!(
                                "Swap failed for token {}: {}",
                                &dust.mint[..8],
                                swap_result.error.unwrap_or_default()
                            );
                            stats.errors.push(error_msg.clone());
                            log(LogTag::Swap, "BURN_FAILED", &format!("❌ {}", error_msg));
                        }
                    }
                    Err(e) => {
                        let error_msg =
                            format!("Swap execution failed for token {}: {}", &dust.mint[..8], e);
                        stats.errors.push(error_msg.clone());
                        log(LogTag::Swap, "BURN_ERROR", &format!("❌ {}", error_msg));
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Quote failed for token {}: {}", &dust.mint[..8], e);
                stats.errors.push(error_msg.clone());
                log(LogTag::Swap, "QUOTE_ERROR", &format!("❌ {}", error_msg));
            }
        }

        // Add delay between burns to avoid overwhelming RPC
        if i < dust_tokens.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
    }

    Ok(())
}

/// Clean up any remaining empty ATAs after burning
async fn cleanup_empty_atas(
    wallet_address: &str,
    stats: &mut BurnStats,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get all token accounts again to find empty ones
    let token_accounts = get_all_token_accounts(wallet_address).await?;
    let empty_accounts: Vec<_> = token_accounts
        .iter()
        .filter(|acc| acc.balance == 0)
        .collect();

    if empty_accounts.is_empty() {
        log(LogTag::Swap, "NO_EMPTY_ATAS", "✅ No empty ATAs found");
        return Ok(());
    }

    log(
        LogTag::Swap,
        "CLOSING_ATAS",
        &format!("🧹 Closing {} empty ATAs...", empty_accounts.len()),
    );

    for account in empty_accounts {
        match close_single_ata(wallet_address, &account.mint).await {
            Ok(signature) => {
                stats.atas_closed += 1;
                stats.total_rent_reclaimed += 0.00203928; // Standard ATA rent

                log(
                    LogTag::Swap,
                    "ATA_CLOSED",
                    &format!(
                        "✅ Closed ATA for {} - Tx: {} - Rent: 0.00203928 SOL",
                        &account.mint[..8],
                        signature
                    ),
                );
            }
            Err(e) => {
                let error_msg = format!("Failed to close ATA for {}: {}", &account.mint[..8], e);
                stats.errors.push(error_msg.clone());
                log(
                    LogTag::Swap,
                    "ATA_CLOSE_ERROR",
                    &format!("❌ {}", error_msg),
                );
            }
        }

        // Rate limit ATA closures
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }

    Ok(())
}

/// Print help information
fn print_help() {
    println!("\n🔥 Dust Token Burner Tool");
    println!("========================");
    println!(
        "\nBurns dust tokens (very small amounts) by swapping them to SOL and closes empty ATAs to reclaim rent."
    );
    println!("\nUSAGE:");
    println!("  cargo run --bin burn_dust_tokens [OPTIONS]");
    println!("\nOPTIONS:");
    println!("  --dry-run    Show what would be burned without executing transactions");
    println!("  --force      Skip confirmation prompts");
    println!("  --help, -h   Show this help message");
    println!("\nDUST DETECTION CRITERIA:");
    println!("  - Token amount ≤ {} UI tokens", DUST_THRESHOLD_UI);
    println!(
        "  - Raw balance ≥ {} lamports (minimum for swap)",
        MIN_SWAP_AMOUNT_LAMPORTS
    );
    println!("  - Excludes tokens without cached decimals");
    println!("\nSAFETY FEATURES:");
    println!("  - Uses ExactOut swap mode for complete liquidation");
    println!("  - Confirmation prompts (unless --force)");
    println!("  - Detailed logging and error handling");
    println!("  - Rate limiting between operations");
    println!("\nEXAMPLES:");
    println!("  cargo run --bin burn_dust_tokens --dry-run");
    println!("  cargo run --bin burn_dust_tokens --force");
    println!();
}

/// Prompt user for confirmation
fn confirm_burn_operation(dust_tokens: &[DustToken]) -> bool {
    println!("\n⚠️  DUST TOKEN BURN CONFIRMATION");
    println!("================================");
    println!("You are about to burn {} dust tokens.", dust_tokens.len());
    println!("This will swap them to SOL using ExactOut mode and close empty ATAs.");
    println!("\nTokens to burn:");

    for (i, dust) in dust_tokens.iter().enumerate() {
        println!(
            "  {}. {} - {} tokens",
            i + 1,
            &dust.mint[..8],
            dust.balance_ui
        );
    }

    println!(
        "\nEstimated rent to reclaim: ~{:.6} SOL from ATA closures",
        (dust_tokens.len() as f64) * 0.00203928
    );

    print!("\nDo you want to proceed? (y/N): ");
    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    let response = input.trim().to_lowercase();
    response == "y" || response == "yes"
}

/// Print dry run summary
fn print_dust_summary(dust_tokens: &[DustToken]) {
    println!("\n📋 DRY RUN SUMMARY");
    println!("=================");
    println!("Tokens that would be burned: {}", dust_tokens.len());
    println!(
        "Estimated rent to reclaim: ~{:.6} SOL",
        (dust_tokens.len() as f64) * 0.00203928
    );

    println!("\nDust tokens:");
    for (i, dust) in dust_tokens.iter().enumerate() {
        println!(
            "  {}. {} - {} tokens ({} raw)",
            i + 1,
            &dust.mint[..8],
            dust.balance_ui,
            dust.balance
        );
    }
}

/// Print final operation report
fn print_final_report(stats: &BurnStats) {
    println!("\n🎯 BURN OPERATION REPORT");
    println!("========================");
    println!("Total tokens found: {}", stats.total_tokens_found);
    println!("Dust tokens detected: {}", stats.dust_tokens_detected);
    println!("Tokens successfully burned: {}", stats.tokens_burned);
    println!("ATAs closed: {}", stats.atas_closed);
    println!("Total SOL received: {:.6}", stats.total_sol_received);
    println!("Total rent reclaimed: {:.6}", stats.total_rent_reclaimed);
    println!(
        "Total benefit: {:.6} SOL",
        stats.total_sol_received + stats.total_rent_reclaimed
    );

    if !stats.errors.is_empty() {
        println!("\n❌ ERRORS ({}):", stats.errors.len());
        for (i, error) in stats.errors.iter().enumerate() {
            println!("  {}. {}", i + 1, error);
        }
    }

    if stats.tokens_burned > 0 || stats.atas_closed > 0 {
        println!("\n✅ Operation completed successfully!");
    }
}
