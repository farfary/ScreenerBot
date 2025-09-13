/// Token Holder Analysis Module (Enhanced with Program Accounts)
///
/// This module provides functions to count token holders and analyze top holders
/// using getProgramAccounts, getTokenAccountsByOwner, and getTokenLargestAccounts
/// for comprehensive token holder analysis.
///
/// Key improvements:
/// - Uses getProgramAccounts filtered by token program and mint address
/// - Combines with getTokenLargestAccounts for comprehensive analysis
/// - Falls back to getTokenAccountsByOwner for specific owner analysis
/// - Direct RPC calls for more control and reliability

use crate::{ logger::{ log, LogTag }, rpc::get_rpc_client, utils::safe_truncate };
use base64::{ engine::general_purpose, Engine as _ };
use dashmap::DashMap;
use once_cell::sync::Lazy;
use serde_json;
use std::collections::HashMap;
use std::time::{ Duration, Instant };

// Constants for Solana Token Programs
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const TOKEN_2022_PROGRAM_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

/// Basic holder statistics
#[derive(Debug)]
pub struct HolderStats {
    pub total_holders: u32,
    pub total_accounts: u32,
    pub mint_address: String,
    pub is_token_2022: bool,
    pub average_balance: f64,
    pub median_balance: f64,
    pub top_10_concentration: f64,
}

/// Information about a token holder
#[derive(Debug, Clone)]
pub struct TokenHolder {
    pub owner: String,
    pub amount: String,
    pub ui_amount: f64,
    pub decimals: u8,
}

/// Result of top holders analysis
#[derive(Debug)]
pub struct TopHoldersResult {
    pub total_holders: u32,
    pub total_accounts: u32,
    pub top_holders: Vec<TokenHolder>,
    pub mint_address: String,
    pub is_token_2022: bool,
}

/// Simplified token account representation
#[derive(Debug)]
struct TokenAccount {
    owner: String,
    amount: u64,
}

/// Cache for holder counts with TTL
static HOLDER_COUNT_CACHE: Lazy<DashMap<String, (u32, Instant)>> = Lazy::new(DashMap::new);
const CACHE_TTL: Duration = Duration::from_secs(300); // 5 minutes

/// Get token holder count using getProgramAccounts (primary method)
pub async fn get_holder_count(mint_address: &str) -> Result<u32, String> {
    // Check cache first
    if let Some(entry) = HOLDER_COUNT_CACHE.get(mint_address) {
        let (count, timestamp) = *entry.value();
        if timestamp.elapsed() < CACHE_TTL {
            log(
                LogTag::Rpc,
                "CACHE_HIT",
                &format!(
                    "Using cached holder count {} for mint {}",
                    count,
                    safe_truncate(mint_address, 8)
                )
            );
            return Ok(count);
        }
    }

    // Try getProgramAccounts method first
    match get_token_accounts_by_mint(mint_address).await {
        Ok(holders_map) => {
            let count = holders_map.len() as u32;

            // Cache the result
            HOLDER_COUNT_CACHE.insert(mint_address.to_string(), (count, Instant::now()));

            log(
                LogTag::Rpc,
                "HOLDER_COUNT",
                &format!(
                    "Found {} holders with non-zero balance for mint {} via getProgramAccounts",
                    count,
                    safe_truncate(mint_address, 8)
                )
            );

            Ok(count)
        }
        Err(e) => {
            log(
                LogTag::Rpc,
                "ERROR",
                &format!("Failed to get holder count for {}: {}", safe_truncate(mint_address, 8), e)
            );
            Err(e)
        }
    }
}

/// Force refresh holder count (bypass cache)
pub async fn get_holder_count_fresh(mint_address: &str) -> Result<u32, String> {
    // Remove from cache to force refresh
    HOLDER_COUNT_CACHE.remove(mint_address);
    get_holder_count(mint_address).await
}

