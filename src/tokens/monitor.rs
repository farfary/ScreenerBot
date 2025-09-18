use crate::global::is_debug_monitor_enabled;
/// Token monitoring system for periodic updates of database tokens
/// Updates existing tokens based on liquidity priority and time constraints
use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::dexscreener::get_global_dexscreener_api;
use chrono::{ DateTime, Utc };
use futures::TryFutureExt;
use rand::seq::SliceRandom;
use std::collections::{ HashMap, HashSet };
use std::sync::{ Arc, OnceLock };
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Monitoring cycle duration in seconds (currently runs every 1 second)
const MONITOR_CYCLE_SECONDS: u64 = 1;

/// Minimum time between updates for a token (in minutes)
const MIN_UPDATE_INTERVAL_MINUTES: i64 = 30; // lowered from 60 to 30 to keep data fresher

/// Maximum time before forced update (2 hours)
const MAX_UPDATE_INTERVAL_HOURS: i64 = 2;

/// Number of tokens to update per cycle (adjust based on performance)
const TOKENS_PER_CYCLE: usize = 30;

/// Batch size for API calls (DexScreener limit)
const API_BATCH_SIZE: usize = 30;

// =============================================================================
// NEW TOKEN BOOST (lightweight)
// =============================================================================

/// Consider tokens "new" for this many minutes from their pair creation time
const NEW_TOKEN_BOOST_MAX_AGE_MINUTES: i64 = 60; // first hour of life
/// Minimum staleness required to recheck a new token
const NEW_TOKEN_BOOST_MIN_STALE_MINUTES: i64 = 12; // ~12 minutes between touches
/// Hard cap of boosted tokens per cycle to avoid API pressure
const NEW_TOKEN_BOOST_PER_CYCLE: usize = 2;

// =============================================================================
// FAIRNESS / TIERING CONFIG
// =============================================================================

/// Liquidity tiers (USD) used to prevent starvation of small-liquidity tokens
/// High: >= 10k, Mid: 1k-10k, Low: 100-1k, Micro: < 100
const LIQ_TIER_HIGH_MIN: f64 = 10_000.0;
const LIQ_TIER_MID_MIN: f64 = 1_000.0;
const LIQ_TIER_LOW_MIN: f64 = 100.0;

/// Per-cycle quotas by tier (percentages of TOKENS_PER_CYCLE)
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

// =============================================================================
// TOKEN MONITOR
// =============================================================================

pub struct TokenMonitor {
    database: TokenDatabase,
    cycle_counter: u64,
}

impl TokenMonitor {
    /// Create new token monitor instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;

