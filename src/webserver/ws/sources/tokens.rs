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
            log(LogTag::Webserver, "INFO", "ws.sources.tokens started");
        }
    } else if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "WARN",
            "ws.sources.tokens: subscribe_prices() returned None",
        );
    }
}

async fn run(hub: Arc<WsHub>, mut rx: broadcast::Receiver<pools::PriceUpdate>) {
    loop {
        match rx.recv().await {
            Ok(update) => {
                let seq = hub.next_seq("tokens.update").await;
                let mint = update.mint.clone();
                let data = serde_json::to_value(&update).unwrap_or_else(|err| {
                    log(
                        LogTag::Webserver,
                        "ERROR",
                        &format!(
                            "ws.sources.tokens failed to serialize price update for {}: {}",
                            mint, err
                        ),
                    );
                    serde_json::json!({ "mint": mint, "error": "serialize_failed" })
                });
                let envelope = topics::tokens::token_to_envelope(&update.mint, data, seq);
                hub.broadcast(envelope).await;
            }
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!("ws.sources.tokens lagged, skipped {} messages", skipped),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "ws.sources.tokens closed, exiting",
                );
                break;
            }
        }
    }
}
