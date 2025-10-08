/// WebSocket connection handler
///
/// Manages individual WebSocket connections with:
/// - Upgrade handshake
/// - Control message handling (subscribe/unsubscribe/ping/filters)
/// - Message forwarding from hub to client
/// - Health monitoring and heartbeat
/// - Graceful shutdown

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

use crate::{
    arguments::is_debug_webserver_enabled,
    config,
    logger::{log, LogTag},
};

use super::{
    health::{ConnectionHealth, HealthConfig},
    hub::{ConnectionId, WsHub},
    message::{ClientMessage, ServerMessage, WsEnvelope},
    metrics::ConnectionMetrics,
};

/// Handle a WebSocket connection
pub async fn handle_connection(socket: WebSocket, hub: Arc<WsHub>) {
    // Register connection with hub
    let (conn_id, mut hub_rx) = hub.register_connection().await;
    
    // Split socket
    let (mut ws_tx, mut ws_rx) = socket.split();
    
    // Initialize health tracker
    let health_config = config::with_config(|cfg| {
        HealthConfig::from_config(
            cfg.webserver.websocket.heartbeat_secs,
            cfg.webserver.websocket.client_idle_timeout_secs,
        )
    });
    let mut health = ConnectionHealth::new(health_config);
    
    // Initialize metrics
    let metrics = ConnectionMetrics::new();
    
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!("Connection {} started", conn_id),
        );
    }
    
    // Main message loop
    loop {
        tokio::select! {
            biased;
            
            // Messages from hub (broadcast to client)
            Some(envelope) = hub_rx.recv() => {
                if let Err(e) = forward_to_client(&mut ws_tx, envelope, &metrics).await {
                    log(
                        LogTag::Webserver,
                        "WARN",
                        &format!("Connection {}: failed to send message: {}", conn_id, e),
                    );
                    break;
                }
            }
            
            // Messages from client (control commands)
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        health.record_activity();
                        
                        if let Err(e) = handle_client_message(&text, &mut ws_tx, conn_id).await {
                            log(
                                LogTag::Webserver,
                                "WARN",
                                &format!("Connection {}: error handling client message: {}", conn_id, e),
                            );
                        }
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {
                        health.record_activity();
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        if is_debug_webserver_enabled() {
                            log(
                                LogTag::Webserver,
                                "DEBUG",
                                &format!("Connection {}: client closed", conn_id),
                            );
                        }
                        break;
                    }
                    Some(Err(e)) => {
                        log(
                            LogTag::Webserver,
                            "WARN",
                            &format!("Connection {}: websocket error: {}", conn_id, e),
                        );
                        break;
                    }
                    _ => {}
                }
            }
            
            // Health checks
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                // Check if client is idle
                if health.is_idle() {
                    log(
                        LogTag::Webserver,
                        "WARN",
                        &format!(
                            "Connection {}: idle timeout ({}s)",
                            conn_id,
                            health.seconds_since_activity()
                        ),
                    );
                    break;
                }
                
                // Check if pong is overdue
                if health.is_pong_overdue() {
                    log(
                        LogTag::Webserver,
                        "WARN",
                        &format!("Connection {}: pong timeout", conn_id),
                    );
                    break;
                }
                
                // Send ping if needed
                if health.needs_ping() {
                    if is_debug_webserver_enabled() {
                        log(
                            LogTag::Webserver,
                            "DEBUG",
                            &format!("Connection {}: sending ping", conn_id),
                        );
                    }
                    if let Err(_) = ws_tx.send(Message::Ping(vec![])).await {
                        break;
                    }
                    health.record_ping();
                }
            }
        }
    }
    
    // Cleanup
    hub.unregister_connection(conn_id).await;
    
    if is_debug_webserver_enabled() {
        let snapshot = metrics.snapshot();
        log(
            LogTag::Webserver,
            "DEBUG",
            &format!(
                "Connection {} closed (sent={}, dropped={}, lag_events={})",
                conn_id, snapshot.messages_sent, snapshot.messages_dropped, snapshot.lag_events
            ),
        );
    }
}

