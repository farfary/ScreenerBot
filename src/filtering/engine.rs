use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant as StdInstant;

use chrono::Utc;
use futures::stream::{self, StreamExt};
use serde_json::json;

use crate::config::FilteringConfig;
use crate::events::{record_filtering_event, Severity};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::tokens::types::{DataSource, Token};
use crate::tokens::{
    get_full_token_async, get_full_token_for_source_async, list_blacklisted_tokens_async,
    list_tokens_async,
};

use super::sources::{self, FilterRejectionReason};
use super::types::{
    BlacklistSourceInfo, FilteringSnapshot, PassedToken, RejectedToken, TokenEntry,
    MAX_DECISION_HISTORY,
};

const TOKEN_FETCH_CONCURRENCY: usize = 24;

pub async fn compute_snapshot(config: FilteringConfig) -> Result<FilteringSnapshot, String> {
    let start = StdInstant::now();

    // INFO: Record snapshot computation start
    record_filtering_event(
        "snapshot_compute_start",
        Severity::Info,
        None,
        None,
        json!({}),
    )
    .await;

    let metadata = list_tokens_async(usize::MAX)
        .await
        .map_err(|e| format!("Failed to list tokens: {}", e))?;

    if metadata.is_empty() {
        logger::debug(
            LogTag::Filtering,
            "Token store empty - snapshot remains empty",
        );

        // DEBUG: Record empty token store
        record_filtering_event(
            "snapshot_empty_store",
            Severity::Debug,
            None,
            None,
            json!({
                "reason": "token_store_empty",
            }),
        )
        .await;

        return Ok(FilteringSnapshot::empty());
    }

    let total_candidates = metadata.len();

    let tokens_with_index: Vec<(usize, Token)> =
        stream::iter(metadata.into_iter().enumerate().map(|(index, meta)| {
            let mint = meta.mint.clone();
            async move {
                match get_full_token_async(&mint).await {
                    Ok(Some(token)) => Some((index, token)),
                    Ok(None) => None,
                    Err(err) => {
                        logger::debug(LogTag::Filtering, &format!("mint={} error={}", mint, err));
                        None
                    }
                }
            }
        }))
        .buffer_unordered(TOKEN_FETCH_CONCURRENCY)
        .filter_map(|entry| async move { entry })
        .collect()
        .await;

    if tokens_with_index.is_empty() {
        logger::info(
            LogTag::Filtering,
            &format!(
                "Unable to load full tokens for any candidates (total_candidates={})",
                total_candidates
            ),
        );

        // WARN: Record no tokens loaded
        record_filtering_event(
            "snapshot_no_tokens_loaded",
            Severity::Warn,
            None,
            None,
            json!({
                "total_candidates": total_candidates,
                "reason": "failed_to_load_full_tokens",
            }),
        )
        .await;

        return Ok(FilteringSnapshot::empty());
    }

    let mut tokens_sorted = tokens_with_index;
    tokens_sorted.sort_by_key(|(index, _)| *index);
    let mut tokens: Vec<Token> = tokens_sorted.into_iter().map(|(_, token)| token).collect();

    // Aggregate blacklist metadata from token database and pools subsystem
    let mut blacklist_sources_map: HashMap<String, Vec<BlacklistSourceInfo>> = HashMap::new();

    match list_blacklisted_tokens_async().await {
        Ok(entries) => {
            for entry in entries {
                let info = BlacklistSourceInfo {
                    category: "token".to_string(),
                    reason: entry.reason,
                    detail: if entry.source.is_empty() {
                        None
                    } else {
                        Some(entry.source)
                    },
                };
                let reasons = blacklist_sources_map
                    .entry(entry.mint)
                    .or_insert_with(Vec::new);
                if !reasons.contains(&info) {
                    reasons.push(info);
                }
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("failed_to_load_token_blacklist err={}", err),
            );
        }
    }

    match crate::pools::db::list_blacklisted_pools(None).await {
        Ok(records) => {
            for record in records {
                let crate::pools::db::BlacklistedPoolRecord {
                    pool_id,
                    reason,
                    token_mint,
                    program_id,
                    ..
                } = record;

                let Some(mut mint) = token_mint else { continue; };
                if mint.is_empty() {
                    continue;
                }

                let detail = if !pool_id.is_empty() {
                    Some(format!("pool={}", pool_id))
                } else {
                    program_id.map(|id| format!("program={}", id))
                };

                let info = BlacklistSourceInfo {
                    category: "pool".to_string(),
                    reason,
                    detail,
                };

                let reasons = blacklist_sources_map.entry(mint).or_insert_with(Vec::new);
                if !reasons.contains(&info) {
                    reasons.push(info);
                }
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("failed_to_load_pool_blacklist err={}", err),
            );
        }
    }

    match crate::pools::db::list_blacklisted_accounts(None).await {
        Ok(records) => {
            for record in records {
                let crate::pools::db::BlacklistedAccountRecord {
                    account_pubkey,
                    reason,
                    source,
                    pool_id,
                    token_mint,
                    ..
                } = record;

                let Some(mut mint) = token_mint else { continue; };
                if mint.is_empty() {
                    continue;
                }

                let detail = if let Some(pool) = pool_id.as_ref() {
                    Some(format!("pool={}, account={}", pool, account_pubkey))
                } else {
                    Some(account_pubkey.clone())
                };

                let info = BlacklistSourceInfo {
                    category: "account".to_string(),
                    reason,
                    detail: source
                        .as_ref()
                        .map(|src| format!("{} ({})", account_pubkey, src))
                        .or(detail),
                };

                let reasons = blacklist_sources_map.entry(mint).or_insert_with(Vec::new);
                if !reasons.contains(&info) {
                    reasons.push(info);
                }
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("failed_to_load_account_blacklist err={}", err),
            );
        }
    }

    let mut blacklist_set: HashSet<String> = blacklist_sources_map.keys().cloned().collect();

    for token in tokens.iter_mut() {
        if blacklist_set.contains(&token.mint) {
            if !token.is_blacklisted {
                token.is_blacklisted = true;
            }
        } else if token.is_blacklisted {
            let reasons = blacklist_sources_map
                .entry(token.mint.clone())
                .or_insert_with(Vec::new);
            let fallback = BlacklistSourceInfo {
                category: "token".to_string(),
                reason: "database".to_string(),
                detail: None,
            };
            if !reasons.contains(&fallback) {
                reasons.push(fallback);
            }
            blacklist_set.insert(token.mint.clone());
        }
    }

    // Refresh set in case new entries were added during normalization
    blacklist_set = blacklist_sources_map.keys().cloned().collect();

    let candidate_mints: Vec<String> = tokens.iter().map(|t| t.mint.clone()).collect();

    let priced_set: HashSet<String> = crate::pools::get_available_tokens().into_iter().collect();
    let open_position_set: HashSet<String> =
        positions::get_open_mints().await.into_iter().collect();

    let ohlcv_set: HashSet<String> =
        match crate::ohlcvs::get_mints_with_data(&candidate_mints).await {
            Ok(set) => set,
            Err(err) => {
                logger::warning(LogTag::Filtering, &format!("error={}", err));
                HashSet::new()
            }
        };

    let mut filtered_mints: Vec<String> = Vec::new();
    let mut rejected_mints: Vec<String> = Vec::new();
    let mut passed_tokens: VecDeque<PassedToken> = VecDeque::new();
    let mut rejected_tokens: VecDeque<RejectedToken> = VecDeque::new();
    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(tokens.len());
    let mut stats = FilteringStats::default();

    for token in tokens.iter() {
        stats.total_processed += 1;

        let has_pool_price = priced_set.contains(&token.mint);
        let has_open_position = open_position_set.contains(&token.mint);
        let has_ohlcv = ohlcv_set.contains(&token.mint);

        token_entries.insert(
            token.mint.clone(),
            TokenEntry {
                token: token.clone(),
                has_pool_price,
                has_open_position,
                has_ohlcv,
                pair_created_at: Some(
                    token
                        .token_birth_at
                        .unwrap_or(token.first_seen_at)
                        .timestamp(),
                ),
                last_updated: token.updated_at,
            },
        );

        match apply_all_filters(token, &config).await {
            Ok(()) => {
                filtered_mints.push(token.mint.clone());
                stats.passed += 1;

                if passed_tokens.len() >= MAX_DECISION_HISTORY {
                    passed_tokens.pop_front();
                }
                passed_tokens.push_back(PassedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    passed_time: Utc::now().timestamp(),
                });

                // Set priority for passed tokens: FilterPassed (60)
                // This is separate from PoolTracked priority (75) to avoid conflicts
                let mint_for_priority = token.mint.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        crate::tokens::update_token_priority_async(&mint_for_priority, 60).await
                    {
                        logger::error(
                            LogTag::Filtering,
                            &format!(
                                "Failed to set FilterPassed priority for {}: {}",
                                mint_for_priority, e
                            ),
                        );
                    }
                });

                // DEBUG: Record token passed (sample to avoid spam)
                if stats.passed % 10 == 1 {
                    let mint = token.mint.clone();
                    let symbol = token.symbol.clone();
                    tokio::spawn(async move {
                        record_filtering_event(
                            "token_passed",
                            Severity::Debug,
                            Some(&mint),
                            None,
                            json!({
                                "mint": mint,
                                "symbol": symbol,
                            }),
                        )
                        .await
                    });
                }
            }
            Err(reason) => {
                stats.record_rejection(reason);
                rejected_mints.push(token.mint.clone());
                if rejected_tokens.len() >= MAX_DECISION_HISTORY {
                    rejected_tokens.pop_front();
                }
                rejected_tokens.push_back(RejectedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    reason: reason.label().to_string(),
                    rejection_time: Utc::now().timestamp(),
                });

                // DEBUG: Record token rejection (sample to avoid spam)
                if stats.rejected % 10 == 1 {
                    let mint = token.mint.clone();
                    let symbol = token.symbol.clone();
                    let reason_str = reason.label().to_string();
                    tokio::spawn(async move {
                        record_filtering_event(
                            "token_rejected",
                            Severity::Debug,
                            Some(&mint),
                            None,
                            json!({
                                "mint": mint,
                                "symbol": symbol,
                                "reason": reason_str,
                            }),
                        )
                        .await
                    });
                }
            }
        }
    }

    let elapsed_ms = start.elapsed().as_millis();

    let rejection_summary = stats.rejection_summary();
    logger::info(LogTag::Filtering, &format!("processed={} passed={} rejected={} duration_ms={} rejection_summary={}",
            stats.total_processed,
            stats.passed,
            stats.rejected,
            elapsed_ms,
            rejection_summary));

    // INFO: Record snapshot computation complete
    let top_rejections: Vec<(String, usize)> = stats
        .rejection_counts
        .iter()
        .map(|(reason, count)| (reason.label().to_string(), *count))
        .collect::<Vec<_>>();

    record_filtering_event(
        "snapshot_compute_complete",
        Severity::Info,
        None,
        None,
        json!({
            "total_candidates": total_candidates,
            "total_processed": stats.total_processed,
            "passed": stats.passed,
            "rejected": stats.rejected,
            "duration_ms": elapsed_ms as u64,
            "top_rejection_reasons": top_rejections,
        }),
    )
    .await;

    let snapshot = FilteringSnapshot {
        updated_at: Utc::now(),
        filtered_mints,
        passed_tokens: passed_tokens.into_iter().collect(),
        rejected_mints,
        rejected_tokens: rejected_tokens.into_iter().collect(),
        tokens: token_entries,
        blacklist_sources: blacklist_sources_map,
    };

    // Store filtered results in tokens module for consumption by other services
    let mut blacklisted_tokens: HashSet<String> = snapshot
        .tokens
        .values()
        .filter(|e| e.token.is_blacklisted)
        .map(|e| e.token.mint.clone())
        .collect();
    for mint in blacklist_set {
        blacklisted_tokens.insert(mint);
    }

    let mut blacklisted_vec: Vec<String> = blacklisted_tokens.into_iter().collect();
    blacklisted_vec.sort_unstable();

    let filtered_lists = crate::tokens::FilteredTokenLists {
        passed: snapshot.filtered_mints.clone(),
        rejected: snapshot.rejected_mints.clone(),
        blacklisted: blacklisted_vec,
        with_pool_price: snapshot
            .tokens
            .values()
            .filter(|e| e.has_pool_price)
            .map(|e| e.token.mint.clone())
            .collect(),
        open_positions: snapshot
            .tokens
            .values()
            .filter(|e| e.has_open_position)
            .map(|e| e.token.mint.clone())
            .collect(),
        updated_at: snapshot.updated_at,
    };

    crate::tokens::store_filtered_results(filtered_lists);

    Ok(snapshot)
}

