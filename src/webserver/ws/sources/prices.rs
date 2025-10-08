use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    pools,
    webserver::ws::{hub::WsHub, topics},
};

pub fn start(hub: Arc<WsHub>) {
    if let Some(rx) = pools::subscribe_prices() {
        tokio::spawn(run(hub, rx));
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "INFO", "ws.sources.prices started");
        }
    } else if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "WARN",
            "ws.sources.prices: subscribe_prices() returned None",
        );
    }
}

async fn run(hub: Arc<WsHub>, mut rx: broadcast::Receiver<pools::PriceUpdate>) {
    loop {
        match rx.recv().await {
            Ok(update) => {
                let seq = hub.next_seq("prices.update").await;
                let envelope = topics::prices::price_to_envelope(&update, seq);
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("ws.sources.prices lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "ws.sources.prices closed, exiting",
                );
                break;
            }
        }
    }
}
