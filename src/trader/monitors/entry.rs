//! Entry opportunity monitoring - orchestration only
//!
//! This module handles:
//! - Monitoring loop and timing
//! - Token reservation (prevents duplicate concurrent entries)
//! - Concurrency control via semaphore
//! - Calling evaluators for entry logic
//! - Executing trades
//! - Event recording

use crate::logger::{self, LogTag};
use crate::pools;
use crate::trader::{config, constants, evaluators, executors};
use std::collections::HashMap;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, Instant};

/// Entry cycle reservations to prevent duplicate concurrent entries for same token
/// Expires after ENTRY_RESERVATION_TIMEOUT_SECS to handle cases where entry fails
static ENTRY_CYCLE_RESERVATIONS: LazyLock<RwLock<HashMap<String, Instant>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Try to reserve a token for entry processing in this cycle
/// Returns true if reservation successful, false if already reserved
async fn try_reserve_token_for_cycle(mint: &str) -> bool {
    let mut reservations = ENTRY_CYCLE_RESERVATIONS.write().await;

    // Clean expired reservations
    reservations.retain(|_, instant| {
        instant.elapsed() < Duration::from_secs(constants::ENTRY_RESERVATION_TIMEOUT_SECS)
    });

    // Try to reserve
    if reservations.contains_key(mint) {
        return false; // Already reserved by another thread
    }

    reservations.insert(mint.to_string(), Instant::now());
    true
}

/// Clear reservation for a token (called after entry attempt completes)
async fn clear_token_reservation(mint: &str) {
    let mut reservations = ENTRY_CYCLE_RESERVATIONS.write().await;
    reservations.remove(mint);
}