/// Forward envelope to client
async fn forward_to_client(
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    envelope: WsEnvelope,
    metrics: &Arc<ConnectionMetrics>,
) -> Result<(), axum::Error> {
    let msg = ServerMessage::Data(envelope);
    
    match msg.to_json() {
        Ok(json) => {
            ws_tx.send(Message::Text(json)).await?;
            metrics.inc_sent();
            Ok(())
        }
        Err(e) => {
            log(
                LogTag::Webserver,
                "ERROR",
                &format!("Failed to serialize message: {}", e),
            );
            Ok(()) // Don't break connection on serialization error
        }
    }
}

/// Handle client control message
async fn handle_client_message(
    text: &str,
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    conn_id: ConnectionId,
) -> Result<(), String> {
    let client_msg: ClientMessage = serde_json::from_str(text)
        .map_err(|e| format!("Invalid client message: {}", e))?;
    
    match client_msg {
        ClientMessage::Hello { client_id, app_version, pages_supported } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!(
                        "Connection {}: hello (client_id={:?}, version={:?}, pages={:?})",
                        conn_id, client_id, app_version, pages_supported
                    ),
                );
            }
            
            let response = ServerMessage::Ack {
                message: "Hello acknowledged".to_string(),
                context: Some(serde_json::json!({
                    "connection_id": conn_id,
                    "protocol_version": super::message::PROTOCOL_VERSION,
                })),
            };
            
            send_control_message(ws_tx, response).await?;
        }
        
        ClientMessage::SetFilters { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: set_filters ({} topics)", conn_id, topics.len()),
                );
            }
            
            // TODO: Store filters and apply them in hub routing
            // For now, just acknowledge
            let response = ServerMessage::Ack {
                message: format!("Filters updated for {} topics", topics.len()),
                context: Some(serde_json::json!({
                    "topics": topics.keys().collect::<Vec<_>>(),
                })),
            };
            
            send_control_message(ws_tx, response).await?;
        }
        
        ClientMessage::Pause { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: pause ({:?})", conn_id, topics),
                );
            }
            
            // TODO: Implement pause logic
            let response = ServerMessage::Ack {
                message: "Pause acknowledged".to_string(),
                context: Some(serde_json::json!({"topics": topics})),
            };
            
            send_control_message(ws_tx, response).await?;
        }
        
        ClientMessage::Resume { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: resume ({:?})", conn_id, topics),
                );
            }
            
            // TODO: Implement resume logic
            let response = ServerMessage::Ack {
                message: "Resume acknowledged".to_string(),
                context: Some(serde_json::json!({"topics": topics})),
            };
            
            send_control_message(ws_tx, response).await?;
        }
        
        ClientMessage::Resync { topics } => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "DEBUG",
                    &format!("Connection {}: resync ({} topics)", conn_id, topics.len()),
                );
            }
            
            // TODO: Implement resync with snapshots
            let response = ServerMessage::Ack {
                message: "Resync acknowledged".to_string(),
                context: Some(serde_json::json!({
                    "topics": topics.keys().collect::<Vec<_>>(),
                })),
            };
            
            send_control_message(ws_tx, response).await?;
        }
        
        ClientMessage::Ping { id } => {
            let response = ServerMessage::Pong { id };
            send_control_message(ws_tx, response).await?;
        }
    }
    
    Ok(())
}

/// Send control message to client
async fn send_control_message(
    ws_tx: &mut futures::stream::SplitSink<WebSocket, Message>,
    msg: ServerMessage,
) -> Result<(), String> {
    let json = msg.to_json().map_err(|e| format!("Serialization error: {}", e))?;
    ws_tx
        .send(Message::Text(json))
        .await
        .map_err(|e| format!("Send error: {}", e))?;
    Ok(())
}
