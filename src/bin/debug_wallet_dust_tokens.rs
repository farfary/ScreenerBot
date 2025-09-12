/// Wallet Dust Token Analysis Tool
///
/// This tool analyzes all token balances and ATAs in the wallet to identify:
/// 1. Small amounts (dust) that cannot be sold
/// 2. Tokens that can be burned
/// 3. Tokens with disabled transfer authority
/// 4. Recommended cleanup actions
///
/// Usage:
///   cargo run --bin debug_wallet_dust_tokens [--min-value-usd <amount>] [--json] [--dry-run]
///
/// Flags:
///   --min-value-usd <amount>  Minimum USD value to consider (default: 0.01)
///   --json                    Output in JSON format
///   --dry-run                 Only analyze, don't perform any cleanup actions
///   --show-zero               Include zero balance accounts
///   --help                    Show this help message

use screenerbot::{
    arguments::{ get_arg_value, has_arg },
    logger::{ init_file_logging, log, LogTag },
    rpc::{ get_rpc_client, TokenAccountInfo },
    tokens::{
        authority::{ get_token_authorities, TokenAuthorities },
        decimals::get_cached_decimals,
    },
    utils::{ get_wallet_address, safe_truncate },
};
use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };

#[derive(Debug, Serialize, Deserialize)]
struct WalletDustAnalysis {
    pub timestamp: DateTime<Utc>,
    pub wallet_address: String,
    pub summary: DustSummary,
    pub dust_tokens: Vec<DustTokenInfo>,
    pub recommendations: Vec<String>,
    pub total_cleanup_value_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DustSummary {
    pub total_token_accounts: usize,
    pub dust_tokens_count: usize,
    pub zero_balance_accounts: usize,
    pub burnable_tokens: usize,
    pub transfer_disabled_tokens: usize,
    pub total_dust_value_usd: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct DustTokenInfo {
    pub mint: String,
    pub ata_address: String,
    pub balance: u64,
    pub balance_ui: f64,
    pub decimals: Option<u8>,
    pub value_usd: Option<f64>,
    pub is_dust: bool,
    pub is_zero_balance: bool,
    pub can_burn: bool,
    pub transfer_disabled: bool,
    pub mint_disabled: bool,
    pub freeze_disabled: bool,
    pub authorities: Option<TokenAuthorities>,
    pub recommended_action: String,
    pub cleanup_priority: u8, // 1-10, higher = more urgent
}

#[tokio::main]
async fn main() {
    // Initialize logging
    init_file_logging();

    // Check for help
    if has_arg("--help") {
        print_help();
        return;
    }

    log(LogTag::System, "INFO", "🔍 Starting wallet dust token analysis...");

    // Parse arguments
    let min_value_usd = get_arg_value("--min-value-usd")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.01);

    let json_output = has_arg("--json");
    let dry_run = has_arg("--dry-run");
    let show_zero = has_arg("--show-zero");

    log(
        LogTag::System,
        "INFO",
        &format!(
            "Analysis parameters: min_value_usd=${:.3}, json={}, dry_run={}, show_zero={}",
            min_value_usd,
            json_output,
            dry_run,
            show_zero
        )
    );

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            eprintln!("❌ Failed to get wallet address: {}", e);
            std::process::exit(1);
        }
    };

    log(LogTag::System, "INFO", &format!("Analyzing wallet: {}", wallet_address));

    // Perform analysis
    match analyze_wallet_dust_tokens(&wallet_address, min_value_usd, show_zero).await {
        Ok(analysis) => {
            if json_output {
                println!("{}", serde_json::to_string_pretty(&analysis).unwrap());
            } else {
                print_analysis_report(&analysis);
            }

            // Cleanup recommendations
            if !dry_run && analysis.dust_tokens.iter().any(|t| t.can_burn) {
                print_cleanup_recommendations(&analysis);
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Analysis failed: {}", e));
            eprintln!("❌ Analysis failed: {}", e);
            std::process::exit(1);
        }
    }

    log(LogTag::System, "INFO", "✅ Wallet dust token analysis completed");
}

