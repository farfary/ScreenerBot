use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::ws::hub::WsHub,
};

pub fn start(_hub: Arc<WsHub>) {
    // Placeholder: trader state stream not implemented yet
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            "ws.sources.trader skipped (not implemented)",
        );
    }
}
