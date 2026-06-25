use axum::{
    extract::{Path, Query, State},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    Json,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt as _;

use crate::webserver::state::AppState;
use super::types::*;

/// Server-Sent Events stream for real-time action updates
pub(super) async fn stream_actions(
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
                            eprintln!("Failed to serialize action update: {e}");
                            continue;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    // Client lagged behind - send informational message
                    let lag_msg = serde_json::json!({
                        "type": "lag",
                        "skipped": skipped,
                        "message": format!("Client lagged behind, {skipped} updates skipped")
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
pub(super) async fn get_active_actions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
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
pub(super) async fn get_all_actions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
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
pub(super) async fn get_action_history(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<ActionHistoryQuery>,
) -> impl IntoResponse {
    // Build filters from query parameters
    let mut filters = crate::actions::ActionFilters::default();
    filters.limit = Some(query.limit);
    filters.offset = Some(query.offset);

    // Parse action type. Accept both the DB form ("swapbuy") and the API/JSON
    // snake_case form the UI sends ("swap_buy") by stripping underscores.
    if let Some(action_type_str) = query.action_type {
        let normalized = action_type_str.to_lowercase().replace('_', "");
        filters.action_type = match normalized.as_str() {
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
        Ok((actions, total)) => Json(ActionHistoryResponse {
            actions,
            total,
            limit: query.limit,
            offset: query.offset,
        }),
        Err(e) => {
            crate::logger::error(
                crate::logger::LogTag::System,
                &format!("Failed to fetch action history: {e}"),
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
pub(super) async fn get_action_by_id(
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
            "error": format!("Action {action_id} not found")
        })),
    }
}

/// Get current subscriber count
pub(super) async fn get_subscriber_count(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let count = crate::actions::subscriber_count();

    Json(SubscriberCountResponse {
        subscriber_count: count,
    })
}