async fn analyze_wallet_dust_tokens(
    wallet_address: &str,
    min_value_usd: f64,
    show_zero: bool
) -> Result<WalletDustAnalysis, String> {
    // Initialize RPC client
    let rpc_client = get_rpc_client();

    log(LogTag::System, "INFO", "Fetching all token accounts...");

    // Get all token accounts for the wallet
    let token_accounts = rpc_client
        .get_all_token_accounts(wallet_address).await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    log(LogTag::System, "INFO", &format!("Found {} token accounts", token_accounts.len()));

    let mut dust_tokens = Vec::new();
    let mut total_dust_value_usd = 0.0;
    let mut zero_balance_count = 0;
    let mut burnable_count = 0;
    let mut transfer_disabled_count = 0;

    // Analyze each token account
    for (index, token_account) in token_accounts.iter().enumerate() {
        log(
            LogTag::System,
            "DEBUG",
            &format!(
                "Analyzing token {}/{}: {} (balance: {})",
                index + 1,
                token_accounts.len(),
                safe_truncate(&token_account.mint, 8),
                token_account.balance
            )
        );

        let dust_token = analyze_single_token(token_account, min_value_usd).await?;

        // Skip zero balance accounts unless requested
        if !show_zero && dust_token.is_zero_balance {
            continue;
        }

        // Update counters
        if dust_token.is_zero_balance {
            zero_balance_count += 1;
        }
        if dust_token.can_burn {
            burnable_count += 1;
        }
        if dust_token.transfer_disabled {
            transfer_disabled_count += 1;
        }
        if let Some(value) = dust_token.value_usd {
            total_dust_value_usd += value;
        }

        dust_tokens.push(dust_token);
    }

    // Sort by cleanup priority (highest first)
    dust_tokens.sort_by(|a, b| b.cleanup_priority.cmp(&a.cleanup_priority));

    let analysis = WalletDustAnalysis {
        timestamp: Utc::now(),
        wallet_address: wallet_address.to_string(),
        summary: DustSummary {
            total_token_accounts: token_accounts.len(),
            dust_tokens_count: dust_tokens
                .iter()
                .filter(|t| t.is_dust)
                .count(),
            zero_balance_accounts: zero_balance_count,
            burnable_tokens: burnable_count,
            transfer_disabled_tokens: transfer_disabled_count,
            total_dust_value_usd,
        },
        recommendations: generate_recommendations(&dust_tokens),
        total_cleanup_value_usd: dust_tokens
            .iter()
            .filter(|t| t.cleanup_priority >= 7)
            .filter_map(|t| t.value_usd)
            .sum(),
        dust_tokens,
    };

    Ok(analysis)
}

async fn analyze_single_token(
    token_account: &TokenAccountInfo,
    min_value_usd: f64
) -> Result<DustTokenInfo, String> {
    // Get token decimals
    let decimals = get_cached_decimals(&token_account.mint);

    // Calculate UI balance
    let balance_ui = if let Some(decimals) = decimals {
        (token_account.balance as f64) / (10_f64).powi(decimals as i32)
    } else {
        0.0
    };

    // Get token authorities
    let authorities = match get_token_authorities(&token_account.mint).await {
        Ok(auth) => Some(auth),
        Err(e) => {
            log(
                LogTag::System,
                "WARN",
                &format!(
                    "Failed to get authorities for {}: {}",
                    safe_truncate(&token_account.mint, 8),
                    e
                )
            );
            None
        }
    };

    // Analyze authorities
    let mint_disabled = authorities
        .as_ref()
        .map(|a| a.mint_authority.is_none())
        .unwrap_or(false);

    let freeze_disabled = authorities
        .as_ref()
        .map(|a| a.freeze_authority.is_none())
        .unwrap_or(false);

    let transfer_disabled = freeze_disabled; // If freeze is disabled, transfers can't be disabled

    // Determine if token can be burned
    let can_burn = token_account.balance == 0 || (mint_disabled && transfer_disabled);

    // Estimate USD value (simplified - you might want to integrate with pricing)
    let value_usd = estimate_token_value_usd(token_account, balance_ui, decimals).await;

    // Determine if it's dust
    let is_dust = value_usd.map(|v| v < min_value_usd).unwrap_or(true) && token_account.balance > 0;
    let is_zero_balance = token_account.balance == 0;

    // Determine recommended action and priority
    let (recommended_action, cleanup_priority) = determine_cleanup_action(
        &token_account,
        is_zero_balance,
        can_burn,
        mint_disabled,
        transfer_disabled,
        value_usd,
        min_value_usd
    );

    Ok(DustTokenInfo {
        mint: token_account.mint.clone(),
        ata_address: token_account.account.clone(),
        balance: token_account.balance,
        balance_ui,
        decimals,
        value_usd,
        is_dust,
        is_zero_balance,
        can_burn,
        transfer_disabled,
        mint_disabled,
        freeze_disabled,
        authorities,
        recommended_action,
        cleanup_priority,
    })
}

