use crate::pools::types::PriceResult;
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use serde::Serialize;
use tokio::sync::broadcast;

// Broadcast channel capacity (higher for price updates - frequent events)
const PRICES_BROADCAST_CAPACITY: usize = 5000;

// Global broadcaster
static PRICES_BROADCAST_TX: OnceCell<broadcast::Sender<PriceUpdate>> = OnceCell::new();

/// Price update for WebSocket broadcasting
#[derive(Clone, Debug, Serialize)]
pub struct PriceUpdate {
    pub mint: String,
    pub price_result: PriceResult,
    pub timestamp: DateTime<Utc>,
}

/// Initialize the prices broadcast system
/// Returns the receiver for the first subscriber (dropped if not used)
pub fn initialize_prices_broadcaster() -> broadcast::Receiver<PriceUpdate> {
    let (tx, rx) = broadcast::channel(PRICES_BROADCAST_CAPACITY);

    match PRICES_BROADCAST_TX.set(tx) {
        Ok(_) => {
            log::info!(
                "✅ Prices broadcast system initialized (capacity: {})",
                PRICES_BROADCAST_CAPACITY
            );
            rx
        }
        Err(_) => {
            log::warn!("⚠️ Prices broadcaster already initialized");
            // Return a new subscription if already initialized
            PRICES_BROADCAST_TX
                .get()
                .expect("Broadcaster exists")
                .subscribe()
        }
    }
}

/// Subscribe to price updates
/// Returns None if broadcaster not initialized
pub fn subscribe() -> Option<broadcast::Receiver<PriceUpdate>> {
    PRICES_BROADCAST_TX.get().map(|tx| tx.subscribe())
}

/// Emit a price update
pub fn emit_price_update(mint: String, price_result: PriceResult) {
    if let Some(tx) = PRICES_BROADCAST_TX.get() {
        let update = PriceUpdate {
            mint,
            price_result,
            timestamp: Utc::now(),
        };

        // Non-blocking send (drops message if no subscribers)
        let _ = tx.send(update);
    }
}

/// Get broadcast statistics (subscriber count)
pub fn get_subscriber_count() -> usize {
    PRICES_BROADCAST_TX
        .get()
        .map(|tx| tx.receiver_count())
        .unwrap_or(0)
}
