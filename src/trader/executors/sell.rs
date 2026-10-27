// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Sell operation execution.

use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::constants::{MAX_EXIT_SHARE_PCT, MIN_EXIT_SHARE_PCT};
use crate::trader::types::{TradeDecision, TradeReason, TradeResult, TradeStep};

/// How much of a position a sell decision actually liquidates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::trader) enum ExitSize {
    /// Sell everything the position still holds.
    Full,
    /// Sell this percentage of the remaining tokens.
    Partial(f64),
}

/// Decide whether a sell is a partial or a full exit.
///
/// Extracted from [`execute_sell`] so the routing can be tested on its own: it is the
/// rule that decides how many of the user's tokens leave the wallet.
///
/// * `requested_pct` is `TradeDecision::exit_percentage`; `None` means a full exit. It
///   is clamped to [`MIN_EXIT_SHARE_PCT`, `MAX_EXIT_SHARE_PCT`]: below the minimum the
///   token amount rounds to zero on many balances, and `partial_close_position` refuses
///   anything outside `(0, 100)`. Every manual entry point refuses a share outside the
///   same bounds through `validate_percentage`, so only an automatic producer is clamped.
/// * Every reason honours the percentage its producer requested. Whether an automatic
///   rule may sell part of a position is that rule's own setting (the stop loss asks
///   for a share only while `trader.stop_loss_allow_partial` is on), so no global
///   switch can upgrade a requested partial, a user's above all, into a full exit.
/// * Emergencies (`Blacklisted`, `RiskManagement`) always liquidate the whole
///   position, whatever percentage the decision carries.
pub(in crate::trader) fn resolve_exit_size(
    reason: &TradeReason,
    requested_pct: Option<f64>,
) -> ExitSize {
    let exit_percentage = requested_pct
        .unwrap_or(MAX_EXIT_SHARE_PCT)
        .clamp(MIN_EXIT_SHARE_PCT, MAX_EXIT_SHARE_PCT);
    if is_emergency_exit(reason) || exit_percentage >= MAX_EXIT_SHARE_PCT {
        ExitSize::Full
    } else {
        ExitSize::Partial(exit_percentage)
    }
}

/// Whether `reason` must sell the whole position. Exhaustive on purpose: a new
/// reason fails to compile until it is classified here.
///
/// A stop loss is not an emergency: it sells the share its rule requested. A force
/// sell is not one either: it bypasses the safety checks, it does not widen the size.
fn is_emergency_exit(reason: &TradeReason) -> bool {
    match reason {
        TradeReason::Blacklisted | TradeReason::RiskManagement => true,
        TradeReason::StrategySignal
        | TradeReason::ManualEntry
        | TradeReason::ForceBuy
        | TradeReason::CopyBuy
        | TradeReason::DCAScheduled
        | TradeReason::TakeProfit
        | TradeReason::StopLoss
        | TradeReason::TrailingStop
        | TradeReason::BreakEvenStop
        | TradeReason::TimeOverride
        | TradeReason::TimeStagnation
        | TradeReason::StrategyExit
        | TradeReason::LlmAnalysisExit
        | TradeReason::ManualExit
        | TradeReason::ForceSell
        | TradeReason::CopySell
        | TradeReason::PaperReset => false,
    }
}

