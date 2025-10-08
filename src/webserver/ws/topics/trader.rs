/// Trader state topic messages - Placeholder

use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert trader state to envelope (stub)
pub fn trader_to_envelope(_data: &serde_json::Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::TraderState, seq, serde_json::json!({}))
}
