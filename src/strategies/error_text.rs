// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Catalog text for strategy validation and evaluation errors.
//!
//! `Display` on [`Error`] stays English for logs. The dashboard receives the
//! [`UiText`] built here (`strategies-error-*` in `locales/en/strategies.ftl`).
//! The tokens a variant carries (`field`, `expected`, `indicator`, `data`,
//! `reason`) are a closed set of `&'static str` literals; each maps to a catalog
//! message and is rendered as a nested text. A token missing from its table
//! falls back to the raw token, and the source scan in the tests fails on it.

use super::types::StrategyType;
use super::Error;
use crate::i18n::{ids, MessageId, UiArg, UiText};

const FIELDS: &[(&str, MessageId)] = &[
    (
        "arm after profit",
        ids::STRATEGIES_ERROR_FIELD_ARM_AFTER_PROFIT,
    ),
    ("average volume", ids::STRATEGIES_ERROR_FIELD_AVERAGE_VOLUME),
    ("candle open", ids::STRATEGIES_ERROR_FIELD_CANDLE_OPEN),
    ("comparison", ids::STRATEGIES_ERROR_FIELD_COMPARISON),
    ("condition type", ids::STRATEGIES_ERROR_FIELD_CONDITION_TYPE),
    ("confirmation", ids::STRATEGIES_ERROR_FIELD_CONFIRMATION),
    ("count", ids::STRATEGIES_ERROR_FIELD_COUNT),
    ("current price", ids::STRATEGIES_ERROR_FIELD_CURRENT_PRICE),
    ("direction", ids::STRATEGIES_ERROR_FIELD_DIRECTION),
    ("distance", ids::STRATEGIES_ERROR_FIELD_DISTANCE),
    (
        "drop percentage",
        ids::STRATEGIES_ERROR_FIELD_DROP_PERCENTAGE,
    ),
    ("entry price", ids::STRATEGIES_ERROR_FIELD_ENTRY_PRICE),
    ("fast period", ids::STRATEGIES_ERROR_FIELD_FAST_PERIOD),
    ("hours", ids::STRATEGIES_ERROR_FIELD_HOURS),
    ("lookback", ids::STRATEGIES_ERROR_FIELD_LOOKBACK),
    (
        "minimum buy share",
        ids::STRATEGIES_ERROR_FIELD_MINIMUM_BUY_SHARE,
    ),
    ("minimum change", ids::STRATEGIES_ERROR_FIELD_MINIMUM_CHANGE),
    (
        "minimum transactions",
        ids::STRATEGIES_ERROR_FIELD_MINIMUM_TRANSACTIONS,
    ),
    (
        "moving average type",
        ids::STRATEGIES_ERROR_FIELD_MOVING_AVERAGE_TYPE,
    ),
    ("moving average", ids::STRATEGIES_ERROR_FIELD_MOVING_AVERAGE),
    ("multiplier", ids::STRATEGIES_ERROR_FIELD_MULTIPLIER),
    ("pattern", ids::STRATEGIES_ERROR_FIELD_PATTERN),
    ("peak price", ids::STRATEGIES_ERROR_FIELD_PEAK_PRICE),
    ("percentage", ids::STRATEGIES_ERROR_FIELD_PERCENTAGE),
    ("period", ids::STRATEGIES_ERROR_FIELD_PERIOD),
    ("position", ids::STRATEGIES_ERROR_FIELD_POSITION),
    (
        "reference candle close",
        ids::STRATEGIES_ERROR_FIELD_REFERENCE_CANDLE_CLOSE,
    ),
    ("slow period", ids::STRATEGIES_ERROR_FIELD_SLOW_PERIOD),
    ("threshold", ids::STRATEGIES_ERROR_FIELD_THRESHOLD),
    ("time unit", ids::STRATEGIES_ERROR_FIELD_TIME_UNIT),
    ("time value", ids::STRATEGIES_ERROR_FIELD_TIME_VALUE),
    ("timeframe", ids::STRATEGIES_ERROR_FIELD_TIMEFRAME),
    ("window", ids::STRATEGIES_ERROR_FIELD_WINDOW),
    ("within candles", ids::STRATEGIES_ERROR_FIELD_WITHIN_CANDLES),
];

