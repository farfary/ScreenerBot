/// Services topic messages

use crate::webserver::routes::services::ServicesOverviewResponse;
use crate::webserver::ws::message::{Topic, WsEnvelope};

/// Convert services snapshot to envelope
pub fn services_to_envelope(snapshot: &ServicesOverviewResponse, seq: u64) -> WsEnvelope {
    let data = serde_json::to_value(snapshot).unwrap_or_default();
    WsEnvelope::new(Topic::ServicesMetrics, seq, data).as_snapshot()
}
