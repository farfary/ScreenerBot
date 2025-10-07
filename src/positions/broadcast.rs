use super::types::Position;
use chrono::{DateTime, Utc};
use once_cell::sync::OnceCell;
use serde::Serialize;
use tokio::sync::broadcast;

// Broadcast channel capacity
const POSITIONS_BROADCAST_CAPACITY: usize = 1000;

// Global broadcaster
static POSITIONS_BROADCAST_TX: OnceCell<broadcast::Sender<PositionUpdate>> = OnceCell::new();

/// Position update types for WebSocket broadcasting
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PositionUpdate {
    /// New position opened
    Opened {
        position: Position,
        timestamp: DateTime<Utc>,
    },
    /// Position updated (P&L, status, etc.)
    Updated {
        position: Position,
        timestamp: DateTime<Utc>,
    },
    /// Position closed
    Closed {
        position: Position,
        timestamp: DateTime<Utc>,
    },
    /// Wallet balance changed
    BalanceChanged {
        sol: f64,
        usdc: f64,
        timestamp: DateTime<Utc>,
    },
}

/// Initialize the positions broadcast system
/// Returns the receiver for the first subscriber (dropped if not used)
pub fn initialize_positions_broadcaster() -> broadcast::Receiver<PositionUpdate> {
    let (tx, rx) = broadcast::channel(POSITIONS_BROADCAST_CAPACITY);

    match POSITIONS_BROADCAST_TX.set(tx) {
        Ok(_) => {
            log::info!(
                "✅ Positions broadcast system initialized (capacity: {})",
                POSITIONS_BROADCAST_CAPACITY
            );
            rx
        }
        Err(_) => {
            log::warn!("⚠️ Positions broadcaster already initialized");
            // Return a new subscription if already initialized
            POSITIONS_BROADCAST_TX
                .get()
                .expect("Broadcaster exists")
                .subscribe()
        }
    }
}

/// Subscribe to position updates
/// Returns None if broadcaster not initialized
pub fn subscribe() -> Option<broadcast::Receiver<PositionUpdate>> {
    POSITIONS_BROADCAST_TX.get().map(|tx| tx.subscribe())
}

/// Emit a position opened event
pub fn emit_position_opened(position: Position) {
    if let Some(tx) = POSITIONS_BROADCAST_TX.get() {
        let update = PositionUpdate::Opened {
            position,
            timestamp: Utc::now(),
        };

        // Non-blocking send (drops message if no subscribers)
        let _ = tx.send(update);
    }
}

/// Emit a position updated event
pub fn emit_position_updated(position: Position) {
    if let Some(tx) = POSITIONS_BROADCAST_TX.get() {
        let update = PositionUpdate::Updated {
            position,
            timestamp: Utc::now(),
        };

        let _ = tx.send(update);
    }
}

/// Emit a position closed event
pub fn emit_position_closed(position: Position) {
    if let Some(tx) = POSITIONS_BROADCAST_TX.get() {
        let update = PositionUpdate::Closed {
            position,
            timestamp: Utc::now(),
        };

        let _ = tx.send(update);
    }
}

/// Emit a balance changed event
pub fn emit_balance_changed(sol: f64, usdc: f64) {
    if let Some(tx) = POSITIONS_BROADCAST_TX.get() {
        let update = PositionUpdate::BalanceChanged {
            sol,
            usdc,
            timestamp: Utc::now(),
        };

        let _ = tx.send(update);
    }
}

/// Get broadcast statistics (subscriber count)
pub fn get_subscriber_count() -> usize {
    POSITIONS_BROADCAST_TX
        .get()
        .map(|tx| tx.receiver_count())
        .unwrap_or(0)
}
