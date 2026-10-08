// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token decimals — on-chain decimal resolution for SPL and Token-2022 mints.

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

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use crate::chains::{adapter_for, runtime_for, ChainId, PerChain, TokenAccountFacts};
use crate::logger::{self, LogTag};
use crate::tokens::database::{database, require_database};

use tokio::sync::Mutex as AsyncMutex;

/// Largest decimals value any token this app trades can carry. Anything above this is junk from a
/// bad data source (the production database holds values like 123 and 186).
pub const MAX_DECIMALS: u8 = 18;

/// Is this a decimals value we are willing to act on?
///
/// Zero is rejected deliberately. It is a legal on-chain value, but every persistence path
/// here already treats a stored `0` as "never resolved" (`get_from_db` and the startup
/// preload both require `> 0`), so honouring it as real would make the same mint resolve
/// differently depending on which layer answered. A genuine 0-decimals mint still resolves
/// correctly through the chain fallback.
pub fn is_valid(decimals: u8) -> bool {
    decimals > 0 && decimals <= MAX_DECIMALS
}

/// Entry ceiling of the in-memory decimals cache.
pub const CACHE_CAPACITY: u64 = 100_000;

/// How many rows the startup preload may load. Deliberately below `CACHE_CAPACITY` so the
/// runtime inserts that follow (every token upsert calls `cache()`) have room to land
/// without evicting a preloaded pool mint the synchronous decoders depend on.
pub const PRELOAD_CAPACITY: usize = (CACHE_CAPACITY as usize) * 4 / 5;

// Single-flight fetch locks are keyed by chain and mint together.
type CacheKey = (ChainId, String);

fn cache_key(chain: ChainId, mint: &str) -> CacheKey {
    (chain, mint.to_owned())
}

// In-memory decimals cache — one bounded moka cache per chain for fast synchronous
// lookups. Populated at startup + updated on every DB write.
static DECIMALS_CACHE: PerChain<moka::sync::Cache<String, u8>> = PerChain::new(new_decimals_cache);

fn new_decimals_cache(_chain: ChainId) -> moka::sync::Cache<String, u8> {
    moka::sync::Cache::builder()
        .max_capacity(CACHE_CAPACITY)
        .build()
}

// Single-flight locks to prevent duplicate fetches
static FETCH_LOCKS: LazyLock<Mutex<HashMap<CacheKey, Arc<AsyncMutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// Track mints with unresolved decimals to avoid repeated expensive lookups
// Bounded moka cache per chain (max 50K entries, 24-hour TTL) to prevent unbounded growth
static FAILED_CACHE: PerChain<moka::sync::Cache<String, ()>> = PerChain::new(new_marker_cache);

// Mints we have already warned about for invalid (>18) decimals, so the warning
// fires once per mint instead of on every token upsert. Bounded + TTL'd, per chain.
static INVALID_DECIMALS_WARNED: PerChain<moka::sync::Cache<String, ()>> =
    PerChain::new(new_marker_cache);

fn new_marker_cache(_chain: ChainId) -> moka::sync::Cache<String, ()> {
    moka::sync::Cache::builder()
        .max_capacity(50_000)
        .time_to_live(Duration::from_secs(86400)) // 24 hours
        .build()
}

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
pub fn get_cached(chain: ChainId, mint: &str) -> Option<u8> {
    // The native asset's decimals are a chain fact
    let adapter = adapter_for(chain);
    if adapter.is_native_asset(mint) {
        return Some(adapter.native_asset_decimals());
    }

    if is_marked_failure(chain, mint) {
        return None;
    }

    DECIMALS_CACHE.get(chain).get(mint)
}

