use axum::{extract::Query, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{events, webserver::state::AppState};

fn default_limit() -> usize {
    100
}

/// Event response structure
#[derive(Debug, Serialize)]
pub struct EventResponse {
    pub id: i64,
    pub event_time: String,
    pub category: String,
    pub subtype: Option<String>,
    pub severity: String,
    pub mint: Option<String>,
    pub reference_id: Option<String>,
    pub message: String, // Extracted from json_payload
    pub payload: serde_json::Value,
    pub created_at: String,
}

/// Events list response with cursor
#[derive(Debug, Serialize)]
pub struct EventsListResponse {
    pub events: Vec<EventResponse>,
    pub count: usize,
    pub max_id: i64,
    pub timestamp: String,
}

#[derive(Debug, Deserialize)]
pub struct HeadQuery {
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct SinceQuery {
    pub after_id: i64,
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct BeforeQuery {
    pub before_id: i64,
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
}

/// Create events routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/events/head", get(get_events_head))
        .route("/events/since", get(get_events_since))
        .route("/events/before", get(get_events_before))
        .route("/events/categories", get(get_categories))
}

/// Get latest events (head) with cursor
async fn get_events_head(Query(params): Query<HeadQuery>) -> Json<EventsListResponse> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let category = params
        .category
        .as_ref()
        .map(|s| events::EventCategory::from_string(s));
    let severity = params
        .severity
        .as_ref()
        .map(|s| events::Severity::from_string(s));
    let mint = params.mint.as_deref();
    let reference = params.reference.as_deref();

    let db = crate::events::EVENTS_DB
        .get()
        .expect("events DB not initialized")
        .clone();
    let (events_vec, max_id) = db
        .get_events_head(limit, category, severity, mint, reference)
        .await
        .unwrap_or((Vec::new(), 0));

    let event_responses: Vec<EventResponse> = events_vec
        .into_iter()
        .map(|e| {
            // Extract message from payload
            let message = e
                .payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("No message")
                .to_string();

            EventResponse {
                id: e.id.unwrap_or(0),
                event_time: e.event_time.to_rfc3339(),
                category: e.category.to_string(),
                subtype: e.subtype,
                severity: e.severity.to_string(),
                mint: e.mint,
                reference_id: e.reference_id,
                message,
                payload: e.payload.clone(),
                created_at: e
                    .created_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            }
        })
        .collect();

    let count = event_responses.len();
    Json(EventsListResponse {
        events: event_responses,
        count,
        max_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get events newer than a cursor (since)
async fn get_events_since(Query(params): Query<SinceQuery>) -> Json<EventsListResponse> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let category = params
        .category
        .as_ref()
        .map(|s| events::EventCategory::from_string(s));
    let severity = params
        .severity
        .as_ref()
        .map(|s| events::Severity::from_string(s));
    let mint = params.mint.as_deref();
    let reference = params.reference.as_deref();
    let after_id = params.after_id;

    let db = crate::events::EVENTS_DB
        .get()
        .expect("events DB not initialized")
        .clone();
    let events_vec = db
        .get_events_since(after_id, limit, category, severity, mint, reference)
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
            let message = e
                .payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("No message")
                .to_string();
            EventResponse {
                id: e.id.unwrap_or(0),
                event_time: e.event_time.to_rfc3339(),
                category: e.category.to_string(),
                subtype: e.subtype,
                severity: e.severity.to_string(),
                mint: e.mint,
                reference_id: e.reference_id,
                message,
                payload: e.payload.clone(),
                created_at: e
                    .created_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            }
        })
        .collect();

    let count = event_responses.len();
    Json(EventsListResponse {
        events: event_responses,
        count,
        max_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get events older than a cursor (before)
async fn get_events_before(Query(params): Query<BeforeQuery>) -> Json<EventsListResponse> {
    let limit = params.limit.unwrap_or(200).min(1000);
    let category = params
        .category
        .as_ref()
        .map(|s| events::EventCategory::from_string(s));
    let severity = params
        .severity
        .as_ref()
        .map(|s| events::Severity::from_string(s));
    let mint = params.mint.as_deref();
    let reference = params.reference.as_deref();
    let before_id = params.before_id;

    let db = crate::events::EVENTS_DB
        .get()
        .expect("events DB not initialized")
        .clone();
    let events_vec = db
        .get_events_before(before_id, limit, category, severity, mint, reference)
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
            let message = e
                .payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("No message")
                .to_string();
            EventResponse {
                id: e.id.unwrap_or(0),
                event_time: e.event_time.to_rfc3339(),
                category: e.category.to_string(),
                subtype: e.subtype,
                severity: e.severity.to_string(),
                mint: e.mint,
                reference_id: e.reference_id,
                message,
                payload: e.payload.clone(),
                created_at: e
                    .created_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            }
        })
        .collect();

    let count = event_responses.len();
    Json(EventsListResponse {
        events: event_responses,
        count,
        max_id,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get available event categories with counts
async fn get_categories() -> Json<serde_json::Value> {
    let counts = events::count_by_category(24).await.unwrap_or_default();

    Json(serde_json::json!({
        "categories": counts,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}
