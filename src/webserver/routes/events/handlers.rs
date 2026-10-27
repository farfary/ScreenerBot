// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Events API handlers - cursor-paginated event listings by head, since and before cursors, plus category counts.

use axum::Json;
use chrono;

use super::types::*;
use crate::events::on_blocking_pool;

/// Get latest events (head) with cursor
pub(super) async fn get_events_head(
    axum::extract::Query(params): axum::extract::Query<HeadQuery>,
) -> Json<EventsListResponse> {
    let limit = params.limit.unwrap_or(200).min(1000);

    // Return promotional fixtures only for owner-initiated media capture.
    if crate::webserver::promo::are_promo_fixtures_enabled() {
        return Json(crate::webserver::promo::get_promo_events(
            limit,
            params.category.as_deref(),
            params.severity.as_deref(),
            params.mint.as_deref(),
            params.search.as_deref(),
        ));
    }

    let category = params
        .category
        .as_ref()
        .map(|s| crate::events::EventCategory::from_string(s));
    let severity = params
        .severity
        .as_ref()
        .map(|s| crate::events::Severity::from_string(s));
    let mint = params.mint.clone();
    let reference = params.reference.clone();
    let search = params.search.clone();

    let db = match crate::events::EVENTS_DB.get() {
        Some(db) => db.clone(),
        None => {
            return Json(EventsListResponse {
                events: vec![],
                count: 0,
                total_count: None,
                max_id: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };
    let (events_vec, max_id) = {
        let (mint, reference, search) = (mint.clone(), reference.clone(), search.clone());
        on_blocking_pool(&db, move |store| {
            store.get_events_head(
                limit,
                category,
                severity,
                mint.as_deref(),
                reference.as_deref(),
                search.as_deref(),
            )
        })
        .await
        .unwrap_or((Vec::new(), 0))
    };

    // Get total count with same filters (recreate from params since we moved them)
    let category_for_count = params
        .category
        .as_ref()
        .map(|s| crate::events::EventCategory::from_string(s));
    let severity_for_count = params
        .severity
        .as_ref()
        .map(|s| crate::events::Severity::from_string(s));
    let total_count = on_blocking_pool(&db, move |store| {
        store.count_events_filtered(
            category_for_count,
            severity_for_count,
            mint.as_deref(),
            reference.as_deref(),
            search.as_deref(),
        )
    })
    .await
    .ok();

    let event_responses: Vec<EventResponse> = events_vec
        .into_iter()
        .map(EventResponse::from_event)
        .collect();

    let count = event_responses.len();
    Json(EventsListResponse {
        events: event_responses,
        count,
        total_count,
        max_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get events newer than a cursor (since)
pub(super) async fn get_events_since(
    axum::extract::Query(params): axum::extract::Query<SinceQuery>,
) -> Json<EventsListResponse> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let category = params
        .category
        .as_ref()
        .map(|s| crate::events::EventCategory::from_string(s));
    let severity = params
        .severity
        .as_ref()
        .map(|s| crate::events::Severity::from_string(s));
    let (mint, reference, search) = (params.mint, params.reference, params.search);
    let after_id = params.after_id;

    let db = match crate::events::EVENTS_DB.get() {
        Some(db) => db.clone(),
        None => {
            return Json(EventsListResponse {
                events: vec![],
                count: 0,
                total_count: None,
                max_id: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };
    let events_vec = on_blocking_pool(&db, move |store| {
        store.get_events_since(
            after_id,
            limit,
            category,
            severity,
            mint.as_deref(),
            reference.as_deref(),
            search.as_deref(),
        )
    })
    .await
    .unwrap_or_default();

    let mut max_id = after_id;
    let event_responses: Vec<EventResponse> = events_vec
        .into_iter()
        .map(|e| {
            if let Some(id) = e.id {
                if id > max_id {
                    max_id = id;
                }
            }
            EventResponse::from_event(e)
        })
        .collect();

    let count = event_responses.len();
    Json(EventsListResponse {
        events: event_responses,
        count,
        total_count: None,
        max_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get events older than a cursor (before)
pub(super) async fn get_events_before(
    axum::extract::Query(params): axum::extract::Query<BeforeQuery>,
) -> Json<EventsListResponse> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let category = params
        .category
        .as_ref()
        .map(|s| crate::events::EventCategory::from_string(s));
    let severity = params
        .severity
        .as_ref()
        .map(|s| crate::events::Severity::from_string(s));
    let (mint, reference, search) = (params.mint, params.reference, params.search);
    let before_id = params.before_id;

    let db = match crate::events::EVENTS_DB.get() {
        Some(db) => db.clone(),
        None => {
            return Json(EventsListResponse {
                events: vec![],
                count: 0,
                total_count: None,
                max_id: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };
    let events_vec = on_blocking_pool(&db, move |store| {
        store.get_events_before(
            before_id,
            limit,
            category,
            severity,
            mint.as_deref(),
            reference.as_deref(),
            search.as_deref(),
        )
    })
    .await
    .unwrap_or_default();

    let mut max_id = 0;
    let event_responses: Vec<EventResponse> = events_vec
        .into_iter()
        .map(|e| {
            if let Some(id) = e.id {
                if id > max_id {
                    max_id = id;
                }
            }
            EventResponse::from_event(e)
        })
        .collect();

    let count = event_responses.len();
    Json(EventsListResponse {
        events: event_responses,
        count,
        total_count: None,
        max_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get available event categories with counts
pub(super) async fn get_categories() -> Json<serde_json::Value> {
    let counts = crate::events::count_by_category(24)
        .await
        .unwrap_or_default();

    Json(serde_json::json!({
        "categories": counts,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
