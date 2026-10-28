// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The strategies overview: every strategy with its run status, its place in the entry
//! evaluation order and the figures of the book its mode trades on. Read in a fixed number
//! of store statements whatever the number of strategies, every one on the blocking pool;
//! the read changes no state and never starts a paper run.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions::paper::PaperStrategyEvidence;
use crate::positions::{self, Book, Position};
use crate::strategies::db::{get_strategies_with_totals_async, StrategiesWithTotals};
use crate::strategies::readiness::{
    counted_rounds, paper_strategy_evidence, readiness_progress, ReadinessProgress,
};
use crate::strategies::types::{Strategy, StrategyType};
use crate::trader::admission::EntryBlock;
use crate::trader::paper::TradingMode;
use crate::trader::stats::{round_totals, RoundTotals};
use crate::trader::{EntryIdle, TraderRunState};

/// Every strategy with what it is doing and what it achieved, plus the trader-level state
/// the roster states once.
#[derive(Debug, Clone, Serialize)]
pub struct StrategiesOverview {
    pub run_state: TraderRunState,
    /// The trader-level block that stops every enabled strategy, if one holds.
    pub notice: Option<RunNotice>,
    /// The built-in exit rules, which decide before any exit strategy, and whether each is
    /// on.
    pub rule_exits: Vec<RuleExitState>,
    pub counts: StrategyCounts,
    /// In the order the store lists them, which is the entry evaluation order of each book.
    pub strategies: Vec<StrategySummary>,
}

/// A trader-level block that stops every enabled strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunNotice {
    /// Explore Mode, or wallet and RPC setup unfinished: the app has no trader yet.
    SetupRequired,
    /// The emergency stop is engaged.
    ForceStopped,
    /// The Auto Trader is switched off.
    TraderOff,
    /// The Auto Trader is on and waits for the core services.
    Waiting,
    /// Both the entry and the exit monitor are switched off.
    MonitorsOff,
}

/// A built-in exit rule family, one per settings sub-tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleExitKind {
    StopLoss,
    TrailingStop,
    TakeProfit,
    TimeExit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RuleExitState {
    pub kind: RuleExitKind,
    pub enabled: bool,
}

/// How many strategies of one kind exist and how many are on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct KindCount {
    pub enabled: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct StrategyCounts {
    pub entry: KindCount,
    pub exit: KindCount,
}

/// What a strategy is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyRunStatus {
    /// It is evaluated on every cycle of its monitor.
    Running,
    /// It is on, and the trader is stopped, waiting or cannot enter on its book.
    NotRunning,
    /// It is switched off.
    Off,
    /// Explore Mode, or wallet and RPC setup unfinished.
    SetupRequired,
    /// The loss limit pauses live entries.
    LivePaused,
    /// The cash of the current paper run does not cover one entry.
    PaperCashShort,
    /// Its book holds as many open positions as the limit allows.
    SlotsFull,
    EntryMonitorOff,
    ExitMonitorOff,
}

/// One strategy with its status and the figures of the book it trades on. A figure the
/// book cannot attribute to the strategy is `None`, never a zero.
#[derive(Debug, Clone, Serialize)]
pub struct StrategySummary {
    #[serde(skip)]
    pub strategy: Strategy,
    pub status: StrategyRunStatus,
    /// Rank among the enabled entry strategies of the same mode in evaluation order, from
    /// 1; `None` for an exit strategy and for one that is off.
    pub order: Option<u32>,
    /// The book the figures read: the one the strategy's mode trades on.
    pub book: Book,
    /// Entry strategies: open positions it opened in `book`.
    pub open: Option<u32>,
    /// Entry strategies: open positions it opened in the other book, while in its other
    /// mode.
    pub open_other_book: Option<u32>,
    /// Exit strategies that are on: open positions it judges now.
    pub watching: Option<u32>,
    /// Closed rounds in `book` the strategy opened, or for an exit strategy closed: the
    /// lifetime on the live book, the current run on the paper book; `None` on the paper
    /// book while its strategy evidence cannot be read.
    pub rounds: Option<RoundTotals>,
    /// Unrealized P&L of the priced positions counted in `open`.
    pub unrealized_native: Option<f64>,
    /// Positions counted in `open` without a price.
    pub unpriced: Option<u32>,
    /// Paper strategies: how far they are from arming live; `None` while the paper strategy
    /// evidence cannot be read.
    pub readiness: Option<ReadinessProgress>,
    pub last_evaluated_at: Option<DateTime<Utc>>,
    pub last_signal_at: Option<DateTime<Utc>>,
}

