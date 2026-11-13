// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Starter strategies: the five switched-off paper strategies a new install begins with.

use crate::errors::{Error, InternalError};
use crate::i18n::{self, ids, LanguageIdentifier, MessageId};
use crate::logger::{self, LogTag};
use crate::strategies::conditions::ConditionRegistry;
use crate::strategies::types::{LogicalOperator, RuleTree, Strategy, StrategyType};
use crate::strategies::{self, db};
use crate::trader::paper::TradingMode;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;

/// The Fluent attribute that carries a starter's description.
const DESCRIPTION_ATTRIBUTE: &str = "description";

/// One condition of a starter: its type and the parameters set away from the schema default.
struct StarterCondition {
    condition_type: &'static str,
    values: Vec<(&'static str, Value)>,
}

/// The definition of one starter strategy. Its rules are the tree every starter ANDs
/// together, which the paper run recorded as winning.
struct StarterSpec {
    /// Fixed ASCII id, equal to the id the create route derives from the English name, so
    /// it does not vary with the language the starter text is resolved in.
    id: &'static str,
    /// The catalog message whose value is the name and whose `.description` is the
    /// description.
    text: MessageId,
    strategy_type: StrategyType,
    timeframe: &'static str,
    priority: i32,
    conditions: Vec<StarterCondition>,
}

fn condition(condition_type: &'static str, values: Vec<(&'static str, Value)>) -> StarterCondition {
    StarterCondition {
        condition_type,
        values,
    }
}

/// The five starters: three entry strategies and two exit strategies.
fn specs() -> Vec<StarterSpec> {
    vec![
        StarterSpec {
            id: "trend-pullback",
            text: ids::STRATEGIES_STARTER_TREND_PULLBACK,
            strategy_type: StrategyType::Entry,
            timeframe: "5m",
            priority: 10,
            conditions: vec![
                condition(
                    "PriceToMA",
                    vec![
                        ("position", json!("ABOVE")),
                        ("period", json!(30)),
                        ("distance", json!(0.1)),
                        ("timeframe", json!("15m")),
                    ],
                ),
                condition(
                    "Rsi",
                    vec![
                        ("comparison", json!("BELOW")),
                        ("threshold", json!(40)),
                        ("period", json!(14)),
                        ("timeframe", json!("5m")),
                    ],
                ),
                condition(
                    "BuyPressure",
                    vec![
                        ("window", json!("1h")),
                        ("min_buy_share_pct", json!(52)),
                        ("min_transactions", json!(100)),
                    ],
                ),
            ],
        },
        StarterSpec {
            id: "oversold-bounce",
            text: ids::STRATEGIES_STARTER_OVERSOLD_BOUNCE,
            strategy_type: StrategyType::Entry,
            timeframe: "15m",
            priority: 40,
            conditions: vec![
                condition(
                    "BollingerBands",
                    vec![
                        ("signal", json!("BELOW_LOWER")),
                        ("period", json!(20)),
                        ("std_dev", json!(2)),
                        ("timeframe", json!("15m")),
                    ],
                ),
                condition(
                    "Rsi",
                    vec![
                        ("comparison", json!("BELOW")),
                        ("threshold", json!(32)),
                        ("period", json!(14)),
                        ("timeframe", json!("15m")),
                    ],
                ),
                condition(
                    "PullbackFromHigh",
                    vec![
                        ("lookback", json!(32)),
                        ("min_drop_pct", json!(15)),
                        ("max_drop_pct", json!(50)),
                        ("timeframe", json!("15m")),
                    ],
                ),
            ],
        },
        StarterSpec {
            id: "trend-dip",
            text: ids::STRATEGIES_STARTER_TREND_DIP,
            strategy_type: StrategyType::Entry,
            timeframe: "15m",
            priority: 15,
            conditions: vec![
                condition(
                    "PriceToMA",
                    vec![
                        ("position", json!("ABOVE")),
                        ("period", json!(100)),
                        ("distance", json!(0.1)),
                        ("timeframe", json!("15m")),
                    ],
                ),
                condition(
                    "PullbackFromHigh",
                    vec![
                        ("lookback", json!(32)),
                        ("min_drop_pct", json!(10)),
                        ("max_drop_pct", json!(25)),
                        ("timeframe", json!("15m")),
                    ],
                ),
                condition(
                    "Rsi",
                    vec![
                        ("comparison", json!("BELOW")),
                        ("threshold", json!(40)),
                        ("period", json!(14)),
                        ("timeframe", json!("15m")),
                    ],
                ),
            ],
        },
        StarterSpec {
            id: "overbought-take",
            text: ids::STRATEGIES_STARTER_OVERBOUGHT_TAKE,
            strategy_type: StrategyType::Exit,
            timeframe: "5m",
            priority: 10,
            conditions: vec![
                condition(
                    "PositionProfit",
                    vec![("comparison", json!("ABOVE")), ("threshold_pct", json!(10))],
                ),
                condition(
                    "Rsi",
                    vec![
                        ("comparison", json!("ABOVE")),
                        ("threshold", json!(78)),
                        ("period", json!(14)),
                        ("timeframe", json!("5m")),
                    ],
                ),
            ],
        },
        StarterSpec {
            id: "trend-break",
            text: ids::STRATEGIES_STARTER_TREND_BREAK,
            strategy_type: StrategyType::Exit,
            timeframe: "15m",
            priority: 20,
            conditions: vec![
                condition(
                    "PositionHoldingTime",
                    vec![("comparison", json!("GREATER_THAN")), ("hours", json!(3.0))],
                ),
                condition(
                    "PositionProfit",
                    vec![("comparison", json!("BELOW")), ("threshold_pct", json!(0))],
                ),
                condition(
                    "PriceToMA",
                    vec![
                        ("position", json!("BELOW")),
                        ("period", json!(20)),
                        ("distance", json!(3.0)),
                        ("timeframe", json!("15m")),
                    ],
                ),
            ],
        },
    ]
}

