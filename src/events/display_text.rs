// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Display text of event payloads.
//!
//! `payload.text` carries the catalog `UiText` the dashboard localizes;
//! `payload.message` carries its source-locale rendering, which stays in the
//! payload because event search matches the stored JSON and agent tools read it
//! as plain text. `message` is always derived from `text` through [`write_text`].

use crate::events::{EventCategory, Severity};
use crate::i18n::{ids, UiArg, UiText};
use crate::logger::{self, LogTag};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

/// Write `text` and its source-locale rendering into a payload object.
pub(super) fn write_text(payload: &mut Map<String, Value>, text: &UiText) {
    payload.insert(
        "text".to_owned(),
        serde_json::to_value(text).unwrap_or(Value::Null),
    );
    payload.insert(
        "message".to_owned(),
        Value::String(text.render_source_plain()),
    );
}

/// Categories that already logged a stray-`message` warning.
static STRAY_MESSAGE_WARNED: LazyLock<Mutex<HashSet<EventCategory>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Write the default text for a payload that carries no `text`.
///
/// A `message` without `text` is a producer that bypassed [`with_text`]. It is
/// overwritten with the default text so `message` always derives from `text`,
/// and the first occurrence per category is logged.
pub(super) fn write_default_text(
    category: &EventCategory,
    payload: &mut Map<String, Value>,
    default: impl FnOnce() -> UiText,
) {
    if payload.contains_key("text") {
        return;
    }
    if payload.contains_key("message") {
        let first = STRAY_MESSAGE_WARNED
            .lock()
            .map(|mut warned| warned.insert(category.clone()))
            .unwrap_or(false);
        if first {
            logger::warning(
                LogTag::System,
                &format!(
                    "Event payload in category '{}' carried a message without text; replaced with the default text",
                    category.to_string()
                ),
            );
        }
    }
    write_text(payload, &default());
}

/// Attach display text to a recorder payload. The result is an object; a
/// non-object payload is kept under `details`, as the recorders do.
pub fn with_text(payload: Value, text: &UiText) -> Value {
    let mut object = match payload {
        Value::Object(object) => object,
        Value::Null => Map::new(),
        other => Map::from_iter([("details".to_owned(), other)]),
    };
    write_text(&mut object, text);
    Value::Object(object)
}

/// Outcome of a scheduled task run, recorded as the event subtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ScheduledTaskOutcome {
    Completed,
    Failed,
    TimedOut,
}

impl ScheduledTaskOutcome {
    /// Stable subtype code stored on the event row.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Completed => "task_completed",
            Self::Failed => "task_failed",
            Self::TimedOut => "task_timed_out",
        }
    }

    pub const fn severity(self) -> Severity {
        match self {
            Self::Completed => Severity::Info,
            Self::Failed | Self::TimedOut => Severity::Warn,
        }
    }

    pub fn title(self, task_name: &str) -> UiText {
        let id = match self {
            Self::Completed => ids::EVENTS_TASK_COMPLETED,
            Self::Failed => ids::EVENTS_TASK_FAILED,
            Self::TimedOut => ids::EVENTS_TASK_TIMED_OUT,
        };
        UiText::new(id).arg("name", UiArg::Text(task_name.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::format_en;
    use serde_json::json;

    fn outcomes() -> [ScheduledTaskOutcome; 3] {
        let listed = |outcome: ScheduledTaskOutcome| match outcome {
            ScheduledTaskOutcome::Completed
            | ScheduledTaskOutcome::Failed
            | ScheduledTaskOutcome::TimedOut => outcome,
        };
        [
            listed(ScheduledTaskOutcome::Completed),
            listed(ScheduledTaskOutcome::Failed),
            listed(ScheduledTaskOutcome::TimedOut),
        ]
    }

    #[test]
    fn message_is_always_the_rendering_of_text() {
        let text = UiText::new(ids::EVENTS_RPC_DEFAULT)
            .arg("method", UiArg::Text("getSlot".to_owned()))
            .arg("action", UiArg::Text("retry".to_owned()));
        let payload = with_text(json!({ "message": "stale", "kept": 1 }), &text);
        assert_eq!(payload["message"], "RPC getSlot - retry");
        assert_eq!(payload["message"], text.render_source_plain().as_str());
        assert_eq!(payload["text"]["id"], "events-rpc-default");
        assert_eq!(payload["kept"], 1);
    }

    #[test]
    fn default_text_overwrites_a_stray_message() {
        let subtype =
            || UiText::new(ids::EVENTS_TRADER_DEFAULT).arg("subtype", UiArg::Text("x".into()));

        let mut stray = Map::from_iter([("message".to_owned(), json!("from producer"))]);
        write_default_text(&EventCategory::Trader, &mut stray, subtype);
        assert_eq!(stray["message"], "Trader event: x");
        assert_eq!(stray["text"]["id"], "events-trader-default");

        let mut empty = Map::new();
        write_default_text(&EventCategory::Trader, &mut empty, subtype);
        assert_eq!(empty["message"], "Trader event: x");
        assert_eq!(empty["text"]["id"], "events-trader-default");
    }

    #[test]
    fn default_text_keeps_text_supplied_by_the_producer() {
        let supplied = with_text(json!({}), &UiText::new(ids::EVENTS_MESSAGE_NONE));
        let Value::Object(mut payload) = supplied else {
            panic!("with_text returns an object");
        };
        write_default_text(&EventCategory::Ohlcv, &mut payload, || {
            UiText::new(ids::EVENTS_OHLCV_DEFAULT).arg("subtype", UiArg::Text("x".into()))
        });
        assert_eq!(payload["text"]["id"], "events-message-none");
        assert_eq!(payload["message"], "No message");
    }

    #[test]
    fn non_object_payload_is_kept_under_details() {
        let text = UiText::new(ids::EVENTS_MESSAGE_NONE);
        let payload = with_text(json!([1, 2]), &text);
        assert_eq!(payload["details"], json!([1, 2]));
        assert_eq!(payload["message"], "No message");
    }

    #[test]
    fn every_outcome_has_code_title_and_subtype_label() {
        for outcome in outcomes() {
            let dashed = outcome.code().replace('_', "-");
            let title = outcome.title("Nightly");
            assert_eq!(title.id, format!("events-{dashed}"));
            assert!(title.render_source_plain().contains("Task 'Nightly' "));
            let label = format!("events-subtype-{dashed}");
            assert_ne!(format_en(&label, None), label, "missing {label}");
        }
        assert_eq!(
            ScheduledTaskOutcome::TimedOut
                .title("A")
                .render_source_plain(),
            "Task 'A' timed out"
        );
    }

    #[test]
    fn default_ids_exist_in_the_catalog() {
        for id in [
            ids::EVENTS_OHLCV_DEFAULT,
            ids::EVENTS_FILTERING_DEFAULT,
            ids::EVENTS_TRADER_DEFAULT,
            ids::EVENTS_RPC_DEFAULT,
            ids::EVENTS_API_DEFAULT,
            ids::EVENTS_MESSAGE_NONE,
        ] {
            assert_ne!(format_en(id.as_str(), None), id.as_str(), "missing {id}");
        }
    }
}
