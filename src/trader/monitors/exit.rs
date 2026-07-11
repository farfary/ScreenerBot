//! Position monitoring and exit strategy application - orchestration only
//!
//! This module handles:
//! - Monitoring loop and timing
//! - Concurrent position evaluation with semaphore
//! - Priority-based trade execution (Emergency → High → Normal)
//! - DCA opportunity processing
//! - Event recording
//! - Action tracking for dashboard visibility

use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::types::{TradeDecision, TradePriority};
use crate::trader::{actions, config, constants, evaluators, executors};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration, Instant};

/// Monitor open positions for exit opportunities
pub async fn monitor_positions(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    logger::info(LogTag::Trader, "Starting position monitor");

    // Record monitor start event
    crate::events::record_trader_event(
        "exit_monitor_started",
        crate::events::Severity::Info,
        None,
        None,
        serde_json::json!({
          "monitor": "exit",
          "message": "Exit/position monitor started",
        }),
    )
    .await;

    let mut was_paused = false; // Track paused state

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            logger::info(LogTag::Trader, "Position monitor shutting down");
            break;
        }

        // Check force stop first (emergency halt)
        if crate::global::is_force_stopped() {
            if !was_paused {
                logger::warning(LogTag::Trader, "Exit monitor paused - FORCE STOPPED");
                was_paused = true;
            }
            sleep(Duration::from_secs(1)).await; // Check more frequently during force stop
            continue;
        }

        // Check if exit monitor specifically is enabled (uses combined check)
        let exit_enabled = config::is_exit_monitor_enabled();
        if !exit_enabled {
            // Only log when transitioning to paused state
            if !was_paused {
                logger::info(LogTag::Trader, "Exit monitor paused - disabled via config");
                was_paused = true;
            }
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Reset pause tracking if we're running
        if was_paused {
            logger::info(LogTag::Trader, "Exit monitor resumed");
            was_paused = false;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Get all open positions
        let open_positions = positions::get_open_positions().await;

        if open_positions.is_empty() {
            // No positions to monitor, just wait
            tokio::select! {
              _ = sleep(Duration::from_secs(constants::POSITION_MONITOR_INTERVAL_SECS)) => {},
              _ = shutdown.changed() => {
                if *shutdown.borrow() {
                  logger::info(LogTag::Trader, "Position monitor shutting down");
                  break;
                }
              }
            }
            continue;
        }

        logger::info(
            LogTag::Trader,
            &format!(
                "Checking {} open positions for exit opportunities",
                open_positions.len()
            ),
        );

        // Create semaphore for concurrent position evaluation
        let sell_concurrency = config::get_sell_concurrency().max(1);
        let semaphore = Arc::new(Semaphore::new(sell_concurrency));
        let mut eval_tasks = Vec::new();

        // Phase 1: Spawn concurrent evaluation tasks for all positions
        for position in open_positions {
            // Manual/force buys are manually managed — the auto-trader never auto-sells
            // or auto-DCAs them. Skip exit evaluation entirely so the user keeps control.
            // Price/PnL tracking happens elsewhere, so the position stays visible/updated.
            if position.manual_management {
                logger::debug(
                    LogTag::Trader,
                    &format!(
                        "Skipping auto-exit for manually managed position {} ({})",
                        position.symbol, position.mint
                    ),
                );
                continue;
            }

            // A partial exit that has been submitted but not yet verified has already sold
            // tokens the position still counts as held: `remaining_token_amount` is only
            // decremented on verification. Evaluating now would re-trigger the same exit
            // against a stale amount every tick until it confirms.
            if positions::is_partial_exit_pending(&position.mint).await {
                logger::debug(
                    LogTag::Trader,
                    &format!(
                        "Skipping exit evaluation for {} - a partial exit is still confirming",
                        position.symbol
                    ),
                );
                continue;
            }

            let sem = semaphore.clone();
            let shutdown_check = shutdown.clone();
            let position_mint = position.mint.clone();
            let position_symbol = position.symbol.clone();

            let task = tokio::spawn(async move {
                // Check shutdown before acquiring semaphore
                if *shutdown_check.borrow() {
                    return None;
                }

                // Acquire semaphore permit (limits concurrent RPC calls)
                let _permit = sem.acquire().await.ok()?;

                // Check shutdown again after acquiring
                if *shutdown_check.borrow() {
                    return None;
                }

                // Evaluate position for exit (all exit checks + DCA)
                match evaluators::evaluate_exit_for_position(position).await {
                    Ok(Some(d)) => Some(PositionEvaluation {
                        mint: position_mint,
                        symbol: position_symbol,
                        decision: Some(d.clone()),
                        priority: d.priority,
                    }),
                    Ok(None) => None,
                    Err(e) => {
                        logger::error(
                            LogTag::Trader,
                            &format!("Exit evaluation failed for {position_symbol}: {e}"),
                        );
                        None
                    }
                }
            });

            eval_tasks.push(task);
        }

        // Await all evaluation tasks
        let eval_results = futures::future::join_all(eval_tasks).await;

        // Phase 2: Process trade decisions sequentially (preserves execution order)
        // Sort by priority: Emergency > High > Normal
        let mut evaluations: Vec<PositionEvaluation> = eval_results
            .into_iter()
            .filter_map(|result| match result {
                Ok(Some(eval)) => Some(eval),
                Ok(None) => None,
                Err(e) => {
                    logger::info(
                        LogTag::Trader,
                        &format!("Position evaluation task failed: {e}"),
                    );
                    None
                }
            })
            .collect();

        // Sort by priority (Emergency first, then High, then Normal)
        evaluations.sort_by(|a, b| {
            use TradePriority::*;
            match (&a.priority, &b.priority) {
                (Emergency, Emergency) => std::cmp::Ordering::Equal,
                (Emergency, _) => std::cmp::Ordering::Less,
                (_, Emergency) => std::cmp::Ordering::Greater,
                (High, High) => std::cmp::Ordering::Equal,
                (High, _) => std::cmp::Ordering::Less,
                (_, High) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            }
        });

        // Track if any rate limit (429) error occurred this cycle
        let mut rate_limit_hit = false;

        // Execute trades sequentially in priority order
        for evaluation in evaluations {
            // Check shutdown before each execution
            if *shutdown.borrow() {
                logger::info(LogTag::Trader, "Position monitor shutting down");
                return Ok(());
            }

            if let Some(decision) = evaluation.decision {
                // Create action for dashboard visibility
                let action = actions::AutoCloseAction::new(
                    &decision.mint,
                    Some(&evaluation.symbol),
                    decision.position_id.as_ref().and_then(|s| s.parse().ok()),
                    &format!("{:?}", decision.reason),
                )
                .await
                .ok();

                // Mark evaluation complete
                if let Some(ref a) = action {
                    a.complete_evaluation().await;
                    a.start_quote().await;
                }

                match executors::execute_trade(&decision).await {
                    Ok(result) => {
                        if result.success {
                            // Complete action successfully
                            if let Some(ref a) = action {
                                a.complete_quote().await;
                                a.start_swap().await;
                                let sig = result.tx_signature.as_deref().unwrap_or("unknown");
                                a.complete_swap(sig, result.executed_size_sol).await;
                                a.complete(result.tx_signature.as_deref()).await;
                            }

                            logger::info(
                                LogTag::Trader,
                                &format!(
                                    "Exit executed for {}: tx={}",
                                    evaluation.symbol,
                                    result.tx_signature.as_deref().unwrap_or("unknown")
                                ),
                            );
                        } else {
                            // Fail action
                            let error_msg = result.error.clone().unwrap_or_default();
                            if let Some(ref a) = action {
                                a.fail(&error_msg).await;
                            }

                            logger::error(
                                LogTag::Trader,
                                &format!("Exit failed for {}: {}", evaluation.symbol, error_msg),
                            );
                        }
                    }
                    Err(e) => {
                        // Fail action
                        if let Some(ref a) = action {
                            a.fail(&e).await;
                        }

                        // Detect Jupiter rate limiting to back off
                        let err_lower = e.to_lowercase();
                        if err_lower.contains("429") || err_lower.contains("rate limit") {
                            logger::warning(
                                LogTag::Trader,
                                &format!(
                                    "Jupiter rate limit hit during exit for {}, will back off",
                                    evaluation.symbol
                                ),
                            );
                            rate_limit_hit = true;
                        }

                        logger::error(
                            LogTag::Trader,
                            &format!("Failed to execute exit for {}: {}", evaluation.symbol, e),
                        );
                    }
                }
            }
        }

        // Check for DCA opportunities (separate from exits)
        match evaluators::dca::process_dca_opportunities().await {
            Ok(dca_decisions) => {
                for decision in dca_decisions {
                    // Get symbol for action
                    let symbol = crate::tokens::get_full_token_async(&decision.mint)
                        .await
                        .ok()
                        .flatten()
                        .map(|t| t.symbol);

                    // Create DCA action for dashboard visibility
                    let action = actions::AutoDcaAction::new(
                        &decision.mint,
                        symbol.as_deref(),
                        decision.position_id.as_deref(),
                        0, // We don't have dca_count here, it's tracked in position
                    )
                    .await
                    .ok();

                    // Mark evaluation complete
                    if let Some(ref a) = action {
                        a.complete_evaluation().await;
                        a.start_quote().await;
                    }

                    logger::info(
                        LogTag::Trader,
                        &format!(
                            "DCA opportunity for position {}",
                            decision.position_id.as_deref().unwrap_or("unknown")
                        ),
                    );

                    match executors::execute_trade(&decision).await {
                        Ok(result) => {
                            if result.success {
                                // Complete action successfully
                                if let Some(ref a) = action {
                                    a.complete_quote().await;
                                    a.start_swap().await;
                                    let sig = result.tx_signature.as_deref().unwrap_or("unknown");
                                    a.complete_swap(sig).await;
                                    a.complete(result.tx_signature.as_deref(), None).await;
                                }

                                logger::info(
                                    LogTag::Trader,
                                    &format!("DCA executed for {}", decision.mint),
                                );
                            } else {
                                // Fail action
                                let error_msg = result.error.clone().unwrap_or_default();
                                if let Some(ref a) = action {
                                    a.fail(&error_msg).await;
                                }

                                logger::error(
                                    LogTag::Trader,
                                    &format!("DCA failed for {}: {}", decision.mint, error_msg),
                                );
                            }
                        }
                        Err(e) => {
                            // Fail action
                            if let Some(ref a) = action {
                                a.fail(&e).await;
                            }

                            logger::error(LogTag::Trader, &format!("Failed to execute DCA: {e}"));
                        }
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Trader,
                    &format!("Error processing DCA opportunities: {e}"),
                );
            }
        }

        // If rate limited, back off 30 seconds before next cycle to avoid hammering Jupiter
        if rate_limit_hit {
            logger::warning(
                LogTag::Trader,
                "Rate limit detected this cycle, backing off 30 seconds before next exit monitor cycle",
            );
            sleep(Duration::from_secs(30)).await;
        }

        // Ensure minimum cycle time
        let cycle_elapsed = cycle_start.elapsed();
        if cycle_elapsed < Duration::from_millis(constants::POSITION_CYCLE_MIN_WAIT_MS) {
            sleep(Duration::from_millis(constants::POSITION_CYCLE_MIN_WAIT_MS) - cycle_elapsed)
                .await;
        }

        // Wait for next cycle or shutdown
        tokio::select! {
          _ = sleep(Duration::from_secs(constants::POSITION_MONITOR_INTERVAL_SECS)) => {},
          _ = shutdown.changed() => {
            if *shutdown.borrow() {
              logger::info(LogTag::Trader, "Position monitor shutting down");
              break;
            }
          }
        }
    }

    Ok(())
}

/// Result of position evaluation for exit
struct PositionEvaluation {
    mint: String,
    symbol: String,
    decision: Option<TradeDecision>,
    priority: TradePriority,
}
