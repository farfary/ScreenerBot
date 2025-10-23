//! Entry opportunity monitoring based on strategies

use crate::logger::{log, LogTag};
use crate::pools;
use crate::trader::auto::strategy_manager::StrategyManager;
use crate::trader::config;
use crate::trader::execution::execute_trade;
use crate::trader::safety::{
    check_position_limits, has_open_position, is_blacklisted, is_in_reentry_cooldown,
};
use tokio::time::{sleep, Duration, Instant};

/// Constants for entry monitoring
const ENTRY_MONITOR_INTERVAL_SECS: u64 = 3;
const ENTRY_CYCLE_MIN_WAIT_MS: u64 = 100;
const ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS: u64 = 30;

/// Monitor for new entry opportunities
pub async fn monitor_entries(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Starting entry opportunity monitor");

    // Create semaphore for concurrent entry checks
    let entry_check_concurrency = config::get_entry_check_concurrency();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(entry_check_concurrency));

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            log(LogTag::Trader, "INFO", "Entry monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            log(LogTag::Trader, "INFO", "Entry monitor paused - trader disabled");
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Check if we can open more positions
        if !check_position_limits().await? {
            log(
                LogTag::Trader,
                "INFO",
                "Position limit reached, skipping entry check",
            );
            sleep(Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS)).await;
            continue;
        }

        // Get available tokens from pools
        let available_tokens = pools::get_available_tokens();

        // Process tokens with concurrency control
        let mut futures = Vec::new();

        for token in &available_tokens {
            // Skip if already has open position
            if has_open_position(&token).await? {
                continue;
            }

            // Skip if in re-entry cooldown
            if is_in_reentry_cooldown(&token).await? {
                continue;
            }

            // Get latest price info
            if let Some(price_info) = pools::get_pool_price(&token) {
                // Check if token is blacklisted
                if is_blacklisted(&token).await? {
                    continue;
                }

                // Acquire semaphore permit with timeout
                let sem_clone = semaphore.clone();
                let token_clone = token.clone();

                let future = tokio::spawn(async move {
                    let _permit = match tokio::time::timeout(
                        Duration::from_secs(ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS),
                        sem_clone.acquire(),
                    )
                    .await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to acquire semaphore for entry check: {}", e),
                            );
                            return None;
                        }
                        Err(_) => {
                            log(
                                LogTag::Trader,
                                "WARN",
                                &format!(
                                    "Timeout waiting for entry check semaphore for {}",
                                    token_clone
                                ),
                            );
                            return None;
                        }
                    };

                    // Check entry strategies
                    match StrategyManager::check_entry_strategies(&token_clone, &price_info).await {
                        Ok(Some(decision)) => Some(decision),
                        Ok(None) => None,
                        Err(e) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Entry strategy check failed for {}: {}",
                                    token_clone, e
                                ),
                            );
                            None
                        }
                    }
                });

                futures.push(future);
            }
        }

        // Collect results and process trade decisions
        for future in futures {
            if let Ok(Some(decision)) = future.await {
                // Execute the trade
                match execute_trade(&decision).await {
                    Ok(result) => {
                        if result.success {
                            log(
                                LogTag::Trader,
                                "SUCCESS",
                                &format!(
                                    "Entry executed for {}: tx={}",
                                    decision.mint,
                                    result.tx_signature.unwrap_or_default()
                                ),
                            );
                        } else {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!(
                                    "Entry failed for {}: {}",
                                    decision.mint,
                                    result.error.unwrap_or_default()
                                ),
                            );
                        }
                    }
                    Err(e) => {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to execute entry for {}: {}", decision.mint, e),
                        );
                    }
                }
            }
        }

        // Calculate wait time for next cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time = if cycle_duration >= Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) {
            Duration::from_millis(ENTRY_CYCLE_MIN_WAIT_MS)
        } else {
            Duration::from_secs(ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
        };

        // Wait for next cycle or shutdown
        tokio::select! {
            _ = sleep(wait_time) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log(LogTag::Trader, "INFO", "Entry monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}
