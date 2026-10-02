// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Types for the events API - EventResponse, EventsListResponse and the head, since and before query structs.

use crate::events::Event;
use crate::i18n::UiText;
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
    /// Catalog text from `payload.text`; absent on rows recorded before events
    /// carried it, or when the stored value is not a valid `UiText`.
    pub text: Option<UiText>,
    /// Source-locale rendering from `payload.message`, kept for rows without `text`.
    pub message: Option<String>,
    pub payload: serde_json::Value,
    pub created_at: String,
}

impl EventResponse {
    pub(super) fn from_event(event: Event) -> Self {
        let text = event
            .payload
            .get("text")
            .and_then(|value| serde_json::from_value::<UiText>(value.clone()).ok());
        let message = event
            .payload
            .get("message")
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        Self {
            id: event.id.unwrap_or_default(),
            event_time: event.event_time.to_rfc3339(),
            category: event.category.to_string(),
            subtype: event.subtype,
            severity: event.severity.to_string(),
            mint: event.mint,
            reference_id: event.reference_id,
            text,
            message,
            payload: event.payload,
            created_at: event
                .created_at
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventCategory, Severity};
    use serde_json::json;

    fn response(payload: serde_json::Value) -> EventResponse {
        let event = Event::new(
            EventCategory::Trader,
            None,
            Severity::Info,
            None,
            None,
            payload,
        );
        EventResponse::from_event(event)
    }

    #[test]
    fn legacy_payload_has_message_and_no_text() {
        let event = response(json!({ "message": "Entry opportunity monitor started" }));
        assert!(event.text.is_none());
        assert_eq!(
            event.message.as_deref(),
            Some("Entry opportunity monitor started")
        );
    }

    #[test]
    fn new_payload_carries_text_and_message() {
        let event = response(json!({
            "text": { "id": "events-task-failed", "args": { "name": { "type": "text", "value": "A" } } },
            "message": "Task 'A' failed"
        }));
        let text = event.text.expect("text");
        assert_eq!(text.id, "events-task-failed");
        assert_eq!(event.message.as_deref(), Some("Task 'A' failed"));
    }

    #[test]
    fn malformed_text_is_dropped_and_missing_message_is_null() {
        let event = response(json!({ "text": "not an object" }));
        assert!(event.text.is_none());
        assert!(event.message.is_none());
        assert!(response(json!({})).message.is_none());
    }
}
