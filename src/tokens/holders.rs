/// Token Holder Analysis Module
///
/// This module provides functions to count token holders and analyze top holders
/// directly from Solana RPC using the existing RPC client system for consistent
/// error handling and rate limiting.
use crate::{
    errors::ScreenerBotError,
    logger::{ log, LogTag },
    rpc::get_rpc_client,
    utils::safe_truncate,
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use std::time::{ Duration, Instant };
use tokio::sync::Mutex;

/// Basic holder statistics
#[derive(Debug)]
pub struct HolderStats {
    pub total_holders: u32,
    pub total_accounts: u32,
    pub mint_address: String,
    pub is_token_2022: bool,
    pub average_balance: f64,
    pub median_balance: f64,
    pub top_10_concentration: f64, // Percentage of total supply held by top 10
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
pub struct TopHoldersAnalysis {
    pub total_holders: u32,
    pub total_accounts: u32,
    pub top_holders: Vec<TokenHolder>,
    pub mint_address: String,
    pub is_token_2022: bool,
}

/// Maximum number of token accounts we'll analyze (to prevent RPC timeouts)
const MAX_ANALYZABLE_ACCOUNTS: usize = 2000;

// Per-mint in-flight lock map to dedupe concurrent estimations
static INFLIGHT_LOCKS: Lazy<DashMap<String, Arc<Mutex<()>>>> = Lazy::new(|| DashMap::new());
// Short-lived cache of recent estimates to avoid immediate repeats across cycles
static ESTIMATE_CACHE: Lazy<DashMap<String, (Instant, usize)>> = Lazy::new(|| DashMap::new());
// TTL for cache entries
const ESTIMATE_TTL: Duration = Duration::from_secs(60);

/// Clear the cached estimate for a specific token
pub fn clear_account_count_cache(mint_address: &str) {
    ESTIMATE_CACHE.remove(mint_address);
    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "CACHE_CLEAR",
            &format!("Cleared cached account count for mint {}", safe_truncate(mint_address, 8))
        );
    }
}

/// Estimate the number of token accounts for a mint without fetching full data
/// This is used to determine if we should skip expensive holder analysis
/// Uses dataSlice to efficiently count accounts without downloading account data
pub async fn get_token_account_count_estimate(mint_address: &str) -> Result<usize, String> {
    get_token_account_count_estimate_with_cache(mint_address, true).await
}

