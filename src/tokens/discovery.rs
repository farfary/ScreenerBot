// tokens/discovery.rs
// Token discovery from multiple sources driven by configuration

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use chrono::Utc;
use solana_sdk::pubkey::Pubkey;

use crate::apis::{CoinGeckoClient, DefiLlamaClient};
use crate::logger::{log, LogTag};

use crate::config::get_config_clone;
use crate::tokens::blacklist;
use crate::tokens::events::{emit, TokenEvent};
use crate::tokens::provider::TokenDataProvider;
// Legacy snapshot import removed; discovery publishes candidates and tokens are hydrated via provider/store
use crate::tokens::types::ApiError;

const GECKO_TRENDING_DURATIONS: &[&str] = &["5m", "1h", "6h", "24h"];

/// Collect candidate mints from multiple discovery sources using config toggles.
/// Returns unique mint/source pairs.
pub async fn discover_from_sources(
    provider: &TokenDataProvider,
) -> Result<Vec<(String, String)>, String> {
    let cfg = get_config_clone();
    let discovery_cfg = cfg.tokens.discovery.clone();
    if !discovery_cfg.enabled {
        return Ok(Vec::new());
    }

    let api = provider.api();
    let mut candidates: Vec<(String, String)> = Vec::new();

    // DexScreener endpoints
    if discovery_cfg.dexscreener.enabled {
        if discovery_cfg.dexscreener.latest_profiles_enabled {
            match api.dexscreener.get_latest_profiles().await {
                Ok(profiles) => {
                    for profile in profiles {
                        if profile
                            .chain_id
                            .as_deref()
                            .map(|c| c.eq_ignore_ascii_case("solana"))
                            .unwrap_or(false)
                        {
                            if let Some(mint) = normalize_mint(&profile.address) {
                                candidates.push((mint, "dexscreener.latest_profiles".into()));
                            }
                        }
                    }
                }
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery DexScreener latest_profiles failed: {}", err),
                ),
            }
        }

        if discovery_cfg.dexscreener.latest_boosts_enabled {
            match api.dexscreener.get_latest_boosted_tokens().await {
                Ok(tokens) => {
                    for token in tokens {
                        if token.chain_id.eq_ignore_ascii_case("solana") {
                            if let Some(mint) = normalize_mint(&token.token_address) {
                                candidates.push((mint, "dexscreener.latest_boosts".into()));
                            }
                        }
                    }
                }
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery DexScreener latest_boosts failed: {}", err),
                ),
            }
        }

        if discovery_cfg.dexscreener.top_boosts_enabled {
            match api.dexscreener.get_top_boosted_tokens(Some("solana")).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.token_address) {
                            candidates.push((mint, "dexscreener.top_boosts".into()));
                        }
                    }
                }
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery DexScreener top_boosts failed: {}", err),
                ),
            }
        }
    }

    // GeckoTerminal endpoints
    if discovery_cfg.geckoterminal.enabled {
        if discovery_cfg.geckoterminal.new_pools_enabled {
            match api
                .geckoterminal
                .fetch_new_pools_by_network("solana", Some("base_token,quote_token,dex"), Some(1))
                .await
            {
                Ok(pools) => {
                    for pool in pools {
                        if let Some(mint) = normalize_mint(&pool.base_token_id) {
                            candidates.push((mint, "gecko.new_pools".into()));
                        }
                        if let Some(mint) = normalize_mint(&pool.quote_token_id) {
                            candidates.push((mint, "gecko.new_pools".into()));
                        }
                    }
                }
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Gecko new_pools failed: {}", err),
                ),
            }
        }

        if discovery_cfg.geckoterminal.recently_updated_enabled {
            match api
                .geckoterminal
                .fetch_recently_updated_tokens(Some("network"), Some("solana"))
                .await
            {
                Ok(response) => {
                    for entry in response.data {
                        if let Some(mint) = normalize_mint(&entry.attributes.address) {
                            candidates.push((mint, "gecko.recently_updated".into()));
                        }
                    }
                }
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Gecko recently_updated failed: {}", err),
                ),
            }
        }

        if discovery_cfg.geckoterminal.trending_enabled {
            for duration in GECKO_TRENDING_DURATIONS {
                match api
                    .geckoterminal
                    .fetch_trending_pools_by_network(
                        Some("solana"),
                        Some(1),
                        Some(duration),
                        Some(vec!["base_token", "quote_token"]),
                    )
                    .await
                {
                    Ok(pools) => {
                        for pool in pools {
                            if let Some(mint) = normalize_mint(&pool.base_token_id) {
                                candidates
                                    .push((mint.clone(), format!("gecko.trending.{}", duration)));
                            }
                            if let Some(mint) = normalize_mint(&pool.quote_token_id) {
                                candidates
                                    .push((mint.clone(), format!("gecko.trending.{}", duration)));
                            }
                        }
                    }
                    Err(err) => log(
                        LogTag::Tokens,
                        "WARN",
                        &format!("Discovery Gecko trending {} failed: {}", duration, err),
                    ),
                }
            }
        }
    }

    // Rugcheck endpoints
    if discovery_cfg.rugcheck.enabled {
        if discovery_cfg.rugcheck.new_tokens_enabled {
            match api.rugcheck.fetch_new_tokens().await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.mint) {
                            candidates.push((mint, "rugcheck.new_tokens".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Rugcheck new_tokens failed: {}", err),
                ),
            }
        }

        if discovery_cfg.rugcheck.recent_enabled {
            match api.rugcheck.fetch_recent_tokens().await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.mint) {
                            candidates.push((mint, "rugcheck.recent".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Rugcheck recent failed: {}", err),
                ),
            }
        }

        if discovery_cfg.rugcheck.trending_enabled {
            match api.rugcheck.fetch_trending_tokens().await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.mint) {
                            candidates.push((mint, "rugcheck.trending".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Rugcheck trending failed: {}", err),
                ),
            }
        }

        if discovery_cfg.rugcheck.verified_enabled {
            match api.rugcheck.fetch_verified_tokens().await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.mint) {
                            candidates.push((mint, "rugcheck.verified".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Rugcheck verified failed: {}", err),
                ),
            }
        }
    }

    // Jupiter endpoints
    if discovery_cfg.jupiter.enabled {
        if discovery_cfg.jupiter.recent_enabled {
            match api.jupiter.fetch_recent_tokens().await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.id) {
                            candidates.push((mint, "jupiter.recent".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Jupiter recent failed: {}", err),
                ),
            }
        }

        if discovery_cfg.jupiter.top_organic_enabled {
            match api.jupiter.fetch_top_organic_score("24h", Some(100)).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.id) {
                            candidates.push((mint, "jupiter.top_organic".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Jupiter top_organic failed: {}", err),
                ),
            }
        }

        if discovery_cfg.jupiter.top_traded_enabled {
            match api.jupiter.fetch_top_traded("24h", Some(100)).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.id) {
                            candidates.push((mint, "jupiter.top_traded".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Jupiter top_traded failed: {}", err),
                ),
            }
        }

        if discovery_cfg.jupiter.top_trending_enabled {
            match api.jupiter.fetch_top_trending("24h", Some(100)).await {
                Ok(tokens) => {
                    for token in tokens {
                        if let Some(mint) = normalize_mint(&token.id) {
                            candidates.push((mint, "jupiter.top_trending".into()));
                        }
                    }
                }
                Err(ApiError::Disabled) => {}
                Err(err) => log(
                    LogTag::Tokens,
                    "WARN",
                    &format!("Discovery Jupiter top_trending failed: {}", err),
                ),
            }
        }
    }

    // CoinGecko (heavy) endpoint
    if discovery_cfg.coingecko.enabled && discovery_cfg.coingecko.markets_enabled {
        match api.coingecko.fetch_coins_list().await {
            Ok(coins) => {
                let addresses = CoinGeckoClient::extract_solana_addresses(&coins);
                for address in addresses.into_iter().take(300) {
                    if let Some(mint) = normalize_mint(&address) {
                        candidates.push((mint, "coingecko.markets".into()));
                    }
                }
            }
            Err(ApiError::Disabled) => {}
            Err(err) => log(
                LogTag::Tokens,
                "WARN",
                &format!("Discovery CoinGecko markets failed: {}", err),
            ),
        }
    }

    // DeFiLlama (heavy) endpoint
    if discovery_cfg.defillama.enabled && discovery_cfg.defillama.protocols_enabled {
        match api.defillama.fetch_protocols().await {
            Ok(protocols) => {
                let addresses = DefiLlamaClient::extract_solana_addresses(&protocols);
                for address in addresses.into_iter().take(300) {
                    if let Some(mint) = normalize_mint(&address) {
                        candidates.push((mint, "defillama.protocols".into()));
                    }
                }
            }
            Err(ApiError::Disabled) => {}
            Err(err) => log(
                LogTag::Tokens,
                "WARN",
                &format!("Discovery DeFiLlama protocols failed: {}", err),
            ),
        }
    }

    let mut unique: HashMap<String, String> = HashMap::new();
    for (mint, source) in candidates {
        if blacklist::is(&mint) {
            continue;
        }
        unique.entry(mint).or_insert(source);
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut results: Vec<(String, String)> = Vec::new();
    for (mint, source) in unique.into_iter() {
        *counts.entry(source.clone()).or_insert(0) += 1;
        results.push((mint, source));
    }

    if !results.is_empty() {
        results.sort_by(|a, b| a.0.cmp(&b.0));
        let summary = counts
            .iter()
            .map(|(src, count)| format!("{}={}", src, count))
            .collect::<Vec<_>>()
            .join(", ");
        log(
            LogTag::Tokens,
            "INFO",
            &format!(
                "Discovery aggregated {} unique mints ({})",
                results.len(),
                summary
            ),
        );
    }

    Ok(results)
}

pub async fn process_new_mints(provider: &TokenDataProvider, entries: Vec<(String, String)>) {
    let mut seen = HashSet::new();
    for (mint, source) in entries {
        if !seen.insert(mint.clone()) {
            continue;
        }
        emit(TokenEvent::TokenDiscovered {
            mint: mint.clone(),
            source: source.clone(),
            at: Utc::now(),
        });
        // Store snapshot upsert removed (unified Token store is authoritative)

        // Fetch complete data to populate snapshot
        if let Err(err) = provider.fetch_complete_data(&mint, None).await {
            log(
                LogTag::Tokens,
                "WARN",
                &format!(
                    "Discovery follow-up fetch failed: mint={} err={}",
                    mint, err
                ),
            );
        }
    }
}

fn normalize_mint(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed
        .strip_prefix("solana:")
        .or_else(|| trimmed.strip_prefix("solana_"))
        .unwrap_or(trimmed);

    Pubkey::from_str(candidate).map(|pk| pk.to_string()).ok()
}
