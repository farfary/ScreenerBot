use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    arguments::is_debug_webserver_enabled,
    events,
    logger::{log, LogTag},
    webserver::ws::{hub::WsHub, topics},
};

pub fn start(hub: Arc<WsHub>) {
    if let Some(rx) = events::subscribe() {
        tokio::spawn(run(hub, rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "INFO", "ws.sources.events started");
        }
    } else if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "WARN",
            "ws.sources.events: subscribe() returned None",
        );
    }
}

async fn run(hub: Arc<WsHub>, mut rx: broadcast::Receiver<events::Event>) {
    loop {
        match rx.recv().await {
            Ok(event) => {
                let envelope =
                    topics::events::event_to_envelope(&event, hub.next_seq("events.new"));
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("ws.sources.events lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "ws.sources.events closed, exiting",
                );
                break;
            }
        }
    }
}
