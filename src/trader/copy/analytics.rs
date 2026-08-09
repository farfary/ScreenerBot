//! Pure copy-trading analytics and latency policy.

use crate::positions::{Position, PositionOrigin};

use super::types::{
    ArrivalDistanceStats, CopyActivityRow, CopyOutcome, CopyTaskStats, CopyTelemetry,
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

pub fn build_task_stats(
    task_id: i64,
    activity: &[CopyActivityRow],
    positions: &[Position],
) -> CopyTaskStats {
    let mut stats = CopyTaskStats {
        task_id,
        ..CopyTaskStats::default()
    };
    let mut arrival = Vec::new();
    for row in activity.iter().filter(|row| row.task_id == task_id) {
        stats.decisions += 1;
        let telemetry = match &row.outcome {
            CopyOutcome::PaperFilled(decision) => {
                stats.filled_buys += 1;
                Some(&decision.telemetry)
            }
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
            CopyOutcome::PaperSellObserved(decision) => {
                stats.observed_sells += 1;
                Some(&decision.telemetry)
            }
            CopyOutcome::LiveSellSubmitted(decision) => {
                stats.observed_sells += 1;
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
        if let Some(distance) = telemetry.and_then(arrival_distance_ms) {
            arrival.push(distance);
        }
    }

    for position in positions.iter().filter(|position| {
        matches!(position.origin, PositionOrigin::Copy { task_id: origin, .. } if origin == task_id)
    }) {
        if position.transaction_exit_verified {
            stats.closed_positions += 1;
            stats.realized_pnl_sol += position.pnl.unwrap_or_default();
        } else if !position.archived {
            stats.open_positions += 1;
            stats.unrealized_pnl_sol += position.unrealized_pnl.unwrap_or_default();
        }
    }
    stats.arrival_distance = summarize_arrival_distances(arrival);
    stats
}
