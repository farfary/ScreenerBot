// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The reason an action or one of its steps failed.
//!
//! A failure is catalog text plus optional technical details. `text` names the
//! operation that failed and is localized by the viewer; `details` holds only
//! technical values such as an underlying error string and is never translated.
//! It serializes as `{ "text": UiText, "details": string | null }`, the same
//! shape an API error uses, so the dashboard renders it with one function.
//!
//! Rows written before failures carried text hold a plain string in
//! `actions.state_data` and `action_steps.error`. They deserialize into the
//! passthrough message `actions-failure-recorded`, so an old row displays
//! exactly the string that was stored and no row can fail to load.

use crate::i18n::{ids, MessageId, UiArg, UiText};
use serde::{Deserialize, Deserializer, Serialize};

/// Catalog text and technical details of a failed action or step.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionFailure {
    pub text: UiText,
    pub details: Option<String>,
}

impl ActionFailure {
    /// Failure described by `id` alone.
    pub fn new(id: MessageId) -> Self {
        Self {
            text: UiText::new(id),
            details: None,
        }
    }

    /// Failure described by `id` with the technical cause in `details`. An empty
    /// cause is dropped.
    pub fn with_details(id: MessageId, details: impl Into<String>) -> Self {
        let details = details.into();
        Self {
            text: UiText::new(id),
            details: (!details.is_empty()).then_some(details),
        }
    }

    /// A failure recorded as a plain string before failures carried catalog text.
    pub fn legacy(message: String) -> Self {
        Self {
            text: UiText::new(ids::ACTIONS_FAILURE_RECORDED).arg("message", UiArg::Text(message)),
            details: None,
        }
    }

    /// Read the `action_steps.error` column: the serialized failure, or a plain
    /// string written before failures were structured. Never fails.
    pub fn from_stored(raw: &str) -> Self {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(value @ serde_json::Value::Object(_)) => {
                serde_json::from_value(value).unwrap_or_else(|_| Self::legacy(raw.to_owned()))
            }
            _ => Self::legacy(raw.to_owned()),
        }
    }

    /// The serialized form written to `action_steps.error`.
    pub fn to_stored(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| self.log_text())
    }

    /// Source-locale plain text with the details appended, for logs.
    pub fn log_text(&self) -> String {
        let text = self.text.render_source_plain();
        match &self.details {
            Some(details) => format!("{text}: {details}"),
            None => text,
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Stored {
    Legacy(String),
    Current {
        text: UiText,
        #[serde(default)]
        details: Option<String>,
    },
}

impl<'de> Deserialize<'de> for ActionFailure {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Stored::deserialize(deserializer)? {
            Stored::Legacy(message) => Self::legacy(message),
            Stored::Current { text, details } => Self { text, details },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ActionState;

    #[test]
    fn legacy_state_data_row_keeps_its_string() {
        let row =
            r#"{"status":"failed","error":"Verification expired: the transaction never landed"}"#;
        let state: ActionState = serde_json::from_str(row).unwrap();
        let ActionState::Failed { error } = state else {
            panic!("expected a failed state");
        };
        assert_eq!(
            error.text.render_source_plain(),
            "Verification expired: the transaction never landed"
        );
        assert_eq!(error.details, None);
    }

    #[test]
    fn legacy_step_error_column_keeps_its_string() {
        for raw in ["Quote failed", "{not json", "42", "\"quoted\"", ""] {
            let failure = ActionFailure::from_stored(raw);
            assert_eq!(failure.text.render_source_plain(), raw);
            assert_eq!(failure.details, None);
        }
    }

    #[test]
    fn new_failure_round_trips_through_state_and_column() {
        let failure = ActionFailure::with_details(ids::ACTIONS_FAILURE_EXIT, "rpc timeout");
        let state = ActionState::Failed {
            error: failure.clone(),
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["error"]["text"]["id"], "actions-failure-exit");
        assert_eq!(json["error"]["details"], "rpc timeout");
        assert_eq!(serde_json::from_value::<ActionState>(json).unwrap(), state);
        assert_eq!(ActionFailure::from_stored(&failure.to_stored()), failure);
    }

    #[test]
    fn failure_has_the_api_error_wire_shape() {
        let json = serde_json::to_value(ActionFailure::new(ids::ACTIONS_FAILURE_ENTRY)).unwrap();
        assert!(json["text"].is_object());
        assert!(json["details"].is_null());
    }

    #[test]
    fn empty_cause_is_not_recorded_as_details() {
        assert_eq!(
            ActionFailure::with_details(ids::ACTIONS_FAILURE_ENTRY, "").details,
            None
        );
    }
}
