// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Deterministic, quote-backed promotional paper fills and holdings.

use chrono::{DateTime, Duration, Utc};

use crate::chains::{adapter_for, RawAmount};
use crate::swaps::{Quote, QuoteRequest, SwapMode};
use crate::trader::copy::{
    settle_quote_samples, CopyOutcome, CopySellDecision, CopyTask, PaperDecision, PaperExitRule,
    PaperFill, PaperPosition, SettledPaperQuote, SizingMode,
};
use crate::trader::policy::{
    ExitPolicyOverrides, RoiPolicyOverrides, StopLossPolicyOverrides, TimePolicyOverrides,
    TrailingPolicyOverrides,
};

use super::super::super::data::{PROMO_CLOSED_TOKENS, PROMO_OPEN_TOKENS};
use super::{arrival_ms, observed, rule_exit, signature, target_trade_sol, Decisions};

const PROMO_TOKEN_DECIMALS: u8 = 6;
const PROMO_NETWORK_FEE_LAMPORTS: u64 = 5_000;

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
pub(super) fn paper_overrides() -> ExitPolicyOverrides {
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

fn fixture_raw_amount(units: f64, decimals: u8) -> Option<RawAmount> {
    RawAmount::from_integral_f64((units * 10_f64.powi(i32::from(decimals))).floor())
        .filter(|amount| *amount > RawAmount::ZERO)
}

fn fixture_quote(
    request: &QuoteRequest,
    output_amount: RawAmount,
    platform_fee_lamports: u64,
) -> Quote {
    Quote {
        chain: request.chain,
        router_id: "promo-fixture".to_owned(),
        router_name: "Promo fixture".to_owned(),
        input_mint: request.input_mint.clone(),
        output_mint: request.output_mint.clone(),
        input_amount: request.input_amount,
        output_amount,
        minimum_output_amount: output_amount,
        price_impact_pct: 0.0,
        platform_fee_lamports: Some(platform_fee_lamports),
        estimated_network_fee_lamports: Some(PROMO_NETWORK_FEE_LAMPORTS),
        slippage_bps: (request.slippage_pct * 100.0).round() as u16,
        route_plan: "promo-fixture".to_owned(),
        swap_mode: request.swap_mode,
        wallet_address: request.wallet_address.clone(),
        exclude_dexes: request.exclude_dexes.clone(),
        execution_data: Vec::new(),
    }
}

fn fixture_quote_request(
    task: &CopyTask,
    input_mint: &str,
    output_mint: &str,
    input_amount: RawAmount,
) -> QuoteRequest {
    QuoteRequest {
        chain: task.chain,
        input_mint: input_mint.to_owned(),
        output_mint: output_mint.to_owned(),
        input_amount,
        wallet_address: "promo-fixture".to_owned(),
        slippage_pct: task.slippage_pct,
        swap_mode: SwapMode::ExactIn,
        exclude_dexes: None,
    }
}

fn paper_buy(
    task: &CopyTask,
    mint: &str,
    market: f64,
    size: f64,
    block: DateTime<Utc>,
    n: usize,
    decisions: &mut Decisions,
) -> Option<(PaperFill, DateTime<Utc>)> {
    let adapter = adapter_for(task.chain);
    let input_amount = fixture_raw_amount(size, adapter.native_asset_decimals())?;
    let fee_fraction = f64::from(adapter.platform_swap_fee_bps()) / 10_000.0;
    let expected_tokens = size / market * (1.0 - fee_fraction);
    let output_amount = fixture_raw_amount(expected_tokens, PROMO_TOKEN_DECIMALS)?;
    let request = fixture_quote_request(task, adapter.native_asset_address(), mint, input_amount);
    let platform_fee_lamports = u64::try_from(
        input_amount
            .raw()
            .checked_mul(u128::from(adapter.platform_swap_fee_bps()))?
            .checked_div(10_000)?,
    )
    .ok()?;
    let initial = fixture_quote(&request, output_amount, platform_fee_lamports);
    let landing = fixture_quote(&request, output_amount, platform_fee_lamports);
    let arrival = arrival_ms(task.id, n);
    let target_price = market * (1.0 + (n % 4) as f64 * 0.0015);
    let mut telemetry = observed(block, arrival, target_price, None, false);
    let initial_at = telemetry.decided_at;
    let landing_at = initial_at + Duration::seconds(1);
    let fill = match settle_quote_samples(
        &request,
        PROMO_TOKEN_DECIMALS,
        initial,
        landing,
        initial_at,
        landing_at,
        1000,
        0,
        0,
    )
    .ok()?
    {
        SettledPaperQuote::Buy(fill) => fill,
        SettledPaperQuote::Sell(_) => return None,
    };
    telemetry.fill_price_sol = Some(fill.fill_price_sol);
    telemetry.decided_at = landing_at;
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

fn paper_sell_fixture(
    task: &CopyTask,
    mint: &str,
    buy: &PaperFill,
    exit_price: f64,
    initial_at: DateTime<Utc>,
) -> Option<crate::trader::copy::PaperSellFill> {
    let adapter = adapter_for(task.chain);
    let input_amount = buy.quote.as_ref()?.final_output_amount;
    let fee_fraction = f64::from(adapter.platform_swap_fee_bps()) / 10_000.0;
    let output_amount = fixture_raw_amount(
        buy.token_amount * exit_price * (1.0 - fee_fraction),
        adapter.native_asset_decimals(),
    )?;
    let request = fixture_quote_request(task, mint, adapter.native_asset_address(), input_amount);
    let platform_fee_lamports = u64::try_from(
        output_amount
            .raw()
            .checked_mul(u128::from(adapter.platform_swap_fee_bps()))?
            .checked_div(10_000)?,
    )
    .ok()?;
    let initial = fixture_quote(&request, output_amount, platform_fee_lamports);
    let landing = fixture_quote(&request, output_amount, platform_fee_lamports);
    match settle_quote_samples(
        &request,
        PROMO_TOKEN_DECIMALS,
        initial,
        landing,
        initial_at,
        initial_at + Duration::seconds(1),
        1000,
        0,
        0,
    )
    .ok()?
    {
        SettledPaperQuote::Sell(fill) => Some(fill),
        SettledPaperQuote::Buy(_) => None,
    }
}

/// The paper task's rounds and holdings, booked the way the ledger books them;
/// returns its spend.
pub(super) fn paper_desk(
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
        spent += fill.total_cost_sol;
        let exit = entry * (1.0 + move_pct / 100.0);
        let initial_at = match rule {
            None => {
                closed_block
                    + Duration::milliseconds(arrival_ms(task.id, n + PAPER_ROUNDS.len()) + 120)
            }
            Some(_) => closed_block + Duration::milliseconds(250),
        };
        let Some(sell) = paper_sell_fixture(task, mint, &fill, exit, initial_at) else {
            continue;
        };
        let landing_at = sell
            .quote
            .as_ref()
            .expect("fixture sells retain quote provenance")
            .landing_at;
        let (telemetry, target_signature, target_tokens, target_sol) = match rule {
            None => {
                let mut telemetry = observed(
                    closed_block,
                    arrival_ms(task.id, n + PAPER_ROUNDS.len()),
                    exit,
                    Some(sell.fill_price_sol),
                    false,
                );
                telemetry.decided_at = landing_at;
                let target_tokens = target_trade_sol(task, size) / entry;
                (
                    telemetry,
                    signature("sell", task.id, n, mint),
                    target_tokens,
                    target_tokens * exit,
                )
            }
            Some(_) => {
                let mut telemetry = rule_exit(closed_block, sell.fill_price_sol);
                telemetry.decided_at = landing_at;
                (
                    telemetry,
                    format!("paper-exit:{mint}:{}:0", opened_at.timestamp()),
                    0.0,
                    0.0,
                )
            }
        };
        let closed_at = telemetry.decided_at;
        book.push(PaperPosition {
            task_id: task.id,
            mint: mint.to_owned(),
            token_amount: 0.0,
            raw_token_amount: Some(crate::chains::RawAmount::ZERO),
            token_decimals: sell.quote.as_ref().and_then(|quote| quote.token_decimals),
            cost_basis_native: 0.0,
            invested_native: fill.total_cost_sol,
            realized_proceeds_native: sell.net_proceeds_sol,
            realized_cost_native: fill.total_cost_sol,
            buys: 1,
            sells: 1,
            last_price_native: Some(exit),
            last_price_at: Some(closed_at),
            opened_at,
            closed_at: Some(closed_at),
            peak_price_native: None,
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
                paper_context: None,
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
        spent += fill.total_cost_sol;
        book.push(PaperPosition {
            task_id: task.id,
            mint: mint.to_owned(),
            token_amount: fill.token_amount,
            raw_token_amount: fill.quote.as_ref().map(|quote| quote.final_output_amount),
            token_decimals: fill.quote.as_ref().and_then(|quote| quote.token_decimals),
            cost_basis_native: fill.total_cost_sol,
            invested_native: fill.total_cost_sol,
            realized_proceeds_native: 0.0,
            realized_cost_native: 0.0,
            buys: 1,
            sells: 0,
            last_price_native: Some(current),
            last_price_at: Some(now),
            opened_at,
            closed_at: None,
            peak_price_native: Some((current * (1.0 + peak_pct / 100.0)).max(fill.fill_price_sol)),
        });
    }
    spent
}
