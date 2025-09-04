use crate::global::{ is_debug_decimals_enabled, TOKENS_DATABASE };
/// Token decimals fetching from Solana blockchain
use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::is_system_or_stable_token;
use crate::utils::safe_truncate;
use once_cell::sync::Lazy;
use rusqlite::{ Connection, Result as SqliteResult };
use solana_program::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use spl_token::state::Mint;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::{ Arc, Mutex };

// =============================================================================
// DECIMAL CONSTANTS
// =============================================================================

/// SOL token decimals constant - ALWAYS use this instead of hardcoding 9
pub const SOL_DECIMALS: u8 = 9;

/// SOL token lamports per SOL constant - ALWAYS use this instead of hardcoding 1_000_000_000
pub const LAMPORTS_PER_SOL: u64 = 1_000_000_000;

// In-memory cache for frequently accessed decimals to avoid database hits
static DECIMAL_CACHE: Lazy<Arc<Mutex<HashMap<String, u8>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

// Cache for failed token lookups to avoid repeated failures
static FAILED_DECIMALS_CACHE: Lazy<Arc<Mutex<HashMap<String, String>>>> = Lazy::new(||
    Arc::new(Mutex::new(HashMap::new()))
);

/// Initialize decimals database tables
fn init_decimals_database() -> SqliteResult<()> {
    let conn = Connection::open(TOKENS_DATABASE)?;

    // Create decimals table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS decimals (
            mint TEXT PRIMARY KEY,
            decimals INTEGER NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        []
    )?;

    // Create failed decimals table for retryable vs permanent failures
    conn.execute(
        "CREATE TABLE IF NOT EXISTS failed_decimals (
            mint TEXT PRIMARY KEY,
            error_message TEXT NOT NULL,
            is_permanent INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        []
    )?;

    // Create indices for performance
    conn.execute("CREATE INDEX IF NOT EXISTS idx_decimals_updated ON decimals(updated_at)", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_failed_decimals_permanent ON failed_decimals(is_permanent)",
        []
    )?;

    Ok(())
}

/// Get decimals from database
fn get_decimals_from_db(mint: &str) -> Result<Option<u8>, String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    let mut stmt = conn
        .prepare("SELECT decimals FROM decimals WHERE mint = ?1")
        .map_err(|e| format!("Database prepare error: {}", e))?;

    let mut rows = stmt
        .query_map([mint], |row| Ok(row.get::<_, i32>(0)? as u8))
        .map_err(|e| format!("Database query error: {}", e))?;

    if let Some(row) = rows.next() {
        let decimals = row.map_err(|e| format!("Database row error: {}", e))?;
        Ok(Some(decimals))
    } else {
        Ok(None)
    }
}

/// Save decimals to database
fn save_decimals_to_db(mint: &str, decimals: u8) -> Result<(), String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    conn
        .execute(
            "INSERT OR REPLACE INTO decimals (mint, decimals, updated_at) VALUES (?1, ?2, datetime('now'))",
            [mint, &decimals.to_string()]
        )
        .map_err(|e| format!("Database save error: {}", e))?;

    Ok(())
}

/// Get failed token from database
fn get_failed_decimals_from_db(mint: &str) -> Result<Option<(String, bool)>, String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    let mut stmt = conn
        .prepare("SELECT error_message, is_permanent FROM failed_decimals WHERE mint = ?1")
        .map_err(|e| format!("Database prepare error: {}", e))?;

    let mut rows = stmt
        .query_map([mint], |row| { Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? == 1)) })
        .map_err(|e| format!("Database query error: {}", e))?;

    if let Some(row) = rows.next() {
        let result = row.map_err(|e| format!("Database row error: {}", e))?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Save failed token to database
fn save_failed_decimals_to_db(mint: &str, error: &str, is_permanent: bool) -> Result<(), String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    conn
        .execute(
            "INSERT OR REPLACE INTO failed_decimals (mint, error_message, is_permanent, updated_at) VALUES (?1, ?2, ?3, datetime('now'))",
            [mint, error, &(if is_permanent { 1 } else { 0 }).to_string()]
        )
        .map_err(|e| format!("Database save error: {}", e))?;

    Ok(())
}

/// Remove failed token from database (when retry succeeds)
fn remove_failed_decimals_from_db(mint: &str) -> Result<(), String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    conn
        .execute("DELETE FROM failed_decimals WHERE mint = ?1", [mint])
        .map_err(|e| format!("Database delete error: {}", e))?;

    Ok(())
}

