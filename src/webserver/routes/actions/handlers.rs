// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Actions API handlers - the SSE update stream, active and history listings, and read and dismiss mutations.

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

use super::outcome::with_outcomes;
use super::types::*;
use crate::actions::ActionType;
use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use crate::webserver::state::AppState;

/// Server-Sent Events stream for real-time action updates
pub(super) async fn stream_actions(
    State(_state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe to action broadcast channel
    let mut rx = crate::actions::subscribe();

    // Create stream that converts ActionUpdate to SSE Event
    let stream = async_stream::stream! {
        loop {
            let received = tokio::select! {
                update = rx.recv() => update,
                _ = crate::webserver::shutdown_notified() => break,
            };
            match received {
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
    let unread = crate::actions::get_unread_count().await;

    Json(ActiveActionsResponse {
        actions,
        count,
        in_progress,
        completed,
        failed,
        unread,
    })
}

/// Get all actions (including completed/failed)
pub(super) async fn get_all_actions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let actions = crate::actions::get_all_actions().await;
    let total = actions.len();

    Json(ActionHistoryResponse {
        actions: with_outcomes(actions).await,
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

    if let Some(action_type) = query.action_type.as_deref() {
        filters.action_types = action_types_for_filter(action_type);
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
            actions: with_outcomes(actions).await,
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

/// The action types a history `action_type` filter selects. The panel filters by trade
/// side, and each side is recorded under more than one type: a manual buy is `SwapBuy`,
/// a strategy buy `PositionOpen`. A single type id is accepted in its serialized form
/// (`swap_buy`) and its stored form (`swapbuy`); an unknown value filters nothing.
fn action_types_for_filter(value: &str) -> Vec<ActionType> {
    match value.to_lowercase().replace('_', "").as_str() {
        "buy" => vec![ActionType::SwapBuy, ActionType::PositionOpen],
        "sell" => vec![
            ActionType::SwapSell,
            ActionType::PositionClose,
            ActionType::PositionPartialExit,
        ],
        "dca" | "positiondca" => vec![ActionType::PositionDca],
        "swapbuy" => vec![ActionType::SwapBuy],
        "swapsell" => vec![ActionType::SwapSell],
        "positionopen" => vec![ActionType::PositionOpen],
        "positionclose" => vec![ActionType::PositionClose],
        "positionpartialexit" => vec![ActionType::PositionPartialExit],
        "manualorder" => vec![ActionType::ManualOrder],
        _ => Vec::new(),
    }
}

/// Get single action by ID
pub(super) async fn get_action_by_id(
    State(_state): State<Arc<AppState>>,
    Path(action_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    match crate::actions::get_action(&action_id).await {
        Some(action) => Ok(Json(serde_json::json!({
            "success": true,
            "action": action
        }))),
        None => Err(
            ApiError::new(ApiErrorCode::NotFound, ids::ERRORS_ACTIONS_NOT_FOUND)
                .text_arg("id", action_id),
        ),
    }
}

/// Mark one action notification as read
pub(super) async fn mark_action_read(
    State(_state): State<Arc<AppState>>,
    Path(action_id): Path<String>,
) -> impl IntoResponse {
    match crate::actions::mark_action_read(&action_id).await {
        Ok(found) => Json(ActionMutationResponse {
            success: found,
            updated: usize::from(found),
            unread: crate::actions::get_unread_count().await,
        }),
        Err(e) => {
            crate::logger::error(
                crate::logger::LogTag::System,
                &format!("Failed to mark action {action_id} read: {e}"),
            );
            Json(ActionMutationResponse {
                success: false,
                updated: 0,
                unread: crate::actions::get_unread_count().await,
            })
        }
    }
}

/// Mark every action notification as read
pub(super) async fn mark_all_actions_read(
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match crate::actions::mark_all_actions_read().await {
        Ok(updated) => Json(ActionMutationResponse {
            success: true,
            updated,
            unread: crate::actions::get_unread_count().await,
        }),
        Err(e) => {
            crate::logger::error(
                crate::logger::LogTag::System,
                &format!("Failed to mark all actions read: {e}"),
            );
            Json(ActionMutationResponse {
                success: false,
                updated: 0,
                unread: crate::actions::get_unread_count().await,
            })
        }
    }
}

/// Dismiss one action notification from live lists
pub(super) async fn dismiss_action(
    State(_state): State<Arc<AppState>>,
    Path(action_id): Path<String>,
) -> impl IntoResponse {
    match crate::actions::dismiss_action(&action_id).await {
        Ok(found) => Json(ActionMutationResponse {
            success: found,
            updated: usize::from(found),
            unread: crate::actions::get_unread_count().await,
        }),
        Err(e) => {
            crate::logger::error(
                crate::logger::LogTag::System,
                &format!("Failed to dismiss action {action_id}: {e}"),
            );
            Json(ActionMutationResponse {
                success: false,
                updated: 0,
                unread: crate::actions::get_unread_count().await,
            })
        }
    }
}

/// Dismiss every action notification from live lists while keeping history rows
pub(super) async fn dismiss_all_actions(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::actions::dismiss_all_actions().await {
        Ok(updated) => Json(ActionMutationResponse {
            success: true,
            updated,
            unread: crate::actions::get_unread_count().await,
        }),
        Err(e) => {
            crate::logger::error(
                crate::logger::LogTag::System,
                &format!("Failed to dismiss all actions: {e}"),
            );
            Json(ActionMutationResponse {
                success: false,
                updated: 0,
                unread: crate::actions::get_unread_count().await,
            })
        }
    }
}

/// Get current subscriber count
pub(super) async fn get_subscriber_count(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    let count = crate::actions::subscriber_count();

    Json(SubscriberCountResponse {
        subscriber_count: count,
    })
}

#[cfg(test)]
mod tests {
    use super::action_types_for_filter;
    use crate::actions::ActionType;

    #[test]
    fn a_trade_side_selects_every_type_it_is_recorded_under() {
        assert_eq!(
            action_types_for_filter("buy"),
            vec![ActionType::SwapBuy, ActionType::PositionOpen]
        );
        assert!(action_types_for_filter("sell").contains(&ActionType::PositionClose));
        assert_eq!(
            action_types_for_filter("dca"),
            vec![ActionType::PositionDca]
        );
    }

    #[test]
    fn a_single_type_is_accepted_in_both_spellings() {
        for spelling in ["swap_buy", "swapbuy", "SWAP_BUY"] {
            assert_eq!(action_types_for_filter(spelling), vec![ActionType::SwapBuy]);
        }
        assert!(action_types_for_filter("anything").is_empty());
    }
}
