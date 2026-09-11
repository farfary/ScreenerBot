//! The promo copy desk: three copied wallets and everything the product records
//! for them -- decisions, the paper ledger and the real positions the live tasks
//! opened. It is built per request, so every Copy Trading endpoint, the Positions
//! page and the Watched tab read one session.
//!
//! Nothing here states a figure the product derives. Fills and sells go through
//! the real paper simulators, and the endpoints run the real stats, rounds,
//! insights and workspace assembly over the desk.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};

use crate::chains::active_chain;
use crate::positions::{Position, PositionManagement, PositionOrigin};
use crate::trader::copy::{
    management_for_exit_mode, simulate_fill, simulate_sell, CopyActivityRow, CopyMode, CopyOutcome,
    CopySellDecision, CopySkip, CopyTask, CopyTelemetry, ExitMode, LiveDecision, PaperCosts,
    PaperDecision, PaperExitRule, PaperFill, PaperPosition, SizingMode, TargetObservations,
};
use crate::trader::policy::{
    ExitPolicyOverrides, RoiPolicyOverrides, StopLossPolicyOverrides, TimePolicyOverrides,
    TrailingPolicyOverrides,
};

use super::super::aggregates::{closed_exit_offset_hours, closed_hold_minutes};
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

const COSTS: PaperCosts = PaperCosts {
    network_fee_sol: 0.000_005,
    priority_fee_sol: 0.000_1,
};

/// Real positions each live task opened: (task id, `PROMO_OPEN_TOKENS` indices,
/// `PROMO_CLOSED_TOKENS` indices). They are positions the Positions page already
/// lists, each sized by its task's rule, closed only for a reason the task's exit
/// mode allows, and opened inside the task's lifetime.
const LIVE_POSITIONS: [(i64, &[usize], &[usize]); 2] =
    [(1, &[0, 1], &[0, 3, 5]), (3, &[2, 4], &[1, 6, 9, 18])];

/// The paper task's closed rounds: (`PROMO_CLOSED_TOKENS` index for the mint and
/// entry price, price move at the exit in %, the rule that sold it -- `None` is a
/// mirrored wallet sell -- hours since the close, minutes held). Each rule exit
/// sits where `paper_overrides` fires.
const PAPER_ROUNDS: [(usize, f64, Option<PaperExitRule>, i64, i64); 12] = [
    (2, 30.6, Some(PaperExitRule::TakeProfit), 5, 95),
    (4, 24.0, None, 11, 140),
    (17, -15.4, Some(PaperExitRule::StopLoss), 20, 45),
    (5, 13.1, Some(PaperExitRule::TrailingStop), 29, 210),
    (7, 31.2, Some(PaperExitRule::TakeProfit), 41, 75),
    (8, 9.0, None, 55, 320),
    (19, -3.1, Some(PaperExitRule::TimeOverride), 70, 360),
    (9, 18.4, Some(PaperExitRule::TrailingStop), 86, 130),
    (10, 27.0, None, 104, 190),
    (21, -15.8, Some(PaperExitRule::StopLoss), 130, 60),
    (11, 30.4, Some(PaperExitRule::TakeProfit), 170, 110),
    (13, 6.5, None, 196, 400),
];

/// The paper task's open holdings: (`PROMO_OPEN_TOKENS` index, minutes held,
/// move since the buy in %, peak above the current price in %). One has armed
/// its trailing stop, one has not, one is under water.
const PAPER_HOLDINGS: [(usize, i64, f64, f64); 3] =
    [(6, 41, 18.0, 2.0), (7, 78, 7.0, 1.0), (8, 33, -4.0, 0.0)];