/// Get token decimals from Solana blockchain with caching
pub async fn get_token_decimals_from_chain(mint: &str) -> Result<u8, String> {
    // CRITICAL: SOL (native token) always has 9 decimals
    if mint == "So11111111111111111111111111111111111111112" {
        return Ok(9);
    }

    // Skip system/stable tokens that shouldn't be processed
    if is_system_or_stable_token(mint) {
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "SKIP_SYSTEM",
                &format!("Skipping system/stable token: {}", mint)
            );
        }
        return Err("System or stable token excluded from processing".to_string());
    }

    // Check in-memory cache first for recently accessed tokens
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        if let Some(&decimals) = cache.get(mint) {
            return Ok(decimals);
        }
    }

    // Check database for previously cached decimals
    match get_decimals_from_db(mint) {
        Ok(Some(decimals)) => {
            // Add to in-memory cache for faster future access
            if let Ok(mut cache) = DECIMAL_CACHE.lock() {
                cache.insert(mint.to_string(), decimals);
            }
            return Ok(decimals);
        }
        Ok(None) => {
            // Not in database, continue to fetch
        }
        Err(e) => {
            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "DB_ERROR",
                    &format!("Database read error for {}: {}", mint, e)
                );
            }
            // Continue to fetch despite database error
        }
    }

    // Check failed decimals database - but allow retries for network/temporary errors
    match get_failed_decimals_from_db(mint) {
        Ok(Some((error, is_permanent))) => {
            if is_permanent {
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "CACHED_FAIL",
                        &format!("Skipping permanently failed token {}: {}", mint, error)
                    );
                }
                return Err(error);
            } else {
                // Network/temporary error - allow retry but log it
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "RETRY_CACHED",
                        &format!(
                            "Retrying previously failed token {} (network error): {}",
                            mint,
                            error
                        )
                    );
                }
                // Continue to fetch - don't return early
            }
        }
        Ok(None) => {
            // Not in failed cache, continue to fetch
        }
        Err(e) => {
            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "DB_ERROR",
                    &format!("Database failed read error for {}: {}", mint, e)
                );
            }
            // Continue to fetch despite database error
        }
    }

    // Use the batch function for single token (more efficient than separate implementation)
    let results = batch_fetch_token_decimals(&[mint.to_string()]).await;

    if let Some((_, result)) = results.first() {
        result.clone()
    } else {
        Err("No results returned from batch fetch".to_string())
    }
}

/// Check if a token has already failed decimal lookup
fn is_token_already_failed(mint: &str) -> bool {
    // Check in-memory cache first
    if let Ok(failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        if failed_cache.contains_key(mint) {
            return true;
        }
    }

    // Check database
    match get_failed_decimals_from_db(mint) {
        Ok(Some(_)) => true,
        _ => false,
    }
}

/// Check if a token failed with a permanent error (not retryable)
fn is_token_failed_permanently(mint: &str) -> bool {
    // Check database for permanent failures
    match get_failed_decimals_from_db(mint) {
        Ok(Some((_, is_permanent))) => is_permanent,
        _ => false,
    }
}

/// Add a token to the failed cache
fn cache_failed_token(mint: &str, error: &str) {
    let is_permanent = should_cache_as_failed(error);

    // Save to database
    if let Err(e) = save_failed_decimals_to_db(mint, error, is_permanent) {
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "DB_SAVE_ERROR",
                &format!("Failed to save failed token to database {}: {}", mint, e)
            );
        }
    }

    // Also keep in memory cache for immediate access
    if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        failed_cache.insert(mint.to_string(), error.to_string());
    }

    if is_debug_decimals_enabled() {
        log(
            LogTag::Decimals,
            "CACHE_FAIL",
            &format!("Cached failed lookup for {} (permanent: {}): {}", mint, is_permanent, error)
        );
    }
}

/// Check if error should be cached as failed (real errors) vs retried (rate limits)
fn should_cache_as_failed(error: &str) -> bool {
    let error_lower = error.to_lowercase();

    // Real blockchain state errors - cache as failed
    if
        error_lower.contains("account not found") ||
        error_lower.contains("invalid account") ||
        error_lower.contains("account does not exist") ||
        error_lower.contains("invalid mint") ||
        error_lower.contains("empty") ||
        error_lower.contains("account owner is not spl token program")
    {
        return true;
    }

    // Rate limiting and temporary issues - retry with different RPC
    if
        error_lower.contains("429") ||
        error_lower.contains("too many requests") ||
        error_lower.contains("rate limit") ||
        error_lower.contains("rate limited") ||
        error_lower.contains("timeout") ||
        error_lower.contains("connection") ||
        error_lower.contains("network") ||
        error_lower.contains("unavailable") ||
        error_lower.contains("error sending request") ||
        error_lower.contains("request failed") ||
        error_lower.contains("connection refused") ||
        error_lower.contains("connection reset") ||
        error_lower.contains("timed out") ||
        error_lower.contains("dns") ||
        error_lower.contains("ssl") ||
        error_lower.contains("tls") ||
        error_lower.contains("failed to get multiple accounts") ||
        error_lower.contains("batch fetch failed")
    {
        return false;
    }

    // Default to caching as failed for unknown errors
    true
}

