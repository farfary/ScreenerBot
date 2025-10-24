//! Position monitoring and exit strategy application

use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions;
use crate::trader::auto::dca;
use crate::trader::auto::strategy_manager::StrategyManager;
use crate::trader::config;
use crate::trader::execution::execute_trade;
use crate::trader::exit::{check_roi_exit, check_time_override, check_trailing_stop};
use crate::trader::safety::check_blacklist_exit;
use crate::trader::types::{TradeDecision, TradePriority};
use futures::future;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration, Instant};

/// Constants for position monitoring
const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;
const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;

/// Monitor open positions for exit opportunities
pub async fn monitor_positions(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    logger::info(LogTag::Trader, "Starting position monitor");

    // Record monitor start event
    crate::events::record_safe(crate::events::Event::new(
        crate::events::EventCategory::Trader,
        Some("exit_monitor_started".to_string()),
        crate::events::Severity::Info,
        None,
        None,
        serde_json::json!({
            "monitor": "exit",
            "message": "Exit/position monitor started",
        }),
    ))
    .await;

    loop {
        // Check if we should shutdown
        if *shutdown.borrow() {
            logger::info(LogTag::Trader, "Position monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            logger::info(LogTag::Trader, "Position monitor paused - trader disabled");
            sleep(Duration::from_secs(5)).await;
            continue;
        }

        // Start cycle timing
        let cycle_start = Instant::now();

        // Get all open positions
        let open_positions = positions::get_open_positions().await;

        if open_positions.is_empty() {
            // No positions to monitor, just wait
            tokio::select! {
                _ = sleep(Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)) => {},
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

        // Create semaphore for concurrent position evaluation (max 5 concurrent)
        let semaphore = Arc::new(Semaphore::new(5));
        let mut eval_tasks = Vec::new();

        // Phase 1: Spawn concurrent evaluation tasks for all positions
        for position in open_positions {
            let sem = semaphore.clone();
            let shutdown_check = shutdown.clone();

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

                // Evaluate position for exit (concurrent safe)
                evaluate_position_for_exit(position).await
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
                        &format!("Position evaluation task failed: {}", e),
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

        // Execute trades sequentially in priority order
        for evaluation in evaluations {
            // Check shutdown before each execution
            if *shutdown.borrow() {
                logger::info(LogTag::Trader, "Position monitor shutting down");
                return Ok(());
            }

            if let Some(decision) = evaluation.decision {
                if let Err(e) = execute_trade(&decision).await {
                    logger::error(
                        LogTag::Trader,
                        &format!("Failed to execute exit for {}: {}", evaluation.symbol, e),
                    );
                }
            }
        }

        // Check for DCA opportunities (separate from exits)
        match dca::process_dca_opportunities().await {
            Ok(dca_decisions) => {
                for decision in dca_decisions {
                    logger::info(
                        LogTag::Trader,
                        &format!(
                            "📈 DCA opportunity for position {}",
                            decision
                                .position_id
                                .as_ref()
                                .unwrap_or(&"unknown".to_string())
                        ),
                    );
                    match execute_trade(&decision).await {
                        Ok(result) => {
                            if result.success {
                                logger::info(
                                    LogTag::Trader,
                                    &format!("✅ DCA executed for {}", decision.mint),
                                );
                            } else {
                                logger::error(
                                    LogTag::Trader,
                                    &format!(
                                        "❌ DCA failed for {}: {}",
                                        decision.mint,
                                        result.error.unwrap_or_default()
                                    ),
                                );
                            }
                        }
                        Err(e) => {
                            logger::error(LogTag::Trader, &format!("Failed to execute DCA: {}", e));
                        }
                    }
                }
            }
            Err(e) => {
                logger::error(
                    LogTag::Trader,
                    &format!("Error processing DCA opportunities: {}", e),
                );
            }
        }

        // Ensure minimum cycle time
        let cycle_elapsed = cycle_start.elapsed();
        if cycle_elapsed < Duration::from_millis(POSITION_CYCLE_MIN_WAIT_MS) {
            sleep(Duration::from_millis(POSITION_CYCLE_MIN_WAIT_MS) - cycle_elapsed).await;
        }

        // Wait for next cycle or shutdown
        tokio::select! {
            _ = sleep(Duration::from_secs(POSITION_MONITOR_INTERVAL_SECS)) => {},
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

/// Evaluate a single position for exit opportunities (concurrent safe)
/// This function can be called concurrently for multiple positions
async fn evaluate_position_for_exit(
    position: crate::positions::Position,
) -> Option<PositionEvaluation> {
    // Get current price
    let current_price = match pools::get_pool_price(&position.mint) {
        Some(price_info) => {
            if price_info.price_sol > 0.0 && price_info.price_sol.is_finite() {
                price_info.price_sol
            } else {
                logger::info(
                    LogTag::Trader,
                    &format!(
                        "Invalid price for {}: {:.9}",
                        position.symbol, price_info.price_sol
                    ),
                );
                return None;
            }
        }
        None => {
            logger::info(
                LogTag::Trader,
                &format!("No price data for {}", position.symbol),
            );
            return None;
        }
    };

    // NOTE: Position price is now updated by positions::price_updater module (every 1s)
    // No need to update here - just use current_price for exit evaluation

    // Get fresh position with updated price_highest for accurate trailing stop calculation
    let fresh_position = match positions::get_position_by_mint(&position.mint).await {
        Some(pos) => pos,
        None => {
            logger::info(
                LogTag::Trader,
                &format!("Position disappeared for {}", position.symbol),
            );
            return None;
        }
    };

    // Check for blacklist (emergency exit) - sync call, highest priority
    if let Some(decision) = check_blacklist_exit(&fresh_position, current_price) {
        logger::info(
            LogTag::Trader,
            &format!(
                "🚨 Token {} blacklisted! Emergency exit signal",
                fresh_position.symbol
            ),
        );
        return Some(PositionEvaluation {
            mint: fresh_position.mint.clone(),
            symbol: fresh_position.symbol.clone(),
            decision: Some(decision),
            priority: TradePriority::Emergency,
        });
    }

    // Check trailing stop (highest priority after blacklist)
    match check_trailing_stop(&fresh_position, current_price).await {
        Ok(Some(decision)) => {
            logger::info(
                LogTag::Trader,
                &format!("📉 Trailing stop triggered for {}", fresh_position.symbol),
            );

            // Record exit signal event
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("exit_signal_trailing_stop".to_string()),
                crate::events::Severity::Info,
                Some(fresh_position.mint.clone()),
                None,
                serde_json::json!({
                    "exit_type": "trailing_stop",
                    "mint": fresh_position.mint,
                    "symbol": fresh_position.symbol,
                    "current_price": current_price,
                }),
            ))
            .await;

            return Some(PositionEvaluation {
                mint: fresh_position.mint.clone(),
                symbol: fresh_position.symbol.clone(),
                decision: Some(decision),
                priority: TradePriority::High,
            });
        }
        Ok(None) => {} // No trailing stop signal
        Err(e) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Error checking trailing stop for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Check ROI target exit
    match check_roi_exit(&fresh_position, current_price).await {
        Ok(Some(decision)) => {
            logger::info(
                LogTag::Trader,
                &format!("🎯 ROI target reached for {}", fresh_position.symbol),
            );

            // Record ROI exit signal event
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("exit_signal_roi_target".to_string()),
                crate::events::Severity::Info,
                Some(fresh_position.mint.clone()),
                None,
                serde_json::json!({
                    "exit_type": "roi_target",
                    "mint": fresh_position.mint,
                    "symbol": fresh_position.symbol,
                    "current_price": current_price,
                }),
            ))
            .await;

            return Some(PositionEvaluation {
                mint: fresh_position.mint.clone(),
                symbol: fresh_position.symbol.clone(),
                decision: Some(decision),
                priority: TradePriority::Normal,
            });
        }
        Ok(None) => {} // No ROI signal
        Err(e) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Error checking ROI exit for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Check time override (forced exit for old positions)
    match check_time_override(&fresh_position, current_price).await {
        Ok(Some(decision)) => {
            logger::info(
                LogTag::Trader,
                &format!("⏰ Time override triggered for {}", fresh_position.symbol),
            );

            // Record time override exit signal event
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("exit_signal_time_override".to_string()),
                crate::events::Severity::Info,
                Some(fresh_position.mint.clone()),
                None,
                serde_json::json!({
                    "exit_type": "time_override",
                    "mint": fresh_position.mint,
                    "symbol": fresh_position.symbol,
                    "current_price": current_price,
                }),
            ))
            .await;

            return Some(PositionEvaluation {
                mint: fresh_position.mint.clone(),
                symbol: fresh_position.symbol.clone(),
                decision: Some(decision),
                priority: TradePriority::Normal,
            });
        }
        Ok(None) => {} // No time override signal
        Err(e) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Error checking time override for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // Check strategy-based exit signals
    match StrategyManager::check_exit_strategies(&fresh_position, current_price).await {
        Ok(Some(decision)) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "📊 Strategy exit signal for {} (strategy: {:?})",
                    fresh_position.symbol, decision.strategy_id
                ),
            );

            // Record strategy exit signal event
            crate::events::record_safe(crate::events::Event::new(
                crate::events::EventCategory::Trader,
                Some("exit_signal_strategy".to_string()),
                crate::events::Severity::Info,
                Some(fresh_position.mint.clone()),
                None,
                serde_json::json!({
                    "exit_type": "strategy",
                    "mint": fresh_position.mint,
                    "symbol": fresh_position.symbol,
                    "strategy_id": decision.strategy_id,
                    "current_price": current_price,
                }),
            ))
            .await;

            return Some(PositionEvaluation {
                mint: fresh_position.mint.clone(),
                symbol: fresh_position.symbol.clone(),
                decision: Some(decision),
                priority: TradePriority::Normal,
            });
        }
        Ok(None) => {} // No strategy signal
        Err(e) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "Error checking strategy exit for {}: {}",
                    fresh_position.symbol, e
                ),
            );
        }
    }

    // No exit signals
    None
}
