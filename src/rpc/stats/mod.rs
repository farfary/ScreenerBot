// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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
pub use database::RpcStatsDatabase;
pub use helpers::{
    get_global_rpc_stats, start_rpc_stats_auto_save_service, RpcMinuteBucket, RpcSessionSnapshot,
    RpcStats,
};
pub use types::*;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Notify, RwLock};
use uuid::Uuid;

use crate::rpc::types::{CircuitState, ProviderKind, RpcCallResult};

/// The trailing window of the displayed call rate.
const CALL_RATE_WINDOW: Duration = Duration::from_secs(60);

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
    /// When each call of the trailing window was recorded. The collector writes calls to the
    /// store in batches every few seconds, so a count read from the store trails the calls made.
    recent_calls: Mutex<VecDeque<Instant>>,
}

impl StatsManager {
    /// Create a stats manager; `enabled` gates collection for its lifetime
    pub async fn new(enabled: bool) -> crate::Result<Self> {
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
            recent_calls: Mutex::new(VecDeque::new()),
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

        {
            let now = Instant::now();
            let mut recent = self.recent_calls.lock().unwrap_or_else(|e| e.into_inner());
            recent.push_back(now);
            prune_call_window(&mut recent, now);
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
    pub fn register_provider(&self, id: &str, url_masked: &str, kind: ProviderKind, priority: u8) {
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
        // A trailing 60 s window, never a calendar minute: the last completed calendar minute
        // kept showing the startup burst for a minute after the rate fell, and a partial
        // first minute while it rose. A session younger than a minute counts every call so far.
        let calls_last_minute = {
            let mut recent = self.recent_calls.lock().unwrap_or_else(|e| e.into_inner());
            prune_call_window(&mut recent, Instant::now());
            recent.len() as u64
        };

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
                avg_latency_ms: self
                    .db
                    .get_avg_latency(&self.session_id)
                    .unwrap_or_default(),
                provider_count,
                healthy_provider_count: healthy_count,
                calls_last_minute,
                breakdown: self
                    .db
                    .get_call_breakdown(&self.session_id)
                    .unwrap_or_default(),
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
                breakdown: RpcCallBreakdown::default(),
            },
        }
    }

    /// Cleanup old data
    pub fn cleanup(&self, retention_hours: u64) -> u64 {
        self.db.cleanup(retention_hours).unwrap_or_default()
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

/// Drops the calls recorded a full window or longer before `now`.
fn prune_call_window(recent: &mut VecDeque<Instant>, now: Instant) {
    while let Some(&front) = recent.front() {
        if now.duration_since(front) >= CALL_RATE_WINDOW {
            recent.pop_front();
        } else {
            break;
        }
    }
}

impl Drop for StatsManager {
    fn drop(&mut self) {
        // End session on drop
        let _ = self.db.end_session(&self.session_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_call_window_keeps_the_calls_of_the_last_minute() {
        let now = Instant::now() + CALL_RATE_WINDOW * 2;
        let ago = |secs: u64| now - Duration::from_secs(secs);
        let mut recent = VecDeque::from([ago(90), ago(60), ago(59), ago(30), ago(0)]);

        prune_call_window(&mut recent, now);

        assert_eq!(recent, VecDeque::from([ago(59), ago(30), ago(0)]));
    }
}
