/// WebSocket event type definitions
///
/// Event structures for real-time WebSocket updates

use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

/// System event for WebSocket broadcasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub event_type: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

/// System status update event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatusEvent {
    pub services: ServiceStatusUpdate,
    pub metrics: Option<MetricsUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatusUpdate {
    pub tokens_system: bool,
    pub positions_system: bool,
    pub pool_service: bool,
    pub security_analyzer: bool,
    pub transactions_system: bool,
    pub all_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsUpdate {
    pub memory_usage_mb: u64,
    pub cpu_usage_percent: f32,
    pub rpc_calls_total: u64,
    pub rpc_calls_failed: u64,
    pub ws_connections: usize,
}
