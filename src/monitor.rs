// use crate::global::LIST_MINTS;
// use crate::global::LIST_TOKENS;
use crate::discovery::update_tokens_from_mints;
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

        // Call all mint-fetching functions with timeout handling
        let discovery_tasks = async {
            if let Err(e) = discovery_dexscreener_fetch_token_profiles().await {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token profiles: {}", e));
            }

            if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
                return;
            }

            if let Err(e) = discovery_dexscreener_fetch_token_boosts().await {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts: {}", e));
            }

            if check_shutdown_or_delay(&shutdown, Duration::from_millis(100)).await {
                return;
            }

            if let Err(e) = discovery_dexscreener_fetch_token_boosts_top().await {
                log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts top: {}", e));
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

        // Update token info for all mints with timeout
        let token_update_timeout = tokio::time::timeout(
            Duration::from_secs(60),
            update_tokens_from_mints(shutdown.clone())
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
