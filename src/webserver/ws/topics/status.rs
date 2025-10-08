/// Status topic messages
use crate::webserver::ws::{
    message::{Topic, WsEnvelope},
    sources::status::StatusSnapshot,
};

/// Convert status snapshot to envelope
pub fn status_to_envelope(snapshot: &StatusSnapshot, seq: u64) -> WsEnvelope {
    let data = serde_json::to_value(snapshot).unwrap_or_default();
    WsEnvelope::new(Topic::SystemStatus, seq, data).as_snapshot()
}
