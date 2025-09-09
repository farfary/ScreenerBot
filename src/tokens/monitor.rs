use crate::global::is_debug_monitor_enabled;
/// Token monitoring system for periodic updates of database tokens
/// Updates existing tokens based on liquidity priority and time constraints
use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::dexscreener::get_global_dexscreener_api;
use crate::tokens::security::{ get_security_analyzer, TokenSecurityInfo, SecurityRiskLevel };
use chrono::{ DateTime, Utc };
use futures::TryFutureExt;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{ sleep, Duration };

// =============================================================================
// CONFIGURATION CONSTANTS
// =============================================================================

/// Monitoring cycle duration in seconds (runs every 30 seconds)
const MONITOR_CYCLE_SECONDS: u64 = 1;

/// Minimum time between updates for a token (1 hour)
const MIN_UPDATE_INTERVAL_HOURS: i64 = 1;

/// Maximum time before forced update (2 hours)
const MAX_UPDATE_INTERVAL_HOURS: i64 = 2;

/// Number of tokens to update per cycle (adjust based on performance)
const TOKENS_PER_CYCLE: usize = 30;

/// Batch size for API calls (DexScreener limit)
const API_BATCH_SIZE: usize = 30;

/// Near-zero liquidity threshold in USD (tokens below this will be removed)
const NEAR_ZERO_LIQUIDITY_USD: f64 = 10.0;

/// Cleanup cycle interval in monitoring cycles (run cleanup every N cycles)
const CLEANUP_CYCLE_INTERVAL: u64 = 12; // Every 12 cycles = 1 minute (12 * 5 seconds)

