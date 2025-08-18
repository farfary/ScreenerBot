/// Enhanced Token Monitoring System with Priority Queue
///
/// This module provides intelligent token monitoring that prioritizes:
/// 1. Tokens with open positions (highest priority)
/// 2. High liquidity tokens for new entry detection
/// 3. Zero or stale price tokens that need updates

use crate::logger::{ log, LogTag };
use crate::global::is_debug_monitor_enabled;
use crate::tokens::dexscreener::{
    get_global_dexscreener_api,
    MAX_TOKENS_PER_API_CALL,
    API_CALLS_PER_MONITORING_CYCLE,
};
use crate::tokens::cache::TokenDatabase;
use crate::tokens::blacklist::{ check_and_track_liquidity, is_token_blacklisted };
use crate::tokens::price::{ get_priority_tokens_safe, update_tokens_prices_safe };
use crate::tokens::pool::{
    refresh_pools_infos_for_tokens_safe,
    get_tokens_with_recent_pools_infos_safe,
};
use crate::tokens::types::*;
use tokio::time::{ sleep, Duration };
use tokio::sync::Semaphore;
use std::sync::Arc;

// =============================================================================
// MONITORING CONFIGURATION CONSTANTS
// =============================================================================

/// Enhanced monitoring cycle duration in seconds (2 seconds for real-time price updates)
pub const ENHANCED_CYCLE_DURATION_SECONDS: u64 = 2;

/// Database cleanup interval in seconds (1 minute)
pub const CLEANUP_INTERVAL_SECONDS: u64 = 60;

/// High liquidity threshold for monitoring prioritization (USD)
pub const MONITOR_HIGH_LIQUIDITY_THRESHOLD: f64 = 50000.0;

/// Maximum number of tokens to monitor per cycle (reduced for 2-second intervals)
pub const MAX_TOKENS_PER_MONITOR_CYCLE: usize = 150;

/// Window for "recent pools infos" refresh (seconds)
pub const MONITOR_RECENT_POOLS_WINDOW_SECONDS: i64 = 600; // 10 minutes

// =============================================================================
// ENHANCED TOKEN MONITOR
// =============================================================================

pub struct TokenMonitor {
    database: TokenDatabase,
    rate_limiter: Arc<Semaphore>,
    current_cycle: usize,
    last_cleanup: std::time::Instant,
}

impl TokenMonitor {
    /// Create new enhanced token monitor instance
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database = TokenDatabase::new()?;
        let rate_limiter = Arc::new(Semaphore::new(API_CALLS_PER_MONITORING_CYCLE));