/// The strategies overview. Store reads: the strategies with their evaluation totals, the
/// paper strategy evidence when a paper strategy exists, and the paper cash when a paper
/// entry strategy may enter. A failed paper evidence read leaves the strategies and the
/// live figures in place and the paper figures absent.
pub async fn strategies_overview() -> crate::Result<StrategiesOverview> {
    let run_state = crate::trader::run_state();
    let StrategiesWithTotals { strategies, totals } = get_strategies_with_totals_async().await?;
    let monitors = Monitors::read();
    let explore = run_state == TraderRunState::Explore;

    // The token-independent entry block of a mode, read only while an enabled entry
    // strategy of that mode would otherwise run.
    let mode_entry_block = |mode: TradingMode| {
        let entry_strategies = strategies
            .iter()
            .filter(|strategy| {
                strategy.strategy_type == StrategyType::Entry && strategy.trades_on(mode)
            })
            .count();
        let entry_monitor = monitors.entry;
        async move {
            match crate::trader::entry_idle(mode, run_state, entry_monitor, Some(entry_strategies))
                .await
            {
                Some(EntryIdle::Blocked { block }) => Some(block),
                _ => None,
            }
        }
    };

    let any_paper = !explore
        && strategies
            .iter()
            .any(|strategy| strategy.mode == TradingMode::Paper);
    let (evidence, live_block, paper_block) = tokio::join!(
        async {
            if any_paper {
                available(paper_strategy_evidence().await)
            } else {
                Some(PaperStrategyEvidence::default())
            }
        },
        mode_entry_block(TradingMode::Live),
        mode_entry_block(TradingMode::Paper),
    );
    let books = if explore {
        None
    } else {
        Some(BookPositions::read().await)
    };

    let order = entry_order(&strategies);
    let summaries = strategies
        .into_iter()
        .map(|strategy| {
            let entry_block = match strategy.mode {
                TradingMode::Live => live_block.as_ref(),
                TradingMode::Paper => paper_block.as_ref(),
            };
            let status = status_of(&strategy, run_state, monitors, entry_block);
            let figures = books
                .as_ref()
                .map(|books| books.figures_of(&strategy, evidence.as_ref()))
                .unwrap_or_default();
            let evaluated = totals.get(&strategy.id);
            StrategySummary {
                status,
                order: order.get(&strategy.id).copied(),
                book: strategy.mode.book(),
                open: figures.open,
                open_other_book: figures.open_other_book,
                watching: figures.watching,
                rounds: figures.rounds,
                unrealized_native: figures.unrealized_native,
                unpriced: figures.unpriced,
                readiness: figures.readiness,
                last_evaluated_at: evaluated.and_then(|totals| totals.last_evaluated_at),
                last_signal_at: evaluated.and_then(|totals| totals.last_signal_at),
                strategy,
            }
        })
        .collect::<Vec<_>>();

    Ok(StrategiesOverview {
        run_state,
        notice: notice_of(run_state),
        rule_exits: rule_exits(),
        counts: counts_of(summaries.iter().map(|summary| &summary.strategy)),
        strategies: summaries,
    })
}

/// The paper strategy evidence when it was read; a read failure is logged and leaves every
/// paper figure absent rather than failing the overview.
fn available(evidence: crate::Result<PaperStrategyEvidence>) -> Option<PaperStrategyEvidence> {
    match evidence {
        Ok(evidence) => Some(evidence),
        Err(error) => {
            logger::warning(
                LogTag::System,
                &format!("Strategies overview without paper figures: paper strategy evidence read failed: {error}"),
            );
            None
        }
    }
}

/// The monitor switches a strategy's status depends on.
#[derive(Debug, Clone, Copy)]
struct Monitors {
    entry: bool,
    exit: bool,
}

