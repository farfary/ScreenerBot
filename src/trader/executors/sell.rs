//! Sell operation execution

use crate::config::with_config;
use crate::logger::{self, LogTag};
use crate::positions;
use crate::trader::types::{FailedTradeStep, TradeDecision, TradeReason, TradeResult, TradeStep};

/// How much of a position a sell decision actually liquidates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ExitSize {
    /// Sell everything the position still holds.
    Full,
    /// Sell this percentage of the remaining tokens.
    Partial(f64),
}

/// Decide whether a sell is a partial or a full exit.
///
/// Extracted from [`execute_sell`] so the routing can be tested on its own — it is the
/// rule that decides how many of the user's tokens leave the wallet, and it has been
/// wrong before in the most destructive way possible (see the ManualExit case below).
///
/// * `requested_pct` is `TradeDecision::exit_percentage`; `None` means a full exit. It
///   is clamped to `[1, 100]` because `execute_sell` clamps below 1% and
///   `partial_close_position` refuses anything outside `(0, 100)`.
/// * `partial_exit_enabled` is `positions.partial_exit_enabled` — an AUTO-TRADER policy
///   switch, never a capability switch for the user.
pub(super) fn resolve_exit_size(
    reason: &TradeReason,
    requested_pct: Option<f64>,
    partial_exit_enabled: bool,
) -> ExitSize {
    let exit_percentage = requested_pct.unwrap_or(100.0).clamp(1.0, 100.0);

    // A USER-initiated exit already carries an explicit percentage: the user picked it
    // in the trade dialog. `positions.partial_exit_enabled` decides whether the
    // AUTO-TRADER may take partial profits on its own — it must never be allowed to
    // silently upgrade the user's 25% take-profit into a FULL exit, which sells the
    // entire position without asking. That is exactly what this did before, with no
    // error and no warning: turning off auto partial exits made manual take-profit
    // impossible, in the most destructive way possible.
    let is_user_exit = matches!(
        reason,
        TradeReason::ManualExit | TradeReason::ForceSell | TradeReason::CopySell
    );

    // Emergencies must fully liquidate regardless of any percentage.
    // Note: StopLoss is NOT an emergency exit - it respects stop_loss_allow_partial config.
    // ForceSell is NOT one either: it means "bypass the safety checks", not "sell
    // everything" — a force sell with a percentage is still a partial exit.
    let is_emergency_exit = matches!(
        reason,
        TradeReason::Blacklisted | TradeReason::RiskManagement
    );

    if (partial_exit_enabled || is_user_exit) && !is_emergency_exit && exit_percentage < 100.0 {
        ExitSize::Partial(exit_percentage)
    } else {
        ExitSize::Full
    }
}

