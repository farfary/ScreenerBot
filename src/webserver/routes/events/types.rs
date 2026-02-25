use serde::{Deserialize, Serialize};

/// Default limit for pagination
pub(super) fn default_limit() -> usize {
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
    pub total_count: Option<i64>,
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
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SinceQuery {
    pub after_id: i64,
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BeforeQuery {
    pub before_id: i64,
    pub limit: Option<usize>,
    pub category: Option<String>,
    pub severity: Option<String>,
    pub mint: Option<String>,
    pub reference: Option<String>,
    pub search: Option<String>,
}