/// Get token account count estimate with optional cache bypass
pub async fn get_token_account_count_estimate_with_cache(
    mint_address: &str,
    use_cache: bool
) -> Result<usize, String> {
    // Fast path: return cached value if fresh and cache is enabled
    if use_cache {
        if let Some((ts, value)) = ESTIMATE_CACHE.get(mint_address).map(|e| *e.value()) {
            if ts.elapsed() < ESTIMATE_TTL {
                if crate::arguments::is_debug_security_enabled() {
                    log(
                        LogTag::Security,
                        "CACHE",
                        &format!(
                            "Using cached account count estimate {} for mint {}",
                            value,
                            safe_truncate(mint_address, 8)
                        )
                    );
                }
                return Ok(value);
            }
        }
    } else {
        // Clear cache if bypass requested
        ESTIMATE_CACHE.remove(mint_address);
        if crate::arguments::is_debug_security_enabled() {
            log(
                LogTag::Security,
                "CACHE_BYPASS",
                &format!(
                    "Bypassing cache for account count estimate of mint {}",
                    safe_truncate(mint_address, 8)
                )
            );
        }
    }

    // Acquire per-mint lock to dedupe concurrent estimations
    let lock = INFLIGHT_LOCKS.entry(mint_address.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _guard = lock.lock().await;

    // Re-check cache after acquiring the lock (another task may have filled it)
    if use_cache {
        if let Some((ts, value)) = ESTIMATE_CACHE.get(mint_address).map(|e| *e.value()) {
            if ts.elapsed() < ESTIMATE_TTL {
                if crate::arguments::is_debug_security_enabled() {
                    log(
                        LogTag::Security,
                        "CACHE",
                        &format!(
                            "Using cached account count estimate {} for mint {}",
                            value,
                            safe_truncate(mint_address, 8)
                        )
                    );
                }
                return Ok(value);
            }
        }
    }

    // Only log when we actually perform the estimation (not for cache hits)
    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!("Estimating token account count for mint {}", safe_truncate(mint_address, 8))
        );
    }

    let rpc_client = get_rpc_client();

    // Determine token type first
    let is_token_2022 = match rpc_client.is_token_2022_mint(mint_address).await {
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

    let program_id = if is_token_2022 {
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    } else {
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    };

    // Create filters for getProgramAccounts
    let filters = if is_token_2022 {
        // Token-2022 accounts can have variable sizes due to extensions
        // Don't filter by dataSize for Token-2022 to catch all accounts
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

    // Use dataSlice with 0 length to only get account count without data (OPTIMIZATION!)
    let data_slice = serde_json::json!({
        "offset": 0,
        "length": 0
    });

    if crate::arguments::is_debug_rpc_enabled() {
        log(
            LogTag::Rpc,
            "DEBUG",
            &format!(
                "Using getProgramAccountsV2 with smart estimation for account counting for mint {}",
                safe_truncate(mint_address, 8)
            )
        );
    }

    // Use V2 method with LIMITED pagination for quick estimation
    // We don't need exact count, just need to know if it's a large token
    match
        rpc_client.get_program_accounts_v2(
            program_id,
            Some(filters.clone()),
            Some("base64"),
            Some(data_slice.clone()),
            Some(2000), // Larger first batch for better estimation
            None, // No pagination key for first request
            None, // No changed_since_slot
            Some(30) // 30 second timeout for count estimation
        ).await
    {
        Ok(response) => {
            let first_page_count = response.accounts.len();

            // Smart estimation: if first page is full and there's more, get better estimate
            let estimated_count = if response.pagination_key.is_some() && first_page_count >= 2000 {
                // Don't multiply by 5 immediately - try to get a second page for better estimate
                match
                    rpc_client.get_program_accounts_v2(
                        program_id,
                        Some(filters.clone()),
                        Some("base64"),
                        Some(data_slice.clone()),
                        Some(2000),
                        response.pagination_key.clone(),
                        None,
                        Some(30)
                    ).await
                {
                    Ok(second_page) => {
                        let total_from_two_pages = first_page_count + second_page.accounts.len();
                        if second_page.pagination_key.is_some() {
                            // Still more pages, but be more conservative with estimation
                            // If we got 4000+ accounts in two pages, it's likely large
                            if total_from_two_pages >= 4000 {
                                total_from_two_pages * 2 // More conservative estimate
                            } else {
                                total_from_two_pages + 1000 // Add buffer for remaining pages
                            }
                        } else {
                            // No more pages, exact count
                            total_from_two_pages
                        }
                    }
                    Err(_) => {
                        // Failed to get second page, use conservative estimate
                        if crate::arguments::is_debug_security_enabled() {
                            log(
                                LogTag::Security,
                                "DEBUG",
                                &format!(
                                    "Failed to get second page for better estimate, using conservative estimate for mint {}",
                                    safe_truncate(mint_address, 8)
                                )
                            );
                        }
                        first_page_count * 2 // More conservative than *5
                    }
                }
            } else if response.pagination_key.is_some() {
                // If there's pagination but first page isn't full, fetch one more page for better estimate
                match
                    rpc_client.get_program_accounts_v2(
                        program_id,
                        Some(filters.clone()),
                        Some("base64"),
                        Some(data_slice.clone()),
                        Some(2000),
                        response.pagination_key,
                        None,
                        Some(30)
                    ).await
                {
                    Ok(second_page) => {
                        let total_from_two_pages = first_page_count + second_page.accounts.len();
                        if second_page.pagination_key.is_some() {
                            // Still more pages, estimate total
                            total_from_two_pages * 3 // Conservative estimate
                        } else {
                            // No more pages, exact count
                            total_from_two_pages
                        }
                    }
                    Err(_) => {
                        // Failed to get second page, use first page estimate
                        first_page_count * 2
                    }
                }
            } else {
                // No pagination, exact count
                first_page_count
            };

            if crate::arguments::is_debug_security_enabled() {
                log(
                    LogTag::Security,
                    "DEBUG",
                    &format!(
                        "Estimated {} token accounts for mint {} (smart estimation with getProgramAccountsV2)",
                        estimated_count,
                        safe_truncate(mint_address, 8)
                    )
                );
            }
            // Update cache only if we're not bypassing cache
            if use_cache {
                ESTIMATE_CACHE.insert(mint_address.to_string(), (Instant::now(), estimated_count));
            }
            Ok(estimated_count)
        }
        Err(e) => {
            log(
                LogTag::Rpc,
                "ERROR",
                &format!(
                    "Failed to get account count for {} with getProgramAccountsV2: {}",
                    safe_truncate(mint_address, 8),
                    e
                )
            );
            Err(format!("Failed to get account count: {}", e))
        }
    }
}

/// Check if a token has too many accounts for efficient analysis
pub async fn should_skip_holder_analysis(mint_address: &str) -> Result<bool, String> {
    let account_count = get_token_account_count_estimate(mint_address).await?;
    let should_skip = account_count > MAX_ANALYZABLE_ACCOUNTS;

    if should_skip {
        if crate::arguments::is_debug_security_enabled() {
            log(
                LogTag::Security,
                "SKIP_ANALYSIS",
                &format!(
                    "Skipping holder analysis for {} - {} accounts exceeds maximum {}",
                    safe_truncate(mint_address, 8),
                    account_count,
                    MAX_ANALYZABLE_ACCOUNTS
                )
            );
        }
    }

    Ok(should_skip)
}

/// Check if holder analysis should be skipped, returning both decision and estimated count
pub async fn should_skip_holder_analysis_with_count(
    mint_address: &str
) -> Result<(bool, usize), String> {
    let account_count = get_token_account_count_estimate(mint_address).await?;
    let should_skip = account_count > MAX_ANALYZABLE_ACCOUNTS;

    if should_skip {
        if crate::arguments::is_debug_security_enabled() {
            log(
                LogTag::Security,
                "SKIP_ANALYSIS",
                &format!(
                    "Skipping holder analysis for {} - {} accounts exceeds maximum {}",
                    safe_truncate(mint_address, 8),
                    account_count,
                    MAX_ANALYZABLE_ACCOUNTS
                )
            );
        }
    }

    Ok((should_skip, account_count))
}

/// Core function to fetch all token accounts for a mint
/// This is the single source of truth for token account data
async fn fetch_token_accounts(
    mint_address: &str
) -> Result<(Vec<serde_json::Value>, bool), String> {
    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "FETCH_ACCOUNTS",
            &format!("Fetching token accounts for mint {}", safe_truncate(mint_address, 8))
        );
    }

    let rpc_client = get_rpc_client();

    // Determine token type first
    let is_token_2022 = match rpc_client.is_token_2022_mint(mint_address).await {
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

    let program_id = if is_token_2022 {
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"
    } else {
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    };

    // Create filters for getProgramAccounts
    let filters = if is_token_2022 {
        // Token-2022 accounts can have variable sizes due to extensions
        // Don't filter by dataSize for Token-2022 to catch all accounts
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

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "FETCH_ACCOUNTS",
            &format!(
                "Querying {} accounts for mint {} (60s timeout)",
                if is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                safe_truncate(mint_address, 8)
            )
        );
    }

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "FETCH_ACCOUNTS",
            &format!(
                "Using getProgramAccountsV2 with LIMITED fetching for {} accounts for mint {} (max {} accounts for security analysis)",
                if is_token_2022 {
                    "Token-2022"
                } else {
                    "SPL Token"
                },
                safe_truncate(mint_address, 8),
                MAX_ANALYZABLE_ACCOUNTS
            )
        );
    }

    // For security analysis, we only need enough accounts to analyze top holders
    // Don't fetch ALL accounts - just enough for security assessment
    let mut all_accounts = Vec::new();
    let mut pagination_key: Option<String> = None;
    let max_pages = 3; // Limit to 3 pages maximum (6000 accounts at 2000 per page)
    let mut page_count = 0;

    loop {
        match
            rpc_client.get_program_accounts_v2(
                program_id,
                Some(filters.clone()),
                Some("jsonParsed"),
                None, // No dataSlice for full account data
                Some(2000), // 2000 accounts per page
                pagination_key.clone(),
                None,
                Some(60) // 60 second timeout per page
            ).await
        {
            Ok(response) => {
                let page_accounts_count = response.accounts.len();
                all_accounts.extend(response.accounts);
                page_count += 1;

                if crate::arguments::is_debug_security_enabled() {
                    log(
                        LogTag::Security,
                        "FETCH_ACCOUNTS",
                        &format!(
                            "Fetched page {} with {} accounts (total: {}) for mint {}",
                            page_count,
                            page_accounts_count,
                            all_accounts.len(),
                            safe_truncate(mint_address, 8)
                        )
                    );
                }

                // Stop if we have enough accounts for security analysis OR reached max pages
                if
                    all_accounts.len() >= MAX_ANALYZABLE_ACCOUNTS ||
                    page_count >= max_pages ||
                    response.pagination_key.is_none()
                {
                    if
                        response.pagination_key.is_some() &&
                        all_accounts.len() >= MAX_ANALYZABLE_ACCOUNTS
                    {
                        if crate::arguments::is_debug_security_enabled() {
                            log(
                                LogTag::Security,
                                "FETCH_ACCOUNTS",
                                &format!(
                                    "Stopping fetch at {} accounts (sufficient for security analysis) for mint {}",
                                    all_accounts.len(),
                                    safe_truncate(mint_address, 8)
                                )
                            );
                        }
                    }
                    break;
                }

                pagination_key = response.pagination_key;
            }
            Err(e) => {
                let error_msg = match e {
                    ScreenerBotError::Network(ref net_err) => {
                        match net_err {
                            crate::errors::NetworkError::ConnectionTimeout {
                                endpoint,
                                timeout_ms,
                            } => {
                                format!(
                                    "Token fetch timeout for page {} ({}ms timeout): {}",
                                    page_count + 1,
                                    timeout_ms,
                                    endpoint
                                )
                            }
                            _ => format!("Network error: {}", net_err),
                        }
                    }
                    ScreenerBotError::RpcProvider(ref rpc_err) => {
                        match rpc_err {
                            crate::errors::RpcProviderError::RateLimitExceeded {
                                provider_name,
                                ..
                            } => {
                                format!("RPC rate limited ({}): Try again later", provider_name)
                            }
                            _ => format!("RPC provider error: {}", rpc_err),
                        }
                    }
                    _ => format!("Error: {}", e),
                };

                log(
                    LogTag::Rpc,
                    "ERROR",
                    &format!(
                        "Failed to fetch accounts page {} for mint {}: {}",
                        page_count + 1,
                        safe_truncate(mint_address, 8),
                        error_msg
                    )
                );

                // If we failed on first page, return error
                // If we failed on later pages but have some data, continue with what we have
                if page_count == 0 {
                    return Err(error_msg);
                } else {
                    log(
                        LogTag::Rpc,
                        "WARN",
                        &format!(
                            "Continuing with {} accounts from {} successful pages for mint {}",
                            all_accounts.len(),
                            page_count,
                            safe_truncate(mint_address, 8)
                        )
                    );
                    break;
                }
            }
        }
    }

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "FETCH_ACCOUNTS",
            &format!(
                "Successfully fetched {} total accounts from {} pages for mint {} (limited for security analysis)",
                all_accounts.len(),
                page_count,
                safe_truncate(mint_address, 8)
            )
        );
    }

    Ok((all_accounts, is_token_2022))
}

