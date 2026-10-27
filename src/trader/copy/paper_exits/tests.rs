// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

#![cfg(test)]

//! Paper exit sizing, policy, and idempotency tests.

use super::*;

use chrono::Duration;

use crate::config::TakeProfitLevel;
use crate::trader::evaluators::dca::DcaConfigSnapshot;
use crate::trader::policy::{StopLossPolicy, TakeProfitPolicy, TimePolicy, TrailingPolicy};

fn task() -> CopyTask {
    CopyTask {
        id: 1,
        chain: crate::chains::ChainId::Solana,
        target_address: "target".to_owned(),
        label: None,
        enabled: true,
        mode: CopyMode::Paper,
        sizing: super::super::types::SizingMode::Fixed { sol: 0.1 },
        exit_mode: super::super::types::ExitMode::Hybrid,
        exit_policy_overrides: Default::default(),
        max_native_per_trade: 1.0,
        max_native_per_token: 2.0,
        total_budget_native: 5.0,
        min_target_trade_native: None,
        max_target_trade_native: None,
        buy_once_per_token: false,
        slippage_pct: 1.0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        require_filter_pass: None,
        copy_stable_quoted: None,
        pause_reason: None,
        paused_at: None,
    }
}

#[test]
fn policy_and_mirror_close_planning_keep_exact_raw_inputs() {
    let task = task();
    let full = paper_sell_request(
        &task,
        "mint",
        RawAmount::from(15_u64),
        None,
        "wallet".to_owned(),
    )
    .unwrap();
    assert_eq!(full.input_amount, RawAmount::from(15_u64));

    let partial = paper_sell_request(
        &task,
        "mint",
        RawAmount::from(17_u64),
        Some(50.0),
        "wallet".to_owned(),
    )
    .unwrap();
    assert_eq!(partial.input_amount, RawAmount::from(8_u64));
    assert_eq!(
        RawAmount::from(17_u64).checked_sub(partial.input_amount),
        Some(RawAmount::from(9_u64))
    );
    assert!(matches!(
        paper_sell_request(
            &task,
            "mint",
            RawAmount::from(15_u64),
            Some(1e-9),
            "wallet".to_owned()
        ),
        Err(CopySkip::InvalidSizing)
    ));
}

#[test]
fn partial_sell_sizing_preserves_large_and_full_width_raw_balances() {
    let above_f64_integer = RawAmount::new((1_u128 << 53) + 17);
    let selected = raw_sell_amount(above_f64_integer, Some(50.0)).unwrap();
    assert_eq!(selected, RawAmount::new(((1_u128 << 53) + 17) / 2));
    assert_eq!(
        raw_sell_amount(RawAmount::MAX, Some(50.0)).unwrap(),
        RawAmount::new(u128::MAX / 2)
    );
    assert_eq!(
        raw_sell_amount(RawAmount::MAX, None).unwrap(),
        RawAmount::MAX
    );
}

fn policy() -> ExitPolicy {
    ExitPolicy {
        stop_loss: StopLossPolicy {
            enabled: true,
            threshold_pct: 30.0,
            min_hold_seconds: 0,
            allow_partial: true,
            partial_exit_default_pct: 50.0,
        },
        trailing: TrailingPolicy {
            enabled: true,
            activation_pct: 20.0,
            distance_pct: 10.0,
        },
        take_profit: TakeProfitPolicy {
            enabled: true,
            levels: vec![TakeProfitLevel::sell_all_at(100.0)],
        },
        time: TimePolicy {
            enabled: true,
            loss_threshold_pct: -10.0,
            duration_seconds: 3600.0,
        },
        dca: DcaConfigSnapshot {
            enabled: false,
            max_count: 0,
            cooldown_minutes: 0,
            threshold_pct: 0.0,
            size_percentage: 0.0,
        },
    }
}

#[test]
fn policy_decision_signatures_distinguish_subsecond_rounds_and_epochs() {
    let opened_at = Utc::now();
    let base = PaperPosition {
        task_id: 1,
        mint: "mint".to_owned(),
        token_amount: 1.0,
        raw_token_amount: Some(RawAmount::from(1_u64)),
        token_decimals: Some(0),
        cost_basis_native: 1.0,
        invested_native: 1.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 1,
        sells: 0,
        last_price_native: Some(1.0),
        last_price_at: Some(opened_at),
        opened_at,
        closed_at: None,
        peak_price_native: Some(1.0),
        take_profit_level: 0,
        round_partial_exits: 0,
    };
    let mut next_round = base.clone();
    next_round.opened_at = opened_at + chrono::Duration::nanoseconds(1);
    assert_ne!(
        paper_exit_signature(&base, 3),
        paper_exit_signature(&next_round, 3)
    );
    assert_ne!(
        paper_exit_signature(&base, 3),
        paper_exit_signature(&base, 4)
    );
}

