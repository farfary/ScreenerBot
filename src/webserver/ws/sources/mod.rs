/// Centralized real-time data sources for the WebSocket hub
///
/// Design goal: keep ALL websocket logic inside webserver::ws.
/// We only consume read-only APIs from other crates/modules (DB helpers,
/// existing broadcast receivers, service manager accessors). We do NOT
/// create new broadcasters outside this module.
///
/// Each source exposes a `start` function that spawns background tasks
/// to feed the WsHub with messages for a specific topic using a common
/// contract:
/// - Uniform logging (start/closed/lagged)
/// - Uniform backpressure handling (handled by WsHub)
/// - Uses ws::topics::* to serialize domain types → WsEnvelope
///
/// Sources implemented:
/// - events: consumes events::subscribe()
/// - positions: consumes positions::subscribe_positions()
/// - status: periodic snapshot gather from ws::snapshots
/// - services: periodic overview gather from routes::services helper
///
pub mod events;
pub mod ohlcvs;
pub mod positions;
pub mod security;
pub mod services;
pub mod status;
pub mod tokens;
pub mod trader;
pub mod transactions;
pub mod wallet;

use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::ws::hub::WsHub,
};

/// Start all WebSocket data sources (replaces old producers.rs)
pub fn start_all(hub: Arc<WsHub>) {
    // Start active sources in deterministic order
    events::start(hub.clone());
    positions::start(hub.clone());
    tokens::start(hub.clone());
    status::start(hub.clone());
    services::start(hub);

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            "✅ ws.sources started (events, positions, tokens, status, services)",
        );
    }
}
