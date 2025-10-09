/// Central WebSocket Hub - Multiplexer and broadcaster
///
/// The WsHub is the core of the centralized WebSocket architecture.
/// It manages:
/// - Per-topic sequence counters
/// - Per-connection message queues with backpressure
/// - Broadcast routing to all active connections
/// - Filter application (future enhancement)
/// - Hub-level metrics
use std::collections::{HashMap, HashSet};
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
    sequences: RwLock<HashMap<String, Arc<AtomicU64>>>,

    /// Active connections (connection_id → sender)
    connections: RwLock<HashMap<ConnectionId, ConnectionSender>>,

    /// Topic subscriptions per connection (topic codes)
    connection_topics: RwLock<HashMap<ConnectionId, HashSet<String>>>,

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
            connection_topics: RwLock::new(HashMap::new()),
            next_conn_id: AtomicU64::new(1),
            metrics: HubMetrics::new(),
            buffer_size,
        })
    }

    /// Get next sequence number for a topic
    pub async fn next_seq(&self, topic: &str) -> u64 {
        if let Some(counter) = {
            let sequences = self.sequences.read().await;
            sequences.get(topic).cloned()
        } {
            return counter.fetch_add(1, Ordering::SeqCst);
        }

        let mut sequences = self.sequences.write().await;
        let counter = sequences
            .entry(topic.to_string())
            .or_insert_with(|| Arc::new(AtomicU64::new(0)))
            .clone();
        counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Register a new connection
    pub async fn register_connection(&self) -> (ConnectionId, mpsc::Receiver<WsEnvelope>) {
        let conn_id = self.next_conn_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel(self.buffer_size);

        self.connections.write().await.insert(conn_id, tx);
        self.connection_topics
            .write()
            .await
            .insert(conn_id, HashSet::new());
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
        self.connection_topics.write().await.remove(&conn_id);
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

        let topics_map = self.connection_topics.read().await;
        let topic_code = envelope.t.clone();

        let mut sent = 0;
        let mut dropped = 0;

        for (conn_id, sender) in connections.iter() {
            match topics_map.get(conn_id) {
                Some(topics) if topics.contains(&topic_code) => {}
                _ => continue,
            }

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
                            "TRACE",
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

        // Removed verbose per-broadcast logging - metrics are tracked in HubMetrics instead
    }

    /// Update the topic subscription set for a connection
    pub async fn update_connection_topics(&self, conn_id: ConnectionId, topics: HashSet<String>) {
        let mut map = self.connection_topics.write().await;
        if topics.is_empty() {
            map.remove(&conn_id);
        } else {
            map.insert(conn_id, topics);
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

    #[tokio::test]
    async fn test_sequence_counter() {
        let hub = WsHub::new(10);

        let seq1 = hub.next_seq("test.topic").await;
        let seq2 = hub.next_seq("test.topic").await;
        let seq3 = hub.next_seq("other.topic").await;

        assert_eq!(seq1, 0);
        assert_eq!(seq2, 1);
        assert_eq!(seq3, 0); // Different topic, separate counter
    }
}
