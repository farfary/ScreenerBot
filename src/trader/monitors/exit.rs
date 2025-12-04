//! Position monitoring and exit strategy application - orchestration only
//!
//! This module handles:
//! - Monitoring loop and timing
//! - Concurrent position evaluation with semaphore
//! - Priority-based trade execution (Emergency → High → Normal)
//! - DCA opportunity processing
//! - Event recording

use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::types::{TradeDecision, TradePriority};
use crate::trader::{config, constants, evaluators, executors};
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
    let sell_concurrency = std::cmp::max(1, config::get_sell_concurrency());
    let semaphore = Arc::new(Semaphore::new(sell_concurrency));
    let mut eval_tasks = Vec::new();

    // Phase 1: Spawn concurrent evaluation tasks for all positions
    for position in open_positions {
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
              &format!("Exit evaluation failed for {}: {}", position_symbol, e),
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
        if let Err(e) = executors::execute_trade(&decision).await {
          logger::error(
            LogTag::Trader,
            &format!("Failed to execute exit for {}: {}", evaluation.symbol, e),
          );
        }
      }
    }

    // Check for DCA opportunities (separate from exits)
    match evaluators::dca::process_dca_opportunities().await {
      Ok(dca_decisions) => {
        for decision in dca_decisions {
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
                logger::info(
                  LogTag::Trader,
 &format!("DCA executed for {}", decision.mint),
                );
              } else {
                logger::error(
                  LogTag::Trader,
                  &format!(
 "DCA failed for {}: {}",
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