/// Security monitoring constants
const SECURITY_UPDATE_CYCLE_INTERVAL: u64 = 5; // Every 5 cycles
const SECURITY_TOKENS_PER_CYCLE: usize = 15; // Smaller batches for RPC-intensive operations
const MAX_SECURITY_UPDATE_AGE_HOURS: i64 = 24; // Force update after 24 hours
const SECURITY_STATIC_UPDATE_AGE_HOURS: i64 = 168; // Static properties: weekly
const SECURITY_DYNAMIC_UPDATE_AGE_HOURS: i64 = 12; // Dynamic properties: twice daily

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

    /// Get tokens that need updating, prioritized by liquidity
    async fn get_tokens_for_update(&self) -> Result<Vec<String>, String> {
        let now = Utc::now();

        // Get all tokens from database
        let all_tokens = self.database
            .get_all_tokens_with_update_time().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        if all_tokens.is_empty() {
            return Ok(Vec::new());
        }

        // Filter tokens that need updating
        let mut tokens_needing_update = Vec::new();
        let mut forced_update_tokens = Vec::new();

        for (mint, _symbol, last_updated, liquidity) in all_tokens {
            let hours_since_update = now.signed_duration_since(last_updated).num_hours();

            if hours_since_update >= MAX_UPDATE_INTERVAL_HOURS {
                // Force update after 2 hours
                forced_update_tokens.push((mint, liquidity, hours_since_update));
            } else if hours_since_update >= MIN_UPDATE_INTERVAL_HOURS {
                // Eligible for update after 1 hour
                tokens_needing_update.push((mint, liquidity, hours_since_update));
            }
        }

        // Always prioritize forced updates first
        forced_update_tokens.sort_by(|a, b|
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        ); // Sort by liquidity descending
        tokens_needing_update.sort_by(|a, b|
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        ); // Sort by liquidity descending

        // Combine lists: forced updates first, then regular updates
        let mut selected_tokens = Vec::new();

        // Add forced updates (up to half of cycle capacity)
        let forced_count = (TOKENS_PER_CYCLE / 2).min(forced_update_tokens.len());
        for (mint, _, _) in forced_update_tokens.into_iter().take(forced_count) {
            selected_tokens.push(mint);
        }

        // Add regular updates for remaining capacity
        let remaining_capacity = TOKENS_PER_CYCLE.saturating_sub(selected_tokens.len());
        if remaining_capacity > 0 {
            // Randomize selection from eligible tokens to ensure fair distribution
            let mut eligible: Vec<_> = tokens_needing_update.into_iter().collect();
            eligible.shuffle(&mut rand::thread_rng());

            for (mint, _, _) in eligible.into_iter().take(remaining_capacity) {
                selected_tokens.push(mint);
            }
        }

        if is_debug_monitor_enabled() && !selected_tokens.is_empty() {
            log(
                LogTag::Monitor,
                "SELECTION",
                &format!(
                    "Selected {} tokens for update (forced: {}, regular: {})",
                    selected_tokens.len(),
                    forced_count,
                    selected_tokens.len() - forced_count
                )
            );
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
        // Get tokens that need updating
        let tokens_to_update = self.get_tokens_for_update().await?;

        if tokens_to_update.is_empty() {
            if is_debug_monitor_enabled() {
                log(LogTag::Monitor, "IDLE", "No tokens need updating at this time");
            }
            return Ok(());
        }

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "START",
                &format!("Starting monitoring cycle for {} tokens", tokens_to_update.len())
            );
        }

        let mut total_updated = 0;

        // Process tokens in API-compatible batches
        for batch in tokens_to_update.chunks(API_BATCH_SIZE) {
            match self.update_token_batch(batch).await {
                Ok(updated_count) => {
                    total_updated += updated_count;
                }
                Err(e) => {
                    log(LogTag::Monitor, "BATCH_ERROR", &format!("Batch update failed: {}", e));
                    // Continue with next batch even if one fails
                }
            }

            // Small delay between batches to respect rate limits
            if batch.len() == API_BATCH_SIZE {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            }
        }

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "COMPLETE",
                &format!("Monitoring cycle completed: {} tokens updated", total_updated)
            );
        }

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "WAITING",
                &format!("Waiting {} seconds for next monitoring cycle", MONITOR_CYCLE_SECONDS)
            );
        }

        Ok(())
    }

    /// Run periodic cleanup of near-zero liquidity tokens
    async fn run_cleanup_cycle(&mut self) -> Result<(), String> {
        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "CLEANUP_START",
                &format!(
                    "Starting cleanup cycle for problematic tokens: blacklisted, decimal-failed, security issues, and low liquidity below ${:.1}",
                    NEAR_ZERO_LIQUIDITY_USD
                )
            );
        }

        match self.database.cleanup_problematic_tokens(NEAR_ZERO_LIQUIDITY_USD).await {
            Ok(deleted_count) => {
                if deleted_count > 0 {
                    log(
                        LogTag::Monitor,
                        "CLEANUP_SUCCESS",
                        &format!("Cleanup cycle completed: {} tokens removed", deleted_count)
                    );
                } else if is_debug_monitor_enabled() {
                    log(LogTag::Monitor, "CLEANUP_IDLE", "Cleanup cycle: No tokens removed");
                }
                Ok(())
            }
            Err(e) => {
                log(LogTag::Monitor, "CLEANUP_ERROR", &format!("Cleanup cycle failed: {}", e));
                Err(format!("Cleanup cycle failed: {}", e))
            }
        }
    }

    /// Get tokens that are missing security information completely
    async fn get_tokens_missing_security_info(&self) -> Result<Vec<String>, String> {
        let all_tokens = self.database
            .get_all_tokens_with_update_time().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        let security_analyzer = get_security_analyzer();
        let mut tokens_missing_security = Vec::new();

        for (mint, _, _, _) in all_tokens {
            // Check if security info exists in database
            match security_analyzer.database.get_security_info(&mint) {
                Ok(None) => tokens_missing_security.push(mint),
                Ok(Some(_)) => {} // Has security info
                Err(_) => tokens_missing_security.push(mint), // Error = treat as missing
            }
        }

        if is_debug_monitor_enabled() && !tokens_missing_security.is_empty() {
            log(
                LogTag::Monitor,
                "SECURITY_MISSING",
                &format!(
                    "Found {} tokens missing security information",
                    tokens_missing_security.len()
                )
            );
        }

        Ok(tokens_missing_security)
    }

    /// Get tokens with stale security information that need updates
    async fn get_tokens_with_stale_security_info(&self) -> Result<Vec<String>, String> {
        let all_tokens = self.database
            .get_all_tokens_with_update_time().await
            .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

        let security_analyzer = get_security_analyzer();
        let mut tokens_with_stale_security = Vec::new();
        let now = Utc::now();

        for (mint, _, _, _) in all_tokens {
            if let Ok(Some(security_info)) = security_analyzer.database.get_security_info(&mint) {
                // Check if static security info is stale (authority, LP lock)
                let static_age = now.signed_duration_since(
                    security_info.timestamps.authority_last_checked
                );
                let dynamic_age = security_info.timestamps.holder_last_checked
                    .map(|t| now.signed_duration_since(t))
                    .unwrap_or_else(||
                        chrono::Duration::hours(SECURITY_DYNAMIC_UPDATE_AGE_HOURS + 1)
                    );

                if
                    static_age.num_hours() > SECURITY_STATIC_UPDATE_AGE_HOURS ||
                    dynamic_age.num_hours() > SECURITY_DYNAMIC_UPDATE_AGE_HOURS
                {
                    tokens_with_stale_security.push(mint);
                }
            }
        }

        if is_debug_monitor_enabled() && !tokens_with_stale_security.is_empty() {
            log(
                LogTag::Monitor,
                "SECURITY_STALE",
                &format!(
                    "Found {} tokens with stale security information",
                    tokens_with_stale_security.len()
                )
            );
        }

        Ok(tokens_with_stale_security)
    }

    /// Update security information for a batch of tokens
    async fn update_security_batch(&mut self, mints: &[String]) -> Result<usize, String> {
        if mints.is_empty() {
            return Ok(0);
        }

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "SECURITY_UPDATE",
                &format!("Updating security info for {} tokens", mints.len())
            );
        }

        let security_analyzer = get_security_analyzer();
        let mut successful_updates = 0;

        // Process tokens in smaller batches to avoid overwhelming RPC
        for batch in mints.chunks(10) {
            let batch_vec: Vec<String> = batch.to_vec();

            match security_analyzer.analyze_multiple_tokens(&batch_vec).await {
                Ok(security_results) => {
                    successful_updates += security_results.len();

                    if is_debug_monitor_enabled() {
                        log(
                            LogTag::Monitor,
                            "SECURITY_BATCH_SUCCESS",
                            &format!(
                                "Successfully analyzed {} tokens in batch",
                                security_results.len()
                            )
                        );
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "SECURITY_BATCH_ERROR",
                        &format!("Failed to analyze security batch: {}", e)
                    );
                }
            }

            // Small delay between batches to avoid RPC overload
            if batch.len() == 10 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "SECURITY_UPDATE_COMPLETE",
                &format!(
                    "Security update completed: {}/{} tokens analyzed",
                    successful_updates,
                    mints.len()
                )
            );
        }

        Ok(successful_updates)
    }

    /// Run security monitoring cycle
    async fn run_security_monitoring_cycle(&mut self) -> Result<(), String> {
        if is_debug_monitor_enabled() {
            log(LogTag::Monitor, "SECURITY_CYCLE_START", "Starting security monitoring cycle");
        }

        // Priority 1: Tokens missing security info completely
        let missing_security_tokens = self.get_tokens_missing_security_info().await?;
        let mut tokens_to_process = missing_security_tokens
            .into_iter()
            .take(SECURITY_TOKENS_PER_CYCLE)
            .collect::<Vec<_>>();

        // Priority 2: Fill remaining capacity with stale security tokens
        if tokens_to_process.len() < SECURITY_TOKENS_PER_CYCLE {
            let remaining_capacity = SECURITY_TOKENS_PER_CYCLE - tokens_to_process.len();
            let stale_security_tokens = self.get_tokens_with_stale_security_info().await?;

            // Randomize to ensure fair distribution
            let mut stale_tokens = stale_security_tokens;
            stale_tokens.shuffle(&mut rand::thread_rng());

            for token in stale_tokens.into_iter().take(remaining_capacity) {
                if !tokens_to_process.contains(&token) {
                    tokens_to_process.push(token);
                }
            }
        }

        if tokens_to_process.is_empty() {
            if is_debug_monitor_enabled() {
                log(LogTag::Monitor, "SECURITY_CYCLE_IDLE", "No tokens need security updates");
            }
            return Ok(());
        }

        // Update security information for selected tokens
        let updated_count = self.update_security_batch(&tokens_to_process).await?;

        if is_debug_monitor_enabled() {
            log(
                LogTag::Monitor,
                "SECURITY_CYCLE_COMPLETE",
                &format!(
                    "Security cycle completed: {}/{} tokens updated",
                    updated_count,
                    tokens_to_process.len()
                )
            );
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

                    // Run security monitoring cycle every 5th cycle
                    if self.cycle_counter % SECURITY_UPDATE_CYCLE_INTERVAL == 0 {
                        if let Err(e) = self.run_security_monitoring_cycle().await {
                            log(
                                LogTag::Monitor,
                                "SECURITY_CYCLE_ERROR",
                                &format!("Security monitoring cycle failed: {}", e)
                            );
                        }
                    }

                    // Run cleanup cycle periodically
                    if self.cycle_counter % CLEANUP_CYCLE_INTERVAL == 0 {
                        if let Err(e) = self.run_cleanup_cycle().await {
                            log(
                                LogTag::Monitor,
                                "CLEANUP_CYCLE_ERROR",
                                &format!("Cleanup cycle failed: {}", e)
                            );
                        }
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
