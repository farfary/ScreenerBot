//! Display text of event payloads.
//!
//! `payload.text` carries the catalog `UiText` the dashboard localizes;
//! `payload.message` carries its source-locale rendering, which stays in the
//! payload because event search matches the stored JSON and agent tools read it
//! as plain text. `message` is always derived from `text` through [`write_text`].

use crate::events::Severity;
use crate::i18n::{ids, UiArg, UiText};
use serde_json::{Map, Value};

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

/// Write the default text unless the producer already supplied a `message`.
pub(super) fn write_default_text(
    payload: &mut Map<String, Value>,
    default: impl FnOnce() -> UiText,
) {
    if !payload.get("message").is_some_and(Value::is_string) {
        write_text(payload, &default());
    }
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    fn default_text_yields_to_a_producer_message() {
        let mut supplied = Map::from_iter([("message".to_owned(), json!("from producer"))]);
        write_default_text(&mut supplied, || UiText::new(ids::EVENTS_MESSAGE_NONE));
        assert_eq!(supplied["message"], "from producer");
        assert!(!supplied.contains_key("text"));

        let mut empty = Map::new();
        let subtype =
            || UiText::new(ids::EVENTS_TRADER_DEFAULT).arg("subtype", UiArg::Text("x".into()));
        write_default_text(&mut empty, subtype);
        assert_eq!(empty["message"], "Trader event: x");
        assert_eq!(empty["text"]["id"], "events-trader-default");
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
