use crate::errors::blockchain::{ parse_structured_solana_error, BlockchainError };
use crate::errors::parse_solana_error;
use crate::errors::ScreenerBotError;
use crate::logger::{ log, LogTag };
use chrono::{ DateTime, Utc };
use solana_sdk::pubkey::Pubkey;
use std::fs;
use std::str::FromStr;
use std::sync::RwLock;
use std::time::{ Duration, Instant };
use tokio::sync::Notify;

/// Safe signature formatting that shows first 8 and last 4 chars, or full string if short
pub fn safe_format_signature(s: &str) -> String {
    let char_count = s.chars().count();
    if char_count > 12 {
        let first_8 = s;
        // Get last 4 characters safely
        let last_4 = if char_count >= 4 {
            s.chars()
                .skip(char_count - 4)
                .collect::<String>()
        } else {
            s.to_string()
        };
        format!("{}...{}", first_8, last_4)
    } else {
        s.to_string()
    }
}

// =============================================================================
// SOLANA-SPECIFIC UTILITIES (Consolidated from multiple files)
// =============================================================================

/// Standard pubkey parsing with consistent error message formatting
/// Consolidates 20+ identical patterns across the codebase
pub fn parse_pubkey_safe(address: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(address).map_err(|e| format!("Invalid pubkey '{}': {}", address, e))
}

/// Read a pubkey from byte data at specified offset with bounds checking
/// Consolidates 17+ duplicate read_pubkey implementations from debug binaries
pub fn read_pubkey_from_data(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }

    let pubkey_bytes = &data[offset..offset + 32];
    match Pubkey::try_from(pubkey_bytes) {
        Ok(pubkey) => {
            // Basic sanity check - reject all-zeros or all-ones
            if pubkey_bytes.iter().all(|&b| b == 0) || pubkey_bytes.iter().all(|&b| b == 255) {
                None
            } else {
                Some(pubkey.to_string())
            }
        }
        Err(_) => None,
    }
}

/// Read a u64 from byte data at specified offset with little-endian byte order
/// Consolidates 9+ duplicate read_u64 implementations from debug binaries
pub fn read_u64_from_data(data: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 > data.len() {
        return None;
    }

    let bytes: [u8; 8] = data[offset..offset + 8].try_into().ok()?;
    Some(u64::from_le_bytes(bytes))
}

/// Read a u32 from byte data at specified offset with little-endian byte order
pub fn read_u32_from_data(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > data.len() {
        return None;
    }

    let bytes: [u8; 4] = data[offset..offset + 4].try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// SOL lamports conversion functions
/// Uses the existing LAMPORTS_PER_SOL constant from decimals.rs
/// Consolidates 20+ hardcoded 1_000_000_000 values across the codebase
use crate::tokens::decimals::LAMPORTS_PER_SOL;

/// Convert lamports to SOL with consistent precision
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / (LAMPORTS_PER_SOL as f64)
}

/// Convert SOL to lamports with proper rounding
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * (LAMPORTS_PER_SOL as f64)).round() as u64
}

/// Format mint address consistently for logs (8 chars + "...")
/// Consolidates multiple patterns for mint addresses
pub fn format_mint_for_log(mint: &str) -> String {
    format!("{}...", mint)
}

// Re-export for backward compatibility
pub use crate::swaps::SwapResult;
// Remove dependency on swaps module for get_wallet_address
// pub use crate::swaps::get_wallet_address;

use bs58;
use solana_sdk::{
    instruction::{ AccountMeta, Instruction },
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
};
use spl_token::instruction::close_account;

/// Get the wallet address from the main wallet private key in config
/// This replaces the swaps::get_wallet_address dependency
pub fn get_wallet_address() -> Result<String, ScreenerBotError> {
    crate::config::get_wallet_pubkey_string().map_err(|e| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidPrivateKey {
            error: format!("Failed to load wallet address: {}", e),
        })
    })
}

