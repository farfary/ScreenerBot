/// WebSocket message schema - Standard envelope and control messages
///
/// All WebSocket messages follow a consistent envelope format with:
/// - Protocol version
/// - Topic routing
/// - Timestamp and sequence number
/// - Optional routing key (mint/pool/service ID)
/// - Typed data payload
/// - Optional metadata (snapshot markers, backpressure warnings)
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// PROTOCOL VERSION
// ============================================================================

pub const PROTOCOL_VERSION: u8 = 1;

// ============================================================================
// TOPIC ENUM
// ============================================================================

/// Topic codes for routing messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    // System topics
    SystemStatus,
    ServicesMetrics,

    // Trading topics
    PositionsUpdate,
    TokensUpdate,

    // Activity topics
    EventsNew,
    OhlcvsUpdate,
    TraderState,
    WalletBalances,
    TransactionsActivity,
    SecurityAlerts,
}

impl Topic {
    /// Get topic code string (used in envelope)
    pub fn code(&self) -> &'static str {
        match self {
            Topic::SystemStatus => "system.status",
            Topic::ServicesMetrics => "services.metrics",
            Topic::PositionsUpdate => "positions.update",
            Topic::TokensUpdate => "tokens.update",
            Topic::EventsNew => "events.new",
            Topic::OhlcvsUpdate => "ohlcvs.update",
            Topic::TraderState => "trader.state",
            Topic::WalletBalances => "wallet.balances",
            Topic::TransactionsActivity => "transactions.activity",
            Topic::SecurityAlerts => "security.alerts",
        }
    }

    /// Parse topic from code string
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "system.status" => Some(Topic::SystemStatus),
            "services.metrics" => Some(Topic::ServicesMetrics),
            "positions.update" => Some(Topic::PositionsUpdate),
            "tokens.update" => Some(Topic::TokensUpdate),
            "events.new" => Some(Topic::EventsNew),
            "ohlcvs.update" => Some(Topic::OhlcvsUpdate),
            "trader.state" => Some(Topic::TraderState),
            "wallet.balances" => Some(Topic::WalletBalances),
            "transactions.activity" => Some(Topic::TransactionsActivity),
            "security.alerts" => Some(Topic::SecurityAlerts),
            _ => None,
        }
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

// ============================================================================
// MESSAGE ENVELOPE
// ============================================================================

/// Standard WebSocket message envelope
///
/// All messages from server to client use this format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsEnvelope {
    /// Protocol version
    pub v: u8,

    /// Topic code (e.g., "positions.update")
    pub t: String,

    /// Server timestamp (unix milliseconds)
    pub ts: i64,

    /// Sequence number (monotonic per topic)
    pub seq: u64,

    /// Routing key (mint/pool/service ID, null for broadcast)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// Message payload (topic-specific)
    pub data: serde_json::Value,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<MessageMetadata>,
}

/// Message metadata (optional)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// True if this is a snapshot (initial state)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<bool>,

    /// Number of messages dropped due to backpressure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dropped: Option<u64>,

    /// Additional context (free-form)
    #[serde(flatten)]
    pub extra: Option<serde_json::Map<String, serde_json::Value>>,
}

// ============================================================================
// CLIENT MESSAGES (Client → Server)
// ============================================================================

/// Client control messages
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Client hello (initial handshake)
    Hello {
        #[serde(default)]
        client_id: Option<String>,
        #[serde(default)]
        app_version: Option<String>,
        #[serde(default)]
        pages_supported: Vec<String>,
    },

    /// Set filters for topics
    SetFilters {
        /// Per-topic filters (topic code → filter object)
        topics: serde_json::Map<String, serde_json::Value>,
    },

    /// Pause streaming for specific topics (or all if empty)
    Pause {
        #[serde(default)]
        topics: Vec<String>,
    },

    /// Resume streaming for specific topics (or all if empty)
    Resume {
        #[serde(default)]
        topics: Vec<String>,
    },

    /// Request resync with last known sequence numbers
    Resync {
        /// Topic code → last known sequence number
        topics: serde_json::Map<String, serde_json::Value>,
    },

    /// Client ping (keepalive)
    Ping {
        #[serde(default)]
        id: Option<String>,
    },
}

// ============================================================================
// SERVER MESSAGES (Server → Client)
// ============================================================================

/// Server control messages
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Data message (uses WsEnvelope)
    Data(WsEnvelope),

    /// Acknowledge control message
    Ack {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<serde_json::Value>,
    },

    /// Error response
    Error {
        message: String,
        code: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<serde_json::Value>,
    },

    /// Pong response to ping
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },

    /// Backpressure warning
    Backpressure {
        topic: String,
        dropped: u64,
        queue_size: usize,
        recommendation: String,
    },

    /// Snapshot begin marker
    SnapshotBegin {
        topic: String,
        total: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<serde_json::Value>,
    },

    /// Snapshot end marker
    SnapshotEnd {
        topic: String,
        sent: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<serde_json::Value>,
    },
}

// ============================================================================
// HELPERS
// ============================================================================

impl WsEnvelope {
    /// Create a new envelope with current timestamp
    pub fn new(topic: Topic, seq: u64, data: serde_json::Value) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            t: topic.code().to_string(),
            ts: chrono::Utc::now().timestamp_millis(),
            seq,
            key: None,
            data,
            meta: None,
        }
    }

    /// Set routing key
    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Set metadata
    pub fn with_meta(mut self, meta: MessageMetadata) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Mark as snapshot
    pub fn as_snapshot(mut self) -> Self {
        let mut meta = self.meta.take().unwrap_or_default();
        meta.snapshot = Some(true);
        self.meta = Some(meta);
        self
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            snapshot: None,
            dropped: None,
            extra: None,
        }
    }
}

impl ServerMessage {
    /// Serialize to JSON text
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_code_roundtrip() {
        for topic in &[
            Topic::SystemStatus,
            Topic::ServicesMetrics,
            Topic::PositionsUpdate,
            Topic::EventsNew,
        ] {
            let code = topic.code();
            let parsed = Topic::from_code(code);
            assert_eq!(parsed, Some(*topic));
        }
    }

    #[test]
    fn test_envelope_creation() {
        let data = serde_json::json!({"test": "value"});
        let envelope = WsEnvelope::new(Topic::EventsNew, 42, data.clone())
            .with_key("test_mint")
            .as_snapshot();

        assert_eq!(envelope.v, PROTOCOL_VERSION);
        assert_eq!(envelope.t, "events.new");
        assert_eq!(envelope.seq, 42);
        assert_eq!(envelope.key, Some("test_mint".to_string()));
        assert_eq!(envelope.data, data);
        assert_eq!(envelope.meta.as_ref().and_then(|m| m.snapshot), Some(true));
    }

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::Ack {
            message: "Filters updated".to_string(),
            context: Some(serde_json::json!({"topics": ["events", "positions"]})),
        };

        let json = msg.to_json().unwrap();
        assert!(json.contains("\"type\":\"ack\""));
        assert!(json.contains("Filters updated"));
    }
}
