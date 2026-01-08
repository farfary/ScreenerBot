use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant as StdInstant;

use chrono::Utc;
use serde_json::json;

use crate::config::FilteringConfig;
use crate::events::{record_filtering_event, Severity};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::tokens::types::{DataSource, Token};
use crate::tokens::{
    batch_clear_rejection_status_async, batch_update_priority_async,
    batch_update_rejection_status_async, batch_upsert_rejection_stats_async,
    get_all_tokens_for_filtering_async, list_blacklisted_tokens_async,
};

use super::sources::{self, FilterRejectionReason};
use super::types::{
    BlacklistReasonInfo, FilteringSnapshot, PassedToken, RejectedToken, TokenEntry,
    MAX_DECISION_HISTORY,
};

const MIN_VALID_BLOCKCHAIN_TIMESTAMP: i64 = 1; // Avoid 0/invalid timestamps from market APIs

pub async fn compute_snapshot(
    config: FilteringConfig,
    previous: Option<&FilteringSnapshot>,
) -> Result<FilteringSnapshot, String> {
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

    // PERF: Load tokens with market data only (reduces 144k -> ~56k tokens)
    // Tokens without DexScreener/GeckoTerminal data are immediately rejected anyway
    let load_start = StdInstant::now();
    let mut tokens = get_all_tokens_for_filtering_async()
        .await
        .map_err(|e| format!("Failed to batch load tokens: {}", e))?;

    let load_duration_ms = load_start.elapsed().as_millis();
    let total_candidates = tokens.len();

    if tokens.is_empty() {
        logger::debug(
            LogTag::Filtering,
            "No tokens with market data found - snapshot remains empty",
        );

        // DEBUG: Record empty token store
        record_filtering_event(
            "snapshot_empty_store",
            Severity::Debug,
            None,
            None,
            json!({
                "reason": "no_tokens_with_market_data",
            }),
        )
        .await;

        return Ok(FilteringSnapshot::empty());
    }

    logger::info(
        LogTag::Filtering,
        &format!(
            "Loaded {} tokens with market data in {}ms (memory optimized)",
            total_candidates, load_duration_ms
        ),
    );

    // Aggregate blacklist metadata from token database and pools subsystem
    let mut blacklist_reasons_map: HashMap<String, Vec<BlacklistReasonInfo>> = HashMap::new();

    match list_blacklisted_tokens_async().await {
        Ok(entries) => {
            for entry in entries {
                let info = BlacklistReasonInfo {
                    category: "token".to_string(),
                    reason: entry.reason,
                    detail: if entry.source.is_empty() {
                        None
                    } else {
                        Some(entry.source)
                    },
                };
                let reasons = blacklist_reasons_map
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

                let Some(mint) = token_mint else {
                    continue;
                };
                if mint.is_empty() {
                    continue;
                }

                let detail = if !pool_id.is_empty() {
                    Some(format!("pool={}", pool_id))
                } else {
                    program_id.map(|id| format!("program={}", id))
                };

                let info = BlacklistReasonInfo {
                    category: "pool".to_string(),
                    reason,
                    detail,
                };

                let reasons = blacklist_reasons_map.entry(mint).or_insert_with(Vec::new);
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

                let Some(mint) = token_mint else {
                    continue;
                };
                if mint.is_empty() {
                    continue;
                }

                let detail = if let Some(pool) = pool_id.as_ref() {
                    Some(format!("pool={}, account={}", pool, account_pubkey))
                } else {
                    Some(account_pubkey.clone())
                };

                let info = BlacklistReasonInfo {
                    category: "account".to_string(),
                    reason,
                    detail: source
                        .as_ref()
                        .map(|src| format!("{} ({})", account_pubkey, src))
                        .or(detail),
                };

                let reasons = blacklist_reasons_map.entry(mint).or_insert_with(Vec::new);
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

    let mut blacklist_set: HashSet<String> = blacklist_reasons_map.keys().cloned().collect();

    for token in tokens.iter_mut() {
        if blacklist_set.contains(&token.mint) {
            if !token.is_blacklisted {
                token.is_blacklisted = true;
            }
        } else if token.is_blacklisted {
            let reasons = blacklist_reasons_map
                .entry(token.mint.clone())
                .or_insert_with(Vec::new);
            let fallback = BlacklistReasonInfo {
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
    blacklist_set = blacklist_reasons_map.keys().cloned().collect();

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
    let mut passed_tokens: Vec<PassedToken> = Vec::new();
    let mut rejected_tokens: Vec<RejectedToken> = Vec::new();
    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(tokens.len());
    let mut stats = FilteringStats::default();

    // PERF: Pre-wrap all tokens in Arc to avoid cloning ~2KB Token structs
    // This reduces memory from ~290MB (144k × 2KB clones) to ~1.2MB (144k × 8 byte refs)
    let arc_tokens: HashMap<String, Arc<Token>> = tokens
        .into_iter()
        .map(|t| (t.mint.clone(), Arc::new(t)))
        .collect();

    let mut previous_pass_times: HashMap<String, i64> = HashMap::new();
    let mut previous_reject_times: HashMap<String, i64> = HashMap::new();

    if let Some(prev_snapshot) = previous {
        for entry in &prev_snapshot.passed_tokens {
            previous_pass_times.insert(entry.mint.clone(), entry.passed_time);
        }
        for entry in &prev_snapshot.rejected_tokens {
            previous_reject_times.insert(entry.mint.clone(), entry.rejection_time);
        }
    }

    // PERF: Collect batch data instead of spawning individual tasks
    // This reduces 260k+ tokio::spawn calls to just 4 batch operations
    let mut batch_clear_mints: Vec<String> = Vec::new();
    let mut batch_priority_mints: Vec<String> = Vec::new();
    let mut batch_rejection_updates: Vec<(String, String, String, i64)> = Vec::new();
    let mut batch_rejection_stats: Vec<(String, String, i64)> = Vec::new();

    for (mint, token) in arc_tokens.iter() {
        stats.total_processed += 1;

        let has_pool_price = priced_set.contains(&token.mint);
        let has_open_position = open_position_set.contains(&token.mint);
        let has_ohlcv = ohlcv_set.contains(&token.mint);

        let creation_timestamp = token
            .blockchain_created_at
            .filter(|dt| dt.timestamp() >= MIN_VALID_BLOCKCHAIN_TIMESTAMP)
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| token.first_discovered_at.timestamp());

        // PERF: Use Arc::clone (8 bytes) instead of Token::clone (~2KB)
        token_entries.insert(
            mint.clone(),
            TokenEntry {
                token: Arc::clone(token),
                has_pool_price,
                has_open_position,
                has_ohlcv,
                pair_created_at: Some(creation_timestamp),
                last_updated: token.market_data_last_fetched_at,
            },
        );

        match apply_all_filters(token, &config).await {
            Ok(()) => {
                filtered_mints.push(token.mint.clone());
                stats.passed += 1;

                let passed_time = previous_pass_times
                    .get(token.mint.as_str())
                    .copied()
                    .unwrap_or_else(|| Utc::now().timestamp());

                passed_tokens.push(PassedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    passed_time,
                });

                // PERF: Collect for batch processing instead of spawning individual tasks
                batch_clear_mints.push(token.mint.clone());
                batch_priority_mints.push(token.mint.clone());

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
                let rejection_time = previous_reject_times
                    .get(token.mint.as_str())
                    .copied()
                    .unwrap_or_else(|| Utc::now().timestamp());

                let reason_label = reason.label().to_string();
                let reason_source = reason.source().as_str().to_string();

                rejected_tokens.push(RejectedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    reason: reason_label.clone(),
                    rejection_time,
                });

                // PERF: Collect for batch processing instead of spawning individual tasks
                batch_rejection_updates.push((
                    token.mint.clone(),
                    reason_label.clone(),
                    reason_source.clone(),
                    rejection_time,
                ));
                batch_rejection_stats.push((reason_label.clone(), reason_source, rejection_time));

                // DEBUG: Record token rejection (sample to avoid spam)
                if stats.rejected % 10 == 1 {
                    let mint = token.mint.clone();
                    let symbol = token.symbol.clone();
                    let reason_str = reason_label;
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

    // PERF: Spawn only 4 batch tasks instead of 260k+ individual tasks
    // This dramatically reduces memory usage and task scheduler overhead
    let passed_count = batch_clear_mints.len();
    let rejected_count = batch_rejection_updates.len();

    if !batch_clear_mints.is_empty() {
        tokio::spawn(async move {
            if let Err(e) = batch_clear_rejection_status_async(batch_clear_mints).await {
                logger::warning(
                    LogTag::Filtering,
                    &format!("Failed to batch clear rejection status: {}", e),
                );
            }
        });
    }

    if !batch_priority_mints.is_empty() {
        tokio::spawn(async move {
            if let Err(e) = batch_update_priority_async(batch_priority_mints, 60).await {
                logger::warning(
                    LogTag::Filtering,
                    &format!("Failed to batch update priorities: {}", e),
                );
            }
        });
    }

    if !batch_rejection_updates.is_empty() {
        tokio::spawn(async move {
            if let Err(e) = batch_update_rejection_status_async(batch_rejection_updates).await {
                logger::warning(
                    LogTag::Filtering,
                    &format!("Failed to batch update rejection status: {}", e),
                );
            }
        });
    }

    if !batch_rejection_stats.is_empty() {
        tokio::spawn(async move {
            if let Err(e) = batch_upsert_rejection_stats_async(batch_rejection_stats).await {
                logger::warning(
                    LogTag::Filtering,
                    &format!("Failed to batch upsert rejection stats: {}", e),
                );
            }
        });
    }

    logger::debug(
        LogTag::Filtering,
        &format!(
            "Batch DB updates: passed={} (clear+priority), rejected={} (status+stats)",
            passed_count, rejected_count
        ),
    );

    let elapsed_ms = start.elapsed().as_millis();

    let rejection_summary = stats.rejection_summary();
    logger::info(
        LogTag::Filtering,
        &format!(
            "processed={} passed={} rejected={} duration_ms={} rejection_summary={}",
            stats.total_processed, stats.passed, stats.rejected, elapsed_ms, rejection_summary
        ),
    );

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
        passed_tokens: {
            passed_tokens.sort_unstable_by(|a, b| {
                b.passed_time
                    .cmp(&a.passed_time)
                    .then_with(|| a.mint.cmp(&b.mint))
            });
            passed_tokens.truncate(MAX_DECISION_HISTORY);
            passed_tokens
        },
        rejected_mints,
        rejected_tokens: {
            rejected_tokens.sort_unstable_by(|a, b| {
                b.rejection_time
                    .cmp(&a.rejection_time)
                    .then_with(|| a.mint.cmp(&b.mint))
            });
            rejected_tokens.truncate(MAX_DECISION_HISTORY);
            rejected_tokens
        },
        tokens: token_entries,
        blacklist_reasons: blacklist_reasons_map,
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

    // PERF: The batch load already fetches preferred source + fallback.
    // If data_source is DexScreener or GeckoTerminal, that data is already loaded.
    // If data_source is Unknown, neither source has data - no point in extra DB queries.
    // Only fetch individual source if explicitly needed AND data comes from OTHER source.

    let dex_token_ref = if config.dexscreener.enabled {
        if token.data_source == DataSource::DexScreener {
            // Already have dexscreener data
            Some(token)
        } else if token.data_source == DataSource::Unknown {
            // No market data at all - batch load already tried both sources
            None
        } else {
            // Has gecko data but not dex - would need individual fetch
            // PERF: Skip this fetch for now - if dex filtering is required and data is missing,
            // the token will be rejected anyway. This avoids N+1 queries.
            None
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

    let gecko_token_ref = if config.geckoterminal.enabled {
        if token.data_source == DataSource::GeckoTerminal {
            // Already have gecko data
            Some(token)
        } else if token.data_source == DataSource::Unknown {
            // No market data at all
            None
        } else {
            // Has dex data but not gecko - skip fetch
            None
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
