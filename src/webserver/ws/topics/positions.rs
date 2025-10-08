/// Positions topic messages

use crate::positions::PositionUpdate;
use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert position update to envelope
pub fn position_to_envelope(update: &PositionUpdate, seq: u64) -> WsEnvelope {
    let data = serde_json::to_value(update).unwrap_or_default();
    let key = match update {
        PositionUpdate::Opened { position, .. } 
        | PositionUpdate::Updated { position, .. } 
        | PositionUpdate::Closed { position, .. } => {
            position.mint.clone()
        }
        PositionUpdate::BalanceChanged { .. } => String::new(),
    };
    WsEnvelope::new(Topic::PositionsUpdate, seq, data).with_key(key)
}
