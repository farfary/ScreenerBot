/// Events API routes
///
/// Provides endpoints for accessing system events from the events database

use axum::{ extract::Query, routing::get, Json, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::{ events, webserver::state::AppState };

/// Query parameters for events endpoint
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Category filter (optional)
    pub category: Option<String>,

    /// Limit number of results
    #[serde(default = "default_limit")]
    pub limit: usize,

    /// Severity filter (optional)
    pub severity: Option<String>,

    /// Search by reference_id (tx signature, pool address, etc.)
    pub reference: Option<String>,

    /// Search by mint address
    pub mint: Option<String>,
}

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
    pub created_at: String,
}

/// Events list response
#[derive(Debug, Serialize)]
pub struct EventsListResponse {
    pub events: Vec<EventResponse>,
    pub count: usize,
    pub timestamp: String,
}

/// Create events routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/events", get(get_events)).route("/events/categories", get(get_categories))
}

/// Get events with optional filtering
async fn get_events(Query(params): Query<EventsQuery>) -> Json<EventsListResponse> {
    // Validate limit
    let limit = params.limit.min(1000);

    // Get events from database
    let events_result = if let Some(reference) = params.reference {
        events::by_reference(&reference, limit).await
    } else if let Some(mint) = params.mint {
        events::by_mint(&mint, limit).await
    } else if let Some(category_str) = params.category {
        let category = events::EventCategory::from_string(&category_str);
        events::recent(category, limit).await
    } else {
        events::recent_all(limit).await
    };

    let events_vec = events_result.unwrap_or_default();

    // Filter by severity if specified
    let filtered_events: Vec<_> = if let Some(severity) = params.severity {
        events_vec
            .into_iter()
            .filter(|e| e.severity.to_string().eq_ignore_ascii_case(&severity))
            .collect()
    } else {
        events_vec
    };

    // Convert to response format
    let event_responses: Vec<EventResponse> = filtered_events
        .into_iter()
        .map(|e| {
            // Extract message from payload
            let message = e.payload
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
                created_at: e.created_at
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
            }
        })
        .collect();

    let count = event_responses.len();

    Json(EventsListResponse {
        events: event_responses,
        count,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get available event categories with counts
async fn get_categories() -> Json<serde_json::Value> {
    let counts = events::count_by_category(24).await.unwrap_or_default();

    Json(
        serde_json::json!({
        "categories": counts,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })
    )
}