/// Execute a live sell trade
pub(super) async fn execute_sell(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
    super::require_live(decision)?;
    let Some(_active_trade) = crate::global::begin_trade() else {
        return Ok(TradeResult::failure_at(
            decision.clone(),
            TradeStep::Validation,
            "Cannot execute sell - an application update is restarting the trading engine"
                .to_owned(),
            0,
        ));
    };
    if crate::global::is_force_stopped() {
        return Ok(TradeResult::failure_at(
            decision.clone(),
            TradeStep::Validation,
            "Cannot execute sell - trading is force stopped".to_owned(),
            0,
        ));
    }
    // Check connectivity before executing trade - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute sell - Unhealthy endpoints: {unhealthy}");
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure_at(
            decision.clone(),
            TradeStep::Validation,
            error,
            0,
        ));
    }

    logger::info(
        LogTag::Trader,
        &format!(
            "Executing sell for position {} token {} (reason: {:?})",
            decision.position_id.as_deref().unwrap_or("unknown"),
            decision.mint,
            decision.reason
        ),
    );

    let exit_size = resolve_exit_size(&decision.reason, decision.exit_percentage);

    let exit_reason = format!("{:?}", decision.reason);

    if let ExitSize::Partial(exit_percentage) = exit_size {
        match positions::partial_close_position(
            &decision.mint,
            exit_percentage,
            &exit_reason.clone(),
            decision.slippage_pct,
            decision.take_profit_level,
        )
        .await
        {
            Ok(transaction_signature) => {
                logger::info(
                    LogTag::Trader,
                    &format!(
                        "Partial sell executed: {} | {}% | TX: {} | Reason: {}",
                        decision.mint, exit_percentage, transaction_signature, exit_reason
                    ),
                );

                Ok(TradeResult::success(
                    decision.clone(),
                    transaction_signature,
                    None,
                    None,
                    decision.position_id.clone(),
                ))
            }
            Err(e) => {
                let error = format!("Partial sell execution failed: {e}");
                logger::error(LogTag::Trader, &error);
                Ok(TradeResult::failure_from(decision.clone(), &e, error))
            }
        }
    } else {
        // Full exit: an emergency, no requested percentage, or 100%
        match positions::close_position_direct(
            &decision.mint,
            exit_reason.clone(),
            decision.slippage_pct,
        )
        .await
        {
            Ok(transaction_signature) => {
                logger::info(
                    LogTag::Trader,
                    &format!(
                        "Full sell executed: {} | TX: {} | Reason: {}",
                        decision.mint, transaction_signature, exit_reason
                    ),
                );

                Ok(TradeResult::success(
                    decision.clone(),
                    transaction_signature,
                    None,
                    None,
                    decision.position_id.clone(),
                ))
            }
            Err(e) => {
                let error = format!("Full sell execution failed: {e}");
                logger::error(LogTag::Trader, &error);
                Ok(TradeResult::failure_from(decision.clone(), &e, error))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit tests for exit-size routing. Co-located because `resolve_exit_size` is
    //! private to the trader and unreachable from the `tests/` integration crates.
    use super::*;

    /// How a reason routes a requested percentage below 100.
    #[derive(Debug, PartialEq)]
    enum Routing {
        /// Always the whole position.
        Emergency,
        /// The requested percentage.
        ByRequest,
    }

    /// The expected routing of every reason, written independently of
    /// `is_emergency_exit`. Exhaustive: a new reason fails to compile here until it
    /// is classified.
    fn expected_routing(reason: &TradeReason) -> Routing {
        match reason {
            TradeReason::Blacklisted | TradeReason::RiskManagement => Routing::Emergency,
            TradeReason::StrategySignal
            | TradeReason::ManualEntry
            | TradeReason::ForceBuy
            | TradeReason::CopyBuy
            | TradeReason::DCAScheduled
            | TradeReason::TakeProfit
            | TradeReason::StopLoss
            | TradeReason::TrailingStop
            | TradeReason::BreakEvenStop
            | TradeReason::TimeOverride
            | TradeReason::TimeStagnation
            | TradeReason::StrategyExit
            | TradeReason::LlmAnalysisExit
            | TradeReason::ManualExit
            | TradeReason::ForceSell
            | TradeReason::CopySell
            | TradeReason::PaperReset => Routing::ByRequest,
        }
    }

    #[test]
    fn every_reason_routes_by_its_requested_percentage() {
        for reason in TradeReason::ALL {
            let expected = match expected_routing(&reason) {
                Routing::Emergency => ExitSize::Full,
                Routing::ByRequest => ExitSize::Partial(40.0),
            };
            assert_eq!(
                resolve_exit_size(&reason, Some(40.0)),
                expected,
                "{reason:?} with 40% requested"
            );
            assert_eq!(
                resolve_exit_size(&reason, None),
                ExitSize::Full,
                "{reason:?} with no percentage"
            );
            assert_eq!(
                resolve_exit_size(&reason, Some(100.0)),
                ExitSize::Full,
                "{reason:?} with 100% requested"
            );
        }
    }

    #[test]
    fn the_reason_list_has_no_duplicates() {
        // `ALL` drives the table above; its completeness is guarded beside `TradeReason`.
        let names: std::collections::BTreeSet<String> = TradeReason::ALL
            .iter()
            .map(|reason| format!("{reason:?}"))
            .collect();
        assert_eq!(names.len(), TradeReason::ALL.len());
    }

    #[test]
    fn no_percentage_means_sell_everything() {
        assert_eq!(
            resolve_exit_size(&TradeReason::TakeProfit, None),
            ExitSize::Full
        );
    }

    #[test]
    fn an_explicit_hundred_percent_is_a_full_exit() {
        // 100% must take the FULL-close path, which sizes off the on-chain balance and
        // leaves no dust, not the partial path, which sizes off a percentage.
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(100.0)),
            ExitSize::Full
        );
    }

    #[test]
    fn a_manual_partial_is_never_upgraded_to_a_full_exit() {
        // A user's 25% take-profit was once silently upgraded to a FULL exit by an
        // auto-trader switch: the whole position sold, no error, no warning.
        for reason in [
            TradeReason::ManualExit,
            TradeReason::ForceSell,
            TradeReason::CopySell,
        ] {
            assert_eq!(
                resolve_exit_size(&reason, Some(25.0)),
                ExitSize::Partial(25.0),
                "{reason:?}"
            );
        }
    }

    #[test]
    fn an_emergency_always_liquidates_the_whole_position() {
        // A blacklisted or catastrophically losing token must leave the wallet
        // entirely, whatever percentage the decision happens to carry.
        for reason in [TradeReason::Blacklisted, TradeReason::RiskManagement] {
            assert_eq!(
                resolve_exit_size(&reason, Some(10.0)),
                ExitSize::Full,
                "{reason:?} must fully liquidate"
            );
        }
    }

    #[test]
    fn a_force_sell_is_not_an_emergency() {
        // Force means "bypass the safety checks", not "sell everything". Treating it
        // as an emergency silently ignored the percentage it was given.
        assert_eq!(
            resolve_exit_size(&TradeReason::ForceSell, Some(30.0)),
            ExitSize::Partial(30.0)
        );
    }

    #[test]
    fn a_stop_loss_sells_the_share_its_rule_requested() {
        // The stop loss requests a share only while `stop_loss_allow_partial` is on;
        // the request alone decides the size.
        assert_eq!(
            resolve_exit_size(&TradeReason::StopLoss, Some(50.0)),
            ExitSize::Partial(50.0)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::StopLoss, None),
            ExitSize::Full
        );
    }

    /// The validator and the executor share one range: a share the validator accepts is
    /// sold as named, and a share the executor would raise is refused before it.
    #[test]
    fn every_share_the_validator_accepts_is_sold_as_named() {
        use crate::trader::manual::guard::validate_percentage;

        for pct in [MIN_EXIT_SHARE_PCT, 1.5, 25.0, 99.9] {
            let accepted = validate_percentage(pct).expect("a share inside the range");
            assert_eq!(
                resolve_exit_size(&TradeReason::ManualExit, Some(accepted)),
                ExitSize::Partial(pct)
            );
        }
        let whole = validate_percentage(MAX_EXIT_SHARE_PCT).expect("the whole position");
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(whole)),
            ExitSize::Full
        );
        for pct in [0.0, 0.5, MIN_EXIT_SHARE_PCT - 0.01] {
            assert!(validate_percentage(pct).is_err(), "{pct} is refused");
        }
    }

    #[test]
    fn the_percentage_is_clamped_into_a_submittable_range() {
        // Below 1% would round to a zero token amount on many balances, and above 100%
        // cannot be sold at all. `partial_close_position` refuses both outright.
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(0.4)),
            ExitSize::Partial(MIN_EXIT_SHARE_PCT)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(-5.0)),
            ExitSize::Partial(MIN_EXIT_SHARE_PCT)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(250.0)),
            ExitSize::Full
        );
    }
}
