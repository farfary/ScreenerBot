/// WebSocket producers - unified startup
///
/// Centralized "sources" modules own all logic for consuming internal
/// broadcasts and periodic snapshot gathering. This file only starts them.

use std::sync::Arc;
use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::ws::{hub::WsHub, sources},
};

/// Start all producers (spawn background tasks)
pub fn start_producers(hub: Arc<WsHub>) {
    sources::start_all(hub);
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "✅ ws.sources started (events, positions, prices, status, services)");
    }
}
// All task implementations live under ws::sources::*
