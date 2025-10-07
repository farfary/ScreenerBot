/// Centralized WebSocket Hub for real-time updates
///
/// This module provides a unified WebSocket endpoint that handles multiple channels:
/// - events: Real-time event stream
/// - positions: Position updates (opened/closed/updated)
/// - prices: Token price updates
/// - status: System status snapshots (always-on)
///
/// Clients subscribe to specific channels and receive only relevant updates.
use std::{ collections::HashSet, sync::Arc };

use axum::{
    extract::{ ws::{ Message, WebSocket, WebSocketUpgrade }, Query, State },
    response::Response,
    routing::get,
    Router,
};
use futures::{ SinkExt, StreamExt };
use serde::{ Deserialize, Serialize };

use crate::{
    arguments::is_debug_webserver_enabled,
    events::{ self, Event, EventCategory, Severity },
    logger::{ log, LogTag },
    pools,
    positions,
    webserver::{ routes::services::ServicesOverviewResponse, state::AppState, status_broadcast },
};

#[derive(Debug, Deserialize, Clone)]
pub struct WsEventsQuery {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
    pub last_id: Option<i64>,
}

/// Client message types
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Subscribe {
        channel: String,
        #[serde(default)]
        filters: serde_json::Value,
    },
    Unsubscribe {
        channel: String,
    },
    Ping,
}

/// Server message types
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Data {
        channel: String,
        data: serde_json::Value,
        timestamp: String,
    },
    Subscribed {
        channel: String,
        message: String,
    },
    Unsubscribed {
        channel: String,
        message: String,
    },
    Error {
        message: String,
        code: String,
    },
    Warning {
        channel: String,
        message: String,
        recommendation: String,
    },
    Pong,
}

async fn log_ws_connection_change(state: &Arc<AppState>, context: &str) {
    if is_debug_webserver_enabled() {
        let active = state.ws_connection_count().await;
        log(LogTag::Webserver, "DEBUG", &format!("{} (active_ws={})", context, active));
    }
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/ws", get(ws_hub_handler)).route("/ws/events", get(ws_events_handler)) // Keep for backward compatibility
}

/// Centralized WebSocket hub handler
pub async fn ws_hub_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    state.increment_ws_connections().await;
    log_ws_connection_change(&state, "Hub WebSocket connection opened").await;
    ws.on_upgrade(move |socket| handle_hub_socket(socket, state))
}

/// Old events-only handler (backward compatibility)
pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsEventsQuery>,
    State(state): State<Arc<AppState>>
) -> Response {
    state.increment_ws_connections().await;
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Legacy events WebSocket connection opened with filters: category={:?} severity={:?} mint={:?} ref={:?} last_id={:?}",
                params.category,
                params.severity,
                params.mint,
                params.reference,
                params.last_id
            )
        );
    }
    log_ws_connection_change(&state, "Events WebSocket connection opened").await;
    ws.on_upgrade(move |socket| handle_events_socket(socket, params, state))
}

