// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The single shared WebSocket connection and its reconnect loop.

use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering::Relaxed};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::{mpsc, watch, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::logger::{self, LogTag};

use super::frames::{parse_message, ParsedMessage, METHOD_NOT_FOUND};
use super::payloads;
use super::registry::Registry;
use super::types::{ConnectionState, Subscription, SubscriptionMetrics, Topic, TopicEvent};

/// A command sent into the running actor from a subscriber handle.
pub(super) enum Command {
    Subscribe {
        topic: Topic,
        handle_id: u64,
        tx: mpsc::UnboundedSender<TopicEvent>,
    },
    Unsubscribe {
        topic: Topic,
        handle_id: u64,
    },
}

/// How long the transport stays down once no configured endpoint serves
/// `logsSubscribe`, before it asks again. Watched wallets are polled meanwhile.
const UNSUPPORTED_RECHECK: Duration = Duration::from_secs(300);

/// How long a connection must hold before its close counts as routine: providers
/// close long-lived sockets every so often, and the reconnect restores every topic.
const STABLE_CONNECTION: Duration = Duration::from_secs(60);

/// The backoff attempt after a connection that held for `lived` closed. A routine
/// close starts the backoff over; an endpoint that keeps dropping new connections
/// keeps escalating instead of being retried every second.
fn attempt_after_close(attempt: u32, lived: Duration) -> u32 {
    if lived >= STABLE_CONNECTION {
        0
    } else {
        attempt
    }
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
/// Counts successful connects. A subscriber that saw `Confirmed` on one generation
/// knows a later generation may have missed notifications in between.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// True while every configured endpoint has refused the subscription methods.
static UNSUPPORTED: AtomicBool = AtomicBool::new(false);
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

/// The number of the current connection; it grows by one on every connect.
pub(super) fn subscriptions_unsupported() -> bool {
    UNSUPPORTED.load(Relaxed)
}

pub(super) fn connection_generation() -> u64 {
    GENERATION.load(Relaxed)
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
                topic,
                handle_id,
                tx,
            } => {
                registry.add_subscriber(&topic, handle_id, tx);
            }
            Command::Unsubscribe { topic, handle_id } => {
                registry.remove_subscriber(&topic, handle_id);
            }
        }
        ACTIVE_ADDRESSES.store(registry.topics().len() as u64, Relaxed);
    }
}

