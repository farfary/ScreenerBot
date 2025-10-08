/// Tokens topic message helpers
use crate::webserver::ws::message::{Topic, WsEnvelope};
use serde_json::Value;

/// Convert a token payload to a websocket envelope with mint routing key
pub fn token_to_envelope(mint: &str, data: Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::TokensUpdate, seq, data).with_key(mint)
}