        Ok(Self {
            database,
            rate_limiter,
            current_cycle: 0,
            last_cleanup: std::time::Instant::now(),
        })
    }

    /// Start enhanced monitoring loop with priority queue (2-second intervals)
    pub async fn start_enhanced_monitoring_loop(&mut self, shutdown: Arc<tokio::sync::Notify>) {
        log(LogTag::Monitor, "START", "Enhanced token monitor started with 2-second price updates");

    // Create the shutdown future once to avoid missing notifications between ticks
    let mut shutdown_fut = Box::pin(shutdown.notified());

        loop {
            tokio::select! {
                _ = shutdown_fut.as_mut() => {
                    log(LogTag::Monitor, "SHUTDOWN", "Enhanced token monitor stopping");
                    break;
                }
                _ = sleep(Duration::from_secs(ENHANCED_CYCLE_DURATION_SECONDS)) => {
                    self.current_cycle += 1;
                    
                    if let Err(e) = self.enhanced_monitor_tokens().await {
                        log(LogTag::System, "ERROR", 
                            &format!("Enhanced monitoring cycle failed: {}", e));
                    }
                }
            }
        }

        log(LogTag::Monitor, "STOP", "Enhanced token monitor stopped");
    }

    /// Enhanced monitoring with priority queue system
    async fn enhanced_monitor_tokens(&mut self) -> Result<(), String> {
        // Check if it's time for database cleanup (every 1 minute)
        let should_cleanup = self.last_cleanup.elapsed().as_secs() >= CLEANUP_INTERVAL_SECONDS;

        // Get priority tokens from price service (includes open positions + high liquidity)
        let priority_mints = get_priority_tokens_safe().await;

        if priority_mints.is_empty() {
            log(LogTag::System, "MONITOR", "No priority tokens to monitor");

            // If no priority tokens, get some high liquidity tokens for new entry detection
            self.monitor_for_new_entries().await?;
        } else {
            log(
                LogTag::System,
                "MONITOR",
                &format!("Enhanced monitoring {} priority tokens", priority_mints.len())
            );

            // Process priority tokens first
            self.process_priority_tokens(priority_mints.clone()).await?;

            // Pre-warm/refresh pools infos for priority tokens without exceeding rate limits
            // Limit refreshes per cycle to avoid API rate caps; 2 API calls worth of tokens
            let refresh_budget = MAX_TOKENS_PER_API_CALL * 2;
            let refreshed = refresh_pools_infos_for_tokens_safe(&priority_mints, refresh_budget).await;
            if refreshed > 0 && is_debug_monitor_enabled() {
                log(
                    LogTag::Monitor,
                    "POOLS_INFO_REFRESH",
                    &format!("Refreshed pools infos for {} priority tokens", refreshed)
                );
            }

            // Refresh pools infos for tokens seen in the last 10 minutes (bounded budget)
            let recent_mints = get_tokens_with_recent_pools_infos_safe(MONITOR_RECENT_POOLS_WINDOW_SECONDS).await;
            if !recent_mints.is_empty() {
                let recent_budget = MAX_TOKENS_PER_API_CALL / 2; // keep it light per cycle
                let refreshed_recent = refresh_pools_infos_for_tokens_safe(&recent_mints, recent_budget).await;
                if refreshed_recent > 0 && is_debug_monitor_enabled() {
                    log(
                        LogTag::Monitor,
                        "POOLS_INFO_RECENT_REFRESH",
                        &format!("Refreshed pools infos for {} recently-seen tokens", refreshed_recent)
                    );
                }
            }

            // Also check for new entry opportunities with remaining API budget
            self.monitor_for_new_entries().await?;
        }

        // Perform database cleanup after token data has been updated
        if should_cleanup {
            self.cleanup_database().await?;
            self.last_cleanup = std::time::Instant::now();
        }

        log(
            LogTag::System,
            "MONITOR",
            &format!("Enhanced monitoring cycle #{} completed", self.current_cycle)
        );

        Ok(())
    }

    /// Process priority tokens (open positions + high priority)
    async fn process_priority_tokens(&mut self, priority_mints: Vec<String>) -> Result<(), String> {
        let mut processed = 0;
        let mut updated = 0;
        let mut errors = 0;

        // Process in batches
        for chunk in priority_mints.chunks(MAX_TOKENS_PER_API_CALL) {
            // Acquire rate limit permit
            let _permit = self.rate_limiter.acquire().await.unwrap();

            let tokens_result = {
                let api = match get_global_dexscreener_api().await {
                    Ok(api) => api,
                    Err(e) => {
                        log(
                            LogTag::Monitor,
                            "ERROR",
                            &format!("Failed to get global API client: {}", e)
                        );
                        errors += 1;
                        continue;
                    }
                };
                let mut api_instance = api.lock().await;
                // CRITICAL: Only hold the lock for the API call, then release immediately
                api_instance.get_tokens_info(chunk).await
            }; // Lock is released here automatically

            match tokens_result {
                Ok(updated_tokens) => {
                    // Update database
                    if !updated_tokens.is_empty() {
                        if let Err(e) = self.database.update_tokens(&updated_tokens).await {
                            log(
                                LogTag::Monitor,
                                "ERROR",
                                &format!("Failed to update priority tokens: {}", e)
                            );
                            errors += 1;
                        } else {
                            // Update price service cache
                            update_tokens_prices_safe(chunk).await;

                            // Track liquidity for blacklisting
                            for token in &updated_tokens {
                                if let Some(liquidity) = &token.liquidity {
                                    if let Some(usd_liquidity) = liquidity.usd {
                                        let age_hours = self.calculate_token_age(&token);
                                        check_and_track_liquidity(
                                            &token.mint,
                                            &token.symbol,
                                            usd_liquidity,
                                            age_hours
                                        );
                                    }
                                }
                            }

                            updated += updated_tokens.len();
                            // Only log significant batch updates to reduce noise
                            if updated_tokens.len() > 10 && is_debug_monitor_enabled() {
                                log(
                                    LogTag::Monitor,
                                    "UPDATE",
                                    &format!("Priority: Updated {} tokens", updated_tokens.len())
                                );
                            }

                            // Hint pool service with tokens we just updated so their pools infos stay hot
                            let mints: Vec<String> = chunk.iter().cloned().collect();
                            let _ = refresh_pools_infos_for_tokens_safe(&mints, MAX_TOKENS_PER_API_CALL).await;
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "ERROR",
                        &format!("Failed to fetch priority token info: {}", e)
                    );
                    errors += 1;
                }
            }

            processed += chunk.len();

            // Rate limiting delay between batches (reduced for faster updates)
            if processed < priority_mints.len() {
                sleep(Duration::from_millis(100)).await;
            }
        }

        // Only log summary if there were significant updates or errors
        if updated > 50 || errors > 0 {
            log(
                LogTag::Monitor,
                "PRIORITY",
                &format!(
                    "Priority tokens - Processed: {}, Updated: {}, Errors: {}",
                    processed,
                    updated,
                    errors
                )
            );
        }

        Ok(())
    }

    /// Monitor high liquidity tokens for new entry opportunities
    async fn monitor_for_new_entries(&mut self) -> Result<(), String> {
        // Get high liquidity tokens from database for new entry detection
        let high_liquidity_tokens = self.database
            .get_tokens_by_liquidity_threshold(MONITOR_HIGH_LIQUIDITY_THRESHOLD).await
            .map_err(|e| format!("Failed to get high liquidity tokens: {}", e))?;

        if high_liquidity_tokens.is_empty() {
            log(LogTag::System, "MONITOR", "No high liquidity tokens for new entry detection");
            return Ok(());
        }

        // Filter out blacklisted and prioritize by liquidity
        let mut new_entry_candidates: Vec<ApiToken> = high_liquidity_tokens
            .into_iter()
            .filter(|token| !is_token_blacklisted(&token.mint))
            .take(MAX_TOKENS_PER_MONITOR_CYCLE) // Use comprehensive monitoring limit
            .collect();

        if new_entry_candidates.is_empty() {
            log(LogTag::System, "MONITOR", "No valid candidates for new entry detection");
            return Ok(());
        }

        // Prioritize tokens with zero or stale prices
        new_entry_candidates.sort_by(|a, b| {
            let a_has_price = a.price_sol.is_some() && a.price_sol.unwrap() > 0.0;
            let b_has_price = b.price_sol.is_some() && b.price_sol.unwrap() > 0.0;

            match (a_has_price, b_has_price) {
                (false, true) => std::cmp::Ordering::Less, // a needs update more
                (true, false) => std::cmp::Ordering::Greater, // b needs update more
                _ => {
                    // Both have similar price status, sort by liquidity
                    let a_liq = a.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0);
                    let b_liq = b.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0);
                    b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
                }
            }
        });

        log(
            LogTag::System,
            "NEW_ENTRY",
            &format!(
                "Monitoring {} high liquidity tokens for new entries",
                new_entry_candidates.len()
            )
        );

        // Process new entry candidates
        let mut processed = 0;
        let mut updated = 0;

        for chunk in new_entry_candidates.chunks(MAX_TOKENS_PER_API_CALL) {
            // Check if we still have rate limit budget
            if self.rate_limiter.available_permits() == 0 {
                log(
                    LogTag::System,
                    "RATE_LIMIT",
                    "Rate limit reached, skipping remaining new entry checks"
                );
                break;
            }

            let _permit = self.rate_limiter.acquire().await.unwrap();

            let mints: Vec<String> = chunk
                .iter()
                .map(|t| t.mint.clone())
                .collect();

            let tokens_result = {
                let api = match get_global_dexscreener_api().await {
                    Ok(api) => api,
                    Err(e) => {
                        log(
                            LogTag::Monitor,
                            "ERROR",
                            &format!("Failed to get global API client: {}", e)
                        );
                        continue;
                    }
                };
                let mut api_instance = api.lock().await;
                // CRITICAL: Only hold the lock for the API call, then release immediately
                api_instance.get_tokens_info(&mints).await
            }; // Lock is released here automatically

            match tokens_result {
                Ok(updated_tokens) => {
                    if !updated_tokens.is_empty() {
                        if let Err(e) = self.database.update_tokens(&updated_tokens).await {
                            log(
                                LogTag::Monitor,
                                "ERROR",
                                &format!("Failed to update new entry tokens: {}", e)
                            );
                        } else {
                            // Update price service cache
                            update_tokens_prices_safe(&mints).await;
                            // Warm pools infos cache for these tokens within budget
                            let _ = refresh_pools_infos_for_tokens_safe(&mints, MAX_TOKENS_PER_API_CALL / 2).await;
                            updated += updated_tokens.len();

                            // Only log significant updates to reduce noise
                            if updated_tokens.len() > 10 {
                                log(
                                    LogTag::Monitor,
                                    "NEW_ENTRY",
                                    &format!(
                                        "Updated {} new entry candidates",
                                        updated_tokens.len()
                                    )
                                );
                            }
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "WARN",
                        &format!("Failed to get new entry token info: {}", e)
                    );
                }
            }

            processed += chunk.len();

            // Rate limiting delay (reduced for faster updates)
            sleep(Duration::from_millis(100)).await;
        }

        Ok(())
    }

    /// Calculate token age in hours
    fn calculate_token_age(&self, token: &ApiToken) -> i64 {
        if let Some(created_at) = token.pair_created_at {
            let now = chrono::Utc::now().timestamp();
            let age_seconds = now - created_at;
            age_seconds / 3600 // Convert to hours
        } else {
            // If no creation time, assume very old
            24 * 7 // 7 days
        }
    }

    /// Cleanup database tokens with zero liquidity
    /// This runs every minute after token data has been updated
    async fn cleanup_database(&mut self) -> Result<(), String> {
        log(LogTag::System, "CLEANUP", "Starting database cleanup for zero liquidity tokens");

        match self.database.cleanup_zero_liquidity_tokens().await {
            Ok(deleted_count) => {
                if deleted_count > 0 {
                    log(
                        LogTag::System,
                        "CLEANUP",
                        &format!("Database cleanup completed: {} tokens removed", deleted_count)
                    );
                } else {
                    log(
                        LogTag::System,
                        "CLEANUP",
                        "Database cleanup completed: No tokens to remove"
                    );
                }
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Database cleanup failed: {}", e));
                return Err(format!("Database cleanup failed: {}", e));
            }
        }

        Ok(())
    }
}

