// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Shared composition and telemetry for Promo Studio's three copied wallets.
//! Paper and live fixture families settle into the same production stats, rounds,
//! insights and workspace assembly.

mod live;
mod paper;

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};

use crate::chains::active_chain;
use crate::positions::{Position, PositionManagement, PositionOrigin};
use crate::trader::copy::{
    management_for_exit_mode, CopyActivityRow, CopyMode, CopyOutcome, CopySkip, CopyTask,
    CopyTelemetry, ExitMode, PaperPosition, SizingMode, TargetObservations,
};
use crate::trader::policy::ExitPolicyOverrides;

use super::super::data::{PROMO_CLOSED_TOKENS, PROMO_OPEN_TOKENS};

/// The copied wallets: (task id, label, address, age in days). The Watched tab
/// lists exactly these, as old as their tasks, each carrying the
/// `WatchSource::Copy` of its task.
pub(in crate::webserver::promo) const PROMO_COPY_WALLETS: [(i64, &str, &str, i64); 3] = [
    (
        1,
        "Whale · early rotations",
        "GDfnEsia2WLAW5t8yx2X5j2mkfA74i5kY9dGZZ2q5wG7",
        11,
    ),
    (
        2,
        "Trend scout",
        "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9",
        9,
    ),
    (
        3,
        "Momentum desk",
        "3nMFwZXwY1s1M5s8vYAHqd4wGs4iSxXE4LRoUMMYqEgF",
        21,
    ),
];

fn base_task(wallet: (i64, &str, &str, i64), now: DateTime<Utc>) -> CopyTask {
    let created_at = now - Duration::days(wallet.3);
    CopyTask {
        id: wallet.0,
        chain: active_chain(),
        target_address: wallet.2.to_owned(),
        label: Some(wallet.1.to_owned()),
        enabled: true,
        mode: CopyMode::Paper,
        sizing: SizingMode::Fixed { sol: 0.1 },
        exit_mode: ExitMode::Hybrid,
        exit_policy_overrides: ExitPolicyOverrides::default(),
        max_native_per_trade: 0.1,
        max_native_per_token: 0.3,
        total_budget_native: 1.0,
        min_target_trade_native: Some(0.5),
        max_target_trade_native: Some(40.0),
        buy_once_per_token: true,
        slippage_pct: 1.5,
        created_at,
        updated_at: created_at + Duration::hours(6),
        require_filter_pass: None,
        pause_reason: None,
        paused_at: None,
    }
}

/// The three tasks: a live whale followed at a share of its size, the paper task
/// being evaluated (the largest budget -- Paper first, then Arm, is the flow the
/// product enforces), and a live fixed-size desk whose exits are its own rules.
pub(super) fn tasks(now: DateTime<Utc>) -> Vec<CopyTask> {
    let [whale, scout, momentum] = PROMO_COPY_WALLETS;
    vec![
        CopyTask {
            mode: CopyMode::Live,
            sizing: SizingMode::RatioOfTarget { pct: 2.5 },
            max_native_per_trade: 0.25,
            max_native_per_token: 0.5,
            total_budget_native: 6.0,
            min_target_trade_native: Some(2.0),
            ..base_task(whale, now)
        },
        CopyTask {
            exit_policy_overrides: paper::paper_overrides(),
            total_budget_native: 8.0,
            max_target_trade_native: Some(25.0),
            ..base_task(scout, now)
        },
        CopyTask {
            mode: CopyMode::Live,
            sizing: SizingMode::Fixed { sol: 0.15 },
            exit_mode: ExitMode::BuyOnly,
            max_native_per_trade: 0.15,
            max_native_per_token: 0.45,
            total_budget_native: 10.0,
            ..base_task(momentum, now)
        },
    ]
}

/// A deterministic live-stream arrival for a task's `n`th observation.
fn arrival_ms(task_id: i64, n: usize) -> i64 {
    let (base, step) = match task_id {
        1 => (480, 120),
        2 => (360, 95),
        _ => (620, 150),
    };
    base + ((n as i64 * 37 + task_id * 11) % 9) * step
}

fn whole_second(at: DateTime<Utc>) -> DateTime<Utc> {
    DateTime::from_timestamp(at.timestamp(), 0).unwrap_or(at)
}

/// Telemetry of a target trade whose block landed at `block` and reached the bot
/// `arrival_ms` later.
fn observed(
    block: DateTime<Utc>,
    arrival_ms: i64,
    target_price: f64,
    fill_price: Option<f64>,
    live: bool,
) -> CopyTelemetry {
    let block = whole_second(block);
    let detected_at = block + Duration::milliseconds(arrival_ms);
    let decided_at = detected_at + Duration::milliseconds(120);
    CopyTelemetry {
        target_block_time: Some(block.timestamp()),
        detected_at,
        decoded_at: detected_at + Duration::milliseconds(35),
        decided_at,
        submitted_at: live.then(|| decided_at + Duration::milliseconds(90)),
        confirmed_at: live.then(|| decided_at + Duration::milliseconds(700)),
        target_price_sol: Some(target_price),
        fill_price_sol: fill_price,
        backfill: false,
    }
}