async fn estimate_token_value_usd(
    _token_account: &TokenAccountInfo,
    _balance_ui: f64,
    _decimals: Option<u8>
) -> Option<f64> {
    // Simplified placeholder - in a real implementation, you'd:
    // 1. Get token price from pool service
    // 2. Calculate USD value based on balance_ui * price
    // 3. Handle decimals properly

    // For now, return None to indicate unknown value
    // This will mark all tokens as potential dust for analysis
    None
}

fn determine_cleanup_action(
    token_account: &TokenAccountInfo,
    is_zero_balance: bool,
    can_burn: bool,
    mint_disabled: bool,
    transfer_disabled: bool,
    value_usd: Option<f64>,
    min_value_usd: f64
) -> (String, u8) {
    if is_zero_balance && can_burn {
        ("Close empty ATA (can burn)".to_string(), 9)
    } else if is_zero_balance {
        ("Close empty ATA".to_string(), 8)
    } else if can_burn && mint_disabled && transfer_disabled {
        ("Burn remaining tokens and close ATA".to_string(), 7)
    } else if let Some(value) = value_usd {
        if value < min_value_usd {
            if transfer_disabled {
                ("Dust token with disabled transfers - manual review".to_string(), 6)
            } else {
                ("Dust token - consider selling or burning".to_string(), 5)
            }
        } else {
            ("Valid token - keep".to_string(), 1)
        }
    } else if token_account.balance < 1000 {
        // Very small balance (less than 1000 raw units)
        if transfer_disabled {
            ("Tiny balance with disabled transfers - likely dust".to_string(), 4)
        } else {
            ("Tiny balance - review manually".to_string(), 3)
        }
    } else {
        ("Unknown value - review manually".to_string(), 2)
    }
}

fn generate_recommendations(dust_tokens: &[DustTokenInfo]) -> Vec<String> {
    let mut recommendations = Vec::new();

    let zero_balance = dust_tokens
        .iter()
        .filter(|t| t.is_zero_balance)
        .count();
    let burnable = dust_tokens
        .iter()
        .filter(|t| t.can_burn)
        .count();
    let high_priority = dust_tokens
        .iter()
        .filter(|t| t.cleanup_priority >= 7)
        .count();

    if zero_balance > 0 {
        recommendations.push(
            format!(
                "Close {} empty token accounts to recover rent ({} SOL)",
                zero_balance,
                (zero_balance as f64) * 0.00203928 // Approximate ATA rent
            )
        );
    }

    if burnable > 0 {
        recommendations.push(
            format!("Consider burning {} tokens with disabled authorities", burnable)
        );
    }

    if high_priority > 0 {
        recommendations.push(
            format!("Focus on {} high-priority cleanup actions first", high_priority)
        );
    }

    recommendations.push("Use Jupiter aggregator to swap dust tokens for SOL".to_string());
    recommendations.push(
        "Check if any dust tokens might have future value before burning".to_string()
    );

    recommendations
}

fn print_analysis_report(analysis: &WalletDustAnalysis) {
    println!("\n🔍 Wallet Dust Token Analysis Report");
    println!("════════════════════════════════════════════════════════════════");
    println!("📅 Analysis Time: {}", analysis.timestamp);
    println!("👛 Wallet Address: {}", analysis.wallet_address);
    println!();

    // Summary
    println!("📊 SUMMARY");
    println!("────────────────────────────────────────────────────────────────");
    println!("Total Token Accounts: {}", analysis.summary.total_token_accounts);
    println!("Dust Tokens: {}", analysis.summary.dust_tokens_count);
    println!("Zero Balance Accounts: {}", analysis.summary.zero_balance_accounts);
    println!("Burnable Tokens: {}", analysis.summary.burnable_tokens);
    println!("Transfer Disabled: {}", analysis.summary.transfer_disabled_tokens);
    println!("Total Dust Value: ${:.4} USD", analysis.summary.total_dust_value_usd);
    println!("Potential Cleanup Value: ${:.4} USD", analysis.total_cleanup_value_usd);
    println!();

    // High priority tokens
    let high_priority: Vec<_> = analysis.dust_tokens
        .iter()
        .filter(|t| t.cleanup_priority >= 7)
        .collect();

    if !high_priority.is_empty() {
        println!("🔥 HIGH PRIORITY CLEANUP (Priority 7-10)");
        println!("────────────────────────────────────────────────────────────────");
        for token in high_priority {
            print_token_info(token);
        }
        println!();
    }

    // Medium priority tokens
    let medium_priority: Vec<_> = analysis.dust_tokens
        .iter()
        .filter(|t| t.cleanup_priority >= 4 && t.cleanup_priority < 7)
        .collect();

    if !medium_priority.is_empty() {
        println!("⚠️  MEDIUM PRIORITY CLEANUP (Priority 4-6)");
        println!("────────────────────────────────────────────────────────────────");
        for token in medium_priority {
            print_token_info(token);
        }
        println!();
    }

    // Low priority tokens (only show first 10)
    let low_priority: Vec<_> = analysis.dust_tokens
        .iter()
        .filter(|t| t.cleanup_priority < 4)
        .take(10)
        .collect();

    if !low_priority.is_empty() {
        println!("ℹ️  LOW PRIORITY / REVIEW NEEDED (Priority 1-3) - First 10");
        println!("────────────────────────────────────────────────────────────────");
        for token in low_priority {
            print_token_info(token);
        }
        if
            analysis.dust_tokens
                .iter()
                .filter(|t| t.cleanup_priority < 4)
                .count() > 10
        {
            println!(
                "... and {} more (use --json for full list)",
                analysis.dust_tokens
                    .iter()
                    .filter(|t| t.cleanup_priority < 4)
                    .count() - 10
            );
        }
        println!();
    }
}

