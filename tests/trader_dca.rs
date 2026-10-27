// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The DCA ladder decision (`DcaEvaluation::evaluate`, pure), the size every add takes,
//! automatic and manual, and the live executor's refusals: an add that carries no size,
//! and a strategy buy whose decision no longer stands because the trader was switched off
//! or its strategy stopped trading live.
//!
//! DCA is the only auto-trader path that SPENDS MORE SOL on a position that is already
//! losing, so every gate it has is a spend limiter: the ladder's length, each step's drop,
//! the waits, an add in flight, a stale price, the loss limit and the minimum trade size.
//! A gate that fails open buys again and again into a token that is going to zero.
//!
//! A config written before the ladder (one drop, one size and a count) migrates into a
//! uniform ladder; the property test at the end holds the migrated ladder to the old rule,
//! kept here as a reference model, decision for decision.

use crate::common;

use chrono::{DateTime, Duration, Utc};
use common::test_position;
use screenerbot::config::utils::parse_config_document;
use screenerbot::config::{DcaLevel, DcaMeasureFrom};
use screenerbot::positions::{Position, PositionManagement, PositionOrigin};
use screenerbot::trader::evaluators::{
    default_add_size_native, DcaConfigSnapshot, DcaContext, DcaEvaluation,
};
use screenerbot::trader::{
    TradeAction, TradeDecision, TradePriority, TradeReason, DCA_PRICE_MAX_AGE_SECS,
    MIN_TRADE_SIZE_NATIVE,
};

fn step(drop_pct: f64, size_multiplier: f64) -> DcaLevel {
    DcaLevel {
        drop_pct,
        size_multiplier,
    }
}

/// A three-step ladder measured from the average entry: -10%, -20%, -40%, buying half,
/// one and two times the first buy.
fn ladder_config() -> DcaConfigSnapshot {
    DcaConfigSnapshot {
        enabled: true,
        levels: vec![step(-10.0, 0.5), step(-20.0, 1.0), step(-40.0, 2.0)],
        measure_from: DcaMeasureFrom::Average,
        cooldown_minutes: 60,
        min_hold_minutes: 0,
    }
}

/// Nothing in flight, entries allowed, at `now`.
fn open_context(now: DateTime<Utc>) -> DcaContext {
    DcaContext {
        add_in_flight: false,
        entries_blocked: false,
        now,
    }
}

/// A 1 SOL position entered at price 1.0, an hour ago, priced at `current_price` just now.
fn position_at(current_price: f64) -> Position {
    let mut position = test_position(1.0, 1.0);
    position.entry_time = Utc::now() - Duration::hours(1);
    position.current_price = Some(current_price);
    position.current_price_updated = Some(Utc::now());
    position
}

fn evaluate(position: &Position, config: DcaConfigSnapshot) -> DcaEvaluation {
    evaluate_in(position, config, open_context(Utc::now()))
}

fn evaluate_in(
    position: &Position,
    config: DcaConfigSnapshot,
    context: DcaContext,
) -> DcaEvaluation {
    DcaEvaluation::evaluate(position, config, context).expect("evaluation must not fail")
}

fn has_reason(evaluation: &DcaEvaluation, needle: &str) -> bool {
    evaluation
        .reasons
        .iter()
        .any(|reason| reason.contains(needle))
}

// ==================== STEP CHOICE ====================

#[test]
fn the_next_add_follows_the_step_its_count_names() {
    // 25% down: past step 1 (-10%) and step 2 (-20%), short of step 3 (-40%).
    for (count, triggers, required) in [(0, true, -10.0), (1, true, -20.0), (2, false, -40.0)] {
        let mut position = position_at(0.75);
        position.dca_count = count;
        let evaluation = evaluate(&position, ladder_config());
        assert_eq!(evaluation.should_trigger, triggers, "count {count}");
        assert_eq!(evaluation.calculations.step, Some(count as usize));
        assert_eq!(evaluation.calculations.required_drop_pct, Some(required));
    }
}