/// Telemetry of an exit the task's own rules made: there is no target trade.
fn rule_exit(at: DateTime<Utc>, fill_price: f64) -> CopyTelemetry {
    CopyTelemetry {
        target_block_time: None,
        detected_at: at,
        decoded_at: at,
        decided_at: at,
        submitted_at: None,
        confirmed_at: None,
        target_price_sol: None,
        fill_price_sol: Some(fill_price),
        backfill: false,
    }
}

fn signature(kind: &str, task_id: i64, n: usize, mint: &str) -> String {
    format!("promo-{kind}-{task_id}-{n}-{}", &mint[..mint.len().min(8)])
}

/// The wallet's own buy that `sized` was copied from.
fn target_trade_sol(task: &CopyTask, sized: f64) -> f64 {
    match task.sizing {
        SizingMode::RatioOfTarget { pct } if pct > 0.0 => sized / (pct / 100.0),
        _ => sized * 24.0,
    }
}

/// The store's decision kind: the outcome's own tag.
fn kind(outcome: &CopyOutcome) -> String {
    serde_json::to_value(outcome)
        .ok()
        .and_then(|value| value.get("outcome")?.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The token a decision is about, as the store's activity filter reads it.
pub(super) fn outcome_mint(outcome: &CopyOutcome) -> Option<&str> {
    match outcome {
        CopyOutcome::PaperFilled(decision) => Some(&decision.mint),
        CopyOutcome::LiveSubmitted(decision)
        | CopyOutcome::LiveConfirmed(decision)
        | CopyOutcome::LiveFailed(decision) => Some(&decision.mint),
        CopyOutcome::PaperSellObserved(decision)
        | CopyOutcome::LiveSellSubmitted(decision)
        | CopyOutcome::LiveSellFailed(decision) => Some(&decision.mint),
        CopyOutcome::Skipped { mint, .. } => mint.as_deref(),
    }
}

/// The pool price a paper holding is marked at: the session's current price.
pub(super) fn mark(position: &PaperPosition) -> Option<f64> {
    PROMO_OPEN_TOKENS
        .iter()
        .find(|token| token.2 == position.mint)
        .map(|token| token.5)
}

/// Who owns a promo position: the live copy task that opened it, if one did,
/// else the Auto Trader.
pub(in crate::webserver::promo) fn position_owner(
    mint: &str,
) -> (PositionOrigin, PositionManagement) {
    let owner = live::LIVE_POSITIONS
        .iter()
        .find(|(_, open, closed)| {
            open.iter().any(|&index| PROMO_OPEN_TOKENS[index].2 == mint)
                || closed
                    .iter()
                    .any(|&index| PROMO_CLOSED_TOKENS[index].2 == mint)
        })
        .and_then(|(task_id, ..)| {
            tasks(Utc::now())
                .into_iter()
                .find(|task| task.id == *task_id)
        });
    match owner {
        Some(task) => (
            PositionOrigin::Copy {
                task_id: task.id,
                source_wallet: task.target_address,
            },
            management_for_exit_mode(task.exit_mode),
        ),
        None => (
            PositionOrigin::Auto { strategy_id: None },
            PositionManagement::AutoTrader,
        ),
    }
}

type Decisions = Vec<(DateTime<Utc>, CopyOutcome)>;

/// A copied paper buy at `market`; returns the fill and when it was decided.
/// Declined target buys, each for a reason its task's rules give.
fn skips(tasks: &[CopyTask], now: DateTime<Utc>, decisions: &mut Decisions) {
    let open = |index: usize| (PROMO_OPEN_TOKENS[index].2, PROMO_OPEN_TOKENS[index].5);
    let closed = |index: usize| (PROMO_CLOSED_TOKENS[index].2, PROMO_CLOSED_TOKENS[index].4);
    // (task id, (mint, price), reason, minutes ago)
    let plan = [
        (
            2,
            open(9),
            CopySkip::TargetBelowMinimum { minimum_sol: 0.5 },
            12,
        ),
        (2, open(6), CopySkip::AlreadyBought, 20),
        (2, closed(2), CopySkip::AlreadyBought, 190),
        (
            2,
            closed(1),
            CopySkip::TargetAboveMaximum { maximum_sol: 25.0 },
            420,
        ),
        (
            2,
            closed(7),
            CopySkip::TargetBelowMinimum { minimum_sol: 0.5 },
            1_800,
        ),
        (
            1,
            open(3),
            CopySkip::TargetBelowMinimum { minimum_sol: 2.0 },
            9,
        ),
        (1, open(0), CopySkip::AlreadyBought, 64),
        (
            1,
            closed(12),
            CopySkip::TargetBelowMinimum { minimum_sol: 2.0 },
            300,
        ),
        (3, open(5), CopySkip::FilterRequired, 26),
        (
            3,
            open(9),
            CopySkip::TargetBelowMinimum { minimum_sol: 0.5 },
            95,
        ),
        (3, closed(15), CopySkip::FilterRequired, 540),
    ];
    // A trade replayed after downtime, far past the arrival limit: recorded as a
    // skip and kept out of every arrival figure.
    let stale = (
        2,
        closed(10),
        CopySkip::StaleObservation {
            arrival_ms: 52_000,
            threshold_ms: 4_000,
        },
        2_900,
    );
    for (n, (task_id, (mint, price), reason, minutes_ago)) in
        plan.into_iter().chain([stale]).enumerate()
    {
        let Some(task) = tasks.iter().find(|task| task.id == task_id) else {
            continue;
        };
        let backfill = matches!(reason, CopySkip::StaleObservation { .. });
        let arrival = match &reason {
            CopySkip::StaleObservation { arrival_ms, .. } => *arrival_ms as i64,
            _ => arrival_ms(task_id, n + 40),
        };
        let mut telemetry = observed(
            now - Duration::minutes(minutes_ago),
            arrival,
            price,
            None,
            false,
        );
        telemetry.backfill = backfill;
        decisions.push((
            telemetry.decided_at,
            CopyOutcome::Skipped {
                task_id: task.id,
                signature: signature("skip", task.id, n, mint),
                mint: Some(mint.to_owned()),
                reason,
                decided_at: telemetry.decided_at,
                telemetry: Some(telemetry),
                paper: None,
            },
        ));
    }
}

pub(super) struct Desk {
    pub(super) tasks: Vec<CopyTask>,
    /// Newest first, as the store reads it.
    pub(super) activity: Vec<CopyActivityRow>,
    pub(super) positions: Vec<Position>,
    paper_book: Vec<PaperPosition>,
    spend: Vec<(i64, f64)>,
}

impl Desk {
    pub(super) fn build() -> Self {
        let now = Utc::now();
        let tasks = tasks(now);
        let mut decisions = Vec::new();
        let mut positions = Vec::new();
        let mut paper_book = Vec::new();
        let mut spend = Vec::with_capacity(tasks.len());
        for task in &tasks {
            let spent = match task.mode {
                CopyMode::Live => live::live_desk(task, now, &mut decisions, &mut positions),
                CopyMode::Paper => paper::paper_desk(task, now, &mut decisions, &mut paper_book),
            };
            spend.push((task.id, spent));
        }
        skips(&tasks, now, &mut decisions);
        // The store numbers decisions in the order they were recorded; the paper
        // replay depends on it.
        decisions.sort_by_key(|(at, _)| *at);
        let mut activity = decisions
            .into_iter()
            .enumerate()
            .map(|(index, (created_at, outcome))| CopyActivityRow {
                id: index as i64 + 1,
                task_id: outcome.task_id(),
                kind: kind(&outcome),
                outcome,
                created_at,
            })
            .collect::<Vec<_>>();
        activity.reverse();
        Self {
            tasks,
            activity,
            positions,
            paper_book,
            spend,
        }
    }

    pub(super) fn task(&self, id: i64) -> Option<&CopyTask> {
        self.tasks.iter().find(|task| task.id == id)
    }

    pub(super) fn task_activity(&self, id: i64) -> Vec<CopyActivityRow> {
        self.activity
            .iter()
            .filter(|row| row.task_id == id)
            .cloned()
            .collect()
    }

    pub(super) fn paper_book(&self, id: i64) -> Vec<PaperPosition> {
        self.paper_book
            .iter()
            .filter(|position| position.task_id == id)
            .cloned()
            .collect()
    }

    pub(super) fn spent_sol(&self, id: i64) -> f64 {
        self.spend
            .iter()
            .find(|(task_id, _)| *task_id == id)
            .map_or(0.0, |(_, spent)| *spent)
    }

    /// The target trades the given tasks observed: every buy they copied or
    /// declined and every wallet sell they mirrored. Exits by the task's own
    /// rules are not the wallet's trades.
    pub(super) fn observations(&self, task_ids: &[i64]) -> TargetObservations {
        let mut observations = TargetObservations::default();
        let mut tokens = HashSet::new();
        for row in self
            .activity
            .iter()
            .filter(|row| task_ids.contains(&row.task_id))
        {
            let buy = match &row.outcome {
                CopyOutcome::PaperSellObserved(decision) if decision.exit_rule.is_some() => {
                    continue
                }
                CopyOutcome::PaperSellObserved(_)
                | CopyOutcome::LiveSellSubmitted(_)
                | CopyOutcome::LiveSellFailed(_) => false,
                _ => true,
            };
            observations.swaps += 1;
            if buy {
                observations.buys += 1;
            } else {
                observations.sells += 1;
            }
            tokens.extend(outcome_mint(&row.outcome));
            observations.first_seen = Some(
                observations
                    .first_seen
                    .map_or(row.created_at, |seen| seen.min(row.created_at)),
            );
            observations.last_seen = Some(
                observations
                    .last_seen
                    .map_or(row.created_at, |seen| seen.max(row.created_at)),
            );
        }
        observations.tokens = tokens.len() as u64;
        observations
    }
}
