use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::ws::hub::WsHub,
};

pub fn start(_hub: Arc<WsHub>) {
    // Placeholder: tokens updates are not wired yet
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            "ws.sources.tokens skipped (not implemented)",
        );
    }
}