/// 100 tokens for 1 SOL: entry 0.01 SOL per token, in a round that booked `sells`
/// partial sales.
fn holding(sells: u64, age: Duration) -> PaperPosition {
    let now = Utc::now();
    PaperPosition {
        task_id: 1,
        mint: "mint".to_owned(),
        token_amount: 100.0,
        raw_token_amount: Some(RawAmount::from(100_u64)),
        token_decimals: Some(0),
        cost_basis_native: 1.0,
        invested_native: 1.0,
        realized_proceeds_native: 0.0,
        realized_cost_native: 0.0,
        buys: 1,
        sells,
        last_price_native: None,
        last_price_at: None,
        opened_at: now - age,
        closed_at: None,
        peak_price_native: Some(0.01),
        take_profit_level: 0,
        round_partial_exits: u32::try_from(sells).expect("a test round's sales fit a u32"),
    }
}

fn evaluate(
    position: &PaperPosition,
    mark: f64,
    peak: f64,
) -> Option<(PaperExitRule, Option<f64>)> {
    evaluate_paper_exit(1, position, mark, peak, &policy(), Utc::now())
        .map(|exit| (exit.rule, exit.exit_percentage))
}

#[test]
fn stop_loss_takes_one_partial_then_exits_fully() {
    let fresh = holding(0, Duration::seconds(60));
    assert_eq!(
        evaluate(&fresh, 0.0065, 0.01),
        Some((PaperExitRule::StopLoss, Some(50.0)))
    );
    let cut = holding(1, Duration::seconds(60));
    assert_eq!(
        evaluate(&cut, 0.0065, 0.01),
        Some((PaperExitRule::StopLoss, None))
    );
}

#[test]
fn trailing_stop_arms_off_the_peak_and_fires_only_in_profit() {
    let position = holding(0, Duration::seconds(60));
    // Peaked at +25%, retraced to +12%: under the stop (0.0125 * 0.9 = 0.01125).
    assert_eq!(
        evaluate(&position, 0.0112, 0.0125),
        Some((PaperExitRule::TrailingStop, None))
    );
    // Same retracement without the peak ever reaching activation: nothing.
    assert_eq!(evaluate(&position, 0.0112, 0.0115), None);
    // Retraced into a loss: the trail never sells at a loss.
    assert_eq!(evaluate(&position, 0.0095, 0.0125), None);
}

/// A holding that reopened keeps every earlier round's sales in `sells`, yet its own
/// round sold nothing: its stop loss takes the partial cut again.
#[test]
fn a_reopened_round_takes_its_own_partial_stop_loss() {
    let mut reopened = holding(0, Duration::seconds(60));
    reopened.sells = 3;
    assert_eq!(
        evaluate(&reopened, 0.0065, 0.01),
        Some((PaperExitRule::StopLoss, Some(50.0)))
    );
}

#[test]
fn take_profit_and_time_override_fire_in_monitor_order() {
    let young = holding(0, Duration::seconds(60));
    assert_eq!(
        evaluate(&young, 0.02, 0.02),
        Some((PaperExitRule::TakeProfit, None))
    );
    assert_eq!(evaluate(&young, 0.0085, 0.01), None);
    let old = holding(0, Duration::hours(2));
    assert_eq!(
        evaluate(&old, 0.0085, 0.01),
        Some((PaperExitRule::TimeOverride, None))
    );
}

#[test]
fn an_impossible_trailing_configuration_is_reported_not_traded() {
    let mut broken = policy();
    broken.trailing.distance_pct = 25.0;
    let position = holding(0, Duration::seconds(60));
    assert_eq!(
        evaluate_paper_exit(1, &position, 0.0105, 0.0125, &broken, Utc::now()),
        None
    );
}

#[test]
fn exit_legs_book_distinct_idempotent_signatures() {
    assert_ne!(
        paper_exit_signature(&holding(0, Duration::seconds(60)), 0),
        paper_exit_signature(&holding(1, Duration::seconds(60)), 0)
    );
}