impl Monitors {
    fn read() -> Self {
        with_config(|cfg| Self {
            entry: cfg.trader.entry_monitor_enabled,
            exit: cfg.trader.exit_monitor_enabled,
        })
    }
}

/// The trader-level block in `run_state` that stops every enabled strategy.
fn notice_of(run_state: TraderRunState) -> Option<RunNotice> {
    match run_state {
        TraderRunState::Explore => Some(RunNotice::SetupRequired),
        TraderRunState::ForceStopped => Some(RunNotice::ForceStopped),
        TraderRunState::Stopped => Some(RunNotice::TraderOff),
        TraderRunState::Waiting => Some(RunNotice::Waiting),
        TraderRunState::Idle => Some(RunNotice::MonitorsOff),
        TraderRunState::EntryPaused | TraderRunState::Running => None,
    }
}

/// The status of `strategy` under `run_state`, the `monitors` and the entry block of its
/// mode.
fn status_of(
    strategy: &Strategy,
    run_state: TraderRunState,
    monitors: Monitors,
    entry_block: Option<&EntryBlock>,
) -> StrategyRunStatus {
    if run_state == TraderRunState::Explore {
        return StrategyRunStatus::SetupRequired;
    }
    if !strategy.enabled {
        return StrategyRunStatus::Off;
    }
    if matches!(
        run_state,
        TraderRunState::ForceStopped | TraderRunState::Stopped | TraderRunState::Waiting
    ) {
        return StrategyRunStatus::NotRunning;
    }
    match strategy.strategy_type {
        StrategyType::Entry if !monitors.entry => StrategyRunStatus::EntryMonitorOff,
        StrategyType::Entry => match entry_block {
            None => StrategyRunStatus::Running,
            Some(EntryBlock::LossLimit) => StrategyRunStatus::LivePaused,
            Some(EntryBlock::PositionLimit) => StrategyRunStatus::SlotsFull,
            Some(EntryBlock::PaperBalanceExhausted) => StrategyRunStatus::PaperCashShort,
            Some(_) => StrategyRunStatus::NotRunning,
        },
        StrategyType::Exit if !monitors.exit => StrategyRunStatus::ExitMonitorOff,
        StrategyType::Exit => StrategyRunStatus::Running,
    }
}

/// The rank of each enabled entry strategy within its own mode, from 1, in the order of
/// `strategies`. Each book runs its own pass over the strategies of its mode, in the order
/// the store lists them, so the roster takes that order from the store and restarts it
/// for each book rather than deciding one of its own.
fn entry_order(strategies: &[Strategy]) -> HashMap<String, u32> {
    let (mut live, mut paper) = (0, 0);
    strategies
        .iter()
        .filter(|strategy| strategy.enabled && strategy.strategy_type == StrategyType::Entry)
        .map(|strategy| {
            let rank = match strategy.mode {
                TradingMode::Live => &mut live,
                TradingMode::Paper => &mut paper,
            };
            *rank += 1;
            (strategy.id.clone(), *rank)
        })
        .collect()
}

fn counts_of<'a>(strategies: impl Iterator<Item = &'a Strategy>) -> StrategyCounts {
    let mut counts = StrategyCounts::default();
    for strategy in strategies {
        let count = match strategy.strategy_type {
            StrategyType::Entry => &mut counts.entry,
            StrategyType::Exit => &mut counts.exit,
        };
        count.total += 1;
        count.enabled += usize::from(strategy.enabled);
    }
    counts
}

/// The built-in exit rule families and whether each is on, in the order they decide.
fn rule_exits() -> Vec<RuleExitState> {
    with_config(|cfg| {
        let trader = &cfg.trader;
        vec![
            RuleExitState {
                kind: RuleExitKind::StopLoss,
                enabled: trader.stop_loss_enabled,
            },
            RuleExitState {
                kind: RuleExitKind::TrailingStop,
                enabled: trader.trailing_stop_enabled || trader.break_even_enabled,
            },
            RuleExitState {
                kind: RuleExitKind::TakeProfit,
                enabled: trader.take_profit_enabled,
            },
            RuleExitState {
                kind: RuleExitKind::TimeExit,
                enabled: trader.time_override_enabled || trader.time_exit_stagnation_enabled,
            },
        ]
    })
}

