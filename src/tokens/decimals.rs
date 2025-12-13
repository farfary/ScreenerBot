// tokens/decimals.rs
// Decimals lookup with memory caching and on-chain fallback
//
// ARCHITECTURE - SINGLE SOURCE OF TRUTH:
// - Memory cache (DECIMALS_CACHE) is the PRIMARY source for all reads
// - Database is ONLY for persistence and startup preload
// - Chain RPC is ONLY fetched once per token, then cached forever
//
// CACHE POPULATION:
// 1. Startup: service_new.rs loads all DB decimals into cache
// 2. Runtime: database.rs::upsert_token() caches on every DB write
// 3. Fallback: decimals::get() fetches from chain if cache/DB miss
//
// USAGE:
// - Pool decoders (sync): MUST use get_cached() - no fallback
// - Business logic (async): Use get() for guaranteed decimals with fallback
// - NEVER read DB directly - always use cache or get()

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};

use crate::logger::{self, LogTag};

pub use crate::constants::{SOL_DECIMALS, SOL_MINT};

use tokio::sync::Mutex as AsyncMutex;

// In-memory decimals cache for fast synchronous lookups
// Populated at startup + updated on every DB write
static DECIMALS_CACHE: std::sync::LazyLock<Arc<RwLock<HashMap<String, u8>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

// Single-flight locks to prevent duplicate fetches
static FETCH_LOCKS: std::sync::LazyLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// Track mints with unresolved decimals to avoid repeated expensive lookups
static FAILED_CACHE: std::sync::LazyLock<Arc<RwLock<HashSet<String>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashSet::new())));

// Cache for Token2022 detection (mint -> is_token_2022)
// true = Token2022, false = standard SPL token
static TOKEN_2022_CACHE: std::sync::LazyLock<Arc<RwLock<HashMap<String, bool>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

// =============================================================================
// PUBLIC API
// =============================================================================

/// Get decimals from in-memory cache only (sync, instant, no fetching)
///
/// Use this in:
/// - Sync contexts (pools calculator, decoders)
/// - Quick checks where you can't await
/// - Filtering where decimals must already exist
///
/// Returns None if not in cache - caller should handle appropriately
pub fn get_cached(mint: &str) -> Option<u8> {
    // SOL always has 9 decimals
    if mint == SOL_MINT {
        return Some(SOL_DECIMALS);
    }

    if is_marked_failure(mint) {
        return None;
    }

    let result = DECIMALS_CACHE
        .read()
        .ok()
        .and_then(|m| m.get(mint).copied());

    result
}

/// Check if a mint is Token2022 from cache only (sync, instant)
///
/// Returns None if not in cache - caller should use is_token_2022() for async check
pub fn is_token_2022_cached(mint: &str) -> Option<bool> {
    // SOL/WSOL is always standard SPL
    if mint == SOL_MINT {
        return Some(false);
    }

    TOKEN_2022_CACHE
        .read()
        .ok()
        .and_then(|m| m.get(mint).copied())
}

/// Check if a mint is Token2022 (async with RPC fallback)
///
/// Checks cache first, then fetches from chain if needed.
/// Result is cached for future calls.
pub async fn is_token_2022(mint: &str) -> bool {
    use crate::constants::TOKEN_2022_PROGRAM_ID;
    use crate::rpc::{get_new_rpc_client, RpcClientMethods};
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    // SOL/WSOL is always standard SPL
    if mint == SOL_MINT {
        return false;
    }

    // Check cache first
    if let Some(is_2022) = is_token_2022_cached(mint) {
        return is_2022;
    }

    // Fetch from chain
    let mint_pubkey = match Pubkey::from_str(mint) {
        Ok(pk) => pk,
        Err(_) => {
            // Invalid mint, assume standard SPL
            cache_token_2022(mint, false);
            return false;
        }
    };

    let rpc_client = get_new_rpc_client();
    match rpc_client.get_account(&mint_pubkey).await {
        Ok(Some(account)) => {
            let is_2022 = account.owner.to_string() == TOKEN_2022_PROGRAM_ID;
            cache_token_2022(mint, is_2022);
            if is_2022 {
                logger::debug(
                    LogTag::Tokens,
                    &format!("Token2022 detected: mint={}", mint),
                );
            }
            is_2022
        }
        Ok(None) => {
            logger::warning(
                LogTag::Tokens,
                &format!("Mint account not found: mint={}", mint),
            );
            false
        }
        Err(e) => {
            logger::warning(
                LogTag::Tokens,
                &format!("Failed to check Token2022 status: mint={} err={}", mint, e),
            );
            // On error, assume standard SPL (safer for fee collection)
            false
        }
    }
}