/// Format a duration (from Option<DateTime<Utc>>) as a human-readable age string (y d h m s)
pub fn format_age_string(created_at: Option<DateTime<Utc>>) -> String {
    if let Some(dt) = created_at {
        let now = Utc::now();
        let mut seconds = if now > dt { (now - dt).num_seconds() } else { 0 };
        let years = seconds / 31_536_000; // 365*24*60*60
        seconds %= 31_536_000;
        let days = seconds / 86_400;
        seconds %= 86_400;
        let hours = seconds / 3_600;
        seconds %= 3_600;
        let minutes = seconds / 60;
        seconds %= 60;
        let mut parts = Vec::new();
        if years > 0 {
            parts.push(format!("{}y", years));
        }
        if days > 0 {
            parts.push(format!("{}d", days));
        }
        if hours > 0 {
            parts.push(format!("{}h", hours));
        }
        if minutes > 0 {
            parts.push(format!("{}m", minutes));
        }
        if seconds > 0 || parts.is_empty() {
            parts.push(format!("{}s", seconds));
        }
        parts.join(" ")
    } else {
        "unknown".to_string()
    }
}

/// Waits for either shutdown signal or delay. Returns true if shutdown was triggered.
pub async fn check_shutdown_or_delay(shutdown: &Notify, duration: Duration) -> bool {
    tokio::select! {
        _ = tokio::time::sleep(duration) => false,
        _ = shutdown.notified() => true,
    }
}

/// Waits for a delay or shutdown signal, whichever comes first.
pub async fn delay_with_shutdown(shutdown: &Notify, duration: Duration) {
    tokio::select! {
        _ = tokio::time::sleep(duration) => {},
        _ = shutdown.notified() => {},
    }
}

/// Helper function to format duration in a compact way
pub fn format_duration_compact(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    let duration = end.signed_duration_since(start);
    let total_seconds = duration.num_seconds();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        format!("{}m", total_seconds / 60)
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours > 0 {
            format!("{}d{}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}

/// Utility function for hex dump debugging - prints data in hex format with ASCII representation
pub fn hex_dump_data(
    data: &[u8],
    start_offset: usize,
    length: usize,
    log_callback: impl Fn(&str, &str)
) {
    let end = std::cmp::min(start_offset + length, data.len());

    for chunk_start in (start_offset..end).step_by(16) {
        let chunk_end = std::cmp::min(chunk_start + 16, end);
        let chunk = &data[chunk_start..chunk_end];

        // Format offset
        let offset_str = format!("{:08X}", chunk_start);

        // Format hex bytes
        let hex_str = chunk
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        // Pad hex string to consistent width (48 chars for 16 bytes)
        let hex_padded = format!("{:<48}", hex_str);

        // Format ASCII representation
        let ascii_str: String = chunk
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' }
            })
            .collect();

        log_callback("DEBUG", &format!("{}: {} |{}|", offset_str, hex_padded, ascii_str));
    }
}

/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>), ScreenerBotError> {
    log(
        LogTag::Wallet,
        "ATA",
        "⚠️ Manual ATA cleanup triggered (normally handled by background service)"
    );
    let wallet_address = get_wallet_address()?;
    close_all_empty_atas(&wallet_address).await
}

/// Checks wallet balance for SOL
pub async fn get_sol_balance(wallet_address: &str) -> Result<f64, ScreenerBotError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_sol_balance(wallet_address).await
}

