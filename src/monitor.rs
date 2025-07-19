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
        // Call all mint-fetching functions
        if let Err(e) = discovery_dexscreener_fetch_token_profiles().await {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token profiles: {}", e));
        }
        if let Err(e) = discovery_dexscreener_fetch_token_boosts().await {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts: {}", e));
        }
        if let Err(e) = discovery_dexscreener_fetch_token_boosts_top().await {
            log(LogTag::Monitor, "ERROR", &format!("Failed to fetch token boosts top: {}", e));
        }

        // Update token info for all mints
        if let Err(e) = update_tokens_from_mints(shutdown.clone()).await {
            log(LogTag::Monitor, "ERROR", &format!("Failed to update tokens from mints: {}", e));
        }

        if check_shutdown_or_delay(&shutdown, Duration::from_secs(10)).await {
            log(LogTag::Monitor, "INFO", "monitor task shutting down...");
            break;
        }
    }
}

// All token update logic is now handled by the API functions and LIST_MINTS
