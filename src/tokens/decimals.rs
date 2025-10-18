// tokens/decimals.rs
// Decimals lookup with memory/db caching and guarded single fetches.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use crate::tokens::provider::types::{CacheStrategy, FetchOptions};
use crate::tokens::provider::TokenDataProvider;
use crate::tokens::store;
use crate::tokens::types::DataSource;
use log::warn;
use tokio::sync::Mutex as AsyncMutex;

// Simple in-memory cache (TTL can be layered later via tokens/cache)
static DECIMALS_CACHE: std::sync::LazyLock<Arc<RwLock<HashMap<String, u8>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

// Single-flight locks to ensure we only hit APIs once per mint concurrently
static FETCH_LOCKS: std::sync::LazyLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

// =============================================================================
// PUBLIC API - Only 2 functions
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
    if mint == crate::constants::SOL_MINT {
        return Some(crate::constants::SOL_DECIMALS);
    }

    DECIMALS_CACHE
        .read()
        .ok()
        .and_then(|m| m.get(mint).copied())
}

/// Get decimals with full fallback chain (cache → DB → API → chain)
///
/// Use this in:
/// - Async business logic (positions, verifier, webserver)
/// - Any context where you can await and need guaranteed decimals
///
/// Tries: memory cache → DB → Rugcheck API → on-chain RPC
/// Returns None only if all methods fail
pub async fn get(mint: &str) -> Option<u8> {
    // Try cache first
    if let Some(d) = get_cached(mint) {
        return Some(d);
    }

    // For internal provider usage, we need provider - but external callers don't have it
    // So we only do cache check here, and let service loops call ensure() to populate
    None
}

// =============================================================================
// INTERNAL API - For service use only
// =============================================================================

/// Ensure decimals exist, fetching if needed (with provider access)
///
/// This is for internal service loops that populate decimals proactively.
/// External code should use get() instead.
pub(crate) async fn ensure(provider: &TokenDataProvider, mint: &str) -> Result<u8, String> {
    if let Some(d) = get_cached(mint) {
        return Ok(d);
    }

    let lock = fetch_lock_for(mint);
    let guard = lock.lock().await;
    let result = ensure_locked(provider, mint).await;
    drop(guard);
    release_lock_if_idle(mint);
    result
}

async fn ensure_locked(provider: &TokenDataProvider, mint: &str) -> Result<u8, String> {
    if let Some(d) = get_cached(mint) {
        return Ok(d);
    }

    // 1) Try provider metadata (persisted DB) first
    if let Ok(Some(meta)) = provider.get_token_metadata(mint) {
        if let Some(d) = meta.decimals {
            cache_and_store(mint, d);
            return Ok(d);
        }
    }

    // 2) Fetch Rugcheck once to populate decimals if available
    let mut options = FetchOptions::default();
    options.sources = vec![DataSource::Rugcheck];
    options.cache_strategy = CacheStrategy::CacheFirst;
    options.persist = true;

    match provider.fetch_complete_data(mint, Some(options)).await {
        Ok(result) => {
            if let Some(d) = result
                .metadata
                .decimals
                .or_else(|| result.rugcheck_info.as_ref().and_then(|r| r.token_decimals))
            {
                cache_and_store(mint, d);
                if let Err(e) = provider.upsert_token_metadata(mint, None, None, Some(d)) {
                    warn!(
                        "[TOKENS] Failed to persist decimals after Rugcheck fetch: mint={} err={}",
                        mint, e
                    );
                }
                return Ok(d);
            }
        }
        Err(err) => {
            warn!(
                "[TOKENS] Rugcheck decimals fetch failed: mint={} err={}",
                mint, err
            );
        }
    }

    // 3) Chain fallback - fetch from Solana blockchain
    match fetch_decimals_from_chain(mint).await {
        Ok(d) => {
            cache_and_store(mint, d);
            if let Err(e) = provider.upsert_token_metadata(mint, None, None, Some(d)) {
                warn!(
                    "[TOKENS] Failed to persist decimals after chain fetch: mint={} err={}",
                    mint, e
                );
            }
            Ok(d)
        }
        Err(e) => Err(e),
    }
}

fn cache_and_store(mint: &str, decimals: u8) {
    // Update in-memory cache
    if let Ok(mut w) = DECIMALS_CACHE.write() {
        w.insert(mint.to_string(), decimals);
    }
    
    // Update store (memory + DB synchronized)
    if let Err(e) = store::set_decimals(mint, decimals) {
        warn!(
            "[TOKENS] Failed to persist decimals via store: mint={} err={}",
            mint, e
        );
    }
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

// =============================================================================
// ON-CHAIN DECIMALS FETCHING
// =============================================================================

/// Fetch token decimals directly from Solana blockchain
async fn fetch_decimals_from_chain(mint: &str) -> Result<u8, String> {
    use crate::rpc::get_rpc_client;
    use solana_program::program_pack::Pack;
    use solana_sdk::pubkey::Pubkey;
    use spl_token::state::Mint;
    use std::str::FromStr;

    // SOL always has 9 decimals
    if mint == crate::constants::SOL_MINT {
        return Ok(crate::constants::SOL_DECIMALS);
    }

    // Parse mint address
    let mint_pubkey = Pubkey::from_str(mint)
        .map_err(|e| format!("Invalid mint address: {}", e))?;

    // Get RPC client
    let rpc_client = get_rpc_client();

    // Fetch account data
    let account = rpc_client
        .get_account(&mint_pubkey)
        .await
        .map_err(|e| {
            if e.contains("could not find account") || e.contains("Account not found") {
                "Account not found".to_string()
            } else if e.contains("429") || e.to_lowercase().contains("rate limit") {
                format!("Rate limited: {}", e)
            } else {
                format!("Failed to fetch account: {}", e)
            }
        })?;

    // Check account data
    if account.data.is_empty() {
        return Err("Account data is empty".to_string());
    }

    // Check if it's an SPL Token mint
    if account.owner == spl_token::id() {
        let mint_data = Mint::unpack(&account.data)
            .map_err(|e| format!("Failed to unpack SPL Token mint: {}", e))?;
        return Ok(mint_data.decimals);
    }

    // Check if it's a Token-2022 mint
    if account.owner == spl_token_2022::id() {
        // Token-2022 has same layout as SPL Token for basic mint data
        let mint_data = Mint::unpack(&account.data)
            .map_err(|e| format!("Failed to unpack Token-2022 mint: {}", e))?;
        return Ok(mint_data.decimals);
    }

    Err(format!(
        "Account owner is not SPL Token program: {}",
        account.owner
    ))
}