/// Parameter ids whose spaced form is not their field label: a missing or mistyped parameter
/// is named by its id, a value out of range by its label, and both read the same text.
const PARAMETER_FIELDS: &[(&str, MessageId)] = &[
    (
        "arm_after_profit_pct",
        ids::STRATEGIES_ERROR_FIELD_ARM_AFTER_PROFIT,
    ),
    ("drop_pct", ids::STRATEGIES_ERROR_FIELD_DROP_PERCENTAGE),
    ("ma_type", ids::STRATEGIES_ERROR_FIELD_MOVING_AVERAGE_TYPE),
    (
        "min_buy_share_pct",
        ids::STRATEGIES_ERROR_FIELD_MINIMUM_BUY_SHARE,
    ),
    (
        "min_transactions",
        ids::STRATEGIES_ERROR_FIELD_MINIMUM_TRANSACTIONS,
    ),
    ("threshold_pct", ids::STRATEGIES_ERROR_FIELD_THRESHOLD),
];

const EXPECTED: &[(&str, MessageId)] = &[
    ("a boolean", ids::STRATEGIES_ERROR_EXPECTED_BOOLEAN),
    ("a number", ids::STRATEGIES_ERROR_EXPECTED_NUMBER),
    ("a string", ids::STRATEGIES_ERROR_EXPECTED_STRING),
];

const DATA: &[(&str, MessageId)] = &[
    ("current price", ids::STRATEGIES_ERROR_DATA_CURRENT_PRICE),
    ("liquidity data", ids::STRATEGIES_ERROR_DATA_LIQUIDITY_DATA),
    ("market data", ids::STRATEGIES_ERROR_DATA_MARKET_DATA),
    ("OHLCV data", ids::STRATEGIES_ERROR_DATA_OHLCV_DATA),
    ("peak price", ids::STRATEGIES_ERROR_DATA_PEAK_PRICE),
    ("position data", ids::STRATEGIES_ERROR_DATA_POSITION_DATA),
    (
        "transaction counts",
        ids::STRATEGIES_ERROR_DATA_TRANSACTION_COUNTS,
    ),
];

const INDICATORS: &[(&str, MessageId)] = &[
    (
        "consecutive candles",
        ids::STRATEGIES_ERROR_INDICATOR_CONSECUTIVE_CANDLES,
    ),
    (
        "moving average",
        ids::STRATEGIES_ERROR_INDICATOR_MOVING_AVERAGE,
    ),
    (
        "moving average cross",
        ids::STRATEGIES_ERROR_INDICATOR_MOVING_AVERAGE_CROSS,
    ),
    (
        "price breakout",
        ids::STRATEGIES_ERROR_INDICATOR_PRICE_BREAKOUT,
    ),
    (
        "price change lookback",
        ids::STRATEGIES_ERROR_INDICATOR_PRICE_CHANGE_LOOKBACK,
    ),
    ("rsi", ids::STRATEGIES_ERROR_INDICATOR_RSI),
    ("volume spike", ids::STRATEGIES_ERROR_INDICATOR_VOLUME_SPIKE),
];

