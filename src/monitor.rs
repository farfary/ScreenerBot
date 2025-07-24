// use crate::global::LIST_MINTS;
// use crate::global::LIST_TOKENS;
use crate::discovery_manager::start_discovery_task;
use crate::token_monitor::start_token_monitoring;
use crate::position_monitor::start_position_monitoring;
use crate::token_blacklist::cleanup_blacklist;
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };
use crate::global::{ TOKEN_DB, LIST_TOKENS };

/// Load all tokens from database into LIST_TOKENS at startup
async fn load_tokens_from_database() {
    log(LogTag::Monitor, "INFO", "Loading tokens from database into LIST_TOKENS...");

    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            match db.get_all_tokens() {
                Ok(tokens) => {
                    let token_count = tokens.len();

                    // Count tokens with liquidity
                    let with_liquidity = tokens
                        .iter()
                        .filter(|token| {
                            token.liquidity
                                .as_ref()
                                .and_then(|l| l.usd)
                                .unwrap_or(0.0) > 0.0
                        })
                        .count();

                    // Update LIST_TOKENS
                    match LIST_TOKENS.write() {
                        Ok(mut list) => {
                            *list = tokens;
                            log(
                                LogTag::Monitor,
                                "SUCCESS",
                                &format!(
                                    "Loaded {} tokens from database to LIST_TOKENS ({} with liquidity)",
                                    token_count,
                                    with_liquidity
                                )
                            );
                        }
                        Err(e) => {
                            log(
                                LogTag::Monitor,
                                "ERROR",
                                &format!("Failed to update LIST_TOKENS: {}", e)
                            );
                        }
                    }
                }
                Err(e) => {
                    log(
                        LogTag::Monitor,
                        "ERROR",
                        &format!("Failed to load tokens from database: {}", e)
                    );
                }
            }
        } else {
            log(LogTag::Monitor, "ERROR", "Token database not initialized");
        }
    } else {
        log(LogTag::Monitor, "ERROR", "Could not acquire token database lock");
    }
}

/// Monitor background task loop - coordinates separate discovery, token monitoring, and position monitoring tasks
pub async fn monitor(shutdown: Arc<Notify>) {
    log(LogTag::Monitor, "INFO", "Starting coordinated monitoring system...");

    // Initialize LIST_TOKENS from database at startup
    load_tokens_from_database().await;

    // Start separate background tasks
    let discovery_shutdown = shutdown.clone();
    let token_monitoring_shutdown = shutdown.clone();
    let position_monitoring_shutdown = shutdown.clone();
    let cleanup_shutdown = shutdown.clone();

    // Start discovery task (mint finding from DexScreener and RugCheck)
    let discovery_handle = tokio::spawn(async move {
        start_discovery_task(discovery_shutdown).await;
    });

    // Start token monitoring task (periodic database checks with liquidity prioritization, excluding open positions)
    let token_monitoring_handle = tokio::spawn(async move {
        start_token_monitoring(token_monitoring_shutdown).await;
    });

    // Start position monitoring task (fast updates for open position tokens)
    let position_monitoring_handle = tokio::spawn(async move {
        start_position_monitoring(position_monitoring_shutdown).await;
    });

    // Start periodic cleanup task
    let cleanup_handle = tokio::spawn(async move {
        let mut cleanup_cycle = 0;
        loop {
            if check_shutdown_or_delay(&cleanup_shutdown, Duration::from_secs(3600)).await {
                // 1 hour
                log(LogTag::Monitor, "INFO", "Cleanup task shutting down...");
                break;
            }

            cleanup_cycle += 1;
            log(LogTag::Monitor, "INFO", &format!("Running cleanup cycle #{}", cleanup_cycle));

            // Cleanup old blacklist entries
            cleanup_blacklist();

            log(LogTag::Monitor, "SUCCESS", &format!("Cleanup cycle #{} completed", cleanup_cycle));
        }
    });

    // Wait for shutdown signal
    loop {
        if check_shutdown_or_delay(&shutdown, Duration::from_secs(5)).await {
            log(LogTag::Monitor, "INFO", "Monitor coordinator shutting down...");
            break;
        }
    }

    // Cancel background tasks
    discovery_handle.abort();
    token_monitoring_handle.abort();
    position_monitoring_handle.abort();
    cleanup_handle.abort();

    log(LogTag::Monitor, "INFO", "All monitoring tasks stopped");
}

// All token discovery and monitoring logic is now handled by separate background tasks:
// - discovery_manager.rs: Mint finding from DexScreener and RugCheck APIs
// - token_monitor.rs: Periodic database checks with liquidity-based prioritization (excludes open positions)
// - position_monitor.rs: Fast monitoring for open position tokens with 15-second cycles
