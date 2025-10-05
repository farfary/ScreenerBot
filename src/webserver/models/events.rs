/// WebSocket event type definitions
///
/// Event structures for real-time WebSocket updates

use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

/// Base WebSocket message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub r#type: String,
    pub channel: String,
    pub timestamp: DateTime<Utc>,
    pub data: serde_json::Value,
}

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessageType {
    /// Subscription confirmation
    Subscribed {
        channels: Vec<String>,
    },

    /// Unsubscribe confirmation
    Unsubscribed {
        channels: Vec<String>,
    },

    /// Data update
    Update {
        channel: String,
        data: serde_json::Value,
    },

    /// Error message
    Error {
        code: String,
        message: String,
    },

    /// Ping/pong for keep-alive
    Ping,
    Pong,
}

// ================================================================================================
// Phase 1: System Status Events
// ================================================================================================

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
//         from_amount: f64,
//         to_amount: f64,
//     },
// }

impl WebSocketMessage {
    /// Create a new WebSocket message
    pub fn new(msg_type: String, channel: String, data: serde_json::Value) -> Self {
        Self {
            r#type: msg_type,
            channel,
            timestamp: Utc::now(),
            data,
        }
    }

    /// Create an update message
    pub fn update(channel: String, data: serde_json::Value) -> Self {
        Self::new("update".to_string(), channel, data)
    }

    /// Create an error message
    pub fn error(code: String, message: String) -> Self {
        Self::new(
            "error".to_string(),
            "system".to_string(),
            serde_json::json!({
                "code": code,
                "message": message
            })
        )
    }

    /// Create a subscribed confirmation
    pub fn subscribed(channels: Vec<String>) -> Self {
        Self::new(
            "subscribed".to_string(),
            "system".to_string(),
            serde_json::json!({
                "channels": channels
            })
        )
    }
}
