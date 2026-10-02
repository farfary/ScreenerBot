// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Localization keys of the condition schemas served to the strategy editor.
//!
//! A schema carries structure only. Every name, description and option label is a
//! message in `locales/en/strategies.ftl`, addressed by the `key` fields that
//! [`attach_text_keys`] adds. The dashboard reads them through
//! `pages/strategies/condition_text.js`; every key lives in the
//! `strategies-condition-` namespace that the test below keeps complete.

use serde_json::{json, Value};

/// Namespace of every message a schema can reference.
const PREFIX: &str = "strategies-condition-";

/// Parameter whose name, description and options are shared by every condition.
const TIMEFRAME_PARAM: &str = "timeframe";

/// Lowercase kebab form of a type or parameter id, splitting `CamelCase` and `snake_case`
/// (`PriceToMA` -> `price-to-ma`, `time_value` -> `time-value`).
fn kebab(id: &str) -> String {
    let chars: Vec<char> = id.chars().collect();
    let mut out = String::with_capacity(id.len() + 4);
    for (index, &ch) in chars.iter().enumerate() {
        if ch == '_' || ch == ' ' {
            out.push('-');
            continue;
        }
        if ch.is_uppercase() && index > 0 {
            let prev = chars[index - 1];
            let next_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
            if prev.is_lowercase() || prev.is_ascii_digit() || (prev.is_uppercase() && next_lower) {
                out.push('-');
            }
        }
        out.extend(ch.to_lowercase());
    }
    out
}

/// Slug of a category id (`Position & Performance` -> `position-performance`).
fn slug(id: &str) -> String {
    id.to_lowercase()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn condition_key(condition_type: &str) -> String {
    format!("{PREFIX}{}", kebab(condition_type))
}

fn category_key(category: &str) -> String {
    format!("{PREFIX}category-{}", slug(category))
}

fn param_key(condition_type: &str, param: &str) -> String {
    if param == TIMEFRAME_PARAM {
        return format!("{PREFIX}param-{TIMEFRAME_PARAM}");
    }
    format!("{}-param-{}", condition_key(condition_type), kebab(param))
}

fn option_key(condition_type: &str, param: &str, value: &str) -> String {
    if param == TIMEFRAME_PARAM {
        return format!("{PREFIX}{TIMEFRAME_PARAM}-option-{}", kebab(value));
    }
    format!(
        "{}-option-{}",
        param_key(condition_type, param),
        kebab(value)
    )
}

/// Add the message key of the condition, its category, each parameter and each enum option.
pub fn attach_text_keys(schema: &mut Value) {
    let Some(condition_type) = schema
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_owned)
    else {
        return;
    };
    schema["key"] = json!(condition_key(&condition_type));
    if let Some(category) = schema.get("category").and_then(Value::as_str) {
        schema["category_key"] = json!(category_key(category));
    }
    let Some(params) = schema.get_mut("parameters").and_then(Value::as_object_mut) else {
        return;
    };
    for (name, spec) in params.iter_mut() {
        spec["key"] = json!(param_key(&condition_type, name));
        let Some(options) = spec.get_mut("options").and_then(Value::as_array_mut) else {
            continue;
        };
        for option in options {
            if let Some(value) = option.get("value").and_then(Value::as_str) {
                option["key"] = json!(option_key(&condition_type, name, value));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::{format_message, source_message_ids, LanguageIdentifier};
    use crate::strategies::conditions::ConditionRegistry;
    use std::collections::BTreeSet;

    fn english() -> LanguageIdentifier {
        "en".parse().expect("valid language tag")
    }

    /// Every message key a schema can reference, with whether it must carry `.description`.
    fn referenced_keys() -> BTreeSet<(String, bool)> {
        let mut keys = BTreeSet::new();
        let schemas = ConditionRegistry::new().get_all_schemas();
        for schema in schemas.as_object().expect("schemas object").values() {
            let key = schema["key"].as_str().expect("condition key");
            keys.insert((key.to_owned(), true));
            let category = schema["category_key"].as_str().expect("category key");
            keys.insert((category.to_owned(), false));
            for spec in schema["parameters"]
                .as_object()
                .expect("parameters")
                .values()
            {
                keys.insert((spec["key"].as_str().expect("param key").to_owned(), true));
                for option in spec["options"].as_array().into_iter().flatten() {
                    let key = option["key"].as_str().expect("option key");
                    keys.insert((key.to_owned(), false));
                }
            }
        }
        keys
    }

    /// Guarantee behind the dashboard's dynamic `strategies-condition-` lookups: every key a
    /// schema emits exists in `en` (with a description where the editor shows one), and the
    /// namespace holds nothing else.
    #[test]
    fn strategies_catalog_covers_conditions() {
        let keys = referenced_keys();
        assert!(keys.len() > 20, "schemas produced no keys");
        for (key, described) in &keys {
            let message = format_message(&english(), key, None)
                .unwrap_or_else(|| panic!("`{key}` is missing from locales/en/strategies.ftl"));
            assert!(
                message
                    .value
                    .as_deref()
                    .is_some_and(|value| !value.is_empty()),
                "`{key}` has no value"
            );
            if *described {
                assert!(
                    message
                        .attributes
                        .iter()
                        .any(|(name, _)| name == "description"),
                    "`{key}` has no `.description`"
                );
            }
            for (name, _) in &message.attributes {
                assert_eq!(
                    name, "description",
                    "`{key}` has unsupported attribute `.{name}`"
                );
            }
        }

        let used: BTreeSet<&str> = keys.iter().map(|(key, _)| key.as_str()).collect();
        let orphans: Vec<&str> = source_message_ids()
            .iter()
            .copied()
            .filter(|id| id.starts_with(PREFIX) && !used.contains(*id))
            .collect();
        assert!(
            orphans.is_empty(),
            "condition messages no schema uses: {orphans:?}"
        );
    }

    /// The timeframe options offered are the ones the evaluators accept.
    #[test]
    fn timeframe_options_match_validation() {
        let schemas = ConditionRegistry::new().get_all_schemas();
        for schema in schemas.as_object().expect("schemas object").values() {
            let Some(spec) = schema["parameters"].get(TIMEFRAME_PARAM) else {
                continue;
            };
            let offered: Vec<&str> = spec["options"]
                .as_array()
                .expect("timeframe options")
                .iter()
                .filter_map(|option| option["value"].as_str())
                .collect();
            assert_eq!(offered, ["1m", "5m", "15m", "1h", "4h", "12h", "1d"]);
        }
    }

    #[test]
    fn kebab_splits_type_and_parameter_ids() {
        assert_eq!(kebab("PriceToMA"), "price-to-ma");
        assert_eq!(kebab("PriceChangePercent"), "price-change-percent");
        assert_eq!(kebab("time_value"), "time-value");
        assert_eq!(kebab("LONG_UPPER_WICK"), "long-upper-wick");
        assert_eq!(slug("Position & Performance"), "position-performance");
    }
}
