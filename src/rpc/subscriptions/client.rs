//! The single shared WebSocket connection and its reconnect loop.

use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{mpsc, watch, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::logger::{self, LogTag};

use super::payloads;
use super::registry::Registry;
use super::types::{ConnectionState, LogsSubscription, SubscriptionEvent, SubscriptionMetrics};

/// A command sent into the running actor from a subscriber handle.
pub(super) enum Command {
    Subscribe {
        address: String,
        handle_id: u64,
        tx: mpsc::UnboundedSender<SubscriptionEvent>,
    },
    Unsubscribe {
        address: String,
        handle_id: u64,
    },
}

/// Parsed form of one inbound WebSocket text frame.
#[derive(Debug, PartialEq)]
pub(super) enum ParsedMessage {
    Notification {
        subscription_id: u64,
        signature: String,
        slot: Option<u64>,
        failed: bool,
        logs: Vec<String>,
    },
    Confirmation {
        request_id: u64,
        subscription_id: u64,
    },
    RpcError {
        request_id: Option<u64>,
        message: String,
    },
    Ignored,
}

/// Parse one inbound WebSocket text frame. Pure, no I/O.
pub(super) fn parse_message(raw: &str) -> ParsedMessage {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return ParsedMessage::Ignored;
    };

    if v["method"] == "logsNotification" {
        let Some(subscription_id) = v["params"]["subscription"].as_u64() else {
            return ParsedMessage::Ignored;
        };
        let value = &v["params"]["result"]["value"];
        let Some(signature) = value["signature"].as_str() else {
            return ParsedMessage::Ignored;
        };
        let slot = v["params"]["result"]["context"]["slot"].as_u64();
        let failed = !value["err"].is_null();
        let logs = value["logs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|entry| entry.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        return ParsedMessage::Notification {
            subscription_id,
            signature: signature.to_owned(),
            slot,
            failed,
            logs,
        };
    }

    if v["error"].is_object() {
        return ParsedMessage::RpcError {
            request_id: v["id"].as_u64(),
            message: v["error"]["message"]
                .as_str()
                .unwrap_or("unknown")
                .to_owned(),
        };
    }

    if let (Some(request_id), Some(subscription_id)) = (v["id"].as_u64(), v["result"].as_u64()) {
        return ParsedMessage::Confirmation {
            request_id,
            subscription_id,
        };
    }

    ParsedMessage::Ignored
}

/// Reconnect backoff in seconds: 1, 2, 4, 8, 16, 32, then capped at 60.
pub(super) fn reconnect_delay_secs(attempt: u32) -> u64 {
    2u64.saturating_pow(attempt.min(6)).min(60).max(1)
}

/// The command channel into the running actor. `None` when no actor is running.
static COMMANDS: LazyLock<Mutex<Option<mpsc::UnboundedSender<Command>>>> =
    LazyLock::new(|| Mutex::new(None));

/// Broadcast of the transport's connection state, paired with a receiver template.
static STATE: LazyLock<(
    watch::Sender<ConnectionState>,
    watch::Receiver<ConnectionState>,
)> = LazyLock::new(|| watch::channel(ConnectionState::Idle));

static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);
static NOTIFICATIONS: AtomicU64 = AtomicU64::new(0);
static RECONNECTS: AtomicU64 = AtomicU64::new(0);
/// Unix seconds of the last notification; 0 means never.
static LAST_MESSAGE_UNIX: AtomicI64 = AtomicI64::new(0);
static ACTIVE_ADDRESSES: AtomicU64 = AtomicU64::new(0);

fn set_state(state: ConnectionState) {
    let _ = STATE.0.send(state);
}

/// Allocate the next subscriber handle id.
pub(super) fn next_handle_id() -> u64 {
    NEXT_HANDLE_ID.fetch_add(1, Relaxed)
}

/// A receiver for the transport's connection state.
pub(super) fn state_receiver() -> watch::Receiver<ConnectionState> {
    STATE.1.clone()
}