// =============================================================================
// ENHANCED MONITOR HELPER FUNCTIONS
// =============================================================================

/// Start enhanced token monitoring in background task
pub async fn start_token_monitoring(
    shutdown: Arc<tokio::sync::Notify>
) -> Result<tokio::task::JoinHandle<()>, String> {
    log(LogTag::Monitor, "START", "Enhanced token monitoring background task started");

    let handle = tokio::spawn(async move {
        let mut monitor = match TokenMonitor::new() {
            Ok(monitor) => monitor,
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to initialize enhanced monitor: {}", e)
                );
                return;
            }
        };

        monitor.start_enhanced_monitoring_loop(shutdown).await;
    });

    Ok(handle)
}

/// Manual enhanced monitoring trigger (for testing)
pub async fn monitor_tokens_once() -> Result<(), String> {
    let mut monitor = TokenMonitor::new().map_err(|e| format!("Failed to create monitor: {}", e))?;
    monitor.enhanced_monitor_tokens().await
}

/// Get enhanced monitoring statistics
pub async fn get_monitoring_stats() -> Result<MonitoringStats, String> {
    let database = TokenDatabase::new().map_err(|e| format!("Failed to create database: {}", e))?;
    let total_tokens = database
        .get_all_tokens().await
        .map_err(|e| format!("Failed to get tokens: {}", e))?
        .len();

    let blacklisted_count = if let Some(stats) = crate::tokens::blacklist::get_blacklist_stats() {
        stats.total_blacklisted
    } else {
        0
    };

    Ok(MonitoringStats {
        total_tokens,
        blacklisted_count,
        active_tokens: total_tokens.saturating_sub(blacklisted_count),
        last_cycle: chrono::Utc::now(),
    })
}

/// Enhanced monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStats {
    pub total_tokens: usize,
    pub blacklisted_count: usize,
    pub active_tokens: usize,
    pub last_cycle: chrono::DateTime<chrono::Utc>,
}
