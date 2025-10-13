use crate::global::is_debug_monitor_enabled;
/// Token monitoring system for periodic updates of cached tokens
/// Updates existing tokens based on liquidity priority and time constraints
use crate::logger::{log, LogTag};
use crate::tokens::{
    config::with_tokens_config,
    dexscreener::get_global_dexscreener_api,
    store::{get_global_token_store, TokenUpdateSource},
};
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Monitoring cycle duration in seconds (currently runs every 1 second)
const MONITOR_CYCLE_SECONDS: u64 = 1;

/// Minimum time between updates for a token (in minutes)
const MIN_UPDATE_INTERVAL_MINUTES: i64 = 30; // lowered from 60 to 30 to keep data fresher

/// Minimum staleness required to recheck a new token
const NEW_TOKEN_BOOST_MIN_STALE_MINUTES: i64 = 12; // ~12 minutes between touches
/// Hard cap of boosted tokens per cycle to avoid API pressure
const NEW_TOKEN_BOOST_PER_CYCLE: usize = 2;

fn tokens_per_cycle_limit() -> usize {
    let configured = with_tokens_config(|cfg| cfg.max_tokens_per_batch);
    configured.max(1)
}

fn api_batch_size_limit() -> usize {
    let configured = with_tokens_config(|cfg| cfg.max_tokens_per_api_call);
    configured.clamp(1, 30)
}

fn new_token_boost_max_age_minutes() -> i64 {
    let configured = with_tokens_config(|cfg| cfg.new_token_boost_max_age_minutes);
    configured.max(1)
}

fn max_update_interval_minutes() -> i64 {
    max_update_interval_hours_setting() * 60
}

fn max_update_interval_hours_setting() -> i64 {
    let configured = with_tokens_config(|cfg| cfg.max_update_interval_hours);
    configured.max(1)
}

// =============================================================================
// FAIRNESS / TIERING CONFIG
// =============================================================================

/// Liquidity tiers (USD) used to prevent starvation of small-liquidity tokens
/// High: >= 10k, Mid: 1k-10k, Low: 100-1k, Micro: < 100
const LIQ_TIER_HIGH_MIN: f64 = 10_000.0;
const LIQ_TIER_MID_MIN: f64 = 1_000.0;
const LIQ_TIER_LOW_MIN: f64 = 100.0;

/// Per-cycle quotas by tier (percentages of configured batch size)
/// We allocate by default: High 40%, Mid 30%, Low 20%, Micro 10%.
/// Any unused quota is reallocated oldest-first across all remaining tokens.
const QUOTA_HIGH_PCT: usize = 40;
const QUOTA_MID_PCT: usize = 30;
const QUOTA_LOW_PCT: usize = 20;
const QUOTA_MICRO_PCT: usize = 10;

// =============================================================================
// BATCH UPDATE RESULT
// =============================================================================

#[derive(Debug, Clone, Default)]
struct BatchUpdateResult {
    updated: usize,
    deleted: usize,
}

#[derive(Clone)]
struct UpdateCandidate {
    mint: String,
    liquidity: f64,
    age_minutes: i64,
    created_at: Option<DateTime<Utc>>,
}

// =============================================================================
// TOKEN MONITOR
// =============================================================================

pub struct TokenMonitor {
    cycle_counter: u64,
}