/// Execute a sell trade
pub async fn execute_sell(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
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

    // Determine exit type based on configuration and reason
    let partial_exit_enabled = with_config(|cfg| cfg.positions.partial_exit_enabled);
    let exit_size = resolve_exit_size(
        &decision.reason,
        decision.exit_percentage,
        partial_exit_enabled,
    );

    let exit_reason = format!("{:?}", decision.reason);

    if let ExitSize::Partial(exit_percentage) = exit_size {
        match positions::partial_close_position(
            &decision.mint,
            exit_percentage,
            &exit_reason.clone(),
            decision.slippage_pct,
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
                    decision.price_sol.unwrap_or_default(),
                    0.0, // Exit size will be calculated by verification
                    decision.position_id.clone(),
                ))
            }
            Err(e) => {
                let step = e.trade_step();
                let error = format!("Partial sell execution failed: {e}");
                logger::error(LogTag::Trader, &error);
                Ok(TradeResult::failure_at(decision.clone(), step, error, 0))
            }
        }
    } else {
        // Full exit (either disabled, emergency exit, or 100%)
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
                    decision.price_sol.unwrap_or_default(),
                    0.0, // Exit size will be calculated by verification
                    decision.position_id.clone(),
                ))
            }
            Err(e) => {
                let step = e.trade_step();
                let error = format!("Full sell execution failed: {e}");
                logger::error(LogTag::Trader, &error);
                Ok(TradeResult::failure_at(decision.clone(), step, error, 0))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! Pure unit tests for exit-size routing. Co-located because `resolve_exit_size` is
    //! `pub(super)` and unreachable from the `tests/` integration crates.
    use super::*;

    #[test]
    fn no_percentage_means_sell_everything() {
        assert_eq!(
            resolve_exit_size(&TradeReason::TakeProfit, None, true),
            ExitSize::Full
        );
    }

    #[test]
    fn an_explicit_hundred_percent_is_a_full_exit() {
        // 100% must take the FULL-close path, which sizes off the on-chain balance and
        // leaves no dust — not the partial path, which sizes off a percentage.
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(100.0), true),
            ExitSize::Full
        );
    }

    #[test]
    fn the_auto_trader_takes_partials_only_when_they_are_enabled() {
        assert_eq!(
            resolve_exit_size(&TradeReason::StopLoss, Some(25.0), true),
            ExitSize::Partial(25.0)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::StopLoss, Some(25.0), false),
            ExitSize::Full
        );
    }

    #[test]
    fn a_manual_partial_survives_the_auto_partial_switch_being_off() {
        // The destructive bug this guards: `partial_exit_enabled` is an AUTO-TRADER
        // policy switch. With it off, a user's manual 25% take-profit was silently
        // upgraded to a FULL exit — the whole position sold, no error, no warning.
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(25.0), false),
            ExitSize::Partial(25.0)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::ForceSell, Some(25.0), false),
            ExitSize::Partial(25.0)
        );
    }

    #[test]
    fn a_copy_partial_survives_the_auto_partial_switch_being_off() {
        assert_eq!(
            resolve_exit_size(&TradeReason::CopySell, Some(30.0), false),
            ExitSize::Partial(30.0)
        );
    }

    #[test]
    fn an_emergency_always_liquidates_the_whole_position() {
        // A blacklisted or catastrophically losing token must leave the wallet
        // entirely, whatever percentage the decision happens to carry.
        for reason in [TradeReason::Blacklisted, TradeReason::RiskManagement] {
            assert_eq!(
                resolve_exit_size(&reason, Some(10.0), true),
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
            resolve_exit_size(&TradeReason::ForceSell, Some(30.0), true),
            ExitSize::Partial(30.0)
        );
    }

    #[test]
    fn a_stop_loss_is_not_an_emergency_either() {
        // Stop loss respects `stop_loss_allow_partial`, which reaches here as an
        // explicit percentage on the decision.
        assert_eq!(
            resolve_exit_size(&TradeReason::StopLoss, Some(50.0), true),
            ExitSize::Partial(50.0)
        );
    }

    #[test]
    fn the_percentage_is_clamped_into_a_submittable_range() {
        // Below 1% would round to a zero token amount on many balances, and above 100%
        // cannot be sold at all. `partial_close_position` refuses both outright.
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(0.4), true),
            ExitSize::Partial(1.0)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(-5.0), true),
            ExitSize::Partial(1.0)
        );
        assert_eq!(
            resolve_exit_size(&TradeReason::ManualExit, Some(250.0), true),
            ExitSize::Full
        );
    }

    #[test]
    fn every_automatic_exit_reason_routes_by_config_alone() {
        // None of these are user-initiated, so with auto partials off they must all
        // become full exits — and with it on, all honour their percentage.
        let automatic = [
            TradeReason::TakeProfit,
            TradeReason::StopLoss,
            TradeReason::TrailingStop,
            TradeReason::TimeOverride,
            TradeReason::StrategyExit,
            TradeReason::AiExit,
        ];
        for reason in automatic {
            assert_eq!(
                resolve_exit_size(&reason, Some(40.0), false),
                ExitSize::Full,
                "{reason:?} with auto partials off"
            );
            assert_eq!(
                resolve_exit_size(&reason, Some(40.0), true),
                ExitSize::Partial(40.0),
                "{reason:?} with auto partials on"
            );
        }
    }
}