/// A snapshot of the transport's counters.
pub(super) fn metrics_snapshot() -> SubscriptionMetrics {
    let last_message_unix = match LAST_MESSAGE_UNIX.load(Relaxed) {
        0 => None,
        secs => Some(secs),
    };
    SubscriptionMetrics {
        state: *STATE.1.borrow(),
        active_addresses: ACTIVE_ADDRESSES.load(Relaxed),
        notifications_received: NOTIFICATIONS.load(Relaxed),
        reconnects: RECONNECTS.load(Relaxed),
        last_message_unix,
    }
}

/// Ensure the shared actor is running and return a sender for its command channel.
/// Spawns the actor on first use, or when the previous actor has exited.
pub(super) async fn ensure_actor() -> crate::Result<mpsc::UnboundedSender<Command>> {
    let mut guard = COMMANDS.lock().await;
    if let Some(sender) = guard.as_ref() {
        if !sender.is_closed() {
            return Ok(sender.clone());
        }
    }
    let (tx, rx) = mpsc::unbounded_channel::<Command>();
    tokio::spawn(run_actor(rx));
    *guard = Some(tx.clone());
    Ok(tx)
}

/// Apply any commands that arrived while the actor was disconnected, so the
/// registry stays current even though there is no socket to (un)subscribe on.
fn drain_pending_commands(
    commands: &mut mpsc::UnboundedReceiver<Command>,
    registry: &mut Registry,
) {
    while let Ok(cmd) = commands.try_recv() {
        match cmd {
            Command::Subscribe {
                address,
                handle_id,
                tx,
            } => {
                registry.add_subscriber(&address, handle_id, tx);
            }
            Command::Unsubscribe { address, handle_id } => {
                registry.remove_subscriber(&address, handle_id);
            }
        }
        ACTIVE_ADDRESSES.store(registry.addresses().len() as u64, Relaxed);
    }
}