/// Get decimals with fallback chain (cache → DB → chain)
///
/// Use this in:
/// - Async business logic (positions, verifier, webserver)
/// - Any context where you can await and need guaranteed decimals
///
/// Tries: memory cache → database → on-chain RPC
/// Returns None only if all methods fail
pub async fn get(chain: ChainId, mint: &str) -> Option<u8> {
    // Try cache first
    if let Some(d) = get_cached(chain, mint) {
        return Some(d);
    }

    if is_marked_failure(chain, mint) {
        return None;
    }

    // Try database
    if let Some(d) = get_from_db(chain, mint).await {
        cache(chain, mint, d);
        return Some(d);
    }

    // Acquire single-flight lock to avoid duplicate chain fetches
    let lock = fetch_lock_for(chain, mint);
    let guard = lock.lock().await;

    // Double-check cache after acquiring lock
    if let Some(d) = get_cached(chain, mint) {
        drop(guard);
        release_lock_if_idle(chain, mint);
        return Some(d);
    }

    // Try the self-hosted ScreenerBot data server FIRST (a fast shared cache of
    // Rugcheck-sourced decimals). On any disabled/miss/timeout it yields None and
    // we fall through to on-chain extraction below. This path never touches a
    // provider rate limiter (the server enforces its own per-IP limit).
    if let Some(d) = get_from_server(chain, mint).await {
        cache(chain, mint, d);
        if let Err(e) = persist_to_db(chain, mint, d).await {
            logger::warning(
                LogTag::Tokens,
                &format!("Failed to persist server decimals to DB: mint={mint} err={e}"),
            );
        }
        drop(guard);
        release_lock_if_idle(chain, mint);
        return Some(d);
    }

    // Fetch from chain (self-extraction) as fallback after the server.
    let chain_result = get_token_decimals_from_chain(chain, mint).await;
    if let Ok(d) = chain_result {
        cache(chain, mint, d);
        if let Err(e) = persist_to_db(chain, mint, d).await {
            logger::warning(
                LogTag::Tokens,
                &format!("Failed to persist decimals to DB: mint={mint} err={e}"),
            );
        }
        drop(guard);
        release_lock_if_idle(chain, mint);
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

    if let Some(d) = get_from_rugcheck(chain, mint).await {
        logger::debug(
            LogTag::Tokens,
            &format!(
                "Resolved decimals via RugCheck: mint={} decimals={}",
                mint, d
            ),
        );
        cache(chain, mint, d);
        if let Err(e) = persist_to_db(chain, mint, d).await {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "Failed to persist RugCheck decimals to DB: mint={} err={}",
                    mint, e
                ),
            );
        }
        drop(guard);
        release_lock_if_idle(chain, mint);
        return Some(d);
    }

    logger::warning(
        LogTag::Tokens,
        &format!(
            "Unable to resolve decimals after all fallbacks: mint={}",
            mint
        ),
    );
    mark_failure(chain, mint);
    drop(guard);
    release_lock_if_idle(chain, mint);
    None
}

/// Fetch token decimals directly from the chain's token account (public for debug bins)
pub async fn get_token_decimals_from_chain(
    chain: ChainId,
    mint: &str,
) -> crate::tokens::Result<u8> {
    // The native asset's decimals are a chain fact
    let adapter = adapter_for(chain);
    if adapter.is_native_asset(mint) {
        return Ok(adapter.native_asset_decimals());
    }

    let runtime = runtime_for(chain).ok_or(crate::chains::Error::ChainNotEnabled { chain })?;
    let facts = runtime.read_token_account(mint).await?;

    // Cache authority data as a side effect of the fetch we already paid for
    // (zero extra RPC cost).
    cache_authorities(chain, mint, &facts);

    Ok(facts.decimals)
}

/// Manually cache a decimals value (used when fetched from other sources)
pub fn cache(chain: ChainId, mint: &str, decimals: u8) {
    // Validate decimals is within reasonable bounds (SOL tokens use max 18 decimals)
    if decimals > MAX_DECIMALS {
        // Warn once per mint — this is called on every token upsert, so a token
        // carrying a junk decimals value (from a bad data source) would otherwise
        // re-log on every market update and flood the log (observed 1800+ lines).
        let warned = INVALID_DECIMALS_WARNED.get(chain);
        if !warned.contains_key(mint) {
            warned.insert(mint.to_owned(), ());
            crate::logger::warning(
                crate::logger::LogTag::Tokens,
                &format!(
                    "Ignoring invalid decimals {} for mint {} (max 18)",
                    decimals, mint
                ),
            );
        }
        return;
    }

    DECIMALS_CACHE.get(chain).insert(mint.to_owned(), decimals);
    clear_failure(chain, mint);
}

/// Clear cached decimals for a specific mint
pub fn clear_cache(chain: ChainId, mint: &str) {
    DECIMALS_CACHE.get(chain).invalidate(mint);
    clear_failure(chain, mint);
}

