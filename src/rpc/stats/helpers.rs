//! RPC Stats Helper Utilities
//!
//! Provides helper types and functions for accessing RPC statistics
//! in a structured format suitable for API responses and monitoring.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::RpcStatsResponse;
use crate::rpc::global::try_get_rpc_client;

/// Aggregated call information for a single minute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMinuteBucket {
    pub minute_start: DateTime<Utc>,
    pub call_count: u64,
    pub error_count: u64,
    pub latency_sum_ms: u64,
}

/// Summary of an RPC session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcSessionSnapshot {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub total_calls: u64,
    pub total_errors: u64,
    pub calls_per_url: HashMap<String, u64>,
    pub calls_per_method: HashMap<String, u64>,
    pub errors_per_url: HashMap<String, u64>,
    pub errors_per_method: HashMap<String, u64>,
}

/// RPC statistics snapshot
///
/// Provides aggregated statistics from the SQLite-based stats system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcStats {
    /// Session ID
    pub session_id: String,
    /// Session startup time
    pub startup_time: DateTime<Utc>,
    /// Total calls
    pub total_calls: u64,
    /// Total errors
    pub total_errors: u64,
    /// Success rate (0-100)
    pub success_rate: f32,
    /// Average response time in milliseconds
    pub avg_latency_ms: f64,
    /// Uptime in seconds
    pub uptime_secs: u64,
    /// Calls in last minute
    pub calls_last_minute: u64,
    /// Provider count
    pub provider_count: usize,
    /// Healthy provider count
    pub healthy_provider_count: usize,
    /// Calls per URL (simplified - just provider counts)
    pub calls_per_url: HashMap<String, u64>,
    /// Errors per URL (simplified)
    pub errors_per_url: HashMap<String, u64>,
    /// Calls per method (simplified)
    pub calls_per_method: HashMap<String, u64>,
    /// Errors per method (simplified)
    pub errors_per_method: HashMap<String, u64>,
    /// Minute buckets (empty in current implementation)
    pub minute_buckets: Vec<RpcMinuteBucket>,
    /// Last session snapshot (None in current implementation)
    pub last_session: Option<RpcSessionSnapshot>,
}

impl RpcStats {
    /// Create from stats response
    pub fn from_response(response: RpcStatsResponse, startup: DateTime<Utc>) -> Self {
        Self {
            session_id: response.session_id.clone(),
            startup_time: startup,
            total_calls: response.total_calls,
            total_errors: response.total_errors,
            success_rate: response.success_rate as f32,
            avg_latency_ms: response.avg_latency_ms,
            uptime_secs: response.uptime_secs,
            calls_last_minute: response.calls_last_minute,
            provider_count: response.provider_count,
            healthy_provider_count: response.healthy_provider_count,
            calls_per_url: HashMap::new(),
            errors_per_url: HashMap::new(),
            calls_per_method: HashMap::new(),
            errors_per_method: HashMap::new(),
            minute_buckets: Vec::new(),
            last_session: None,
        }
    }

    /// Get total calls across all URLs
    pub fn total_calls(&self) -> u64 {
        self.total_calls
    }

    /// Get total errors across all URLs
    pub fn total_errors(&self) -> u64 {
        self.total_errors
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f32 {
        self.success_rate
    }

    /// Get average response time in milliseconds globally
    pub fn average_response_time_ms_global(&self) -> f64 {
        self.avg_latency_ms
    }

    /// Get calls per second since startup
    pub fn calls_per_second(&self) -> f64 {
        if self.uptime_secs > 0 {
            self.total_calls as f64 / self.uptime_secs as f64
        } else {
            0.0
        }
    }

    /// Get calls per minute for recent N minutes
    ///
    /// Uses calls_last_minute as approximation since detailed
    /// minute-by-minute data requires database access.
    pub fn calls_per_minute_recent(&self, _minutes: usize) -> f64 {
        self.calls_last_minute as f64
    }

    /// Get minute buckets
    pub fn get_minute_buckets(&self) -> Vec<RpcMinuteBucket> {
        self.minute_buckets.clone()
    }
}

/// Get global RPC statistics
///
/// Returns statistics using the RpcStats struct,
/// backed by the SQLite-based stats system.
pub fn get_global_rpc_stats() -> Option<RpcStats> {
    let client = try_get_rpc_client()?;

    // Get stats using block_in_place for sync access
    let response = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(client.get_stats())
    });

    // Estimate startup time from uptime
    let startup_time = Utc::now() - chrono::Duration::seconds(response.uptime_secs as i64);

    Some(RpcStats::from_response(response, startup_time))
}

/// Start RPC stats monitoring service
///
/// The stats system uses SQLite and automatically persists data,
/// so this is a lightweight monitoring task that logs warnings
/// when success rate drops below threshold.
pub async fn start_rpc_stats_auto_save_service(shutdown: std::sync::Arc<tokio::sync::Notify>) {
    use crate::logger::{self, LogTag};

    logger::info(LogTag::Rpc, "Starting RPC stats monitoring service");

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                logger::info(LogTag::Rpc, "RPC stats monitoring service stopping");
                break;
            }
            _ = interval.tick() => {
                // Stats are automatically persisted to SQLite by StatsManager
                // This loop now just monitors health
                if let Some(client) = try_get_rpc_client() {
                    let stats = client.get_stats().await;

                    if stats.success_rate < 90.0 {
                        logger::warning(
                            LogTag::Rpc,
                            &format!(
                                "RPC success rate low: {:.1}% ({} errors / {} calls)",
                                stats.success_rate, stats.total_errors, stats.total_calls
                            ),
                        );
                    }
                }
            }
        }
    }

    logger::info(LogTag::Rpc, "RPC stats monitoring service stopped");
}

/// Parse pubkey helper (delegate to utils)
pub fn parse_pubkey(address: &str) -> Result<solana_sdk::pubkey::Pubkey, String> {
    crate::utils::parse_pubkey_safe(address)
}

/// Return the SPL Token program id (use constant)
pub fn spl_token_program_id() -> &'static str {
    crate::constants::SPL_TOKEN_PROGRAM_ID
}
