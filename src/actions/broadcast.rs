//! Broadcasting channel for action updates
//!
//! Provides pub/sub mechanism for real-time action updates to clients via SSE

use super::types::ActionUpdate;
use once_cell::sync::Lazy;
use tokio::sync::broadcast;

static ACTION_BROADCAST: Lazy<broadcast::Sender<ActionUpdate>> = Lazy::new(|| {
    let (tx, _) = broadcast::channel(1000);
    tx
});

/// Broadcast an action update to all subscribers
pub async fn broadcast_update(update: ActionUpdate) {
    // Send update to broadcast channel (non-blocking)
    // If no receivers, message is dropped (that's fine)
    let _ = ACTION_BROADCAST.send(update);
}

/// Subscribe to action updates (for SSE clients)
pub fn subscribe() -> broadcast::Receiver<ActionUpdate> {
    ACTION_BROADCAST.subscribe()
}

/// Get subscriber count (for monitoring)
pub fn subscriber_count() -> usize {
    ACTION_BROADCAST.receiver_count()
}