/// Extract holders from raw account data
/// Returns all holders with non-zero balances, sorted by balance (largest first)
fn extract_holders_from_accounts(accounts: &[serde_json::Value]) -> Vec<TokenHolder> {
    let mut holders: Vec<TokenHolder> = accounts
        .iter()
        .filter_map(|account| {
            if let Some(parsed) = account.get("account")?.get("data")?.get("parsed") {
                if let Some(info) = parsed.get("info") {
                    if let Some(token_amount) = info.get("tokenAmount") {
                        if let Some(amount) = token_amount.get("amount")?.as_str() {
                            if amount != "0" {
                                let ui_amount = token_amount
                                    .get("uiAmount")?
                                    .as_f64()
                                    .unwrap_or(0.0);
                                let decimals = token_amount
                                    .get("decimals")?
                                    .as_u64()
                                    .unwrap_or(0) as u8;
                                let owner = info.get("owner")?.as_str().unwrap_or("").to_string();

                                return Some(TokenHolder {
                                    owner,
                                    amount: amount.to_string(),
                                    ui_amount,
                                    decimals,
                                });
                            }
                        }
                    }
                }
            }
            None
        })
        .collect();

    // Sort by ui_amount (largest first)
    holders.sort_by(|a, b|
        b.ui_amount.partial_cmp(&a.ui_amount).unwrap_or(std::cmp::Ordering::Equal)
    );

    holders
}

