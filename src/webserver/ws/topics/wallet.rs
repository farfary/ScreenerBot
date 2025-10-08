/// Wallet balances topic messages - Placeholder

use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert wallet balance to envelope (stub)
pub fn wallet_to_envelope(_data: &serde_json::Value, seq: u64) -> WsEnvelope {
    WsEnvelope::new(Topic::WalletBalances, seq, serde_json::json!({}))
}
