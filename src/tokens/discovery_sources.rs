//! Discovery source fetch functions.
//!
//! Each function queries a single external API and returns a list of
//! [`DiscoveryRecord`] candidates. Called from [`super::discovery::run_discovery_once`].

use crate::apis::ApiManager;
use crate::tokens::updates::RateLimitCoordinator;
use std::sync::Arc;

use super::discovery::DiscoveryRecord;

// ── helpers ──────────────────────────────────────────────────────────────────

fn parse_gecko_token_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed
        .rsplit(|c| c == ':' || c == '_')
        .next()
        .unwrap_or(trimmed)
        .trim();

    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

fn collect_pool_tokens(pool: &crate::apis::geckoterminal::types::GeckoTerminalPool) -> Vec<String> {
    let mut tokens = Vec::new();

    if let Some(base) = parse_gecko_token_id(&pool.base_token_id) {
        tokens.push(base);
    }
    if let Some(quote) = parse_gecko_token_id(&pool.quote_token_id) {
        tokens.push(quote);
    }

    tokens
}

// ── DexScreener ──────────────────────────────────────────────────────────────

pub(super) async fn fetch_dexscreener_profiles(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    // Use profiles-specific rate limit (60/min, separate from market data updates)
    let permit = coordinator
        .acquire_dexscreener_profiles()
        .await
        .map_err(|e| e.to_string())?;
    let profiles = api.dexscreener.get_latest_profiles().await?;
    permit.forget();

    Ok(profiles
        .into_iter()
        .filter(|profile| {
            profile
                .chain_id
                .as_ref()
                .map(|chain| {
                    chain.eq_ignore_ascii_case(crate::chains::adapter().market_data_network())
                })
                .unwrap_or_default()
        })
        .map(|profile| DiscoveryRecord {
            mint: profile.token_address,
            symbol: None,
            name: None,
            decimals: None,
        })
        .collect())
}

pub(super) async fn fetch_dexscreener_latest_boosts(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    // Use boosts-specific rate limit (60/min, separate from market data updates)
    let permit = coordinator
        .acquire_dexscreener_boosts()
        .await
        .map_err(|e| e.to_string())?;
    let boosts = api.dexscreener.get_latest_boosted_tokens().await?;
    permit.forget();

    Ok(boosts
        .into_iter()
        .filter(|boost| {
            boost
                .chain_id
                .eq_ignore_ascii_case(crate::chains::adapter().market_data_network())
        })
        .map(|boost| DiscoveryRecord {
            mint: boost.token_address,
            symbol: None,
            name: None,
            decimals: None,
        })
        .collect())
}

pub(super) async fn fetch_dexscreener_top_boosts(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    // Use boosts-specific rate limit (60/min, separate from market data updates)
    let permit = coordinator
        .acquire_dexscreener_boosts()
        .await
        .map_err(|e| e.to_string())?;
    let boosts = api
        .dexscreener
        .get_top_boosted_tokens(Some(crate::chains::adapter().market_data_network()))
        .await?;
    permit.forget();

    Ok(boosts
        .into_iter()
        .map(|boost| DiscoveryRecord {
            mint: boost.token_address,
            symbol: None,
            name: None,
            decimals: None,
        })
        .collect())
}

// ── GeckoTerminal ────────────────────────────────────────────────────────────

pub(super) async fn fetch_gecko_new_pools(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_geckoterminal()
        .await
        .map_err(|e| e.to_string())?;
    let pools = api
        .geckoterminal
        .fetch_new_pools_by_network(
            crate::chains::adapter().market_data_network(),
            None,
            Some(1),
        )
        .await?;
    permit.forget();

    let mut records = Vec::new();
    for pool in pools {
        for token in collect_pool_tokens(&pool) {
            records.push(DiscoveryRecord {
                mint: token,
                symbol: None,
                name: None,
                decimals: None,
            });
        }
    }

    Ok(records)
}

pub(super) async fn fetch_gecko_recent_updates(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_geckoterminal()
        .await
        .map_err(|e| e.to_string())?;
    let response = api
        .geckoterminal
        .fetch_recently_updated_tokens(None, Some(crate::chains::adapter().market_data_network()))
        .await?;
    permit.forget();

    Ok(response
        .data
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.attributes.address,
            symbol: Some(token.attributes.symbol),
            name: Some(token.attributes.name),
            decimals: None,
        })
        .collect())
}

