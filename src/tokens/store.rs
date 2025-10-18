// tokens/store.rs
// In-memory token store with synchronized database persistence
// SINGLE SOURCE OF TRUTH for token data - all updates go through this module

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::Utc;
use log::warn;
use once_cell::sync::OnceCell;

use crate::tokens::priorities::Priority;
use crate::tokens::storage::Database;
use crate::tokens::types::{DataSource, Token};

/// Global token store - holds full Token objects
static STORE: std::sync::LazyLock<RwLock<HashMap<String, Token>>> =
    std::sync::LazyLock::new(|| RwLock::new(HashMap::new()));

// Database handle for synchronized persistence
static DB_HANDLE: OnceCell<Arc<Database>> = OnceCell::new();

/// Initialize store with database handle (called once during provider creation)
pub fn initialize_with_database(db: Arc<Database>) -> Result<(), String> {
    DB_HANDLE
        .set(db)
        .map_err(|_| "Store database already initialized".to_string())
}

/// Hydrate store from tokens (used during startup, skips DB write)
pub fn hydrate_from_tokens(tokens: Vec<Token>) -> Result<(), String> {
    if let Ok(mut m) = STORE.write() {
        for token in tokens {
            m.insert(token.mint.clone(), token);
        }
        Ok(())
    } else {
        Err("Failed to acquire write lock for hydration".to_string())
    }
}

/// Read-only: Get token from memory
pub fn get_token(mint: &str) -> Option<Token> {
    STORE.read().ok().and_then(|m| m.get(mint).cloned())
}

/// UNIFIED UPDATE: Memory + Database synchronized
/// This is the ONLY way to update token data - ensures consistency
pub fn upsert_token(token: Token) -> Result<(), String> {
    let mint = token.mint.clone();

    // 1. Update memory store (fast, always succeeds)
    if let Ok(mut m) = STORE.write() {
        m.insert(mint.clone(), token.clone());
    }

    // 2. Persist to database (if initialized)
    if let Some(db) = DB_HANDLE.get() {
        // Update tokens metadata table
        if let Err(e) = crate::tokens::storage::operations::upsert_token_metadata(
            db,
            &mint,
            Some(&token.symbol),
            Some(&token.name),
            Some(token.decimals),
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
        if let Some(token) = m.get_mut(mint) {
            token.priority = priority;
            token.updated_at = Utc::now();
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

/// Read-only: Get all tokens
pub fn all_tokens() -> Vec<Token> {
    STORE
        .read()
        .ok()
        .map(|m| m.values().cloned().collect())
        .unwrap_or_default()
}

/// Read-only: Get tokens by priority
pub fn get_by_priority(priority: Priority) -> Vec<Token> {
    STORE
        .read()
        .ok()
        .map(|m| {
            m.values()
                .filter(|t| t.priority == priority)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Read-only: Get tokens by data source
pub fn get_by_source(source: DataSource) -> Vec<Token> {
    STORE
        .read()
        .ok()
        .map(|m| {
            m.values()
                .filter(|t| t.data_source == source)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Read-only: Search tokens by symbol or name
pub fn search_tokens(query: &str) -> Vec<Token> {
    let query_lower = query.to_lowercase();
    STORE
        .read()
        .ok()
        .map(|m| {
            m.values()
                .filter(|t| {
                    t.symbol.to_lowercase().contains(&query_lower)
                        || t.name.to_lowercase().contains(&query_lower)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Read-only: Get recently updated tokens
pub fn get_recently_updated(limit: usize) -> Vec<Token> {
    STORE
        .read()
        .ok()
        .map(|m| {
            let mut tokens: Vec<_> = m.values().cloned().collect();
            tokens.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            tokens.truncate(limit);
            tokens
        })
        .unwrap_or_default()
}

/// Read-only: Count total tokens
pub fn count_tokens() -> usize {
    STORE.read().ok().map(|m| m.len()).unwrap_or(0)
}

/// Read-only: Filter blacklisted tokens
pub fn filter_blacklisted(include_blacklisted: bool) -> Vec<Token> {
    STORE
        .read()
        .ok()
        .map(|m| {
            m.values()
                .filter(|t| t.is_blacklisted == include_blacklisted)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Read-only: Get tokens with minimum liquidity
pub fn get_by_min_liquidity(min_liquidity_usd: f64) -> Vec<Token> {
    STORE
        .read()
        .ok()
        .map(|m| {
            m.values()
                .filter(|t| {
                    t.liquidity_usd
                        .map(|liq| liq >= min_liquidity_usd)
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// Update decimals: Memory + Database synchronized
pub fn set_decimals(mint: &str, decimals: u8) -> Result<(), String> {
    // Update memory
    if let Ok(mut m) = STORE.write() {
        if let Some(token) = m.get_mut(mint) {
            token.decimals = decimals;
            token.updated_at = Utc::now();
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

/// Check if token exists in store
pub fn token_exists(mint: &str) -> bool {
    STORE
        .read()
        .ok()
        .map(|m| m.contains_key(mint))
        .unwrap_or(false)
}