/// Batch fetch token decimals using the centralized RPC client with automatic fallback
async fn batch_fetch_decimals_with_fallback(
    mint_pubkeys: &[Pubkey]
) -> Result<Vec<(Pubkey, Result<u8, String>)>, String> {
    let rpc_client = get_rpc_client();

    // Split into chunks of 50 to avoid provider HTTP 413 (Request Entity Too Large) limits
    // Some providers enforce tighter POST body limits; 50 is safer in practice.
    const MAX_ACCOUNTS_PER_CALL: usize = 50;
    let mut all_results = Vec::new();

    for chunk in mint_pubkeys.chunks(MAX_ACCOUNTS_PER_CALL) {
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "BATCH_START",
                &format!(
                    "Fetching {} accounts in batch: [{}]",
                    chunk.len(),
                    chunk
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            );
        }
        // Get multiple accounts in one RPC call using centralized client
        let accounts_res = rpc_client.get_multiple_accounts(chunk).await.map_err(|e| {
            // Improve error categorization
            if e.contains("429") || e.contains("rate limit") || e.contains("Too Many Requests") {
                format!("Rate limited: {}", e)
            } else if e.contains("413") || e.to_lowercase().contains("entity too large") {
                // Explicit hint for provider body-size limit
                format!(
                    "Request too large (413). Reduce batch size (currently {}): {}",
                    chunk.len(),
                    e
                )
            } else if
                e.contains("error sending request") ||
                e.to_lowercase().contains("connection")
            {
                format!("Network error: {}", e)
            } else {
                format!("Failed to get multiple accounts: {}", e)
            }
        });

        // If a 413 or batch-size-related error occurs, fall back to smaller sub-chunks (e.g., 25)
        let accounts = match accounts_res {
            Ok(accs) => accs,
            Err(err_msg) => {
                let lower = err_msg.to_lowercase();
                if lower.contains("413") || lower.contains("entity too large") {
                    // Split the current chunk further to 25 to bypass body-size limits
                    let mut recovered: Vec<Option<solana_sdk::account::Account>> =
                        Vec::with_capacity(chunk.len());
                    for sub in chunk.chunks(25) {
                        match rpc_client.get_multiple_accounts(sub).await {
                            Ok(part) => recovered.extend(part.into_iter()),
                            Err(e) => {
                                // If even sub-chunks fail, propagate the original 413 with context
                                return Err(
                                    format!(
                                        "Batch sub-request failed after 413 fallback (sub-size {}): {} | original: {}",
                                        sub.len(),
                                        e,
                                        err_msg
                                    )
                                );
                            }
                        }
                        // Small delay to be polite
                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                    }
                    recovered
                } else {
                    return Err(err_msg);
                }
            }
        };

        // Process each account result
        for (i, account_option) in accounts.iter().enumerate() {
            let mint_pubkey = chunk[i];
            let mint_str = mint_pubkey.to_string();

            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "ACCOUNT_INFO",
                    &format!("Processing account {} (full address: {})", i + 1, mint_str)
                );
            }

            let decimals_result = match account_option {
                Some(account) => {
                    if is_debug_decimals_enabled() {
                        log(
                            LogTag::Decimals,
                            "ACCOUNT_FOUND",
                            &format!(
                                "Account {} exists - Owner: {}, Lamports: {}, Data length: {}",
                                mint_str,
                                account.owner,
                                account.lamports,
                                account.data.len()
                            )
                        );
                    }

                    // Check if account exists and has data
                    if account.data.is_empty() {
                        let error_msg = "Account not found or empty".to_string();
                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "ACCOUNT_EMPTY",
                                &format!("❌ Account {} has empty data", mint_str)
                            );
                        }
                        Err(error_msg)
                    } else if
                        account.owner != spl_token::id() &&
                        account.owner != spl_token_2022::id()
                    {
                        let error_msg = format!(
                            "Account owner is not SPL Token program: {}",
                            account.owner
                        );
                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "WRONG_OWNER",
                                &format!(
                                    "❌ Account {} has wrong owner: {} (expected SPL Token program)",
                                    mint_str,
                                    account.owner
                                )
                            );
                        }
                        Err(error_msg)
                    } else {
                        // Parse mint data based on program type
                        if account.owner == spl_token::id() {
                            if is_debug_decimals_enabled() {
                                log(
                                    LogTag::Decimals,
                                    "PARSING_SPL",
                                    &format!("Account {} is SPL Token - parsing mint data", mint_str)
                                );
                            }

                            match parse_spl_token_mint(&account.data) {
                                Ok(decimals) => {
                                    if is_debug_decimals_enabled() {
                                        log(
                                            LogTag::Decimals,
                                            "DECIMALS_SUCCESS",
                                            &format!(
                                                "✅ Account {} extracted {} decimals from SPL Token mint",
                                                mint_str,
                                                decimals
                                            )
                                        );
                                    }
                                    Ok(decimals)
                                }
                                Err(e) => {
                                    if is_debug_decimals_enabled() {
                                        log(
                                            LogTag::Decimals,
                                            "PARSING_ERROR",
                                            &format!(
                                                "❌ Account {} SPL Token parsing failed: {}",
                                                mint_str,
                                                e
                                            )
                                        );
                                    }
                                    Err(format!("SPL Token parsing failed: {}", e))
                                }
                            }
                        } else {
                            if is_debug_decimals_enabled() {
                                log(
                                    LogTag::Decimals,
                                    "PARSING_2022",
                                    &format!("Account {} is SPL Token-2022 - parsing mint data", mint_str)
                                );
                            }

                            match parse_token_2022_mint(&account.data) {
                                Ok(decimals) => {
                                    if is_debug_decimals_enabled() {
                                        log(
                                            LogTag::Decimals,
                                            "DECIMALS_SUCCESS",
                                            &format!(
                                                "✅ Account {} extracted {} decimals from Token-2022 mint",
                                                mint_str,
                                                decimals
                                            )
                                        );
                                    }
                                    Ok(decimals)
                                }
                                Err(e) => {
                                    if is_debug_decimals_enabled() {
                                        log(
                                            LogTag::Decimals,
                                            "PARSING_ERROR",
                                            &format!(
                                                "❌ Account {} Token-2022 parsing failed: {}",
                                                mint_str,
                                                e
                                            )
                                        );
                                    }
                                    Err(format!("Token-2022 parsing failed: {}", e))
                                }
                            }
                        }
                    }
                }
                None => {
                    let error_msg = "Account not found".to_string();
                    if is_debug_decimals_enabled() {
                        log(
                            LogTag::Decimals,
                            "ACCOUNT_NOT_FOUND",
                            &format!("❌ Account {} does not exist on blockchain", mint_str)
                        );
                    }
                    Err(error_msg)
                }
            };

            all_results.push((mint_pubkey, decimals_result));
        }

        // Progressive delay between batches to avoid rate limiting
        if mint_pubkeys.len() > MAX_ACCOUNTS_PER_CALL {
            let delay_ms = if all_results.len() > 200 { 300 } else { 150 }; // Longer delay for large batches
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
    }

    if is_debug_decimals_enabled() {
        let success_count = all_results
            .iter()
            .filter(|(_, r)| r.is_ok())
            .count();
        let failed_count = all_results.len() - success_count;
        log(
            LogTag::Decimals,
            "BATCH_COMPLETE",
            &format!(
                "Batch processing complete: {} success, {} failed",
                success_count,
                failed_count
            )
        );
    }

    Ok(all_results)
}

