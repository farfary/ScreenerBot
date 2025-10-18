// tokens_new/decimals.rs
// Decimals lookup with API-first strategy; chain fallback will be added later.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::tokens_new::provider::TokenDataProvider;
use crate::tokens_new::types::DataSource;

// Simple in-memory cache (TTL can be layered later via tokens_new/cache)
static DECIMALS_CACHE: std::sync::LazyLock<Arc<RwLock<HashMap<String, u8>>>> =
    std::sync::LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

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

    // 1) Try provider metadata (DB) first
    if let Ok(Some(meta)) = provider.get_token_metadata(mint) {
        if let Some(d) = meta.decimals {
            if let Ok(mut w) = DECIMALS_CACHE.write() {
                w.insert(mint.to_string(), d);
            }
            return Ok(d);
        }
    }

    // 2) Fetch Rugcheck only to get decimals if available
    if let Ok(result) = provider
        .fetch_complete_data(
            mint,
            Some(crate::tokens_new::provider::types::FetchOptions {
                sources: vec![DataSource::Rugcheck],
                ..Default::default()
            }),
        )
        .await
    {
        if let Some(r) = result.rugcheck_info.as_ref() {
            if let Some(d) = r.token_decimals {
                if let Ok(mut w) = DECIMALS_CACHE.write() {
                    w.insert(mint.to_string(), d);
                }
                return Ok(d);
            }
        }
    }

    // 3) Chain fallback via existing tokens::decimals helper
    match crate::tokens::decimals::get_token_decimals_from_chain(mint).await {
        Ok(d) => {
            if let Ok(mut w) = DECIMALS_CACHE.write() {
                w.insert(mint.to_string(), d);
            }
            Ok(d)
        }
        Err(e) => Err(e),
    }
}
