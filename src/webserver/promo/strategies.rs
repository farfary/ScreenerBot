// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Promo generators for the Strategies sub-tab: a fixed roster of entry and exit
//! strategies whose rounds are the promo trades, assembled by the live overview code so
//! every status and figure is the one the real roster computes.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::positions::paper::PaperStrategyEvidence;
use crate::positions::Position;
use crate::strategies::conditions::ConditionRegistry;
use crate::strategies::overview::{
    assemble, BookPositions, Monitors, OverviewInputs, StrategiesOverview,
};
use crate::strategies::types::{
    Condition, EvaluationTotals, LogicalOperator, Parameter, RuleTree, Strategy, StrategyType,
};
use crate::trader::paper::TradingMode;

use super::data::{PROMO_CLOSED_TOKENS, PROMO_OPEN_TOKENS};
use super::trader::{
    get_promo_monitors_status, get_promo_trader_status, promo_closed_rounds, promo_open_rounds,
};

/// One promo strategy. Its rules join `conditions` with AND, each at the defaults of its
/// condition schema, so the descriptions state what those defaults do.
struct PromoStrategy {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    strategy_type: StrategyType,
    mode: TradingMode,
    enabled: bool,
    timeframe: &'static str,
    conditions: &'static [&'static str],
    /// Days since it was created, which is when its rules last changed.
    age_days: i64,
}

/// The roster in store order (priority, then name). The live entry strategies opened
/// every Auto Trader promo position ([`entry_strategy_of`]); none of the promo trades was
/// closed by an exit strategy, so the exit strategies hold no rounds. The switched-off exit
/// would have sold every open position held past one hour.
const PROMO_STRATEGIES: &[PromoStrategy] = &[
    PromoStrategy {
        id: "promo-momentum-breakout",
        name: "Momentum Breakout",
        description:
            "Buys a breakout above the 20-candle high when volume doubles and buyers lead.",
        strategy_type: StrategyType::Entry,
        mode: TradingMode::Live,
        enabled: true,
        timeframe: "5m",
        conditions: &["PriceBreakout", "VolumeSpike", "BuyPressure"],
        age_days: 21,
    },
    PromoStrategy {
        id: "promo-dip-recovery",
        name: "Dip Recovery",
        description: "Buys a 10-30% pullback from the recent high once RSI is oversold.",
        strategy_type: StrategyType::Entry,
        mode: TradingMode::Live,
        enabled: true,
        timeframe: "15m",
        conditions: &["PullbackFromHigh", "Rsi", "LiquidityLevel"],
        age_days: 34,
    },
    PromoStrategy {
        id: "promo-early-momentum",
        name: "Early Momentum",
        description: "Buys tokens past their first 30 minutes whose trading is accelerating.",
        strategy_type: StrategyType::Entry,
        mode: TradingMode::Live,
        enabled: true,
        timeframe: "1m",
        conditions: &["TokenAge", "TransactionVelocity", "LiquidityLevel"],
        age_days: 12,
    },
    PromoStrategy {
        id: "promo-vwap-reclaim",
        name: "VWAP Reclaim",
        description: "Paper-tests buying above VWAP right after a bullish EMA cross.",
        strategy_type: StrategyType::Entry,
        mode: TradingMode::Paper,
        enabled: true,
        timeframe: "5m",
        conditions: &["Vwap", "MaCross"],
        age_days: 2,
    },
    PromoStrategy {
        id: "promo-profit-giveback",
        name: "Profit Giveback Guard",
        description: "Sells a winner above 20% once it gives back half of its peak gain.",
        strategy_type: StrategyType::Exit,
        mode: TradingMode::Live,
        enabled: true,
        timeframe: "1m",
        conditions: &["PositionProfit", "ProfitGiveback"],
        age_days: 28,
    },
    PromoStrategy {
        id: "promo-drawdown-brake",
        name: "Drawdown Brake",
        description: "Sells when the price falls 10% from its peak since entry.",
        strategy_type: StrategyType::Exit,
        mode: TradingMode::Live,
        enabled: true,
        timeframe: "1m",
        conditions: &["DrawdownFromPeak"],
        age_days: 28,
    },
    PromoStrategy {
        id: "promo-stale-exit",
        name: "Stale Position Exit",
        description: "Sells any position held for more than one hour.",
        strategy_type: StrategyType::Exit,
        mode: TradingMode::Live,
        enabled: false,
        timeframe: "5m",
        conditions: &["PositionHoldingTime"],
        age_days: 40,
    },
];

/// Seconds since an enabled strategy's last evaluation: about one monitor cycle.
const PROMO_EVALUATED_SECS_AGO: i64 = 3;

/// Days a switched-off strategy has been off; its last evaluation was then.
const PROMO_OFF_DAYS: i64 = 3;

/// Generate the promo strategies in store order.
pub fn get_promo_strategies() -> Vec<Strategy> {
    let now = Utc::now();
    let registry = ConditionRegistry::new();
    PROMO_STRATEGIES
        .iter()
        .zip(1..)
        .map(|(promo, priority)| promo_strategy(promo, priority, &registry, now))
        .collect()
}

