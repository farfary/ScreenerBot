use crate::apis::get_api_manager;
use crate::config;
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::pools::utils::{is_sol_mint, is_stablecoin_mint};
use crate::tokens::database::TokenDatabase;
use crate::tokens::events::{self, TokenEvent};
use crate::tokens::priorities::Priority;
use crate::tokens::updates::RateLimitCoordinator;
use chrono::Utc;
use futures::future::{join_all, BoxFuture};
use solana_sdk::pubkey::Pubkey;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::sleep;

/// Discovery run interval (seconds)
const DISCOVERY_INTERVAL_SECS: u64 = 60;
/// Initial delay before first discovery run (seconds)
const INITIAL_DELAY_SECS: u64 = 10;

/// Outcome metrics for a discovery run
#[derive(Debug, Default, Clone)]
pub struct DiscoveryStats {
    pub total_candidates: usize,
    pub unique_mints: usize,
    pub newly_added: usize,
    pub already_known: usize,
    pub blacklisted: usize,
    pub invalid: usize,
    pub errors: usize,
    pub duration_ms: u64,
    pub by_source: HashMap<String, usize>,
    pub skip_reason: Option<String>,
}

impl DiscoveryStats {
    fn skipped(reason: &str) -> Self {
        let mut stats = DiscoveryStats::default();
        stats.skip_reason = Some(reason.to_string());
        stats
    }
}

/// Start background discovery loop
pub fn start_discovery_loop(
    db: Arc<TokenDatabase>,
    shutdown: Arc<Notify>,
    coordinator: Arc<RateLimitCoordinator>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut wait = Duration::from_secs(INITIAL_DELAY_SECS);
        let mut last_skip_reason: Option<String> = None;

        loop {
            tokio::select! {
                _ = shutdown.notified() => break,
                _ = sleep(wait) => {
                    wait = Duration::from_secs(DISCOVERY_INTERVAL_SECS);

                    match run_discovery_once(&db, coordinator.clone()).await {
                        Ok(stats) => {
                            if let Some(reason) = stats.skip_reason.clone() {
                                if last_skip_reason.as_ref() != Some(&reason) {
                                    logger::info(
                                        LogTag::Tokens,
                                        &format!("[DISCOVERY] Skipping discovery loop: {}", reason),
                                    );
                                    last_skip_reason = Some(reason);
                                }
                                continue;
                            }

                            last_skip_reason = None;

                            let source_summary = if stats.by_source.is_empty() {
                                "-".to_string()
                            } else {
                                let mut parts: Vec<String> = stats
                                    .by_source
                                    .iter()
                                    .map(|(source, count)| format!("{}:{}", source, count))
                                    .collect();
                                parts.sort();
                                parts.join(", ")
                            };

                            logger::info(
                                LogTag::Tokens,
                                &format!(
                                    "[DISCOVERY] Completed: {} candidates, {} unique, {} new, {} known, {} blacklisted, {} invalid, {} errors ({} ms) | sources: {}",
                                    stats.total_candidates,
                                    stats.unique_mints,
                                    stats.newly_added,
                                    stats.already_known,
                                    stats.blacklisted,
                                    stats.invalid,
                                    stats.errors,
                                    stats.duration_ms,
                                    source_summary
                                ),
                            );

                            // Record discovery run completion (INFO if new tokens found, DEBUG otherwise)
                            let severity = if stats.newly_added > 0 { Severity::Info } else { Severity::Debug };
                            tokio::spawn({
                                let stats = stats.clone();
                                async move {
                                    record_token_event(
                                        "system", // system-level event, no specific mint
                                        "discovery_run_complete",
                                        severity,
                                        serde_json::json!({
                                            "total_candidates": stats.total_candidates,
                                            "unique_mints": stats.unique_mints,
                                            "newly_added": stats.newly_added,
                                            "already_known": stats.already_known,
                                            "blacklisted": stats.blacklisted,
                                            "invalid": stats.invalid,
                                            "errors": stats.errors,
                                            "duration_ms": stats.duration_ms,
                                            "by_source": stats.by_source,
                                        }),
                                    )
                                    .await;
                                }
                            });
                        }
                        Err(err) => {
                            logger::error(
                                LogTag::Tokens,
                                &format!("[DISCOVERY] Run failed: {}", err),
                            );

                            // Record discovery error
                            tokio::spawn({
                                let error_msg = err.to_string();
                                async move {
                                    record_token_event(
                                        "system",
                                        "discovery_run_failed",
                                        Severity::Error,
                                        serde_json::json!({
                                            "error": error_msg,
                                        }),
                                    )
                                    .await;
                                }
                            });
                        }
                    }
                }
            }
        }
    })
}

