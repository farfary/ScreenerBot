//! Statistics management for RPC module
//!
//! Provides SQLite-backed statistics storage with:
//! - Per-provider call tracking
//! - Per-method statistics
//! - Time-series data (calls per minute)
//! - Background collection with buffering

pub mod collector;
pub mod database;
pub mod helpers;
pub mod types;

pub use collector::{StatsCollector, StatsMessage};
pub use database::{get_rpc_stats_db_path, RpcStatsDatabase};
pub use helpers::{
    get_global_rpc_stats, parse_pubkey, spl_token_program_id, start_rpc_stats_auto_save_service,
    RpcMinuteBucket, RpcSessionSnapshot, RpcStats,
};
pub use types::*;

use std::sync::Arc;
use tokio::sync::{mpsc, Notify, RwLock};
use uuid::Uuid;

use crate::rpc::types::{CircuitState, ProviderKind, RpcCallResult};

/// Statistics manager
pub struct StatsManager {
    /// Database
    db: Arc<RpcStatsDatabase>,
    /// Current session ID
    session_id: String,
    /// Collector channel
    collector_tx: Option<mpsc::Sender<StatsMessage>>,
    /// Shutdown signal
    shutdown: Arc<Notify>,
    /// Is running
    is_running: RwLock<bool>,
    /// Whether stats collection is enabled
    enabled: bool,
}

impl StatsManager {
    /// Create new stats manager
    pub async fn new() -> Result<Self, String> {
        let enabled = crate::config::with_config(|cfg| cfg.rpc.stats_enabled);

        let db = Arc::new(RpcStatsDatabase::new()?);
        let session_id = format!(
            "session_{}",
            Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("unknown")
        );

        // Start new session
        db.start_session(&session_id)?;

        Ok(Self {
            db,
            session_id,
            collector_tx: None,
            shutdown: Arc::new(Notify::new()),
            is_running: RwLock::new(false),
            enabled,
        })
    }

    /// Create with existing database
    pub async fn with_database(db: Arc<RpcStatsDatabase>) -> Result<Self, String> {
        let enabled = crate::config::with_config(|cfg| cfg.rpc.stats_enabled);

        let session_id = format!(
            "session_{}",
            Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap_or("unknown")
        );
        db.start_session(&session_id)?;

        Ok(Self {
            db,
            session_id,
            collector_tx: None,
            shutdown: Arc::new(Notify::new()),
            is_running: RwLock::new(false),
            enabled,
        })
    }

    /// Start background collector
    pub async fn start(&mut self) {
        // Skip if stats collection is disabled
        if !self.enabled {
            return;
        }

        let mut is_running = self.is_running.write().await;
        if *is_running {
            return;
        }

        let tx = StatsCollector::spawn_channel_collector(
            self.db.clone(),
            &self.session_id,
            self.shutdown.clone(),
        );
        self.collector_tx = Some(tx);
        *is_running = true;
    }

    /// Stop background collector
    pub async fn stop(&mut self) {
        let mut is_running = self.is_running.write().await;
        if !*is_running {
            return;
        }

        // Signal shutdown
        self.shutdown.notify_waiters();

        // Send shutdown message
        if let Some(tx) = &self.collector_tx {
            let _ = tx.send(StatsMessage::Shutdown).await;
        }

        // End session
        let _ = self.db.end_session(&self.session_id);

        self.collector_tx = None;
        *is_running = false;
    }

    /// Record a call result
    pub async fn record_call(&self, result: RpcCallResult) {
        // Skip if stats collection is disabled
        if !self.enabled {
            return;
        }

        let record = RpcCallRecord {
            provider_id: result.provider_id,
            method: result.method.to_string(),
            success: result.success,
            latency_ms: result.latency_ms,
            error_code: None, // Could be parsed from error
            error_message: result.error,
            was_retried: result.retry_count > 0,
            retry_count: result.retry_count,
            was_rate_limited: result.was_rate_limited,
            timestamp: result.timestamp,
        };

        if let Some(tx) = &self.collector_tx {
            let _ = tx.send(StatsMessage::RecordCall(record)).await;
        } else {
            // Direct write if no collector
            let _ = self.db.record_call(&self.session_id, &record);
        }
    }

    /// Register a provider
    pub fn register_provider(
        &self,
        id: &str,
        url_masked: &str,
        kind: ProviderKind,
        priority: u8,
    ) {
        let _ = self.db.upsert_provider(id, url_masked, kind, priority);
    }

    /// Update provider health
    pub fn update_provider_health(
        &self,
        provider_id: &str,
        circuit_state: CircuitState,
        consecutive_failures: u32,
        consecutive_successes: u32,
        avg_latency_ms: f64,
        current_rate_limit: u32,
        base_rate_limit: u32,
        last_error: Option<&str>,
    ) {
        let _ = self.db.update_provider_health(
            provider_id,
            circuit_state,
            consecutive_failures,
            consecutive_successes,
            avg_latency_ms,
            current_rate_limit,
            base_rate_limit,
            last_error,
        );
    }

    /// Get current session stats
    pub fn get_session_stats(&self) -> Option<SessionStats> {
        self.db.get_session_stats(&self.session_id).ok().flatten()
    }

    /// Get calls per minute (last 60 minutes)
    pub fn get_calls_per_minute(&self) -> Vec<TimeBucketStats> {
        self.db
            .get_calls_per_minute(&self.session_id, 60)
            .unwrap_or_default()
    }

    /// Get method stats (top 10)
    pub fn get_method_stats(&self) -> Vec<MethodStats> {
        self.db
            .get_method_stats(&self.session_id, 10)
            .unwrap_or_default()
    }

    /// Get stats response for API
    pub fn get_stats_response(
        &self,
        provider_count: usize,
        healthy_count: usize,
    ) -> RpcStatsResponse {
        let session = self.get_session_stats();
        let calls_per_minute = self.get_calls_per_minute();

        let calls_last_minute = calls_per_minute.first().map(|b| b.call_count).unwrap_or(0);

        match session {
            Some(stats) => RpcStatsResponse {
                session_id: stats.session_id,
                uptime_secs: stats.duration_secs,
                total_calls: stats.total_calls,
                total_errors: stats.total_errors,
                success_rate: if stats.total_calls > 0 {
                    100.0 * (stats.total_calls - stats.total_errors) as f64
                        / stats.total_calls as f64
                } else {
                    100.0
                },
                avg_latency_ms: self.db.get_avg_latency(&self.session_id).unwrap_or(0.0),
                provider_count,
                healthy_provider_count: healthy_count,
                calls_last_minute,
            },
            None => RpcStatsResponse {
                session_id: self.session_id.clone(),
                uptime_secs: 0,
                total_calls: 0,
                total_errors: 0,
                success_rate: 100.0,
                avg_latency_ms: 0.0,
                provider_count,
                healthy_provider_count: healthy_count,
                calls_last_minute: 0,
            },
        }
    }

    /// Cleanup old data
    pub fn cleanup(&self, retention_hours: u64) -> u64 {
        self.db.cleanup(retention_hours).unwrap_or(0)
    }

    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Flush pending stats
    pub async fn flush(&self) {
        if let Some(tx) = &self.collector_tx {
            let _ = tx.send(StatsMessage::Flush).await;
        }
    }
}

impl Drop for StatsManager {
    fn drop(&mut self) {
        // End session on drop
        let _ = self.db.end_session(&self.session_id);
    }
}
