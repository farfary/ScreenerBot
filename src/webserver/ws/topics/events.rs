/// Events topic messages
use crate::events::Event;
use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert event to envelope
pub fn event_to_envelope(event: &Event, seq: u64) -> WsEnvelope {
    let data = serde_json::to_value(event).unwrap_or_default();
    WsEnvelope::new(Topic::EventsNew, seq, data).with_key(event.mint.clone().unwrap_or_default())
}