/// Perform a single discovery run
pub async fn run_discovery_once(
    db: &TokenDatabase,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<DiscoveryStats, String> {
    // Check if tools are running - skip discovery to reduce RPC contention
    if crate::global::are_tools_active() {
        return Ok(DiscoveryStats::skipped(
            "tools active (reducing RPC contention)",
        ));
    }

    let start = Instant::now();
    let cfg = config::get_config_clone();
    let discovery_cfg = &cfg.tokens.discovery;

    if !discovery_cfg.enabled {
        return Ok(DiscoveryStats::skipped("tokens.discovery.enabled=false"));
    }

    let sources_cfg = &cfg.tokens.sources;
    let apis = get_api_manager();

    let mut tasks: Vec<BoxFuture<'static, DiscoveryFetchOutcome>> = Vec::new();

    if discovery_cfg.dexscreener.enabled && sources_cfg.dexscreener.enabled {
        if discovery_cfg.dexscreener.latest_profiles_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "dexscreener.latest_profiles".to_string(),
                    fetch_dexscreener_profiles(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.dexscreener.latest_boosts_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "dexscreener.latest_boosts".to_string(),
                    fetch_dexscreener_latest_boosts(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.dexscreener.top_boosts_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "dexscreener.top_boosts".to_string(),
                    fetch_dexscreener_top_boosts(&api, coord.clone()).await,
                )
            }));
        }
    }

    if discovery_cfg.geckoterminal.enabled && sources_cfg.geckoterminal.enabled {
        if discovery_cfg.geckoterminal.new_pools_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "geckoterminal.new_pools".to_string(),
                    fetch_gecko_new_pools(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.geckoterminal.recently_updated_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "geckoterminal.recently_updated".to_string(),
                    fetch_gecko_recent_updates(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.geckoterminal.trending_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "geckoterminal.trending".to_string(),
                    fetch_gecko_trending(&api, coord.clone()).await,
                )
            }));
        }
    }

    if discovery_cfg.rugcheck.enabled && sources_cfg.rugcheck.enabled {
        if discovery_cfg.rugcheck.new_tokens_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.new_tokens".to_string(),
                    fetch_rugcheck_new_tokens(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.rugcheck.recent_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.recent".to_string(),
                    fetch_rugcheck_recent_tokens(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.rugcheck.trending_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.trending".to_string(),
                    fetch_rugcheck_trending_tokens(&api, coord.clone()).await,
                )
            }));
        }

        if discovery_cfg.rugcheck.verified_enabled {
            let api = apis.clone();
            let coord = coordinator.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.verified".to_string(),
                    fetch_rugcheck_verified_tokens(&api, coord.clone()).await,
                )
            }));
        }
    }

    if discovery_cfg.jupiter.enabled {
        if discovery_cfg.jupiter.recent_enabled {
            let api = apis.clone();
            tasks.push(Box::pin(async move {
                (
                    "jupiter.recent".to_string(),
                    fetch_jupiter_recent(&api).await,
                )
            }));
        }

        if discovery_cfg.jupiter.top_organic_enabled {
            let api = apis.clone();
            tasks.push(Box::pin(async move {
                (
                    "jupiter.top_organic".to_string(),
                    fetch_jupiter_top_organic(&api).await,
                )
            }));
        }

        if discovery_cfg.jupiter.top_traded_enabled {
            let api = apis.clone();
            tasks.push(Box::pin(async move {
                (
                    "jupiter.top_traded".to_string(),
                    fetch_jupiter_top_traded(&api).await,
                )
            }));
        }

        if discovery_cfg.jupiter.top_trending_enabled {
            let api = apis.clone();
            tasks.push(Box::pin(async move {
                (
                    "jupiter.top_trending".to_string(),
                    fetch_jupiter_top_trending(&api).await,
                )
            }));
        }
    }

    if discovery_cfg.coingecko.enabled && discovery_cfg.coingecko.markets_enabled {
        let api = apis.clone();
        tasks.push(Box::pin(async move {
            (
                "coingecko.markets".to_string(),
                fetch_coingecko_markets(&api).await,
            )
        }));
    }

    if discovery_cfg.defillama.enabled && discovery_cfg.defillama.protocols_enabled {
        let api = apis.clone();
        tasks.push(Box::pin(async move {
            (
                "defillama.protocols".to_string(),
                fetch_defillama_protocols(&api).await,
            )
        }));
    }

    if tasks.is_empty() {
        return Ok(DiscoveryStats::skipped("no discovery sources enabled"));
    }

    let mut stats = DiscoveryStats::default();
    let mut candidates: HashMap<String, CandidateAggregate> = HashMap::new();

    let results = join_all(tasks).await;
    for (source, outcome) in results {
        match outcome {
            Ok(records) => {
                let mut valid_from_source = 0usize;
                for record in records {
                    stats.total_candidates += 1;
                    match normalize_mint(&record.mint) {
                        Some(mint) => {
                            valid_from_source += 1;
                            let entry = candidates
                                .entry(mint.clone())
                                .or_insert_with(CandidateAggregate::default);
                            entry.sources.insert(source.clone());

                            if entry.symbol.is_none() {
                                entry.symbol = record.symbol.clone();
                            }
                            if entry.name.is_none() {
                                entry.name = record.name.clone();
                            }
                            if entry.decimals.is_none() {
                                entry.decimals = record.decimals;
                            }
                        }
                        None => {
                            stats.invalid += 1;
                        }
                    }
                }

                if valid_from_source > 0 {
                    stats
                        .by_source
                        .entry(source)
                        .and_modify(|count| *count += valid_from_source)
                        .or_insert(valid_from_source);
                }
            }
            Err(err) => {
                stats.errors += 1;
                logger::error(
                    LogTag::Tokens,
                    &format!("[DISCOVERY] Source {} failed: {}", source, err),
                );
            }
        }
    }

    stats.unique_mints = candidates.len();

    for (mint, aggregate) in candidates {
        if db.is_blacklisted(&mint).map_err(|e| e.to_string())? {
            stats.blacklisted += 1;
            continue;
        }

        if db.token_exists(&mint).map_err(|e| e.to_string())? {
            stats.already_known += 1;
            continue;
        }

        db.upsert_token(
            &mint,
            aggregate.symbol.as_deref(),
            aggregate.name.as_deref(),
            aggregate.decimals,
        )
        .map_err(|e| e.to_string())?;

        if let Err(err) = db.update_priority(&mint, Priority::FilterPassed.to_value()) {
            logger::error(
                LogTag::Tokens,
                &format!("[DISCOVERY] Failed to set priority for {}: {}", mint, err),
            );
        }

        let mut sources: Vec<String> = aggregate.sources.into_iter().collect();
        sources.sort();
        let source_summary = sources.join(",");

        events::emit(TokenEvent::TokenDiscovered {
            mint: mint.clone(),
            source: source_summary.clone(),
            at: Utc::now(),
        });

        // Record token discovery event (sampled - every 10th to avoid spam)
        if stats.newly_added % 10 == 0 {
            tokio::spawn({
                let mint = mint.clone();
                let source = source_summary.clone();
                async move {
                    record_token_event(
                        &mint,
                        "token_discovered",
                        Severity::Debug,
                        serde_json::json!({
                            "source": source,
                            "newly_added_count": stats.newly_added + 1,
                        }),
                    )
                    .await;
                }
            });
        }

        stats.newly_added += 1;
    }

    stats.duration_ms = start.elapsed().as_millis() as u64;
    Ok(stats)
}

