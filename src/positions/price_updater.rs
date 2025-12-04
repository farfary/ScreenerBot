use crate::logger::{self, LogTag};
use crate::pools;
use crate::positions::state::update_position_state;
use crate::positions::{get_open_positions, update_position_price};
use crate::tokens;
use std::time::Duration;
use tokio::time::sleep;

/// Position price updater service
/// Updates all open positions'current_price every second
/// Priority: Pool price (real-time) > API price (if fresh < 5s)

const UPDATE_INTERVAL_SECS: u64 = 1;
const API_PRICE_MAX_AGE_SECS: i64 = 5;

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

  let mut updated_count = 0;
  let mut pool_price_count = 0;
  let mut api_price_count = 0;
  let mut failed_count = 0;

  for position in positions {
    match get_current_price(&position.mint).await {
      Some((price, source)) => {
        // Atomically update both price AND PnL in single operation
        match update_position_price_and_pnl(&position.mint, price).await {
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
              &format!("Failed to update price+PnL for {}: {}", position.symbol, e),
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
        "Price+PnL update: updated={} (pool={}, api={}) failed={}",
        updated_count, pool_price_count, api_price_count, failed_count
      ),
    );
  }
}

/// Atomically update position price and PnL in a single database operation
/// This eliminates the race condition and reduces DB writes from 2 to 1 per position
async fn update_position_price_and_pnl(token_mint: &str, current_price: f64) -> Result<(), String> {
  if !current_price.is_finite() || current_price <= 0.0 {
    return Err(format!("Invalid price: {}", current_price));
  }

  let _lock = crate::positions::acquire_position_lock(token_mint).await;

  let now = chrono::Utc::now();

  // First, update price in memory and get the updated position
  let updated = update_position_state(token_mint, |pos| {
    pos.current_price = Some(current_price);
    pos.current_price_updated = Some(now);

    if current_price > pos.price_highest {
      pos.price_highest = current_price;
    }

    if current_price < pos.price_lowest || pos.price_lowest == 0.0 {
      pos.price_lowest = current_price;
    }
  })
  .await;

  if !updated {
    return Err(format!("Position not found for mint: {}", token_mint));
  }

  // Get updated position for PnL calculation
  let mut position = crate::positions::get_position_by_mint(token_mint)
    .await
    .ok_or_else(|| format!("Position disappeared after price update: {}", token_mint))?;

  // Calculate PnL with the new price
  let (pnl_sol, pnl_pct) =
    crate::positions::calculate_position_pnl(&position, Some(current_price)).await;

  // Update PnL fields in memory
  position.unrealized_pnl = Some(pnl_sol);
  position.unrealized_pnl_percent = Some(pnl_pct);

  // Store back to in-memory state
  update_position_state(token_mint, |pos| {
    pos.unrealized_pnl = Some(pnl_sol);
    pos.unrealized_pnl_percent = Some(pnl_pct);
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
        .signed_duration_since(token.market_data_last_fetched_at)
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