/// Get holder count for any token
/// This is the primary function that should be used everywhere
pub async fn get_count_holders(mint_address: &str) -> Result<u32, String> {
    log(
        LogTag::Rpc,
        "HOLDER_COUNT",
        &format!("Counting holders for mint {}", safe_truncate(mint_address, 8))
    );

    let (accounts, is_token_2022) = fetch_token_accounts(mint_address).await?;

    // Count accounts with non-zero balance
    let holder_count = accounts
        .iter()
        .filter_map(|account| {
            if let Some(parsed) = account.get("account")?.get("data")?.get("parsed") {
                if let Some(info) = parsed.get("info") {
                    if let Some(token_amount) = info.get("tokenAmount") {
                        if let Some(amount) = token_amount.get("amount")?.as_str() {
                            if amount != "0" {
                                return Some(());
                            }
                        }
                    }
                }
            }
            None
        })
        .count();

    log(
        LogTag::Rpc,
        "HOLDER_COUNT",
        &format!(
            "Found {} {} holders out of {} total accounts for mint {}",
            holder_count,
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            },
            accounts.len(),
            safe_truncate(mint_address, 8)
        )
    );

    Ok(holder_count as u32)
}

/// Get top holders analysis for any token
/// Returns detailed information about the largest token holders
pub async fn get_top_holders_analysis(
    mint_address: &str,
    limit: Option<u32>
) -> Result<TopHoldersAnalysis, String> {
    let limit = limit.unwrap_or(50); // Default to top 50 holders

    log(
        LogTag::Rpc,
        "TOP_HOLDERS",
        &format!("Analyzing top {} holders for mint {}", limit, safe_truncate(mint_address, 8))
    );

    // Pre-check if token has too many accounts for efficient analysis
    if should_skip_holder_analysis(mint_address).await? {
        return Err(
            format!("Token has too many holders for single query (>{})", MAX_ANALYZABLE_ACCOUNTS)
        );
    }

    let (accounts, is_token_2022) = fetch_token_accounts(mint_address).await?;
    let mut holders = extract_holders_from_accounts(&accounts);

    let total_holders = holders.len() as u32;

    // Take only the top holders
    holders.truncate(limit as usize);

    log(
        LogTag::Rpc,
        "TOP_HOLDERS",
        &format!(
            "Found {} {} holders out of {} total accounts for mint {}. Returning top {}",
            total_holders,
            if is_token_2022 {
                "Token-2022"
            } else {
                "SPL Token"
            },
            accounts.len(),
            safe_truncate(mint_address, 8),
            holders.len()
        )
    );

    Ok(TopHoldersAnalysis {
        total_holders,
        total_accounts: accounts.len() as u32,
        top_holders: holders,
        mint_address: mint_address.to_string(),
        is_token_2022,
    })
}