/// The leaf of `starter`: the condition at its schema defaults with the starter's values
/// set, so the editor shows it as a user's own condition.
fn starter_leaf(
    registry: &ConditionRegistry,
    starter: &StarterCondition,
) -> crate::Result<RuleTree> {
    let mut leaf = registry
        .condition_at_defaults(starter.condition_type)
        .ok_or_else(|| strategies::Error::InvalidConditionValue {
            field: "condition type",
            value: starter.condition_type.to_owned(),
        })?;
    for (name, value) in &starter.values {
        let parameter = leaf
            .parameters
            .get_mut(*name)
            .ok_or(strategies::Error::MissingConditionParameter { field: *name })?;
        parameter.value = value.clone();
    }
    Ok(RuleTree::leaf(leaf))
}

/// The name and description of a starter in `locale`, falling back per key to the source
/// language.
fn starter_text(locale: &LanguageIdentifier, text: MessageId) -> crate::Result<(String, String)> {
    // The catalogs ship inside the binary, so a missing message or part is a defect of
    // the starter definitions.
    let missing = || {
        Error::Internal(InternalError::InvariantViolation {
            message: format!(
                "starter text {} has no name or description in the catalogs",
                text.as_str()
            ),
        })
    };
    let message = i18n::format_message(locale, text.as_str(), None).ok_or_else(missing)?;
    let name = message.value.ok_or_else(missing)?;
    let description = message
        .attributes
        .into_iter()
        .find_map(|(attribute, value)| (attribute == DESCRIPTION_ATTRIBUTE).then_some(value))
        .ok_or_else(missing)?;
    Ok((name, description))
}

fn starter(
    spec: &StarterSpec,
    registry: &ConditionRegistry,
    locale: &LanguageIdentifier,
    now: DateTime<Utc>,
) -> crate::Result<Strategy> {
    Strategy::check_priority(spec.priority)?;
    let (name, description) = starter_text(locale, spec.text)?;
    let leaves = spec
        .conditions
        .iter()
        .map(|item| starter_leaf(registry, item))
        .collect::<crate::Result<Vec<_>>>()?;
    Ok(Strategy {
        id: spec.id.to_owned(),
        name,
        description: Some(description),
        strategy_type: spec.strategy_type,
        enabled: false,
        priority: spec.priority,
        timeframe: spec.timeframe.to_owned(),
        rules: RuleTree::branch(LogicalOperator::And, leaves),
        parameters: HashMap::new(),
        created_at: now,
        updated_at: now,
        // A starter is an ordinary user record once written: it has no author.
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: TradingMode::Paper,
        rules_changed_at: now,
    })
}

/// The five starter strategies with their text in `locale`: switched off, in paper mode.
pub(crate) fn starters(
    locale: &LanguageIdentifier,
    now: DateTime<Utc>,
) -> crate::Result<Vec<Strategy>> {
    let registry = ConditionRegistry::new();
    specs()
        .iter()
        .map(|spec| starter(spec, &registry, locale, now))
        .collect()
}

/// Wait for the saved dashboard language without blocking a runtime worker. Starter text
/// is persisted once, so config lock contention must never select a fallback language.
async fn starter_locale() -> crate::Result<LanguageIdentifier> {
    tokio::task::spawn_blocking(|| {
        crate::config::with_config(|config| {
            i18n::resolve_locale(&config.gui.dashboard.interface.language, None)
        })
    })
    .await
    .map_err(Error::from)
}

/// Create the starter strategies when the strategies store was created empty and has not
/// received them yet, with the name and description in the language of the dashboard.
/// Each starter passes the validation the create route applies before it is stored, and
/// the store writes all five and consumes its mark in one transaction (see
/// [`db::seed_starters`]). A failure leaves the store as it was; the next start tries again.
pub(crate) async fn seed_pending() -> crate::Result<()> {
    if !db::starters_pending_async().await? {
        return Ok(());
    }
    let locale = starter_locale().await?;
    let starters = starters(&locale, Utc::now())?;
    for starter in &starters {
        strategies::validate_strategy(starter).await?;
    }
    if db::seed_starters_async(starters).await? {
        logger::info(
            LogTag::System,
            &format!("Starter strategies created in {locale}"),
        );
    }
    Ok(())
}

#[cfg(test)]
#[path = "starters_tests.rs"]
mod tests;