/// The promo strategy `id`, if the roster has one.
pub fn get_promo_strategy(id: &str) -> Option<Strategy> {
    get_promo_strategies()
        .into_iter()
        .find(|strategy| strategy.id == id)
}

/// Generate the promo strategies overview: the roster under the promo trader, with the
/// promo positions as its live book and an empty paper run.
pub fn get_promo_strategies_overview() -> StrategiesOverview {
    let now = Utc::now();
    let strategies = get_promo_strategies();
    let open = promo_open_rounds(now);
    let closed = promo_closed_rounds(now);
    let monitors = get_promo_monitors_status();
    let totals = evaluation_totals(&strategies, open.iter().chain(&closed), now);
    assemble(OverviewInputs {
        run_state: get_promo_trader_status().run_state,
        strategies,
        totals,
        monitors: Monitors {
            entry: monitors.entry_monitor.enabled,
            exit: monitors.exit_monitor.enabled,
        },
        live_block: None,
        paper_block: None,
        evidence: Some(PaperStrategyEvidence::default()),
        books: Some(BookPositions::of(open, Vec::new(), &closed)),
    })
}

/// The enabled promo strategies of each mode, live then paper, as the trader status counts
/// them.
pub(super) fn enabled_counts() -> (usize, usize) {
    let enabled = |mode| {
        PROMO_STRATEGIES
            .iter()
            .filter(|promo| promo.enabled && promo.mode == mode)
            .count()
    };
    (enabled(TradingMode::Live), enabled(TradingMode::Paper))
}

/// The live entry strategy that opened the Auto Trader's promo position in `mint`: the
/// promo positions take turns over the enabled live entry strategies by their place in the
/// fixture arrays. `None` for a mint no promo position holds.
pub(super) fn entry_strategy_of(mint: &str) -> Option<String> {
    let index = PROMO_OPEN_TOKENS
        .iter()
        .position(|row| row.2 == mint)
        .or_else(|| PROMO_CLOSED_TOKENS.iter().position(|row| row.2 == mint))?;
    let openers: Vec<&PromoStrategy> = PROMO_STRATEGIES
        .iter()
        .filter(|promo| {
            promo.enabled
                && promo.mode == TradingMode::Live
                && promo.strategy_type == StrategyType::Entry
        })
        .collect();
    openers
        .get(index % openers.len().max(1))
        .map(|promo| promo.id.to_owned())
}

fn promo_strategy(
    promo: &PromoStrategy,
    priority: i32,
    registry: &ConditionRegistry,
    now: DateTime<Utc>,
) -> Strategy {
    let created_at = now - Duration::days(promo.age_days);
    let updated_at = if promo.enabled {
        created_at
    } else {
        now - Duration::days(PROMO_OFF_DAYS)
    };
    Strategy {
        id: promo.id.to_owned(),
        name: promo.name.to_owned(),
        description: Some(promo.description.to_owned()),
        strategy_type: promo.strategy_type,
        enabled: promo.enabled,
        priority,
        timeframe: promo.timeframe.to_owned(),
        rules: RuleTree::branch(
            LogicalOperator::And,
            promo
                .conditions
                .iter()
                .filter_map(|condition_type| schema_default(registry, condition_type))
                .map(RuleTree::leaf)
                .collect(),
        ),
        parameters: HashMap::new(),
        created_at,
        updated_at,
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: promo.mode,
        rules_changed_at: created_at,
    }
}

/// A `condition_type` condition with every parameter at its schema default, as the editor
/// adds one; `None` for a type the registry does not know.
fn schema_default(registry: &ConditionRegistry, condition_type: &str) -> Option<Condition> {
    let schema = registry.get(condition_type)?.parameter_schema();
    let parameters = schema["parameters"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, spec)| {
            let default = spec["default"].clone();
            (
                name.clone(),
                Parameter {
                    value: default.clone(),
                    default,
                    constraints: None,
                },
            )
        })
        .collect();
    Some(Condition {
        condition_type: condition_type.to_owned(),
        parameters,
    })
}