/// Checks wallet balance for a specific token (SINGLE ACCOUNT ONLY - use get_total_token_balance for exits)
pub async fn get_token_balance(wallet_address: &str, mint: &str) -> Result<u64, ScreenerBotError> {
    use crate::arguments::is_debug_ata_enabled;
    use crate::logger::{ log, LogTag };

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("🔍 TOKEN_BALANCE_START: wallet={}, mint={}", wallet_address, mint)
        );
    }

    log(
        LogTag::Wallet,
        "DEBUG",
        &format!("🔍 Fetching token balance: wallet={}, mint={}", wallet_address, mint)
    );

    let rpc_client = crate::rpc::get_rpc_client();

    if is_debug_ata_enabled() {
        log(LogTag::Wallet, "DEBUG", &format!("🌐 TOKEN_BALANCE_RPC: querying RPC for balance"));
    }

    match rpc_client.get_token_balance(wallet_address, mint).await {
        Ok(balance) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "📊 TOKEN_BALANCE_RESULT: {} units for mint {}",
                        balance,
                        format_mint_for_log(&mint)
                    )
                );
            }
            log(
                LogTag::Wallet,
                "DEBUG",
                &format!(
                    "✅ Token balance fetched successfully: {} units for mint {}",
                    balance,
                    mint
                )
            );
            Ok(balance)
        }
        Err(e) => {
            let blockchain_error = parse_solana_error(&e.to_string(), None, "get_token_balance");
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "❌ TOKEN_BALANCE_ERROR: {} for mint {}",
                        blockchain_error,
                        format_mint_for_log(&mint)
                    )
                );
            }
            log(
                LogTag::Wallet,
                "ERROR",
                &format!(
                    "❌ Failed to fetch token balance for mint {}: {}",
                    format_mint_for_log(&mint),
                    e
                )
            );
            Err(e)
        }
    }
}

/// Get TOTAL token balance across ALL token accounts for a mint (USE FOR EXITS TO SELL ALL)
pub async fn get_total_token_balance(
    wallet_address: &str,
    mint: &str
) -> Result<u64, ScreenerBotError> {
    use crate::arguments::is_debug_ata_enabled;
    use crate::logger::{ log, LogTag };

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("🔍 TOTAL_TOKEN_BALANCE_START: wallet={}, mint={}", wallet_address, mint)
        );
    }

    // Get all token accounts for this wallet
    let all_accounts = get_all_token_accounts(wallet_address).await?;

    // Filter accounts for the specific mint and sum balances
    let mut total_balance = 0u64;
    let mut account_count = 0usize;

    for account in all_accounts {
        if account.mint == mint {
            total_balance = total_balance.saturating_add(account.balance);
            account_count += 1;

            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "📊 Found account {} with {} tokens ({})",
                        &account.account,
                        account.balance,
                        if account.is_token_2022 {
                            "Token-2022"
                        } else {
                            "SPL Token"
                        }
                    )
                );
            }
        }
    }

    log(
        LogTag::Wallet,
        "TOTAL_BALANCE",
        &format!(
            "✅ Total balance for mint {}: {} tokens across {} accounts",
            mint,
            total_balance,
            account_count
        )
    );

    if account_count > 1 {
        log(
            LogTag::Wallet,
            "MULTI_ACCOUNT",
            &format!(
                "⚠️ MULTIPLE ACCOUNTS DETECTED for mint {}: {} accounts with total {} tokens",
                mint,
                account_count,
                total_balance
            )
        );
    }

    Ok(total_balance)
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(
    wallet_address: &str
) -> Result<Vec<crate::rpc::TokenAccountInfo>, ScreenerBotError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_all_token_accounts(wallet_address).await
}