/// An open live position entered at 0.01 like [`holding`], opened `age` before `now`,
/// with its basis peak at `peak` and `partial_exits` partial sales booked.
fn live_position(
    peak: f64,
    age: Duration,
    partial_exits: u32,
    now: DateTime<Utc>,
) -> crate::positions::Position {
    let mut position: crate::positions::Position = serde_json::from_str(
        r#"{
        "id": 1,
        "mint": "mint",
        "symbol": "TEST",
        "name": "Test",
        "entry_price": 0.01,
        "entry_time": "2026-10-01T00:00:00Z",
        "exit_price": null,
        "exit_time": null,
        "position_type": "buy",
        "entry_size_native": 1.0,
        "total_size_native": 1.0,
        "price_highest": 0.01,
        "price_lowest": 0.01,
        "basis_peak_price": 0.01,
        "entry_transaction_signature": "entry",
        "exit_transaction_signature": null,
        "token_amount": "100",
        "effective_entry_price": 0.01,
        "effective_exit_price": null,
        "native_received": null,
        "profit_target_min": null,
        "profit_target_max": null,
        "liquidity_tier": null,
        "transaction_entry_verified": true,
        "transaction_exit_verified": false,
        "entry_fee_raw": null,
        "exit_fee_raw": null,
        "current_price": null,
        "current_price_updated": null,
        "phantom_remove": false,
        "phantom_confirmations": 0,
        "phantom_first_seen": null,
        "synthetic_exit": false,
        "closed_reason": null,
        "pnl": null,
        "pnl_percent": null,
        "unrealized_pnl": null,
        "unrealized_pnl_percent": null,
        "remaining_token_amount": "100",
        "total_exited_amount": "0",
        "average_exit_price": null,
        "partial_exit_count": 0,
        "dca_count": 0,
        "average_entry_price": 0.01,
        "last_dca_time": null,
        "origin": { "kind": "auto", "strategy_id": null },
        "management": "auto_trader"
    }"#,
    )
    .expect("position fixture deserializes");
    position.basis_peak_price = peak;
    position.price_highest = peak;
    position.entry_time = now - age;
    position.partial_exit_count = partial_exits;
    position
}

/// For the same holding, mark, peak, age and policy, the live exit monitor and the paper
/// book pick the same rule and the same share: each rule alone, the order when several
/// fire, the partial already taken, and nothing firing.
#[test]
fn live_and_paper_books_pick_the_same_rule_for_the_same_inputs() {
    use crate::trader::evaluators::{exit, exit_rules};
    use crate::trader::TradeReason;

    let policy = policy();
    let now = Utc::now();
    let young = Duration::seconds(60);
    let old = Duration::hours(2);
    let stop_loss = Some((TradeReason::StopLoss, PaperExitRule::StopLoss));
    let trailing = Some((TradeReason::TrailingStop, PaperExitRule::TrailingStop));
    let take_profit = Some((TradeReason::TakeProfit, PaperExitRule::TakeProfit));
    let time_loss = Some((TradeReason::TimeOverride, PaperExitRule::TimeOverride));
    // (case, mark, peak, age, partial already taken, expected rules, expected share)
    let cases = [
        (
            "stop loss, first leg",
            0.0065,
            0.01,
            young,
            false,
            stop_loss.clone(),
            Some(50.0),
        ),
        (
            "stop loss after a partial",
            0.0065,
            0.01,
            young,
            true,
            stop_loss.clone(),
            None,
        ),
        (
            "trailing stop",
            0.0112,
            0.0125,
            young,
            false,
            trailing.clone(),
            None,
        ),
        (
            "take profit",
            0.02,
            0.02,
            young,
            false,
            take_profit.clone(),
            None,
        ),
        (
            "time loss",
            0.0085,
            0.01,
            old,
            false,
            time_loss.clone(),
            None,
        ),
        (
            "time loss after a partial",
            0.0085,
            0.01,
            old,
            true,
            time_loss.clone(),
            None,
        ),
        (
            "stop loss before time loss",
            0.0065,
            0.01,
            old,
            false,
            stop_loss.clone(),
            Some(50.0),
        ),
        (
            "trailing before take profit",
            0.022,
            0.025,
            young,
            false,
            trailing.clone(),
            None,
        ),
        ("nothing fires", 0.0095, 0.01, young, false, None, None),
        (
            "nothing fires after a partial",
            0.0095,
            0.01,
            old,
            true,
            None,
            None,
        ),
    ];
    for (case, mark, peak, age, partial_taken, expected, share) in cases {
        let live_position = live_position(peak, age, u32::from(partial_taken), now);
        let live = exit_rules::decide(&exit::live_exit_inputs(&live_position, mark, now), &policy)
            .outcome
            .map(|outcome| {
                let decision = exit::rule_decision(&live_position, mark, &outcome);
                (decision.reason, decision.exit_percentage)
            });

        let mut paper_position = holding(u64::from(partial_taken), age);
        paper_position.opened_at = now - age;
        let paper = evaluate_paper_exit(1, &paper_position, mark, peak, &policy, now)
            .map(|exit| (exit.rule, exit.exit_percentage));

        assert_eq!(
            live,
            expected.clone().map(|(reason, _)| (reason, share)),
            "{case}: live"
        );
        assert_eq!(
            paper,
            expected.map(|(_, rule)| (rule, share)),
            "{case}: paper"
        );
    }
}

