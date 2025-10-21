use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant as StdInstant;

use chrono::Utc;
use futures::stream::{self, StreamExt};

use crate::config::FilteringConfig;
use crate::global::is_debug_filtering_enabled;
use crate::logger::{log, LogTag};
use crate::positions;
use crate::tokens::types::Token;
use crate::tokens::{get_full_token_async, list_tokens_async};

use super::sources::{self, FilterRejectionReason};
use super::types::{
    FilteringSnapshot, PassedToken, RejectedToken, TokenEntry, MAX_DECISION_HISTORY,
};

const TOKEN_FETCH_CONCURRENCY: usize = 24;

pub async fn compute_snapshot(config: FilteringConfig) -> Result<FilteringSnapshot, String> {
    let debug_enabled = is_debug_filtering_enabled();
    let start = StdInstant::now();

    let max_candidates = config.max_tokens_to_process.max(100);
    let fetch_limit = if config.target_filtered_tokens > 0 {
        max_candidates.max(config.target_filtered_tokens)
    } else {
        max_candidates
    };

    let metadata = list_tokens_async(fetch_limit)
        .await
        .map_err(|e| format!("Failed to list tokens: {}", e))?;

    if metadata.is_empty() {
        if debug_enabled {
            log(
                LogTag::Filtering,
                "SNAPSHOT_EMPTY",
                "Token store empty - snapshot remains empty",
            );
        }
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
                        log(
                            LogTag::Filtering,
                            "TOKEN_LOAD_ERROR",
                            &format!("mint={} error={}", mint, err),
                        );
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
        if debug_enabled {
            log(
                LogTag::Filtering,
                "SNAPSHOT_NO_TOKENS",
                &format!(
                    "Unable to load full tokens for any candidates (total_candidates={})",
                    total_candidates
                ),
            );
        }
        return Ok(FilteringSnapshot::empty());
    }

    let mut tokens_sorted = tokens_with_index;
    tokens_sorted.sort_by_key(|(index, _)| *index);
    let tokens: Vec<Token> = tokens_sorted.into_iter().map(|(_, token)| token).collect();

    let candidate_mints: Vec<String> = tokens.iter().map(|t| t.mint.clone()).collect();

    let priced_set: HashSet<String> = crate::pools::get_available_tokens().into_iter().collect();
    let open_position_set: HashSet<String> =
        positions::get_open_mints().await.into_iter().collect();

    let ohlcv_set: HashSet<String> =
        match crate::ohlcvs::get_mints_with_data(&candidate_mints).await {
            Ok(set) => set,
            Err(err) => {
                log(
                    LogTag::Filtering,
                    "OHLCV_LOOKUP_FAILED",
                    &format!("error={}", err),
                );
                HashSet::new()
            }
        };

    let mut filtered_mints: Vec<String> = Vec::new();
    let mut rejected_mints: Vec<String> = Vec::new();
    let mut passed_tokens: VecDeque<PassedToken> = VecDeque::new();
    let mut rejected_tokens: VecDeque<RejectedToken> = VecDeque::new();
    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(tokens.len());
    let mut stats = FilteringStats::default();

    for token in tokens.iter().take(config.max_tokens_to_process) {
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
            }
        }

        if config.target_filtered_tokens > 0
            && filtered_mints.len() >= config.target_filtered_tokens
        {
            stats.target_reached = true;
            break;
        }
    }

    let elapsed_ms = start.elapsed().as_millis();

    if debug_enabled {
        let rejection_summary = stats.rejection_summary();
        log(
            LogTag::Filtering,
            "REFRESH_COMPLETE",
            &format!(
                "processed={} passed={} rejected={} target_reached={} duration_ms={} rejection_summary={}",
                stats.total_processed,
                stats.passed,
                stats.rejected,
                stats.target_reached,
                elapsed_ms,
                rejection_summary
            ),
        );
    }

    let snapshot = FilteringSnapshot {
        updated_at: Utc::now(),
        filtered_mints,
        passed_tokens: passed_tokens.into_iter().collect(),
        rejected_mints,
        rejected_tokens: rejected_tokens.into_iter().collect(),
        tokens: token_entries,
    };

    // Store filtered results in tokens module for consumption by other services
    let filtered_lists = crate::tokens::FilteredTokenLists {
        passed: snapshot.filtered_mints.clone(),
        rejected: snapshot.rejected_mints.clone(),
        blacklisted: snapshot
            .tokens
            .values()
            .filter(|e| e.token.is_blacklisted)
            .map(|e| e.token.mint.clone())
            .collect(),
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
    sources::dexscreener::evaluate(token, &config.dexscreener)?;
    sources::rugcheck::evaluate(token, &config.rugcheck)?;

    Ok(())
}

#[derive(Default)]
struct FilteringStats {
    total_processed: usize,
    passed: usize,
    rejected: usize,
    target_reached: bool,
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
