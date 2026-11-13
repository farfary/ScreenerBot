// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Tests of the starter strategies: their definitions, their validation by the strategy
//! engine and their text in every registered locale.

use super::*;
use crate::strategies::engine::{EngineConfig, StrategyEngine};
use crate::strategies::types::Condition;

fn locale(code: &str) -> LanguageIdentifier {
    code.parse().unwrap()
}

fn english() -> LanguageIdentifier {
    locale(i18n::source_locale())
}

fn english_starters() -> Vec<Strategy> {
    starters(&english(), Utc::now()).unwrap()
}

/// The conditions of a starter in rule order: every starter is one AND branch of leaves.
fn conditions(strategy: &Strategy) -> Vec<&Condition> {
    assert_eq!(strategy.rules.operator, Some(LogicalOperator::And));
    strategy
        .rules
        .conditions
        .iter()
        .flatten()
        .map(|leaf| {
            leaf.condition
                .as_ref()
                .expect("every starter node is a leaf")
        })
        .collect()
}

fn value<'a>(condition: &'a Condition, name: &str) -> &'a Value {
    &condition.parameters[name].value
}

/// What the create route derives an id from: the name, lowercased, spaces as hyphens and
/// every other character that is not alphanumeric dropped.
fn route_id(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

#[test]
fn the_starters_are_three_entry_and_two_exit_strategies_switched_off_in_paper_mode() {
    let starters = english_starters();
    let shape: Vec<_> = starters
        .iter()
        .map(|s| {
            (
                s.id.as_str(),
                s.strategy_type,
                s.timeframe.as_str(),
                s.priority,
            )
        })
        .collect();
    assert_eq!(
        shape,
        [
            ("trend-pullback", StrategyType::Entry, "5m", 10),
            ("oversold-bounce", StrategyType::Entry, "15m", 40),
            ("trend-dip", StrategyType::Entry, "15m", 15),
            ("overbought-take", StrategyType::Exit, "5m", 10),
            ("trend-break", StrategyType::Exit, "15m", 20),
        ]
    );
    for starter in &starters {
        assert!(!starter.enabled, "{} starts switched off", starter.id);
        assert_eq!(starter.mode, TradingMode::Paper, "{}", starter.id);
        assert_eq!(
            starter.author, None,
            "{} is an ordinary user record",
            starter.id
        );
        assert_eq!(starter.version, 1);
        assert!(
            starter.entry_strategy_ids.is_empty(),
            "an exit starter manages every position"
        );
        assert_eq!(starter.rules_changed_at, starter.created_at);
    }
}

#[test]
fn every_starter_passes_the_validation_of_the_create_route() {
    let engine = StrategyEngine::new(EngineConfig::default());
    for starter in english_starters() {
        engine
            .validate_strategy(&starter)
            .unwrap_or_else(|e| panic!("{} fails validation: {e}", starter.id));
        Strategy::check_priority(starter.priority).unwrap();
    }
}

#[test]
fn a_starter_id_is_the_id_the_create_route_derives_from_its_english_name() {
    for starter in english_starters() {
        assert_eq!(starter.id, route_id(&starter.name));
    }
}

/// The thresholds the paper run measured, one recorded case per starter.
#[test]
fn the_starters_carry_the_rules_the_paper_run_tested() {
    let starters = english_starters();
    let by_id = |id: &str| starters.iter().find(|s| s.id == id).unwrap();

    let pullback = conditions(by_id("trend-pullback"));
    assert_eq!(
        pullback
            .iter()
            .map(|c| c.condition_type.as_str())
            .collect::<Vec<_>>(),
        ["PriceToMA", "Rsi", "BuyPressure"]
    );
    assert_eq!(value(pullback[0], "period"), &json!(30));
    assert_eq!(value(pullback[1], "threshold"), &json!(40));
    assert_eq!(value(pullback[2], "min_buy_share_pct"), &json!(52));

    let bounce = conditions(by_id("oversold-bounce"));
    assert_eq!(
        bounce
            .iter()
            .map(|c| c.condition_type.as_str())
            .collect::<Vec<_>>(),
        ["BollingerBands", "Rsi", "PullbackFromHigh"]
    );
    assert_eq!(value(bounce[1], "threshold"), &json!(32));
    assert_eq!(value(bounce[2], "min_drop_pct"), &json!(15));
    assert_eq!(value(bounce[2], "max_drop_pct"), &json!(50));

    let dip = conditions(by_id("trend-dip"));
    assert_eq!(value(dip[0], "period"), &json!(100));
    assert_eq!(value(dip[1], "min_drop_pct"), &json!(10));
    assert_eq!(value(dip[1], "max_drop_pct"), &json!(25));
    assert_eq!(value(dip[2], "threshold"), &json!(40));

    let take = conditions(by_id("overbought-take"));
    assert_eq!(
        take.iter()
            .map(|c| c.condition_type.as_str())
            .collect::<Vec<_>>(),
        ["PositionProfit", "Rsi"]
    );
    assert_eq!(value(take[0], "threshold_pct"), &json!(10));
    assert_eq!(value(take[1], "threshold"), &json!(78));

    let break_ = conditions(by_id("trend-break"));
    assert_eq!(
        break_
            .iter()
            .map(|c| c.condition_type.as_str())
            .collect::<Vec<_>>(),
        ["PositionHoldingTime", "PositionProfit", "PriceToMA"]
    );
    assert_eq!(value(break_[0], "hours"), &json!(3.0));
    assert_eq!(value(break_[1], "threshold_pct"), &json!(0));
    assert_eq!(value(break_[2], "distance"), &json!(3.0));
}

/// A starter condition reads like one the editor adds: every parameter of its schema is
/// present, with the schema default beside the value.
#[test]
fn every_starter_condition_lists_the_whole_schema_with_its_defaults() {
    let registry = ConditionRegistry::new();
    for starter in english_starters() {
        for condition in conditions(&starter) {
            let schema = registry
                .condition_at_defaults(&condition.condition_type)
                .expect("a starter condition is a registered type");
            let mut present: Vec<_> = condition.parameters.keys().collect();
            let mut expected: Vec<_> = schema.parameters.keys().collect();
            present.sort();
            expected.sort();
            assert_eq!(
                present, expected,
                "{} {}",
                starter.id, condition.condition_type
            );
            for (name, parameter) in &condition.parameters {
                assert_eq!(
                    parameter.default, schema.parameters[name].default,
                    "{} {} {name}",
                    starter.id, condition.condition_type
                );
            }
        }
    }
}

#[test]
fn a_condition_or_parameter_the_registry_lacks_is_a_defect_not_a_panic() {
    let registry = ConditionRegistry::new();
    let unknown_type = StarterCondition {
        condition_type: "NoSuchCondition",
        values: Vec::new(),
    };
    assert!(starter_leaf(&registry, &unknown_type).is_err());
    let unknown_parameter = StarterCondition {
        condition_type: "Rsi",
        values: vec![("no_such_parameter", json!(1))],
    };
    assert!(starter_leaf(&registry, &unknown_parameter).is_err());
}

#[test]
fn the_text_resolves_in_every_registered_locale() {
    let english = english_starters();
    for info in i18n::available_locales() {
        let localized = starters(&locale(&info.code), Utc::now())
            .unwrap_or_else(|e| panic!("{} has no starter text: {e}", info.code));
        let mut names = std::collections::HashSet::new();
        for (starter, source) in localized.iter().zip(&english) {
            assert_eq!(starter.id, source.id, "the id does not follow the language");
            assert_eq!(starter.rules.operator, source.rules.operator);
            assert!(
                !starter.name.trim().is_empty(),
                "{} {}",
                info.code,
                starter.id
            );
            let description = starter.description.as_deref().unwrap_or_default();
            assert!(
                !description.trim().is_empty(),
                "{} {}",
                info.code,
                starter.id
            );
            assert!(
                names.insert(starter.name.clone()),
                "{}: two starters are both named {}",
                info.code,
                starter.name
            );
            if info.code != i18n::source_locale() {
                assert_ne!(
                    starter.description, source.description,
                    "{} {} is not translated",
                    info.code, starter.id
                );
            }
        }
    }
}

#[test]
fn the_text_is_resolved_in_the_requested_language() {
    let german = starters(&locale("de"), Utc::now()).unwrap();
    let trend_break = german.iter().find(|s| s.id == "trend-break").unwrap();
    assert_eq!(trend_break.name, "Trendbruch");
    let english = english_starters();
    let trend_break = english.iter().find(|s| s.id == "trend-break").unwrap();
    assert_eq!(trend_break.name, "Trend Break");
}

#[test]
fn starter_text_waits_for_the_configured_language_while_config_is_being_saved() {
    use std::sync::{mpsc, RwLock};
    use std::time::Duration;

    let config =
        crate::config::CONFIG.get_or_init(|| RwLock::new(crate::config::Config::default()));
    let mut saving = config.write().unwrap();
    let previous = saving.gui.dashboard.interface.language.clone();
    saving.gui.dashboard.interface.language = "de".to_owned();
    let (started, starting) = mpsc::channel();
    let (finished, result) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        started.send(()).unwrap();
        finished.send(runtime.block_on(starter_locale())).unwrap();
    });

    starting.recv_timeout(Duration::from_secs(5)).unwrap();
    let premature = result.recv_timeout(Duration::from_millis(50));
    saving.gui.dashboard.interface.language = "fr".to_owned();
    drop(saving);
    reader.join().unwrap();
    config.write().unwrap().gui.dashboard.interface.language = previous;

    assert!(matches!(premature, Err(mpsc::RecvTimeoutError::Timeout)));
    assert_eq!(result.recv().unwrap().unwrap(), locale("fr"));
}
