//! Pure performance analytics for one task. Closed rounds are replayed from the
//! recorded paper decisions -- the same arithmetic the paper ledger books -- or
//! read from the real positions a live task opened; every breakdown is built on
//! those rounds and on the decision telemetry.

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::positions::{Position, PositionOrigin};

use super::analytics::{arrival_distance_ms, summarize_arrival_distances};
use super::types::{
    ArrivalDistanceStats, CopyActivityRow, CopyBook, CopyOutcome, CopySkip, PaperExitRule,
};

/// Remaining tokens below this fraction of the round count as sold out, as in
/// the paper ledger.
const CLOSE_RESIDUE_FRACTION: f64 = 1e-9;
/// Newest closed rounds kept in the response.
const RECENT_ROUNDS: usize = 50;
/// Upper bounds of the arrival histogram buckets; the last bucket is open.
const LATENCY_BUCKETS_MS: [u64; 5] = [500, 1_000, 2_000, 4_000, 8_000];

/// An inclusive time window; an absent bound is open.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsightRange {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
}

impl InsightRange {
    fn contains(&self, at: DateTime<Utc>) -> bool {
        self.from.is_none_or(|from| at >= from) && self.to.is_none_or(|to| at <= to)
    }
}

/// One buy-to-sold-out round of a token.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CopyRound {
    pub mint: String,
    pub book: CopyBook,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
    pub invested_sol: f64,
    pub proceeds_sol: f64,
    pub pnl_sol: f64,
    pub pnl_pct: Option<f64>,
    pub hold_seconds: i64,
    /// What closed it: `target_sell`, an exit rule, or the live close reason.
    pub exit: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CurvePoint {
    pub at: DateTime<Utc>,
    pub cumulative_pnl_sol: f64,
    pub round_pnl_sol: f64,
    pub mint: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ExitBucket {
    pub exit: String,
    pub legs: usize,
    pub pnl_sol: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CountBucket {
    pub key: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct LatencyBucket {
    pub upper_ms: Option<u64>,
    pub count: usize,
}

/// Execution price against the target's own price, in percent of the target
/// price; positive means worse than the target (paid more, received less).
#[derive(Debug, Clone, Default, Serialize)]
pub struct SlippageStats {
    pub samples: usize,
    pub average_pct: Option<f64>,
    pub median_pct: Option<f64>,
    pub p95_pct: Option<f64>,
    pub worst_pct: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DecisionCounts {
    pub fills: usize,
    pub exits: usize,
    pub skips: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CopyInsights {
    pub task_id: i64,
    pub book: CopyBook,
    pub rounds: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate_pct: Option<f64>,
    pub realized_pnl_sol: f64,
    pub average_win_sol: Option<f64>,
    pub average_loss_sol: Option<f64>,
    pub best_round_sol: Option<f64>,
    pub worst_round_sol: Option<f64>,
    pub profit_factor: Option<f64>,
    pub average_hold_seconds: Option<f64>,
    pub decisions: DecisionCounts,
    pub pnl_curve: Vec<CurvePoint>,
    pub exit_breakdown: Vec<ExitBucket>,
    pub skip_breakdown: Vec<CountBucket>,
    pub arrival: ArrivalDistanceStats,
    pub arrival_histogram: Vec<LatencyBucket>,
    pub slippage: SlippageStats,
    pub recent_rounds: Vec<CopyRound>,
}

/// The label an exit leg is grouped under.
pub fn exit_label(rule: Option<PaperExitRule>) -> String {
    match rule {
        None => "target_sell",
        Some(PaperExitRule::StopLoss) => "stop_loss",
        Some(PaperExitRule::TrailingStop) => "trailing_stop",
        Some(PaperExitRule::TakeProfit) => "take_profit",
        Some(PaperExitRule::TimeOverride) => "time_override",
        Some(PaperExitRule::Manual) => "manual",
    }
    .to_owned()
}

#[derive(Default)]
struct OpenRound {
    opened_at: Option<DateTime<Utc>>,
    tokens: f64,
    cost: f64,
    realized_cost: f64,
    proceeds: f64,
}

/// A sell leg with its realized result, for the exit breakdown.
struct ExitLeg {
    at: DateTime<Utc>,
    exit: String,
    pnl_sol: f64,
}

/// Replay the paper book from its decisions, oldest first.
fn replay_paper(rows: &[&CopyActivityRow]) -> (Vec<CopyRound>, Vec<ExitLeg>) {
    let mut open: HashMap<&str, OpenRound> = HashMap::new();
    let mut rounds = Vec::new();
    let mut legs = Vec::new();
    for row in rows {
        match &row.outcome {
            CopyOutcome::PaperFilled(decision) => {
                let round = open.entry(decision.mint.as_str()).or_default();
                round.opened_at.get_or_insert(decision.telemetry.decided_at);
                round.tokens += decision.fill.token_amount;
                round.cost += decision.fill.total_cost_sol;
            }
            CopyOutcome::PaperSellObserved(decision) => {
                let Some(fill) = &decision.paper_fill else {
                    continue;
                };
                let Some(round) = open.get_mut(decision.mint.as_str()) else {
                    continue;
                };
                if round.tokens <= 0.0 {
                    continue;
                }
                let held = round.tokens;
                let sold = fill.token_amount.min(held);
                let cost_sold = round.cost * sold / held;
                round.tokens -= sold;
                round.cost -= cost_sold;
                round.realized_cost += cost_sold;
                round.proceeds += fill.net_proceeds_sol;
                let exit = exit_label(decision.exit_rule);
                let at = decision.telemetry.decided_at;
                legs.push(ExitLeg {
                    at,
                    exit: exit.clone(),
                    pnl_sol: fill.net_proceeds_sol - cost_sold,
                });
                if round.tokens <= held * CLOSE_RESIDUE_FRACTION {
                    if let Some(closed) = open.remove(decision.mint.as_str()) {
                        let opened_at = closed.opened_at.unwrap_or(at);
                        let pnl = closed.proceeds - closed.realized_cost;
                        rounds.push(CopyRound {
                            mint: decision.mint.clone(),
                            book: CopyBook::Paper,
                            opened_at,
                            closed_at: at,
                            invested_sol: closed.realized_cost,
                            proceeds_sol: closed.proceeds,
                            pnl_sol: pnl,
                            pnl_pct: (closed.realized_cost > 0.0)
                                .then(|| pnl / closed.realized_cost * 100.0),
                            hold_seconds: (at - opened_at).num_seconds(),
                            exit,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    (rounds, legs)
}

/// Closed real positions this task opened.
fn live_rounds(task_id: i64, positions: &[Position]) -> Vec<CopyRound> {
    positions
        .iter()
        .filter(|position| {
            matches!(position.origin, PositionOrigin::Copy { task_id: origin, .. } if origin == task_id)
                && position.transaction_exit_verified
        })
        .filter_map(|position| {
            let closed_at = position.exit_time?;
            let pnl = position.pnl.unwrap_or_default();
            Some(CopyRound {
                mint: position.mint.clone(),
                book: CopyBook::Live,
                opened_at: position.entry_time,
                closed_at,
                invested_sol: position.total_size_sol,
                proceeds_sol: position.sol_received.unwrap_or_default(),
                pnl_sol: pnl,
                pnl_pct: position.pnl_percent,
                hold_seconds: (closed_at - position.entry_time).num_seconds(),
                exit: position
                    .closed_reason
                    .clone()
                    .unwrap_or_else(|| "closed".to_owned()),
            })
        })
        .collect()
}

/// Every closed round of a task in the given book, oldest close first. The
/// activity may come in any order.
pub fn closed_rounds(
    task_id: i64,
    book: CopyBook,
    activity: &[CopyActivityRow],
    positions: &[Position],
) -> Vec<CopyRound> {
    let mut rounds = match book {
        CopyBook::Paper => replay_paper(&ordered(task_id, activity)).0,
        CopyBook::Live => live_rounds(task_id, positions),
    };
    rounds.sort_by_key(|round| round.closed_at);
    rounds
}

fn ordered(task_id: i64, activity: &[CopyActivityRow]) -> Vec<&CopyActivityRow> {
    let mut rows = activity
        .iter()
        .filter(|row| row.task_id == task_id)
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| row.id);
    rows
}

fn skip_key(reason: &CopySkip) -> String {
    let value = serde_json::to_value(reason).unwrap_or_default();
    let kind = value["kind"].as_str().unwrap_or("unknown");
    match value["block"]["kind"].as_str() {
        Some(block) => format!("{kind}.{block}"),
        None => kind.to_owned(),
    }
}

/// Target-relative execution cost of a fill, if both prices are known.
fn slippage_pct(outcome: &CopyOutcome) -> Option<f64> {
    let (target, fill, buy) = match outcome {
        CopyOutcome::PaperFilled(decision) => (
            decision.telemetry.target_price_sol,
            Some(decision.fill.fill_price_sol),
            true,
        ),
        CopyOutcome::LiveConfirmed(decision) => (
            decision.telemetry.target_price_sol,
            decision.telemetry.fill_price_sol,
            true,
        ),
        CopyOutcome::PaperSellObserved(decision) if decision.exit_rule.is_none() => (
            decision.telemetry.target_price_sol,
            decision.paper_fill.as_ref().map(|fill| fill.fill_price_sol),
            false,
        ),
        _ => return None,
    };
    let (target, fill) = (target?, fill?);
    if !(target.is_finite() && fill.is_finite() && target > 0.0 && fill > 0.0) {
        return None;
    }
    Some(if buy {
        (fill / target - 1.0) * 100.0
    } else {
        (1.0 - fill / target) * 100.0
    })
}

fn summarize_slippage(mut samples: Vec<f64>) -> SlippageStats {
    if samples.is_empty() {
        return SlippageStats::default();
    }
    samples.sort_by(f64::total_cmp);
    let count = samples.len();
    let percentile =
        |pct: usize| samples[(count * pct).div_ceil(100).saturating_sub(1).min(count - 1)];
    SlippageStats {
        samples: count,
        average_pct: Some(samples.iter().sum::<f64>() / count as f64),
        median_pct: Some(percentile(50)),
        p95_pct: Some(percentile(95)),
        worst_pct: samples.last().copied(),
    }
}

fn latency_histogram(samples: &[u64]) -> Vec<LatencyBucket> {
    let mut buckets = LATENCY_BUCKETS_MS
        .iter()
        .map(|upper| LatencyBucket {
            upper_ms: Some(*upper),
            count: 0,
        })
        .chain(std::iter::once(LatencyBucket {
            upper_ms: None,
            count: 0,
        }))
        .collect::<Vec<_>>();
    for sample in samples {
        let index = LATENCY_BUCKETS_MS
            .iter()
            .position(|upper| sample < upper)
            .unwrap_or(LATENCY_BUCKETS_MS.len());
        buckets[index].count += 1;
    }
    buckets
}

/// The task's full analytics for `book` within `range`: rounds count by close
/// time, decisions by record time.
pub fn build_insights(
    task_id: i64,
    book: CopyBook,
    activity: &[CopyActivityRow],
    positions: &[Position],
    range: InsightRange,
) -> CopyInsights {
    let rows = ordered(task_id, activity);
    let (paper_rounds, legs) = replay_paper(&rows);
    let mut rounds = match book {
        CopyBook::Paper => paper_rounds,
        CopyBook::Live => live_rounds(task_id, positions),
    };
    rounds.retain(|round| range.contains(round.closed_at));
    rounds.sort_by_key(|round| round.closed_at);

    let mut insights = CopyInsights {
        task_id,
        book,
        rounds: rounds.len(),
        ..CopyInsights::default()
    };
    let (wins, losses): (Vec<f64>, Vec<f64>) = rounds
        .iter()
        .map(|round| round.pnl_sol)
        .partition(|pnl| *pnl > 0.0);
    insights.wins = wins.len();
    insights.losses = losses.len();
    insights.realized_pnl_sol = rounds.iter().map(|round| round.pnl_sol).sum();
    let average = |values: &[f64]| {
        (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
    };
    insights.win_rate_pct =
        (!rounds.is_empty()).then(|| wins.len() as f64 / rounds.len() as f64 * 100.0);
    insights.average_win_sol = average(&wins);
    insights.average_loss_sol = average(&losses);
    insights.best_round_sol = rounds.iter().map(|round| round.pnl_sol).reduce(f64::max);
    insights.worst_round_sol = rounds.iter().map(|round| round.pnl_sol).reduce(f64::min);
    let gross_loss = -losses.iter().sum::<f64>();
    insights.profit_factor = (gross_loss > 0.0).then(|| wins.iter().sum::<f64>() / gross_loss);
    insights.average_hold_seconds = average(
        &rounds
            .iter()
            .map(|round| round.hold_seconds as f64)
            .collect::<Vec<_>>(),
    );
    let mut cumulative = 0.0;
    insights.pnl_curve = rounds
        .iter()
        .map(|round| {
            cumulative += round.pnl_sol;
            CurvePoint {
                at: round.closed_at,
                cumulative_pnl_sol: cumulative,
                round_pnl_sol: round.pnl_sol,
                mint: round.mint.clone(),
            }
        })
        .collect();

    let mut exits: BTreeMap<String, ExitBucket> = BTreeMap::new();
    match book {
        CopyBook::Paper => {
            for leg in legs.iter().filter(|leg| range.contains(leg.at)) {
                let bucket = exits.entry(leg.exit.clone()).or_default();
                bucket.legs += 1;
                bucket.pnl_sol += leg.pnl_sol;
            }
        }
        CopyBook::Live => {
            for round in &rounds {
                let bucket = exits.entry(round.exit.clone()).or_default();
                bucket.legs += 1;
                bucket.pnl_sol += round.pnl_sol;
            }
        }
    }
    insights.exit_breakdown = exits
        .into_iter()
        .map(|(exit, bucket)| ExitBucket { exit, ..bucket })
        .collect();

    let mut skips: HashMap<String, usize> = HashMap::new();
    let mut arrival = Vec::new();
    let mut slippage = Vec::new();
    for row in rows.iter().filter(|row| range.contains(row.created_at)) {
        match &row.outcome {
            CopyOutcome::PaperFilled(_)
            | CopyOutcome::LiveSubmitted(_)
            | CopyOutcome::LiveConfirmed(_) => insights.decisions.fills += 1,
            CopyOutcome::PaperSellObserved(_) | CopyOutcome::LiveSellSubmitted(_) => {
                insights.decisions.exits += 1
            }
            CopyOutcome::LiveFailed(_) | CopyOutcome::LiveSellFailed(_) => {
                insights.decisions.errors += 1
            }
            CopyOutcome::Skipped { reason, .. } => {
                insights.decisions.skips += 1;
                *skips.entry(skip_key(reason)).or_default() += 1;
            }
        }
        if let Some(distance) = row
            .outcome
            .telemetry()
            .filter(|telemetry| !telemetry.backfill)
            .and_then(arrival_distance_ms)
        {
            arrival.push(distance);
        }
        if let Some(pct) = slippage_pct(&row.outcome) {
            slippage.push(pct);
        }
    }
    let mut skip_breakdown = skips
        .into_iter()
        .map(|(key, count)| CountBucket { key, count })
        .collect::<Vec<_>>();
    skip_breakdown.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    insights.skip_breakdown = skip_breakdown;
    insights.arrival_histogram = latency_histogram(&arrival);
    insights.arrival = summarize_arrival_distances(arrival);
    insights.slippage = summarize_slippage(slippage);
    insights.recent_rounds = rounds.iter().rev().take(RECENT_ROUNDS).cloned().collect();
    insights
}

#[cfg(test)]
#[path = "insights_tests.rs"]
mod tests;
