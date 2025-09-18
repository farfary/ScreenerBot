use crate::global::is_debug_monitor_enabled;
/// Token monitoring system for periodic updates of database tokens
/// Updates existing tokens based on liquidity priority and time constraints
use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::dexscreener::get_global_dexscreener_api;
use chrono::{ DateTime, Utc };
use futures::TryFutureExt;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::{ Arc, OnceLock };
use tokio::sync::RwLock;
use tokio::time::{ sleep, Duration };

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Monitoring cycle duration in seconds (currently runs every 1 second)
const MONITOR_CYCLE_SECONDS: u64 = 1;

/// Minimum time between updates for a token (1 hour)
const MIN_UPDATE_INTERVAL_HOURS: i64 = 1;

/// Maximum time before forced update (2 hours)
const MAX_UPDATE_INTERVAL_HOURS: i64 = 2;

/// Number of tokens to update per cycle (adjust based on performance)
const TOKENS_PER_CYCLE: usize = 30;

/// Batch size for API calls (DexScreener limit)
const API_BATCH_SIZE: usize = 30;

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

        // Filter tokens that need updating (at least 1 hour old)
        // Collect as (mint, liquidity, age_hours)
        let mut needing_update: Vec<(String, f64, i64)> = Vec::new();
        for (mint, _symbol, last_updated, liquidity) in all_tokens {
            let age_hours = now.signed_duration_since(last_updated).num_hours();
            if age_hours >= MIN_UPDATE_INTERVAL_HOURS {
                let liq = liquidity;
                needing_update.push((mint, liq, age_hours));
            }
        }

        if needing_update.is_empty() {
            return Ok(Vec::new());
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

        // Compute quotas
        let quota = |pct: usize| -> usize { (TOKENS_PER_CYCLE * pct) / 100 };
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

        let mut selected_tokens: Vec<String> = Vec::with_capacity(TOKENS_PER_CYCLE);

        // Helper to drain up to n mints from a bucket
        let mut take_from_bucket = |
            bucket: &mut Vec<(String, f64, i64)>,
            n: usize,
            out: &mut Vec<String>
        | -> usize {
            let take_n = std::cmp::min(n, bucket.len());
            for (mint, _liq, _age) in bucket.drain(..take_n) {
                out.push(mint);
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
                selected_tokens.push(mint);
            }
        }

        Ok(selected_tokens)
    }

    /// Update a batch of tokens with fresh data from DexScreener
    async fn update_token_batch(&mut self, mints: &[String]) -> Result<usize, String> {
        if mints.is_empty() {
            return Ok(0);
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
                if tokens.is_empty() {
                    if is_debug_monitor_enabled() {
                        log(LogTag::Monitor, "WARN", "No token data returned from API for batch");
                    }
                    return Ok(0);
                }

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

                // Update tokens in database with fresh data
                match self.database.update_tokens(&tokens).await {
                    Ok(()) => {
                        let updated_count = tokens.len();
                        if is_debug_monitor_enabled() {
                            log(
                                LogTag::Monitor,
                                "SUCCESS",
                                &format!("Updated {} tokens in database", updated_count)
                            );
                        }
                        Ok(updated_count)
                    }
                    Err(e) => {
                        log(
                            LogTag::Monitor,
                            "ERROR",
                            &format!("Failed to update tokens in database: {}", e)
                        );
                        Err(format!("Database update failed: {}", e))
                    }
                }
            }
            Err(e) => {
                log(LogTag::Monitor, "ERROR", &format!("Failed to get token info from API: {}", e));
                Err(format!("API request failed: {}", e))
            }
        }
    }

    /// Main monitoring cycle - update random tokens based on priority
    async fn run_monitoring_cycle(&mut self) -> Result<(), String> {
        // Mark cycle start in stats
        {
            let stats_handle = get_monitor_stats_handle();
            let mut stats = stats_handle.write().await;
            stats.total_cycles += 1;
            stats.last_cycle_started = Some(Utc::now());
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
        let mut batches_ok = 0usize;
        let mut batches_failed = 0usize;

        // Process tokens in API-compatible batches
        for batch in tokens_to_update.chunks(API_BATCH_SIZE) {
            match self.update_token_batch(batch).await {
                Ok(updated_count) => {
                    total_updated += updated_count;
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

        // Update stats and print a styled summary similar to discovery
        {
            let stats_handle = get_monitor_stats_handle();
            let mut stats = stats_handle.write().await;
            stats.last_cycle_selected = tokens_to_update.len();
            stats.last_cycle_updated = total_updated;
            stats.last_cycle_batches_ok = batches_ok;
            stats.last_cycle_batches_failed = batches_failed;
            stats.last_cycle_tiers = selected_tiers.clone();
            stats.total_updated += total_updated as u64;
            stats.last_cycle_completed = Some(Utc::now());

            // Optionally compute backlog snapshot (only in debug to avoid heavy queries each second)
            if is_debug_monitor_enabled() {
                match self.database.get_tokens_needing_update(1).await {
                    Ok(v) => {
                        stats.backlog_over_1h = v.len();
                    }
                    Err(_) => {}
                }
                match self.database.get_tokens_needing_update(2).await {
                    Ok(v) => {
                        stats.backlog_over_2h = v.len();
                    }
                    Err(_) => {}
                }
            }
        }

        print_monitor_cycle_summary().await;

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
    last_cycle_started: Option<DateTime<Utc>>,
    last_cycle_completed: Option<DateTime<Utc>>,
    last_cycle_selected: usize,
    last_cycle_updated: usize,
    last_cycle_batches_ok: usize,
    last_cycle_batches_failed: usize,
    last_cycle_tiers: TierCounts,
    backlog_over_1h: usize,
    backlog_over_2h: usize,
    last_error: Option<String>,
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

/// Single comprehensive summary log per monitor cycle
async fn print_monitor_cycle_summary() {
    let stats = get_monitor_stats().await;

    // Duration if available
    let duration_ms = if
        let (Some(start), Some(end)) = (stats.last_cycle_started, stats.last_cycle_completed)
    {
        (end - start).num_milliseconds().max(0) as u128
    } else {
        0
    };

    // Emoji based on effectiveness
    let emoji = if stats.last_cycle_updated > 0 { "✅" } else { "⏸️" };

    // Backlog line only when debug (we stored it only in debug cycles)
    let backlog_info = if stats.backlog_over_1h > 0 || stats.backlog_over_2h > 0 {
        format!(
            "\n  • Backlog  ⏱️  >=1h: {}  |  >=2h: {}",
            stats.backlog_over_1h,
            stats.backlog_over_2h
        )
    } else {
        String::new()
    };

    let header_line = "════════════════════════════════════════════════════════════";
    let title = format!("{} MONITOR CYCLE #{}", emoji, stats.total_cycles);
    let selected_line = format!(
        "  • Selected  🧩  {}  (H:{}  |  M:{}  |  L:{}  |  m:{})",
        stats.last_cycle_selected,
        stats.last_cycle_tiers.high,
        stats.last_cycle_tiers.mid,
        stats.last_cycle_tiers.low,
        stats.last_cycle_tiers.micro
    );
    let updated_line = format!(
        "  • Updated   🔄  {}  |  Batches ✅/❌  {} / {}",
        stats.last_cycle_updated,
        stats.last_cycle_batches_ok,
        stats.last_cycle_batches_failed
    );
    let timing_line = format!("  • Duration  🕒  {} ms", duration_ms);

    let body = format!(
        "\n{header}\n{title}\n{header}\n{selected}\n{updated}\n{timing}{backlog}\n{header}",
        header = header_line,
        title = title,
        selected = selected_line,
        updated = updated_line,
        timing = timing_line,
        backlog = backlog_info
    );

    log(LogTag::Monitor, "SUMMARY", &body);
}
