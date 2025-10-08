/// Central WebSocket Hub - Multiplexer and broadcaster
///
/// The WsHub is the core of the centralized WebSocket architecture.
/// It manages:
/// - Per-topic sequence counters
/// - Per-connection message queues with backpressure
/// - Broadcast routing to all active connections
/// - Filter application (future enhancement)
/// - Hub-level metrics
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
};

use super::message::WsEnvelope;
use super::metrics::HubMetrics;

// ============================================================================
// HUB TYPES
// ============================================================================

/// Connection ID (unique per WebSocket connection)
pub type ConnectionId = u64;

/// Per-connection sender (bounded channel)
pub type ConnectionSender = mpsc::Sender<WsEnvelope>;

// ============================================================================
// WS HUB
// ============================================================================

/// Central WebSocket hub
pub struct WsHub {
    /// Per-topic sequence counters
    sequences: RwLock<HashMap<String, AtomicU64>>,

    /// Active connections (connection_id → sender)
    connections: RwLock<HashMap<ConnectionId, ConnectionSender>>,

    /// Next connection ID
    next_conn_id: AtomicU64,

    /// Hub metrics
    metrics: Arc<HubMetrics>,

    /// Per-client buffer size (from config)
    buffer_size: usize,
}

impl WsHub {
    /// Create new hub
    pub fn new(buffer_size: usize) -> Arc<Self> {
        Arc::new(Self {
            sequences: RwLock::new(HashMap::new()),
            connections: RwLock::new(HashMap::new()),
            next_conn_id: AtomicU64::new(1),
            metrics: HubMetrics::new(),
            buffer_size,
        })
    }

    /// Get next sequence number for a topic
    pub fn next_seq(&self, topic: &str) -> u64 {
        // This is a simplified sync access for now
        // In production, you'd want to ensure proper ordering
        let sequences = self.sequences.blocking_read();
        if let Some(counter) = sequences.get(topic) {
            counter.fetch_add(1, Ordering::SeqCst)
        } else {
            drop(sequences);
            let mut sequences = self.sequences.blocking_write();
            sequences.insert(topic.to_string(), AtomicU64::new(1));
            0
        }
    }

    /// Register a new connection
    pub async fn register_connection(&self) -> (ConnectionId, mpsc::Receiver<WsEnvelope>) {
        let conn_id = self.next_conn_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel(self.buffer_size);

        self.connections.write().await.insert(conn_id, tx);
        self.metrics.connection_opened();

        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "WsHub: connection {} registered (active={})",
                    conn_id,
                    self.connections.read().await.len()
                ),
            );
        }

        (conn_id, rx)
    }

    /// Unregister a connection
    pub async fn unregister_connection(&self, conn_id: ConnectionId) {
        self.connections.write().await.remove(&conn_id);
        self.metrics.connection_closed();

        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "WsHub: connection {} unregistered (active={})",
                    conn_id,
                    self.connections.read().await.len()
                ),
            );
        }
    }

    /// Broadcast message to all connections
    pub async fn broadcast(&self, envelope: WsEnvelope) {
        let connections = self.connections.read().await;
        let conn_count = connections.len();

        if conn_count == 0 {
            return;
        }

        let mut sent = 0;
        let mut dropped = 0;

        for (conn_id, sender) in connections.iter() {
            match sender.try_send(envelope.clone()) {
                Ok(_) => {
                    sent += 1;
                    self.metrics.message_sent();
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    dropped += 1;
                    self.metrics.message_dropped(1);
                    if is_debug_webserver_enabled() {
                        log(
                            LogTag::Webserver,
                            "DEBUG",
                            &format!(
                                "WsHub: message dropped for connection {} (queue full)",
                                conn_id
                            ),
                        );
                    }
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // Connection closed, will be cleaned up later
                    dropped += 1;
                }
            }
        }

        if is_debug_webserver_enabled() && (sent > 0 || dropped > 0) {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "WsHub: broadcast {} (sent={}, dropped={})",
                    envelope.t, sent, dropped
                ),
            );
        }
    }

    /// Get hub metrics
    pub fn metrics(&self) -> Arc<HubMetrics> {
        self.metrics.clone()
    }

    /// Get active connection count
    pub async fn active_connections(&self) -> usize {
        self.connections.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webserver::ws::message::{Topic, WsEnvelope};

    #[tokio::test]
    async fn test_hub_registration() {
        let hub = WsHub::new(10);

        let (conn_id1, _rx1) = hub.register_connection().await;
        let (conn_id2, _rx2) = hub.register_connection().await;

        assert_eq!(hub.active_connections().await, 2);
        assert_ne!(conn_id1, conn_id2);

        hub.unregister_connection(conn_id1).await;
        assert_eq!(hub.active_connections().await, 1);
    }

    #[tokio::test]
    async fn test_hub_broadcast() {
        let hub = WsHub::new(10);

        let (_conn_id, mut rx) = hub.register_connection().await;

        let envelope = WsEnvelope::new(Topic::EventsNew, 1, serde_json::json!({"test": "data"}));

        hub.broadcast(envelope.clone()).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received.t, "events.new");
        assert_eq!(received.seq, 1);
    }

    #[test]
    fn test_sequence_counter() {
        let hub = WsHub::new(10);

        let seq1 = hub.next_seq("test.topic");
        let seq2 = hub.next_seq("test.topic");
        let seq3 = hub.next_seq("other.topic");

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(seq3, 0); // Different topic, separate counter
    }
}