impl TokenMonitor {
    /// Create new token monitor instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self { cycle_counter: 0 })
    }

    /// Get tokens that need updating with fairness across liquidity tiers and age-first priority
    async fn get_tokens_for_update(&self) -> Result<Vec<String>, String> {
        let now = Utc::now();
        let store = get_global_token_store();
        let snapshots = store.all();

        if snapshots.is_empty() {
            return Ok(Vec::new());
        }

        let mut candidates: Vec<UpdateCandidate> = Vec::new();
        for snapshot in snapshots.into_iter() {
            let age_minutes = now
                .signed_duration_since(snapshot.data.last_updated)
                .num_minutes();
            if age_minutes < MIN_UPDATE_INTERVAL_MINUTES {
                continue;
            }

            let liquidity = snapshot.liquidity_usd().unwrap_or(0.0);
            candidates.push(UpdateCandidate {
                mint: snapshot.data.mint.clone(),
                liquidity,
                age_minutes,
                created_at: snapshot.data.created_at,
            });
        }

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        // Config-driven limits
        let tokens_per_cycle = tokens_per_cycle_limit();
        let max_interval_minutes = max_update_interval_minutes();
        let boost_age_limit = new_token_boost_max_age_minutes();

        let mut selected_tokens: Vec<String> = Vec::new();
        let mut selected_set: HashSet<String> = HashSet::new();

        // Prefer very new tokens with stale data
        if NEW_TOKEN_BOOST_PER_CYCLE > 0 {
            let mut boost_candidates: Vec<&UpdateCandidate> = candidates
                .iter()
                .filter(|candidate| {
                    candidate
                        .created_at
                        .map(|created| {
                            let age_since_creation =
                                now.signed_duration_since(created).num_minutes();
                            age_since_creation <= boost_age_limit
                                && candidate.age_minutes >= NEW_TOKEN_BOOST_MIN_STALE_MINUTES
                        })
                        .unwrap_or(false)
                })
                .collect();

            boost_candidates.sort_by(|a, b| {
                let a_created = a.created_at.unwrap();
                let b_created = b.created_at.unwrap();
                b_created.cmp(&a_created)
            });

            for candidate in boost_candidates
                .into_iter()
                .take(NEW_TOKEN_BOOST_PER_CYCLE.min(tokens_per_cycle))
            {
                if selected_set.insert(candidate.mint.clone()) {
                    selected_tokens.push(candidate.mint.clone());
                    if selected_tokens.len() >= tokens_per_cycle {
                        return Ok(selected_tokens);
                    }
                }
            }
        }

        let mut remaining: Vec<UpdateCandidate> = candidates
            .into_iter()
            .filter(|candidate| !selected_set.contains(&candidate.mint))
            .collect();

        // Force include tokens that are far beyond the maximum allowed interval
        if max_interval_minutes > 0 {
            let mut forced: Vec<UpdateCandidate> = Vec::new();
            remaining.retain(|candidate| {
                if candidate.age_minutes >= max_interval_minutes {
                    forced.push(candidate.clone());
                    false
                } else {
                    true
                }
            });

            forced.sort_by(|a, b| match b.age_minutes.cmp(&a.age_minutes) {
                std::cmp::Ordering::Equal => b
                    .liquidity
                    .partial_cmp(&a.liquidity)
                    .unwrap_or(std::cmp::Ordering::Equal),
                other => other,
            });

            for candidate in forced {
                if selected_tokens.len() >= tokens_per_cycle {
                    return Ok(selected_tokens);
                }
                if selected_set.insert(candidate.mint.clone()) {
                    selected_tokens.push(candidate.mint);
                }
            }
        }

        let mut high: Vec<UpdateCandidate> = Vec::new();
        let mut mid: Vec<UpdateCandidate> = Vec::new();
        let mut low: Vec<UpdateCandidate> = Vec::new();
        let mut micro: Vec<UpdateCandidate> = Vec::new();

        for candidate in remaining.into_iter() {
            if candidate.liquidity >= LIQ_TIER_HIGH_MIN {
                high.push(candidate);
            } else if candidate.liquidity >= LIQ_TIER_MID_MIN {
                mid.push(candidate);
            } else if candidate.liquidity >= LIQ_TIER_LOW_MIN {
                low.push(candidate);
            } else {
                micro.push(candidate);
            }
        }

        let by_age_then_liq =
            |a: &UpdateCandidate, b: &UpdateCandidate| match b.age_minutes.cmp(&a.age_minutes) {
                std::cmp::Ordering::Equal => b
                    .liquidity
                    .partial_cmp(&a.liquidity)
                    .unwrap_or(std::cmp::Ordering::Equal),
                other => other,
            };

        high.sort_by(by_age_then_liq);
        mid.sort_by(by_age_then_liq);
        low.sort_by(by_age_then_liq);
        micro.sort_by(by_age_then_liq);

        let capacity = tokens_per_cycle.saturating_sub(selected_tokens.len());
        if capacity == 0 {
            return Ok(selected_tokens);
        }

        let quota = |pct: usize| -> usize { (capacity * pct) / 100 };
        let mut q_high = quota(QUOTA_HIGH_PCT).max(1);
        let mut q_mid = quota(QUOTA_MID_PCT).max(1);
        let mut q_low = quota(QUOTA_LOW_PCT).max(1);
        let mut q_micro = quota(QUOTA_MICRO_PCT).max(1);

        let mut total_q = q_high + q_mid + q_low + q_micro;
        while total_q > tokens_per_cycle {
            if q_micro > 1 {
                q_micro -= 1;
            } else if q_low > 1 {
                q_low -= 1;
            } else if q_mid > 1 {
                q_mid -= 1;
            } else if q_high > 1 {
                q_high -= 1;
            }
            total_q = q_high + q_mid + q_low + q_micro;
        }

        let mut take_from_bucket = |bucket: &mut Vec<UpdateCandidate>, max_take: usize| {
            if selected_tokens.len() >= tokens_per_cycle {
                return;
            }
            let remaining_capacity = tokens_per_cycle.saturating_sub(selected_tokens.len());
            if remaining_capacity == 0 {
                return;
            }
            let take_n = max_take.min(bucket.len()).min(remaining_capacity);
            for candidate in bucket.drain(..take_n) {
                if selected_set.insert(candidate.mint.clone()) {
                    selected_tokens.push(candidate.mint);
                }
            }
        };

        take_from_bucket(&mut high, q_high);
        take_from_bucket(&mut mid, q_mid);
        take_from_bucket(&mut low, q_low);
        take_from_bucket(&mut micro, q_micro);

        if selected_tokens.len() < tokens_per_cycle {
            let mut fallback: Vec<UpdateCandidate> = Vec::new();
            fallback.extend(high.into_iter());
            fallback.extend(mid.into_iter());
            fallback.extend(low.into_iter());
            fallback.extend(micro.into_iter());
            fallback.sort_by(by_age_then_liq);

            for candidate in fallback {
                if selected_tokens.len() >= tokens_per_cycle {
                    break;
                }
                if selected_set.insert(candidate.mint.clone()) {
                    selected_tokens.push(candidate.mint);
                }
            }
        }

        Ok(selected_tokens)
    }

    /// Update a batch of tokens with fresh data from DexScreener
    async fn update_token_batch(&mut self, mints: &[String]) -> Result<BatchUpdateResult, String> {
        if mints.is_empty() {
            return Ok(BatchUpdateResult::default());
        }

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "UPDATE",
                &format!("Updating {} tokens with fresh data", mints.len()),
            );
        }

        // Get fresh token information from DexScreener API
        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "API_REQUEST",
                &format!(
                    "Requesting token data from DexScreener API for {} tokens",
                    mints.len()
                ),
            );
        }

        let tokens_result = {
            let api = get_global_dexscreener_api()
                .await
                .map_err(|e| format!("Failed to get global API client: {}", e))?;
            let mut api_instance = api.lock().await;
            api_instance.get_tokens_info(mints).await
        };

        match tokens_result {
            Ok(tokens) => {
                // Track which tokens were returned by the API
                let returned_mints: std::collections::HashSet<String> =
                    tokens.iter().map(|t| t.mint.clone()).collect();

                // Find tokens that were requested but not returned (no longer exist)
                let missing_mints: Vec<String> = mints
                    .iter()
                    .filter(|mint| !returned_mints.contains(*mint))
                    .cloned()
                    .collect();

                // Build metadata map for detailed logging before moving tokens
                let token_metadata: HashMap<String, (String, f64)> = tokens
                    .iter()
                    .map(|t| {
                        let liquidity_usd = t.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
                        (t.mint.clone(), (t.symbol.clone(), liquidity_usd))
                    })
                    .collect();

                let mut total_updated = 0;
                let mut fetched_tokens = tokens;

                // Enforce configured price deviation limits before persisting updates
                let deviation_limit =
                    with_tokens_config(|cfg| cfg.max_price_deviation_percent).max(0.0);
                let mut skipped_for_deviation: Vec<(String, f64, f64, f64)> = Vec::new();

                if deviation_limit > 0.0 {
                    let store = get_global_token_store();
                    let existing_snapshots = store.get_many(mints);
                    let mut existing_prices: HashMap<String, f64> =
                        HashMap::with_capacity(existing_snapshots.len());

                    for snapshot in existing_snapshots {
                        if let Some(price) = snapshot.data.price_dexscreener_sol {
                            if price > 0.0 {
                                existing_prices.insert(snapshot.data.mint.clone(), price);
                            }
                        }
                    }

                    if !existing_prices.is_empty() {
                        // Use f64 precision limit as epsilon (truly zero vs micro-cap prices)
                        const PRICE_EPSILON: f64 = 1e-15;

                        fetched_tokens.retain(|token| {
                            // Get new price from API response
                            let new_price = token.price_dexscreener_sol.unwrap_or(0.0);

                            // Get old price from cache (0.0 if not found)
                            let old_price =
                                existing_prices.get(&token.mint).copied().unwrap_or(0.0);

                            // Handle truly zero-price cases (stale/missing data)
                            if new_price.abs() < PRICE_EPSILON && old_price.abs() < PRICE_EPSILON {
                                // Both zero - no price data available, allow through
                                return true;
                            }

                            if new_price.abs() < PRICE_EPSILON {
                                // New price is zero but old wasn't - stale API data
                                skipped_for_deviation.push((
                                    token.mint.clone(),
                                    100.0, // Mark as 100% for logging
                                    old_price,
                                    new_price,
                                ));
                                return false;
                            }

                            if old_price.abs() < PRICE_EPSILON {
                                // No cached price to compare - this is first price, allow through
                                return true;
                            }

                            // Calculate deviation between valid prices (including micro-cap)
                            // Note: Micro-cap tokens (0.000000001 SOL) can have extreme volatility
                            let deviation =
                                ((new_price - old_price).abs() / old_price.abs()) * 100.0;

                            if deviation > deviation_limit {
                                skipped_for_deviation.push((
                                    token.mint.clone(),
                                    deviation,
                                    old_price,
                                    new_price,
                                ));
                                return false;
                            }

                            true
                        });
                    }
                }

                if !skipped_for_deviation.is_empty() {
                    // Helper function to format price with appropriate precision
                    let format_price = |price: f64| -> String {
                        if price.abs() < 1e-15 {
                            // Truly zero (below f64 precision)
                            "0".to_string()
                        } else if price.abs() < 1e-6 {
                            // Micro-cap: use scientific notation for clarity
                            format!("{:.2e}", price)
                        } else if price.abs() < 0.01 {
                            // Small price: use 9 decimals
                            format!("{:.9}", price)
                        } else {
                            // Normal price: use 6 decimals
                            format!("{:.6}", price)
                        }
                    };

                    // Log first 3 with full details
                    let preview: Vec<String> =
                        skipped_for_deviation
                            .iter()
                            .take(3)
                            .map(|(mint, deviation, old_price, new_price)| {
                                let (symbol, liquidity_usd) = token_metadata
                                    .get(mint)
                                    .map(|(s, l)| (s.as_str(), *l))
                                    .unwrap_or(("?", 0.0));

                                const EPSILON: f64 = 1e-15; // Below this is truly zero
                                let is_stale = new_price.abs() < EPSILON;

                                if is_stale {
                                    format!(
                                    "{} ({}) old={}→new=0 SOL (stale API data) liquidity=${:.0}",
                                    mint, symbol, format_price(*old_price), liquidity_usd
                                )
                                } else {
                                    format!(
                                        "{} ({}) dev={:.1}% ({}→{} SOL) liquidity=${:.0}",
                                        mint,
                                        symbol,
                                        deviation,
                                        format_price(*old_price),
                                        format_price(*new_price),
                                        liquidity_usd
                                    )
                                }
                            })
                            .collect();

                    // Compute summary stats for remaining tokens
                    let extras = skipped_for_deviation.len().saturating_sub(3);
                    let summary = if extras > 0 {
                        const EPSILON: f64 = 1e-15; // Truly zero threshold
                        const MICROCAP_THRESHOLD: f64 = 1e-6; // Below 0.000001 SOL

                        let stale_count = skipped_for_deviation
                            .iter()
                            .skip(3)
                            .filter(|(_, _, _, new)| new.abs() < EPSILON)
                            .count();

                        let microcap_count = skipped_for_deviation
                            .iter()
                            .skip(3)
                            .filter(|(_, _, old, new)| {
                                new.abs() >= EPSILON
                                    && (old.abs() < MICROCAP_THRESHOLD
                                        || new.abs() < MICROCAP_THRESHOLD)
                            })
                            .count();

                        // Calculate average for all non-stale tokens
                        let real_deviations: Vec<f64> = skipped_for_deviation
                            .iter()
                            .skip(3)
                            .filter_map(|(_, dev, _, new)| {
                                if new.abs() >= EPSILON {
                                    Some(*dev)
                                } else {
                                    None
                                }
                            })
                            .collect();

                        let avg_deviation_str = if !real_deviations.is_empty() {
                            let avg: f64 =
                                real_deviations.iter().sum::<f64>() / real_deviations.len() as f64;
                            format!("avg_deviation={:.1}%", avg)
                        } else {
                            "all_stale".to_string()
                        };

                        format!(
                            " (+{} more: {}, stale={}, microcap={}/{}, check Pool Service health)",
                            extras, avg_deviation_str, stale_count, microcap_count, extras
                        )
                    } else {
                        String::new()
                    };

                    let detail = format!("{}{}", preview.join("; "), summary);

                    log(
                        LogTag::Monitor,
                        "PRICE_DEVIATION_SKIP",
                        &format!(
                            "Skipped {} tokens due to price deviation > {:.2}% threshold. Details: {}",
                            skipped_for_deviation.len(),
                            deviation_limit,
                            detail
                        ),
                    );
                }

                // Update tokens that were returned by the API
                if !fetched_tokens.is_empty() {
                    if is_debug_monitor_enabled() {
                        log(
                            LogTag::Monitor,
                            "API_RESULT",
                            &format!(
                                "API returned {} tokens out of {} requested ({} accepted)",
                                returned_mints.len(),
                                mints.len(),
                                fetched_tokens.len()
                            ),
                        );
                    }

                    let store = get_global_token_store();
                    match store
                        .ingest_tokens(fetched_tokens.clone(), TokenUpdateSource::Monitor)
                        .await
                    {
                        Ok(stats) => {
                            total_updated += stats.total_processed;
                            if is_debug_monitor_enabled() {
                                log(
                                    LogTag::Monitor,
                                    "TOKEN_STORE_INGEST",
                                    &format!(
                                        "Ingested {} tokens (inserted={}, updated={}, removed={})",
                                        stats.total_processed,
                                        stats.inserted,
                                        stats.updated,
                                        stats.removed
                                    ),
                                );
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Monitor,
                                "ERROR",
                                &format!("Failed to ingest tokens into store: {}", e),
                            );
                            return Err(format!("Token store ingest failed: {}", e));
                        }
                    }
                }

                // Delete tokens that were not returned by the API (no longer exist)
                // SAFETY: Filter out tokens with open positions before deletion
                let mut deleted_count = 0;
                if !missing_mints.is_empty() {
                    // Check which tokens are safe to delete (no open positions)
                    let mut safe_to_delete = Vec::new();
                    let mut protected_tokens = Vec::new();

                    for mint in &missing_mints {
                        if crate::positions::is_open_position(mint).await {
                            protected_tokens.push(mint.clone());
                        } else {
                            safe_to_delete.push(mint.clone());
                        }
                    }

                    if !protected_tokens.is_empty() {
                        if is_debug_monitor_enabled() {
                            log(
                                LogTag::Monitor,
                                "SAFETY_PROTECTION",
                                &format!(
                                    "🛡️  Protected {} tokens from deletion due to open positions: {:?}",
                                    protected_tokens.len(),
                                    protected_tokens
                                )
                            );
                        }
                    }

                    if !safe_to_delete.is_empty() {
                        if is_debug_monitor_enabled() {
                            log(
                                LogTag::Monitor,
                                "CLEANUP",
                                &format!(
                                    "Removing {} stale tokens that no longer exist on DexScreener: {:?}",
                                    safe_to_delete.len(),
                                    safe_to_delete
                                )
                            );
                        }

                        let store = get_global_token_store();
                        let mut removed_this_batch = 0;

                        for mint in &safe_to_delete {
                            match store.remove_token(mint, TokenUpdateSource::Monitor).await {
                                Ok(Some(_)) => {
                                    removed_this_batch += 1;
                                }
                                Ok(None) => {
                                    // Token already absent; nothing to do
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Monitor,
                                        "ERROR",
                                        &format!("Failed to remove stale token {}: {}", mint, e),
                                    );
                                }
                            }
                        }

                        if removed_this_batch > 0 {
                            deleted_count += removed_this_batch;
                            if is_debug_monitor_enabled() {
                                log(
                                    LogTag::Monitor,
                                    "CLEANUP_SUCCESS",
                                    &format!(
                                        "Deleted {} stale tokens from store/database",
                                        removed_this_batch
                                    ),
                                );
                            }
                        }
                    }
                }

                // Return counts of successfully updated and deleted tokens
                Ok(BatchUpdateResult {
                    updated: total_updated,
                    deleted: deleted_count,
                })
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to get token info from API: {}", e),
                );
                Err(format!("API request failed: {}", e))
            }
        }
    }

    /// Main monitoring cycle - update random tokens based on priority
    async fn run_monitoring_cycle(&mut self) -> Result<(), String> {
        // Mark cycle start in stats and initialize 30s interval if needed
        let cycle_started = Utc::now();
        {
            let stats_handle = get_monitor_stats_handle();
            let mut stats = stats_handle.write().await;
            stats.total_cycles += 1;
            stats.last_cycle_started = Some(cycle_started);
            if stats.interval_started.is_none() {
                stats.interval_started = Some(cycle_started);
            }
        }

        // Get tokens that need updating (we will print a summary regardless of count)
        let tokens_to_update = self.get_tokens_for_update().await?;

        // Compute tier breakdown for selection (fetch liquidity for selected mints)
        let mut selected_tiers = TierCounts::default();
        if !tokens_to_update.is_empty() {
            let store = get_global_token_store();
            let snapshots = store.get_many(&tokens_to_update);
            for snapshot in snapshots {
                let liq = snapshot.liquidity_usd().unwrap_or(0.0);
                selected_tiers.add_liquidity(liq);
            }
        }

        let mut total_updated = 0;
        let mut total_deleted = 0;
        let mut batches_ok = 0usize;
        let mut batches_failed = 0usize;

        let max_batch_size = api_batch_size_limit();

        // Process tokens in API-compatible batches
        for batch in tokens_to_update.chunks(max_batch_size) {
            match self.update_token_batch(batch).await {
                Ok(result) => {
                    total_updated += result.updated;
                    total_deleted += result.deleted;
                    batches_ok += 1;
                }
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "BATCH_ERROR",
                        &format!("Batch update failed: {}", e),
                    );
                    // Continue with next batch even if one fails
                    batches_failed += 1;
                }
            }

            // Small delay between batches to respect rate limits
            if batch.len() == max_batch_size {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }

        // Update stats and aggregate interval (no per-cycle summary)
        {
            let stats_handle = get_monitor_stats_handle();
            let mut stats = stats_handle.write().await;
            stats.last_cycle_selected = tokens_to_update.len();
            stats.last_cycle_updated = total_updated;
            stats.last_cycle_deleted = total_deleted;
            stats.last_cycle_batches_ok = batches_ok;
            stats.last_cycle_batches_failed = batches_failed;
            stats.last_cycle_tiers = selected_tiers.clone();
            stats.total_updated += total_updated as u64;
            stats.total_deleted += total_deleted as u64;
            let cycle_completed = Utc::now();
            stats.last_cycle_completed = Some(cycle_completed);

            // Aggregate into the current ~30s interval
            stats.interval_cycles += 1;
            stats.interval_selected += tokens_to_update.len();
            stats.interval_updated += total_updated;
            stats.interval_deleted += total_deleted;
            stats.interval_batches_ok += batches_ok;
            stats.interval_batches_failed += batches_failed;
            stats.interval_tiers.high += selected_tiers.high;
            stats.interval_tiers.mid += selected_tiers.mid;
            stats.interval_tiers.low += selected_tiers.low;
            stats.interval_tiers.micro += selected_tiers.micro;
            let dur_ms = (cycle_completed - cycle_started).num_milliseconds().max(0) as u128;
            stats.interval_duration_ms_sum += dur_ms;
        }

        // Print one styled summary at the end of each ~30s window and reset interval
        let should_print = {
            let stats_handle = get_monitor_stats_handle();
            let stats = stats_handle.read().await;
            if let Some(start) = stats.interval_started {
                (Utc::now() - start).num_seconds() >= 30
            } else {
                false
            }
        };

        if should_print {
            // Compute backlog snapshot once per summary to keep overhead low
            let (over1h, over2h, over7d) = if is_debug_monitor_enabled() {
                let now = Utc::now();
                let cutoff_1h = now - chrono::Duration::hours(1);
                let cutoff_2h = now - chrono::Duration::hours(2);
                let cutoff_7d = now - chrono::Duration::hours(24 * 7);
                let store = get_global_token_store();
                let snapshots = store.all();

                let mut count_1h = 0usize;
                let mut count_2h = 0usize;
                let mut count_7d = 0usize;

                for snapshot in snapshots {
                    let last_updated = snapshot.data.last_updated;
                    if last_updated < cutoff_1h {
                        count_1h += 1;
                    }
                    if last_updated < cutoff_2h {
                        count_2h += 1;
                    }
                    if last_updated < cutoff_7d {
                        count_7d += 1;
                    }
                }

                (count_1h, count_2h, count_7d)
            } else {
                (0, 0, 0)
            };

            {
                let stats_handle = get_monitor_stats_handle();
                let mut stats = stats_handle.write().await;
                stats.backlog_over_1h = over1h;
                stats.backlog_over_2h = over2h;
                stats.backlog_over_7d = over7d;
            }

            print_monitor_interval_summary().await;

            // Reset interval
            {
                let stats_handle = get_monitor_stats_handle();
                let mut stats = stats_handle.write().await;
                stats.interval_started = Some(Utc::now());
                stats.interval_cycles = 0;
                stats.interval_selected = 0;
                stats.interval_updated = 0;
                stats.interval_deleted = 0;
                stats.interval_batches_ok = 0;
                stats.interval_batches_failed = 0;
                stats.interval_tiers = TierCounts::default();
                stats.interval_duration_ms_sum = 0;
            }
        }

        Ok(())
    }

    /// Start continuous monitoring loop in background
    pub async fn start_monitoring_loop(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        if is_debug_monitor_enabled() {
            log(LogTag::Monitor, "INIT", "Token monitoring loop started");
        }

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    if is_debug_monitor_enabled() {
                        log(LogTag::Monitor, "SHUTDOWN", "Token monitoring loop stopping");
                    }
                    break;
                }
                _ = sleep(Duration::from_secs(MONITOR_CYCLE_SECONDS)) => {
                    self.cycle_counter += 1;

                    if is_debug_monitor_enabled() {
                        log(LogTag::Monitor, "CYCLE", &format!("Starting monitoring cycle #{}", self.cycle_counter));
                    }

                    // Run normal monitoring cycle
                    if let Err(e) = self.run_monitoring_cycle().await {
                        log(
                            LogTag::Monitor,
                            "CYCLE_ERROR",
                            &format!("Monitoring cycle failed: {}", e)
                        );
                    }
                }
            }
        }

        if is_debug_monitor_enabled() {
            log(LogTag::Monitor, "STOP", "Token monitoring loop stopped");
        }
    }
}

