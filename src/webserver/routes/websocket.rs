/// WebSocket route - Upgrade handler for centralized WebSocket hub
///
/// Single endpoint `/ws` that handles all real-time data streaming.
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::state::AppState,
};

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(websocket_handler))
}

/// WebSocket upgrade handler
async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    // Get hub from state (ws_hub() returns &Arc<WsHub>, not Option)
    let hub = state.ws_hub().clone();

    // Check connection limit
    let current = hub.active_connections().await;
    let max_allowed = state.config.websocket.max_connections;

    if current >= max_allowed {
        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "DEBUG",
                &format!(
                    "WebSocket connection rejected: limit reached (current={}, max={})",
                    current, max_allowed
                ),
            );
        }
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "WebSocket connection limit reached",
        )
            .into_response();
    }

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!("WebSocket upgrade accepted (active={})", current),
        );
    }

    // Upgrade and handle connection
    ws.on_upgrade(move |socket| crate::webserver::ws::connection::handle_connection(socket, hub))
}
