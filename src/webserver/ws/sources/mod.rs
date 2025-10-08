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
/// - prices: consumes pools::subscribe_prices()
/// - status: periodic snapshot gather from ws::snapshots
/// - services: periodic overview gather from routes::services helper
///
/// Missing topics (stubs may be added later): tokens, ohlcvs, trader, wallet,
/// transactions, security.
pub mod events;
pub mod ohlcvs;
pub mod positions;
pub mod prices;
pub mod security;
pub mod services;
pub mod status;
pub mod tokens;
pub mod trader;
pub mod transactions;
pub mod wallet;

use std::sync::Arc;

use crate::webserver::ws::hub::WsHub;

/// Start all sources
pub fn start_all(hub: Arc<WsHub>) {
    // Start in deterministic order for logs
    events::start(hub.clone());
    positions::start(hub.clone());
    prices::start(hub.clone());
    status::start(hub.clone());
    services::start(hub);
    // Additional sources (currently stubs) for uniform structure:
    // tokens::start(hub.clone());
    // trader::start(hub.clone());
    // wallet::start(hub.clone());
    // ohlcvs::start(hub.clone());
    // transactions::start(hub.clone());
    // security::start(hub);
}