/// Handle centralized WebSocket hub connection
async fn handle_hub_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut subscriptions: HashSet<String> = HashSet::new();

    // Subscribe to all broadcast channels
    let mut events_rx = events::subscribe();
    let mut positions_rx = positions::subscribe_positions();
    let mut prices_rx = pools::subscribe_prices();
    let mut status_rx = status_broadcast::subscribe();
    let mut services_rx = crate::webserver::services_broadcast::subscribe();

    // Check if broadcasters are ready
    let has_events = events_rx.is_some();
    let has_positions = positions_rx.is_some();
    let has_prices = prices_rx.is_some();
    let has_status = status_rx.is_some();
    let has_services = services_rx.is_some();

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Hub socket started (events_ready={} positions_ready={} prices_ready={} status_ready={} services_ready={})",
                has_events,
                has_positions,
                has_prices,
                has_status,
                has_services
            )
        );
    }

    if !has_events {
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Events broadcaster unavailable for hub connection");
        }
        let _ = send_error(&mut sender, "Events broadcaster not ready", "EVENTS_NOT_READY").await;
    }
    if !has_positions {
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Positions broadcaster unavailable for hub connection");
        }
        let _ = send_error(
            &mut sender,
            "Positions broadcaster not ready",
            "POSITIONS_NOT_READY"
        ).await;
    }
    if !has_prices {
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Prices broadcaster unavailable for hub connection");
        }
        let _ = send_error(&mut sender, "Prices broadcaster not ready", "PRICES_NOT_READY").await;
    }
    if !has_status {
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Status broadcaster unavailable for hub connection");
        }
        let _ = send_error(&mut sender, "Status broadcaster not ready", "STATUS_NOT_READY").await;
    }
    if !has_services {
        if is_debug_webserver_enabled() {
            log(LogTag::Webserver, "DEBUG", "Services broadcaster unavailable for hub connection");
        }
        let _ = send_error(
            &mut sender,
            "Services broadcaster not ready",
            "SERVICES_NOT_READY"
        ).await;
    }

    // Auto-subscribe to status (always-on channel)
    subscriptions.insert("status".to_string());
    let _ = send_subscribed(&mut sender, "status", "Auto-subscribed to status updates").await;
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "DEBUG", "Client auto-subscribed to status channel");
    }

    // Main multiplexer loop
    loop {
        tokio::select! {
            biased;

            // Client messages (subscribe/unsubscribe/ping)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if is_debug_webserver_enabled() {
                            log(LogTag::Webserver, "DEBUG", &format!("Hub socket received message: {}", text));
                        }
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            handle_client_message(client_msg, &mut subscriptions, &mut sender).await;
                        } else if is_debug_webserver_enabled() {
                            log(LogTag::Webserver, "DEBUG", "Failed to parse client WebSocket message");
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(_))) => {
                        // Axum handles ping/pong automatically
                    }
                    _ => {}
                }
            }

            // Events channel
            evt = async {
                match &mut events_rx {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if subscriptions.contains("events") {
                    match evt {
                        Ok(event) => {
                            let _ = forward_event(&mut sender, &event).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let _ = send_warning(&mut sender, "events", "lagged", "http_catchup").await;
                        }
                        Err(_) => break,
                    }
                }
            }

            // Positions channel
            pos = async {
                match &mut positions_rx {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if subscriptions.contains("positions") {
                    match pos {
                        Ok(update) => {
                            let _ = forward_position(&mut sender, &update).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let _ = send_warning(&mut sender, "positions", "lagged", "http_catchup").await;
                        }
                        Err(_) => break,
                    }
                }
            }

            // Prices channel
            price = async {
                match &mut prices_rx {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if subscriptions.contains("prices") {
                    match price {
                        Ok(update) => {
                            let _ = forward_price(&mut sender, &update).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let _ = send_warning(&mut sender, "prices", "lagged", "http_catchup").await;
                        }
                        Err(_) => break,
                    }
                }
            }

            // Services channel
            services_snapshot = async {
                match &mut services_rx {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if subscriptions.contains("services") {
                    match services_snapshot {
                        Ok(snapshot) => {
                            let _ = forward_services(&mut sender, &snapshot).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            let _ = send_warning(&mut sender, "services", "lagged", "http_catchup").await;
                        }
                        Err(_) => break,
                    }
                }
            }

            // Status channel (always forwarded, no subscription check)
            status = async {
                match &mut status_rx {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                match status {
                    Ok(snapshot) => {
                        let _ = forward_status(&mut sender, &snapshot).await;
                    }
                    Err(_) => {
                        // Status broadcaster died, continue without it
                    }
                }
            }
        }
    }

    state.decrement_ws_connections().await;
    log_ws_connection_change(&state, "Hub WebSocket connection closed").await;
}

/// Handle client messages (subscribe/unsubscribe/ping)
async fn handle_client_message(
    msg: ClientMessage,
    subscriptions: &mut HashSet<String>,
    sender: &mut futures::stream::SplitSink<WebSocket, Message>
) {
    match msg {
        ClientMessage::Subscribe { channel, filters: _ } => {
            // Validate channel
            if !is_valid_channel(&channel) {
                if is_debug_webserver_enabled() {
                    log(
                        LogTag::Webserver,
                        "DEBUG",
                        &format!("Client attempted to subscribe to invalid channel '{}'", channel)
                    );
                }
                let _ = send_error(
                    sender,
                    &format!("Unknown channel: {}", channel),
                    "INVALID_CHANNEL"
                ).await;
                return;
            }

            // Add to subscriptions
            subscriptions.insert(channel.clone());
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Client subscribed to '{}' (total_subscriptions={})",
                        channel,
                        subscriptions.len()
                    )
                );
            }
            let _ = send_subscribed(
                sender,
                &channel,
                &format!("Successfully subscribed to {}", channel)
            ).await;
        }
        ClientMessage::Unsubscribe { channel } => {
            subscriptions.remove(&channel);
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Client unsubscribed from '{}' (remaining_subscriptions={})",
                        channel,
                        subscriptions.len()
                    )
                );
            }
            let _ = send_unsubscribed(
                sender,
                &channel,
                &format!("Successfully unsubscribed from {}", channel)
            ).await;
        }
        ClientMessage::Ping => {
            if is_debug_webserver_enabled() {
                log(LogTag::Webserver, "DEBUG", "Received client ping (responding with pong)");
            }
            let _ = send_pong(sender).await;
        }
    }
}

