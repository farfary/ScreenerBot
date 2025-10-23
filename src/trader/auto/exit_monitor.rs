//! Position monitoring and exit strategy application

use crate::logger::{log, LogTag};
use crate::pools;
use crate::positions;
use crate::trader::auto::dca;
use crate::trader::auto::strategy_manager::StrategyManager;
use crate::trader::config;
use crate::trader::execution::execute_trade;
use crate::trader::exit::{check_roi_exit, check_time_override, check_trailing_stop};
use crate::trader::safety::check_blacklist_exit;
use tokio::time::{sleep, Duration, Instant};

/// Constants for position monitoring
const POSITION_MONITOR_INTERVAL_SECS: u64 = 5;
const POSITION_CYCLE_MIN_WAIT_MS: u64 = 200;

/// Monitor open positions for exit opportunities
pub async fn monitor_positions(
    mut shutdown: tokio::sync::watch::Receiver<bool>,
) -> Result<(), String> {
    log(LogTag::Trader, "INFO", "Starting position monitor");

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
            log(LogTag::Trader, "INFO", "Position monitor shutting down");
            break;
        }

        // Check if trader is enabled
        let trader_enabled = config::is_trader_enabled();
        if !trader_enabled {
            log(
                LogTag::Trader,
                "INFO",
                "Position monitor paused - trader disabled",
            );
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
                        log(LogTag::Trader, "INFO", "Position monitor shutting down");
                        break;
                    }
                }
            }
            continue;
        }

        log(
            LogTag::Trader,
            "INFO",
            &format!("Checking {} open positions for exit opportunities", open_positions.len()),
        );

        // Process each position
        for position in open_positions {
            // Check if we should shutdown
            if *shutdown.borrow() {
                log(LogTag::Trader, "INFO", "Position monitor shutting down");
                return Ok(());
            }

            // Get current price
            let current_price = match pools::get_pool_price(&position.mint) {
                Some(price_info) => {
                    if price_info.price_sol > 0.0 && price_info.price_sol.is_finite() {
                        price_info.price_sol
                    } else {
                        log(
                            LogTag::Trader,
                            "WARN",
                            &format!("Invalid price for {}: {:.9}", position.symbol, price_info.price_sol),
                        );
                        continue;
                    }
                }
                None => {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!("No price data for {}", position.symbol),
                    );
                    continue;
                }
            };

            // CRITICAL: Update position price BEFORE checking trailing stop
            // This ensures price_highest is current when trailing stop logic evaluates
            if let Err(e) = positions::update_position_price(&position.mint, current_price).await {
                log(
                    LogTag::Trader,
                    "ERROR",
                    &format!("Failed to update price for {}: {}", position.symbol, e),
                );
                // Continue anyway - we can still check exits with current price
            }

            // Get fresh position with updated price_highest for accurate trailing stop calculation
            let fresh_position = match positions::get_position_by_mint(&position.mint).await {
                Some(pos) => pos,
                None => {
                    log(
                        LogTag::Trader,
                        "WARN",
                        &format!("Position disappeared for {}", position.symbol),
                    );
                    continue;
                }
            };

            // Check for blacklist (emergency exit)
            match check_blacklist_exit(&fresh_position, current_price).await {
                Ok(Some(decision)) => {
                    log(
                        LogTag::Trader,
                        "EMERGENCY",
                        &format!("🚨 Token {} blacklisted! Executing emergency exit", fresh_position.symbol),
                    );
                    if let Err(e) = execute_trade(&decision).await {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to execute blacklist exit for {}: {}", fresh_position.symbol, e),
                        );
                    }
                    continue;
                }
                Ok(None) => {} // Not blacklisted
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Error checking blacklist for {}: {}", fresh_position.symbol, e),
                    );
                }
            }

            // Check trailing stop (highest priority) - now using fresh_position with updated price_highest
            match check_trailing_stop(&fresh_position, current_price).await {
                Ok(Some(decision)) => {
                    log(
                        LogTag::Trader,
                        "SIGNAL",
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
                    
                    if let Err(e) = execute_trade(&decision).await {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to execute trailing stop for {}: {}", fresh_position.symbol, e),
                        );
                    }
                    continue;
                }
                Ok(None) => {} // No trailing stop signal
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Error checking trailing stop for {}: {}", fresh_position.symbol, e),
                    );
                }
            }

            // Check ROI target exit
            match check_roi_exit(&fresh_position, current_price).await {
                Ok(Some(decision)) => {
                    log(
                        LogTag::Trader,
                        "SIGNAL",
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
                    if let Err(e) = execute_trade(&decision).await {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to execute ROI exit for {}: {}", fresh_position.symbol, e),
                        );
                    }
                    continue;
                }
                Ok(None) => {} // No ROI signal
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Error checking ROI exit for {}: {}", fresh_position.symbol, e),
                    );
                }
            }

            // Check time override (forced exit for old positions)
            match check_time_override(&fresh_position, current_price).await {
                Ok(Some(decision)) => {
                    log(
                        LogTag::Trader,
                        "SIGNAL",
                        &format!("⏰ Time override triggered for {}", position.symbol),
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
                    
                    if let Err(e) = execute_trade(&decision).await {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to execute time override exit for {}: {}", fresh_position.symbol, e),
                        );
                    }
                    continue;
                }
                Ok(None) => {} // No time override signal
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Error checking time override for {}: {}", fresh_position.symbol, e),
                    );
                }
            }

            // Check strategy-based exit signals
            match StrategyManager::check_exit_strategies(&fresh_position, current_price).await {
                Ok(Some(decision)) => {
                    log(
                        LogTag::Trader,
                        "SIGNAL",
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
                    if let Err(e) = execute_trade(&decision).await {
                        log(
                            LogTag::Trader,
                            "ERROR",
                            &format!("Failed to execute strategy exit for {}: {}", fresh_position.symbol, e),
                        );
                    }
                    continue;
                }
                Ok(None) => {} // No strategy signal
                Err(e) => {
                    log(
                        LogTag::Trader,
                        "ERROR",
                        &format!("Error checking strategy exit for {}: {}", fresh_position.symbol, e),
                    );
                }
            }

            // No exit signals, position continues holding
        }

        // Check for DCA opportunities (separate from exits)
        match dca::process_dca_opportunities().await {
            Ok(dca_decisions) => {
                for decision in dca_decisions {
                    log(
                        LogTag::Trader,
                        "SIGNAL",
                        &format!("📈 DCA opportunity for position {}", decision.position_id.as_ref().unwrap_or(&"unknown".to_string())),
                    );
                    match execute_trade(&decision).await {
                        Ok(result) => {
                            if result.success {
                                log(
                                    LogTag::Trader,
                                    "SUCCESS",
                                    &format!("✅ DCA executed for {}", decision.mint),
                                );
                            } else {
                                log(
                                    LogTag::Trader,
                                    "ERROR",
                                    &format!("❌ DCA failed for {}: {}", decision.mint, result.error.unwrap_or_default()),
                                );
                            }
                        }
                        Err(e) => {
                            log(
                                LogTag::Trader,
                                "ERROR",
                                &format!("Failed to execute DCA: {}", e),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Trader,
                    "ERROR",
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
                    log(LogTag::Trader, "INFO", "Position monitor shutting down");
                    break;
                }
            }
        }
    }

    Ok(())
}
