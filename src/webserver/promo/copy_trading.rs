//! Promo generator for the Copy Trading workspace.
//!
//! Copy Trading is the one Auto Trader tab whose whole surface is a list: with no
//! tasks the page renders its onboarding panel instead, so a fresh capture shows
//! "Add your first wallet" rather than the feature. These tasks describe the same
//! session every other fixture describes — they buy the tokens `data.rs` already
//! holds open positions in, and the paper task is deliberately the one carrying
//! the larger budget, because Paper first then Arm is the flow the product
//! enforces.

use chrono::{Duration, Utc};

use crate::chains::ChainId;
use crate::trader::copy::{
    ArrivalDistanceStats, CopyActivityRow, CopyMode, CopyOutcome, CopySkip, CopyTask,
    CopyTaskStats, CopyTelemetry, ExitMode, PaperDecision, PaperFill, SizingMode,
};
use crate::webserver::routes::copy_trading::{OverviewResponse, StatusResponse, TaskSummary};

use super::data::PROMO_OPEN_TOKENS;

/// One copied wallet: (id, label, address, live, sizing SOL, budget, filled buys,
/// open positions, closed positions, mint index).
///
/// Fills and positions are stated, not derived, because they have to reconcile
/// with the rest of the session: the six open copy positions are a subset of the
/// ten `PROMO_OPEN_TOKENS` positions every other endpoint reports, and spend is
/// `filled_buys * sizing` because each task sizes every entry the same.
type PromoTask = (
    i64,
    &'static str,
    &'static str,
    bool,
    f64,
    f64,
    usize,
    usize,
    usize,
    usize,
);

const PROMO_TASKS: &[PromoTask] = &[
    (
        1,
        "Whale · early rotations",
        "GDfnEsia2WLAW5t8yx2X5j2mkfA74i5kY9dGZZ2q5wG7",
        true,
        0.25,
        6.0,
        8,
        3,
        2,
        0,
    ),
    (
        2,
        "Launch sniper (paper)",
        "5tzFkiKscXHK5ZXCGbXZxdw7gTjjD1mBwuoFbhUvuAi9",
        false,
        0.1,
        8.0,
        12,
        2,
        1,
        1,
    ),
    (
        3,
        "Momentum desk",
        "3nMFwZXwY1s1M5s8vYAHqd4wGs4iSxXE4LRoUMMYqEgF",
        true,
        0.4,
        10.0,
        9,
        1,
        2,
        2,
    ),
];

/// Telemetry for a decision that landed `arrival_ms` after the target's block.
fn telemetry(age_secs: i64, arrival_ms: i64, price_sol: f64) -> CopyTelemetry {
    let decided_at = Utc::now() - Duration::seconds(age_secs);
    let detected_at = decided_at - Duration::milliseconds(arrival_ms);
    CopyTelemetry {
        target_block_time: Some(detected_at.timestamp()),
        detected_at,
        decoded_at: decided_at - Duration::milliseconds(arrival_ms / 4),
        decided_at,
        submitted_at: Some(decided_at + Duration::milliseconds(90)),
        confirmed_at: Some(decided_at + Duration::milliseconds(640)),
        target_price_sol: Some(price_sol),
        fill_price_sol: Some(price_sol * 1.004),
    }
}

/// SOL actually spent by a task: one sized entry per filled buy.
fn spent_sol(entry: &PromoTask) -> f64 {
    entry.6 as f64 * entry.4
}

fn task(entry: &PromoTask) -> CopyTask {
    let (id, label, address, live, size_sol, budget, ..) = *entry;
    let created_at = Utc::now() - Duration::days(id + 3);
    CopyTask {
        id,
        chain: ChainId::Solana,
        target_address: address.to_owned(),
        label: Some(label.to_owned()),
        enabled: true,
        mode: if live {
            CopyMode::Live
        } else {
            CopyMode::Paper
        },
        sizing: SizingMode::Fixed { sol: size_sol },
        exit_mode: if live {
            ExitMode::Mirror
        } else {
            ExitMode::BuyOnly
        },
        exit_policy_overrides: Default::default(),
        max_sol_per_trade: size_sol,
        max_sol_per_token: size_sol * 2.0,
        total_budget_sol: budget,
        min_target_trade_sol: Some(0.5),
        max_target_trade_sol: Some(40.0),
        buy_once_per_token: true,
        slippage_pct: 1.5,
        created_at,
        updated_at: created_at + Duration::hours(6),
    }
}

