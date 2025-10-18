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

pub async fn get(mint: &str) -> Option<u8> {
    DECIMALS_CACHE
        .read()
        .ok()
        .and_then(|m| m.get(mint).copied())
}

pub async fn ensure(provider: &TokenDataProvider, mint: &str) -> Result<u8, String> {
    if let Some(d) = get(mint).await {
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
    if let Some(d) = get(mint).await {
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

    // 3) Chain fallback via existing tokens::decimals helper
    match crate::tokens::decimals::get_token_decimals_from_chain(mint).await {
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