#[test]
fn a_used_up_ladder_never_adds() {
    let mut position = position_at(0.1);
    position.dca_count = 3;
    let evaluation = evaluate(&position, ladder_config());
    assert!(!evaluation.should_trigger);
    assert_eq!(evaluation.calculations.step, None);
    assert_eq!(evaluation.calculations.dca_amount_native, None);
    assert!(has_reason(&evaluation, "Every DCA step is used"));
}

#[test]
fn a_drop_exactly_at_the_level_does_not_add() {
    // The drop must pass the level: exactly -10% waits, one tick deeper adds.
    assert!(!evaluate(&position_at(0.90), ladder_config()).should_trigger);
    assert!(evaluate(&position_at(0.8999), ladder_config()).should_trigger);
}

#[test]
fn a_position_in_profit_never_adds() {
    let evaluation = evaluate(&position_at(2.0), ladder_config());
    assert!(!evaluation.should_trigger);
    assert!(has_reason(&evaluation, "Price drop insufficient"));
}

#[test]
fn a_disabled_ladder_never_adds() {
    let mut config = ladder_config();
    config.enabled = false;
    let evaluation = evaluate(&position_at(0.5), config);
    assert!(!evaluation.should_trigger);
    assert!(has_reason(&evaluation, "DCA disabled"));
}

// ==================== BASIS ====================

#[test]
fn the_average_basis_measures_from_the_average_entry() {
    // The first buy filled at 1.0; an add moved the average to 0.8. At 0.7 the price is
    // 12.5% below the average, short of step 2 (-20%).
    let mut position = position_at(0.7);
    position.average_entry_price = 0.8;
    position.dca_count = 1;
    let evaluation = evaluate(&position, ladder_config());
    assert_eq!(evaluation.calculations.basis_price, Some(0.8));
    assert!((evaluation.calculations.drop_pct.unwrap() - -12.5).abs() < 1e-9);
    assert!(!evaluation.should_trigger);
}

#[test]
fn the_first_entry_basis_measures_from_the_first_fill() {
    // The same position measured from the first fill (1.0) is 30% down: past step 2.
    let mut position = position_at(0.7);
    position.average_entry_price = 0.8;
    position.dca_count = 1;
    let mut config = ladder_config();
    config.measure_from = DcaMeasureFrom::FirstEntry;
    let evaluation = evaluate(&position, config);
    assert_eq!(evaluation.calculations.basis_price, Some(1.0));
    assert!((evaluation.calculations.drop_pct.unwrap() - -30.0).abs() < 1e-9);
    assert!(
        evaluation.should_trigger,
        "reasons: {:?}",
        evaluation.reasons
    );
}

#[test]
fn a_missing_first_fill_refuses_rather_than_falling_back() {
    // Before the entry verifies there is no first fill: the market price at open or the
    // average must not stand in for it.
    let mut position = position_at(0.5);
    position.effective_entry_price = None;
    let mut config = ladder_config();
    config.measure_from = DcaMeasureFrom::FirstEntry;
    let evaluation = evaluate(&position, config);
    assert!(!evaluation.should_trigger);
    assert_eq!(evaluation.calculations.basis_price, None);
    assert!(has_reason(&evaluation, "No valid first_entry price"));
}

#[test]
fn a_broken_basis_is_refused_rather_than_dividing_by_it() {
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut position = position_at(0.5);
        position.average_entry_price = bad;
        let evaluation = evaluate(&position, ladder_config());
        assert!(
            !evaluation.should_trigger,
            "average entry {bad} must refuse"
        );
        assert_eq!(evaluation.calculations.drop_pct, None);
    }
}

// ==================== WAITS ====================

#[test]
fn the_wait_after_entry_gates_only_the_first_add() {
    let mut config = ladder_config();
    config.min_hold_minutes = 120; // the position was entered an hour ago
    let first = evaluate(&position_at(0.5), config.clone());
    assert!(!first.should_trigger);
    assert!(has_reason(&first, "before the first add"));

    let mut later = position_at(0.5);
    later.dca_count = 1;
    later.last_dca_time = Some(Utc::now() - Duration::hours(2));
    assert!(evaluate(&later, config).should_trigger);
}

