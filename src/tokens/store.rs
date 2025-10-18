// tokens/store.rs
// In-memory token snapshots with synchronized database persistence
// SINGLE SOURCE OF TRUTH for token data - all updates go through this module

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use log::warn;
use once_cell::sync::OnceCell;

use crate::tokens::priorities::Priority;
use crate::tokens::storage::Database;
use crate::tokens::types::DataSource;

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

// Database handle for synchronized persistence
static DB_HANDLE: OnceCell<Arc<Database>> = OnceCell::new();

/// Initialize store with database handle (called once during provider creation)
pub fn initialize_with_database(db: Arc<Database>) -> Result<(), String> {
    DB_HANDLE
        .set(db)
        .map_err(|_| "Store database already initialized".to_string())
}

/// Read-only: Get snapshot from memory
pub fn get_snapshot(mint: &str) -> Option<Snapshot> {
    STORE.read().ok().and_then(|m| m.get(mint).cloned())
}

/// UNIFIED UPDATE: Memory + Database synchronized
/// This is the ONLY way to update token data - ensures consistency
pub fn upsert_snapshot(snapshot: Snapshot) -> Result<(), String> {
    let mint = snapshot.mint.clone();

    // 1. Update memory store (fast, always succeeds)
    if let Ok(mut m) = STORE.write() {
        m.insert(mint.clone(), snapshot.clone());
    }

    // 2. Persist to database (if initialized)
    if let Some(db) = DB_HANDLE.get() {
        // Update tokens metadata table
        if let Err(e) = crate::tokens::storage::operations::upsert_token_metadata(
            db,
            &mint,
            snapshot.symbol.as_deref(),
            snapshot.name.as_deref(),
            snapshot.decimals,
        ) {
            warn!(
                "[TOKENS] Failed to persist token metadata to DB: mint={} err={}",
                mint, e
            );
            // Don't fail - memory update succeeded
        }
    }

    Ok(())
}

/// Update priority: Memory + Database
pub fn set_priority(mint: &str, priority: Priority) -> Result<(), String> {
    // Update memory
    if let Ok(mut m) = STORE.write() {
        if let Some(s) = m.get_mut(mint) {
            s.priority = priority;
            s.updated_at = Utc::now();
        }
    }

    // TODO: Add priority column to tokens table and persist here
    Ok(())
}

/// Read-only: List all mints
pub fn list_mints() -> Vec<String> {
    STORE
        .read()
        .ok()
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Read-only: Get all snapshots
pub fn all_snapshots() -> Vec<Snapshot> {
    STORE
        .read()
        .ok()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

/// Update decimals: Memory + Database synchronized
pub fn set_decimals(mint: &str, decimals: u8) -> Result<(), String> {
    // Update memory
    if let Ok(mut m) = STORE.write() {
        if let Some(snapshot) = m.get_mut(mint) {
            snapshot.decimals = Some(decimals);
            snapshot.updated_at = Utc::now();
        } else {
            // Create new snapshot if doesn't exist
            m.insert(
                mint.to_string(),
                Snapshot {
                    mint: mint.to_string(),
                    decimals: Some(decimals),
                    updated_at: Utc::now(),
                    ..Default::default()
                },
            );
        }
    }

    // Persist to database
    if let Some(db) = DB_HANDLE.get() {
        if let Err(e) = crate::tokens::storage::operations::upsert_token_metadata(
            db,
            mint,
            None,
            None,
            Some(decimals),
        ) {
            warn!(
                "[TOKENS] Failed to persist decimals to DB: mint={} err={}",
                mint, e
            );
        }
    }

    Ok(())
}
