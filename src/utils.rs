use chrono::{ DateTime, Utc };
use tokio::sync::Notify;
use std::time::Duration;
use std::fs;
use serde_json;
use crate::positions::*;
use crate::logger::{ log, LogTag };
use crate::global::POSITIONS_FILE;
use rand::{ Rng, seq::SliceRandom };

// Wallet-related imports
use crate::global::read_configs;
use crate::rpc::SwapError;
// Re-export for backward compatibility
pub use crate::swaps::interface::SwapResult;
pub use crate::swaps::get_wallet_address;

use solana_sdk::{
    signature::Keypair,
    signer::Signer,
    pubkey::Pubkey,
    instruction::{ Instruction, AccountMeta },
    transaction::Transaction,
};
use spl_token::instruction::close_account;
use bs58;
use std::str::FromStr;

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

pub fn save_positions_to_file(positions: &Vec<Position>) {
    if let Ok(json) = serde_json::to_string_pretty(positions) {
        if let Err(e) = fs::write(POSITIONS_FILE, json) {
            log(LogTag::Trader, "ERROR", &format!("Failed to write {}: {}", POSITIONS_FILE, e));
        }
    }
}

pub fn load_positions_from_file() -> Vec<Position> {
    match fs::read_to_string(POSITIONS_FILE) {
        Ok(content) =>
            match serde_json::from_str::<Vec<Position>>(&content) {
                Ok(positions) => positions,
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Failed to parse {}: {}", POSITIONS_FILE, e)
                    );
                    Vec::new()
                }
            }
        Err(_) => Vec::new(), // Return empty vector if file doesn't exist
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
            .map(|&b| if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' })
            .collect();

        log_callback("DEBUG", &format!("{}: {} |{}|", offset_str, hex_padded, ascii_str));
    }
}

/// Public function to manually close all empty ATAs for the configured wallet
/// Note: ATA cleanup is now handled automatically by background service (see ata_cleanup.rs)
/// This function is kept for manual cleanup or emergency situations
pub async fn cleanup_all_empty_atas() -> Result<(u32, Vec<String>), SwapError> {
    log(
        LogTag::Wallet,
        "ATA",
        "⚠️ Manual ATA cleanup triggered (normally handled by background service)"
    );
    let wallet_address = get_wallet_address()?;
    close_all_empty_atas(&wallet_address).await
}

/// Checks wallet balance for SOL
pub async fn get_sol_balance(wallet_address: &str) -> Result<f64, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_sol_balance(wallet_address).await
}

/// Checks wallet balance for a specific token
pub async fn get_token_balance(wallet_address: &str, mint: &str) -> Result<u64, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_token_balance(wallet_address, mint).await
}

/// Gets all token accounts for a wallet
pub async fn get_all_token_accounts(
    wallet_address: &str
) -> Result<Vec<crate::rpc::TokenAccountInfo>, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_all_token_accounts(wallet_address).await
}

/// Closes a single empty ATA (Associated Token Account) for a specific mint
/// Returns the transaction signature if successful
pub async fn close_single_ata(wallet_address: &str, mint: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close single ATA for mint {}", &mint[..8]));

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
            Err(SwapError::InvalidAmount(error_msg))
        }
    }
}