/// Stats consistent with the task's own budget and with the session's positions.
fn stats(entry: &PromoTask) -> CopyTaskStats {
    let (id, _label, _address, live, _size_sol, _budget, filled_buys, open, closed, _mint) = *entry;
    let spent = spent_sol(entry);
    let skipped = filled_buys / 2 + 1;
    CopyTaskStats {
        task_id: id,
        decisions: filled_buys + skipped,
        filled_buys,
        observed_sells: closed,
        skipped,
        submitted: if live { filled_buys } else { 0 },
        failed: 0,
        open_positions: open,
        closed_positions: closed,
        realized_pnl_sol: spent * 0.09,
        unrealized_pnl_sol: spent * 0.04,
        arrival_distance: ArrivalDistanceStats {
            samples: filled_buys,
            minimum_ms: Some(310),
            median_ms: Some(680),
            p95_ms: Some(1_240),
            maximum_ms: Some(1_910),
            average_ms: Some(742),
        },
    }
}

/// A short decision feed: one paper fill and one policy skip per task, newest
/// first, so the detail pane shows both halves of what the tab reports.
fn activity() -> Vec<CopyActivityRow> {
    let mut rows = Vec::new();
    let mut id = (PROMO_TASKS.len() * 2) as i64;

    for (index, entry) in PROMO_TASKS.iter().enumerate() {
        let (
            task_id,
            _label,
            address,
            _live,
            size_sol,
            _budget,
            _fills,
            _open,
            _closed,
            mint_index,
        ) = *entry;
        let token = &PROMO_OPEN_TOKENS[mint_index];
        let mint = token.2.to_owned();
        let price_sol = token.4;
        let fill_age = 240 + (index as i64) * 130;

        let fill_telemetry = telemetry(fill_age, 680, price_sol);
        rows.push(CopyActivityRow {
            id,
            task_id,
            kind: "buy".to_owned(),
            created_at: fill_telemetry.decided_at,
            outcome: CopyOutcome::PaperFilled(PaperDecision {
                task_id,
                target_address: address.to_owned(),
                signature: format!("PromoCopyFill{task_id:0>8}"),
                mint: mint.clone(),
                target_size_sol: size_sol * 12.0,
                target_token_amount: size_sol * 12.0 / price_sol,
                sized_sol: size_sol,
                fill: PaperFill {
                    input_sol: size_sol,
                    market_price_sol: price_sol,
                    fill_price_sol: price_sol * 1.004,
                    token_amount: size_sol / (price_sol * 1.004),
                    referral_fee_sol: size_sol * 0.005,
                    network_fee_sol: 0.000005,
                    priority_fee_sol: 0.00012,
                    total_cost_sol: size_sol * 1.005 + 0.000125,
                },
                telemetry: fill_telemetry,
            }),
        });
        id -= 1;

        let skip_age = fill_age + 95;
        rows.push(CopyActivityRow {
            id,
            task_id,
            kind: "buy".to_owned(),
            created_at: Utc::now() - Duration::seconds(skip_age),
            outcome: CopyOutcome::Skipped {
                task_id,
                signature: format!("PromoCopySkip{task_id:0>8}"),
                mint: Some(mint),
                reason: CopySkip::TargetBelowMinimum { minimum_sol: 0.5 },
                decided_at: Utc::now() - Duration::seconds(skip_age),
                telemetry: Some(telemetry(skip_age, 540, price_sol)),
            },
        });
        id -= 1;
    }

    rows
}

/// Generate the Copy Trading overview: status header, task summaries, decision feed.
pub fn get_promo_copy_trading_overview() -> OverviewResponse {
    let tasks: Vec<CopyTask> = PROMO_TASKS.iter().map(task).collect();
    let live_tasks = tasks
        .iter()
        .filter(|task| task.mode == CopyMode::Live)
        .count();

    let status = StatusResponse {
        enabled: true,
        live_available: true,
        blocked_reason: None,
        default_mode: "paper".to_owned(),
        default_slippage_pct: 1.5,
        force_stop_blocks: true,
        total_tasks: tasks.len(),
        active_tasks: tasks.len(),
        paper_tasks: tasks.len() - live_tasks,
        live_tasks,
    };

    let summaries = PROMO_TASKS
        .iter()
        .zip(tasks)
        .map(|(entry, task)| {
            let spent = spent_sol(entry);
            TaskSummary {
                stats: stats(entry),
                spent_sol: spent,
                remaining_budget_sol: (task.total_budget_sol - spent).max(0.0),
                effective_state: if task.mode == CopyMode::Live {
                    "live"
                } else {
                    "paper"
                },
                task,
            }
        })
        .collect();

    OverviewResponse {
        status,
        tasks: summaries,
        activity: activity(),
    }
}