/// Get basic holder statistics
pub async fn get_holder_stats(mint_address: &str) -> Result<HolderStats, String> {
    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "HOLDER_STATS",
            &format!("Getting holder statistics for mint {}", safe_truncate(mint_address, 8))
        );
    }

    // Pre-check if token has too many accounts for efficient analysis
    if should_skip_holder_analysis(mint_address).await? {
        return Err(
            format!("Token has too many holders for single query (>{})", MAX_ANALYZABLE_ACCOUNTS)
        );
    }

    let (accounts, is_token_2022) = fetch_token_accounts(mint_address).await?;
    let holders = extract_holders_from_accounts(&accounts);

    let total_holders = holders.len() as u32;
    let total_accounts = accounts.len() as u32;

    // Calculate total supply from all holders
    let total_supply: f64 = holders
        .iter()
        .map(|h| h.ui_amount)
        .sum();
    let average_balance = if total_holders > 0 {
        total_supply / (total_holders as f64)
    } else {
        0.0
    };

    // Calculate median from all holders
    let mut all_balances: Vec<f64> = holders
        .iter()
        .map(|h| h.ui_amount)
        .collect();
    all_balances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median_balance = if all_balances.is_empty() {
        0.0
    } else if all_balances.len() % 2 == 0 {
        let mid = all_balances.len() / 2;
        (all_balances[mid - 1] + all_balances[mid]) / 2.0
    } else {
        all_balances[all_balances.len() / 2]
    };

    // Calculate top 10 concentration
    let top_10_supply: f64 = holders
        .iter()
        .take(10)
        .map(|h| h.ui_amount)
        .sum();
    let top_10_concentration = if total_supply > 0.0 {
        (top_10_supply / total_supply) * 100.0
    } else {
        0.0
    };

    log(
        LogTag::Rpc,
        "HOLDER_STATS",
        &format!(
            "Stats for {}: {} holders, avg: {:.2}, median: {:.2}, top10: {:.1}%",
            safe_truncate(mint_address, 8),
            total_holders,
            average_balance,
            median_balance,
            top_10_concentration
        )
    );

    Ok(HolderStats {
        total_holders,
        total_accounts,
        mint_address: mint_address.to_string(),
        is_token_2022,
        average_balance,
        median_balance,
        top_10_concentration,
    })
}
