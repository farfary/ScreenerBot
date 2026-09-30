//! Stable identifiers for the steps of a trade action.
//!
//! Step names are persisted (`action_steps.name`, `actions.state_data`) and sent
//! over REST and SSE. They are codes; the dashboard maps each code to its label
//! from the catalog (`actions-step-<code>`).

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// One step of a trade action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStepCode {
    Evaluate,
    Validate,
    Quote,
    Swap,
    Verify,
    /// A stored name that matches neither a code nor a legacy English name.
    Unknown,
}

/// English names written before steps were codes. They remain in stored rows and
/// map to the same variants.
pub const LEGACY_STEP_NAMES: &[(&str, ActionStepCode)] = &[
    ("Evaluating", ActionStepCode::Evaluate),
    ("Validating", ActionStepCode::Validate),
    ("Getting Quote", ActionStepCode::Quote),
    ("Executing Swap", ActionStepCode::Swap),
    ("Verifying", ActionStepCode::Verify),
];

impl ActionStepCode {
    pub const ALL: [ActionStepCode; 6] = [
        Self::Evaluate,
        Self::Validate,
        Self::Quote,
        Self::Swap,
        Self::Verify,
        Self::Unknown,
    ];

    /// Serialized code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Evaluate => "evaluate",
            Self::Validate => "validate",
            Self::Quote => "quote",
            Self::Swap => "swap",
            Self::Verify => "verify",
            Self::Unknown => "unknown",
        }
    }

    /// Catalog id of the step label.
    pub const fn message_key(self) -> &'static str {
        match self {
            Self::Evaluate => "actions-step-evaluate",
            Self::Validate => "actions-step-validate",
            Self::Quote => "actions-step-quote",
            Self::Swap => "actions-step-swap",
            Self::Verify => "actions-step-verify",
            Self::Unknown => "actions-step-unknown",
        }
    }

    /// Resolve a stored or received name: a code, a legacy English name, or
    /// `Unknown`. Never fails, so an old row always loads.
    pub fn from_stored(name: &str) -> Self {
        if let Some(code) = Self::ALL.iter().copied().find(|code| code.as_str() == name) {
            return code;
        }
        LEGACY_STEP_NAMES
            .iter()
            .find(|(legacy, _)| *legacy == name)
            .map(|(_, code)| *code)
            .unwrap_or(Self::Unknown)
    }
}

impl Serialize for ActionStepCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ActionStepCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(Self::from_stored(&name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::ActionState;

    #[test]
    fn every_code_has_an_english_label() {
        for code in ActionStepCode::ALL {
            let key = code.message_key();
            assert_eq!(
                key,
                format!("actions-step-{}", code.as_str().replace('_', "-")),
                "key does not follow the serialized code"
            );
            assert_ne!(crate::i18n::format_en(key, None), key, "missing {key}");
            let short = format!("{key}-short");
            assert_ne!(
                crate::i18n::format_en(&short, None),
                short,
                "missing {short}"
            );
        }
    }

    #[test]
    fn all_lists_every_variant() {
        for code in ActionStepCode::ALL {
            match code {
                ActionStepCode::Evaluate
                | ActionStepCode::Validate
                | ActionStepCode::Quote
                | ActionStepCode::Swap
                | ActionStepCode::Verify
                | ActionStepCode::Unknown => {}
            }
            assert_eq!(ActionStepCode::from_stored(code.as_str()), code);
        }
    }

    #[test]
    fn legacy_state_data_row_deserializes() {
        let row = r#"{"status":"in_progress","current_step":"Getting Quote","current_step_index":1,"total_steps":4,"progress_pct":25}"#;
        let state: ActionState = serde_json::from_str(row).unwrap();
        assert_eq!(
            state,
            ActionState::InProgress {
                current_step: ActionStepCode::Quote,
                current_step_index: 1,
                total_steps: 4,
                progress_pct: 25,
            }
        );
    }

    #[test]
    fn legacy_names_map_to_their_codes() {
        for (legacy, code) in LEGACY_STEP_NAMES {
            let parsed: ActionStepCode = serde_json::from_value(serde_json::json!(legacy)).unwrap();
            assert_eq!(parsed, *code);
            assert_eq!(ActionStepCode::from_stored(legacy), *code);
        }
    }

    #[test]
    fn unrecognised_name_never_fails_the_row() {
        assert_eq!(
            ActionStepCode::from_stored("Something Else"),
            ActionStepCode::Unknown
        );
    }

    #[test]
    fn new_rows_round_trip_with_the_same_json_shape() {
        let state = ActionState::InProgress {
            current_step: ActionStepCode::Swap,
            current_step_index: 2,
            total_steps: 4,
            progress_pct: 50,
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "status": "in_progress",
                "current_step": "swap",
                "current_step_index": 2,
                "total_steps": 4,
                "progress_pct": 50,
            })
        );
        let back: ActionState = serde_json::from_value(json).unwrap();
        assert_eq!(back, state);
    }
}