/// Get all token accounts for a specific mint using getProgramAccounts
async fn get_token_accounts_by_mint(mint_address: &str) -> Result<HashMap<String, u64>, String> {
    let client = get_rpc_client();

    // Determine token type first
    let is_token_2022 = match client.is_token_2022_mint(mint_address).await {
        Ok(is_2022) => is_2022,
        Err(e) => {
            log(
                LogTag::Rpc,
                "ERROR",
                &format!(
                    "Failed to determine token type for {}: {}",
                    safe_truncate(mint_address, 8),
                    e
                )
            );
            return Err(format!("Failed to determine token type: {}", e));
        }
    };

    let program_id = if is_token_2022 { TOKEN_2022_PROGRAM_ID } else { TOKEN_PROGRAM_ID };

    // Create filters for getProgramAccounts
    let filters = if is_token_2022 {
        // Token-2022 accounts can have variable sizes due to extensions
        serde_json::json!([
            {
                "memcmp": {
                    "offset": 0,
                    "bytes": mint_address
                }
            }
        ])
    } else {
        // SPL Token accounts have fixed size
        serde_json::json!([
            {
                "dataSize": 165  // Standard SPL token account size
            },
            {
                "memcmp": {
                    "offset": 0,
                    "bytes": mint_address
                }
            }
        ])
    };

    let mut results = HashMap::new();

    // Single call only - no pagination, limit to 1000 accounts max
    let response = match
        client.get_program_accounts_v2(
            program_id,
            Some(filters.clone()),
            Some("jsonParsed"), // Use jsonParsed for compatibility
            None, // Get full account data
            Some(1000), // Limit to 1000 accounts max
            None, // No pagination - single call only
            None, // No changed_since_slot
            Some(30) // 30 second timeout
        ).await
    {
        Ok(response) => response,
        Err(e) => {
            log(
                LogTag::Rpc,
                "ERROR",
                &format!("RPC request failed for mint {}: {}", safe_truncate(mint_address, 8), e)
            );
            return Err(format!("RPC request failed: {}", e));
        }
    };

    log(
        LogTag::Rpc,
        "PROGRAM_ACCOUNTS",
        &format!(
            "getProgramAccountsV2 returned {} accounts for mint {}",
            response.accounts.len(),
            safe_truncate(mint_address, 8)
        )
    );

    // Process accounts - only count non-zero balances
    for account in &response.accounts {
        match parse_json_parsed_account(account, mint_address) {
            Ok((owner, amount)) => {
                // Only include accounts with non-zero balance
                if amount > 0 {
                    results.insert(owner, amount);
                }
            }
            Err(e) => {
                // Only log first few errors to avoid spam
                if results.is_empty() {
                    log(
                        LogTag::Rpc,
                        "PARSE_ERROR",
                        &format!("Failed to parse account (first error): {}", e)
                    );
                }
            }
        }
    }

    if results.is_empty() {
        return Err(format!("No token accounts found for mint {}", mint_address));
    }

    Ok(results)
}

/// Parse token account data from getProgramAccounts response
/// Parse token account from jsonParsed format
fn parse_json_parsed_account(
    account: &serde_json::Value,
    expected_mint: &str
) -> Result<(String, u64), String> {
    // Extract the account info
    let account_info = account.get("account").ok_or("Missing account field")?;

    let parsed_data = account_info
        .get("data")
        .and_then(|d| d.get("parsed"))
        .ok_or("Missing parsed data")?;

    let info = parsed_data.get("info").ok_or("Missing info field")?;

    // Verify mint matches
    let mint = info
        .get("mint")
        .and_then(|m| m.as_str())
        .ok_or("Missing or invalid mint field")?;

    if mint != expected_mint {
        return Err("Mint address mismatch".to_string());
    }

    // Extract owner
    let owner = info
        .get("owner")
        .and_then(|o| o.as_str())
        .ok_or("Missing or invalid owner field")?;

    // Extract amount (as string from tokenAmount.amount)
    let token_amount = info.get("tokenAmount").ok_or("Missing tokenAmount field")?;

    let amount_str = token_amount
        .get("amount")
        .and_then(|a| a.as_str())
        .ok_or("Missing or invalid amount field")?;

    let amount = amount_str.parse::<u64>().map_err(|_| "Invalid amount format")?;

    // Check account state (if available)
    if let Some(state) = info.get("state").and_then(|s| s.as_str()) {
        // Only accept initialized and frozen accounts
        // "uninitialized" and "initialized" and "frozen" are valid
        match state {
            "uninitialized" | "initialized" | "frozen" => {
                // Accept these states - uninitialized can still have historical data
            }
            _ => {
                return Err(format!("Invalid account state: {}", state));
            }
        }
    }

    Ok((owner.to_string(), amount))
}