type DiscoveryFetchOutcome = (String, Result<Vec<DiscoveryRecord>, String>);

#[derive(Debug, Clone)]
struct DiscoveryRecord {
    mint: String,
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<u8>,
}

#[derive(Debug, Default)]
struct CandidateAggregate {
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<u8>,
    sources: HashSet<String>,
}

fn normalize_mint(candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return None;
    }

    let len = trimmed.len();
    if len < 32 || len > 44 {
        return None;
    }

    if Pubkey::from_str(trimmed).is_err() {
        return None;
    }

    if is_sol_mint(trimmed) || is_stablecoin_mint(trimmed) {
        return None;
    }

    Some(trimmed.to_string())
}

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

async fn fetch_dexscreener_profiles(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    // Use profiles-specific rate limit (60/min, separate from market data updates)
    let _ = coordinator
        .acquire_dexscreener_profiles()
        .await
        .map_err(|e| e.to_string())?;
    let profiles = api.dexscreener.get_latest_profiles().await?;

    Ok(profiles
        .into_iter()
        .filter(|profile| {
            profile
                .chain_id
                .as_ref()
                .map(|chain| chain.eq_ignore_ascii_case("solana"))
                .unwrap_or(false)
        })
        .map(|profile| DiscoveryRecord {
            mint: profile.token_address,
            symbol: None,
            name: None,
            decimals: None,
        })
        .collect())
}