/// Check if channel is valid
fn is_valid_channel(channel: &str) -> bool {
    matches!(channel, "events" | "positions" | "prices" | "services" | "status")
}

/// Forward event to client
async fn forward_event(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    event: &Event
) -> Result<(), axum::Error> {
    let data = map_event(event);
    let msg = ServerMessage::Data {
        channel: "events".to_string(),
        data: serde_json::to_value(&data).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Forward position update to client
async fn forward_position(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    update: &positions::PositionUpdate
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Data {
        channel: "positions".to_string(),
        data: serde_json::to_value(update).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Forward price update to client
async fn forward_price(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    update: &pools::PriceUpdate
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Data {
        channel: "prices".to_string(),
        data: serde_json::to_value(update).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Forward services snapshot to client
async fn forward_services(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    snapshot: &ServicesOverviewResponse
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Data {
        channel: "services".to_string(),
        data: serde_json::to_value(snapshot).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Forward status snapshot to client
async fn forward_status(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    snapshot: &status_broadcast::StatusSnapshot
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Data {
        channel: "status".to_string(),
        data: serde_json::to_value(snapshot).unwrap_or_default(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Send subscribed confirmation
async fn send_subscribed(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    channel: &str,
    message: &str
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Subscribed {
        channel: channel.to_string(),
        message: message.to_string(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Send unsubscribed confirmation
async fn send_unsubscribed(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    channel: &str,
    message: &str
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Unsubscribed {
        channel: channel.to_string(),
        message: message.to_string(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Send error message
async fn send_error(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    message: &str,
    code: &str
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Error {
        message: message.to_string(),
        code: code.to_string(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Send warning message
async fn send_warning(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>,
    channel: &str,
    message: &str,
    recommendation: &str
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Warning {
        channel: channel.to_string(),
        message: message.to_string(),
        recommendation: recommendation.to_string(),
    };

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

/// Send pong message
async fn send_pong(
    sender: &mut futures::stream::SplitSink<WebSocket, Message>
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Pong;

    if let Ok(text) = serde_json::to_string(&msg) {
        sender.send(Message::Text(text)).await
    } else {
        Ok(())
    }
}

async fn handle_events_socket(mut socket: WebSocket, params: WsEventsQuery, state: Arc<AppState>) {
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Events WebSocket handler started with filters: category={:?} severity={:?} mint={:?} ref={:?} last_id={:?}",
                params.category,
                params.severity,
                params.mint,
                params.reference,
                params.last_id
            )
        );
    }

    // Backfill missed events if last_id provided
    if let Some(after_id) = params.last_id {
        if let Some(db) = events::EVENTS_DB.get() {
            let category = params.category.as_ref().map(|s| EventCategory::from_string(s));
            let severity = params.severity.as_ref().map(|s| Severity::from_string(s));
            let mint = params.mint.as_deref();
            let reference = params.reference.as_deref();
            if
                let Ok(backfill) = db.get_events_since(
                    after_id,
                    500,
                    category,
                    severity,
                    mint,
                    reference
                ).await
            {
                if is_debug_webserver_enabled() {
                    log(
                        LogTag::Webserver,
                        "DEBUG",
                        &format!("Sending {} backfill events", backfill.len())
                    );
                }
                for e in backfill {
                    if let Ok(text) = serde_json::to_string(&map_event(&e)) {
                        if socket.send(Message::Text(text)).await.is_err() {
                            state.decrement_ws_connections().await;
                            log_ws_connection_change(
                                &state,
                                "Events WebSocket closed during backfill send"
                            ).await;
                            return;
                        }
                    }
                }
            }
        }
    }

    // Subscribe to broadcaster
    let mut rx = match events::subscribe() {
        Some(r) => r,
        None => {
            let _ = socket.send(
                Message::Text("{\"error\":\"events broadcaster not ready\"}".into())
            ).await;
            let _ = socket.close().await;
            state.decrement_ws_connections().await;
            log_ws_connection_change(
                &state,
                "Events WebSocket closed - broadcaster unavailable"
            ).await;
            return;
        }
    };

    // Receive loop: forward matching events
    loop {
        tokio::select! {
            biased;
            // If client closes, exit
            msg = socket.recv() => {
                if msg.is_none() { break; }
                // Ignore client messages (ping/pong handled by axum)
            }
            evt = rx.recv() => {
                match evt {
                    Ok(event) => {
                        if !matches_filters(&event, &params) { continue; }
                        match serde_json::to_string(&map_event(&event)) {
                            Ok(text) => {
                                if socket.send(Message::Text(text)).await.is_err() {
                                    if is_debug_webserver_enabled() {
                                        log(LogTag::Webserver, "DEBUG", "Events WebSocket send failed; terminating connection");
                                    }
                                    break;
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Notify client of lag and recommend HTTP catch-up
                        let _ = socket.send(Message::Text("{\"warning\":\"lagged\"}".into())).await;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    state.decrement_ws_connections().await;
    log_ws_connection_change(&state, "Events WebSocket connection closed").await;
}

fn matches_filters(e: &Event, q: &WsEventsQuery) -> bool {
    if let Some(ref c) = q.category {
        if e.category.to_string() != *c {
            return false;
        }
    }
    if let Some(ref s) = q.severity {
        if e.severity.to_string() != *s {
            return false;
        }
    }
    if let Some(ref m) = q.mint {
        if e.mint.as_deref() != Some(m.as_str()) {
            return false;
        }
    }
    if let Some(ref r) = q.reference {
        if e.reference_id.as_deref() != Some(r.as_str()) {
            return false;
        }
    }
    true
}

#[derive(serde::Serialize)]
struct WsEventMessage {
    id: i64,
    event_time: String,
    category: String,
    subtype: Option<String>,
    severity: String,
    mint: Option<String>,
    reference_id: Option<String>,
    message: String,
    payload: serde_json::Value,
    created_at: String,
}

fn map_event(e: &Event) -> WsEventMessage {
    let message = e.payload
        .get("message")
        .and_then(|v| v.as_str())
        .unwrap_or("No message")
        .to_string();
    WsEventMessage {
        id: e.id.unwrap_or(0),
        event_time: e.event_time.to_rfc3339(),
        category: e.category.to_string(),
        subtype: e.subtype.clone(),
        severity: e.severity.to_string(),
        mint: e.mint.clone(),
        reference_id: e.reference_id.clone(),
        message,
        payload: e.payload.clone(),
        created_at: e.created_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    }
}
