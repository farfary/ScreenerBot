// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Filtering engine — orchestrates multi-source token filtering with parallel execution.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant as StdInstant;

use chrono::{DateTime, Utc};
use serde_json::json;

use crate::chains::{runtime_for, ChainId};
use crate::config::FilteringConfig;
use crate::events::{record_filtering_event, Severity};
use crate::logger::{self, LogTag};
use crate::positions;
use crate::tokens::{
    batch_update_priority_async, get_all_tokens_for_filtering_async, list_blacklisted_tokens_async,
    record_filter_build_async, FilterOutcome, FilterRejectionRecord,
};

use super::sources::FilterRejectionReason;
use super::types::{
    BlacklistReasonInfo, FilteringSnapshot, PassedToken, RejectedToken, TokenEntry,
    MAX_DECISION_HISTORY,
};
use super::{Error, Result};

/// Compute a full filtering snapshot by evaluating all tokens against configured filters.
pub async fn compute_snapshot(
    chain: ChainId,
    config: FilteringConfig,
    previous: Option<&FilteringSnapshot>,
) -> Result<FilteringSnapshot> {
    let start = StdInstant::now();

    // The build's stamp: strictly after the previous build's, so a token the previous
    // build evaluated can never be mistaken for one this build did.
    let now = Utc::now().timestamp();
    let evaluated_at = previous.map_or(now, |p| now.max(p.evaluated_at + 1));

    // The chain's stage sequence, resolved once per refresh.
    let profile = runtime_for(chain)
        .ok_or(Error::RuntimeUnavailable { chain })?
        .filter_profile();

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
    let mut tokens = get_all_tokens_for_filtering_async(chain)
        .await
        .map_err(|e| Error::TokenSetLoad {
            kind: "batch filtering",
            source: e,
        })?;

    let load_duration_ms = load_start.elapsed().as_millis();
    let total_candidates = tokens.len();

    if tokens.is_empty() {
        // An empty corpus is a complete build: it records no outcomes, and the write
        // below clears every rejection the store still holds.
        logger::debug(
            LogTag::Filtering,
            "No tokens with market data found - every stored rejection is cleared",
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

    match list_blacklisted_tokens_async(chain).await {
        Ok(entries) => {
            for entry in entries {
                let info = BlacklistReasonInfo {
                    category: "token".to_owned(),
                    reason: entry.reason,
                    detail: if entry.source.is_empty() {
                        None
                    } else {
                        Some(entry.source)
                    },
                };
                let reasons = blacklist_reasons_map.entry(entry.mint).or_default();
                if !reasons.contains(&info) {
                    reasons.push(info);
                }
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("failed_to_load_token_blacklist err={err}"),
            );
        }
    }

    match crate::pools::db::list_blacklisted_pools(chain, None).await {
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
                    Some(format!("pool={pool_id}"))
                } else {
                    program_id.map(|id| format!("program={id}"))
                };

                let info = BlacklistReasonInfo {
                    category: "pool".to_owned(),
                    reason,
                    detail,
                };

                let reasons = blacklist_reasons_map.entry(mint).or_default();
                if !reasons.contains(&info) {
                    reasons.push(info);
                }
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("failed_to_load_pool_blacklist err={err}"),
            );
        }
    }

    match crate::pools::db::list_blacklisted_accounts(chain, None).await {
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
                    Some(format!("pool={pool}, account={account_pubkey}"))
                } else {
                    Some(account_pubkey.clone())
                };

                let info = BlacklistReasonInfo {
                    category: "account".to_owned(),
                    reason,
                    detail: source
                        .as_ref()
                        .map(|src| format!("{account_pubkey} ({src})"))
                        .or(detail),
                };

                let reasons = blacklist_reasons_map.entry(mint).or_default();
                if !reasons.contains(&info) {
                    reasons.push(info);
                }
            }
        }
        Err(err) => {
            logger::warning(
                LogTag::Filtering,
                &format!("failed_to_load_account_blacklist err={err}"),
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
            let reasons = blacklist_reasons_map.entry(token.mint.clone()).or_default();
            let fallback = BlacklistReasonInfo {
                category: "token".to_owned(),
                reason: "database".to_owned(),
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

    let priced_set: HashSet<String> = crate::pools::get_available_tokens(chain)
        .into_iter()
        .collect();
    let open_position_set: HashSet<String> = positions::books::open_mints(positions::Book::Live)
        .await
        .into_iter()
        .collect();

    let ohlcv_set: HashSet<String> =
        match crate::ohlcvs::get_mints_with_data(chain, &candidate_mints).await {
            Ok(set) => set,
            Err(err) => {
                logger::warning(LogTag::Filtering, &format!("error={err}"));
                HashSet::new()
            }
        };

    let mut filtered_mints: Vec<String> = Vec::new();
    let mut rejected_mints: Vec<String> = Vec::new();
    let mut passed_tokens: Vec<PassedToken> = Vec::new();
    let mut rejected_tokens: Vec<RejectedToken> = Vec::new();
    let mut awaiting_rugcheck_report: Vec<(DateTime<Utc>, String)> = Vec::new();
    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(tokens.len());
    let mut stats = FilteringStats::default();

    let mut outcomes: Vec<FilterOutcome> = Vec::with_capacity(tokens.len());

    for mut token in tokens {
        stats.total_processed += 1;

        let has_pool_price = priced_set.contains(&token.mint);
        let has_open_position = open_position_set.contains(&token.mint);
        let has_ohlcv = ohlcv_set.contains(&token.mint);

        let creation_timestamp = token.created_at().at.timestamp();

        let mut passed_at = None;

        match profile.evaluate(&token, &config).await {
            Ok(()) => {
                filtered_mints.push(token.mint.clone());
                stats.passed += 1;

                // The loaded rejection columns are the previous build's write; this build
                // supersedes them.
                token.last_rejection_reason = None;
                token.last_rejection_source = None;
                token.last_rejection_at = None;
                outcomes.push(FilterOutcome {
                    mint: token.mint.clone(),
                    rejection: None,
                });

                let passed_time = previous
                    .and_then(|prev| prev.tokens.get(token.mint.as_str()))
                    .and_then(|entry| entry.passed_at)
                    .unwrap_or(evaluated_at);
                passed_at = Some(passed_time);

                passed_tokens.push(PassedToken {
                    chain,
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    passed_time,
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
            // Every earlier stage passed and only the report is missing: the token
            // waits for it, neither passed nor rejected, and carries no rejection row.
            // A blacklisted token (by its own entry, a pool or an account) is counted
            // but never queued for one of the few direct report fetches per cycle.
            Err(FilterRejectionReason::RugcheckDataMissing) => {
                stats.awaiting_rugcheck_report += 1;
                token.last_rejection_reason = None;
                token.last_rejection_source = None;
                token.last_rejection_at = None;
                outcomes.push(FilterOutcome {
                    mint: token.mint.clone(),
                    rejection: None,
                });
                if !token.is_blacklisted {
                    awaiting_rugcheck_report.push((token.first_discovered_at, token.mint.clone()));
                }
            }
            Err(reason) => {
                let reason_label = reason.label();
                let reason_source = reason.source().as_str().to_owned();

                stats.record_rejection(reason);
                rejected_mints.push(token.mint.clone());

                // A rejection keeps its start while the reason is unchanged and restarts
                // when the reason changes.
                let since = token
                    .last_rejection_at
                    .map(|at| at.timestamp())
                    .filter(|_| {
                        token.last_rejection_reason.as_deref() == Some(reason_label.as_str())
                    })
                    .unwrap_or(evaluated_at);
                token.last_rejection_reason = Some(reason_label.clone());
                token.last_rejection_source = Some(reason_source.clone());
                token.last_rejection_at = DateTime::from_timestamp(since, 0);

                rejected_tokens.push(RejectedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    reason: reason_label.clone(),
                    rejection_time: since,
                });
                outcomes.push(FilterOutcome {
                    mint: token.mint.clone(),
                    rejection: Some(FilterRejectionRecord {
                        reason: reason_label.clone(),
                        source: reason_source,
                        since,
                    }),
                });

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

        // The entry is built after the verdict, so it carries this build's outcome.
        let last_updated = token.market_data_last_fetched_at;
        let mint = token.mint.clone();
        token_entries.insert(
            mint,
            TokenEntry {
                token: Arc::new(token),
                has_pool_price,
                has_open_position,
                has_ohlcv,
                pair_created_at: Some(creation_timestamp),
                last_updated,
                passed_at,
            },
        );
    }

    // The build's database state is written before the snapshot exists, so a snapshot
    // never describes tokens the store has not yet been told about.
    match record_filter_build_async(chain, evaluated_at, outcomes).await {
        Ok(write) => {
            if write.cleared > 0 {
                logger::info(
                    LogTag::Filtering,
                    &format!(
                        "Filtering build {evaluated_at}: cleared {} rejections of tokens the build did not evaluate",
                        write.cleared
                    ),
                );
            }
        }
        Err(e) => logger::warning(
            LogTag::Filtering,
            &format!("Failed to record filtering build {evaluated_at}: {e}"),
        ),
    }

    if !filtered_mints.is_empty() {
        if let Err(e) = batch_update_priority_async(chain, filtered_mints.clone(), 60).await {
            logger::warning(
                LogTag::Filtering,
                &format!("Failed to batch update priorities: {e}"),
            );
        }
    }

    let elapsed_ms = start.elapsed().as_millis();

    let rejection_summary = stats.rejection_summary();
    logger::info(
        LogTag::Filtering,
        &format!(
            "processed={} passed={} rejected={} awaiting_rugcheck_report={} duration_ms={} rejection_summary={}",
            stats.total_processed,
            stats.passed,
            stats.rejected,
            stats.awaiting_rugcheck_report,
            elapsed_ms,
            rejection_summary
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
            "awaiting_rugcheck_report": stats.awaiting_rugcheck_report,
            "duration_ms": elapsed_ms as u64,
            "top_rejection_reasons": top_rejections,
        }),
    )
    .await;

    let snapshot = FilteringSnapshot {
        updated_at: Utc::now(),
        evaluated_at,
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
        awaiting_rugcheck_report: {
            // Newest first: the report backlog fetches them in this order.
            awaiting_rugcheck_report.sort_unstable_by(|a, b| b.cmp(a));
            awaiting_rugcheck_report
                .into_iter()
                .map(|(_, mint)| mint)
                .collect()
        },
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

    crate::tokens::store_filtered_results(chain, filtered_lists);

    Ok(snapshot)
}

#[derive(Default)]
struct FilteringStats {
    total_processed: usize,
    passed: usize,
    rejected: usize,
    awaiting_rugcheck_report: usize,
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
            return "-".to_owned();
        }

        let mut parts: Vec<(FilterRejectionReason, usize)> = self
            .rejection_counts
            .iter()
            .map(|(k, v)| (k.clone(), *v))
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