async fn fetch_dexscreener_latest_boosts(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    // Use boosts-specific rate limit (60/min, separate from market data updates)
    let _ = coordinator
        .acquire_dexscreener_boosts()
        .await
        .map_err(|e| e.to_string())?;
    let boosts = api.dexscreener.get_latest_boosted_tokens().await?;

    Ok(boosts
        .into_iter()
        .filter(|boost| boost.chain_id.eq_ignore_ascii_case("solana"))
        .map(|boost| DiscoveryRecord {
            mint: boost.token_address,
            symbol: None,
            name: None,
            decimals: None,
        })
        .collect())
}

async fn fetch_dexscreener_top_boosts(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    // Use boosts-specific rate limit (60/min, separate from market data updates)
    let _ = coordinator
        .acquire_dexscreener_boosts()
        .await
        .map_err(|e| e.to_string())?;
    let boosts = api
        .dexscreener
        .get_top_boosted_tokens(Some("solana"))
        .await?;

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

async fn fetch_gecko_new_pools(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_geckoterminal()
        .await
        .map_err(|e| e.to_string())?;
    let pools = api
        .geckoterminal
        .fetch_new_pools_by_network("solana", None, Some(1))
        .await?;

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

async fn fetch_gecko_recent_updates(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_geckoterminal()
        .await
        .map_err(|e| e.to_string())?;
    let response = api
        .geckoterminal
        .fetch_recently_updated_tokens(None, Some("solana"))
        .await?;

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

async fn fetch_gecko_trending(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_geckoterminal()
        .await
        .map_err(|e| e.to_string())?;
    let pools = api
        .geckoterminal
        .fetch_trending_pools_by_network(Some("solana"), Some(1), None, None)
        .await?;

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

async fn fetch_rugcheck_new_tokens(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_new_tokens()
        .await
        .map_err(|e| e.to_string())?;

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

async fn fetch_rugcheck_recent_tokens(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_recent_tokens()
        .await
        .map_err(|e| e.to_string())?;

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

async fn fetch_rugcheck_trending_tokens(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_trending_tokens()
        .await
        .map_err(|e| e.to_string())?;

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

async fn fetch_rugcheck_verified_tokens(
    api: &Arc<crate::apis::ApiManager>,
    coordinator: Arc<RateLimitCoordinator>,
) -> Result<Vec<DiscoveryRecord>, String> {
    let _ = coordinator
        .acquire_rugcheck()
        .await
        .map_err(|e| e.to_string())?;
    let tokens = api
        .rugcheck
        .fetch_verified_tokens()
        .await
        .map_err(|e| e.to_string())?;

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

async fn fetch_jupiter_recent(
    api: &Arc<crate::apis::ApiManager>,
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

async fn fetch_jupiter_top_organic(
    api: &Arc<crate::apis::ApiManager>,
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

async fn fetch_jupiter_top_traded(
    api: &Arc<crate::apis::ApiManager>,
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

async fn fetch_jupiter_top_trending(
    api: &Arc<crate::apis::ApiManager>,
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

async fn fetch_coingecko_markets(
    api: &Arc<crate::apis::ApiManager>,
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

async fn fetch_defillama_protocols(
    api: &Arc<crate::apis::ApiManager>,
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