/// Closes a single empty ATA (Associated Token Account) for a specific mint
/// Returns the transaction signature if successful
pub async fn close_single_ata(
    wallet_address: &str,
    mint: &str
) -> Result<String, ScreenerBotError> {
    log(
        LogTag::Wallet,
        "ATA",
        &format!("Attempting to close single ATA for mint {}", format_mint_for_log(&mint))
    );

    // Get all token accounts to find the specific one
    let token_accounts = get_all_token_accounts(wallet_address).await?;

    // Find the account for this mint
    let target_account = token_accounts
        .iter()
        .find(|account| account.mint == mint && account.balance == 0);

    match target_account {
        Some(account) => {
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Found empty ATA {} for mint {}", account.account, &mint[..8])
            );

            // Close the ATA
            match close_ata(wallet_address, &account.account, mint, account.is_token_2022).await {
                Ok(signature) => {
                    log(
                        LogTag::Wallet,
                        "SUCCESS",
                        &format!(
                            "Closed ATA {} for mint {}. TX: {}",
                            account.account,
                            &mint[..8],
                            signature
                        )
                    );
                    Ok(signature)
                }
                Err(e) => {
                    log(
                        LogTag::Wallet,
                        "ERROR",
                        &format!(
                            "Failed to close ATA {} for mint {}: {}",
                            account.account,
                            &mint[..8],
                            e
                        )
                    );
                    Err(e)
                }
            }
        }
        None => {
            let error_msg = format!("No empty ATA found for mint {}", &mint[..8]);
            log(LogTag::Wallet, "WARNING", &error_msg);
            Err(
                ScreenerBotError::invalid_amount(
                    error_msg.clone(),
                    "No empty ATA found".to_string()
                )
            )
        }
    }
}

/// Closes all empty ATAs (Associated Token Accounts) for a wallet
/// This reclaims the rent SOL (~0.002 SOL per account) from all empty token accounts
/// Returns the number of accounts closed and total signatures
pub async fn close_all_empty_atas(
    wallet_address: &str
) -> Result<(u32, Vec<String>), ScreenerBotError> {
    log(LogTag::Wallet, "ATA", "🔍 Checking for empty token accounts to close...");

    // Get all token accounts for the wallet
    let all_accounts = get_all_token_accounts(wallet_address).await?;

    if all_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No token accounts found in wallet");
        return Ok((0, vec![]));
    }

    // Filter for empty accounts (balance = 0)
    let empty_accounts: Vec<&crate::rpc::TokenAccountInfo> = all_accounts
        .iter()
        .filter(|account| account.balance == 0)
        .collect();

    if empty_accounts.is_empty() {
        log(LogTag::Wallet, "ATA", "No empty token accounts found to close");
        return Ok((0, vec![]));
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Found {} empty token accounts to close", empty_accounts.len())
    );

    let mut signatures = Vec::new();
    let mut closed_count = 0u32;

    // Close each empty account
    for account_info in empty_accounts {
        log(
            LogTag::Wallet,
            "ATA",
            &format!(
                "Closing empty {} account {} for mint {}",
                if account_info.is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                account_info.account,
                account_info.mint
            )
        );

        match
            close_ata(
                wallet_address,
                &account_info.account,
                &account_info.mint,
                account_info.is_token_2022
            ).await
        {
            Ok(signature) => {
                log(
                    LogTag::Wallet,
                    "SUCCESS",
                    &format!("✅ Closed empty ATA {}. TX: {}", account_info.account, signature)
                );
                signatures.push(signature);
                closed_count += 1;

                // Small delay between closures to avoid overwhelming the network
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
            Err(e) => {
                log(
                    LogTag::Wallet,
                    "ERROR",
                    &format!("❌ Failed to close ATA {}: {}", account_info.account, e)
                );
                // Continue with other accounts even if one fails
            }
        }
    }

    let rent_reclaimed = (closed_count as f64) * 0.00203928; // Approximate ATA rent in SOL
    log(
        LogTag::Wallet,
        "ATA",
        &format!(
            "🎉 ATA cleanup complete! Closed {} accounts, reclaimed ~{:.6} SOL in rent",
            closed_count,
            rent_reclaimed
        )
    );

    Ok((closed_count, signatures))
}

/// Closes the Associated Token Account (ATA) for a given token mint after selling all tokens
/// This reclaims the rent SOL (~0.002 SOL) from empty token accounts
/// Supports both regular SPL tokens and Token-2022 tokens
///
/// # Parameters
/// * `mint` - The token mint address
/// * `wallet_address` - The wallet address
/// * `recently_sold` - Optional flag indicating if tokens were recently sold (enables longer wait times)
pub async fn close_token_account(
    mint: &str,
    wallet_address: &str
) -> Result<String, ScreenerBotError> {
    close_token_account_with_context(mint, wallet_address, false).await
}