#[test]
fn the_wait_after_entry_ends_at_the_configured_minute() {
    let now = Utc::now();
    let mut config = ladder_config();
    config.min_hold_minutes = 30;
    let mut position = position_at(0.5);
    position.entry_time = now - Duration::minutes(30);
    position.current_price_updated = Some(now);
    assert!(evaluate_in(&position, config, open_context(now)).should_trigger);
}

/// An entry time ahead of the evaluator's clock counts as no time held: it never blocks
/// the first add when no wait is configured, and it never shortens a configured wait.
#[test]
fn an_entry_time_ahead_of_the_clock_counts_as_no_time_held() {
    let now = Utc::now();
    let mut position = position_at(0.5);
    position.entry_time = now + Duration::minutes(2);
    position.current_price_updated = Some(now);
    let unwaited = evaluate_in(&position, ladder_config(), open_context(now));
    assert!(unwaited.should_trigger, "reasons {:?}", unwaited.reasons);
    assert_eq!(unwaited.calculations.minutes_held, 0);

    let mut config = ladder_config();
    config.min_hold_minutes = 30;
    let waiting = evaluate_in(&position, config, open_context(now));
    assert!(!waiting.should_trigger);
    assert!(has_reason(&waiting, "Held 0/30 minutes"));
}

#[test]
fn the_cooldown_blocks_a_rapid_second_add_and_expires_on_its_minute() {
    let now = Utc::now();
    let mut position = position_at(0.5);
    position.dca_count = 1;
    position.current_price_updated = Some(now);
    position.last_dca_time = Some(now - Duration::minutes(10)); // cooldown is 60
    let blocked = evaluate_in(&position, ladder_config(), open_context(now));
    assert!(!blocked.should_trigger);
    assert!(has_reason(&blocked, "DCA cooldown active"));

    position.last_dca_time = Some(now - Duration::minutes(60));
    assert!(evaluate_in(&position, ladder_config(), open_context(now)).should_trigger);
}

// ==================== LIVE GATES ====================

#[test]
fn a_stale_price_is_refused() {
    let now = Utc::now();
    let mut position = position_at(0.5);
    position.current_price_updated = Some(now - Duration::seconds(DCA_PRICE_MAX_AGE_SECS));
    assert!(evaluate_in(&position, ladder_config(), open_context(now)).should_trigger);

    position.current_price_updated = Some(now - Duration::seconds(DCA_PRICE_MAX_AGE_SECS + 1));
    let stale = evaluate_in(&position, ladder_config(), open_context(now));
    assert!(!stale.should_trigger);
    assert!(has_reason(&stale, "old"));

    position.current_price_updated = None;
    assert!(!evaluate_in(&position, ladder_config(), open_context(now)).should_trigger);
}

#[test]
fn the_loss_limit_stops_automatic_adds() {
    let position = position_at(0.5);
    let context = DcaContext {
        entries_blocked: true,
        ..open_context(Utc::now())
    };
    let evaluation = evaluate_in(&position, ladder_config(), context);
    assert!(!evaluation.should_trigger);
    assert!(has_reason(&evaluation, "loss limit"));
}

#[test]
fn an_add_in_flight_blocks_the_next_add() {
    // The count and the cooldown only move when the first add verifies; deciding again on
    // the unmoved figures would buy twice into the same drop.
    let position = position_at(0.5);
    assert!(evaluate(&position, ladder_config()).should_trigger);
    let context = DcaContext {
        add_in_flight: true,
        ..open_context(Utc::now())
    };
    let evaluation = evaluate_in(&position, ladder_config(), context);
    assert!(!evaluation.should_trigger);
    assert!(has_reason(&evaluation, "still in flight"));
}

