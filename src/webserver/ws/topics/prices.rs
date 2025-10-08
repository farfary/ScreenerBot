/// Prices topic messages
use crate::pools::PriceUpdate;
use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert price update to envelope
pub fn price_to_envelope(update: &PriceUpdate, seq: u64) -> WsEnvelope {
    let data = serde_json::to_value(update).unwrap_or_default();
    WsEnvelope::new(Topic::PricesUpdate, seq, data).with_key(&update.mint)
}