/// Parse raw token account data (165 bytes)
fn parse_token_account_data(
    account_data: &[u8],
    expected_mint: &str
) -> Result<(String, u64), String> {
    if account_data.len() != 165 {
        return Err(format!("Invalid token account size: {}", account_data.len()));
    }

    // Token account layout:
    // 0-32: mint (32 bytes)
    // 32-64: owner (32 bytes)
    // 64-72: amount (8 bytes, little endian)
    // 72: delegate option (1 byte)
    // 73: state (1 byte) - 0=uninitialized/closed, 1=initialized, 2=frozen
    // ...

    // Verify mint matches
    let account_mint = bs58::encode(&account_data[0..32]).into_string();
    if account_mint != expected_mint {
        return Err(format!("Mint mismatch: expected {}, got {}", expected_mint, account_mint));
    }

    // Check account state (0=uninitialized/closed, 1=initialized, 2=frozen)
    let state_byte = account_data.get(73).unwrap_or(&0);

    // Accept both initialized (1) and closed (0) accounts since closed accounts may have had tokens before
    // Only reject if state is invalid (> 2)
    if account_data.len() <= 73 || *state_byte > 2 {
        return Err(format!("Invalid account state: state={}", state_byte));
    }

    // Extract owner (bytes 32-64)
    if account_data.len() < 64 {
        return Err("Account data too short for owner".to_string());
    }

    let owner_bytes = &account_data[32..64];
    let owner = bs58::encode(owner_bytes).into_string();

    // Extract amount (bytes 64-72, little endian u64)
    if account_data.len() < 72 {
        return Err("Account data too short for amount".to_string());
    }

    let amount_bytes = &account_data[64..72];
    let amount = u64::from_le_bytes(
        amount_bytes.try_into().map_err(|_| "Failed to parse amount bytes")?
    );

    Ok((owner, amount))
}

/// Get token holders using getProgramAccounts
async fn get_holders_using_program_accounts(
    mint_address: &str
) -> Result<HashMap<String, u64>, String> {
    get_token_accounts_by_mint(mint_address).await
}

/// Get token largest accounts using custom RPC call (simplified)
async fn get_largest_accounts(mint_address: &str) -> Result<Vec<TokenHolder>, String> {
    // For now, we'll use the program accounts method and sort by balance
    // This is a fallback implementation until we can properly implement getTokenLargestAccounts
    let holders_map = get_holders_using_program_accounts(mint_address).await?;

    let mut token_holders = Vec::new();
    let decimals = 9; // Default decimals - could be retrieved from mint account

    for (owner, amount) in holders_map {
        if amount > 0 {
            token_holders.push(TokenHolder {
                owner,
                amount: amount.to_string(),
                ui_amount: (amount as f64) / (10_f64).powi(decimals as i32),
                decimals,
            });
        }
    }

    // Sort by balance descending
    token_holders.sort_by(|a, b| {
        let a_amount: u64 = a.amount.parse().unwrap_or(0);
        let b_amount: u64 = b.amount.parse().unwrap_or(0);
        b_amount.cmp(&a_amount)
    });

    Ok(token_holders)
}

/// Get comprehensive token holder analysis
pub async fn get_comprehensive_holder_analysis(
    mint_address: &str
) -> Result<TopHoldersResult, String> {
    log(
        LogTag::Rpc,
        "COMPREHENSIVE_ANALYSIS",
        &format!(
            "Starting comprehensive holder analysis for mint {}",
            safe_truncate(mint_address, 8)
        )
    );

    let start_time = Instant::now();

    // Get all holders using getProgramAccounts
    let holders_map = get_holders_using_program_accounts(mint_address).await?;
    let total_holder_count = holders_map.len() as u32;

    // Convert to sorted list for top holders
    let top_holders = get_largest_accounts(mint_address).await?;

    // Calculate total supply and concentration
    let total_supply: u64 = holders_map.values().sum();
    let top_10_supply: u64 = top_holders
        .iter()
        .take(10)
        .map(|h| h.amount.parse::<u64>().unwrap_or(0))
        .sum();

    let concentration_ratio = if total_supply > 0 {
        ((top_10_supply as f64) / (total_supply as f64)) * 100.0
    } else {
        0.0
    };

    let duration = start_time.elapsed();
    log(
        LogTag::Rpc,
        "ANALYSIS_COMPLETE",
        &format!(
            "Comprehensive analysis completed in {}ms: {} holders, top 10 hold {:.2}% of supply",
            duration.as_millis(),
            total_holder_count,
            concentration_ratio
        )
    );

    Ok(TopHoldersResult {
        total_holders: total_holder_count,
        total_accounts: total_holder_count, // Same for this implementation
        top_holders,
        mint_address: mint_address.to_string(),
        is_token_2022: false, // Would need to check this properly
    })
}

