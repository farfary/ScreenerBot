use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    positions,
    webserver::ws::{hub::WsHub, topics},
};

pub fn start(hub: Arc<WsHub>) {
    if let Some(rx) = positions::subscribe_positions() {
        tokio::spawn(run(hub, rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "INFO", "ws.sources.positions started");
        }
    } else if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "WARN",
            "ws.sources.positions: subscribe_positions() returned None",
        );
    }
}

async fn run(hub: Arc<WsHub>, mut rx: broadcast::Receiver<positions::PositionUpdate>) {
    loop {
        match rx.recv().await {
            Ok(update) => {
                let seq = hub.next_seq("positions.update").await;
                let envelope = topics::positions::position_to_envelope(&update, seq);
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("ws.sources.positions lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "ws.sources.positions closed, exiting",
                );
                break;
            }
        }
    }
}