#[test]
fn an_add_in_flight_cannot_carry_the_count_past_the_ladder() {
    // The last step's add is in flight: one decided now would verify as a fourth add of a
    // three-step ladder.
    let mut position = position_at(0.1);
    position.dca_count = 2;
    let context = DcaContext {
        add_in_flight: true,
        ..open_context(Utc::now())
    };
    assert!(!evaluate_in(&position, ladder_config(), context).should_trigger);
}

#[test]
fn every_non_auto_trader_management_refuses_auto_dca() {
    for management in [
        PositionManagement::UserOnly,
        PositionManagement::CopyTask,
        PositionManagement::Hybrid,
    ] {
        let mut position = position_at(0.5);
        position.management = management;
        let evaluation = evaluate(&position, ladder_config());
        assert!(!evaluation.should_trigger, "management={management:?}");
        assert!(has_reason(&evaluation, "does not allow auto-DCA"));
    }
}

#[test]
fn every_failing_gate_is_reported_not_just_the_first() {
    let mut position = position_at(0.99);
    position.dca_count = 1;
    position.last_dca_time = Some(Utc::now());
    let context = DcaContext {
        add_in_flight: true,
        entries_blocked: true,
        now: Utc::now(),
    };
    let evaluation = evaluate_in(&position, ladder_config(), context);
    for needle in [
        "still in flight",
        "loss limit",
        "Price drop insufficient",
        "cooldown",
    ] {
        assert!(
            has_reason(&evaluation, needle),
            "{needle}: {:?}",
            evaluation.reasons
        );
    }
}

// ==================== SIZE ====================

#[test]
fn each_step_buys_its_multiple_of_the_first_buy() {
    // The base is the FIRST buy, never the cumulative size: exposure grows by the ladder's
    // multiples, not by compounding on earlier adds.
    for (count, expected) in [(0, 0.5), (1, 1.0), (2, 2.0)] {
        let mut position = position_at(0.1);
        position.entry_size_native = 1.0;
        position.total_size_native = 4.0;
        position.dca_count = count;
        let evaluation = evaluate(&position, ladder_config());
        assert_eq!(
            evaluation.calculations.dca_amount_native,
            Some(expected),
            "count {count}"
        );
    }
}

#[test]
fn an_add_under_the_minimum_trade_is_refused_and_flagged() {
    let mut position = position_at(0.5);
    position.entry_size_native = MIN_TRADE_SIZE_NATIVE; // half of it is under the minimum
    let evaluation = evaluate(&position, ladder_config());
    assert!(!evaluation.should_trigger);
    assert!(evaluation.calculations.below_minimum_trade);
    assert!(has_reason(&evaluation, "below minimum"));

    position.entry_size_native = MIN_TRADE_SIZE_NATIVE * 2.0; // half lands on the minimum
    let evaluation = evaluate(&position, ladder_config());
    assert!(evaluation.should_trigger);
    assert!(!evaluation.calculations.below_minimum_trade);
}

#[test]
fn the_minimum_trade_flag_names_only_an_add_every_other_gate_allows() {
    let mut position = position_at(0.99); // not down enough
    position.entry_size_native = MIN_TRADE_SIZE_NATIVE;
    let evaluation = evaluate(&position, ladder_config());
    assert!(!evaluation.calculations.below_minimum_trade);
}

/// The configured trade size the manual-default tests pass.
const TRADE_SIZE: f64 = 0.05;

#[test]
fn a_manual_add_defaults_to_the_next_step_then_to_the_first_buy() {
    let levels = ladder_config().levels;
    let mut position = position_at(1.0);
    position.entry_size_native = 0.2;
    for (count, expected) in [(0, 0.1), (1, 0.2), (2, 0.4), (3, 0.2), (7, 0.2)] {
        position.dca_count = count;
        let size = default_add_size_native(&position, &levels, TRADE_SIZE)
            .expect("a first buy to size from");
        assert!((size - expected).abs() < 1e-12, "count {count}: {size}");
    }
}