/// Outer reconnect loop: resolve the URL, connect, resubscribe everything still
/// registered, then service commands and frames until the connection drops.
async fn run_actor(mut commands: mpsc::UnboundedReceiver<Command>) {
    let mut registry = Registry::new();
    let mut attempt: u32 = 0;
    let mut provider_index: usize = 0;
    let mut next_request_id: u64 = 1;
    // Endpoints that answered `logsSubscribe` with method-not-found.
    let mut unsupported: std::collections::HashSet<String> = std::collections::HashSet::new();

    'outer: loop {
        let urls = match payloads::get_websocket_urls() {
            Ok(urls) => urls,
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

        let usable: Vec<&String> = urls
            .iter()
            .filter(|url| !unsupported.contains(*url))
            .collect();
        if usable.is_empty() {
            logger::info(
                LogTag::Websocket,
                &format!(
                    "No configured RPC endpoint serves the subscriptions the app uses; watched wallets are polled, next check in {}s",
                    UNSUPPORTED_RECHECK.as_secs()
                ),
            );
            UNSUPPORTED.store(true, Relaxed);
            set_state(ConnectionState::Down);
            drain_pending_commands(&mut commands, &mut registry);
            tokio::time::sleep(UNSUPPORTED_RECHECK).await;
            drain_pending_commands(&mut commands, &mut registry);
            unsupported.clear();
            continue 'outer;
        }
        provider_index %= usable.len();
        let url = usable[provider_index].as_str().to_owned();
        let usable_count = usable.len();
        let stream = match crate::net::connect_ws(&url).await {
            Ok(s) => s,
            Err(e) => {
                provider_index = (provider_index + 1) % usable_count;
                let completed_cycle = provider_index == 0;
                let delay = if completed_cycle {
                    reconnect_delay_secs(attempt)
                } else {
                    0
                };
                logger::info(
                    LogTag::Websocket,
                    &format!(
                        "WebSocket provider {} failed: {e} (next provider in {delay}s)",
                        crate::logger::redact_url(&url)
                    ),
                );
                set_state(if attempt >= 4 {
                    ConnectionState::Down
                } else {
                    ConnectionState::Reconnecting
                });
                drain_pending_commands(&mut commands, &mut registry);
                if delay > 0 {
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
                if completed_cycle {
                    attempt += 1;
                }
                continue 'outer;
            }
        };
        logger::info(
            LogTag::Websocket,
            &format!("WebSocket connected ({})", crate::logger::redact_url(&url)),
        );
        let connected_at = std::time::Instant::now();
        UNSUPPORTED.store(false, Relaxed);
        GENERATION.fetch_add(1, Relaxed);
        set_state(ConnectionState::Connected);
        let (mut ws_tx, mut ws_rx) = stream.split();

        // Server subscription ids never survive a reconnect; resubscribe everything.
        registry.clear_server_state();
        let mut connection_lost = false;
        let mut endpoint_unsupported = false;
        let mut refusals_logged = std::collections::HashSet::new();
        for topic in registry.topics() {
            let id = next_request_id;
            next_request_id += 1;
            if ws_tx
                .send(Message::Text(
                    payloads::subscribe_payload(&topic, id).into(),
                ))
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
            registry.note_pending(id, &topic);
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
                            Some(Command::Subscribe { topic, handle_id, tx }) => {
                                let is_new = registry.add_subscriber(&topic, handle_id, tx);
                                ACTIVE_ADDRESSES.store(registry.topics().len() as u64, Relaxed);
                                if is_new {
                                    let id = next_request_id;
                                    next_request_id += 1;
                                    if ws_tx
                                        .send(Message::Text(payloads::subscribe_payload(&topic, id).into()))
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                    registry.note_pending(id, &topic);
                                }
                            }
                            Some(Command::Unsubscribe { topic, handle_id }) => {
                                registry.remove_subscriber(&topic, handle_id);
                                ACTIVE_ADDRESSES.store(registry.topics().len() as u64, Relaxed);
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
                        if ws_tx.send(Message::Ping(Vec::new().into())).await.is_err() {
                            break;
                        }
                    }
                    frame = ws_rx.next() => {
                        match frame {
                            Some(Ok(Message::Text(text))) => {
                                if handle_text(&text, &mut registry, &mut refusals_logged) {
                                    endpoint_unsupported = true;
                                    break;
                                }
                            }
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
                let mut released = true;
                for (topic, sub_id) in registry.take_releases() {
                    let id = next_request_id;
                    next_request_id += 1;
                    if ws_tx
                        .send(Message::Text(
                            payloads::unsubscribe_payload(&topic, sub_id, id).into(),
                        ))
                        .await
                        .is_err()
                    {
                        released = false;
                        break;
                    }
                }
                if !released {
                    break;
                }
            }
        }

        if endpoint_unsupported {
            // Not a disconnect: this endpoint has no log subscriptions. Try the next one
            // at once; the loop above stops when none is left.
            let _ = ws_tx.send(Message::Close(None)).await;
            logger::info(
                LogTag::Websocket,
                &format!(
                    "WebSocket provider {} does not serve a subscription method the app uses; trying the next endpoint",
                    crate::logger::redact_url(&url)
                ),
            );
            unsupported.insert(url);
            set_state(ConnectionState::Reconnecting);
            provider_index = 0;
            continue 'outer;
        }
        let lived = connected_at.elapsed();
        attempt = attempt_after_close(attempt, lived);
        RECONNECTS.fetch_add(1, Relaxed);
        set_state(ConnectionState::Reconnecting);
        provider_index = (provider_index + 1) % usable_count;
        let completed_cycle = provider_index == 0;
        let delay = if completed_cycle {
            reconnect_delay_secs(attempt)
        } else {
            0
        };
        if lived >= STABLE_CONNECTION {
            logger::info(
                LogTag::Websocket,
                &format!(
                    "WebSocket closed after {}s - reconnecting in {delay}s",
                    lived.as_secs()
                ),
            );
        } else {
            logger::warning(
                LogTag::Websocket,
                &format!(
                    "WebSocket dropped {}s after connecting - failing over in {delay}s",
                    lived.as_secs()
                ),
            );
        }
        if delay > 0 {
            tokio::time::sleep(Duration::from_secs(delay)).await;
        }
        if completed_cycle {
            attempt += 1;
        }
    }
}

/// Handle one parsed inbound frame against the registry. Returns `true` when the
/// endpoint does not serve `logsSubscribe` at all.
fn handle_text(
    raw: &str,
    registry: &mut Registry,
    refusals_logged: &mut std::collections::HashSet<String>,
) -> bool {
    match parse_message(raw) {
        ParsedMessage::Mention {
            subscription_id,
            mention,
        } => deliver(registry, subscription_id, TopicEvent::Mention(mention)),
        ParsedMessage::AccountChange {
            subscription_id,
            slot,
        } => deliver(
            registry,
            subscription_id,
            TopicEvent::AccountChanged { slot },
        ),
        ParsedMessage::Confirmation {
            request_id,
            subscription_id,
        } => {
            let topic = registry.confirm(request_id, subscription_id);
            logger::debug(
                LogTag::Websocket,
                &format!(
                    "Subscription confirmed: request {request_id} -> id {subscription_id} ({topic:?})"
                ),
            );
        }
        ParsedMessage::RpcError {
            request_id,
            code,
            message,
        } => {
            // An endpoint that lacks a subscription method is skipped as a whole.
            if code == Some(METHOD_NOT_FOUND)
                && request_id.is_some_and(|id| registry.pending_topic(id).is_some())
            {
                return true;
            }
            // A refused subscribe ends its subscribers' streams, so each watcher knows
            // it is not subscribed and polls instead of waiting on silence.
            let refused = request_id.and_then(|id| registry.refuse(id));
            if refusals_logged.insert(message.clone()) {
                logger::info(
                    LogTag::Websocket,
                    &format!(
                        "Subscription refused{}: {message}",
                        refused
                            .map(|topic| format!(" ({} {})", topic.method(), topic.address()))
                            .unwrap_or_default()
                    ),
                );
            }
        }
        ParsedMessage::Ignored => {}
    }
    false
}

/// Hand one notification to the subscribers of the topic its subscription id names.
fn deliver(registry: &mut Registry, subscription_id: u64, event: TopicEvent) {
    let Some(topic) = registry.topic_for_subscription(subscription_id).cloned() else {
        logger::debug(
            LogTag::Websocket,
            &format!("Notification for unknown subscription id {subscription_id}"),
        );
        return;
    };
    NOTIFICATIONS.fetch_add(1, Relaxed);
    LAST_MESSAGE_UNIX.store(chrono::Utc::now().timestamp(), Relaxed);
    registry.fanout(&topic, &event);
}

impl Drop for Subscription {
    fn drop(&mut self) {
        // Send on the channel the handle carries rather than looking the actor up
        // through COMMANDS: a lock we failed to take here would silently skip the
        // unsubscribe and leave the address subscribed on the server forever.
        let _ = self.commands.send(Command::Unsubscribe {
            topic: self.topic.clone(),
            handle_id: self.id,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_routine_close_starts_the_backoff_over_and_a_quick_drop_escalates_it() {
        assert_eq!(attempt_after_close(3, STABLE_CONNECTION), 0);
        assert_eq!(attempt_after_close(3, Duration::from_secs(5)), 3);
        assert_eq!(
            reconnect_delay_secs(attempt_after_close(3, Duration::from_secs(5))),
            8,
            "a dropping endpoint is not retried every second"
        );
    }

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
}