/// Parse SPL Token mint data to extract decimals
fn parse_spl_token_mint(data: &[u8]) -> Result<u8, String> {
    if is_debug_decimals_enabled() {
        log(
            LogTag::Decimals,
            "SPL_PARSE_START",
            &format!(
                "Parsing SPL Token mint data - length: {}, expected: {}",
                data.len(),
                Mint::LEN
            )
        );
    }

    if data.len() < Mint::LEN {
        let error_msg = format!(
            "Invalid mint data length: expected {}, got {}",
            Mint::LEN,
            data.len()
        );
        if is_debug_decimals_enabled() {
            log(LogTag::Decimals, "SPL_PARSE_ERROR", &format!("❌ {}", error_msg));
        }
        return Err(error_msg);
    }

    // Parse using SPL Token library
    let mint = Mint::unpack(data).map_err(|e| {
        let error_msg = format!("Failed to unpack SPL Token mint: {}", e);
        if is_debug_decimals_enabled() {
            log(LogTag::Decimals, "SPL_UNPACK_ERROR", &format!("❌ {}", error_msg));
        }
        error_msg
    })?;

    if is_debug_decimals_enabled() {
        log(
            LogTag::Decimals,
            "SPL_PARSE_SUCCESS",
            &format!(
                "✅ SPL Token mint parsed - decimals: {}, supply: {}, mint_authority: {:?}, freeze_authority: {:?}",
                mint.decimals,
                mint.supply,
                mint.mint_authority,
                mint.freeze_authority
            )
        );
    }

    Ok(mint.decimals)
}

