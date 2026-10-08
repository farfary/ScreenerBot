// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token search functionality using DexScreener and GeckoTerminal APIs
//!
//! Provides unified search across multiple data sources with deduplication.
//! DexScreener supports direct search, GeckoTerminal requires mint-based lookup.
//! A token found in several pools is represented by its busiest pool, and the
//! results are ranked by relevance to the query, then by 24h volume.
//!
//! When tokens are found via external APIs, they are automatically added to the
//! local database for future lookups and analysis.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::apis::dexscreener::DexScreenerPool;
use crate::apis::get_api_manager;
use crate::chains::{adapter_for, ChainId};
use crate::logger::{self, LogTag};
use crate::tokens::database::database;

// =============================================================================
// SEARCH TYPES
// =============================================================================

/// Single token search result with unified fields from any source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSearchResult {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub logo_url: Option<String>,
    pub price_usd: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity_usd: Option<f64>,
    /// Creation time of the pool the row was read from, in Unix milliseconds.
    pub pair_created_at: Option<i64>,
    pub source: String,
}

/// Aggregated search results from all sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub results: Vec<TokenSearchResult>,
    pub query: String,
    pub total: usize,
}

// =============================================================================
// DATABASE PERSISTENCE
// =============================================================================

/// Persist a token search result to the database
///
/// This ensures tokens discovered via search are available for future lookups
/// and can be used by the token analyzer.
/// Returns true if token was successfully persisted, false otherwise.
async fn persist_token_to_database(chain: ChainId, result: &TokenSearchResult) -> bool {
    let db = match database(chain) {
        Some(db) => db,
        None => return false,
    };

    let mint = result.mint.clone();
    let symbol = if !result.symbol.is_empty() {
        Some(result.symbol.clone())
    } else {
        None
    };
    let name = if !result.name.is_empty() {
        Some(result.name.clone())
    } else {
        None
    };

    // Wrap blocking DB call in spawn_blocking
    let persist_result = tokio::task::spawn_blocking(move || {
        db.upsert_token(&mint, symbol.as_deref(), name.as_deref(), None)
    })
    .await;

    match persist_result {
        Ok(Ok(())) => {
            logger::debug(
                LogTag::Tokens,
                &format!(
                    "[SEARCH] Token persisted to DB: mint={} symbol={:?} name={:?}",
                    result.mint,
                    if !result.symbol.is_empty() {
                        Some(&result.symbol)
                    } else {
                        None
                    },
                    if !result.name.is_empty() {
                        Some(&result.name)
                    } else {
                        None
                    }
                ),
            );
            true
        }
        Ok(Err(e)) => {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[SEARCH] Failed to persist token to DB: mint={} error={}",
                    result.mint, e
                ),
            );
            false
        }
        Err(e) => {
            logger::warning(
                LogTag::Tokens,
                &format!(
                    "[SEARCH] spawn_blocking failed for token persist: mint={} error={}",
                    result.mint, e
                ),
            );
            false
        }
    }
}

// =============================================================================
// POOL SELECTION AND RANKING
// =============================================================================

/// One search row read from one DexScreener pool.
fn result_from_pool(pool: &DexScreenerPool) -> TokenSearchResult {
    TokenSearchResult {
        mint: pool.mint.clone(),
        name: pool.base_token_name.clone(),
        symbol: pool.base_token_symbol.clone(),
        logo_url: pool.info_image_url.clone(),
        price_usd: pool.price_usd.parse().ok(),
        price_change_h24: pool.price_change_h24,
        market_cap: pool.market_cap,
        fdv: pool.fdv,
        volume_24h: pool.volume_h24,
        liquidity_usd: pool.liquidity_usd,
        pair_created_at: pool.pair_created_at,
        source: "dexscreener".to_owned(),
    }
}

/// Keep one row per token: the row from its busiest pool. Price and change are
/// read from the pool most of the token's trading happens in, never from
/// whichever pool a provider listed first.
fn keep_busiest(rows: &mut HashMap<String, TokenSearchResult>, row: TokenSearchResult) {
    match rows.get(&row.mint) {
        Some(kept) if activity_order(kept, &row) != Ordering::Greater => {}
        _ => {
            rows.insert(row.mint.clone(), row);
        }
    }
}

/// The query as matching compares it: trimmed, case-folded, and without the
/// `$` a ticker is often written with.
fn normalize_query(query: &str) -> String {
    query.trim().trim_start_matches('$').to_lowercase()
}

/// The relevance tier of a row to a normalized query; lower is closer. An exact
/// symbol, then a symbol prefix, then an exact name, then a name prefix, then
/// any other match (an address or a provider's fuzzy hit).
fn match_tier(query: &str, row: &TokenSearchResult) -> u8 {
    if query.is_empty() {
        return 0;
    }
    let symbol = row.symbol.to_lowercase();
    let name = row.name.to_lowercase();
    if symbol == query {
        0
    } else if symbol.starts_with(query) {
        1
    } else if name == query {
        2
    } else if name.starts_with(query) {
        3
    } else {
        4
    }
}

