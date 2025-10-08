/// Transactions activity topic messages - Placeholder

use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert transaction to envelope (stub)
pub fn transaction_to_envelope(_data: &serde_json::Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::TransactionsActivity, seq, serde_json::json!({}))
}
