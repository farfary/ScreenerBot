// use crate::global::LIST_MINTS;
// use crate::global::LIST_TOKENS;
use crate::discovery::update_tokens_from_mints_concurrent;
use crate::discovery::*;
use std::sync::Arc;
use tokio::sync::Notify;
use std::time::Duration;
use crate::utils::check_shutdown_or_delay;
use crate::logger::{ log, LogTag };

/// Monitor background task loop
pub async fn monitor(shutdown: Arc<Notify>) {
    loop {
        // Check for shutdown before each API call
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
            log(LogTag::Monitor, "INFO", "monitor task shutting down...");
            break;
        }

        // Call all mint-fetching functions concurrently for better performance
        let discovery_tasks = async {
            // Run all discovery tasks concurrently using tokio::join!
            let (
                profiles_result,
                boosts_result,
                boosts_top_result,
                rugcheck_verified_result,
                rugcheck_trending_result,
                rugcheck_recent_result,
                rugcheck_new_tokens_result,
            ) = tokio::join!(
                discovery_dexscreener_fetch_token_profiles(),
                discovery_dexscreener_fetch_token_boosts(),
                discovery_dexscreener_fetch_token_boosts_top(),
                discovery_rugcheck_fetch_verified(),
                discovery_rugcheck_fetch_trending(),
                discovery_rugcheck_fetch_recent(),
                discovery_rugcheck_fetch_new_tokens()
            );

            // Log any errors from the concurrent tasks
            if let Err(e) = profiles_result {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token profiles: {}", e));
            }
            if let Err(e) = boosts_result {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts: {}", e));
            }
            if let Err(e) = boosts_top_result {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts top: {}", e));
            }
            if let Err(e) = rugcheck_verified_result {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck verified: {}", e));
            }
            if let Err(e) = rugcheck_trending_result {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck trending: {}", e));
            }
            if let Err(e) = rugcheck_recent_result {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch RugCheck recent: {}", e));
            }
            if let Err(e) = rugcheck_new_tokens_result {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to fetch RugCheck new tokens: {}", e)
                );
            }
        };

        // Run discovery tasks with timeout
        let discovery_timeout = tokio::time::timeout(Duration::from_secs(30), discovery_tasks);

        match discovery_timeout.await {
            Ok(_) => {
                // Discovery completed successfully
            }
            Err(_) => {
                log(LogTag::Monitor, "WARN", "Discovery tasks timed out");
            }
        }

        // Check shutdown again before token update
        if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
            log(LogTag::Monitor, "INFO", "monitor task shutting down...");
            break;
        }

        // Update token info for all mints with concurrent batching
        let token_update_timeout = tokio::time::timeout(
            Duration::from_secs(120), // Increased timeout for concurrent processing
            update_tokens_from_mints_concurrent(shutdown.clone())
        );

        match token_update_timeout.await {
            Ok(result) => {
                if let Err(e) = result {
                    log(
                        LogTag::Monitor,
                        "ERROR",
                        &format!("Failed to update tokens from mints: {}", e)
                    );
                }
            }
            Err(_) => {
                log(LogTag::Monitor, "WARN", "Token update timed out");
            }
        }

        if check_shutdown_or_delay(&shutdown, Duration::from_secs(5)).await {
            log(LogTag::Monitor, "INFO", "monitor task shutting down...");
            break;
        }
    }
}

// All token update logic is now handled by the API functions and LIST_MINTS