/// Every origin has a manual default while the trade size is positive: a wallet-derived
/// round, which has no first buy, takes the trade size itself at every count. The
/// automatic add never takes that stand-in and refuses the same round.
#[test]
fn every_origin_has_a_manual_default_but_only_a_first_buy_sizes_an_automatic_add() {
    let levels = ladder_config().levels;
    for origin in [
        PositionOrigin::Auto { strategy_id: None },
        PositionOrigin::Manual,
        PositionOrigin::Copy {
            task_id: 1,
            source_wallet: "source".to_owned(),
        },
        PositionOrigin::External,
    ] {
        for count in [0, 1, 2, 3, 7] {
            let mut position = position_at(0.5);
            position.origin = origin.clone();
            position.dca_count = count;
            position.last_dca_time = Some(Utc::now() - Duration::days(1));
            let size = default_add_size_native(&position, &levels, TRADE_SIZE)
                .unwrap_or_else(|| panic!("{origin:?} count {count} has a manual default"));
            assert!(size.is_finite() && size > 0.0, "{origin:?} count {count}");
            if origin == PositionOrigin::External {
                assert_eq!(size, TRADE_SIZE, "count {count}");
                let automatic = evaluate(&position, ladder_config());
                assert!(!automatic.should_trigger, "count {count}");
                assert_eq!(automatic.calculations.dca_amount_native, None);
                if count < 3 {
                    assert!(
                        has_reason(&automatic, "No first buy size"),
                        "count {count}: {:?}",
                        automatic.reasons
                    );
                }
            }
        }
    }
}

#[test]
fn a_manual_default_that_is_not_a_positive_amount_is_none() {
    let levels = ladder_config().levels;
    let mut broken = position_at(1.0);
    broken.entry_size_native = 0.0;
    assert_eq!(default_add_size_native(&broken, &levels, TRADE_SIZE), None);

    let mut wallet = position_at(1.0);
    wallet.origin = PositionOrigin::External;
    for trade_size in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            default_add_size_native(&wallet, &levels, trade_size),
            None,
            "trade size {trade_size}"
        );
    }
}

#[test]
fn the_summary_names_the_next_add_when_triggering() {
    let mut position = position_at(0.75);
    position.dca_count = 1;
    let evaluation = evaluate(&position, ladder_config());
    assert!(evaluation.should_trigger);
    assert!(
        evaluation.summary().starts_with("DCA #2:"),
        "summary: {}",
        evaluation.summary()
    );
}

// ==================== EXECUTOR ====================

#[tokio::test]
async fn the_executor_refuses_an_add_with_no_size() {
    let decision = TradeDecision {
        position_id: Some("1".to_owned()),
        mint: common::TEST_MINT.to_owned(),
        action: TradeAction::DCA,
        reason: TradeReason::DCAScheduled,
        strategy_id: None,
        book: screenerbot::positions::Book::Live,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_native: Some(1.0),
        size_native: None,
        exit_percentage: None,
        slippage_pct: None,
        take_profit_level: None,
        paper_leg_key: None,
    };
    let context = screenerbot::trader::executors::ExecutionContext::for_decision(&decision);
    let error = screenerbot::trader::executors::execute_trade(&decision, context)
        .await
        .expect_err("an add without a size must be refused");
    assert!(
        matches!(error, screenerbot::trader::Error::AddSizeRequired { ref mint } if mint == common::TEST_MINT),
        "{error:?}"
    );
}

/// A live buy decided by `strategy_id`, as the live entry pass builds it.
fn live_strategy_buy(strategy_id: &str) -> TradeDecision {
    TradeDecision {
        position_id: None,
        mint: common::TEST_MINT.to_owned(),
        action: TradeAction::Buy,
        reason: TradeReason::StrategySignal,
        strategy_id: Some(strategy_id.to_owned()),
        book: screenerbot::positions::Book::Live,
        timestamp: Utc::now(),
        priority: TradePriority::Normal,
        price_native: Some(1.0),
        size_native: None,
        exit_percentage: None,
        slippage_pct: None,
        take_profit_level: None,
        paper_leg_key: None,
    }
}