/// The last evaluation and the last signal of each strategy. An enabled strategy was
/// evaluated on the latest monitor cycle and a switched-off one when it was switched off;
/// an entry strategy last signalled at the latest entry of the `rounds` it opened. The
/// overview reads only these two moments, so the counters stay at zero.
fn evaluation_totals<'a>(
    strategies: &[Strategy],
    rounds: impl Iterator<Item = &'a Position>,
    now: DateTime<Utc>,
) -> HashMap<String, EvaluationTotals> {
    let mut last_entry: HashMap<&str, DateTime<Utc>> = HashMap::new();
    for round in rounds {
        if let Some(id) = round.origin.entry_strategy_id() {
            let latest = last_entry.entry(id).or_insert(round.entry_time);
            *latest = (*latest).max(round.entry_time);
        }
    }
    strategies
        .iter()
        .map(|strategy| {
            let totals = EvaluationTotals {
                last_evaluated_at: Some(if strategy.enabled {
                    now - Duration::seconds(PROMO_EVALUATED_SECS_AGO)
                } else {
                    strategy.updated_at
                }),
                last_signal_at: last_entry.get(strategy.id.as_str()).copied(),
                ..EvaluationTotals::default()
            };
            (strategy.id.clone(), totals)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::positions::PositionOrigin;
    use crate::strategies::engine::{EngineConfig, StrategyEngine};
    use crate::strategies::overview::StrategyRunStatus;

    fn auto_rounds(rounds: &[Position]) -> Vec<&Position> {
        rounds
            .iter()
            .filter(|round| matches!(round.origin, PositionOrigin::Auto { .. }))
            .collect()
    }

    #[test]
    fn every_promo_strategy_validates_with_every_condition_known() {
        let engine = StrategyEngine::new(EngineConfig::default());
        let registry = ConditionRegistry::new();
        for promo in PROMO_STRATEGIES {
            for condition_type in promo.conditions {
                assert!(
                    registry.get(condition_type).is_some(),
                    "{}: unknown condition {condition_type}",
                    promo.id
                );
            }
        }
        for strategy in get_promo_strategies() {
            engine
                .validate_strategy(&strategy)
                .unwrap_or_else(|error| panic!("{}: {error}", strategy.id));
            let leaves = strategy.rules.conditions.as_ref().map_or(0, Vec::len);
            let promo = PROMO_STRATEGIES
                .iter()
                .find(|promo| promo.id == strategy.id)
                .unwrap();
            assert_eq!(leaves, promo.conditions.len(), "{}", strategy.id);
        }
    }

    #[test]
    fn the_roster_is_in_store_order_and_every_id_resolves() {
        let strategies = get_promo_strategies();
        assert!(strategies
            .windows(2)
            .all(|pair| (pair[0].priority, &pair[0].name) < (pair[1].priority, &pair[1].name)));
        for strategy in &strategies {
            let found = get_promo_strategy(&strategy.id).expect("roster id resolves");
            assert_eq!(found.name, strategy.name);
        }
        assert!(get_promo_strategy("unknown").is_none());
    }

    #[test]
    fn every_auto_position_was_opened_by_an_enabled_live_entry_strategy() {
        let strategies = get_promo_strategies();
        let now = Utc::now();
        let open = promo_open_rounds(now);
        let closed = promo_closed_rounds(now);
        let auto: Vec<&Position> = auto_rounds(&open)
            .into_iter()
            .chain(auto_rounds(&closed))
            .collect();
        assert!(!auto.is_empty());
        for round in auto {
            let id = round
                .origin
                .entry_strategy_id()
                .unwrap_or_else(|| panic!("{} has no entry strategy", round.symbol));
            let opener = strategies.iter().find(|s| s.id == id).unwrap();
            assert!(opener.enabled);
            assert_eq!(opener.mode, TradingMode::Live);
            assert_eq!(opener.strategy_type, StrategyType::Entry);
        }
    }

    #[test]
    fn the_overview_reconciles_with_the_promo_books_and_the_trader() {
        crate::config::utils::install_default_config();
        let overview = get_promo_strategies_overview();
        let now = Utc::now();
        let open = promo_open_rounds(now);
        let closed = promo_closed_rounds(now);

        assert_eq!(overview.notice, None);
        for summary in &overview.strategies {
            let expected = if summary.strategy.enabled {
                StrategyRunStatus::Running
            } else {
                StrategyRunStatus::Off
            };
            assert_eq!(summary.status, expected, "{}", summary.strategy.id);
            assert!(summary.last_evaluated_at.is_some());
        }

        let entries: Vec<_> = overview
            .strategies
            .iter()
            .filter(|summary| {
                summary.strategy.strategy_type == StrategyType::Entry
                    && summary.strategy.mode == TradingMode::Live
            })
            .collect();
        let opened: u32 = entries.iter().filter_map(|summary| summary.open).sum();
        assert_eq!(opened as usize, auto_rounds(&open).len());
        let rounds: usize = entries
            .iter()
            .filter_map(|summary| summary.rounds.map(|rounds| rounds.closed))
            .sum();
        assert_eq!(rounds, auto_rounds(&closed).len());
        let realized: f64 = entries
            .iter()
            .filter_map(|summary| summary.rounds.map(|rounds| rounds.realized_native))
            .sum();
        let expected: f64 = auto_rounds(&closed)
            .iter()
            .filter_map(|round| round.pnl)
            .sum();
        assert!((realized - expected).abs() < 1e-9);
        for summary in &entries {
            assert_eq!(summary.unpriced, Some(0), "{}", summary.strategy.id);
            assert!(summary.last_signal_at.is_some(), "{}", summary.strategy.id);
        }

        let (live, paper) = enabled_counts();
        let status = get_promo_trader_status();
        assert_eq!(status.live_strategies, live);
        assert_eq!(status.paper_strategies, paper);
        assert!(live > 0 && paper > 0);
    }
}