/// Parse SPL Token-2022 mint data to extract decimals
fn parse_token_2022_mint(data: &[u8]) -> Result<u8, String> {
    if is_debug_decimals_enabled() {
        log(
            LogTag::Decimals,
            "2022_PARSE_START",
            &format!("Parsing Token-2022 mint data - length: {}, minimum required: 44", data.len())
        );
    }

    // For Token-2022, the decimals are at the same position as in standard SPL Token
    // The first 44 bytes are the same structure for both token programs
    if data.len() < 44 {
        let error_msg = format!(
            "Invalid Token-2022 mint data length: expected at least 44, got {}",
            data.len()
        );
        if is_debug_decimals_enabled() {
            log(LogTag::Decimals, "2022_PARSE_ERROR", &format!("❌ {}", error_msg));
        }
        return Err(error_msg);
    }

    // Decimals are at offset 44 in both SPL Token and SPL Token-2022
    let decimals = data[44];

    if is_debug_decimals_enabled() {
        log(
            LogTag::Decimals,
            "2022_PARSE_SUCCESS",
            &format!("✅ Token-2022 mint parsed - decimals: {} (extracted from offset 44)", decimals)
        );

        // Show some additional data for debugging
        if data.len() >= 48 {
            log(
                LogTag::Decimals,
                "2022_EXTRA_DATA",
                &format!(
                    "Additional Token-2022 data - bytes 0-8: {:?}, bytes 44-48: {:?}",
                    &data[0..(8).min(data.len())],
                    &data[44..(48).min(data.len())]
                )
            );
        }
    }

    Ok(decimals)
}

