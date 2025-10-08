use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::ws::hub::WsHub,
};

pub fn start(_hub: Arc<WsHub>) {
    // Placeholder: transactions activity stream not implemented yet
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            "ws.sources.transactions skipped (not implemented)",
        );
    }
}