/// An enabled live entry strategy stored under the id `id`.
fn stored_live_entry(id: &str) -> screenerbot::strategies::types::Strategy {
    use screenerbot::strategies::types::{RuleTree, Strategy, StrategyType};
    let strategy = Strategy {
        id: id.to_owned(),
        name: id.to_owned(),
        description: None,
        strategy_type: StrategyType::Entry,
        enabled: true,
        priority: 1,
        timeframe: "5m".to_owned(),
        rules: RuleTree::leaf(common::condition(
            "LiquidityLevel",
            &[
                ("threshold", serde_json::json!(10.0)),
                ("comparison", serde_json::json!("GREATER_EQUAL")),
            ],
        )),
        parameters: Default::default(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        author: None,
        version: 1,
        entry_strategy_ids: Vec::new(),
        mode: screenerbot::trader::paper::TradingMode::Live,
    };
    screenerbot::strategies::db::insert_strategy(&strategy).expect("insert strategy");
    strategy
}

/// Switches the trader on or off, as its controls do.
fn set_trader_enabled(enabled: bool) {
    screenerbot::config::update_config_section(|cfg| cfg.trader.enabled = enabled, false)
        .expect("switch the trader");
}

/// Executes `decision` on the live path with the context it implies.
async fn execute(decision: &TradeDecision) -> screenerbot::trader::TradeResult {
    let context = screenerbot::trader::executors::ExecutionContext::for_decision(decision);
    screenerbot::trader::executors::execute_trade(decision, context)
        .await
        .expect("a refusal is a failed result, not an error")
}

/// A live buy whose strategy was switched off, returned to paper or removed after the
/// decision is withdrawn at validation, before the trade guard, the connectivity check, a
/// quote or a send: it carries no signature and names the strategy.
#[tokio::test]
async fn a_live_buy_whose_strategy_left_live_is_refused_and_sends_nothing() {
    let _dir = common::isolated_env();
    screenerbot::strategies::db::init_strategies_db().expect("open the strategies store");
    set_trader_enabled(true);

    let switched_off = stored_live_entry("switched-off");
    stored_live_entry("returned-to-paper");
    stored_live_entry("removed");
    let decisions: Vec<_> = ["switched-off", "returned-to-paper", "removed"]
        .into_iter()
        .map(live_strategy_buy)
        .collect();

    // The decisions above were taken while each strategy traded live; each then leaves.
    screenerbot::strategies::db::update_strategy(&screenerbot::strategies::types::Strategy {
        enabled: false,
        ..switched_off
    })
    .expect("switch the strategy off");
    screenerbot::strategies::db::set_strategy_mode(
        "returned-to-paper",
        screenerbot::trader::paper::TradingMode::Paper,
    )
    .expect("return the strategy to paper");
    screenerbot::strategies::db::delete_strategy("removed").expect("remove the strategy");

    for decision in decisions {
        let strategy_id = decision.strategy_id.clone().expect("a strategy decision");
        let result = execute(&decision).await;
        assert!(!result.success, "{strategy_id}");
        assert_eq!(result.tx_signature, None, "{strategy_id}");
        assert_eq!(
            result.failed_step,
            Some(screenerbot::trader::TradeStep::Validation),
            "{strategy_id}"
        );
        assert_eq!(
            result.withdrawn,
            Some(screenerbot::trader::Withdrawal::StrategyStoppedTrading),
            "{strategy_id}"
        );
        let error = result.error.unwrap_or_default();
        assert!(
            error.contains(&strategy_id) && error.contains("after the entry was decided"),
            "{strategy_id} was not refused by its strategy check: {error}"
        );
    }
}

/// A live strategy buy whose strategy still trades live is withdrawn once the trader is
/// switched off after the decision, before anything is quoted or sent.
#[tokio::test]
async fn a_live_strategy_buy_is_withdrawn_once_the_trader_is_switched_off() {
    let _dir = common::isolated_env();
    screenerbot::strategies::db::init_strategies_db().expect("open the strategies store");
    set_trader_enabled(true);
    stored_live_entry("still-live");
    let decision = live_strategy_buy("still-live");

    set_trader_enabled(false);
    let result = execute(&decision).await;
    assert!(!result.success);
    assert_eq!(result.tx_signature, None);
    assert_eq!(
        result.failed_step,
        Some(screenerbot::trader::TradeStep::Validation)
    );
    assert_eq!(
        result.withdrawn,
        Some(screenerbot::trader::Withdrawal::TraderStopped)
    );
    let error = result.error.unwrap_or_default();
    assert!(
        error.contains("trader was switched off") && error.contains("still-live"),
        "the buy was not refused by the trader check: {error}"
    );
}

// ==================== MIGRATED LADDER == UNIFORM RULE ====================

/// The retired uniform DCA settings: one drop, one size and a count of adds.
#[derive(Debug, Clone, Copy)]
struct UniformRule {
    enabled: bool,
    threshold_pct: f64,
    max_count: u32,
    size_percentage: f64,
    cooldown_minutes: i64,
}

impl UniformRule {
    /// Whether the uniform rule added to `position` at `now`, and how much, exactly as the
    /// evaluator decided before the ladder.
    fn decide(&self, position: &Position, add_in_flight: bool, now: DateTime<Utc>) -> Option<f64> {
        let current = position
            .current_price
            .filter(|p| *p > 0.0 && p.is_finite())?;
        let entry = position.average_entry_price;
        if entry <= 0.0 || !entry.is_finite() {
            return None;
        }
        let pnl_pct = ((current - entry) / entry) * 100.0;
        let amount = position.entry_size_native * (self.size_percentage / 100.0);
        let cooled = position.last_dca_time.map_or(true, |last| {
            (now - last).num_minutes() >= self.cooldown_minutes
        });
        let fires = position.management.allows_auto_dca()
            && self.enabled
            && !add_in_flight
            && position.dca_count < self.max_count
            && cooled
            && pnl_pct < self.threshold_pct
            && amount >= MIN_TRADE_SIZE_NATIVE;
        fires.then_some(amount)
    }

    /// The `[trader]` section a config file held for this rule.
    fn toml(&self) -> String {
        format!(
            "[trader]\ndca_enabled = {}\ndca_threshold_pct = {:?}\ndca_max_count = {}\n\
             dca_size_percentage = {:?}\ndca_cooldown_minutes = {}\n",
            self.enabled,
            self.threshold_pct,
            self.max_count,
            self.size_percentage,
            self.cooldown_minutes
        )
    }
}

/// A small deterministic generator, so every run checks the same cases.
struct Cases(u64);

impl Cases {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[(self.next() % items.len() as u64) as usize]
    }
}

