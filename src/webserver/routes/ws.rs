use std::sync::Arc;

use axum::{
    extract::{ ws::{ Message, WebSocket, WebSocketUpgrade }, Query, State },
    response::Response,
    routing::get,
    Router,
};
use futures::{ SinkExt, StreamExt };
use serde::Deserialize;

use crate::{ events::{ self, Event, EventCategory, Severity }, webserver::state::AppState };

#[derive(Debug, Deserialize, Clone)]
pub struct WsEventsQuery {
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
    pub last_id: Option<i64>,
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/ws/events", get(ws_events_handler))
}

pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<WsEventsQuery>,
    State(state): State<Arc<AppState>>
) -> Response {
    state.increment_ws_connections().await;
    ws.on_upgrade(move |socket| handle_events_socket(socket, params, state))
}

async fn handle_events_socket(mut socket: WebSocket, params: WsEventsQuery, state: Arc<AppState>) {
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
                for e in backfill {
                    if let Ok(text) = serde_json::to_string(&map_event(&e)) {
                        if socket.send(Message::Text(text)).await.is_err() {
                            state.decrement_ws_connections().await;
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
                                if socket.send(Message::Text(text)).await.is_err() { break; }
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
        created_at: e.created_at
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
    }
}
