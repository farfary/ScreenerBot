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
                let seq = hub.next_seq("events.new").await;
                let envelope = topics::events::event_to_envelope(&event, seq);
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("ws.sources.events lagged, skipped {} messages", skipped),
                );
                // CRITICAL FIX: Add backpressure delay to prevent CPU spin when events come faster
                // than WebSocket can broadcast them. Without this delay, recv() returns Lagged
                // error immediately, creating a tight CPU-consuming loop.
                // 100ms gives the broadcast channel time to drain while keeping latency acceptable.
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
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
