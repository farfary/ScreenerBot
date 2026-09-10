//! Pure copy-trading analytics and latency policy.

use crate::positions::{Position, PositionOrigin};

use super::types::{
    ArrivalDistanceStats, CopyActivityRow, CopyBook, CopyOutcome, CopyTaskStats, CopyTelemetry,
    PaperPosition,
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

/// Replace the live-position figures with the task's paper book. `mark` prices an
/// open position; one it cannot price is counted as unpriced rather than as zero.
pub fn apply_paper_book(
    stats: &mut CopyTaskStats,
    positions: &[PaperPosition],
    mark: impl Fn(&PaperPosition) -> Option<f64>,
) {
    stats.book = CopyBook::Paper;
    stats.open_positions = 0;
    stats.closed_positions = 0;
    stats.realized_pnl_sol = 0.0;
    stats.unrealized_pnl_sol = 0.0;
    stats.unpriced_positions = 0;
    for position in positions {
        stats.realized_pnl_sol += position.realized_proceeds_sol - position.realized_cost_sol;
        if !position.is_open() {
            stats.closed_positions += 1;
            continue;
        }
        stats.open_positions += 1;
        match mark(position).filter(|price| price.is_finite() && *price > 0.0) {
            Some(price) => {
                stats.unrealized_pnl_sol += position.token_amount * price - position.cost_basis_sol
            }
            None => stats.unpriced_positions += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn position(
        mint: &str,
        tokens: f64,
        cost: f64,
        proceeds: f64,
        realized_cost: f64,
    ) -> PaperPosition {
        PaperPosition {
            task_id: 1,
            mint: mint.to_owned(),
            token_amount: tokens,
            cost_basis_sol: cost,
            invested_sol: cost + realized_cost,
            realized_proceeds_sol: proceeds,
            realized_cost_sol: realized_cost,
            buys: 1,
            sells: u64::from(proceeds > 0.0),
            last_price_sol: None,
            last_price_at: None,
            opened_at: Utc::now(),
            closed_at: (tokens == 0.0).then(Utc::now),
            peak_price_sol: None,
        }
    }

    #[test]
    fn the_paper_book_marks_open_positions_and_counts_unpriced_ones() {
        let mut stats = CopyTaskStats::default();
        let book = [
            position("priced", 100.0, 1.0, 0.5, 0.4),
            position("unpriced", 50.0, 0.5, 0.0, 0.0),
            position("closed", 0.0, 0.0, 2.0, 1.5),
        ];
        apply_paper_book(&mut stats, &book, |p| (p.mint == "priced").then_some(0.012));
        assert_eq!(stats.book, CopyBook::Paper);
        assert_eq!((stats.open_positions, stats.closed_positions), (2, 1));
        assert_eq!(stats.unpriced_positions, 1);
        assert!((stats.realized_pnl_sol - 0.6).abs() < 1e-12);
        assert!((stats.unrealized_pnl_sol - 0.2).abs() < 1e-12);
    }
}
