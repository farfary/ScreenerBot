// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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

use std::collections::HashSet;
use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::chains::{ChainId, PerChain};
use crate::logger::{self, LogTag};

/// Authority data extracted from a token's account (zero extra RPC cost)
#[derive(Clone, Debug)]
pub struct MintAuthorities {
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
}

// In-memory cache per chain — mint address → authorities
// Bounded to 100K entries per chain (same as the decimals cache)
static AUTHORITIES_CACHE: PerChain<moka::sync::Cache<String, MintAuthorities>> =
    PerChain::new(new_authorities_cache);

fn new_authorities_cache(_chain: ChainId) -> moka::sync::Cache<String, MintAuthorities> {
    moka::sync::Cache::builder().max_capacity(100_000).build()
}

// Blocked authorities set per chain — addresses confirmed as scam factories
// Loaded from DB on startup, refreshed periodically by background task
// Uses ArcSwap for atomic replacement (no race condition during refresh); a
// refresh replaces only its own chain's set
static BLOCKED_AUTHORITIES: PerChain<ArcSwap<HashSet<String>>> =
    PerChain::new(new_blocked_authorities);

fn new_blocked_authorities(_chain: ChainId) -> ArcSwap<HashSet<String>> {
    ArcSwap::from_pointee(HashSet::new())
}

// ============================================================================
// PUBLIC API — FILTERING (hot path, sync)
// ============================================================================

/// Check if an authority address is in the blocked set. O(1), no DB/RPC calls.
pub fn is_blocked_authority(chain: ChainId, address: &str) -> bool {
    BLOCKED_AUTHORITIES.get(chain).load().contains(address)
}

/// Get cached authorities for a mint (sync, instant)
pub fn get_cached(chain: ChainId, mint: &str) -> Option<MintAuthorities> {
    AUTHORITIES_CACHE.get(chain).get(mint)
}

// ============================================================================
// CACHE POPULATION — called by decimals.rs during chain fetch
// ============================================================================

/// Cache authorities extracted from SPL Mint during decimals fetch.
/// This is called as a side effect — zero extra RPC cost.
pub fn cache_mint_authorities(chain: ChainId, mint: &str, authorities: MintAuthorities) {
    AUTHORITIES_CACHE
        .get(chain)
        .insert(mint.to_owned(), authorities);
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

/// Refresh `chain`'s in-memory blocked set from the database.
/// Called on startup and periodically by the background discovery task.
/// Uses atomic swap — no race condition during refresh — and replaces only
/// this chain's set.
pub fn refresh_blocked_from_db(chain: ChainId, blocked_addresses: Vec<String>) {
    let count = blocked_addresses.len();
    let new_set: HashSet<String> = blocked_addresses.into_iter().collect();
    BLOCKED_AUTHORITIES.get(chain).store(Arc::new(new_set));
    logger::info(
        LogTag::Filtering,
        &format!("Authority reputation refreshed: {count} blocked authorities loaded"),
    );
}

/// Clear all caches on every chain (for testing/reset)
pub fn clear_cache() {
    for (_, cache) in AUTHORITIES_CACHE.built() {
        cache.invalidate_all();
    }
    for (_, blocked) in BLOCKED_AUTHORITIES.built() {
        blocked.store(Arc::new(HashSet::new()));
    }
}