/// Outer reconnect loop: resolve the URL, connect, resubscribe everything still
/// registered, then service commands and frames until the connection drops.
async fn run_actor(mut commands: mpsc::UnboundedReceiver<Command>) {
    let mut registry = Registry::new();
    let mut attempt: u32 = 0;
    let mut next_request_id: u64 = 1;

    'outer: loop {
        let url = match payloads::get_websocket_url() {
            Ok(u) => u,
            Err(e) => {
                logger::warning(
                    LogTag::Websocket,
                    &format!("Cannot resolve WebSocket URL: {e}"),
                );
                set_state(ConnectionState::Down);
                drain_pending_commands(&mut commands, &mut registry);
                tokio::time::sleep(Duration::from_secs(reconnect_delay_secs(attempt))).await;
                attempt += 1;
                continue 'outer;
            }
        };

        let stream = match crate::net::connect_ws(&url).await {
            Ok(s) => s,
            Err(e) => {
                let delay = reconnect_delay_secs(attempt);
                logger::info(
                    LogTag::Websocket,
                    &format!(
                        "WebSocket connect failed: {e} (retry in {delay}s, attempt {})",
                        attempt + 1
                    ),
                );
                set_state(if attempt >= 4 {
                    ConnectionState::Down
                } else {
                    ConnectionState::Reconnecting
                });
                drain_pending_commands(&mut commands, &mut registry);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                attempt += 1;
                continue 'outer;
            }
        };
        logger::info(
            LogTag::Websocket,
            &format!("WebSocket connected ({})", crate::rpc::mask_url(&url)),
        );
        attempt = 0;
        set_state(ConnectionState::Connected);
        let (mut ws_tx, mut ws_rx) = stream.split();

        // Server subscription ids never survive a reconnect; resubscribe everything.
        registry.clear_server_state();
        let mut connection_lost = false;
        for address in registry.addresses() {
            let id = next_request_id;
            next_request_id += 1;
            if ws_tx
                .send(Message::Text(payloads::logs_subscribe_payload(
                    &address, id,
                )))
                .await
                .is_err()
            {
                // Losing the socket while resubscribing is a disconnect like any
                // other. Fall through to the backoff below instead of jumping
                // straight back to connect, which would spin hot against an
                // endpoint that accepts the connection and then drops it.
                connection_lost = true;
                break;
            }
            registry.note_pending(id, &address);
        }

        if !connection_lost {
            let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
            heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    maybe_cmd = commands.recv() => {
                        match maybe_cmd {
                            None => {
                                let _ = ws_tx.send(Message::Close(None)).await;
                                set_state(ConnectionState::Idle);
                                ACTIVE_ADDRESSES.store(0, Relaxed);
                                *COMMANDS.lock().await = None;
                                return;
                            }
                            Some(Command::Subscribe { address, handle_id, tx }) => {
                                let is_new = registry.add_subscriber(&address, handle_id, tx);
                                ACTIVE_ADDRESSES.store(registry.addresses().len() as u64, Relaxed);
                                if is_new {
                                    let id = next_request_id;
                                    next_request_id += 1;
                                    if ws_tx
                                        .send(Message::Text(payloads::logs_subscribe_payload(&address, id)))
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                    registry.note_pending(id, &address);
                                }
                            }
                            Some(Command::Unsubscribe { address, handle_id }) => {
                                if let Some(sub_id) = registry.remove_subscriber(&address, handle_id) {
                                    let id = next_request_id;
                                    next_request_id += 1;
                                    if ws_tx
                                        .send(Message::Text(payloads::logs_unsubscribe_payload(sub_id, id)))
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                                ACTIVE_ADDRESSES.store(registry.addresses().len() as u64, Relaxed);
                                if registry.is_empty() {
                                    logger::info(LogTag::Websocket, "No subscriptions remain - closing WebSocket");
                                    let _ = ws_tx.send(Message::Close(None)).await;
                                    set_state(ConnectionState::Idle);
                                    ACTIVE_ADDRESSES.store(0, Relaxed);
                                    *COMMANDS.lock().await = None;
                                    return;
                                }
                            }
                        }
                    }
                    _ = heartbeat.tick() => {
                        if ws_tx.send(Message::Ping(Vec::new())).await.is_err() {
                            break;
                        }
                    }
                    frame = ws_rx.next() => {
                        match frame {
                            Some(Ok(Message::Text(text))) => handle_text(&text, &mut registry),
                            Some(Ok(Message::Ping(p))) => {
                                if ws_tx.send(Message::Pong(p)).await.is_err() {
                                    break;
                                }
                            }
                            Some(Ok(Message::Pong(_))) => {}
                            Some(Ok(Message::Binary(_))) | Some(Ok(Message::Frame(_))) => {}
                            // Every remaining case is a disconnect: a server close frame, a
                            // transport error and an ended stream are never a successful exit.
                            Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                        }
                    }
                }
            }
        }

        RECONNECTS.fetch_add(1, Relaxed);
        set_state(ConnectionState::Reconnecting);
        let delay = reconnect_delay_secs(attempt);
        logger::warning(
            LogTag::Websocket,
            &format!("WebSocket disconnected - reconnecting in {delay}s"),
        );
        tokio::time::sleep(Duration::from_secs(delay)).await;
        attempt += 1;
    }
}

/// Handle one parsed inbound frame against the registry.
fn handle_text(raw: &str, registry: &mut Registry) {
    match parse_message(raw) {
        ParsedMessage::Notification {
            subscription_id,
            signature,
            slot,
            failed,
            logs,
        } => {
            let Some(address) = registry.address_for_subscription(subscription_id) else {
                logger::debug(
                    LogTag::Websocket,
                    &format!("Notification for unknown subscription id {subscription_id}"),
                );
                return;
            };
            let address = address.to_owned();
            NOTIFICATIONS.fetch_add(1, Relaxed);
            LAST_MESSAGE_UNIX.store(chrono::Utc::now().timestamp(), Relaxed);
            // Truncate by characters, never by bytes: this frame came off the wire
            // and a byte slice through a multi-byte character would panic.
            let sig8: String = signature.chars().take(8).collect();
            logger::debug(
                LogTag::Websocket,
                &format!("logs notification {sig8} for {address}"),
            );
            let event = SubscriptionEvent {
                address: address.clone(),
                signature,
                slot,
                failed,
                logs,
            };
            registry.fanout(&address, &event);
        }
        ParsedMessage::Confirmation {
            request_id,
            subscription_id,
        } => {
            let address = registry.confirm(request_id, subscription_id);
            logger::debug(
                LogTag::Websocket,
                &format!(
                    "Subscription confirmed: request {request_id} -> id {subscription_id} ({address:?})"
                ),
            );
        }
        ParsedMessage::RpcError { message, .. } => {
            logger::warning(
                LogTag::Websocket,
                &format!("Subscription RPC error: {message}"),
            );
        }
        ParsedMessage::Ignored => {}
    }
}