// =============================================================================
// PUBLIC INTERFACE
// =============================================================================

/// Start token monitoring background task
pub async fn start_token_monitoring(
    shutdown: Arc<tokio::sync::Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<tokio::task::JoinHandle<()>, String> {
    if is_debug_monitor_enabled() {
        log(
            LogTag::Monitor,
            "START",
            "Starting token monitoring background task (instrumented)",
        );
    }

    let handle = tokio::spawn(monitor.instrument(async move {
        let mut monitor = match TokenMonitor::new() {
            Ok(monitor) => {
                if is_debug_monitor_enabled() {
                    log(
                        LogTag::Monitor,
                        "INIT",
                        "Token monitor instance created successfully",
                    );
                }
                monitor
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to initialize token monitor: {}", e),
                );
                return;
            }
        };

        // Wait for Transactions system to be ready before starting monitoring
        let mut last_log = std::time::Instant::now();
        loop {
            let tx_ready =
                crate::global::TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst);

            if tx_ready {
                if is_debug_monitor_enabled() {
                    log(
                        LogTag::Monitor,
                        "READY",
                        "✅ Transactions ready. Starting token monitoring loop",
                    );
                }
                break;
            }

            // Log only every 15 seconds
            if last_log.elapsed() >= std::time::Duration::from_secs(15) {
                if is_debug_monitor_enabled() {
                    log(
                        LogTag::Monitor,
                        "READY",
                        "⏳ Waiting for Transactions system to be ready...",
                    );
                }
                last_log = std::time::Instant::now();
            }

            tokio::select! {
                _ = shutdown.notified() => {
                    if is_debug_monitor_enabled() {
                        log(LogTag::Monitor, "EXIT", "Token monitoring exiting during dependency wait");
                    }
                    return;
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {}
            }
        }

        monitor.start_monitoring_loop(shutdown).await;
        if is_debug_monitor_enabled() {
            log(LogTag::Monitor, "EXIT", "Token monitoring task ended");
        }
    }));

    Ok(handle)
}

