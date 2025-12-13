//! Background statistics collector

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Notify, RwLock};

use super::database::RpcStatsDatabase;
use super::types::*;

/// Message for stats collector channel
pub enum StatsMessage {
    RecordCall(RpcCallRecord),
    Flush,
    Shutdown,
}

/// Background stats collector
pub struct StatsCollector {
    /// Database
    db: Arc<RpcStatsDatabase>,
    /// Current session ID
    session_id: String,
    /// Pending records buffer
    pending: RwLock<VecDeque<RpcCallRecord>>,
    /// Flush interval
    flush_interval: Duration,
    /// Max buffer size before auto-flush
    max_buffer_size: usize,
}

impl StatsCollector {
    /// Create new collector
    pub fn new(db: Arc<RpcStatsDatabase>, session_id: &str) -> Self {
        Self {
            db,
            session_id: session_id.to_string(),
            pending: RwLock::new(VecDeque::with_capacity(100)),
            flush_interval: Duration::from_secs(5),
            max_buffer_size: 100,
        }
    }

    /// Create with custom settings
    pub fn with_settings(
        db: Arc<RpcStatsDatabase>,
        session_id: &str,
        flush_interval: Duration,
        max_buffer_size: usize,
    ) -> Self {
        Self {
            db,
            session_id: session_id.to_string(),
            pending: RwLock::new(VecDeque::with_capacity(max_buffer_size)),
            flush_interval,
            max_buffer_size,
        }
    }

    /// Add record to buffer
    pub async fn record(&self, record: RpcCallRecord) {
        let mut pending = self.pending.write().await;
        pending.push_back(record);

        // Auto-flush if buffer is full
        if pending.len() >= self.max_buffer_size {
            drop(pending);
            self.flush().await;
        }
    }

    /// Flush pending records to database
    pub async fn flush(&self) {
        let records: Vec<RpcCallRecord> = {
            let mut pending = self.pending.write().await;
            pending.drain(..).collect()
        };

        if records.is_empty() {
            return;
        }

        if let Err(e) = self.db.record_calls(&self.session_id, &records) {
            // Log error but don't fail - stats are not critical
            eprintln!("Failed to flush RPC stats: {}", e);
        }
    }

    /// Get pending count
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }

    /// Start background flush loop
    pub async fn start_background_loop(self: Arc<Self>, shutdown: Arc<Notify>) {
        let mut interval = tokio::time::interval(self.flush_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.flush().await;
                }
                _ = shutdown.notified() => {
                    // Final flush on shutdown
                    self.flush().await;
                    break;
                }
            }
        }
    }

    /// Create channel-based collector
    pub fn spawn_channel_collector(
        db: Arc<RpcStatsDatabase>,
        session_id: &str,
        shutdown: Arc<Notify>,
    ) -> mpsc::Sender<StatsMessage> {
        let (tx, mut rx) = mpsc::channel::<StatsMessage>(1000);
        let collector = Arc::new(Self::new(db, session_id));

        tokio::spawn(async move {
            let mut flush_interval = tokio::time::interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => {
                        match msg {
                            StatsMessage::RecordCall(record) => {
                                collector.record(record).await;
                            }
                            StatsMessage::Flush => {
                                collector.flush().await;
                            }
                            StatsMessage::Shutdown => {
                                collector.flush().await;
                                break;
                            }
                        }
                    }
                    _ = flush_interval.tick() => {
                        collector.flush().await;
                    }
                    _ = shutdown.notified() => {
                        collector.flush().await;
                        break;
                    }
                }
            }
        });

        tx
    }
}

impl std::fmt::Debug for StatsCollector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatsCollector")
            .field("session_id", &self.session_id)
            .field("flush_interval", &self.flush_interval)
            .field("max_buffer_size", &self.max_buffer_size)
            .finish()
    }
}
