// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Events API handlers - cursor-paginated event listings by head, since and before cursors, plus category counts.

use axum::Json;
use chrono;

use super::types::*;

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
    let mint = params.mint.as_deref();
    let reference = params.reference.as_deref();
    let search = params.search.as_deref();

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
    let (events_vec, max_id) = db
        .get_events_head(limit, category, severity, mint, reference, search)
        .await
        .unwrap_or((Vec::new(), 0));

    // Get total count with same filters (recreate from params since we moved them)
    let category_for_count = params
        .category
        .as_ref()
        .map(|s| crate::events::EventCategory::from_string(s));
    let severity_for_count = params
        .severity
        .as_ref()
        .map(|s| crate::events::Severity::from_string(s));
    let total_count = db
        .count_events_filtered(
            category_for_count,
            severity_for_count,
            mint,
            reference,
            search,
        )
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
    let mint = params.mint.as_deref();
    let reference = params.reference.as_deref();
    let search = params.search.as_deref();
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
    let events_vec = db
        .get_events_since(after_id, limit, category, severity, mint, reference, search)
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
    let mint = params.mint.as_deref();
    let reference = params.reference.as_deref();
    let search = params.search.as_deref();
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
    let events_vec = db
        .get_events_before(
            before_id, limit, category, severity, mint, reference, search,
        )
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