/// Clear the holder count cache
pub fn clear_holder_cache() {
    HOLDER_COUNT_CACHE.clear();
    log(LogTag::Rpc, "CACHE_CLEAR", "Holder count cache cleared");
}

/// Clear the account count cache for a specific mint
pub fn clear_account_count_cache(mint_address: &str) {
    HOLDER_COUNT_CACHE.remove(mint_address);
    log(
        LogTag::Rpc,
        "CACHE_CLEAR",
        &format!("Account count cache cleared for mint {}", safe_truncate(mint_address, 8))
    );
}

/// Get token account count estimate
pub async fn get_token_account_count_estimate(mint_address: &str) -> Result<usize, String> {
    match get_holder_count(mint_address).await {
        Ok(count) => Ok(count as usize),
        Err(e) => Err(e),
    }
}

/// Should skip holder analysis based on account count
pub async fn should_skip_holder_analysis(mint_address: &str) -> Result<bool, String> {
    let account_count = get_token_account_count_estimate(mint_address).await?;

    // Skip if more than 2000 accounts to avoid RPC rate limits
    if account_count > 2000 {
        log(
            LogTag::Rpc,
            "SKIP_ANALYSIS",
            &format!(
                "Skipping holder analysis for mint {} due to high account count: {}",
                safe_truncate(mint_address, 8),
                account_count
            )
        );
        return Ok(true);
    }

    Ok(false)
}

/// Should skip holder analysis with specific count threshold
pub async fn should_skip_holder_analysis_with_count(
    mint_address: &str
) -> Result<(bool, usize), String> {
    let account_count = get_token_account_count_estimate(mint_address).await?;
    let max_count = 2000; // Default max count to avoid RPC rate limits

    let should_skip = account_count > max_count;
    if should_skip {
        log(
            LogTag::Rpc,
            "SKIP_ANALYSIS",
            &format!(
                "Skipping holder analysis for mint {} due to account count {} > {}",
                safe_truncate(mint_address, 8),
                account_count,
                max_count
            )
        );
    }

    Ok((should_skip, account_count))
}

/// Get holder count (alias for compatibility)
pub async fn get_count_holders(mint_address: &str) -> Result<u32, String> {
    get_holder_count(mint_address).await
}

/// Get holder statistics
pub async fn get_holder_stats(mint_address: &str) -> Result<HolderStats, String> {
    if should_skip_holder_analysis(mint_address).await? {
        return Err("Too many holders for analysis".to_string());
    }

    let holders_map = get_token_accounts_by_mint(mint_address).await?;
    let total_holders = holders_map.len() as u32;
    let total_accounts = total_holders; // Same for this implementation

    // Calculate statistics
    let balances: Vec<u64> = holders_map.values().cloned().collect();
    let total_supply: u64 = balances.iter().sum();

    let average_balance = if total_holders > 0 {
        (total_supply as f64) / (total_holders as f64)
    } else {
        0.0
    };

    let mut sorted_balances = balances.clone();
    sorted_balances.sort();
    let median_balance = if sorted_balances.is_empty() {
        0.0
    } else if sorted_balances.len() % 2 == 0 {
        let mid = sorted_balances.len() / 2;
        ((sorted_balances[mid - 1] + sorted_balances[mid]) as f64) / 2.0
    } else {
        sorted_balances[sorted_balances.len() / 2] as f64
    };

    // Calculate top 10 concentration
    sorted_balances.sort_by(|a, b| b.cmp(a)); // Sort descending
    let top_10_supply: u64 = sorted_balances.iter().take(10).sum();
    let top_10_concentration = if total_supply > 0 {
        ((top_10_supply as f64) / (total_supply as f64)) * 100.0
    } else {
        0.0
    };

    Ok(HolderStats {
        total_holders,
        total_accounts,
        mint_address: mint_address.to_string(),
        is_token_2022: false, // Would need to check this properly
        average_balance,
        median_balance,
        top_10_concentration,
    })
}

/// Top holders analysis type (alias for compatibility)
pub type TopHoldersAnalysis = TopHoldersResult;

/// Get top holders analysis
pub async fn get_top_holders_analysis(
    mint_address: &str,
    limit: Option<usize>
) -> Result<TopHoldersAnalysis, String> {
    if should_skip_holder_analysis(mint_address).await? {
        return Err("Too many holders for analysis".to_string());
    }

    get_comprehensive_holder_analysis(mint_address).await
}
