//! Decision cache for retry management

use crate::trader::types::TradeDecision;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

static DECISION_CACHE: OnceLock<RwLock<HashMap<String, CachedDecision>>> = OnceLock::new();

fn get_decision_cache() -> &'static RwLock<HashMap<String, CachedDecision>> {
    DECISION_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[derive(Clone)]
struct CachedDecision {
    decision: TradeDecision,
    cached_at: DateTime<Utc>,
    retry_count: u32,
}

/// Initialize the decision cache
pub fn init_cache() -> Result<(), String> {
    // Cache is initialized via OnceLock
    Ok(())
}

/// Cache a sell decision for retry
pub async fn cache_sell_decision(decision: &TradeDecision) -> Result<(), String> {
    let position_id = decision
        .position_id
        .as_ref()
        .ok_or("Cannot cache decision without position_id")?;

    let mut cache = get_decision_cache().write().await;
    cache.insert(
        position_id.clone(),
        CachedDecision {
            decision: decision.clone(),
            cached_at: Utc::now(),
            retry_count: 0,
        },
    );

    Ok(())
}

/// Get decisions ready for retry (older than 5 minutes)
pub async fn get_pending_sell_decisions() -> Vec<TradeDecision> {
    let cache = get_decision_cache().read().await;
    let cutoff_time = Utc::now() - Duration::minutes(5);

    cache
        .values()
        .filter(|cached| cached.cached_at <= cutoff_time && cached.retry_count < 5)
        .map(|cached| cached.decision.clone())
        .collect()
}

/// Mark a decision as complete (remove from cache)
pub async fn mark_sell_complete(position_id: &str) -> bool {
    let mut cache = get_decision_cache().write().await;
    cache.remove(position_id).is_some()
}

/// Increment retry count for a decision
pub async fn increment_retry_count(position_id: &str) -> Result<(), String> {
    let mut cache = get_decision_cache().write().await;
    if let Some(cached) = cache.get_mut(position_id) {
        cached.retry_count += 1;
        Ok(())
    } else {
        Err(format!("Decision not found in cache: {}", position_id))
    }
}

/// Clean up old cached decisions (older than 24 hours)
pub async fn cleanup_old_decisions() {
    let mut cache = get_decision_cache().write().await;
    let cutoff_time = Utc::now() - Duration::hours(24);

    cache.retain(|_, cached| cached.cached_at > cutoff_time);
}