/// The 24h volume of a token that traded; zero or unknown volume is no trading.
fn traded_volume(volume: Option<f64>) -> Option<f64> {
    volume.filter(|volume| *volume > 0.0)
}

/// Larger first; an unknown value after every known one.
fn larger_first(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => b.partial_cmp(&a).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Traded volume first, liquidity breaking ties. Liquidity alone is never the
/// order: a pool can report liquidity nothing trades against, and ranking by it
/// put untraded look-alike tokens above the token a query names.
fn activity_order(a: &TokenSearchResult, b: &TokenSearchResult) -> Ordering {
    larger_first(traded_volume(a.volume_24h), traded_volume(b.volume_24h))
        .then_with(|| larger_first(a.liquidity_usd, b.liquidity_usd))
}

/// Order rows for `query` (raw, as typed): relevance tier, then activity, the
/// mint breaking ties so equal rows keep a stable order.
fn rank_search_results(query: &str, rows: &mut [TokenSearchResult]) {
    let query = normalize_query(query);
    rows.sort_by(|a, b| {
        match_tier(&query, a)
            .cmp(&match_tier(&query, b))
            .then_with(|| activity_order(a, b))
            .then_with(|| a.mint.cmp(&b.mint))
    });
}

// =============================================================================
// SEARCH IMPLEMENTATION
// =============================================================================

/// Search for tokens across available data sources
///
/// Strategy:
/// - If query looks like a mint address, fetch token data directly by mint
/// - Otherwise, use DexScreener's search endpoint
/// - Deduplicate results by mint address, preferring DexScreener data and each
///   token's busiest pool
/// - Rank the whole candidate set, then keep the first `limit` rows
pub async fn search_tokens(
    chain: ChainId,
    query: &str,
    limit: Option<usize>,
) -> crate::tokens::Result<SearchResults> {
    let query = query.trim();
    if query.is_empty() {
        return Err(crate::tokens::Error::InvalidSearchQuery {
            reason: "the query is empty".to_owned(),
        });
    }

    let max_results = limit.unwrap_or(20).min(50);

    logger::debug(
        LogTag::Api,
        &format!("Token search: query='{query}', limit={max_results}"),
    );

    let apis = get_api_manager();

    // Collect results from sources
    let mut results_map: HashMap<String, TokenSearchResult> = HashMap::new();

    // If it looks like a mint address, do direct lookups
    if adapter_for(chain).looks_like_address(query) {
        logger::debug(
            LogTag::Api,
            &format!(
                "Query '{}' looks like mint address, fetching directly",
                query
            ),
        );

        // Try DexScreener first. The token-pairs list also holds pools where the
        // token is the quote side, so only pools that list it as the base count.
        if apis.dexscreener.is_enabled() {
            match apis.dexscreener.fetch_token_pools(query, None).await {
                Ok(pools) => {
                    let base_pools: Vec<&DexScreenerPool> =
                        pools.iter().filter(|pool| pool.mint == query).collect();
                    let candidates = if base_pools.is_empty() {
                        pools.iter().take(1).collect()
                    } else {
                        base_pools
                    };
                    for pool in candidates {
                        keep_busiest(&mut results_map, result_from_pool(pool));
                    }
                }
                Err(e) => {
                    logger::debug(LogTag::Api, &format!("DexScreener mint lookup failed: {e}"));
                }
            }
        }

        // Try GeckoTerminal as fallback/supplement
        if apis.geckoterminal.is_enabled() {
            match apis.geckoterminal.fetch_pools(query).await {
                Ok(pools) => {
                    if let Some(pool) = pools.first() {
                        // Only add if not already found via DexScreener
                        if !results_map.contains_key(&pool.mint) {
                            let result = TokenSearchResult {
                                mint: pool.mint.clone(),
                                name: pool.pool_name.clone(),
                                symbol: pool
                                    .pool_name
                                    .split('/')
                                    .next()
                                    .unwrap_or_default()
                                    .to_string(),
                                logo_url: None,
                                price_usd: pool.token_price_usd.parse().ok(),
                                price_change_h24: None,
                                market_cap: pool.market_cap_usd,
                                fdv: None,
                                volume_24h: pool.volume_h24,
                                liquidity_usd: pool.reserve_usd,
                                pair_created_at: None,
                                source: "geckoterminal".to_owned(),
                            };
                            results_map.insert(result.mint.clone(), result);
                        }
                    }
                }
                Err(e) => {
                    logger::debug(
                        LogTag::Api,
                        &format!("GeckoTerminal mint lookup failed: {e}"),
                    );
                }
            }
        }
    } else {
        // Use DexScreener search for name/symbol queries
        if apis.dexscreener.is_enabled() {
            match apis.dexscreener.search(query).await {
                Ok(pools) => {
                    logger::debug(
                        LogTag::Api,
                        &format!("DexScreener search returned {} pools", pools.len()),
                    );

                    // Every pool on this chain counts: the ranking runs over the
                    // whole response, and each token keeps its busiest pool.
                    let network = adapter_for(chain).market_data_network();
                    for pool in pools.iter().filter(|pool| pool.chain_id == network) {
                        keep_busiest(&mut results_map, result_from_pool(pool));
                    }
                }
                Err(e) => {
                    logger::debug(LogTag::Api, &format!("DexScreener search failed: {e}"));
                }
            }
        }
    }

    let mut results: Vec<TokenSearchResult> = results_map.into_values().collect();
    rank_search_results(query, &mut results);
    results.truncate(max_results);

    // Persist all found tokens to the database
    // This ensures tokens discovered via search can be used by analyzer and other features
    let mut persisted_count = 0;
    for result in &results {
        if persist_token_to_database(chain, result).await {
            persisted_count += 1;
        }
    }

    let total = results.len();

    logger::info(
        LogTag::Api,
        &format!(
            "Token search completed: query='{}', results={}, persisted={}",
            query, total, persisted_count
        ),
    );

    Ok(SearchResults {
        results,
        query: query.to_string(),
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_mint_address() {
        let adapter = adapter_for(ChainId::Solana);

        // Valid Solana addresses
        assert!(adapter.looks_like_address("So11111111111111111111111111111111111111112"));
        assert!(adapter.looks_like_address("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));

        // Invalid - too short
        assert!(!adapter.looks_like_address("So11111111"));

        // Invalid - contains invalid characters
        assert!(!adapter.looks_like_address("0xabcdef1234567890abcdef1234567890abcdef12"));

        // Invalid - has spaces
        assert!(!adapter.looks_like_address("So11 1111111111111111111111111111111111112"));

        // Name/symbol queries
        assert!(!adapter.looks_like_address("BONK"));
        assert!(!adapter.looks_like_address("solana"));
        assert!(!adapter.looks_like_address("pepe"));
    }

    fn row(
        mint: &str,
        symbol: &str,
        name: &str,
        volume: Option<f64>,
        liquidity: f64,
    ) -> TokenSearchResult {
        TokenSearchResult {
            mint: mint.to_owned(),
            name: name.to_owned(),
            symbol: symbol.to_owned(),
            logo_url: None,
            price_usd: None,
            price_change_h24: None,
            market_cap: None,
            fdv: None,
            volume_24h: volume,
            liquidity_usd: Some(liquidity),
            pair_created_at: None,
            source: "dexscreener".to_owned(),
        }
    }

    fn mints(rows: &[TokenSearchResult]) -> Vec<&str> {
        rows.iter().map(|row| row.mint.as_str()).collect()
    }

    #[test]
    fn relevance_tier_leads_and_volume_orders_within_a_tier() {
        let mut rows = vec![
            row("name-prefix", "XYZ", "Bonkers", Some(9e9), 1e9),
            row("fuzzy", "AAA", "Something", Some(9e9), 1e9),
            row("exact-small", "BONK", "Clone", Some(10.0), 1e3),
            row("exact-busy", "BONK", "Bonk", Some(5e7), 1e6),
            row("prefix", "BONKY", "Other", Some(1e8), 1e7),
            row("exact-name", "ZZZ", "bonk", Some(1e9), 1e8),
        ];
        rank_search_results("$Bonk ", &mut rows);
        assert_eq!(
            mints(&rows),
            [
                "exact-busy",
                "exact-small",
                "prefix",
                "exact-name",
                "name-prefix",
                "fuzzy"
            ]
        );
    }

    #[test]
    fn untraded_liquidity_never_outranks_a_traded_token() {
        let mut rows = vec![
            row("fake-liquidity", "PEPE", "Pepe", Some(0.0), 9e12),
            row("unknown-volume", "PEPE", "Pepe", None, 8e12),
            row("real", "PEPE", "Pepe", Some(2e6), 4e5),
        ];
        rank_search_results("pepe", &mut rows);
        assert_eq!(mints(&rows), ["real", "fake-liquidity", "unknown-volume"]);
    }

    #[test]
    fn a_token_keeps_its_busiest_pool() {
        let mut kept = HashMap::new();
        let mut thin = row("mint", "TKN", "Token", Some(10.0), 9e9);
        thin.price_usd = Some(1.0);
        let mut busy = row("mint", "TKN", "Token", Some(1e6), 1e4);
        busy.price_usd = Some(2.0);
        let mut quiet = row("mint", "TKN", "Token", None, 1e12);
        quiet.price_usd = Some(3.0);

        keep_busiest(&mut kept, thin);
        keep_busiest(&mut kept, busy);
        keep_busiest(&mut kept, quiet);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept["mint"].price_usd, Some(2.0));
    }
}
