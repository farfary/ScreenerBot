use serde::Serialize;
/// WebSocket metrics collection
///
/// Per-connection statistics for monitoring and debugging.
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

// ============================================================================
// CONNECTION METRICS
// ============================================================================

/// Per-connection metrics (thread-safe)
#[derive(Debug)]
pub struct ConnectionMetrics {
    /// Total messages sent
    messages_sent: AtomicU64,

    /// Total messages dropped (backpressure)
    messages_dropped: AtomicU64,

    /// Total lag events (receiver lagged)
    lag_events: AtomicU64,

    /// Current queue size
    queue_size: AtomicUsize,

    /// Peak queue size
    peak_queue_size: AtomicUsize,

    /// Total backpressure warnings sent
    backpressure_warnings: AtomicU64,
}

impl ConnectionMetrics {
    /// Create new metrics tracker
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            messages_sent: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            lag_events: AtomicU64::new(0),
            queue_size: AtomicUsize::new(0),
            peak_queue_size: AtomicUsize::new(0),
            backpressure_warnings: AtomicU64::new(0),
        })
    }

    /// Increment messages sent
    pub fn inc_sent(&self) {
        self.messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment messages dropped
    pub fn inc_dropped(&self, count: u64) {
        self.messages_dropped.fetch_add(count, Ordering::Relaxed);
    }

    /// Increment lag events
    pub fn inc_lag(&self) {
        self.lag_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Update queue size
    pub fn set_queue_size(&self, size: usize) {
        self.queue_size.store(size, Ordering::Relaxed);

        // Update peak if needed
        let current_peak = self.peak_queue_size.load(Ordering::Relaxed);
        if size > current_peak {
            self.peak_queue_size.store(size, Ordering::Relaxed);
        }
    }

    /// Increment backpressure warnings
    pub fn inc_backpressure_warnings(&self) {
        self.backpressure_warnings.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot for API
    pub fn snapshot(&self) -> ConnectionMetricsSnapshot {
        ConnectionMetricsSnapshot {
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_dropped: self.messages_dropped.load(Ordering::Relaxed),
            lag_events: self.lag_events.load(Ordering::Relaxed),
            queue_size: self.queue_size.load(Ordering::Relaxed),
            peak_queue_size: self.peak_queue_size.load(Ordering::Relaxed),
            backpressure_warnings: self.backpressure_warnings.load(Ordering::Relaxed),
        }
    }
}

impl Default for ConnectionMetrics {
    fn default() -> Self {
        Self {
            messages_sent: AtomicU64::new(0),
            messages_dropped: AtomicU64::new(0),
            lag_events: AtomicU64::new(0),
            queue_size: AtomicUsize::new(0),
            peak_queue_size: AtomicUsize::new(0),
            backpressure_warnings: AtomicU64::new(0),
        }
    }
}

/// Metrics snapshot (serializable)
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionMetricsSnapshot {
    pub messages_sent: u64,
    pub messages_dropped: u64,
    pub lag_events: u64,
    pub queue_size: usize,
    pub peak_queue_size: usize,
    pub backpressure_warnings: u64,
}

// ============================================================================
// HUB METRICS
// ============================================================================

/// Hub-level metrics (aggregate across all connections)
#[derive(Debug)]
pub struct HubMetrics {
    /// Total connections (lifetime)
    total_connections: AtomicU64,

    /// Current active connections
    active_connections: AtomicUsize,

    /// Total messages sent (all connections)
    total_messages_sent: AtomicU64,

    /// Total messages dropped (all connections)
    total_messages_dropped: AtomicU64,
}

impl HubMetrics {
    /// Create new hub metrics
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            total_messages_sent: AtomicU64::new(0),
            total_messages_dropped: AtomicU64::new(0),
        })
    }

    /// Record new connection
    pub fn connection_opened(&self) {
        self.total_connections.fetch_add(1, Ordering::Relaxed);
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Record connection closed
    pub fn connection_closed(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Record message sent
    pub fn message_sent(&self) {
        self.total_messages_sent.fetch_add(1, Ordering::Relaxed);
    }

    /// Record message dropped
    pub fn message_dropped(&self, count: u64) {
        self.total_messages_dropped
            .fetch_add(count, Ordering::Relaxed);
    }

    /// Get snapshot
    pub fn snapshot(&self) -> HubMetricsSnapshot {
        HubMetricsSnapshot {
            total_connections: self.total_connections.load(Ordering::Relaxed),
            active_connections: self.active_connections.load(Ordering::Relaxed),
            total_messages_sent: self.total_messages_sent.load(Ordering::Relaxed),
            total_messages_dropped: self.total_messages_dropped.load(Ordering::Relaxed),
        }
    }
}

impl Default for HubMetrics {
    fn default() -> Self {
        Self {
            total_connections: AtomicU64::new(0),
            active_connections: AtomicUsize::new(0),
            total_messages_sent: AtomicU64::new(0),
            total_messages_dropped: AtomicU64::new(0),
        }
    }
}

/// Hub metrics snapshot
#[derive(Debug, Clone, Serialize)]
pub struct HubMetricsSnapshot {
    pub total_connections: u64,
    pub active_connections: usize,
    pub total_messages_sent: u64,
    pub total_messages_dropped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_metrics() {
        let metrics = ConnectionMetrics::new();

        metrics.inc_sent();
        metrics.inc_sent();
        metrics.inc_dropped(5);
        metrics.set_queue_size(10);
        metrics.set_queue_size(20);
        metrics.set_queue_size(15);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.messages_sent, 2);
        assert_eq!(snapshot.messages_dropped, 5);
        assert_eq!(snapshot.queue_size, 15);
        assert_eq!(snapshot.peak_queue_size, 20);
    }

    #[test]
    fn test_hub_metrics() {
        let metrics = HubMetrics::new();

        metrics.connection_opened();
        metrics.connection_opened();
        metrics.message_sent();
        metrics.message_sent();
        metrics.message_dropped(3);
        metrics.connection_closed();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.total_connections, 2);
        assert_eq!(snapshot.active_connections, 1);
        assert_eq!(snapshot.total_messages_sent, 2);
        assert_eq!(snapshot.total_messages_dropped, 3);
    }
}
