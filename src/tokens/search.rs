/// Token search functionality using DexScreener and GeckoTerminal APIs
///
/// Provides unified search across multiple data sources with deduplication.
/// DexScreener supports direct search, GeckoTerminal requires mint-based lookup.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::apis::get_api_manager;
use crate::logger::{self, LogTag};

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
    pub market_cap: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity_usd: Option<f64>,
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
// SEARCH IMPLEMENTATION
// =============================================================================

/// Check if a string looks like a Solana mint address (base58, ~44 chars)
fn is_mint_address(query: &str) -> bool {
    let trimmed = query.trim();
    // Solana addresses are base58 encoded and typically 32-44 characters
    if trimmed.len() < 32 || trimmed.len() > 44 {
        return false;
    }
    // Base58 alphabet: 123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
    trimmed.chars().all(|c| {
        matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z')
    })
}

/// Search for tokens across available data sources
///
/// Strategy:
/// - If query looks like a mint address, fetch token data directly by mint
/// - Otherwise, use DexScreener's search endpoint
/// - Deduplicate results by mint address, preferring DexScreener data
pub async fn search_tokens(query: &str, limit: Option<usize>) -> Result<SearchResults, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    let max_results = limit.unwrap_or(20).min(50);

    logger::debug(
        LogTag::Api,
        &format!("Token search: query='{}', limit={}", query, max_results),
    );

    let apis = get_api_manager();

    // Collect results from sources
    let mut results_map: HashMap<String, TokenSearchResult> = HashMap::new();

    // If it looks like a mint address, do direct lookups
    if is_mint_address(query) {
        logger::debug(
            LogTag::Api,
            &format!("Query '{}' looks like mint address, fetching directly", query),
        );

        // Try DexScreener first
        if apis.dexscreener.is_enabled() {
            match apis.dexscreener.fetch_token_pools(query, None).await {
                Ok(pools) => {
                    if let Some(pool) = pools.first() {
                        let result = TokenSearchResult {
                            mint: pool.mint.clone(),
                            name: pool.base_token_name.clone(),
                            symbol: pool.base_token_symbol.clone(),
                            logo_url: pool.info_image_url.clone(),
                            price_usd: pool.price_usd.parse().ok(),
                            market_cap: pool.market_cap,
                            volume_24h: pool.volume_h24,
                            liquidity_usd: pool.liquidity_usd,
                            source: "dexscreener".to_string(),
                        };
                        results_map.insert(result.mint.clone(), result);
                    }
                }
                Err(e) => {
                    logger::debug(
                        LogTag::Api,
                        &format!("DexScreener mint lookup failed: {}", e),
                    );
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
                                symbol: pool.pool_name.split('/').next().unwrap_or("").to_string(),
                                logo_url: None,
                                price_usd: pool.token_price_usd.parse().ok(),
                                market_cap: pool.market_cap_usd,
                                volume_24h: pool.volume_h24,
                                liquidity_usd: pool.reserve_usd,
                                source: "geckoterminal".to_string(),
                            };
                            results_map.insert(result.mint.clone(), result);
                        }
                    }
                }
                Err(e) => {
                    logger::debug(
                        LogTag::Api,
                        &format!("GeckoTerminal mint lookup failed: {}", e),
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

                    // Filter to only Solana tokens and deduplicate by mint
                    for pool in pools {
                        if pool.chain_id != "solana" {
                            continue;
                        }
                        if results_map.len() >= max_results {
                            break;
                        }
                        if !results_map.contains_key(&pool.mint) {
                            let result = TokenSearchResult {
                                mint: pool.mint.clone(),
                                name: pool.base_token_name.clone(),
                                symbol: pool.base_token_symbol.clone(),
                                logo_url: pool.info_image_url.clone(),
                                price_usd: pool.price_usd.parse().ok(),
                                market_cap: pool.market_cap,
                                volume_24h: pool.volume_h24,
                                liquidity_usd: pool.liquidity_usd,
                                source: "dexscreener".to_string(),
                            };
                            results_map.insert(result.mint.clone(), result);
                        }
                    }
                }
                Err(e) => {
                    logger::debug(LogTag::Api, &format!("DexScreener search failed: {}", e));
                }
            }
        }
    }

    // Convert to sorted results (by liquidity, descending)
    let mut results: Vec<TokenSearchResult> = results_map.into_values().collect();
    results.sort_by(|a, b| {
        b.liquidity_usd
            .unwrap_or(0.0)
            .partial_cmp(&a.liquidity_usd.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(max_results);

    let total = results.len();

    logger::info(
        LogTag::Api,
        &format!(
            "Token search completed: query='{}', results={}",
            query, total
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
        // Valid Solana addresses
        assert!(is_mint_address("So11111111111111111111111111111111111111112"));
        assert!(is_mint_address("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"));

        // Invalid - too short
        assert!(!is_mint_address("So11111111"));

        // Invalid - contains invalid characters
        assert!(!is_mint_address("0xabcdef1234567890abcdef1234567890abcdef12"));

        // Invalid - has spaces
        assert!(!is_mint_address("So11 1111111111111111111111111111111111112"));

        // Name/symbol queries
        assert!(!is_mint_address("BONK"));
        assert!(!is_mint_address("solana"));
        assert!(!is_mint_address("pepe"));
    }
}
