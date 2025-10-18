// tokens_new/discovery.rs
// Token discovery from multiple sources (skeleton)

use std::collections::HashSet;

use chrono::Utc;

use crate::tokens_new::events::{emit, TokenEvent};
use crate::tokens_new::provider::TokenDataProvider;
use crate::tokens_new::store::upsert_snapshot;
use crate::tokens_new::store::Snapshot;

pub async fn discover_from_sources(provider: &TokenDataProvider) -> Result<Vec<String>, String> {
    // Placeholder: will call api clients for new tokens. Return empty for now.
    Ok(Vec::new())
}

pub async fn process_new_mints(provider: &TokenDataProvider, mints: Vec<String>) {
    let mut seen = HashSet::new();
    for mint in mints {
        if !seen.insert(mint.clone()) {
            continue;
        }
        emit(TokenEvent::TokenDiscovered { mint: mint.clone(), source: "unknown".into(), at: Utc::now() });
        // Create minimal snapshot to unblock downstream
        upsert_snapshot(Snapshot { mint: mint.clone(), updated_at: Utc::now(), ..Default::default() });
        // Kick off metadata/pools in higher layers later
        let _ = provider.fetch_complete_data(&mint, None).await;
    }
}
