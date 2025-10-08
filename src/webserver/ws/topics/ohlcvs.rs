/// OHLCV topic messages - Placeholder
use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert OHLCV update to envelope (stub)
pub fn ohlcv_to_envelope(_data: &serde_json::Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::OhlcvsUpdate, seq, serde_json::json!({}))
}