impl Drop for LogsSubscription {
    fn drop(&mut self) {
        // Send on the channel the handle carries rather than looking the actor up
        // through COMMANDS: a lock we failed to take here would silently skip the
        // unsubscribe and leave the address subscribed on the server forever.
        let _ = self.commands.send(Command::Unsubscribe {
            address: self.address.clone(),
            handle_id: self.id,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_delay_matches_schedule() {
        assert_eq!(reconnect_delay_secs(0), 1);
        assert_eq!(reconnect_delay_secs(1), 2);
        assert_eq!(reconnect_delay_secs(2), 4);
        assert_eq!(reconnect_delay_secs(3), 8);
        assert_eq!(reconnect_delay_secs(4), 16);
        assert_eq!(reconnect_delay_secs(5), 32);
        assert_eq!(reconnect_delay_secs(6), 60);
        assert_eq!(reconnect_delay_secs(50), 60);
    }

    #[test]
    fn parse_message_notification_success() {
        let raw = r#"{
            "jsonrpc": "2.0",
            "method": "logsNotification",
            "params": {
                "result": {
                    "context": { "slot": 5208469 },
                    "value": {
                        "signature": "5h6xBEauJ3PK6SWCZ1PGjBvj8vDdWG3KpwATGy1ARAXFSDwt8GFXM7W5Ncn16wmqokgpiKRLuS83KUxyZyv2sUYv",
                        "err": null,
                        "logs": [
                            "Program 11111111111111111111111111111111 invoke [1]",
                            "Program 11111111111111111111111111111111 success"
                        ]
                    }
                },
                "subscription": 24040
            }
        }"#;

        match parse_message(raw) {
            ParsedMessage::Notification {
                subscription_id,
                signature,
                slot,
                failed,
                logs,
            } => {
                assert_eq!(subscription_id, 24040);
                assert_eq!(
                    signature,
                    "5h6xBEauJ3PK6SWCZ1PGjBvj8vDdWG3KpwATGy1ARAXFSDwt8GFXM7W5Ncn16wmqokgpiKRLuS83KUxyZyv2sUYv"
                );
                assert_eq!(slot, Some(5208469));
                assert!(!failed);
                assert_eq!(logs.len(), 2);
            }
            other => panic!("expected Notification, got {other:?}"),
        }
    }

    #[test]
    fn parse_message_notification_failed_transaction() {
        let raw = r#"{
            "jsonrpc": "2.0",
            "method": "logsNotification",
            "params": {
                "result": {
                    "context": { "slot": 1 },
                    "value": {
                        "signature": "sig",
                        "err": { "InstructionError": [0, { "Custom": 1 }] },
                        "logs": []
                    }
                },
                "subscription": 1
            }
        }"#;

        match parse_message(raw) {
            ParsedMessage::Notification { failed, .. } => assert!(failed),
            other => panic!("expected Notification, got {other:?}"),
        }
    }

    #[test]
    fn parse_message_confirmation() {
        let raw = r#"{"jsonrpc":"2.0","result":24040,"id":1}"#;
        assert_eq!(
            parse_message(raw),
            ParsedMessage::Confirmation {
                request_id: 1,
                subscription_id: 24040,
            }
        );
    }

    #[test]
    fn parse_message_rpc_error() {
        let raw = r#"{"jsonrpc":"2.0","error":{"code":-32602,"message":"Invalid params"},"id":1}"#;
        match parse_message(raw) {
            ParsedMessage::RpcError {
                request_id,
                message,
            } => {
                assert_eq!(request_id, Some(1));
                assert_eq!(message, "Invalid params");
            }
            other => panic!("expected RpcError, got {other:?}"),
        }
    }

    #[test]
    fn parse_message_ignored_cases() {
        assert_eq!(parse_message("not json"), ParsedMessage::Ignored);
        assert_eq!(parse_message("{}"), ParsedMessage::Ignored);
    }
}
