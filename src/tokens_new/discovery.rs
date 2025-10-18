// tokens_new/discovery.rs
// Token discovery from multiple sources (skeleton)

use std::collections::{HashMap, HashSet};

use chrono::Utc;

use crate::tokens_new::events::{emit, TokenEvent};
use crate::tokens_new::provider::TokenDataProvider;
use crate::tokens_new::store::upsert_snapshot;
use crate::tokens_new::store::Snapshot;
use crate::tokens_new::blacklist;

/// Collect candidate mints from multiple discovery sources.
/// Returns unique mint list.
pub async fn discover_from_sources(provider: &TokenDataProvider) -> Result<Vec<String>, String> {
    let api = provider.api();

    let mut candidates: Vec<(String, String)> = Vec::new(); // (mint, source)

    // DexScreener latest boosted tokens (often new/promoted tokens)
    if let Ok(list) = api.dexscreener.get_latest_boosted_tokens().await {
        for item in list.into_iter().filter(|b| b.chain_id.to_lowercase() == "solana") {
            candidates.push((item.token_address, "dexscreener.boosts_latest".into()));
        }
    }

    // GeckoTerminal new pools (Solana)
    if let Ok(pools) = api.geckoterminal.fetch_new_pools_by_network("solana", Some("base_token,quote_token,dex"), Some(1)).await {
        for p in pools {
            // Prefer base token id; if empty, skip
            if !p.base_token_id.is_empty() {
                candidates.push((p.base_token_id, "gecko.new_pools".into()));
            }
        }
    }

    // GeckoTerminal recently updated tokens (filter solana)
    if let Ok(recent) = api.geckoterminal.fetch_recently_updated_tokens(Some("network"), Some("solana")).await {
        for t in recent.data {
            // attributes.address is the token address
            let addr = t.attributes.address;
            if !addr.is_empty() {
                candidates.push((addr, "gecko.recently_updated".into()));
            }
        }
    }

    // Deduplicate, filter blacklist and empties
    let mut seen: HashMap<String, String> = HashMap::new();
    for (mint, src) in candidates {
        if mint.is_empty() || blacklist::is(&mint) { continue; }
        seen.entry(mint).or_insert(src);
    }

    Ok(seen.into_keys().collect())
}

pub async fn process_new_mints(provider: &TokenDataProvider, mints: Vec<String>) {
    let mut seen = HashSet::new();
    for mint in mints {
        if !seen.insert(mint.clone()) {
            continue;
        }
        emit(TokenEvent::TokenDiscovered { mint: mint.clone(), source: "discovery".into(), at: Utc::now() });
        // Create minimal snapshot to unblock downstream
        upsert_snapshot(Snapshot { mint: mint.clone(), updated_at: Utc::now(), ..Default::default() });
        // Kick off metadata/pools in higher layers later
        let _ = provider.fetch_complete_data(&mint, None).await;
    }
}
