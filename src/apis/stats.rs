/// API statistics tracking
use crate::events::{record_api_event, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub last_request_time: Option<DateTime<Utc>>,
    pub last_success_time: Option<DateTime<Utc>>,
    pub last_error_time: Option<DateTime<Utc>>,
    pub last_error_message: Option<String>,
    pub average_response_time_ms: f64,
}

impl Default for ApiStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            last_request_time: None,
            last_success_time: None,
            last_error_time: None,
            last_error_message: None,
            average_response_time_ms: 0.0,
        }
    }
}

/// Thread-safe API statistics tracker
pub struct ApiStatsTracker {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    last_request_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    last_success_time: Arc<RwLock<Option<DateTime<Utc>>>>,
    last_error: Arc<RwLock<Option<(DateTime<Utc>, String)>>>,
    avg_response_time: Arc<RwLock<f64>>,
}

impl Default for ApiStatsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiStatsTracker {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            last_request_time: Arc::new(RwLock::new(None)),
            last_success_time: Arc::new(RwLock::new(None)),
            last_error: Arc::new(RwLock::new(None)),
            avg_response_time: Arc::new(RwLock::new(0.0)),
        }
    }

    pub async fn record_request(&self, success: bool, response_time_ms: f64) {
        let now = Utc::now();
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        *self.last_request_time.write().await = Some(now);

        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
            *self.last_success_time.write().await = Some(now);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        // Update average response time
        let mut avg = self.avg_response_time.write().await;
        let total = self.total_requests.load(Ordering::Relaxed);
        let previous_total = (total - 1) as f64;
        let accumulated = *avg * previous_total;
        *avg = (accumulated + response_time_ms) / (total as f64);
    }

    pub async fn record_error(&self, error_message: String) {
        *self.last_error.write().await = Some((Utc::now(), error_message));
    }

    /// Record error with event logging (for important API failures)
    pub async fn record_error_with_event(
        &self,
        api_name: &str,
        action: &str,
        error_message: String,
    ) {
        self.record_error(error_message.clone()).await;

        // Record API error event (sampled - every 10th error to avoid spam)
        let failed = self.failed_requests.load(Ordering::Relaxed);
        if failed % 10 == 1 {
            tokio::spawn({
                let api = api_name.to_string();
                let act = action.to_string();
                let err = error_message;
                let total = self.total_requests.load(Ordering::Relaxed);
                let success_rate = self.success_rate();
                async move {
                    record_api_event(
                        &api,
                        &act,
                        Severity::Warn,
                        serde_json::json!({
                            "status": "error",
                            "error": err,
                            "total_requests": total,
                            "failed_count": failed,
                            "success_rate": format!("{:.1}%", success_rate),
                        }),
                    )
                    .await;
                }
            });
        }
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn get_stats(&self) -> ApiStats {
        let last_error_guard = self.last_error.read().await;
        let (last_error_time, last_error_message) = match &*last_error_guard {
            Some((time, msg)) => (Some(*time), Some(msg.clone())),
            None => (None, None),
        };

        ApiStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            last_request_time: *self.last_request_time.read().await,
            last_success_time: *self.last_success_time.read().await,
            last_error_time,
            last_error_message,
            average_response_time_ms: *self.avg_response_time.read().await,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            let successful = self.successful_requests.load(Ordering::Relaxed);
            (successful as f64 / total as f64) * 100.0
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            (hits as f64 / total as f64) * 100.0
        }
    }
}