/// Closes all empty ATAs (Associated Token Accounts) for a wallet
/// This reclaims the rent SOL (~0.002 SOL per account) from all empty token accounts
/// Returns the number of accounts closed and total signatures
pub async fn close_all_empty_atas(wallet_address: &str) -> Result<(u32, Vec<String>), SwapError> {
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
pub async fn close_token_account(mint: &str, wallet_address: &str) -> Result<String, SwapError> {
    log(LogTag::Wallet, "ATA", &format!("Attempting to close token account for mint: {}", mint));

    // First verify the token balance is actually zero
    match get_token_balance(wallet_address, mint).await {
        Ok(balance) => {
            if balance > 0 {
                return Err(
                    SwapError::InvalidAmount(
                        format!("Cannot close token account - still has {} tokens", balance)
                    )
                );
            }
            log(
                LogTag::Wallet,
                "ATA",
                &format!("Verified zero balance for {}, proceeding to close ATA", mint)
            );
        }
        Err(e) => {
            log(
                LogTag::Wallet,
                "WARN",
                &format!("Could not verify token balance before closing ATA: {}", e)
            );
            // Continue anyway - the close instruction will fail if tokens remain
        }
    }

    // Get the associated token account address
    let token_account = match get_associated_token_account(wallet_address, mint).await {
        Ok(account) => account,
        Err(e) => {
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
    let rpc_client = crate::rpc::get_rpc_client();
    let is_token_2022 = rpc_client
        .is_token_account_token_2022(&token_account).await
        .unwrap_or(false);

    if is_token_2022 {
        log(LogTag::Wallet, "ATA", "Detected Token-2022, using Token Extensions program");
    } else {
        log(LogTag::Wallet, "ATA", "Using standard SPL Token program");
    }

    // Create and send the close account instruction using GMGN API approach
    match close_ata(wallet_address, &token_account, mint, is_token_2022).await {
        Ok(signature) => {
            log(
                LogTag::Wallet,
                "SUCCESS",
                &format!("Successfully closed token account for {}. TX: {}", mint, signature)
            );
            Ok(signature)
        }
        Err(e) => {
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
) -> Result<String, SwapError> {
    let rpc_client = crate::rpc::get_rpc_client();
    rpc_client.get_associated_token_account(wallet_address, mint).await
}

/// Closes ATA using proper Solana SDK for real ATA closing
async fn close_ata(
    wallet_address: &str,
    token_account: &str,
    mint: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
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
    match build_and_send_close_instruction(wallet_address, token_account, is_token_2022).await {
        Ok(signature) => {
            log(LogTag::Wallet, "SUCCESS", &format!("ATA closed successfully. TX: {}", signature));
            Ok(signature)
        }
        Err(e) => {
            Err(e)
        }
    }
}

/// Builds and sends close account instruction using Solana SDK
async fn build_and_send_close_instruction(
    wallet_address: &str,
    token_account: &str,
    is_token_2022: bool
) -> Result<String, SwapError> {
    let configs = read_configs().map_err(|e| SwapError::ConfigError(e.to_string()))?;

    // Parse addresses
    let owner_pubkey = Pubkey::from_str(wallet_address).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid wallet address: {}", e))
    )?;

    let token_account_pubkey = Pubkey::from_str(token_account).map_err(|e|
        SwapError::InvalidAmount(format!("Invalid token account: {}", e))
    )?;

    // Decode private key
    let private_key_bytes = bs58
        ::decode(&configs.main_wallet_private)
        .into_vec()
        .map_err(|e| SwapError::ConfigError(format!("Invalid private key: {}", e)))?;

    let keypair = Keypair::try_from(&private_key_bytes[..]).map_err(|e|
        SwapError::ConfigError(format!("Failed to create keypair: {}", e))
    )?;

    // Build close account instruction
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
        ).map_err(|e|
            SwapError::TransactionError(format!("Failed to build close instruction: {}", e))
        )?
    };

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
    let rpc_client = crate::rpc::get_rpc_client();
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;

    // Build transaction
    let transaction = Transaction::new_signed_with_payer(
        &[close_instruction],
        Some(&owner_pubkey),
        &[&keypair],
        recent_blockhash
    );

    log(LogTag::Wallet, "ATA", "Built and signed close transaction");

    // Send transaction via RPC
    rpc_client.send_transaction(&transaction).await
}

/// Builds close instruction for Token-2022 accounts
fn build_token_2022_close_instruction(
    token_account: &Pubkey,
    owner: &Pubkey
) -> Result<Instruction, SwapError> {
    // Token-2022 uses the same close account instruction format as SPL Token
    // but with different program ID
    let token_2022_program_id = Pubkey::from_str(
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    ).map_err(|e| SwapError::TransactionError(format!("Invalid Token-2022 program ID: {}", e)))?;

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
// UNIVERSAL BROWSER RANDOMIZATION SYSTEM
// =============================================================================

/// Generate a random, complex user agent string that mimics real browsers
/// Uses combinations of browsers, OS versions, and additional components
pub fn get_random_user_agent() -> String {
    let mut rng = rand::thread_rng();
    
    // Browser components with versions
    let chrome_versions = [
        "121.0.0.0", "122.0.0.0", "123.0.0.0", "124.0.0.0", "125.0.0.0",
        "120.0.0.0", "119.0.0.0", "118.0.0.0", "117.0.0.0", "116.0.0.0"
    ];
    
    let firefox_versions = [
        "122.0", "123.0", "124.0", "125.0", "126.0",
        "121.0", "120.0", "119.0", "118.0", "117.0"
    ];
    
    let safari_versions = [
        "17.2.1", "17.3.1", "17.4.1", "16.6", "16.5.2",
        "17.1.2", "16.4.1", "16.3.3", "15.6.1", "15.5"
    ];
    
    let edge_versions = [
        "121.0.0.0", "122.0.0.0", "123.0.0.0", "124.0.0.0",
        "120.0.0.0", "119.0.0.0", "118.0.0.0", "117.0.0.0"
    ];
    
    // Operating system variations
    let windows_versions = [
        "Windows NT 10.0; Win64; x64",
        "Windows NT 11.0; Win64; x64", 
        "Windows NT 10.0; WOW64",
        "Windows NT 6.1; Win64; x64",
        "Windows NT 6.3; Win64; x64"
    ];
    
    let mac_versions = [
        "Macintosh; Intel Mac OS X 10_15_7",
        "Macintosh; Intel Mac OS X 11_7_10", 
        "Macintosh; Intel Mac OS X 12_6_8",
        "Macintosh; Intel Mac OS X 13_5_2",
        "Macintosh; Intel Mac OS X 14_1_2",
        "Macintosh; Apple M1 Mac OS X 12_6",
        "Macintosh; Apple M2 Mac OS X 13_4"
    ];
    
    let linux_versions = [
        "X11; Linux x86_64",
        "X11; Ubuntu; Linux x86_64",
        "X11; Linux i686",
        "X11; Fedora; Linux x86_64",
        "X11; CentOS; Linux x86_64"
    ];
    
    // Browser choice weights (Chrome most common, then Edge, Firefox, Safari)
    let browser_choice = rng.gen_range(0..100);
    
    match browser_choice {
        // Chrome variants (50% probability)
        0..=49 => {
            let version = chrome_versions.choose(&mut rng).unwrap();
            let webkit = format!("537.{}", rng.gen_range(30..40));
            let os_choice = rng.gen_range(0..3);
            
            let os = match os_choice {
                0 => windows_versions.choose(&mut rng).unwrap(),
                1 => mac_versions.choose(&mut rng).unwrap(),
                _ => linux_versions.choose(&mut rng).unwrap(),
            };
            
            format!("Mozilla/5.0 ({}) AppleWebKit/{} (KHTML, like Gecko) Chrome/{} Safari/{}", 
                   os, webkit, version, webkit)
        },
        
        // Edge variants (20% probability)
        50..=69 => {
            let version = edge_versions.choose(&mut rng).unwrap();
            let webkit = format!("537.{}", rng.gen_range(30..40));
            let os = windows_versions.choose(&mut rng).unwrap();
            
            format!("Mozilla/5.0 ({}) AppleWebKit/{} (KHTML, like Gecko) Chrome/{} Safari/{} Edg/{}", 
                   os, webkit, version, webkit, version)
        },
        
        // Firefox variants (20% probability)
        70..=89 => {
            let version = firefox_versions.choose(&mut rng).unwrap();
            let gecko_version = format!("{}.0", rng.gen_range(20100000..20240000));
            let os_choice = rng.gen_range(0..3);
            
            let os = match os_choice {
                0 => windows_versions.choose(&mut rng).unwrap().replace("Win64; x64", "rv:122.0"),
                1 => mac_versions.choose(&mut rng).unwrap().replace("Intel", "rv:122.0; Intel"),
                _ => linux_versions.choose(&mut rng).unwrap().replace("Linux x86_64", "Linux x86_64; rv:122.0"),
            };
            
            format!("Mozilla/5.0 ({}) Gecko/{} Firefox/{}", os, gecko_version, version)
        },
        
        // Safari variants (10% probability)
        _ => {
            let version = safari_versions.choose(&mut rng).unwrap();
            let webkit = format!("605.1.{}", rng.gen_range(10..20));
            let os = mac_versions.choose(&mut rng).unwrap();
            
            format!("Mozilla/5.0 ({}) AppleWebKit/{} (KHTML, like Gecko) Version/{} Safari/{}", 
                   os, webkit, version, webkit)
        }
    }
}

/// Generate a random, undetectable partner/application identifier
/// Creates realistic-looking application names that don't stand out
pub fn get_random_partner_id() -> String {
    let mut rng = rand::thread_rng();
    
    // Business/financial application prefixes
    let prefixes = [
        "trade", "crypto", "defi", "swap", "finance", "market", "invest", "capital",
        "yield", "liquidity", "portfolio", "asset", "vault", "bridge", "protocol",
        "analytics", "tracker", "monitor", "scanner", "aggregator", "optimizer"
    ];
    
    // Suffixes that sound professional
    let suffixes = [
        "pro", "hub", "lab", "tech", "net", "app", "tool", "bot", "ai", "platform",
        "suite", "engine", "core", "link", "bridge", "flow", "sync", "pulse",
        "scout", "lens", "radar", "scope", "dash", "view", "edge", "wave"
    ];
    
    // Company/organization style endings
    let company_endings = [
        "labs", "tech", "solutions", "systems", "ventures", "capital", "group",
        "partners", "analytics", "research", "finance", "trading", "protocols"
    ];
    
    let style_choice = rng.gen_range(0..100);
    
    match style_choice {
        // Simple compound names (40% probability)
        0..=39 => {
            let prefix = prefixes.choose(&mut rng).unwrap();
            let suffix = suffixes.choose(&mut rng).unwrap();
            format!("{}{}", prefix, suffix)
        },
        
        // Compound with separator (30% probability) 
        40..=69 => {
            let prefix = prefixes.choose(&mut rng).unwrap();
            let suffix = suffixes.choose(&mut rng).unwrap();
            let separators = ["-", "_", ""];
            let sep = separators.choose(&mut rng).unwrap();
            format!("{}{}{}", prefix, sep, suffix)
        },
        
        // Company style names (20% probability)
        70..=89 => {
            let base = prefixes.choose(&mut rng).unwrap();
            let ending = company_endings.choose(&mut rng).unwrap();
            format!("{}{}", base, ending)
        },
        
        // Numbered/versioned names (10% probability)
        _ => {
            let prefix = prefixes.choose(&mut rng).unwrap();
            let suffix = suffixes.choose(&mut rng).unwrap();
            let number = rng.gen_range(1..99);
            let patterns = [
                format!("{}{}{}", prefix, suffix, number),
                format!("{}{}v{}", prefix, suffix, number),
                format!("{}{}-{}", prefix, suffix, number),
            ];
            patterns.choose(&mut rng).unwrap().clone()
        }
    }
}

/// Generate a complex, randomized HTTP client configuration
/// Returns a configured reqwest::Client with random user agent and headers
pub fn create_randomized_http_client() -> Result<reqwest::Client, String> {
    let user_agent = get_random_user_agent();
    let mut rng = rand::thread_rng();
    
    // Additional random headers to make requests less detectable
    let accept_languages = [
        "en-US,en;q=0.9",
        "en-US,en;q=0.9,es;q=0.8",
        "en-GB,en;q=0.9,en-US;q=0.8",
        "en-US,en;q=0.8,es;q=0.6,fr;q=0.4",
        "en,en-US;q=0.9,en-GB;q=0.8"
    ];
    
    let accept_encodings = [
        "gzip, deflate, br",
        "gzip, deflate",
        "gzip, deflate, br, zstd"
    ];
    
    let connection_types = ["keep-alive", "close"];
    
    let sec_fetch_sites = ["same-origin", "cross-site", "same-site"];
    let sec_fetch_modes = ["cors", "navigate", "no-cors"];
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(&user_agent)
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            
            // Accept header variations
            headers.insert(
                reqwest::header::ACCEPT,
                "application/json, text/plain, */*".parse().unwrap()
            );
            
            // Random accept-language
            headers.insert(
                reqwest::header::ACCEPT_LANGUAGE,
                accept_languages.choose(&mut rng).unwrap().parse().unwrap()
            );
            
            // Random accept-encoding
            headers.insert(
                reqwest::header::ACCEPT_ENCODING,
                accept_encodings.choose(&mut rng).unwrap().parse().unwrap()
            );
            
            // Connection type
            headers.insert(
                reqwest::header::CONNECTION,
                connection_types.choose(&mut rng).unwrap().parse().unwrap()
            );
            
            // Modern browser security headers (randomized)
            if rng.gen_bool(0.8) {
                headers.insert(
                    reqwest::header::HeaderName::from_static("sec-fetch-site"),
                    sec_fetch_sites.choose(&mut rng).unwrap().parse().unwrap()
                );
            }
            
            if rng.gen_bool(0.8) {
                headers.insert(
                    reqwest::header::HeaderName::from_static("sec-fetch-mode"),
                    sec_fetch_modes.choose(&mut rng).unwrap().parse().unwrap()
                );
            }
            
            if rng.gen_bool(0.7) {
                headers.insert(
                    reqwest::header::HeaderName::from_static("sec-fetch-dest"),
                    "empty".parse().unwrap()
                );
            }
            
            headers
        })
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    Ok(client)
}