/// Enhanced version of close_token_account with additional context
pub async fn close_token_account_with_context(
    mint: &str,
    wallet_address: &str,
    recently_sold: bool
) -> Result<String, ScreenerBotError> {
    use crate::arguments::is_debug_ata_enabled;

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "ATA",
            &format!("Attempting to close token account for mint: {}", mint)
        );
    } else {
        log(
            LogTag::Wallet,
            "ATA",
            &format!("Attempting to close token account for mint: {}", format_mint_for_log(mint))
        );
    }

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🔧 ATA_CLOSE_START: wallet={}, mint={}, recently_sold={}",
                &wallet_address[..8],
                &mint[..8],
                recently_sold
            )
        );
    }

    // First verify the token balance is actually zero with retry logic for blockchain propagation
    let mut balance_check_attempts = 0;
    let max_checks = if recently_sold { 8 } else { 5 }; // More attempts if recently sold
    let delay_ms = if recently_sold { 3000 } else { 2000 }; // Longer delay if recently sold

    if is_debug_ata_enabled() && recently_sold {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "⚠️ ATA_RECENTLY_SOLD: using extended retry logic ({}x{}ms) for recently sold token",
                max_checks,
                delay_ms
            )
        );
    }

    loop {
        balance_check_attempts += 1;

        if is_debug_ata_enabled() {
            log(
                LogTag::Wallet,
                "DEBUG",
                &format!(
                    "🔍 ATA_BALANCE_CHECK: attempt {}/{} for mint {}",
                    balance_check_attempts,
                    max_checks,
                    &mint[..8]
                )
            );
        }

        match get_token_balance(wallet_address, mint).await {
            Ok(balance) => {
                if is_debug_ata_enabled() {
                    log(
                        LogTag::Wallet,
                        "DEBUG",
                        &format!(
                            "📊 ATA_BALANCE_RESULT: {} tokens remaining for mint {}",
                            balance,
                            &mint[..8]
                        )
                    );
                }

                if balance > 0 {
                    if balance_check_attempts < max_checks {
                        if is_debug_ata_enabled() {
                            log(
                                LogTag::Wallet,
                                "DEBUG",
                                &format!(
                                    "⏳ ATA_BALANCE_RETRY: {} tokens still present, waiting {}ms before retry (attempt {}/{})",
                                    balance,
                                    delay_ms,
                                    balance_check_attempts,
                                    max_checks
                                )
                            );
                        }
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        continue;
                    } else {
                        if is_debug_ata_enabled() {
                            log(
                                LogTag::Wallet,
                                "DEBUG",
                                &format!(
                                    "❌ ATA_BALANCE_FAILED: {} tokens still present after {} attempts",
                                    balance,
                                    max_checks
                                )
                            );
                        }
                        return Err(
                            ScreenerBotError::invalid_amount(
                                balance.to_string(),
                                format!(
                                    "Cannot close token account - still has {} tokens after {} balance checks",
                                    balance,
                                    max_checks
                                )
                            )
                        );
                    }
                }

                if is_debug_ata_enabled() {
                    log(
                        LogTag::Wallet,
                        "DEBUG",
                        &format!(
                            "✅ ATA_BALANCE_ZERO: confirmed zero balance for mint {} after {} attempts",
                            &mint[..8],
                            balance_check_attempts
                        )
                    );
                }

                log(
                    LogTag::Wallet,
                    "ATA",
                    &format!("Verified zero balance for {}, proceeding to close ATA", mint)
                );
                break;
            }
            Err(e) => {
                if is_debug_ata_enabled() {
                    log(
                        LogTag::Wallet,
                        "DEBUG",
                        &format!(
                            "⚠️ ATA_BALANCE_ERROR: attempt {}/{} failed: {}",
                            balance_check_attempts,
                            max_checks,
                            e
                        )
                    );
                }

                if balance_check_attempts < max_checks {
                    if is_debug_ata_enabled() {
                        log(
                            LogTag::Wallet,
                            "DEBUG",
                            &format!("🔄 ATA_BALANCE_RETRY: waiting {}ms before retry due to error", delay_ms)
                        );
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                } else {
                    log(
                        LogTag::Wallet,
                        "WARN",
                        &format!(
                            "Could not verify token balance before closing ATA after {} attempts: {}",
                            max_checks,
                            e
                        )
                    );
                    // Continue anyway - the close instruction will fail if tokens remain
                    break;
                }
            }
        }
    }

    // Get the associated token account address
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("🔍 ATA_DISCOVER: finding associated token account for mint {}", &mint[..8])
        );
    }

    let token_account = match get_associated_token_account(wallet_address, mint).await {
        Ok(account) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "✅ ATA_FOUND: token_account={} for mint={}",
                        &account[..8],
                        &mint[..8]
                    )
                );
            }
            account
        }
        Err(e) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "❌ ATA_NOT_FOUND: error finding token account for mint {}: {}",
                        &mint[..8],
                        e
                    )
                );
            }
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not find associated token account for {}: {}", mint, e)
            );
            return Err(e);
        }
    };

    log(LogTag::Wallet, "ATA", &format!("Found token account to close: {}", token_account));

    // Determine if this is a Token-2022 account by checking the token ACCOUNT's program (not the mint)
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🔍 ATA_PROGRAM_CHECK: determining token program for account {}",
                &token_account[..8]
            )
        );
    }

    let rpc_client = crate::rpc::get_rpc_client();
    let is_token_2022 = rpc_client
        .is_token_account_token_2022(&token_account).await
        .unwrap_or(false);

    if is_token_2022 {
        if is_debug_ata_enabled() {
            log(
                LogTag::Wallet,
                "DEBUG",
                &format!(
                    "🆕 ATA_TOKEN2022: using Token Extensions program for account {}",
                    &token_account[..8]
                )
            );
        }
        log(LogTag::Wallet, "ATA", "Detected Token-2022, using Token Extensions program");
    } else {
        if is_debug_ata_enabled() {
            log(
                LogTag::Wallet,
                "DEBUG",
                &format!(
                    "🔧 ATA_SPL_TOKEN: using standard SPL Token program for account {}",
                    &token_account[..8]
                )
            );
        }
        log(LogTag::Wallet, "ATA", "Using standard SPL Token program");
    }

    // Create and send the close account instruction using GMGN API approach
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🚀 ATA_CLOSE_EXECUTE: initiating close instruction for account {}",
                &token_account[..8]
            )
        );
    }
    match close_ata(wallet_address, &token_account, mint, is_token_2022).await {
        Ok(signature) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "🎉 ATA_CLOSE_SUCCESS: transaction={}, account={}, mint={}",
                        &signature[..8],
                        &token_account[..8],
                        &mint[..8]
                    )
                );
            }
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!("Successfully closed token account for {}. TX: {}", mint, signature)
            );
            Ok(signature)
        }
        Err(e) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "❌ ATA_CLOSE_FAILED: account={}, mint={}, error={}",
                        &token_account[..8],
                        &mint[..8],
                        e
                    )
                );
            }
            log(
                LogTag::Wallet,
                "ERROR",
                &format!("Failed to close token account for {}: {}", mint, e)
            );
            Err(e)
        }
    }
}

