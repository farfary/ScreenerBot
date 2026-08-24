//! Position price updater — refreshes current prices for all active positions.

use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions::get_open_positions;
use crate::positions::{Error, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::time::sleep;

/// Tracks whether we have already logged the "skipping while offline" notice, so
/// the per-second loop emits exactly one line on going offline and one on
/// resuming, instead of spamming every tick.
static OFFLINE_SKIP_LOGGED: AtomicBool = AtomicBool::new(false);

/// Position price updater service
/// Updates all open positions' current_price every second. Price resolution is
/// delegated to the canonical `price_resolution::get_price_with_api_fallback`
/// (pool → fresh API → force-fetched API) so the updater and the trading paths
/// always agree on price.

const UPDATE_INTERVAL_SECS: u64 = 1;

pub async fn start_price_updater(mut shutdown: tokio::sync::watch::Receiver<bool>) {
    logger::info(
        LogTag::Positions,
        "Starting position price updater (1s interval)",
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

    // While the internet is confirmed offline, both price sources (pool RPC and
    // API) are unreachable, so a per-second refresh just times out on DNS and
    // floods the log. Skip the cycle and keep the last known prices; positions
    // refresh automatically once connectivity returns. Log once per transition.
    if crate::connectivity::is_network_offline() {
        if !OFFLINE_SKIP_LOGGED.swap(true, Ordering::Relaxed) {
            logger::info(
                LogTag::Positions,
                "Network offline - pausing position price updates (keeping last known prices)",
            );
        }
        return;
    }
    if OFFLINE_SKIP_LOGGED.swap(false, Ordering::Relaxed) {
        logger::info(
            LogTag::Positions,
            "Network restored - resuming position price updates",
        );
    }

    let mut updated_count = 0;
    let mut pool_price_count = 0;
    let mut api_price_count = 0;
    let mut api_fresh_count = 0;
    let mut failed_count = 0;

    for position in &positions {
        match get_current_price(&position.mint).await {
            Some((price, source)) => {
                // Use position ID to target the correct position (avoids updating
                // a closed position when multiple positions share the same mint)
                let position_id = position.id.unwrap_or(-1);
                match update_position_price_and_pnl(position_id, &position.mint, price).await {
                    Ok(_) => {
                        updated_count += 1;
                        match source {
                            PriceSource::Pool => pool_price_count += 1,
                            PriceSource::Api => api_price_count += 1,
                            PriceSource::ApiFresh => api_fresh_count += 1,
                        }
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::Positions,
                            &format!("[PRICE_UPD] Failed update {}: {}", position.symbol, e),
                        );
                        failed_count += 1;
                    }
                }
            }
            None => {
                // Log at info level to diagnose frozen prices
                let has_pool = pools::get_pool_price(&position.mint).is_some();
                logger::info(
                    LogTag::Positions,
                    &format!(
                        "[PRICE_UPD] No price for {} (mint={}...) pool_available={}",
                        position.symbol,
                        &position.mint[..8.min(position.mint.len())],
                        has_pool
                    ),
                );
                failed_count += 1;
            }
        }
    }

    // Log every cycle when there are open positions (info level for debugging)
    if !positions.is_empty() {
        logger::info(
            LogTag::Positions,
            &format!(
                "[PRICE_UPD] positions={} updated={} (pool={}, api={}, fresh={}) failed={}",
                positions.len(),
                updated_count,
                pool_price_count,
                api_price_count,
                api_fresh_count,
                failed_count
            ),
        );
    }
}

/// Atomically update position price and PnL in a single database operation
/// Uses position_id to target the specific position (critical when multiple
/// positions share the same token mint — e.g. re-entry after closing)
async fn update_position_price_and_pnl(
    position_id: i64,
    token_mint: &str,
    current_price: f64,
) -> Result<()> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(Error::InvalidPrice {
            mint: token_mint.to_owned(),
            price: current_price,
        });
    }

    let _lock = crate::positions::acquire_position_lock(token_mint).await;

    // Apply pool price bias correction (BUG-31: DAMM pools underestimate by ~5-6%)
    // Uses the ratio of actual swap price to pool price at entry time to correct ongoing bias
    let corrected_price = {
        let pos_opt = crate::positions::state::get_position_by_id(position_id).await;
        if let Some(ref pos) = pos_opt {
            crate::positions::price_resolution::apply_pool_bias_correction(
                current_price,
                pos.entry_price,
                pos.effective_entry_price,
            )
        } else {
            current_price
        }
    };

    let now = chrono::Utc::now();

    // Update by position ID to avoid updating a closed position with the same mint
    let updated = crate::positions::state::update_position_state_by_id(position_id, |pos| {
        pos.current_price = Some(corrected_price);
        pos.current_price_updated = Some(now);

        if corrected_price > pos.price_highest {
            pos.price_highest = corrected_price;
        }

        if corrected_price < pos.price_lowest || pos.price_lowest == 0.0 {
            pos.price_lowest = corrected_price;
        }
    })
    .await;

    if !updated {
        return Err(Error::NotFoundById { position_id });
    }

    // Get position by ID for PnL calculation
    let mut position = crate::positions::state::get_position_by_id(position_id)
        .await
        .ok_or(Error::NotFoundById { position_id })?;

    // Calculate PnL with the bias-corrected price.
    //
    // A wallet-derived round with no established cost basis has NO honest unrealized
    // P&L: `total_size_sol` is zero for it, so the calculation would report the whole
    // current value as pure profit. Leave both fields None and let the dashboard render
    // "—" rather than a plausible, permanently wrong number.
    let (pnl_sol, pnl_pct) = if position.has_trustworthy_pnl() {
        let (sol, pct) =
            crate::positions::calculate_position_pnl(&position, Some(corrected_price)).await;
        (Some(sol), Some(pct))
    } else {
        (None, None)
    };

    // Update PnL fields in memory
    position.unrealized_pnl = pnl_sol;
    position.unrealized_pnl_percent = pnl_pct;

    // Store back to in-memory state by position ID
    crate::positions::state::update_position_state_by_id(position_id, |pos| {
        pos.unrealized_pnl = pnl_sol;
        pos.unrealized_pnl_percent = pnl_pct;
    })
    .await;

    // Release per-mint lock before database write
    drop(_lock);

    // Single database write with all updated fields
    crate::positions::update_position(&position)
        .await
        .map_err(|e| {
            logger::warning(
                LogTag::Positions,
                &format!(
                    "Failed to sync price+PnL to database for {}: {}",
                    token_mint, e
                ),
            );
            e
        })?;

    Ok(())
}

/// Local classification of where a resolved price came from, used only for the
/// per-cycle pool/api/fresh logging counters.
#[derive(Debug)]
enum PriceSource {
    Pool,
    Api,
    ApiFresh,
}

/// Resolve the current price via the canonical resolver shared with the trading
/// paths (`price_resolution::get_price_with_api_fallback`). This is the single
/// source of truth for pool → fresh API → force-fetched API resolution; here we
/// only classify the result for the logging counters.
async fn get_current_price(mint: &str) -> Option<(f64, PriceSource)> {
    let (price_result, source) =
        crate::positions::price_resolution::get_price_with_api_fallback(mint).await?;

    if !(price_result.price_sol > 0.0 && price_result.price_sol.is_finite()) {
        return None;
    }

    let classified = match source {
        crate::positions::types::PriceSource::Pool => PriceSource::Pool,
        crate::positions::types::PriceSource::Api => {
            // The resolver tags a force-fetched price with source_pool "api_fresh".
            if price_result.source_pool.as_deref() == Some("api_fresh") {
                PriceSource::ApiFresh
            } else {
                PriceSource::Api
            }
        }
    };

    Some((price_result.price_sol, classified))
}