async fn apply_all_filters(
    token: &Token,
    config: &FilteringConfig,
) -> Result<(), FilterRejectionReason> {
    sources::meta::evaluate(token, config).await?;

    let mut dex_overlay: Option<Token> = None;
    let dex_token_ref = if config.dexscreener.enabled {
        if token.data_source == DataSource::DexScreener {
            Some(token)
        } else {
            match get_full_token_for_source_async(&token.mint, DataSource::DexScreener).await {
                Ok(Some(full_token)) => {
                    dex_overlay = Some(full_token);
                    dex_overlay.as_ref()
                }
                Ok(None) => None,
                Err(err) => {
                    logger::debug(
                        LogTag::Filtering,
                        &format!("mint={} err={}", token.mint, err),
                    );
                    None
                }
            }
        }
    } else {
        None
    };

    if config.dexscreener.enabled {
        if let Some(dex_token) = dex_token_ref {
            sources::dexscreener::evaluate(dex_token, &config.dexscreener)?;
        } else {
            return Err(FilterRejectionReason::DexScreenerDataMissing);
        }
    }

    let mut gecko_overlay: Option<Token> = None;
    let gecko_token_ref = if config.geckoterminal.enabled {
        if token.data_source == DataSource::GeckoTerminal {
            Some(token)
        } else {
            match get_full_token_for_source_async(&token.mint, DataSource::GeckoTerminal).await {
                Ok(Some(full_token)) => {
                    gecko_overlay = Some(full_token);
                    gecko_overlay.as_ref()
                }
                Ok(None) => None,
                Err(err) => {
                    logger::info(
                        LogTag::Filtering,
                        &format!("mint={} err={}", token.mint, err),
                    );
                    None
                }
            }
        }
    } else {
        None
    };

    if config.geckoterminal.enabled {
        if let Some(gecko_token) = gecko_token_ref {
            sources::geckoterminal::evaluate(gecko_token, &config.geckoterminal)?;
        } else {
            return Err(FilterRejectionReason::GeckoTerminalDataMissing);
        }
    }

    if config.rugcheck.enabled {
        let has_rug_data = token.security_score.is_some()
            || token.token_type.is_some()
            || token.mint_authority.is_some()
            || token.freeze_authority.is_some()
            || token.graph_insiders_detected.is_some()
            || token.lp_provider_count.is_some()
            || token.total_holders.is_some()
            || !token.security_risks.is_empty()
            || !token.top_holders.is_empty()
            || token.creator_balance_pct.is_some()
            || token.transfer_fee_pct.is_some()
            || token.transfer_fee_max_amount.is_some()
            || token.transfer_fee_authority.is_some();

        if !has_rug_data {
            return Err(FilterRejectionReason::RugcheckDataMissing);
        }

        sources::rugcheck::evaluate(token, &config.rugcheck)?;
    }

    Ok(())
}

#[derive(Default)]
struct FilteringStats {
    total_processed: usize,
    passed: usize,
    rejected: usize,
    rejection_counts: HashMap<FilterRejectionReason, usize>,
}

impl FilteringStats {
    fn record_rejection(&mut self, reason: FilterRejectionReason) {
        self.rejected += 1;
        self.rejection_counts
            .entry(reason)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    fn rejection_summary(&self) -> String {
        if self.rejection_counts.is_empty() {
            return "-".to_string();
        }

        let mut parts: Vec<(FilterRejectionReason, usize)> = self
            .rejection_counts
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();
        parts.sort_by(|a, b| b.1.cmp(&a.1));

        parts
            .iter()
            .take(5)
            .map(|(reason, count)| format!("{}:{}", reason.label(), count))
            .collect::<Vec<_>>()
            .join(",")
    }
}