/// Batch fetch decimals for multiple tokens using efficient batch RPC calls
pub async fn batch_fetch_token_decimals(mints: &[String]) -> Vec<(String, Result<u8, String>)> {
    if mints.is_empty() {
        return Vec::new();
    }

    // Convert mint strings to Pubkeys, filtering out invalid ones and handling SOL
    let mut valid_mints = Vec::new();
    let mut invalid_results = Vec::new();
    let mut sol_results = Vec::new();

    for mint in mints {
        // CRITICAL: Handle SOL (native token) first
        if mint == "So11111111111111111111111111111111111111112" {
            sol_results.push((mint.clone(), Ok(9u8)));
            continue;
        }

        // Skip system/stable tokens that shouldn't be in watch lists
        if is_system_or_stable_token(mint) {
            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "SKIP_SYSTEM",
                    &format!("Skipping system/stable token: {}", mint)
                );
            }
            continue;
        }

        match Pubkey::from_str(mint) {
            Ok(pubkey) => valid_mints.push((mint.clone(), pubkey)),
            Err(e) => {
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "INVALID_MINT",
                        &format!("Invalid mint address {}: {}", mint, e)
                    );
                }
                invalid_results.push((mint.clone(), Err(format!("Invalid mint address: {}", e))));
            }
        }
    }

    if valid_mints.is_empty() {
        // Return SOL results + invalid results if no other valid mints
        let mut all_results = sol_results;
        all_results.extend(invalid_results);
        return all_results;
    }

    // Check which tokens are not in cache and not previously failed
    let mut uncached_mints = Vec::new();
    let mut cached_results = Vec::new();

    for (mint_str, pubkey) in &valid_mints {
        // Check in-memory cache first
        if let Ok(cache) = DECIMAL_CACHE.lock() {
            if let Some(&decimals) = cache.get(mint_str) {
                cached_results.push((mint_str.clone(), Ok(decimals)));
                continue;
            }
        }

        // Check database for previously cached decimals
        match get_decimals_from_db(mint_str) {
            Ok(Some(decimals)) => {
                // Add to in-memory cache for faster future access
                if let Ok(mut cache) = DECIMAL_CACHE.lock() {
                    cache.insert(mint_str.clone(), decimals);
                }
                cached_results.push((mint_str.clone(), Ok(decimals)));
                continue;
            }
            Ok(None) => {
                // Not in database, check if permanently failed
            }
            Err(e) => {
                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "DB_ERROR",
                        &format!("Database read error for {}: {}", mint_str, e)
                    );
                }
                // Continue to process despite database error
            }
        }

        // Check if permanently failed
        if is_token_failed_permanently(mint_str) {
            // Get the error from database or memory cache
            let error = match get_failed_decimals_from_db(mint_str) {
                Ok(Some((error, _))) => error,
                _ => "Previously failed".to_string(),
            };
            cached_results.push((mint_str.clone(), Err(error.clone())));
            if is_debug_decimals_enabled() {
                log(
                    LogTag::Decimals,
                    "SKIP_FAILED",
                    &format!("Skipping permanently failed token {}: {}", mint_str, error)
                );
            }
        } else {
            // Either not failed, or failed with retryable error
            if is_token_already_failed(mint_str) && is_debug_decimals_enabled() {
                if let Ok(Some((error, _))) = get_failed_decimals_from_db(mint_str) {
                    log(
                        LogTag::Decimals,
                        "RETRY_BATCH",
                        &format!("Retrying token {} (network error): {}", mint_str, error)
                    );
                }
            }
            uncached_mints.push((mint_str.clone(), *pubkey));
        }
    }

    // Only log and fetch if there are uncached tokens and debug is enabled
    if uncached_mints.is_empty() {
        // Return all cached results in original order
        let mut all_results = Vec::new();
        for (mint_str, _) in &valid_mints {
            if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint_str) {
                all_results.push(cached_result.clone());
            }
        }
        all_results.extend(invalid_results);
        return all_results;
    }

    // Only log batch operations if debug is enabled and significant batch size
    if is_debug_decimals_enabled() && uncached_mints.len() > 3 {
        log(
            LogTag::Decimals,
            "BATCH_FETCH",
            &format!(
                "Fetching decimals for {} tokens (batch operation, cached: {})",
                uncached_mints.len(),
                cached_results.len()
            )
        );
    }

    // Use centralized RPC client with automatic fallback handling
    let mut fetch_results = Vec::new();
    let mut new_cache_entries = HashMap::new();

    if !uncached_mints.is_empty() {
        let uncached_pubkeys: Vec<Pubkey> = uncached_mints
            .iter()
            .map(|(_, pubkey)| *pubkey)
            .collect();

        match batch_fetch_decimals_with_fallback(&uncached_pubkeys).await {
            Ok(batch_results) => {
                for (i, (_pubkey, decimals_result)) in batch_results.iter().enumerate() {
                    let mint_str = &uncached_mints[i].0;

                    match decimals_result {
                        Ok(decimals) => {
                            // Save to database
                            if let Err(e) = save_decimals_to_db(mint_str, *decimals) {
                                if is_debug_decimals_enabled() {
                                    log(
                                        LogTag::Decimals,
                                        "DB_SAVE_ERROR",
                                        &format!(
                                            "Failed to save decimals to database {}: {}",
                                            mint_str,
                                            e
                                        )
                                    );
                                }
                            }

                            // Save to in-memory cache for immediate access
                            new_cache_entries.insert(mint_str.clone(), *decimals);
                            fetch_results.push((mint_str.clone(), Ok(*decimals)));

                            // Remove from failed cache if it was previously failed
                            if is_token_already_failed(mint_str) {
                                // Remove from database
                                if let Err(e) = remove_failed_decimals_from_db(mint_str) {
                                    if is_debug_decimals_enabled() {
                                        log(
                                            LogTag::Decimals,
                                            "DB_REMOVE_ERROR",
                                            &format!(
                                                "Failed to remove failed token from database {}: {}",
                                                mint_str,
                                                e
                                            )
                                        );
                                    }
                                }

                                // Remove from memory cache
                                if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
                                    if let Some(old_error) = failed_cache.remove(mint_str) {
                                        if is_debug_decimals_enabled() {
                                            log(
                                                LogTag::Decimals,
                                                "RETRY_SUCCESS",
                                                &format!(
                                                    "Token {} succeeded on retry, removed from failed cache (was: {})",
                                                    mint_str,
                                                    old_error
                                                )
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Cache failure for permanent errors
                            if should_cache_as_failed(e) {
                                cache_failed_token(mint_str, e);
                            }
                            fetch_results.push((mint_str.clone(), Err(e.clone())));

                            if is_debug_decimals_enabled() {
                                log(
                                    LogTag::Decimals,
                                    "FETCH_ERROR",
                                    &format!("Token {} failed: {}", mint_str, e)
                                );
                            }
                        }
                    }
                }

                if is_debug_decimals_enabled() && !fetch_results.is_empty() {
                    let success_count = fetch_results
                        .iter()
                        .filter(|(_, r)| r.is_ok())
                        .count();
                    log(
                        LogTag::Decimals,
                        "BATCH_SUCCESS",
                        &format!(
                            "Successfully fetched decimals for {}/{} tokens using centralized RPC client",
                            success_count,
                            fetch_results.len()
                        )
                    );
                }
            }
            Err(e) => {
                // If entire batch fails, mark all as failed with the batch error
                let error_msg = format!("Batch fetch failed: {}", e);
                let should_cache = should_cache_as_failed(&error_msg);

                if is_debug_decimals_enabled() {
                    log(
                        LogTag::Decimals,
                        "BATCH_ERROR",
                        &format!(
                            "Batch fetch failed for {} tokens: {} (caching: {})",
                            uncached_mints.len(),
                            e,
                            should_cache
                        )
                    );
                }

                for (mint_str, _) in &uncached_mints {
                    if should_cache {
                        cache_failed_token(mint_str, &error_msg);
                    } else {
                        // For network errors, just log but don't cache permanently
                        if is_debug_decimals_enabled() {
                            log(
                                LogTag::Decimals,
                                "BATCH_RETRY_LATER",
                                &format!("Network error for {}, will retry later: {}", mint_str, e)
                            );
                        }
                    }
                    fetch_results.push((mint_str.clone(), Err(error_msg.clone())));
                }
            }
        }
    }

    // Update in-memory cache if we have new entries
    if !new_cache_entries.is_empty() {
        if let Ok(mut cache) = DECIMAL_CACHE.lock() {
            let old_size = cache.len();
            cache.extend(new_cache_entries.clone());
            let new_size = cache.len();

            // Only log significant cache updates or in debug mode
            if is_debug_decimals_enabled() || new_cache_entries.len() > 5 {
                log(
                    LogTag::Decimals,
                    "CACHE_UPDATE",
                    &format!(
                        "Updated in-memory decimal cache: {} → {} entries (+{} new: {})",
                        old_size,
                        new_size,
                        new_cache_entries.len(),
                        new_cache_entries.keys().take(3).cloned().collect::<Vec<_>>().join(", ")
                    )
                );
            }
        }
    }

    // Combine cached and fetched results in original order
    let mut all_results = Vec::new();
    for (mint_str, _) in &valid_mints {
        // Check if this mint was cached
        if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint_str) {
            all_results.push(cached_result.clone());
        } else {
            // Find in fetch results
            if let Some(fetch_result) = fetch_results.iter().find(|(m, _)| m == mint_str) {
                all_results.push(fetch_result.clone());
            } else {
                // This shouldn't happen, but handle gracefully
                all_results.push((mint_str.clone(), Err("Failed to fetch decimals".to_string())));
            }
        }
    }

    // Add back the SOL results and invalid mint results
    all_results.extend(sol_results);
    all_results.extend(invalid_results);

    all_results
}

/// Get decimals from cache only (no RPC call)
pub fn get_cached_decimals(mint: &str) -> Option<u8> {
    // CRITICAL: SOL (native token) always has 9 decimals
    if mint == "So11111111111111111111111111111111111111112" {
        return Some(9);
    }

    // Check in-memory cache first
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        if let Some(&decimals) = cache.get(mint) {
            return Some(decimals);
        }
    }

    // Check database
    match get_decimals_from_db(mint) {
        Ok(Some(decimals)) => {
            // Add to in-memory cache for faster future access
            if let Ok(mut cache) = DECIMAL_CACHE.lock() {
                cache.insert(mint.to_string(), decimals);
            }
            Some(decimals)
        }
        _ => None,
    }
}

/// Batch get token decimals from blockchain with caching - efficient for multiple tokens
pub async fn get_multiple_token_decimals_from_chain(
    mints: &[String]
) -> Vec<(String, Result<u8, String>)> {
    if mints.is_empty() {
        return Vec::new();
    }

    // Check cache for all mints first
    let mut cached_results = Vec::new();
    let mut uncached_mints = Vec::new();

    if let Ok(cache) = DECIMAL_CACHE.lock() {
        for mint in mints {
            if let Some(&decimals) = cache.get(mint) {
                cached_results.push((mint.clone(), Ok(decimals)));
            } else {
                uncached_mints.push(mint.clone());
            }
        }
    } else {
        uncached_mints = mints.to_vec();
    }

    // If some mints are not cached, fetch them in batch
    let mut batch_results = Vec::new();
    if !uncached_mints.is_empty() {
        batch_results = batch_fetch_token_decimals(&uncached_mints).await;
    }

    // Combine cached and fetched results in original order
    let mut all_results = Vec::new();

    for mint in mints {
        // Check if this mint was cached
        if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint) {
            all_results.push(cached_result.clone());
        } else {
            // Find in batch results
            if let Some(batch_result) = batch_results.iter().find(|(m, _)| m == mint) {
                all_results.push(batch_result.clone());
            } else {
                // This shouldn't happen, but handle gracefully
                all_results.push((mint.clone(), Err("Failed to fetch decimals".to_string())));
            }
        }
    }

    all_results
}

