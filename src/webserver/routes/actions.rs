use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt as _;

use crate::webserver::state::AppState;

/// Active actions response
#[derive(Debug, Serialize)]
pub struct ActiveActionsResponse {
    pub actions: Vec<crate::actions::Action>,
    pub count: usize,
    pub in_progress: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Action history response with pagination
#[derive(Debug, Serialize)]
pub struct ActionHistoryResponse {
    pub actions: Vec<crate::actions::Action>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Action history query parameters
#[derive(Debug, Deserialize)]
pub struct ActionHistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
    pub action_type: Option<String>,
    pub entity_id: Option<String>,
    pub state: Option<String>,
    pub started_after: Option<String>,
    pub started_before: Option<String>,
}

fn default_limit() -> usize {
    50
}

/// Subscriber count response
#[derive(Debug, Serialize)]
pub struct SubscriberCountResponse {
    pub subscriber_count: usize,
}

/// Create actions routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/actions/stream", get(stream_actions))
        .route("/actions/active", get(get_active_actions))
        .route("/actions/all", get(get_all_actions))
        .route("/actions/history", get(get_action_history))
        .route("/actions/:action_id", get(get_action_by_id))
        .route("/actions/subscribers", get(get_subscriber_count))
}

/// Server-Sent Events stream for real-time action updates
async fn stream_actions(
    State(_state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe to action broadcast channel
    let mut rx = crate::actions::subscribe();

    // Create stream that converts ActionUpdate to SSE Event
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(update) => {
                    // Serialize update to JSON
                    match serde_json::to_string(&update) {
                        Ok(json) => {
                            yield Ok(Event::default().data(json));
                        }
                        Err(e) => {
                            eprintln!("Failed to serialize action update: {}", e);
                            continue;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    // Client lagged behind - send informational message
                    let lag_msg = serde_json::json!({
                        "type": "lag",
                        "skipped": skipped,
                        "message": format!("Client lagged behind, {} updates skipped", skipped)
                    });
                    if let Ok(json) = serde_json::to_string(&lag_msg) {
                        yield Ok(Event::default().event("lag").data(json));
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    // Channel closed - end stream
                    break;
                }
            }
        }
    };

    // Configure SSE with keep-alive
    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    )
}

/// Get currently active actions (in-progress only)
async fn get_active_actions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let actions = crate::actions::get_active_actions().await;
    let count = actions.len();
    let (in_progress, completed, failed, _cancelled) = crate::actions::get_action_counts().await;

    Json(ActiveActionsResponse {
        actions,
        count,
        in_progress,
        completed,
        failed,
    })
}

/// Get all actions (including completed/failed)
async fn get_all_actions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let actions = crate::actions::get_all_actions().await;
    let total = actions.len();

    Json(ActionHistoryResponse {
        actions,
        total,
        limit: 0,
        offset: 0,
    })
}

/// Get action history with pagination and filters
async fn get_action_history(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<ActionHistoryQuery>,
) -> impl IntoResponse {
    // Build filters from query parameters
    let mut filters = crate::actions::ActionFilters::default();
    filters.limit = Some(query.limit);
    filters.offset = Some(query.offset);
    
    // Parse action type
    if let Some(action_type_str) = query.action_type {
        filters.action_type = match action_type_str.to_lowercase().as_str() {
            "swapbuy" => Some(crate::actions::ActionType::SwapBuy),
            "swapsell" => Some(crate::actions::ActionType::SwapSell),
            "positionopen" => Some(crate::actions::ActionType::PositionOpen),
            "positionclose" => Some(crate::actions::ActionType::PositionClose),
            "positiondca" => Some(crate::actions::ActionType::PositionDca),
            "positionpartialexit" => Some(crate::actions::ActionType::PositionPartialExit),
            "manualorder" => Some(crate::actions::ActionType::ManualOrder),
            _ => None,
        };
    }
    
    filters.entity_id = query.entity_id;
    
    // Parse state filters
    if let Some(state) = query.state {
        filters.state = Some(vec![state]);
    }
    
    // Parse datetime filters if provided
    if let Some(after) = query.started_after {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&after) {
            filters.started_after = Some(dt.with_timezone(&Utc));
        }
    }
    if let Some(before) = query.started_before {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&before) {
            filters.started_before = Some(dt.with_timezone(&Utc));
        }
    }

    // Fetch from database using helper function
    match crate::actions::query_action_history(filters).await {
        Ok((actions, total)) => {
            Json(ActionHistoryResponse {
                actions,
                total,
                limit: query.limit,
                offset: query.offset,
            })
        }
        Err(e) => {
            crate::logger::error(
                crate::logger::LogTag::System,
                &format!("Failed to fetch action history: {}", e),
            );
            Json(ActionHistoryResponse {
                actions: vec![],
                total: 0,
                limit: query.limit,
                offset: query.offset,
            })
        }
    }
}

/// Get single action by ID
async fn get_action_by_id(
    State(_state): State<Arc<AppState>>,
    Path(action_id): Path<String>,
) -> impl IntoResponse {
    match crate::actions::get_action(&action_id).await {
        Some(action) => Json(serde_json::json!({
            "success": true,
            "action": action
        })),
        None => Json(serde_json::json!({
            "success": false,
            "error": format!("Action {} not found", action_id)
        })),
    }
}

/// Get current subscriber count
async fn get_subscriber_count(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let count = crate::actions::subscriber_count();

    Json(SubscriberCountResponse {
        subscriber_count: count,
    })
}
