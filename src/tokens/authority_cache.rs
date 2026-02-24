//! Token authority cache — in-memory cache for mint/freeze authority lookups.

// tokens/authority_cache.rs
// Lightweight cache for token mint authorities (freeze, mint, update)
//
// ARCHITECTURE:
// - Populated as a side effect of decimals.rs chain fetches (zero extra RPC cost)
// - In-memory moka cache for fast sync lookups during filtering
// - DB persistence in `authority_reputation` table for auto-discovery
//
// This module does NOT fetch from chain itself — it relies on decimals.rs
// calling `cache_mint_authorities()` when it unpacks SPL Mint data.

use std::sync::LazyLock;

use crate::logger::{self, LogTag};

/// Authority data extracted from SPL Mint account (zero extra RPC cost)
#[derive(Clone, Debug)]
pub struct MintAuthorities {
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub supply: u64,
}

// In-memory cache — mint address → authorities
// Bounded to 100K entries (same as decimals cache)
static AUTHORITIES_CACHE: LazyLock<moka::sync::Cache<String, MintAuthorities>> =
    LazyLock::new(|| moka::sync::Cache::builder().max_capacity(100_000).build());

// Blocked authorities set — addresses confirmed as scam factories
// Loaded from DB on startup, refreshed periodically by background task
// Uses ArcSwap for atomic replacement (no race condition during refresh)
static BLOCKED_AUTHORITIES: LazyLock<arc_swap::ArcSwap<dashmap::DashSet<String>>> =
    LazyLock::new(|| arc_swap::ArcSwap::from_pointee(dashmap::DashSet::new()));

// ============================================================================
// PUBLIC API — FILTERING (hot path, sync)
// ============================================================================

/// Check if an authority address is in the blocked set. O(1), no DB/RPC calls.
pub fn is_blocked_authority(address: &str) -> bool {
    BLOCKED_AUTHORITIES.load().contains(address)
}

/// Get cached authorities for a mint (sync, instant)
pub fn get_cached(mint: &str) -> Option<MintAuthorities> {
    AUTHORITIES_CACHE.get(mint)
}

// ============================================================================
// CACHE POPULATION — called by decimals.rs during chain fetch
// ============================================================================

/// Cache authorities extracted from SPL Mint during decimals fetch.
/// This is called as a side effect — zero extra RPC cost.
pub fn cache_mint_authorities(mint: &str, authorities: MintAuthorities) {
    AUTHORITIES_CACHE.insert(mint.to_string(), authorities);
}

// ============================================================================
// AUTHORITY REPUTATION — auto-discovery system
// ============================================================================

/// Authority reputation record from the database
#[derive(Clone, Debug)]
pub struct AuthorityReputation {
    pub address: String,
    pub total_token_count: u32,
    pub flagged_token_count: u32,
    pub confidence: f64,
    pub is_blocked: bool,
}

/// Refresh the in-memory blocked set from the database.
/// Called on startup and periodically by the background discovery task.
/// Uses atomic swap — no race condition during refresh.
pub fn refresh_blocked_from_db(blocked_addresses: Vec<String>) {
    let new_set = dashmap::DashSet::new();
    for addr in &blocked_addresses {
        new_set.insert(addr.clone());
    }
    BLOCKED_AUTHORITIES.store(std::sync::Arc::new(new_set));
    logger::info(
        LogTag::Filtering,
        &format!(
            "Authority reputation refreshed: {} blocked authorities loaded",
            blocked_addresses.len()
        ),
    );
}

/// Clear all caches (for testing/reset)
pub fn clear_cache() {
    AUTHORITIES_CACHE.invalidate_all();
    BLOCKED_AUTHORITIES.store(std::sync::Arc::new(dashmap::DashSet::new()));
}
