// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Token discovery engine.
//!
//! Polls multiple data sources (DexScreener, GeckoTerminal, RugCheck, the
//! chain's own feeds) on a configurable interval to find new and trending
//! tokens. One loop runs per chain against that chain's token database and rate
//! budget. Discovered tokens are deduplicated, validated, and stored for
//! further analysis by the filtering and strategy pipelines.

use crate::apis::get_api_manager;
use crate::chains::{adapter_for, runtime_for, ChainId};
use crate::config;
use crate::events::{record_token_event, Severity};
use crate::logger::{self, LogTag};
use crate::tokens::database::TokenDatabase;
use crate::tokens::priorities::Priority;
use crate::tokens::updates::RateLimitCoordinator;
use crate::utils::{check_shutdown_or_delay, run_or_shutdown};
use futures::future::{join_all, BoxFuture};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use super::discovery_sources::*;

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

/// Start the background discovery loop for the chain of `db`
pub fn start_discovery_loop(
    db: Arc<TokenDatabase>,
    shutdown: Arc<Notify>,
    budget: Arc<RateLimitCoordinator>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut wait = Duration::from_secs(INITIAL_DELAY_SECS);
        let mut last_skip_reason: Option<String> = None;

        loop {
            if check_shutdown_or_delay(&shutdown, wait).await {
                break;
            }
            wait = Duration::from_secs(DISCOVERY_INTERVAL_SECS);

            // Race the discovery run (network API calls) against shutdown so a stop
            // mid-run returns at once and stays parked on notified() to never miss the
            // one-shot shutdown broadcast.
            match run_or_shutdown(&shutdown, run_discovery_once(&db, budget.clone())).await {
                None => break,
                Some(Ok(stats)) => {
                    if let Some(reason) = stats.skip_reason.clone() {
                        if last_skip_reason.as_ref() != Some(&reason) {
                            logger::info(
                                LogTag::Tokens,
                                &format!("[DISCOVERY] Skipping discovery loop: {reason}"),
                            );
                            last_skip_reason = Some(reason);
                        }
                        continue;
                    }

                    last_skip_reason = None;

                    let source_summary = if stats.by_source.is_empty() {
                        "-".to_owned()
                    } else {
                        let mut parts: Vec<String> = stats
                            .by_source
                            .iter()
                            .map(|(source, count)| format!("{source}:{count}"))
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
                    let severity = if stats.newly_added > 0 {
                        Severity::Info
                    } else {
                        Severity::Debug
                    };
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
                Some(Err(err)) => {
                    logger::error(LogTag::Tokens, &format!("[DISCOVERY] Run failed: {err}"));

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
    })
}

/// Perform a single discovery run for the chain of `db`
pub async fn run_discovery_once(
    db: &TokenDatabase,
    budget: Arc<RateLimitCoordinator>,
) -> crate::tokens::Result<DiscoveryStats> {
    let chain = db.chain();

    // Check if tools are running - skip discovery to reduce RPC contention
    if crate::global::are_tools_active() {
        return Ok(DiscoveryStats::skipped(
            "tools active (reducing RPC contention)",
        ));
    }

    // Skip the whole cycle when the internet is confirmed offline: every source
    // would just time out on DNS and spam the log. The loop's skip_reason logging
    // throttles to one line per state change, and we resume automatically once
    // connectivity returns. Only triggers on a CONFIRMED outage, never at startup.
    if crate::connectivity::is_network_offline() {
        return Ok(DiscoveryStats::skipped("network offline"));
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
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "dexscreener.latest_profiles".to_owned(),
                    fetch_dexscreener_profiles(&api, coord, chain).await,
                )
            }));
        }

        if discovery_cfg.dexscreener.latest_boosts_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "dexscreener.latest_boosts".to_owned(),
                    fetch_dexscreener_latest_boosts(&api, coord, chain).await,
                )
            }));
        }

        if discovery_cfg.dexscreener.top_boosts_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "dexscreener.top_boosts".to_owned(),
                    fetch_dexscreener_top_boosts(&api, coord, chain).await,
                )
            }));
        }
    }

    if discovery_cfg.geckoterminal.enabled && sources_cfg.geckoterminal.enabled {
        if discovery_cfg.geckoterminal.new_pools_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "geckoterminal.new_pools".to_owned(),
                    fetch_gecko_new_pools(&api, coord, chain).await,
                )
            }));
        }

        if discovery_cfg.geckoterminal.recently_updated_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "geckoterminal.recently_updated".to_owned(),
                    fetch_gecko_recent_updates(&api, coord, chain).await,
                )
            }));
        }

        if discovery_cfg.geckoterminal.trending_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "geckoterminal.trending".to_owned(),
                    fetch_gecko_trending(&api, coord, chain).await,
                )
            }));
        }
    }

    if discovery_cfg.rugcheck.enabled
        && sources_cfg.rugcheck.enabled
        && adapter_for(chain).has_rugcheck_reports()
    {
        if discovery_cfg.rugcheck.new_tokens_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.new_tokens".to_owned(),
                    fetch_rugcheck_new_tokens(&api, coord).await,
                )
            }));
        }

        if discovery_cfg.rugcheck.recent_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.recent".to_owned(),
                    fetch_rugcheck_recent_tokens(&api, coord).await,
                )
            }));
        }

        if discovery_cfg.rugcheck.trending_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.trending".to_owned(),
                    fetch_rugcheck_trending_tokens(&api, coord).await,
                )
            }));
        }

        if discovery_cfg.rugcheck.verified_enabled {
            let api = apis.clone();
            let coord = budget.clone();
            tasks.push(Box::pin(async move {
                (
                    "rugcheck.verified".to_owned(),
                    fetch_rugcheck_verified_tokens(&api, coord).await,
                )
            }));
        }
    }

    // Feeds the chain contributes itself; each reads its own config on every run.
    let chain_feeds = runtime_for(chain)
        .map(|runtime| runtime.discovery_feeds())
        .unwrap_or_default();
    for feed in chain_feeds {
        tasks.push(Box::pin(async move {
            (feed.label.to_owned(), (feed.fetch)().await)
        }));
    }

    if discovery_cfg.coingecko.enabled && discovery_cfg.coingecko.markets_enabled {
        let api = apis.clone();
        tasks.push(Box::pin(async move {
            (
                "coingecko.markets".to_owned(),
                fetch_coingecko_markets(&api, chain).await,
            )
        }));
    }

    if discovery_cfg.defillama.enabled && discovery_cfg.defillama.protocols_enabled {
        let api = apis.clone();
        tasks.push(Box::pin(async move {
            (
                "defillama.protocols".to_owned(),
                fetch_defillama_protocols(&api, chain).await,
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
                    match normalize_mint(chain, &record.mint) {
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
                    &format!("[DISCOVERY] Source {source} failed: {err}"),
                );
            }
        }
    }

    stats.unique_mints = candidates.len();

    for (mint, aggregate) in candidates {
        if db.is_blacklisted(&mint)? {
            stats.blacklisted += 1;
            continue;
        }

        if db.token_exists(&mint)? {
            stats.already_known += 1;
            continue;
        }

        db.upsert_token(
            &mint,
            aggregate.symbol.as_deref(),
            aggregate.name.as_deref(),
            aggregate.decimals,
        )?;

        if let Err(err) = db.update_priority(&mint, Priority::FilterPassed.to_value()) {
            logger::error(
                LogTag::Tokens,
                &format!("[DISCOVERY] Failed to set priority for {mint}: {err}"),
            );
        }

        let mut sources: Vec<String> = aggregate.sources.into_iter().collect();
        sources.sort();
        let source_summary = sources.join(",");

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

type DiscoveryFetchOutcome = (String, crate::tokens::Result<Vec<DiscoveryRecord>>);

#[derive(Debug, Clone)]
pub struct DiscoveryRecord {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
}

#[derive(Debug, Default)]
struct CandidateAggregate {
    symbol: Option<String>,
    name: Option<String>,
    decimals: Option<u8>,
    sources: HashSet<String>,
}

/// The candidate as a discoverable token address on `chain`, or `None` when it is
/// not a valid address there or is the chain's native or stablecoin asset.
fn normalize_mint(chain: ChainId, candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return None;
    }

    let adapter = adapter_for(chain);
    if adapter.validate_address(trimmed).is_err() {
        return None;
    }

    if adapter.is_native_asset(trimmed) || adapter.is_stable_asset(trimmed) {
        return None;
    }

    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::normalize_mint;
    use crate::chains::{adapter_for, ChainId};

    const SOL: &str = "So11111111111111111111111111111111111111112";
    const USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    const VALID_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263";

    /// An explicit 32-44 character window ahead of address validation and the
    /// native and stablecoin checks. Address validation alone must agree with it.
    fn length_window_rule(candidate: &str) -> Option<String> {
        let trimmed = candidate.trim();
        if trimmed.is_empty() || trimmed.len() < 32 || trimmed.len() > 44 {
            return None;
        }
        let adapter = adapter_for(ChainId::Solana);
        if adapter.validate_address(trimmed).is_err()
            || adapter.is_native_asset(trimmed)
            || adapter.is_stable_asset(trimmed)
        {
            return None;
        }
        Some(trimmed.to_string())
    }

    #[test]
    fn normalize_mint_matches_the_length_window_rule() {
        let thirty_one = "1".repeat(31);
        let forty_five = format!("{VALID_MINT}A");
        let cases: [(&str, Option<&str>); 9] = [
            (SOL, None),
            (USDC, None),
            (VALID_MINT, Some(VALID_MINT)),
            (
                "  DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263 ",
                Some(VALID_MINT),
            ),
            (&thirty_one, None),
            (&forty_five, None),
            ("0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl0OIl", None),
            ("11111111111111111111111111111111", None),
            ("", None),
        ];
        for (candidate, expected) in cases {
            let normalized = normalize_mint(ChainId::Solana, candidate);
            assert_eq!(normalized.as_deref(), expected, "{candidate:?}");
            assert_eq!(normalized, length_window_rule(candidate), "{candidate:?}");
        }
        assert_eq!(thirty_one.len(), 31);
        assert_eq!(forty_five.len(), 45);
    }
}
