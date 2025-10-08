/// Security alerts topic messages - Placeholder

use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert security alert to envelope (stub)
pub fn security_to_envelope(_data: &serde_json::Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::SecurityAlerts, seq, serde_json::json!({}))
}