/// The live exit monitor and the paper book climb the same take-profit ladder: for each
/// step already taken they fire the same next step with the same share of what is left,
/// and a finished ladder fires nothing in either book.
#[test]
fn live_and_paper_books_climb_the_same_take_profit_ladder() {
    use crate::trader::evaluators::{exit, exit_rules};
    use crate::trader::TradeReason;

    let mut policy = policy();
    policy.stop_loss.enabled = false;
    policy.trailing.enabled = false;
    policy.time.enabled = false;
    policy.take_profit.levels = vec![
        TakeProfitLevel {
            profit_pct: 50.0,
            sell_pct: 30.0,
        },
        TakeProfitLevel {
            profit_pct: 100.0,
            sell_pct: 30.0,
        },
        TakeProfitLevel {
            profit_pct: 200.0,
            sell_pct: 40.0,
        },
    ];
    let now = Utc::now();
    let age = Duration::seconds(60);
    // (case, mark, steps taken, partial already taken, expected share and step)
    let cases = [
        ("below the first level", 0.014, 0, false, None),
        ("first step", 0.016, 0, false, Some((Some(30.0), 1))),
        (
            "past every level, first step only",
            0.05,
            0,
            false,
            Some((Some(30.0), 1)),
        ),
        ("first step taken, below the second", 0.016, 1, true, None),
        ("second step", 0.02, 1, true, Some((Some(300.0 / 7.0), 2))),
        (
            "last step sells what is left",
            0.04,
            2,
            true,
            Some((None, 3)),
        ),
        ("finished ladder", 0.05, 3, true, None),
    ];
    for (case, mark, taken, partial_taken, expected) in cases {
        let mut live_position = live_position(mark, age, u32::from(partial_taken), now);
        live_position.take_profit_level = taken;
        let live = exit_rules::decide(&exit::live_exit_inputs(&live_position, mark, now), &policy)
            .outcome
            .map(|outcome| {
                let decision = exit::rule_decision(&live_position, mark, &outcome);
                assert_eq!(decision.reason, TradeReason::TakeProfit, "{case}: live");
                (
                    decision.exit_percentage,
                    decision
                        .take_profit_level
                        .expect("a ladder step carries its level"),
                )
            });

        let mut paper_position = holding(u64::from(partial_taken), age);
        paper_position.opened_at = now - age;
        paper_position.take_profit_level = taken;
        let paper = evaluate_paper_exit(1, &paper_position, mark, mark, &policy, now).map(|exit| {
            assert_eq!(exit.rule, PaperExitRule::TakeProfit, "{case}: paper");
            (
                exit.exit_percentage,
                exit.take_profit_level
                    .expect("a ladder step carries its level"),
            )
        });

        assert_eq!(live.is_some(), expected.is_some(), "{case}: live");
        assert_eq!(paper.is_some(), expected.is_some(), "{case}: paper");
        for (book, found) in [("live", live), ("paper", paper)] {
            let (Some((share, step)), Some((expected_share, expected_step))) = (found, expected)
            else {
                continue;
            };
            assert_eq!(step, expected_step, "{case}: {book} step");
            match (share, expected_share) {
                (Some(share), Some(expected_share)) => assert!(
                    (share - expected_share).abs() < 1e-9,
                    "{case}: {book} share {share} != {expected_share}"
                ),
                (share, expected_share) => {
                    assert_eq!(share, expected_share, "{case}: {book} share")
                }
            }
        }
    }
}