/// Clear decimals cache (in-memory only, database preserved)
pub fn clear_decimals_cache() {
    if let Ok(mut cache) = DECIMAL_CACHE.lock() {
        let old_size = cache.len();
        cache.clear();
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "CACHE_CLEAR",
                &format!("Cleared in-memory decimal cache ({} entries), database preserved", old_size)
            );
        }
    }

    if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        let old_size = failed_cache.len();
        failed_cache.clear();
        if is_debug_decimals_enabled() {
            log(
                LogTag::Decimals,
                "FAILED_CACHE_CLEAR",
                &format!("Cleared in-memory failed cache ({} entries), database preserved", old_size)
            );
        }
    }
}

/// Get cache statistics
pub fn get_cache_stats() -> (usize, usize) {
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        let size = cache.len();
        let capacity = cache.capacity();
        (size, capacity)
    } else {
        (0, 0)
    }
}

/// Get database statistics for decimals
pub fn get_database_stats() -> Result<(usize, usize), String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    // Count decimals
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM decimals")
        .map_err(|e| format!("Database prepare error: {}", e))?;
    let decimals_count: i64 = stmt
        .query_row([], |row| row.get(0))
        .map_err(|e| format!("Database query error: {}", e))?;

    // Count failed decimals
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM failed_decimals")
        .map_err(|e| format!("Database prepare error: {}", e))?;
    let failed_count: i64 = stmt
        .query_row([], |row| row.get(0))
        .map_err(|e| format!("Database query error: {}", e))?;

    Ok((decimals_count as usize, failed_count as usize))
}

