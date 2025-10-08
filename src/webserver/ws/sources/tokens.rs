use std::sync::Arc;
use tokio::sync::broadcast;

use crate::{
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    pools,
    tokens::{self, TokenRealtimeEvent},
    webserver::ws::{hub::WsHub, message::MessageMetadata, topics},
};

pub fn start(hub: Arc<WsHub>) {
    let token_rx = tokens::subscribe_token_updates();
    tokio::spawn(run_token_updates(hub.clone(), token_rx));

    if let Some(rx) = pools::subscribe_prices() {
        tokio::spawn(run_price_updates(hub, rx));
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

async fn run_price_updates(hub: Arc<WsHub>, mut rx: broadcast::Receiver<pools::PriceUpdate>) {
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

async fn run_token_updates(hub: Arc<WsHub>, mut rx: broadcast::Receiver<TokenRealtimeEvent>) {
    loop {
        match rx.recv().await {
            Ok(event) => broadcast_token_event(&hub, event).await,
            Err(broadcast::error::RecvError::Lagged(skipped)) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    &format!(
                        "ws.sources.tokens summary lagged, skipped {} messages",
                        skipped
                    ),
                );
            }
            Err(broadcast::error::RecvError::Closed) => {
                log(
                    LogTag::Webserver,
                    "WARN",
                    "ws.sources.tokens summary channel closed, exiting",
                );
                break;
            }
        }
    }
}

async fn broadcast_token_event(hub: &Arc<WsHub>, event: TokenRealtimeEvent) {
    use TokenRealtimeEvent::*;

    match event {
        Summary(summary) => {
            let mint = summary.mint.clone();
            let seq = hub.next_seq("tokens.update").await;
            let data = serde_json::to_value(&summary).unwrap_or_else(|err| {
                log(
                    LogTag::Webserver,
                    "ERROR",
                    &format!(
                        "ws.sources.tokens failed to serialize summary update for {}: {}",
                        mint, err
                    ),
                );
                serde_json::json!({ "mint": mint.clone(), "error": "serialize_failed" })
            });

            let mut extra = serde_json::Map::new();
            extra.insert("update".to_string(), serde_json::json!("summary"));

            let envelope =
                topics::tokens::token_to_envelope(&mint, data, seq).with_meta(MessageMetadata {
                    snapshot: None,
                    dropped: None,
                    extra: Some(extra),
                });

            hub.broadcast(envelope).await;
        }
        Removed(mint) => {
            let seq = hub.next_seq("tokens.update").await;
            let data = serde_json::json!({ "mint": mint.clone(), "removed": true });

            let mut extra = serde_json::Map::new();
            extra.insert("update".to_string(), serde_json::json!("removed"));

            let envelope =
                topics::tokens::token_to_envelope(&mint, data, seq).with_meta(MessageMetadata {
                    snapshot: None,
                    dropped: None,
                    extra: Some(extra),
                });

            hub.broadcast(envelope).await;
        }
    }
}
