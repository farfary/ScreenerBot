//! Position lifecycle operations.
//!
//! High-level commands for opening, closing, partial-closing, and DCA-ing into
//! positions. Each operation acquires the position lock, executes the swap via
//! the configured router, and enqueues transaction verification. Price fetching
//! with API fallback and position persistence with retry are also handled here.

mod close;
mod dca;
mod open;
mod partial_close;

pub use close::close_position_direct;
pub use dca::add_to_position;
pub use open::{open_position_direct, open_position_with_size};
pub use partial_close::partial_close_position;

use crate::logger::{self, LogTag};
use crate::positions::db::{save_position, update_position_price_fields};
use crate::positions::state::acquire_position_lock;
use crate::positions::types::Position;
use chrono::Utc;
use serde_json::json;
use tokio::time::{sleep, Duration};

const SOLANA_BLOCKHASH_VALIDITY_SLOTS: u64 = 150;
const POSITION_SAVE_MAX_RETRIES: usize = 5;
const POSITION_SAVE_BASE_BACKOFF_MS: u64 = 200;
const POSITION_SAVE_MAX_BACKOFF_MS: u64 = 3_000;

// =============================================================================
// PERSISTENCE HELPERS
// =============================================================================

fn position_save_backoff_ms(attempt: usize) -> u64 {
    let exp = attempt.saturating_sub(1) as u32;
    let growth = 1_u64 << exp.min(12);
    (POSITION_SAVE_BASE_BACKOFF_MS * growth).min(POSITION_SAVE_MAX_BACKOFF_MS)
}

async fn persist_position_with_retry(position: &Position) -> i64 {
    let mut attempt = 1;
    loop {
        match save_position(position).await {
            Ok(id) => {
                if attempt > 1 {
                    logger::info(
                        LogTag::Positions,
                        &format!(
                            "Recovered after {} persistence retries for mint {} (ID {})",
                            attempt - 1,
                            position.mint,
                            id
                        ),
                    );
                }
                return id;
            }
            Err(err) => {
                logger::warning(
                    LogTag::Positions,
                    &format!(
                        "Persisting position for mint {} failed on attempt {}: {}",
                        position.mint, attempt, err
                    ),
                );

                if attempt >= POSITION_SAVE_MAX_RETRIES {
                    crate::events::record_position_event_flexible(
                        "open_persist_failed",
                        crate::events::Severity::Error,
                        Some(&position.mint),
                        None,
                        json!({
                          "mint": position.mint,
                          "attempts": attempt,
                          "error": err,
                        }),
                    )
                    .await;

                    let fatal = format!(
            "Critical: failed to persist position for mint {} after {} attempts. Manual intervention required.",
            position.mint, attempt
          );
                    logger::error(LogTag::Positions, &fatal);

                    panic!("{fatal}");
                }

                let backoff = position_save_backoff_ms(attempt);
                sleep(Duration::from_millis(backoff)).await;
                attempt += 1;
            }
        }
    }
}

// =============================================================================
// PRICE CALCULATION HELPERS
// =============================================================================

/// Update position's current price and track high/low for trailing stop
pub async fn update_position_price(token_mint: &str, current_price: f64) -> Result<(), String> {
    if !current_price.is_finite() || current_price <= 0.0 {
        return Err(format!("Invalid price: {current_price}"));
    }

    let _lock = acquire_position_lock(token_mint).await;

    let now = Utc::now();
    let updated = crate::positions::state::update_position_state(token_mint, |pos| {
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
        return Err(format!("Position not found for mint: {token_mint}"));
    }

    let position = crate::positions::state::get_position_by_mint(token_mint).await;

    // Release per-mint lock before hitting the database to reduce contention
    drop(_lock);

    let position = position.ok_or_else(|| {
        format!(
            "Position disappeared after price update (mint: {})",
            token_mint
        )
    })?;

    update_position_price_fields(&position).await?;

    Ok(())
}
