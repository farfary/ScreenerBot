//! Pending swap state — tracks in-flight partial exits and DCA swaps with persistence.

use super::db;
pub use super::types::{PendingDcaSwap, PendingPartialExit};
use crate::logger::{self, LogTag};
use std::{collections::HashMap, sync::LazyLock};
use tokio::sync::RwLock;

// Pending partial exits registry (mint -> count of pending partial exits)
// We serialize to a single pending at a time, but using a count keeps API flexible
static PENDING_PARTIAL_EXITS: LazyLock<RwLock<HashMap<String, u32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static PENDING_PARTIAL_EXIT_DETAILS: LazyLock<RwLock<HashMap<String, PendingPartialExit>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
const PENDING_PARTIAL_EXIT_METADATA_KEY: &str = "pending_partial_exits";

// Pending DCA swaps registry: ensures DCA verifications survive restarts and duplicate submissions
static PENDING_DCA_SWAPS: LazyLock<RwLock<HashMap<String, PendingDcaSwap>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

const PENDING_DCA_METADATA_KEY: &str = "pending_dca_swaps";

/// Mark that a partial exit is pending for a mint (increments count)
pub async fn mark_partial_exit_pending(mint: &str) {
    let mut map = PENDING_PARTIAL_EXITS.write().await;
    let counter = map.entry(mint.to_string()).or_default();
    *counter = counter.saturating_add(1);
}

/// Clear pending mark for a partial exit for a mint (decrements count and removes if zero)
pub async fn clear_partial_exit_pending(mint: &str) {
    let mut map = PENDING_PARTIAL_EXITS.write().await;
    if let Some(counter) = map.get_mut(mint) {
        if *counter > 1 {
            *counter -= 1;
        } else {
            map.remove(mint);
        }
    }
}

/// Persist current pending DCA map to the database metadata store
async fn persist_pending_dca_swaps() -> Result<(), String> {
    let pending: Vec<PendingDcaSwap> = {
        let map = PENDING_DCA_SWAPS.read().await;
        map.values().cloned().collect()
    };

    let serialized = serde_json::to_string(&pending)
        .map_err(|e| format!("Failed to serialize pending DCA swaps: {e}"))?;

    db::set_metadata(PENDING_DCA_METADATA_KEY, &serialized).await
}

/// Register a pending DCA swap for durability
pub async fn register_pending_dca_swap(entry: PendingDcaSwap) -> Result<(), String> {
    let signature = entry.signature.clone();
    {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.insert(signature.clone(), entry);
    }

    if let Err(err) = persist_pending_dca_swaps().await {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.remove(&signature);
        return Err(err);
    }

    Ok(())
}

/// Clear a pending DCA swap once processed
pub async fn clear_pending_dca_swap(signature: &str) -> Result<Option<PendingDcaSwap>, String> {
    let removed = {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.remove(signature)
    };

    if let Some(entry) = removed.clone() {
        if let Err(err) = persist_pending_dca_swaps().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending DCA metadata after clearing {}: {}",
                    signature, err
                ),
            );
            // Reinsert to keep in-memory state consistent if persistence fails
            {
                let mut map = PENDING_DCA_SWAPS.write().await;
                map.insert(entry.signature.clone(), entry);
            }
            return Err(err);
        }
    }

    Ok(removed)
}

/// Load pending DCA swaps from metadata into memory (used at startup)
pub async fn rehydrate_pending_dca_swaps() -> Result<Vec<PendingDcaSwap>, String> {
    let raw = db::get_metadata(PENDING_DCA_METADATA_KEY).await?;

    let entries: Vec<PendingDcaSwap> = match raw {
        Some(payload) if !payload.is_empty() => serde_json::from_str(&payload)
            .map_err(|e| format!("Failed to deserialize pending DCA metadata payload: {e}"))?,
        _ => Vec::new(),
    };

    {
        let mut map = PENDING_DCA_SWAPS.write().await;
        map.clear();
        for entry in &entries {
            map.insert(entry.signature.clone(), entry.clone());
        }
    }

    Ok(entries)
}

async fn persist_pending_partial_exits() -> Result<(), String> {
    let pending: Vec<PendingPartialExit> = {
        let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
        map.values().cloned().collect()
    };

    let serialized = serde_json::to_string(&pending)
        .map_err(|e| format!("Failed to serialize pending partial exits: {e}"))?;

    db::set_metadata(PENDING_PARTIAL_EXIT_METADATA_KEY, &serialized).await
}

/// Register a pending partial exit for durability
pub async fn register_pending_partial_exit(entry: PendingPartialExit) -> Result<(), String> {
    let signature = entry.signature.clone();
    {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.insert(signature.clone(), entry);
    }

    if let Err(err) = persist_pending_partial_exits().await {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.remove(&signature);
        return Err(err);
    }

    Ok(())
}

/// Clear a pending partial exit once processed
pub async fn clear_pending_partial_exit(
    signature: &str,
) -> Result<Option<PendingPartialExit>, String> {
    let removed = {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.remove(signature)
    };

    if let Some(entry) = removed.clone() {
        if let Err(err) = persist_pending_partial_exits().await {
            logger::error(
                LogTag::Positions,
                &format!(
                    "Failed to persist pending partial exit metadata after clearing {}: {}",
                    signature, err
                ),
            );

            let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
            map.insert(entry.signature.clone(), entry);
            return Err(err);
        }
    }

    Ok(removed)
}

/// Fetch a pending partial exit by signature
pub async fn get_pending_partial_exit(signature: &str) -> Option<PendingPartialExit> {
    let map = PENDING_PARTIAL_EXIT_DETAILS.read().await;
    map.get(signature).cloned()
}

/// Load pending partial exits from metadata into memory (used at startup)
pub async fn rehydrate_pending_partial_exits() -> Result<Vec<PendingPartialExit>, String> {
    let raw = db::get_metadata(PENDING_PARTIAL_EXIT_METADATA_KEY).await?;

    let entries: Vec<PendingPartialExit> = match raw {
        Some(payload) if !payload.is_empty() => serde_json::from_str(&payload)
            .map_err(|e| format!("Failed to deserialize pending partial exit payload: {e}"))?,
        _ => Vec::new(),
    };

    {
        let mut map = PENDING_PARTIAL_EXIT_DETAILS.write().await;
        map.clear();
        for entry in &entries {
            map.insert(entry.signature.clone(), entry.clone());
        }
    }

    {
        let mut counters = PENDING_PARTIAL_EXITS.write().await;
        counters.clear();
        for entry in &entries {
            let counter = counters.entry(entry.mint.clone()).or_default();
            *counter = counter.saturating_add(1);
        }
    }

    Ok(entries)
}
