// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Pure copy-trading analytics and latency policy.

use crate::positions::{Book, Position, PositionOrigin};
use crate::trader::TradeReason;

use super::types::{
    ArrivalDistanceStats, CopyActivityRow, CopyBook, CopyOutcome, CopyTaskStats, CopyTelemetry,
};

pub fn arrival_distance_ms(telemetry: &CopyTelemetry) -> Option<u64> {
    let block_time = telemetry.target_block_time?;
    let detected_ms = telemetry.detected_at.timestamp_millis();
    let block_ms = block_time.checked_mul(1_000)?;
    u64::try_from(detected_ms.saturating_sub(block_ms)).ok()
}

pub fn summarize_arrival_distances(mut samples: Vec<u64>) -> ArrivalDistanceStats {
    if samples.is_empty() {
        return ArrivalDistanceStats::default();
    }
    samples.sort_unstable();
    let count = samples.len();
    let percentile = |numerator: usize, denominator: usize| {
        let rank = count.saturating_mul(numerator).div_ceil(denominator);
        samples[rank.saturating_sub(1).min(count - 1)]
    };
    ArrivalDistanceStats {
        samples: count,
        minimum_ms: samples.first().copied(),
        median_ms: Some(percentile(50, 100)),
        p95_ms: Some(percentile(95, 100)),
        maximum_ms: samples.last().copied(),
        average_ms: Some(
            (samples.iter().map(|value| u128::from(*value)).sum::<u128>() / count as u128) as u64,
        ),
    }
}

pub fn latency_should_pause(samples: &[u64], window_size: usize, threshold_ms: u64) -> bool {
    if samples.len() < window_size || window_size == 0 {
        return false;
    }
    let window = &samples[samples.len() - window_size..];
    let average = window.iter().map(|value| u128::from(*value)).sum::<u128>() / window_size as u128;
    average > u128::from(threshold_ms)
}

/// How a closed round of a copy task closed, from its stored close reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoundClose {
    /// An exit rule of the exit monitor sold it.
    Policy,
    /// It was closed by hand.
    Manual,
    /// A mirrored target sell, a paper run reset or a reconciliation closed it.
    Other,
}

/// The kind of close a stored `closed_reason` names; a reason a pending verification
/// or a forced close decorates keeps its kind.
fn round_close(closed_reason: Option<&str>) -> RoundClose {
    let Some(reason) = closed_reason else {
        return RoundClose::Other;
    };
    let reason = reason.trim_end_matches("_pending_verification");
    let is = |trade: TradeReason| reason == format!("{trade:?}");
    if is(TradeReason::ManualExit) || is(TradeReason::ForceSell) {
        RoundClose::Manual
    } else if [
        TradeReason::TakeProfit,
        TradeReason::StopLoss,
        TradeReason::TrailingStop,
        TradeReason::BreakEvenStop,
        TradeReason::TimeOverride,
        TradeReason::TimeStagnation,
        TradeReason::StrategyExit,
        TradeReason::LlmAnalysisExit,
        TradeReason::RiskManagement,
        TradeReason::Blacklisted,
    ]
    .into_iter()
    .any(is)
    {
        RoundClose::Policy
    } else {
        RoundClose::Other
    }
}

/// A task's figures in `book`: decision counts and arrival from its `activity`, and
/// the rounds it opened in that book from `positions`. A live round counts closed
/// once its exit is verified; a paper round once it closes, and an open paper round
/// without a mark is counted as unpriced rather than at zero.
pub fn build_task_stats(
    task_id: i64,
    book: CopyBook,
    activity: &[CopyActivityRow],
    positions: &[Position],
) -> CopyTaskStats {
    let mut stats = CopyTaskStats {
        task_id,
        book,
        ..CopyTaskStats::default()
    };
    let mut arrival = Vec::new();
    for row in activity.iter().filter(|row| row.task_id == task_id) {
        stats.decisions += 1;
        let telemetry = match &row.outcome {
            CopyOutcome::PaperFilled(decision) if decision.fill.has_quote_provenance() => {
                stats.filled_buys += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::PaperFilled(_) => None,
            CopyOutcome::LiveSubmitted(decision) => {
                stats.filled_buys += 1;
                stats.submitted += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::LiveConfirmed(decision) => {
                stats.filled_buys += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::LiveFailed(decision) => {
                stats.failed += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::PaperSellObserved(decision)
                if decision
                    .paper_fill
                    .as_ref()
                    .is_some_and(|fill| fill.has_quote_provenance()) =>
            {
                if decision.exit_rule.is_none() {
                    stats.target_sells += 1;
                }
                Some(&decision.telemetry)
            }
            CopyOutcome::PaperSellObserved(_) => None,
            CopyOutcome::LiveSellSubmitted(decision) => {
                stats.target_sells += 1;
                stats.submitted += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::LiveSellFailed(decision) => {
                stats.failed += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::Skipped { telemetry, .. } => {
                stats.skipped += 1;
                telemetry.as_ref()
            }
        };
        // Backfilled trades arrive by design long after their block; like the
        // latency kill switch, the arrival figures measure the live stream only.
        if let Some(distance) = telemetry
            .filter(|telemetry| !telemetry.backfill)
            .and_then(arrival_distance_ms)
        {
            arrival.push(distance);
        }
    }

    let store = book.store();
    for position in positions.iter().filter(|position| {
        matches!(position.origin, PositionOrigin::Copy { task_id: origin, .. } if origin == task_id)
            && position.book == store
    }) {
        let closed = match store {
            Book::Live => position.transaction_exit_verified,
            Book::Paper => position.exit_time.is_some(),
        };
        if closed {
            stats.closed_positions += 1;
            stats.realized_pnl_native += position.pnl.unwrap_or_default();
            match round_close(position.closed_reason.as_deref()) {
                RoundClose::Policy => stats.policy_exits += 1,
                RoundClose::Manual => stats.manual_closes += 1,
                RoundClose::Other => {}
            }
        } else if !position.archived {
            stats.open_positions += 1;
            let priced = position
                .current_price
                .is_some_and(|price| price.is_finite() && price > 0.0);
            if store == Book::Paper && !priced {
                stats.unpriced_positions += 1;
            } else {
                stats.unrealized_pnl_native += position.unrealized_pnl.unwrap_or_default();
            }
        }
    }
    stats.arrival_distance = summarize_arrival_distances(arrival);
    stats
}

#[cfg(test)]
#[path = "analytics_tests.rs"]
mod tests;