/// Manual monitoring cycle for testing
pub async fn run_monitoring_cycle_once() -> Result<(), String> {
    let mut monitor =
        TokenMonitor::new().map_err(|e| format!("Failed to create monitor: {}", e))?;
    monitor.run_monitoring_cycle().await
}

// =============================================================================
// MONITOR STATS & SUMMARY (similar to discovery.rs)
// =============================================================================

#[derive(Debug, Clone, Default)]
struct MonitorStats {
    total_cycles: u64,
    total_updated: u64,
    total_deleted: u64,
    last_cycle_started: Option<DateTime<Utc>>,
    last_cycle_completed: Option<DateTime<Utc>>,
    last_cycle_selected: usize,
    last_cycle_updated: usize,
    last_cycle_deleted: usize,
    last_cycle_batches_ok: usize,
    last_cycle_batches_failed: usize,
    last_cycle_tiers: TierCounts,
    backlog_over_1h: usize,
    backlog_over_2h: usize,
    backlog_over_7d: usize, // Count tokens older than 7 days
    last_error: Option<String>,

    // 30-second interval aggregation
    interval_started: Option<DateTime<Utc>>,
    interval_cycles: u64,
    interval_selected: usize,
    interval_updated: usize,
    interval_deleted: usize,
    interval_batches_ok: usize,
    interval_batches_failed: usize,
    interval_tiers: TierCounts,
    interval_duration_ms_sum: u128,
}

