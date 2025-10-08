/// Tokens topic messages - Placeholder

use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert token update to envelope (stub)
pub fn token_to_envelope(_data: &serde_json::Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::TokensUpdate, seq, serde_json::json!({}))
}