/// The figures of one strategy; every one absent until a book attributes it.
#[derive(Debug, Default)]
struct Figures {
    open: Option<u32>,
    open_other_book: Option<u32>,
    watching: Option<u32>,
    rounds: Option<RoundTotals>,
    unrealized_native: Option<f64>,
    unpriced: Option<u32>,
    readiness: Option<ReadinessProgress>,
}

/// The open positions of both books and the closed live rounds of each strategy, under
/// the entry strategy that opened each round and the exit strategy that closed it, read
/// from memory once per overview.
struct BookPositions {
    open_live: Vec<Position>,
    open_paper: Vec<Position>,
    live_rounds: HashMap<String, Vec<Position>>,
}

impl BookPositions {
    async fn read() -> Self {
        let (open_live, open_paper, live_rounds) = tokio::join!(
            positions::books::open_positions(Book::Live),
            positions::books::open_positions(Book::Paper),
            positions::books::live_auto_rounds(),
        );
        Self {
            open_live,
            open_paper,
            live_rounds,
        }
    }

    fn open(&self, book: Book) -> &[Position] {
        match book {
            Book::Live => &self.open_live,
            Book::Paper => &self.open_paper,
        }
    }

    /// The figures of `strategy` in the book of its mode, the paper rounds counted in its
    /// evidence window from `evidence`; without it every paper figure is absent. Open
    /// positions are state and are counted whenever they were entered.
    fn figures_of(&self, strategy: &Strategy, evidence: Option<&PaperStrategyEvidence>) -> Figures {
        let book = strategy.mode.book();
        let readiness = evidence
            .filter(|_| strategy.mode == TradingMode::Paper)
            .map(|evidence| readiness_progress(strategy, evidence));
        let paper_rounds = || {
            evidence.map(|evidence| {
                let (rounds, _) = counted_rounds(strategy, evidence);
                round_totals(rounds.iter().filter(|round| round.exit_time.is_some()))
            })
        };
        match strategy.strategy_type {
            StrategyType::Entry => {
                let opened = |book| {
                    self.open(book).iter().filter(|position| {
                        position.origin.entry_strategy_id() == Some(strategy.id.as_str())
                    })
                };
                let other = match book {
                    Book::Live => Book::Paper,
                    Book::Paper => Book::Live,
                };
                let open: Vec<&Position> = opened(book).collect();
                let unrealized = positions::open_unrealized(open.iter().copied());
                Figures {
                    open: Some(count(open.len())),
                    open_other_book: Some(count(opened(other).count())),
                    watching: None,
                    rounds: match book {
                        Book::Live => Some(self.live_rounds_of(strategy)),
                        Book::Paper => paper_rounds(),
                    },
                    unrealized_native: unrealized.native,
                    unpriced: Some(count(unrealized.unpriced)),
                    readiness,
                }
            }
            StrategyType::Exit => Figures {
                watching: strategy.enabled.then(|| count(self.watched_by(strategy))),
                rounds: match book {
                    Book::Live => Some(self.live_rounds_of(strategy)),
                    Book::Paper => paper_rounds(),
                },
                readiness,
                ..Figures::default()
            },
        }
    }

    /// The totals of the closed live rounds `strategy` took part in: those its entry
    /// strategy opened, or those its exit strategy closed.
    fn live_rounds_of(&self, strategy: &Strategy) -> RoundTotals {
        round_totals(
            self.live_rounds
                .get(&strategy.id)
                .map(Vec::as_slice)
                .unwrap_or_default(),
        )
    }

    /// The open positions `strategy`, an exit strategy, judges now: in every book its mode
    /// judges, those it manages whose management admits policy exits.
    fn watched_by(&self, strategy: &Strategy) -> usize {
        Book::ALL
            .into_iter()
            .filter(|book| crate::strategies::exit_strategy_modes(*book).contains(&strategy.mode))
            .flat_map(|book| self.open(book))
            .filter(|position| {
                position.management.allows_policy_exits()
                    && strategy.manages(position.origin.entry_strategy_id())
            })
            .count()
    }
}

fn count(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
#[path = "overview_tests.rs"]
mod tests;
