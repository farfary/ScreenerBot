// tokens_new/store.rs
// In-memory token snapshots for fast access by other modules

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};

use crate::tokens_new::priorities::Priority;
use crate::tokens_new::types::DataSource;

#[derive(Debug, Clone, Default)]
pub struct BestPoolSummary {
    pub program_id: Option<String>,
    pub pool_address: Option<String>,
    pub dex: Option<String>,
    pub liquidity_sol: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub is_blacklisted: bool,
    pub best_pool: Option<BestPoolSummary>,
    pub sources: Vec<DataSource>,
    pub priority: Priority,
    pub fetched_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

static STORE: std::sync::LazyLock<RwLock<HashMap<String, Snapshot>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn get_snapshot(mint: &str) -> Option<Snapshot> {
    STORE.read().ok().and_then(|m| m.get(mint).cloned())
}

pub fn upsert_snapshot(snapshot: Snapshot) {
    if let Ok(mut m) = STORE.write() {
        m.insert(snapshot.mint.clone(), snapshot);
    }
}

pub fn set_priority(mint: &str, priority: Priority) {
    if let Ok(mut m) = STORE.write() {
        if let Some(s) = m.get_mut(mint) {
            s.priority = priority;
            s.updated_at = Utc::now();
        }
    }
}

pub fn list_mints() -> Vec<String> {
    STORE
        .read()
        .ok()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

pub fn all_snapshots() -> Vec<Snapshot> {
    STORE
        .read()
        .ok()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}