/// Gets the associated token account address for a wallet and mint
async fn get_associated_token_account(
    wallet_address: &str,
    mint: &str
) -> Result<String, ScreenerBotError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_associated_token_account(wallet_address, mint).await
}

/// Closes ATA using proper Solana SDK for real ATA closing
async fn close_ata(
    wallet_address: &str,
    token_account: &str,
    mint: &str,
    is_token_2022: bool
) -> Result<String, ScreenerBotError> {
    use crate::arguments::is_debug_ata_enabled;

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🔧 ATA_SDK_START: wallet={}, account={}, mint={}, program={}",
                &wallet_address[..8],
                &token_account[..8],
                &mint[..8],
                if is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                }
            )
        );
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Closing ATA {} for mint {} using {} program", token_account, mint, if
            is_token_2022
        {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Use proper Solana SDK to build and send close instruction
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🛠️ ATA_BUILD_INSTRUCTION: preparing close instruction for account {}",
                &token_account[..8]
            )
        );
    }

    match build_and_send_close_instruction(wallet_address, token_account, is_token_2022).await {
        Ok(signature) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "✅ ATA_SDK_SUCCESS: instruction executed, transaction={}",
                        &signature[..8]
                    )
                );
            }
            log(LogTag::Wallet, "SUCCESS", &format!("ATA closed successfully. TX: {}", signature));
            Ok(signature)
        }
        Err(e) => {
            if is_debug_ata_enabled() {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "❌ ATA_SDK_FAILED: instruction failed for account {}: {}",
                        &token_account[..8],
                        e
                    )
                );
            }
            Err(e)
        }
    }
}

