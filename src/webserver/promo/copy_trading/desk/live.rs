// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Synthetic live-copy history and corresponding promotional position rows.

use chrono::{DateTime, Duration, Utc};

use crate::chains::RawAmount;
use crate::positions::{Position, PositionOrigin, PriceSource};
use crate::trader::copy::{management_for_exit_mode, CopyOutcome, CopyTask, LiveDecision};

use super::super::super::aggregates::{closed_exit_offset_hours, closed_hold_minutes};
use super::super::super::data::{PROMO_CLOSED_TOKENS, PROMO_OPEN_TOKENS};
use super::{arrival_ms, observed, signature, target_trade_sol, Decisions};

/// Real positions each live task opened: (task id, `PROMO_OPEN_TOKENS` indices,
/// `PROMO_CLOSED_TOKENS` indices). They are positions the Positions page already
/// lists, each sized by its task's rule, closed only for a reason the task's exit
/// mode allows, and opened inside the task's lifetime.
pub(super) const LIVE_POSITIONS: [(i64, &[usize], &[usize]); 2] =
    [(1, &[0, 1], &[0, 3, 5]), (3, &[2, 4], &[1, 6, 9, 18])];

/// A real position a live task opened.
struct LiveRound<'a> {
    mint: &'a str,
    symbol: &'a str,
    name: &'a str,
    entry: f64,
    size: f64,
    entry_time: DateTime<Utc>,
    /// (exit price, exit time, closed reason) once it has closed.
    close: Option<(f64, DateTime<Utc>, &'a str)>,
    /// The current price while it is open.
    current: Option<f64>,
}

/// The position as the positions store holds it; its figures match the Positions
/// page's row for the same token.
fn live_position(task: &CopyTask, round: &LiveRound, now: DateTime<Utc>) -> Position {
    let tokens =
        RawAmount::from_integral_f64((round.size / round.entry * 1e9).trunc()).unwrap_or_default();
    let exit_price = round.close.map(|(price, ..)| price);
    let exit_time = round.close.map(|(_, at, _)| at);
    let move_pct = |price: f64| (price - round.entry) / round.entry * 100.0;
    let pnl = exit_price.map(|price| move_pct(price) / 100.0 * round.size);
    let unrealized = round
        .current
        .map(|price| move_pct(price) / 100.0 * round.size);
    let reference = exit_price.or(round.current).unwrap_or(round.entry);
    let peak = reference.max(round.entry) * 1.03;
    Position {
        id: None,
        mint: round.mint.to_owned(),
        symbol: round.symbol.to_owned(),
        name: round.name.to_owned(),
        entry_price: round.entry,
        entry_time: round.entry_time,
        exit_price,
        exit_time,
        position_type: "long".to_owned(),
        entry_size_native: round.size,
        total_size_native: round.size,
        price_highest: peak,
        price_lowest: reference.min(round.entry) * 0.97,
        basis_peak_price: peak,
        basis_peak_restarted: false,
        entry_transaction_signature: Some(signature("entry", task.id, 0, round.mint)),
        exit_transaction_signature: exit_time.map(|_| signature("exit", task.id, 0, round.mint)),
        token_amount: Some(tokens),
        effective_entry_price: Some(round.entry),
        effective_exit_price: exit_price,
        native_received: pnl.map(|pnl| round.size + pnl),
        profit_target_min: Some(15.0),
        profit_target_max: Some(50.0),
        liquidity_tier: Some("high".to_owned()),
        transaction_entry_verified: true,
        transaction_exit_verified: exit_time.is_some(),
        entry_fee_raw: Some(5000),
        exit_fee_raw: exit_time.map(|_| 5000),
        current_price: round.current,
        current_price_updated: round.current.map(|_| now),
        current_price_source: round.current.map(|_| PriceSource::Pool),
        phantom_remove: false,
        phantom_confirmations: 0,
        phantom_first_seen: None,
        synthetic_exit: false,
        closed_reason: round.close.map(|(.., reason)| reason.to_owned()),
        pnl,
        pnl_percent: exit_price.map(move_pct),
        unrealized_pnl: unrealized,
        unrealized_pnl_percent: round.current.map(move_pct),
        remaining_token_amount: round.current.map(|_| tokens),
        total_exited_amount: if exit_time.is_some() {
            tokens
        } else {
            RawAmount::ZERO
        },
        average_exit_price: exit_price,
        partial_exit_count: 0,
        take_profit_level: 0,
        entry_liquidity_native: None,
        entry_liquidity_pool: None,
        dca_count: 0,
        average_entry_price: round.entry,
        last_dca_time: None,
        archived: false,
        archived_at: None,
        origin: PositionOrigin::Copy {
            task_id: task.id,
            source_wallet: task.target_address.clone(),
        },
        management: management_for_exit_mode(task.exit_mode),
        round_key: None,
        basis_complete: true,
        history_complete: true,
        holding_state: None,
    }
}

/// The copy that opened a live position, confirmed at its entry time.
fn live_buy(task: &CopyTask, round: &LiveRound, n: usize, decisions: &mut Decisions) {
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
        LiveRound {
            mint,
            symbol,
            name,
            entry,
            size,
            entry_time: now - Duration::minutes(hold_minutes),
            close: None,
            current: Some(current),
        }
    });
    let closed_rounds = closed.iter().map(|&index| {
        let (symbol, name, mint, _logo, entry, exit, size, reason) = PROMO_CLOSED_TOKENS[index];
        let exit_time = now - Duration::hours(closed_exit_offset_hours(index));
        LiveRound {
            mint,
            symbol,
            name,
            entry,
            size,
            entry_time: exit_time - Duration::minutes(closed_hold_minutes(index)),
            close: Some((exit, exit_time, reason)),
            current: None,
        }
    });
    let mut spent = 0.0;
    for (n, round) in open_rounds.chain(closed_rounds).enumerate() {
        live_buy(task, &round, n, decisions);
        positions.push(live_position(task, &round, now));
        spent += round.size;
    }
    spent
}
