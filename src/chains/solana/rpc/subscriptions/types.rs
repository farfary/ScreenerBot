//! Types for the shared RPC subscription transport.

use tokio::sync::mpsc;

/// One `logsNotification` delivered for a subscribed address.
#[derive(Debug, Clone)]
pub struct SubscriptionEvent {
    /// The subscribed address this event was delivered for (base58).
    pub address: String,
    /// Transaction signature (base58).
    pub signature: String,
    /// Slot from the notification context, when present.
    pub slot: Option<u64>,
    /// True when the notification carried a non-null `err` (the transaction failed on chain).
    pub failed: bool,
    /// Raw log lines exactly as delivered.
    pub logs: Vec<String>,
}

/// Health of the single shared WebSocket connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// No connection has been attempted yet, or the last subscriber went away.
    #[default]
    Idle,
    /// Socket is open.
    Connected,
    /// Socket dropped; a reconnect is scheduled or in progress.
    Reconnecting,
    /// Reconnect attempts have been failing continuously.
    Down,
}

/// Transport counters for service metrics.
#[derive(Debug, Clone, Default)]
pub struct SubscriptionMetrics {
    pub state: ConnectionState,
    pub active_addresses: u64,
    pub notifications_received: u64,
    pub reconnects: u64,
    /// Unix seconds of the last notification, when one has been received.
    pub last_message_unix: Option<i64>,
}

/// A live subscription to one address. Dropping it unsubscribes.
pub struct LogsSubscription {
    pub(super) id: u64,
    pub(super) address: String,
    pub(super) receiver: mpsc::UnboundedReceiver<SubscriptionEvent>,
    /// The actor's command channel, held so `Drop` can unsubscribe without
    /// contending for a lock — a missed unsubscribe would leak a server-side
    /// subscription for an address nobody is listening to any more.
    pub(super) commands: mpsc::UnboundedSender<super::client::Command>,
}

impl LogsSubscription {
    /// The subscribed address.
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Await the next notification. `None` means the transport dropped this subscription.
    pub async fn recv(&mut self) -> Option<SubscriptionEvent> {
        self.receiver.recv().await
    }
}