/// The snapshot the evaluator reads from the migrated file, as the trader builds it.
fn migrated_snapshot(rule: &UniformRule) -> DcaConfigSnapshot {
    let parsed = parse_config_document(&rule.toml()).expect("a uniform DCA config parses");
    let trader = parsed.config.trader;
    DcaConfigSnapshot {
        enabled: trader.dca_enabled,
        levels: trader.dca_levels(),
        measure_from: trader.dca_measure_from,
        cooldown_minutes: trader.dca_cooldown_minutes,
        min_hold_minutes: trader.dca_min_hold_minutes,
    }
}

/// A uniform rule drawn from `cases`, its drop from `thresholds` and its size from `sizes`.
fn sampled_rule(cases: &mut Cases, thresholds: &[f64], sizes: &[f64]) -> UniformRule {
    UniformRule {
        enabled: cases.pick(&[true, true, true, false]),
        threshold_pct: cases.pick(thresholds),
        max_count: cases.pick(&[0, 1, 2, 3, 4, 5]),
        size_percentage: cases.pick(sizes),
        cooldown_minutes: cases.pick(&[1, 30, 60, 1440]),
    }
}

/// A position held two hours, priced at `now`, and whether an add of it is in flight,
/// drawn from `cases`.
fn sampled_position(cases: &mut Cases, now: DateTime<Utc>) -> (Position, bool) {
    let mut position = test_position(1.0, cases.pick(&[0.001, 0.0015, 0.005, 0.1, 1.0]));
    position.entry_time = now - Duration::hours(2);
    position.current_price = Some(cases.pick(&[0.01, 0.04, 0.5, 0.75, 0.9, 0.95, 0.99, 1.0, 1.2]));
    position.current_price_updated = Some(now);
    position.average_entry_price = cases.pick(&[0.6, 0.8, 1.0]);
    position.dca_count = cases.pick(&[0, 1, 2, 3, 4, 5, 6]);
    position.last_dca_time = cases.pick(&[
        None,
        Some(now - Duration::minutes(5)),
        Some(now - Duration::minutes(60)),
        Some(now - Duration::days(2)),
    ]);
    position.management = cases.pick(&[
        PositionManagement::AutoTrader,
        PositionManagement::AutoTrader,
        PositionManagement::UserOnly,
    ]);
    (position, cases.pick(&[false, false, true]))
}

