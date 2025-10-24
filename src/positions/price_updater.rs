use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions::{get_open_positions, update_position_price};
use crate::tokens;
use std::time::Duration;
use tokio::time::sleep;

/// Position price updater service
/// Updates all open positions' current_price every second
/// Priority: Pool price (real-time) > API price (if fresh < 5s)

const UPDATE_INTERVAL_SECS: u64 = 1;
const API_PRICE_MAX_AGE_SECS: i64 = 5;

pub async fn start_price_updater(mut shutdown: tokio::sync::watch::Receiver<bool>) {
    logger::info(
        LogTag::Positions,
        "🔄 Starting position price updater (1s interval)",
    );

    loop {
        tokio::select! {
            _ = sleep(Duration::from_secs(UPDATE_INTERVAL_SECS)) => {
                update_all_position_prices().await;
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    logger::info(LogTag::Positions, "Position price updater shutting down");
                    break;
                }
            }
        }
    }
}

async fn update_all_position_prices() {
    let positions = get_open_positions().await;

    if positions.is_empty() {
        return;
    }

    let mut updated_count = 0;
    let mut pool_price_count = 0;
    let mut api_price_count = 0;
    let mut failed_count = 0;

    for position in positions {
        match get_current_price(&position.mint).await {
            Some((price, source)) => {
                match update_position_price(&position.mint, price).await {
                    Ok(_) => {
                        updated_count += 1;
                        match source {
                            PriceSource::Pool => pool_price_count += 1,
                            PriceSource::Api => api_price_count += 1,
                        }
                    }
                    Err(e) => {
                        logger::debug(
                            LogTag::Positions,
                            &format!(
                                "Failed to update price for {}: {}",
                                position.symbol, e
                            ),
                        );
                        failed_count += 1;
                    }
                }
            }
            None => {
                logger::debug(
                    LogTag::Positions,
                    &format!("No valid price available for {}", position.symbol),
                );
                failed_count += 1;
            }
        }
    }

    if updated_count > 0 {
        logger::debug(
            LogTag::Positions,
            &format!(
                "Price update: updated={} (pool={}, api={}) failed={}",
                updated_count, pool_price_count, api_price_count, failed_count
            ),
        );
    }
}

#[derive(Debug)]
enum PriceSource {
    Pool,
    Api,
}

/// Get current price for a token with priority: Pool > Fresh API
async fn get_current_price(mint: &str) -> Option<(f64, PriceSource)> {
    // Priority 1: Try pool price (real-time on-chain data)
    if let Some(price_result) = pools::get_pool_price(mint) {
        if price_result.price_sol > 0.0 && price_result.price_sol.is_finite() {
            return Some((price_result.price_sol, PriceSource::Pool));
        }
    }

    // Priority 2: Try API price if fresh (updated within last 5 seconds)
    match tokens::get_full_token_async(mint).await {
        Ok(Some(token)) => {
            // Check if price is recent (within last 5 seconds)
            let now = chrono::Utc::now();
            let age_secs = now
                .signed_duration_since(token.updated_at)
                .num_seconds();

            if age_secs <= API_PRICE_MAX_AGE_SECS {
                if token.price_sol > 0.0 && token.price_sol.is_finite() {
                    return Some((token.price_sol, PriceSource::Api));
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            logger::debug(
                LogTag::Positions,
                &format!("Failed to get token data for {}: {}", mint, e),
            );
        }
    }

    None
}
