//! Entry opportunity monitoring - orchestration only
//!
//! This module handles:
//! - Monitoring loop and timing
//! - Token reservation (prevents duplicate concurrent entries)
//! - Concurrency control via semaphore
//! - Calling evaluators for entry logic
//! - Executing trades
//! - Event recording
//! - Action tracking for dashboard visibility

use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::trader::{actions, config, constants, evaluators, executors};
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
    crate::events::record_trader_event(
        "entry_monitor_started",
        crate::events::Severity::Info,
        None,
        None,
        serde_json::json!({
            "monitor": "entry",
            "message": "Entry opportunity monitor started",
        }),
    )
    .await;

    // Create semaphore for concurrent entry checks
    let entry_check_concurrency = config::get_entry_check_concurrency();
    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(entry_check_concurrency));

    let mut was_paused = false; // Track paused state

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            logger::info(LogTag::Trader, "Entry monitor shutting down");
            break;
        }

        // Check force stop first (emergency halt)
        if crate::global::is_force_stopped() {
            if !was_paused {
                logger::warning(LogTag::Trader, "Entry monitor paused - FORCE STOPPED");
                was_paused = true;
            }
            sleep(Duration::from_secs(1)).await; // Check more frequently during force stop
            continue;
        }

        // Check if entry monitor specifically is enabled (uses combined check)
        let entry_enabled = config::is_entry_monitor_enabled();
        if !entry_enabled {
            // Only log when transitioning to paused state
            if !was_paused {
                logger::info(LogTag::Trader, "Entry monitor paused - disabled via config");
                was_paused = true;
            }
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Reset pause tracking if we're running
        if was_paused {
            logger::info(LogTag::Trader, "Entry monitor resumed");
            was_paused = false;
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
                    crate::events::record_trader_event(
                        "entry_signal_generated",
                        crate::events::Severity::Info,
                        Some(&decision.mint),
                        None,
                        serde_json::json!({
                            "action": "entry_signal",
                            "mint": decision.mint,
                            "strategy_id": decision.strategy_id,
                            "reason": format!("{:?}", decision.reason),
                            "priority": format!("{:?}", decision.priority),
                        }),
                    )
                    .await;

                    // Create action for dashboard visibility
                    let symbol = crate::tokens::get_full_token_async(&decision.mint)
                        .await
                        .ok()
                        .flatten()
                        .map(|t| t.symbol);

                    let action = actions::AutoOpenAction::new(
                        &decision.mint,
                        symbol.as_deref(),
                        decision.strategy_id.as_deref(),
                        &format!("{:?}", decision.reason),
                    )
                    .await
                    .ok();

                    // Mark evaluation complete (we got a decision)
                    if let Some(ref a) = action {
                        a.complete_evaluation().await;
                        a.start_quote().await;
                    }

                    // Execute the trade
                    let mint_for_cleanup = decision.mint.clone();
                    match executors::execute_trade(&decision).await {
                        Ok(result) => {
                            // Clear reservation after execution attempt
                            clear_token_reservation(&mint_for_cleanup).await;

                            if result.success {
                                let tx_sig = result.tx_signature.clone();

                                // Complete action
                                if let Some(ref a) = action {
                                    a.complete_quote().await;
                                    a.start_swap().await;
                                    a.complete_swap(tx_sig.as_deref().unwrap_or("unknown"))
                                        .await;
                                    a.complete(tx_sig.as_deref()).await;
                                }

                                logger::info(
                                    LogTag::Trader,
                                    &format!(
                                        "Entry executed for {}: tx={}",
                                        decision.mint,
                                        tx_sig.clone().unwrap_or_default()
                                    ),
                                );

                                // Record successful entry event
                                crate::events::record_trader_event(
                                    "entry_executed",
                                    crate::events::Severity::Info,
                                    Some(&decision.mint),
                                    tx_sig.as_deref(),
                                    serde_json::json!({
                                        "success": true,
                                        "mint": decision.mint,
                                        "tx_signature": tx_sig,
                                    }),
                                )
                                .await;
                            } else {
                                let error_msg = result.error.clone().unwrap_or_default();

                                // Fail action
                                if let Some(ref a) = action {
                                    a.fail(&error_msg).await;
                                }

                                if let Some(remaining) =
                                    positions::parse_position_slot_error(&error_msg)
                                {
                                    logger::info(
                                        LogTag::Trader,
                                        &format!(
                                            "Entry blocked for {} – capacity guard engaged (permits left: {})",
                                            decision.mint, remaining
                                        ),
                                    );

                                    crate::events::record_trader_event(
                                        "entry_capacity_guard",
                                        crate::events::Severity::Info,
                                        Some(&decision.mint),
                                        None,
                                        serde_json::json!({
                                            "mint": decision.mint,
                                            "reason": error_msg,
                                            "remaining_permits": remaining,
                                        }),
                                    )
                                    .await;
                                } else {
                                    logger::error(
                                        LogTag::Trader,
                                        &format!(
                                            "Entry failed for {}: {}",
                                            decision.mint, error_msg
                                        ),
                                    );

                                    crate::events::record_trader_event(
                                        "entry_failed",
                                        crate::events::Severity::Error,
                                        Some(&decision.mint),
                                        None,
                                        serde_json::json!({
                                            "success": false,
                                            "mint": decision.mint,
                                            "error": result.error,
                                        }),
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(e) => {
                            // Clear reservation on error
                            clear_token_reservation(&mint_for_cleanup).await;

                            // Fail action
                            if let Some(ref a) = action {
                                a.fail(&e).await;
                            }

                            logger::error(
                                LogTag::Trader,
                                &format!("Failed to execute entry for {}: {}", decision.mint, e),
                            );

                            // Record execution error event
                            crate::events::record_trader_event(
                                "entry_execution_error",
                                crate::events::Severity::Error,
                                Some(&decision.mint),
                                None,
                                serde_json::json!({
                                    "mint": decision.mint,
                                    "error": e.to_string(),
                                }),
                            )
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