/// Builds and sends close account instruction using Solana SDK
async fn build_and_send_close_instruction(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool
) -> Result<String, ScreenerBotError> {
    use crate::arguments::is_debug_ata_enabled;

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🔨 ATA_INSTRUCTION_START: building close instruction for account {}",
                &token_account[..8]
            )
        );
    }

    // Parse addresses
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!(
                "🔑 ATA_PARSE_ADDRESSES: wallet={}, account={}",
                &wallet_address[..8],
                &token_account[..8]
            )
        );
    }

    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e| {
        ScreenerBotError::invalid_amount(
            format!("Invalid wallet address: {}", e),
            "Wallet validation failed".to_string()
        )
    })?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e| {
        ScreenerBotError::invalid_amount(
            format!("Invalid token account: {}", e),
            "Token account validation failed".to_string()
        )
    })?;

    // Load keypair from config
    if is_debug_ata_enabled() {
        log(LogTag::Wallet, "DEBUG", &format!("🔐 ATA_KEYPAIR: creating keypair from config"));
    }

    let keypair = crate::config::get_wallet_keypair().map_err(|e| {
        ScreenerBotError::Configuration(crate::errors::ConfigurationError::InvalidPrivateKey {
            error: format!("Failed to load wallet keypair: {}", e),
        })
    })?;

    // Build close account instruction
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("⚙️ ATA_INSTRUCTION_BUILD: creating {} close instruction", if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            })
        );
    }

    let close_instruction = if is_token_2022 {
        // For Token-2022, use the Token Extensions program
        build_token_2022_close_instruction(&token_account_pubkey, &owner_pubkey)?
    } else {
        // For regular SPL tokens, use standard close_account instruction
        close_account(
            &spl_token::id(),
            &token_account_pubkey,
            &owner_pubkey,
            &owner_pubkey,
            &[]
        ).map_err(|e| {
            ScreenerBotError::Blockchain(crate::errors::BlockchainError::InvalidInstruction {
                signature: "unknown".to_string(),
                instruction_index: 0,
                reason: format!("Failed to build close instruction: {}", e),
            })
        })?
    };

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("✅ ATA_INSTRUCTION_BUILT: close instruction created for {} program", if
                is_token_2022
            {
                "Token-2022"
            } else {
                "SPL Token"
            })
        );
    }

    log(
        LogTag::Wallet,
        "ATA",
        &format!("Built close instruction for {} account", if is_token_2022 {
            "Token-2022"
        } else {
            "SPL Token"
        })
    );

    // Get recent blockhash via RPC
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("🌐 ATA_BLOCKHASH: fetching recent blockhash via RPC")
        );
    }

    let rpc_client = crate::rpc::get_rpc_client();
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("📦 ATA_BLOCKHASH_OK: blockhash={}", &recent_blockhash.to_string()[..8])
        );
    }

    // Build transaction
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("📝 ATA_TRANSACTION_BUILD: creating signed transaction")
        );
    }

    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash
    );

    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("✅ ATA_TRANSACTION_READY: transaction built and signed")
        );
    }

    log(LogTag::Wallet, "ATA", "Built and signed close transaction");

    // Send transaction via RPC with confirmation
    if is_debug_ata_enabled() {
        log(
            LogTag::Wallet,
            "DEBUG",
            &format!("🚀 ATA_TRANSACTION_SEND: submitting transaction to network with confirmation")
        );
    }

    let result = rpc_client.send_and_confirm_signed_transaction(&transaction).await;

    if is_debug_ata_enabled() {
        match &result {
            Ok(signature) => {
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!(
                        "🎉 ATA_TRANSACTION_CONFIRMED: transaction confirmed, signature={}",
                        &signature[..8]
                    )
                );
            }
            Err(e) => {
                let blockchain_error = parse_solana_error(
                    &e.to_string(),
                    None,
                    "create_ata_transaction"
                );
                log(
                    LogTag::Wallet,
                    "DEBUG",
                    &format!("❌ ATA_TRANSACTION_FAILED: {}", blockchain_error)
                );
            }
        }
    }

    result
}