/// Monitor for new entry opportunities
pub async fn monitor_entries(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    logger::info(LogTag::Trader, "Starting entry opportunity monitor");

    // Record monitor start event
    crate::events::record_safe(crate::events::Event::new(
        crate::events::EventCategory::Trader,
        Some("entry_monitor_started".to_string()),
        crate::events::Severity::Info,
        None,
        None,
        serde_json::json!({
            "monitor": "entry",
            "message": "Entry opportunity monitor started",
        }),
    ))
    .await;

    // Create semaphore for concurrent entry checks
    let entry_check_concurrency = config::get_entry_check_concurrency();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(entry_check_concurrency));

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            logger::info(LogTag::Trader, "Entry monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            logger::info(LogTag::Trader, "Entry monitor paused - trader disabled");
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Get available tokens from pools
        let available_tokens = pools::get_available_tokens();

        // Process tokens with concurrency control
        let mut futures = Vec::new();

        for token in &available_tokens {
            // Try to reserve token for this cycle - prevents duplicate concurrent entries
            if !try_reserve_token_for_cycle(&token).await {
                logger::debug(
                    LogTag::Trader,
                    &format!(
                        "Token {} already reserved by another thread, skipping",
                        token
                    ),
                );
                continue;
            }

            // Get latest price info
            // Note: If no price info, let reservation expire naturally via timeout
            // instead of clearing immediately to avoid race conditions
            if let Some(price_info) = pools::get_pool_price(&token) {
                // Acquire semaphore permit with timeout
                let sem_clone = semaphore.clone();
                let token_clone = token.clone();

                let future = tokio::spawn(async move {
                    let _permit = match tokio::time::timeout(
                        Duration::from_secs(constants::ENTRY_CHECK_ACQUIRE_TIMEOUT_SECS),
                        sem_clone.acquire(),
                    )
                    .await
                    {
                        Ok(Ok(permit)) => permit,
                        Ok(Err(e)) => {
                            logger::error(
                                LogTag::Trader,
                                &format!("Failed to acquire semaphore for entry check: {}", e),
                            );
                            return None;
                        }
                        Err(_) => {
                            logger::warning(
                                LogTag::Trader,
                                &format!(
                                    "Timeout waiting for entry check semaphore for {}",
                                    token_clone
                                ),
                            );
                            return None;
                        }
                    };

                    // Evaluate entry opportunity (all safety checks + strategy evaluation)
                    match evaluators::evaluate_entry_for_token(&token_clone, &price_info).await {
                        Ok(Some(decision)) => Some(decision),
                        Ok(None) => None,
                        Err(e) => {
                            logger::error(
                                LogTag::Trader,
                                &format!("Entry evaluation failed for {}: {}", token_clone, e),
                            );
                            None
                        }
                    }
                });

                futures.push((token.clone(), future));
            }
            // Note: If no price info available, reservation will expire via timeout
            // This prevents race conditions from immediate clearing
        }

        // Collect results and process trade decisions
        for (token, future) in futures {
            match future.await {
                Ok(Some(decision)) => {
                    // Record entry signal event
                    crate::events::record_safe(crate::events::Event::new(
                        crate::events::EventCategory::Trader,
                        Some("entry_signal_generated".to_string()),
                        crate::events::Severity::Info,
                        Some(decision.mint.clone()),
                        None,
                        serde_json::json!({
                            "action": "entry_signal",
                            "mint": decision.mint,
                            "strategy_id": decision.strategy_id,
                            "reason": format!("{:?}", decision.reason),
                            "priority": format!("{:?}", decision.priority),
                        }),
                    ))
                    .await;

                    // Execute the trade
                    let mint_for_cleanup = decision.mint.clone();
                    match executors::execute_trade(&decision).await {
                        Ok(result) => {
                            // Clear reservation after execution attempt
                            clear_token_reservation(&mint_for_cleanup).await;

                            if result.success {
                                let tx_sig = result.tx_signature.clone();
                                logger::info(
                                    LogTag::Trader,
                                    &format!(
                                        "Entry executed for {}: tx={}",
                                        decision.mint,
                                        tx_sig.clone().unwrap_or_default()
                                    ),
                                );

                                // Record successful entry event
                                crate::events::record_safe(crate::events::Event::new(
                                    crate::events::EventCategory::Trader,
                                    Some("entry_executed".to_string()),
                                    crate::events::Severity::Info,
                                    Some(decision.mint.clone()),
                                    tx_sig.clone(),
                                    serde_json::json!({
                                        "success": true,
                                        "mint": decision.mint,
                                        "tx_signature": tx_sig,
                                    }),
                                ))
                                .await;
                            } else {
                                let error_msg = result.error.clone().unwrap_or_default();
                                logger::error(
                                    LogTag::Trader,
                                    &format!("Entry failed for {}: {}", decision.mint, error_msg),
                                );

                                // Record failed entry event
                                crate::events::record_safe(crate::events::Event::new(
                                    crate::events::EventCategory::Trader,
                                    Some("entry_failed".to_string()),
                                    crate::events::Severity::Error,
                                    Some(decision.mint.clone()),
                                    None,
                                    serde_json::json!({
                                        "success": false,
                                        "mint": decision.mint,
                                        "error": result.error,
                                    }),
                                ))
                                .await;
                            }
                        }
                        Err(e) => {
                            // Clear reservation on error
                            clear_token_reservation(&mint_for_cleanup).await;

                            logger::error(
                                LogTag::Trader,
                                &format!("Failed to execute entry for {}: {}", decision.mint, e),
                            );

                            // Record execution error event
                            crate::events::record_safe(crate::events::Event::new(
                                crate::events::EventCategory::Trader,
                                Some("entry_execution_error".to_string()),
                                crate::events::Severity::Error,
                                Some(decision.mint.clone()),
                                None,
                                serde_json::json!({
                                    "mint": decision.mint,
                                    "error": e.to_string(),
                                }),
                            ))
                            .await;
                        }
                    }
                }
                Ok(None) => {
                    // No entry signal, clear reservation
                    clear_token_reservation(&token).await;
                }
                Err(e) => {
                    // Task error, clear reservation
                    clear_token_reservation(&token).await;
                    logger::error(
                        LogTag::Trader,
                        &format!("Entry evaluation task failed for {}: {}", token, e),
                    );
                }
            }
        }

        // Calculate wait time for next cycle
        let cycle_duration = cycle_start.elapsed();
        let wait_time =
            if cycle_duration >= Duration::from_secs(constants::ENTRY_MONITOR_INTERVAL_SECS) {
                Duration::from_millis(constants::ENTRY_CYCLE_MIN_WAIT_MS)
            } else {
                Duration::from_secs(constants::ENTRY_MONITOR_INTERVAL_SECS) - cycle_duration
            };

        // Wait for next cycle or shutdown
        tokio::select! {
            _ = sleep(wait_time) => {},
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    logger::info(LogTag::Trader, "Entry monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}