        Ok(Self {
            database,
            cycle_counter: 0,
        })
    }

    /// Get tokens that need updating with fairness across liquidity tiers and age-first priority
    async fn get_tokens_for_update(&self) -> Result<Vec<String>, String> {
        let now = Utc::now();

        // Get all tokens from database
        let all_tokens = self.database
            .get_all_tokens_with_update_time().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        if all_tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Filter tokens that need updating (at least MIN_UPDATE_INTERVAL_MINUTES stale)
        // Collect as (mint, liquidity, age_minutes)
        let mut needing_update: Vec<(String, f64, i64)> = Vec::new();
        for (mint, _symbol, last_updated, liquidity) in all_tokens {
            let age_minutes = now.signed_duration_since(last_updated).num_minutes();
            if age_minutes >= MIN_UPDATE_INTERVAL_MINUTES {
                let liq = liquidity;
                needing_update.push((mint, liq, age_minutes));
            }
        }

        // Pre-select boosted "new" tokens with a hard per-cycle cap
        let mut selected_tokens: Vec<String> = Vec::new();
        let mut selected_set: std::collections::HashSet<String> = std::collections::HashSet::new();
        if NEW_TOKEN_BOOST_PER_CYCLE > 0 {
            if
                let Ok(boost_mints) = self.database.get_new_tokens_needing_boost(
                    NEW_TOKEN_BOOST_MAX_AGE_MINUTES,
                    NEW_TOKEN_BOOST_MIN_STALE_MINUTES,
                    NEW_TOKEN_BOOST_PER_CYCLE
                ).await
            {
                for mint in boost_mints.into_iter() {
                    if selected_set.insert(mint.clone()) {
                        selected_tokens.push(mint);
                    }
                }
            }
        }

        // If no other tokens need update and boost is empty, we can early return
        if needing_update.is_empty() {
            return Ok(selected_tokens);
        }

        // Bucket by liquidity tiers
        let mut high: Vec<(String, f64, i64)> = Vec::new();
        let mut mid: Vec<(String, f64, i64)> = Vec::new();
        let mut low: Vec<(String, f64, i64)> = Vec::new();
        let mut micro: Vec<(String, f64, i64)> = Vec::new();

        for t in needing_update.into_iter() {
            let liq = t.1;
            if liq >= LIQ_TIER_HIGH_MIN {
                high.push(t);
            } else if liq >= LIQ_TIER_MID_MIN {
                mid.push(t);
            } else if liq >= LIQ_TIER_LOW_MIN {
                low.push(t);
            } else {
                micro.push(t);
            }
        }

        // Sort each bucket by age descending (oldest first); tie-breaker by liquidity desc
        let by_age_then_liq = |a: &(String, f64, i64), b: &(String, f64, i64)| {
            match b.2.cmp(&a.2) {
                std::cmp::Ordering::Equal =>
                    b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal),
                other => other,
            }
        };
        high.sort_by(by_age_then_liq);
        mid.sort_by(by_age_then_liq);
        low.sort_by(by_age_then_liq);
        micro.sort_by(by_age_then_liq);

        // Compute quotas based on remaining capacity after boost pre-selection
        let capacity = TOKENS_PER_CYCLE.saturating_sub(selected_tokens.len());
        if capacity == 0 {
            return Ok(selected_tokens);
        }
        let quota = |pct: usize| -> usize { (capacity * pct) / 100 };
        let mut q_high = quota(QUOTA_HIGH_PCT).max(1);
        let mut q_mid = quota(QUOTA_MID_PCT).max(1);
        let mut q_low = quota(QUOTA_LOW_PCT).max(1);
        let mut q_micro = quota(QUOTA_MICRO_PCT).max(1);

        // Ensure we don't exceed TOKENS_PER_CYCLE due to max(1)
        let mut total_q = q_high + q_mid + q_low + q_micro;
        while total_q > TOKENS_PER_CYCLE {
            // Reduce micro first, then low, then mid, then high
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

        // Helper to drain up to n mints from a bucket
        let mut take_from_bucket = |
            bucket: &mut Vec<(String, f64, i64)>,
            n: usize,
            out: &mut Vec<String>
        | -> usize {
            let take_n = std::cmp::min(n, bucket.len());
            for (mint, _liq, _age) in bucket.drain(..take_n) {
                if selected_set.insert(mint.clone()) {
                    out.push(mint);
                }
            }
            take_n
        };

        // Take per-bucket quotas
        take_from_bucket(&mut high, q_high, &mut selected_tokens);
        take_from_bucket(&mut mid, q_mid, &mut selected_tokens);
        take_from_bucket(&mut low, q_low, &mut selected_tokens);
        take_from_bucket(&mut micro, q_micro, &mut selected_tokens);

        // Fill remaining capacity oldest-first across all remaining tokens
        let mut remaining_capacity = TOKENS_PER_CYCLE.saturating_sub(selected_tokens.len());
        if remaining_capacity > 0 {
            let mut all_remaining: Vec<(String, f64, i64)> = Vec::new();
            all_remaining.extend(high.into_iter());
            all_remaining.extend(mid.into_iter());
            all_remaining.extend(low.into_iter());
            all_remaining.extend(micro.into_iter());
            // Sort oldest-first globally, tie-break by liquidity desc
            all_remaining.sort_by(by_age_then_liq);

            for (mint, _liq, _age) in all_remaining.into_iter().take(remaining_capacity) {
                if selected_set.insert(mint.clone()) {
                    selected_tokens.push(mint);
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
                &format!("Updating {} tokens with fresh data", mints.len())
            );
        }

        // Get fresh token information from DexScreener API
        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "API_REQUEST",
                &format!("Requesting token data from DexScreener API for {} tokens", mints.len())
            );
        }

        let tokens_result = {
            let api = get_global_dexscreener_api().await.map_err(|e|
                format!("Failed to get global API client: {}", e)
            )?;
            let mut api_instance = api.lock().await;
            api_instance.get_tokens_info(mints).await
        };

        match tokens_result {
            Ok(tokens) => {
                // Track which tokens were returned by the API
                let returned_mints: std::collections::HashSet<String> = tokens
                    .iter()
                    .map(|t| t.mint.clone())
                    .collect();

                // Find tokens that were requested but not returned (no longer exist)
                let missing_mints: Vec<String> = mints
                    .iter()
                    .filter(|mint| !returned_mints.contains(*mint))
                    .cloned()
                    .collect();

                let mut total_updated = 0;

                // Update tokens that were returned by the API
                if !tokens.is_empty() {
                    if is_debug_monitor_enabled() {
                        log(
                            LogTag::Monitor,
                            "API_RESULT",
                            &format!(
                                "API returned {} tokens out of {} requested",
                                tokens.len(),
                                mints.len()
                            )
                        );
                    }

                    match self.database.update_tokens(&tokens).await {
                        Ok(()) => {
                            total_updated += tokens.len();
                            if is_debug_monitor_enabled() {
                                log(
                                    LogTag::Monitor,
                                    "SUCCESS",
                                    &format!("Updated {} tokens in database", tokens.len())
                                );
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Monitor,
                                "ERROR",
                                &format!("Failed to update tokens in database: {}", e)
                            );
                            return Err(format!("Database update failed: {}", e));
                        }
                    }
                }

                // Delete tokens that were not returned by the API (no longer exist)
                let mut deleted_count = 0;
                if !missing_mints.is_empty() {
                    if is_debug_monitor_enabled() {
                        log(
                            LogTag::Monitor,
                            "CLEANUP",
                            &format!(
                                "Removing {} stale tokens that no longer exist on DexScreener: {:?}",
                                missing_mints.len(),
                                missing_mints
                            )
                        );
                    }

                    match self.database.delete_tokens(&missing_mints).await {
                        Ok(actual_deleted) => {
                            deleted_count = actual_deleted;
                            if is_debug_monitor_enabled() {
                                log(
                                    LogTag::Monitor,
                                    "CLEANUP_SUCCESS",
                                    &format!("Deleted {} stale tokens from database", deleted_count)
                                );
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Monitor,
                                "ERROR",
                                &format!("Failed to delete stale tokens: {}", e)
                            );
                            // Don't fail the entire operation if cleanup fails
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
                log(LogTag::Monitor, "ERROR", &format!("Failed to get token info from API: {}", e));
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
            if let Ok(tokens) = self.database.get_tokens_by_mints(&tokens_to_update).await {
                for t in tokens {
                    let liq = t.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0);
                    selected_tiers.add_liquidity(liq);
                }
            }
        }

        let mut total_updated = 0;
        let mut total_deleted = 0;
        let mut batches_ok = 0usize;
        let mut batches_failed = 0usize;

        // Process tokens in API-compatible batches
        for batch in tokens_to_update.chunks(API_BATCH_SIZE) {
            match self.update_token_batch(batch).await {
                Ok(result) => {
                    total_updated += result.updated;
                    total_deleted += result.deleted;
                    batches_ok += 1;
                }
                Err(e) => {
                    log(LogTag::Monitor, "BATCH_ERROR", &format!("Batch update failed: {}", e));
                    // Continue with next batch even if one fails
                    batches_failed += 1;
                }
            }

            // Small delay between batches to respect rate limits
            if batch.len() == API_BATCH_SIZE {
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
                let a = self.database
                    .get_tokens_needing_update(1).await
                    .ok()
                    .map(|v| v.len())
                    .unwrap_or(0);
                let b = self.database
                    .get_tokens_needing_update(2).await
                    .ok()
                    .map(|v| v.len())
                    .unwrap_or(0);
                let c = self.database
                    .get_tokens_needing_update(168)
                    .await // 7 days = 168 hours
                    .ok()
                    .map(|v| v.len())
                    .unwrap_or(0);
                (a, b, c)
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
        log(LogTag::Monitor, "INIT", "Token monitoring loop started");

        loop {
            tokio::select! {
                _ = shutdown.notified() => {
                    log(LogTag::Monitor, "SHUTDOWN", "Token monitoring loop stopping");
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

        log(LogTag::Monitor, "STOP", "Token monitoring loop stopped");
    }
}

// =============================================================================
// PUBLIC INTERFACE
// =============================================================================

/// Start token monitoring background task
pub async fn start_token_monitoring(
    shutdown: Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::Monitor, "START", "Starting token monitoring background task");

    let handle = tokio::spawn(async move {
        let mut monitor = match TokenMonitor::new() {
            Ok(monitor) => {
                log(LogTag::Monitor, "INIT", "Token monitor instance created successfully");
                monitor
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to initialize token monitor: {}", e)
                );
                return;
            }
        };

        log(LogTag::Monitor, "READY", "Starting token monitoring loop");
        monitor.start_monitoring_loop(shutdown).await;
        log(LogTag::Monitor, "EXIT", "Token monitoring task ended");
    });

    Ok(handle)
}

/// Manual monitoring cycle for testing
pub async fn run_monitoring_cycle_once() -> Result<(), String> {
    let mut monitor = TokenMonitor::new().map_err(|e| format!("Failed to create monitor: {}", e))?;
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
    MONITOR_STATS.get_or_init(|| Arc::new(RwLock::new(MonitorStats::default()))).clone()
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
    let emoji = if stats.interval_updated > 0 { "✅" } else { "⏸️" };

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

    let backlog_info = if
        stats.backlog_over_1h > 0 ||
        stats.backlog_over_2h > 0 ||
        stats.backlog_over_7d > 0
    {
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

    log(LogTag::Monitor, "SUMMARY", &body);
}