/// Builds close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey
) -> Result<Instruction, ScreenerBotError> {
    // Token-2022 uses the same close account instruction format as SPL Token
    // but with different program ID
    let token_2022_program_id = Pubkey::from_str(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ).map_err(|e| {
        ScreenerBotError::Blockchain(crate::errors::BlockchainError::InvalidAccountData {
            signature: "unknown".to_string(),
            account: "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb".to_string(),
            expected_owner: "Program ID".to_string(),
            actual_owner: None,
        })
    })?;

    // Manually build the close account instruction for Token-2022
    // CloseAccount instruction: [9] (instruction discriminator)
    let instruction_data = vec![9u8]; // CloseAccount instruction ID

    let accounts = vec![
        AccountMeta::new(*token_account, false), // Token account to close
        AccountMeta::new(*owner, false), // Destination for lamports
        AccountMeta::new_readonly(*owner, true) // Authority (signer)
    ];

    Ok(Instruction {
        program_id: token_2022_program_id,
        accounts,
        data: instruction_data,
    })
}

// =============================================================================
// UTILITY FUNCTIONS MOVED FROM TRADER.RS
// =============================================================================

/// Safe wrapper for RwLock read operations that logs poison errors instead of panicking
pub fn safe_read_lock<'a, T>(
    lock: &'a std::sync::RwLock<T>,
    operation: &str
) -> Option<std::sync::RwLockReadGuard<'a, T>> {
    match lock.read() {
        Ok(guard) => Some(guard),
        Err(e) => {
            log(
                LogTag::Trader,
                "LOCK_POISON_ERROR",
                &format!("🔒 RwLock read poisoned during {}: {}", operation, e)
            );
            None
        }
    }
}

/// Safe wrapper for RwLock write operations that logs poison errors instead of panicking
pub fn safe_write_lock<'a, T>(
    lock: &'a std::sync::RwLock<T>,
    operation: &str
) -> Option<std::sync::RwLockWriteGuard<'a, T>> {
    match lock.write() {
        Ok(guard) => Some(guard),
        Err(e) => {
            log(
                LogTag::Trader,
                "LOCK_POISON_ERROR",
                &format!("🔒 RwLock write poisoned during {}: {}", operation, e)
            );
            None
        }
    }
}

/// Helper function for conditional debug trader logs
pub fn debug_trader_log(log_type: &str, message: &str) {
    use crate::global::is_debug_trader_enabled;
    if is_debug_trader_enabled() {
        log(LogTag::Trader, log_type, message);
    }
}
