// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Synthetic live-copy history and corresponding promotional position rows.

use chrono::{DateTime, Duration, Utc};

use crate::positions::{Book, Position};
use crate::trader::copy::{CopyOutcome, CopyTask, LiveDecision};

use super::super::super::aggregates::{closed_exit_offset_hours, closed_hold_minutes};
use super::super::super::data::{PROMO_CLOSED_TOKENS, PROMO_OPEN_TOKENS};
use super::{arrival_ms, observed, round_position, signature, target_trade_sol, Decisions, Round};

/// Real positions each live task opened: (task id, `PROMO_OPEN_TOKENS` indices,
/// `PROMO_CLOSED_TOKENS` indices). They are positions the Positions page already
/// lists, each sized by its task's rule, closed only for a reason the task's exit
/// mode allows, and opened inside the task's lifetime.
pub(super) const LIVE_POSITIONS: [(i64, &[usize], &[usize]); 2] =
    [(1, &[0, 1], &[0, 3, 5]), (3, &[2, 4], &[1, 6, 9, 18])];

/// The copy that opened a live position, confirmed at its entry time.
fn live_buy(task: &CopyTask, round: &Round, n: usize, decisions: &mut Decisions) {
    let arrival = arrival_ms(task.id, n);
    let block = round.entry_time - Duration::milliseconds(arrival + 900);
    let target_price = round.entry / (1.002 + (n % 5) as f64 * 0.001);
    let telemetry = observed(block, arrival, target_price, Some(round.entry), true);
    let target_size = target_trade_sol(task, round.size);
    decisions.push((
        telemetry.decided_at,
        CopyOutcome::LiveConfirmed(LiveDecision {
            task_id: task.id,
            target_address: task.target_address.clone(),
            target_signature: signature("target", task.id, n, round.mint),
            mint: round.mint.to_owned(),
            target_size_sol: target_size,
            target_token_amount: target_size / target_price,
            sized_sol: round.size,
            transaction_signature: Some(signature("copy", task.id, n, round.mint)),
            error: None,
            telemetry,
        }),
    ));
}

/// A live task's positions and the copies that opened them; returns its spend.
pub(super) fn live_desk(
    task: &CopyTask,
    now: DateTime<Utc>,
    decisions: &mut Decisions,
    positions: &mut Vec<Position>,
) -> f64 {
    let Some((_, open, closed)) = LIVE_POSITIONS.iter().find(|(id, ..)| *id == task.id) else {
        return 0.0;
    };
    let open_rounds = open.iter().map(|&index| {
        let (symbol, name, mint, _logo, entry, current, size, hold_minutes) =
            PROMO_OPEN_TOKENS[index];
        Round {
            id: None,
            mint,
            symbol,
            name,
            entry,
            size,
            entry_time: now - Duration::minutes(hold_minutes),
            close: None,
            current: Some(current),
            peak: None,
        }
    });
    let closed_rounds = closed.iter().map(|&index| {
        let (symbol, name, mint, _logo, entry, exit, size, reason) = PROMO_CLOSED_TOKENS[index];
        let exit_time = now - Duration::hours(closed_exit_offset_hours(index));
        Round {
            id: None,
            mint,
            symbol,
            name,
            entry,
            size,
            entry_time: exit_time - Duration::minutes(closed_hold_minutes(index)),
            close: Some((exit, exit_time, reason.to_owned())),
            current: None,
            peak: None,
        }
    });
    let mut spent = 0.0;
    for (n, round) in open_rounds.chain(closed_rounds).enumerate() {
        live_buy(task, &round, n, decisions);
        positions.push(round_position(task, &round, Book::Live, now));
        spent += round.size;
    }
    spent
}