/// Cache Token2022 detection result
fn cache_token_2022(mint: &str, is_2022: bool) {
    if let Ok(mut w) = TOKEN_2022_CACHE.write() {
        w.insert(mint.to_string(), is_2022);
    }
}

/// Get decimals with fallback chain (cache → DB → chain)
///
/// Use this in:
/// - Async business logic (positions, verifier, webserver)
/// - Any context where you can await and need guaranteed decimals
///
/// Tries: memory cache → database → on-chain RPC
/// Returns None only if all methods fail
pub async fn get(mint: &str) -> Option<u8> {
    // Try cache first
    if let Some(d) = get_cached(mint) {
        return Some(d);
    }

    if is_marked_failure(mint) {
        return None;
    }

    // Try database
    if let Some(d) = get_from_db(mint).await {
        cache(mint, d);
        return Some(d);
    }

    // Acquire single-flight lock to avoid duplicate chain fetches
    let lock = fetch_lock_for(mint);
    let guard = lock.lock().await;

    // Double-check cache after acquiring lock
    if let Some(d) = get_cached(mint) {
        drop(guard);
        release_lock_if_idle(mint);
        return Some(d);
    }

    // Fetch from chain as last resort
    let chain_result = get_token_decimals_from_chain(mint).await;
    if let Ok(d) = chain_result {
        cache(mint, d);
        if let Err(e) = persist_to_db(mint, d).await {
            logger::warning(
                LogTag::Tokens,
                &format!("Failed to persist decimals to DB: mint={} err={}", mint, e),
            );
        }
        drop(guard);
        release_lock_if_idle(mint);
        return Some(d);
    }

    if let Err(err) = &chain_result {
        logger::warning(
            LogTag::Tokens,
            &format!(
                "Failed to fetch decimals from chain: mint={} err={}",
                mint, err
            ),
        );
    }

    if let Some(d) = get_from_rugcheck(mint).await {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Resolved decimals via RugCheck: mint={} decimals={}",
                mint, d
            ),
        );
        cache(mint, d);
        if let Err(e) = persist_to_db(mint, d).await {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "Failed to persist RugCheck decimals to DB: mint={} err={}",
                    mint, e
                ),
            );
        }
        drop(guard);
        release_lock_if_idle(mint);
        return Some(d);
    }

    logger::warning(
        LogTag::Tokens,
        &format!(
            "Unable to resolve decimals after all fallbacks: mint={}",
            mint
        ),
    );
    mark_failure(mint);
    drop(guard);
    release_lock_if_idle(mint);
    None
}

/// Fetch token decimals directly from Solana blockchain (public for debug bins)
pub async fn get_token_decimals_from_chain(mint: &str) -> Result<u8, String> {
    use crate::rpc::{get_new_rpc_client, RpcClientMethods};
    use solana_program::program_pack::Pack;
    use solana_sdk::pubkey::Pubkey;
    use spl_token::state::Mint as SplMint;
    use spl_token_2022::state::Mint as Mint2022;
    use std::str::FromStr;

    // SOL always has 9 decimals
    if mint == SOL_MINT {
        return Ok(SOL_DECIMALS);
    }

    // Parse mint address
    let mint_pubkey = Pubkey::from_str(mint).map_err(|e| format!("Invalid mint address: {}", e))?;

    // Get RPC client
    let rpc_client = get_new_rpc_client();

    // Fetch account data
    let account_opt = rpc_client.get_account(&mint_pubkey).await.map_err(|e| {
        if e.contains("could not find account") || e.contains("Account not found") {
            "Account not found".to_string()
        } else if e.contains("429") || e.to_lowercase().contains("rate limit") {
            format!("Rate limited: {}", e)
        } else {
            format!("Failed to fetch account: {}", e)
        }
    })?;

    let account = account_opt.ok_or_else(|| "Account not found".to_string())?;

    // Check account data
    if account.data.is_empty() {
        return Err("Account data is empty".to_string());
    }

    // Check if it's an SPL Token mint
    if account.owner == spl_token::id() {
        let mint_data = SplMint::unpack(&account.data)
            .map_err(|e| format!("Failed to unpack SPL Token mint: {}", e))?;
        return Ok(mint_data.decimals);
    }

    // Check if it's a Token-2022 mint
    if account.owner == spl_token_2022::id() {
        // First, try unpack via the Token-2022 Mint directly
        if let Ok(mint_data) = Mint2022::unpack(&account.data) {
            return Ok(mint_data.decimals);
        }

        // Fallback: unpack with extensions-aware parser and read base.decimals
        // Some Token-2022 mints include extensions that require this API.
        match spl_token_2022::extension::StateWithExtensionsOwned::<Mint2022>::unpack(
            account.data.clone(),
        ) {
            Ok(state) => return Ok(state.base.decimals),
            Err(e) => {
                return Err(format!(
                    "Failed to unpack Token-2022 mint with extensions: {}",
                    e
                ))
            }
        }
    }

    Err(format!(
        "Account owner is not a supported token program: {}",
        account.owner
    ))
}