const RULE_FAULTS: &[(&str, MessageId)] = &[
    (
        "a group needs at least one condition switched on",
        ids::STRATEGIES_ERROR_RULE_A_GROUP_NEEDS_AT_LEAST_ONE_CONDITION_SWITCHED_ON,
    ),
    (
        "branch node missing conditions",
        ids::STRATEGIES_ERROR_RULE_BRANCH_NODE_MISSING_CONDITIONS,
    ),
    (
        "branch node missing operator",
        ids::STRATEGIES_ERROR_RULE_BRANCH_NODE_MISSING_OPERATOR,
    ),
    (
        "branch node must have at least one child",
        ids::STRATEGIES_ERROR_RULE_BRANCH_NODE_MUST_HAVE_AT_LEAST_ONE_CHILD,
    ),
    (
        "invalid rule tree structure",
        ids::STRATEGIES_ERROR_RULE_INVALID_RULE_TREE_STRUCTURE,
    ),
    (
        "leaf node missing condition",
        ids::STRATEGIES_ERROR_RULE_LEAF_NODE_MISSING_CONDITION,
    ),
    (
        "NOT operator cannot have a switched-off child",
        ids::STRATEGIES_ERROR_RULE_NOT_OPERATOR_CANNOT_HAVE_A_SWITCHED_OFF_CHILD,
    ),
    (
        "NOT operator must have exactly one child",
        ids::STRATEGIES_ERROR_RULE_NOT_OPERATOR_MUST_HAVE_EXACTLY_ONE_CHILD,
    ),
    (
        "rules nest deeper than four levels",
        ids::STRATEGIES_ERROR_RULE_RULES_NEST_DEEPER_THAN_FOUR_LEVELS,
    ),
    (
        "the root group cannot be switched off",
        ids::STRATEGIES_ERROR_RULE_THE_ROOT_GROUP_CANNOT_BE_SWITCHED_OFF,
    ),
];

/// Nested text for a closed-set token, or the raw token when it is unmapped.
fn token(table: &[(&str, MessageId)], name: &str) -> UiArg {
    match table.iter().find(|(known, _)| *known == name) {
        Some((_, id)) => UiArg::Nested(Box::new(UiText::new(*id))),
        None => UiArg::Text(name.to_owned()),
    }
}

/// Nested text for a field, named either by its label (`time value`) or by its parameter id
/// (`time_value`, `drop_pct`).
fn field_text(name: &str) -> UiArg {
    match PARAMETER_FIELDS.iter().find(|(known, _)| *known == name) {
        Some((_, id)) => UiArg::Nested(Box::new(UiText::new(*id))),
        None => token(FIELDS, &name.replace('_', " ")),
    }
}

fn count(value: usize) -> UiArg {
    UiArg::Text(value.to_string())
}

fn seconds(value: i64) -> UiArg {
    UiArg::Text(value.to_string())
}