/// What the evaluator decides for `position` under the file `rule` migrates into: the
/// amount of the add it fires, and the evaluation for the failure message.
fn migrated_decision(
    rule: &UniformRule,
    position: &Position,
    add_in_flight: bool,
    now: DateTime<Utc>,
) -> (Option<f64>, DcaEvaluation) {
    let context = DcaContext {
        add_in_flight,
        entries_blocked: false,
        now,
    };
    let evaluation = evaluate_in(position, migrated_snapshot(rule), context);
    let decided = evaluation
        .should_trigger
        .then_some(evaluation.calculations.dca_amount_native)
        .flatten();
    (decided, evaluation)
}

#[test]
fn a_migrated_uniform_rule_decides_every_add_as_it_did() {
    let now = Utc::now();
    let mut cases = Cases(0x9E37_79B9_7F4A_7C15);
    for _ in 0..2_000 {
        let rule = sampled_rule(
            &mut cases,
            &[-1.0, -5.0, -7.5, -10.0, -25.0, -50.0, -95.0],
            &[10.0, 25.0, 50.0, 75.0, 100.0],
        );
        let (position, add_in_flight) = sampled_position(&mut cases, now);
        let expected = rule.decide(&position, add_in_flight, now);
        let (decided, evaluation) = migrated_decision(&rule, &position, add_in_flight, now);
        assert_eq!(
            decided,
            expected,
            "rule {rule:?}, count {}, price {:?}, average {}, reasons {:?}",
            position.dca_count,
            position.current_price,
            position.average_entry_price,
            evaluation.reasons
        );
    }
}

/// A stored value outside the ladder's bounds cannot migrate unchanged. Whatever it was,
/// the migrated file never adds where the uniform rule did not, and never buys more than
/// it did: a clamp that would add sooner or buy more turns DCA off instead.
#[test]
fn a_migrated_out_of_bounds_rule_never_adds_sooner_or_more() {
    let now = Utc::now();
    let mut cases = Cases(0xD1B5_4A32_D192_ED03);
    for _ in 0..2_000 {
        let rule = sampled_rule(
            &mut cases,
            &[-100.0, -97.0, -95.5, -10.0, -0.5, 0.0, 15.0, 97.0],
            &[0.0, 0.5, 5.0, 9.0, 50.0, 600.0],
        );
        let (position, add_in_flight) = sampled_position(&mut cases, now);
        let expected = rule.decide(&position, add_in_flight, now);
        let (decided, evaluation) = migrated_decision(&rule, &position, add_in_flight, now);
        if let Some(amount) = decided {
            let legacy = expected.unwrap_or_else(|| {
                panic!(
                    "rule {rule:?} made no add, the migrated file adds {amount}: count {}, \
                     price {:?}, average {}, reasons {:?}",
                    position.dca_count,
                    position.current_price,
                    position.average_entry_price,
                    evaluation.reasons
                )
            });
            assert!(
                amount <= legacy * (1.0 + 1e-12),
                "rule {rule:?} added {legacy}, the migrated file adds {amount}"
            );
        }
    }
}