fn print_token_info(token: &DustTokenInfo) {
    let status_icons = format!(
        "{}{}{}{}",
        if token.is_zero_balance {
            "💀"
        } else {
            "💰"
        },
        if token.can_burn {
            "🔥"
        } else {
            "🚫"
        },
        if token.mint_disabled {
            "🚨"
        } else {
            "✅"
        },
        if token.transfer_disabled {
            "🔒"
        } else {
            "🔓"
        }
    );

    let value_str = if let Some(value) = token.value_usd {
        format!("${:.6}", value)
    } else {
        "Unknown".to_string()
    };

    println!(
        "{} {} | Balance: {:.6} | Value: {} | Priority: {} | {}",
        status_icons,
        safe_truncate(&token.mint, 8),
        token.balance_ui,
        value_str,
        token.cleanup_priority,
        token.recommended_action
    );

    if token.cleanup_priority >= 7 {
        println!("   ATA: {}", token.ata_address);
        if let Some(auth) = &token.authorities {
            println!("   Mint Authority: {:?}", auth.mint_authority);
            println!("   Freeze Authority: {:?}", auth.freeze_authority);
        }
    }
}

fn print_cleanup_recommendations(analysis: &WalletDustAnalysis) {
    println!("\n💡 CLEANUP RECOMMENDATIONS");
    println!("════════════════════════════════════════════════════════════════");
    for (i, recommendation) in analysis.recommendations.iter().enumerate() {
        println!("{}. {}", i + 1, recommendation);
    }
    println!();

    println!("🚨 IMPORTANT NOTES:");
    println!("• Always verify token value before burning - some may gain value later");
    println!("• Test with small amounts first");
    println!("• Empty ATAs can be closed to recover ~0.002 SOL rent each");
    println!("• Use --dry-run flag for analysis without any actual operations");
    println!();

    println!("Legend:");
    println!("💀 = Zero balance  💰 = Has balance  🔥 = Can burn  🚫 = Cannot burn");
    println!(
        "🚨 = Mint disabled  ✅ = Mint enabled  🔒 = Transfer disabled  🔓 = Transfer enabled"
    );
}

fn print_help() {
    println!("Wallet Dust Token Analysis Tool");
    println!();
    println!("Analyzes all token balances and ATAs in wallet to identify dust tokens,");
    println!("empty accounts, and cleanup opportunities.");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin debug_wallet_dust_tokens [FLAGS]");
    println!();
    println!("FLAGS:");
    println!("    --min-value-usd <amount>  Minimum USD value to consider (default: 0.01)");
    println!("    --json                    Output in JSON format for scripting");
    println!("    --dry-run                 Only analyze, don't suggest cleanup actions");
    println!("    --show-zero               Include zero balance accounts in output");
    println!("    --help                    Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    cargo run --bin debug_wallet_dust_tokens");
    println!("    cargo run --bin debug_wallet_dust_tokens --min-value-usd 0.001");
    println!("    cargo run --bin debug_wallet_dust_tokens --json > wallet_analysis.json");
    println!("    cargo run --bin debug_wallet_dust_tokens --show-zero --dry-run");
    println!();
    println!("OUTPUT LEGEND:");
    println!("    Priority 9-10: Immediate action recommended");
    println!("    Priority 7-8:  High priority cleanup");
    println!("    Priority 4-6:  Medium priority, review needed");
    println!("    Priority 1-3:  Low priority or keep");
}