#[derive(Debug, Clone, Default)]
struct TierCounts {
    high: usize,
    mid: usize,
    low: usize,
    micro: usize,
}

impl TierCounts {
    fn add_liquidity(&mut self, liq: f64) {
        if liq >= LIQ_TIER_HIGH_MIN {
            self.high += 1;
        } else if liq >= LIQ_TIER_MID_MIN {
            self.mid += 1;
        } else if liq >= LIQ_TIER_LOW_MIN {
            self.low += 1;
        } else {
            self.micro += 1;
        }
    }
}

static MONITOR_STATS: OnceLock<Arc<RwLock<MonitorStats>>> = OnceLock::new();

fn get_monitor_stats_handle() -> Arc<RwLock<MonitorStats>> {
    MONITOR_STATS
        .get_or_init(|| Arc::new(RwLock::new(MonitorStats::default())))
        .clone()
}

/// Public snapshot for dashboards or tooling
pub async fn get_monitor_stats() -> MonitorStats {
    let handle = get_monitor_stats_handle();
    let guard = handle.read().await;
    guard.clone()
}

/// Single comprehensive summary log per ~30s interval
async fn print_monitor_interval_summary() {
    let stats = get_monitor_stats().await;

    // Emoji based on effectiveness
    let emoji = if stats.interval_updated > 0 {
        "✅"
    } else {
        "⏸️"
    };

    // Average duration per cycle
    let avg_ms = if stats.interval_cycles > 0 {
        stats.interval_duration_ms_sum / (stats.interval_cycles as u128)
    } else {
        0
    };

    let header_line = "════════════════════════════════════════════════════════════";
    let title = format!("{} MONITOR SUMMARY (last ~30s)", emoji);
    let cycles_line = format!("  • Cycles    🔁  {}", stats.interval_cycles);
    let selected_line = format!(
        "  • Selected  🧩  {}  (H:{}  |  M:{}  |  L:{}  |  m:{})",
        stats.interval_selected,
        stats.interval_tiers.high,
        stats.interval_tiers.mid,
        stats.interval_tiers.low,
        stats.interval_tiers.micro
    );
    let updated_line = format!(
        "  • Updated   🔄  {}  |  Deleted 🗑️  {}  |  Batches ✅/❌  {} / {}",
        stats.interval_updated,
        stats.interval_deleted,
        stats.interval_batches_ok,
        stats.interval_batches_failed
    );
    let timing_line = format!("  • Avg cycle 🕒  {} ms", avg_ms);

    let backlog_info =
        if stats.backlog_over_1h > 0 || stats.backlog_over_2h > 0 || stats.backlog_over_7d > 0 {
            let mut parts = Vec::new();
            if stats.backlog_over_1h > 0 {
                parts.push(format!(">=1h: {}", stats.backlog_over_1h));
            }
            if stats.backlog_over_2h > 0 {
                parts.push(format!(">=2h: {}", stats.backlog_over_2h));
            }
            if stats.backlog_over_7d > 0 {
                parts.push(format!(">=7d: {}", stats.backlog_over_7d));
            }
            format!("\n  • Backlog  ⏱️  {}", parts.join("  |  "))
        } else {
            String::new()
        };

    let body = format!(
        "\n{header}\n{title}\n{header}\n{cycles}\n{selected}\n{updated}\n{timing}{backlog}\n{header}",
        header = header_line,
        title = title,
        cycles = cycles_line,
        selected = selected_line,
        updated = updated_line,
        timing = timing_line,
        backlog = backlog_info
    );

    if is_debug_monitor_enabled() {
        log(LogTag::Monitor, "SUMMARY", &body);
    }
}