pub(super) async fn fetch_gecko_trending(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_geckoterminal()
        .await
        .map_err(|e| e.to_string())?;
    let pools = api
        .geckoterminal
        .fetch_trending_pools_by_network(
            Some(crate::chains::adapter().market_data_network()),
            Some(1),
            None,
            None,
        )
        .await?;
    permit.forget();

    let mut records = Vec::new();
    for pool in pools {
        for token in collect_pool_tokens(&pool) {
            records.push(DiscoveryRecord {
                mint: token,
                symbol: None,
                name: None,
                decimals: None,
            });
        }
    }

    Ok(records)
}

// ── Rugcheck ─────────────────────────────────────────────────────────────────

pub(super) async fn fetch_rugcheck_new_tokens(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_new_tokens()
        .await
        .map_err(|e| e.to_string())?;
    permit.forget();

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.mint,
            symbol: Some(token.symbol),
            name: None,
            decimals: Some(token.decimals),
        })
        .collect())
}

pub(super) async fn fetch_rugcheck_recent_tokens(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_recent_tokens()
        .await
        .map_err(|e| e.to_string())?;
    permit.forget();

    Ok(tokens
        .into_iter()
        .filter_map(|token| {
            // Only include tokens with metadata
            token.metadata.map(|meta| DiscoveryRecord {
                mint: token.mint.clone(),
                symbol: Some(meta.symbol),
                name: Some(meta.name),
                decimals: None,
            })
        })
        .collect())
}

pub(super) async fn fetch_rugcheck_trending_tokens(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_trending_tokens()
        .await
        .map_err(|e| e.to_string())?;
    permit.forget();

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.mint,
            symbol: None,
            name: None,
            decimals: None,
        })
        .collect())
}

pub(super) async fn fetch_rugcheck_verified_tokens(
    api: &Arc<ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let permit = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_verified_tokens()
        .await
        .map_err(|e| e.to_string())?;
    permit.forget();

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.mint,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: None,
        })
        .collect())
}

// ── Jupiter ──────────────────────────────────────────────────────────────────

pub(super) async fn fetch_jupiter_recent(
    api: &Arc<ApiManager>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let tokens = api
        .jupiter
        .fetch_recent_tokens()
        .await
        .map_err(|e| e.to_string())?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

pub(super) async fn fetch_jupiter_top_organic(
    api: &Arc<ApiManager>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let tokens = api
        .jupiter
        .fetch_top_organic_score("1h", None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

pub(super) async fn fetch_jupiter_top_traded(
    api: &Arc<ApiManager>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let tokens = api
        .jupiter
        .fetch_top_traded("1h", None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

pub(super) async fn fetch_jupiter_top_trending(
    api: &Arc<ApiManager>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let tokens = api
        .jupiter
        .fetch_top_trending("1h", None)
        .await
        .map_err(|e| e.to_string())?;

    Ok(tokens
        .into_iter()
        .map(|token| DiscoveryRecord {
            mint: token.id,
            symbol: Some(token.symbol),
            name: Some(token.name),
            decimals: Some(token.decimals),
        })
        .collect())
}

// ── CoinGecko ────────────────────────────────────────────────────────────────

pub(super) async fn fetch_coingecko_markets(
    api: &Arc<ApiManager>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let coins = api
        .coingecko
        .fetch_coins_list()
        .await
        .map_err(|e| e.to_string())?;

    let entries =
        crate::apis::coingecko::CoinGeckoClient::extract_solana_addresses_with_names(&coins);

    Ok(entries
        .into_iter()
        .map(|(name, mint)| DiscoveryRecord {
            mint,
            symbol: None,
            name: Some(name),
            decimals: None,
        })
        .collect())
}

// ── DefiLlama ────────────────────────────────────────────────────────────────

pub(super) async fn fetch_defillama_protocols(
    api: &Arc<ApiManager>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let protocols = api
        .defillama
        .fetch_protocols()
        .await
        .map_err(|e| e.to_string())?;

    let entries =
        crate::apis::defillama::DefiLlamaClient::extract_solana_addresses_with_names(&protocols);

    Ok(entries
        .into_iter()
        .map(|(name, mint)| DiscoveryRecord {
            mint,
            symbol: None,
            name: Some(name),
            decimals: None,
        })
        .collect())
}