/// Clear all cached decimals, on every chain
pub fn clear_all_cache() {
    for (_, cache) in DECIMALS_CACHE.built() {
        cache.invalidate_all();
    }
    for (_, cache) in FAILED_CACHE.built() {
        cache.invalidate_all();
    }
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/// Try to get decimals from database
async fn get_from_db(chain: ChainId, mint: &str) -> Option<u8> {
    let db = database(chain)?;
    let mint_owned = mint.to_string();

    // Use spawn_blocking for synchronous database access
    let join_result = tokio::task::spawn_blocking(move || db.get_token(&mint_owned))
        .await
        .ok()?;

    match join_result {
        Ok(Some(token)) => token
            .decimals
            .and_then(|value| (value > 0).then_some(value)),
        Ok(None) => None,
        Err(_) => None,
    }
}

/// Try to get decimals from stored RugCheck data
/// Fetch decimals from the self-hosted ScreenerBot data server's `/v1/decimals`
/// endpoint. Returns `None` when the source is disabled/unconfigured or the
/// request misses/times out/errors, so the caller falls back to on-chain
/// extraction. Gated by the shared `[tokens.sources.screenerbot_server]` config;
/// deliberately at the request layer so no provider rate limiter is consumed.
async fn get_from_server(chain: ChainId, mint: &str) -> Option<u8> {
    // Response shape: { "decimals": { "<mint>": <n> }, "requested": N }.
    // A cold token returns an empty map (fetch scheduled server-side); treat that
    // as a miss and fall back to chain, warming the server cache for next time.
    let body: serde_json::Value = crate::data_server::get_json(
        crate::data_server::Surface::Tokens,
        chain,
        "/v1/decimals",
        &[("mint", mint.to_string())],
    )
    .await?;
    let value = body.pointer(&format!("/decimals/{mint}"))?.as_u64()?;
    (value <= u8::MAX as u64).then_some(value as u8)
}

async fn get_from_rugcheck(chain: ChainId, mint: &str) -> Option<u8> {
    let db = database(chain)?;
    let mint_owned = mint.to_string();

    let join_result = tokio::task::spawn_blocking(move || db.get_rugcheck_data(&mint_owned))
        .await
        .ok()?;

    match join_result {
        Ok(Some(data)) => data
            .token_decimals
            .and_then(|value| (value > 0).then_some(value)),
        Ok(None) => None,
        Err(_) => None,
    }
}

/// Persist decimals to database (internal - only called by get() after chain fetch)
///
/// NOTE: This calls upsert_token() which will ALSO update the cache automatically.
/// This ensures cache and DB stay synchronized.
async fn persist_to_db(chain: ChainId, mint: &str, decimals: u8) -> crate::tokens::Result<()> {
    let db = require_database(chain)?;
    let mint = mint.to_string();

    // Use spawn_blocking for synchronous database access
    tokio::task::spawn_blocking(move || db.upsert_token(&mint, None, None, Some(decimals)))
        .await
        .map_err(crate::errors::InternalError::from)??;
    Ok(())
}

fn fetch_lock_for(chain: ChainId, mint: &str) -> Arc<AsyncMutex<()>> {
    let mut map = FETCH_LOCKS.lock().expect("decimals fetch locks poisoned");

    // Periodic cleanup to prevent unbounded growth
    if map.len() > 10000 {
        crate::logger::warning(
            crate::logger::LogTag::Tokens,
            &format!(
                "Decimals fetch lock map has {} entries, clearing to prevent memory leak",
                map.len()
            ),
        );
        map.clear();
    }

    Arc::clone(
        map.entry(cache_key(chain, mint))
            .or_insert_with(|| Arc::new(AsyncMutex::new(()))),
    )
}

fn release_lock_if_idle(chain: ChainId, mint: &str) {
    if let Ok(mut map) = FETCH_LOCKS.lock() {
        map.remove(&cache_key(chain, mint));
    }
}

fn mark_failure(chain: ChainId, mint: &str) {
    FAILED_CACHE.get(chain).insert(mint.to_owned(), ());
}

fn clear_failure(chain: ChainId, mint: &str) {
    FAILED_CACHE.get(chain).invalidate(mint);
}

fn is_marked_failure(chain: ChainId, mint: &str) -> bool {
    FAILED_CACHE.get(chain).contains_key(mint)
}

// ============================================================================
// AUTHORITY CACHING — side effect of chain fetch, zero extra RPC cost
// ============================================================================

/// Cache the mint/freeze authority data a chain fetch already carried.
fn cache_authorities(chain: ChainId, mint: &str, facts: &TokenAccountFacts) {
    crate::tokens::authority_cache::cache_mint_authorities(
        chain,
        mint,
        crate::tokens::authority_cache::MintAuthorities {
            mint_authority: facts.mint_authority.clone(),
            freeze_authority: facts.freeze_authority.clone(),
        },
    );
}