/// Clean up temporary/network errors from failed cache, keeping only permanent blockchain errors
pub fn cleanup_retryable_failed_cache() -> Result<(usize, usize), String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    // Delete non-permanent failures from database
    let removed_count = conn
        .execute("DELETE FROM failed_decimals WHERE is_permanent = 0", [])
        .map_err(|e| format!("Database delete error: {}", e))? as usize;

    // Get count of remaining permanent failures
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM failed_decimals WHERE is_permanent = 1")
        .map_err(|e| format!("Database prepare error: {}", e))?;
    let permanent_count: i64 = stmt
        .query_row([], |row| row.get(0))
        .map_err(|e| format!("Database query error: {}", e))?;

    // Also clean in-memory cache
    if let Ok(mut failed_cache) = FAILED_DECIMALS_CACHE.lock() {
        let original_memory_size = failed_cache.len();
        failed_cache.retain(|_mint, error| should_cache_as_failed(error));
        let cleaned_memory_size = failed_cache.len();
        let memory_removed = original_memory_size - cleaned_memory_size;

        if is_debug_decimals_enabled() && memory_removed > 0 {
            log(
                LogTag::Decimals,
                "MEMORY_CLEANUP",
                &format!("Cleaned in-memory failed cache: removed {} retryable errors", memory_removed)
            );
        }
    }

    if is_debug_decimals_enabled() {
        log(
            LogTag::Decimals,
            "CACHE_CLEANUP",
            &format!(
                "Cleaned failed cache database: removed {} retryable errors, kept {} permanent errors",
                removed_count,
                permanent_count
            )
        );
    }

    Ok((removed_count, permanent_count as usize))
}

/// Get failed cache statistics for debugging (from database)
pub fn get_failed_cache_stats() -> Result<(usize, usize, Vec<String>), String> {
    init_decimals_database().map_err(|e| format!("Database init error: {}", e))?;

    let conn = Connection::open(TOKENS_DATABASE).map_err(|e|
        format!("Database connection error: {}", e)
    )?;

    // Get total count
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM failed_decimals")
        .map_err(|e| format!("Database prepare error: {}", e))?;
    let total_count: i64 = stmt
        .query_row([], |row| row.get(0))
        .map_err(|e| format!("Database query error: {}", e))?;

    // Get permanent count
    let mut stmt = conn
        .prepare("SELECT COUNT(*) FROM failed_decimals WHERE is_permanent = 1")
        .map_err(|e| format!("Database prepare error: {}", e))?;
    let permanent_count: i64 = stmt
        .query_row([], |row| row.get(0))
        .map_err(|e| format!("Database query error: {}", e))?;

    // Get sample errors
    let mut stmt = conn
        .prepare("SELECT mint, error_message, is_permanent FROM failed_decimals LIMIT 5")
        .map_err(|e| format!("Database prepare error: {}", e))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)? == 1))
        })
        .map_err(|e| format!("Database query error: {}", e))?;

    let mut sample_errors = Vec::new();
    for row in rows {
        let (mint, error, is_permanent) = row.map_err(|e| format!("Database row error: {}", e))?;
        let permanent_flag = if is_permanent { "[P]" } else { "[T]" };
        sample_errors.push(format!("{}: {} {}", safe_truncate(&mint, 8), error, permanent_flag));
    }

    Ok((total_count as usize, permanent_count as usize, sample_errors))
}

// =============================================================================
// LAMPORTS CONVERSION UTILITIES
// =============================================================================

/// Convert lamports to SOL using the proper SOL decimals constant
pub fn lamports_to_sol(lamports: u64) -> f64 {
    (lamports as f64) / (LAMPORTS_PER_SOL as f64)
}

/// Convert SOL to lamports using the proper SOL decimals constant
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * (LAMPORTS_PER_SOL as f64)) as u64
}

/// Convert token amount to UI amount using provided decimals
pub fn raw_to_ui_amount(raw_amount: u64, decimals: u8) -> f64 {
    (raw_amount as f64) / (10f64).powi(decimals as i32)
}

/// Convert UI amount to raw token amount using provided decimals
pub fn ui_to_raw_amount(ui_amount: f64, decimals: u8) -> u64 {
    (ui_amount * (10f64).powi(decimals as i32)) as u64
}