impl Error {
    /// Catalog message for this error, with typed arguments.
    pub fn ui_text(&self) -> UiText {
        match self {
            Error::MissingConditionParameter { field } => {
                UiText::new(ids::STRATEGIES_ERROR_MISSING_PARAMETER).arg("field", field_text(field))
            }
            Error::ConditionParameterType { field, expected } => {
                UiText::new(ids::STRATEGIES_ERROR_PARAMETER_TYPE)
                    .arg("field", field_text(field))
                    .arg("expected", token(EXPECTED, expected))
            }
            Error::InvalidConditionValue { field, value } => {
                UiText::new(ids::STRATEGIES_ERROR_INVALID_VALUE)
                    .arg("field", field_text(field))
                    .arg("value", UiArg::Text(value.clone()))
            }
            Error::MissingContextData { data } => {
                UiText::new(ids::STRATEGIES_ERROR_MISSING_DATA).arg("data", token(DATA, data))
            }
            Error::NoCandleData { timeframe } => UiText::new(ids::STRATEGIES_ERROR_NO_CANDLE_DATA)
                .arg("timeframe", UiArg::Text(timeframe.clone())),
            Error::InsufficientHistory {
                indicator,
                available_seconds,
                required_seconds,
            } => UiText::new(ids::STRATEGIES_ERROR_INSUFFICIENT_HISTORY)
                .arg("indicator", token(INDICATORS, indicator))
                .arg("available", seconds(*available_seconds))
                .arg("required", seconds(*required_seconds)),
            Error::InsufficientCandles {
                indicator,
                available,
                required,
            } => UiText::new(ids::STRATEGIES_ERROR_INSUFFICIENT_CANDLES)
                .arg("indicator", token(INDICATORS, indicator))
                .arg("available", count(*available))
                .arg("required", count(*required)),
            Error::StaleCandleData {
                timeframe,
                age_seconds,
                max_age_seconds,
            } => UiText::new(ids::STRATEGIES_ERROR_STALE_CANDLE_DATA)
                .arg("timeframe", UiArg::Text(timeframe.clone()))
                .arg("age", seconds(*age_seconds))
                .arg("max", seconds(*max_age_seconds)),
            Error::StaleMarketData {
                window,
                age_seconds,
                max_age_seconds,
            } => UiText::new(ids::STRATEGIES_ERROR_STALE_MARKET_DATA)
                .arg("window", UiArg::Text(window.clone()))
                .arg("age", seconds(*age_seconds))
                .arg("max", seconds(*max_age_seconds)),
            Error::UnusableData { field, value } => {
                UiText::new(ids::STRATEGIES_ERROR_UNUSABLE_DATA)
                    .arg("field", field_text(field))
                    .arg("value", UiArg::Text(value.clone()))
            }
            Error::InvalidRuleTree { reason } => {
                UiText::new(ids::STRATEGIES_ERROR_INVALID_RULE_TREE)
                    .arg("reason", token(RULE_FAULTS, reason))
            }
            Error::ConditionNotApplicable {
                condition_type,
                strategy_type,
            } => UiText::new(ids::STRATEGIES_ERROR_CONDITION_NOT_APPLICABLE)
                .arg("condition", UiArg::Text(condition_type.clone()))
                .arg(
                    "strategy",
                    UiArg::Nested(Box::new(UiText::new(match strategy_type {
                        StrategyType::Entry => ids::STRATEGIES_TYPE_ENTRY,
                        StrategyType::Exit => ids::STRATEGIES_TYPE_EXIT,
                    }))),
                ),
            Error::EvaluationTimeout { timeout_ms } => {
                UiText::new(ids::STRATEGIES_ERROR_EVALUATION_TIMEOUT)
                    .arg("timeout", UiArg::Text(timeout_ms.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::ErrorClass;
    use crate::i18n::{format_en, LanguageIdentifier};
    use std::path::Path;

    /// One instance of every variant, built through an exhaustive match so a new
    /// variant fails to compile until it is added here and to the catalog.
    fn all() -> Vec<Error> {
        let listed = |error: Error| match error {
            Error::MissingConditionParameter { .. }
            | Error::ConditionParameterType { .. }
            | Error::InvalidConditionValue { .. }
            | Error::MissingContextData { .. }
            | Error::NoCandleData { .. }
            | Error::InsufficientHistory { .. }
            | Error::InsufficientCandles { .. }
            | Error::StaleCandleData { .. }
            | Error::StaleMarketData { .. }
            | Error::UnusableData { .. }
            | Error::InvalidRuleTree { .. }
            | Error::ConditionNotApplicable { .. }
            | Error::EvaluationTimeout { .. } => error,
        };
        vec![
            listed(Error::MissingConditionParameter { field: "period" }),
            listed(Error::ConditionParameterType {
                field: "period",
                expected: "a number",
            }),
            listed(Error::InvalidConditionValue {
                field: "direction",
                value: "sideways".to_owned(),
            }),
            listed(Error::MissingContextData {
                data: "market data",
            }),
            listed(Error::NoCandleData {
                timeframe: "5m".to_owned(),
            }),
            listed(Error::InsufficientHistory {
                indicator: "volume spike",
                available_seconds: 60,
                required_seconds: 300,
            }),
            listed(Error::InsufficientCandles {
                indicator: "moving average",
                available: 3,
                required: 20,
            }),
            listed(Error::StaleCandleData {
                timeframe: "1m".to_owned(),
                age_seconds: 400,
                max_age_seconds: 120,
            }),
            listed(Error::StaleMarketData {
                window: "1h".to_owned(),
                age_seconds: 4000,
                max_age_seconds: 3600,
            }),
            listed(Error::UnusableData {
                field: "moving average",
                value: "0".to_owned(),
            }),
            listed(Error::InvalidRuleTree {
                reason: "leaf node missing condition",
            }),
            listed(Error::ConditionNotApplicable {
                condition_type: "PositionProfit".to_owned(),
                strategy_type: StrategyType::Entry,
            }),
            listed(Error::EvaluationTimeout { timeout_ms: 250 }),
        ]
    }

    fn exists(id: &str) -> bool {
        format_en(id, None) != id
    }

    #[test]
    fn every_error_has_catalog_text_with_resolved_tokens() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        for error in all() {
            let text = error.ui_text();
            assert!(exists(&text.id), "missing {}", text.id);
            let rendered = text.render_plain(&en);
            assert!(!rendered.is_empty());
            assert!(!rendered.contains("strategies-error-"), "{rendered}");
        }
    }

    /// A rule tree reads an error as "this condition is unknown" exactly when the error
    /// is the failed-dependency class the dashboard is told about as 424.
    #[test]
    fn data_unavailable_is_exactly_the_failed_dependency_class() {
        for error in all() {
            assert_eq!(
                error.is_data_unavailable(),
                error.http_status() == 424,
                "{error:?}"
            );
        }
    }

    #[test]
    fn rendered_text_matches_the_display_wording() {
        let en: LanguageIdentifier = "en".parse().unwrap();
        let error = Error::InsufficientCandles {
            indicator: "moving average",
            available: 3,
            required: 20,
        };
        assert_eq!(
            error.ui_text().render_plain(&en),
            "Not enough candles for moving average: have 3, need 20"
        );
    }

    /// Every parameter of every condition schema names translated text when it is missing,
    /// mistyped or out of range, so no message shows a raw parameter id.
    #[test]
    fn every_schema_parameter_names_a_field() {
        let schemas = crate::strategies::conditions::ConditionRegistry::new().get_all_schemas();
        let mut checked = 0;
        for (condition, schema) in schemas.as_object().unwrap() {
            for parameter in schema["parameters"].as_object().unwrap().keys() {
                assert!(
                    matches!(field_text(parameter), UiArg::Nested(_)),
                    "{condition}.{parameter} has no field text"
                );
                checked += 1;
            }
        }
        assert!(checked > 40, "only {checked} parameters checked");
    }

    #[test]
    fn every_table_entry_exists_in_the_catalog() {
        for table in [
            FIELDS,
            PARAMETER_FIELDS,
            EXPECTED,
            DATA,
            INDICATORS,
            RULE_FAULTS,
        ] {
            for (name, id) in table {
                assert!(exists(id.as_str()), "{name}: missing {id}");
            }
        }
    }

    fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    /// Every token literal the strategies module puts into an error variant is
    /// in its table, so no validation message shows an untranslated token.
    #[test]
    fn every_token_literal_in_the_module_is_mapped() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/strategies");
        let mut files = Vec::new();
        rust_files(&root, &mut files);
        let tables: [(&str, &[(&str, MessageId)]); 5] = [
            ("field", FIELDS),
            ("expected", EXPECTED),
            ("data", DATA),
            ("indicator", INDICATORS),
            ("reason", RULE_FAULTS),
        ];
        let mut seen = 0;
        let basis_needle = format!("usable_basis({}", '"');
        for file in files
            .iter()
            .filter(|f| !f.ends_with("error_text.rs") && !f.ends_with("database.rs"))
        {
            let source = std::fs::read_to_string(file).unwrap();
            for (at, _) in source.match_indices(&basis_needle) {
                let rest = &source[at + basis_needle.len()..];
                let literal = &rest[..rest.find('"').unwrap()];
                seen += 1;
                assert!(
                    FIELDS.iter().any(|(name, _)| *name == literal),
                    "{}: unmapped basis label {literal:?}",
                    file.display()
                );
            }
            for (key, table) in tables {
                let needle = format!("{key}: {}", '"');
                for (at, _) in source.match_indices(&needle) {
                    let rest = &source[at + needle.len()..];
                    let literal = &rest[..rest.find('"').unwrap()];
                    seen += 1;
                    assert!(
                        table.iter().any(|(name, _)| *name == literal),
                        "{}: unmapped {key} token {literal:?}",
                        file.display()
                    );
                }
            }
        }
        assert!(seen > 30, "source scan found only {seen} tokens");
    }
}