/// Rules the paper task runs under instead of the Trader's.
fn paper_overrides() -> ExitPolicyOverrides {
    ExitPolicyOverrides {
        stop_loss: StopLossPolicyOverrides {
            enabled: Some(true),
            threshold_pct: Some(15.0),
            ..Default::default()
        },
        trailing: TrailingPolicyOverrides {
            enabled: Some(true),
            activation_pct: Some(12.0),
            distance_pct: Some(5.0),
        },
        roi: RoiPolicyOverrides {
            enabled: Some(true),
            target_profit_pct: Some(30.0),
        },
        time: TimePolicyOverrides {
            enabled: Some(true),
            loss_threshold_pct: Some(0.0),
            duration_seconds: Some(14_400.0),
        },
    }
}

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
        max_sol_per_trade: 0.1,
        max_sol_per_token: 0.3,
        total_budget_sol: 1.0,
        min_target_trade_sol: Some(0.5),
        max_target_trade_sol: Some(40.0),
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
            max_sol_per_trade: 0.25,
            max_sol_per_token: 0.5,
            total_budget_sol: 6.0,
            min_target_trade_sol: Some(2.0),
            ..base_task(whale, now)
        },
        CopyTask {
            exit_policy_overrides: paper_overrides(),
            total_budget_sol: 8.0,
            max_target_trade_sol: Some(25.0),
            ..base_task(scout, now)
        },
        CopyTask {
            mode: CopyMode::Live,
            sizing: SizingMode::Fixed { sol: 0.15 },
            exit_mode: ExitMode::BuyOnly,
            max_sol_per_trade: 0.15,
            max_sol_per_token: 0.45,
            total_budget_sol: 10.0,
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
    let owner = LIVE_POSITIONS
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
    let tokens = (round.size / round.entry * 1e9) as u64;
    let exit_price = round.close.map(|(price, ..)| price);
    let exit_time = round.close.map(|(_, at, _)| at);
    let move_pct = |price: f64| (price - round.entry) / round.entry * 100.0;
    let pnl = exit_price.map(|price| move_pct(price) / 100.0 * round.size);
    let unrealized = round
        .current
        .map(|price| move_pct(price) / 100.0 * round.size);
    let reference = exit_price.or(round.current).unwrap_or(round.entry);
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
        entry_size_sol: round.size,
        total_size_sol: round.size,
        price_highest: reference.max(round.entry) * 1.03,
        price_lowest: reference.min(round.entry) * 0.97,
        entry_transaction_signature: Some(signature("entry", task.id, 0, round.mint)),
        exit_transaction_signature: exit_time.map(|_| signature("exit", task.id, 0, round.mint)),
        token_amount: Some(tokens),
        effective_entry_price: Some(round.entry),
        effective_exit_price: exit_price,
        sol_received: pnl.map(|pnl| round.size + pnl),
        profit_target_min: Some(15.0),
        profit_target_max: Some(50.0),
        liquidity_tier: Some("high".to_owned()),
        transaction_entry_verified: true,
        transaction_exit_verified: exit_time.is_some(),
        entry_fee_lamports: Some(5000),
        exit_fee_lamports: exit_time.map(|_| 5000),
        current_price: round.current,
        current_price_updated: round.current.map(|_| now),
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
        total_exited_amount: if exit_time.is_some() { tokens } else { 0 },
        average_exit_price: exit_price,
        partial_exit_count: 0,
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

type Decisions = Vec<(DateTime<Utc>, CopyOutcome)>;

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
fn live_desk(
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

/// A copied paper buy at `market`; returns the fill and when it was decided.
fn paper_buy(
    task: &CopyTask,
    mint: &str,
    market: f64,
    size: f64,
    block: DateTime<Utc>,
    n: usize,
    decisions: &mut Decisions,
) -> Option<(PaperFill, DateTime<Utc>)> {
    let fill = simulate_fill(size, market, task.slippage_pct, COSTS).ok()?;
    let arrival = arrival_ms(task.id, n);
    let target_price = market * (1.0 + (n % 4) as f64 * 0.0015);
    let telemetry = observed(
        block,
        arrival,
        target_price,
        Some(fill.fill_price_sol),
        false,
    );
    let decided_at = telemetry.decided_at;
    let target_size = target_trade_sol(task, size);
    decisions.push((
        decided_at,
        CopyOutcome::PaperFilled(PaperDecision {
            task_id: task.id,
            target_address: task.target_address.clone(),
            signature: signature("buy", task.id, n, mint),
            mint: mint.to_owned(),
            target_size_sol: target_size,
            target_token_amount: target_size / target_price,
            sized_sol: size,
            fill: fill.clone(),
            telemetry,
        }),
    ));
    Some((fill, decided_at))
}

/// The paper task's rounds and holdings, booked the way the ledger books them;
/// returns its spend.
fn paper_desk(
    task: &CopyTask,
    now: DateTime<Utc>,
    decisions: &mut Decisions,
    book: &mut Vec<PaperPosition>,
) -> f64 {
    let SizingMode::Fixed { sol: size } = task.sizing else {
        return 0.0;
    };
    let mut spent = 0.0;
    for (n, (index, move_pct, rule, closed_hours, held_minutes)) in
        PAPER_ROUNDS.into_iter().enumerate()
    {
        let (_, _, mint, _, entry, ..) = PROMO_CLOSED_TOKENS[index];
        let closed_block = now - Duration::hours(closed_hours);
        let opened_block = closed_block - Duration::minutes(held_minutes);
        let Some((fill, opened_at)) =
            paper_buy(task, mint, entry, size, opened_block, n, decisions)
        else {
            continue;
        };
        spent += size;
        let exit = entry * (1.0 + move_pct / 100.0);
        let Ok(sell) = simulate_sell(fill.token_amount, exit, task.slippage_pct, COSTS) else {
            continue;
        };
        let (telemetry, target_signature, target_tokens, target_sol) = match rule {
            None => {
                let target_tokens = target_trade_sol(task, size) / entry;
                (
                    observed(
                        closed_block,
                        arrival_ms(task.id, n + PAPER_ROUNDS.len()),
                        exit,
                        Some(sell.fill_price_sol),
                        false,
                    ),
                    signature("sell", task.id, n, mint),
                    target_tokens,
                    target_tokens * exit,
                )
            }
            Some(_) => (
                rule_exit(closed_block, sell.fill_price_sol),
                format!("paper-exit:{mint}:{}:0", opened_at.timestamp()),
                0.0,
                0.0,
            ),
        };
        let closed_at = telemetry.decided_at;
        book.push(PaperPosition {
            task_id: task.id,
            mint: mint.to_owned(),
            token_amount: 0.0,
            cost_basis_sol: 0.0,
            invested_sol: fill.total_cost_sol,
            realized_proceeds_sol: sell.net_proceeds_sol,
            realized_cost_sol: fill.total_cost_sol,
            buys: 1,
            sells: 1,
            last_price_sol: Some(exit),
            last_price_at: Some(closed_at),
            opened_at,
            closed_at: Some(closed_at),
            peak_price_sol: None,
        });
        decisions.push((
            closed_at,
            CopyOutcome::PaperSellObserved(CopySellDecision {
                task_id: task.id,
                target_address: task.target_address.clone(),
                target_signature,
                mint: mint.to_owned(),
                target_token_amount: target_tokens,
                target_sol_amount: target_sol,
                exit_percentage: None,
                transaction_signature: None,
                error: None,
                telemetry,
                paper_fill: Some(sell),
                exit_rule: rule,
            }),
        ));
    }
    for (n, (index, held_minutes, move_pct, peak_pct)) in PAPER_HOLDINGS.into_iter().enumerate() {
        let (_, _, mint, _, _, current, ..) = PROMO_OPEN_TOKENS[index];
        let market = current / (1.0 + move_pct / 100.0);
        let block = now - Duration::minutes(held_minutes);
        let n = PAPER_ROUNDS.len() * 2 + n;
        let Some((fill, opened_at)) = paper_buy(task, mint, market, size, block, n, decisions)
        else {
            continue;
        };
        spent += size;
        book.push(PaperPosition {
            task_id: task.id,
            mint: mint.to_owned(),
            token_amount: fill.token_amount,
            cost_basis_sol: fill.total_cost_sol,
            invested_sol: fill.total_cost_sol,
            realized_proceeds_sol: 0.0,
            realized_cost_sol: 0.0,
            buys: 1,
            sells: 0,
            last_price_sol: Some(current),
            last_price_at: Some(now),
            opened_at,
            closed_at: None,
            peak_price_sol: Some((current * (1.0 + peak_pct / 100.0)).max(fill.fill_price_sol)),
        });
    }
    spent
}

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
                CopyMode::Live => live_desk(task, now, &mut decisions, &mut positions),
                CopyMode::Paper => paper_desk(task, now, &mut decisions, &mut paper_book),
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