/// Manually cache a decimals value (used when fetched from other sources)
pub fn cache(mint: &str, decimals: u8) {
    if let Ok(mut w) = DECIMALS_CACHE.write() {
        w.insert(mint.to_string(), decimals);
    }
    clear_failure(mint);
}

/// Clear cached decimals for a specific mint
pub fn clear_cache(mint: &str) {
    if let Ok(mut w) = DECIMALS_CACHE.write() {
        w.remove(mint);
    }
    clear_failure(mint);
}

/// Clear all cached decimals
pub fn clear_all_cache() {
    if let Ok(mut w) = DECIMALS_CACHE.write() {
        w.clear();
    }
    if let Ok(mut w) = FAILED_CACHE.write() {
        w.clear();
    }
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/// Try to get decimals from database
async fn get_from_db(mint: &str) -> Option<u8> {
    use crate::tokens::database::get_global_database;

    let db = get_global_database()?;
    let mint_owned = mint.to_string();
    let db_clone = db.clone();

    // Use spawn_blocking for synchronous database access
    let join_result = tokio::task::spawn_blocking(move || db_clone.get_token(&mint_owned))
        .await
        .ok()?;

    match join_result {
        Ok(Some(token)) => token
            .decimals
            .and_then(|value| if value > 0 { Some(value) } else { None }),
        Ok(None) => None,
        Err(_) => None,
    }
}

/// Try to get decimals from stored RugCheck data
async fn get_from_rugcheck(mint: &str) -> Option<u8> {
    use crate::tokens::database::get_global_database;

    let db = get_global_database()?;
    let mint_owned = mint.to_string();
    let db_clone = db.clone();

    let join_result = tokio::task::spawn_blocking(move || db_clone.get_rugcheck_data(&mint_owned))
        .await
        .ok()?;

    match join_result {
        Ok(Some(data)) => data
            .token_decimals
            .and_then(|value| if value > 0 { Some(value) } else { None }),
        Ok(None) => None,
        Err(_) => None,
    }
}

/// Persist decimals to database (internal - only called by get() after chain fetch)
///
/// NOTE: This calls upsert_token() which will ALSO update the cache automatically.
/// This ensures cache and DB stay synchronized.
async fn persist_to_db(mint: &str, decimals: u8) -> Result<(), String> {
    use crate::tokens::database::get_global_database;

    let db = get_global_database().ok_or("Database not initialized")?;
    let mint = mint.to_string();

    // Use spawn_blocking for synchronous database access
    tokio::task::spawn_blocking(move || db.upsert_token(&mint, None, None, Some(decimals)))
        .await
        .map_err(|e| format!("Task join error: {}", e))?
        .map_err(|e| format!("Database error: {}", e))
}

fn fetch_lock_for(mint: &str) -> Arc<AsyncMutex<()>> {
    let mut map = FETCH_LOCKS.lock().expect("decimals fetch locks poisoned");
    Arc::clone(
        map.entry(mint.to_string())
            .or_insert_with(|| Arc::new(AsyncMutex::new(()))),
    )
}

fn release_lock_if_idle(mint: &str) {
    if let Ok(mut map) = FETCH_LOCKS.lock() {
        map.remove(mint);
    }
}

fn mark_failure(mint: &str) {
    if let Ok(mut w) = FAILED_CACHE.write() {
        w.insert(mint.to_string());
    }
}

fn clear_failure(mint: &str) {
    if let Ok(mut w) = FAILED_CACHE.write() {
        w.remove(mint);
    }
}

fn is_marked_failure(mint: &str) -> bool {
    FAILED_CACHE
        .read()
        .map(|set| set.contains(mint))
        .unwrap_or(false)
}
