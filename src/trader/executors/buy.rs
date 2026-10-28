// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Buy operation execution.

use crate::logger::{self, LogTag};
use crate::positions::{self, PositionOrigin, TradeOrigin};
use crate::strategies::types::{DecisionLapse, Strategy};
use crate::trader::config;
use crate::trader::paper::TradingMode;
use crate::trader::types::{TradeDecision, TradeResult, TradeStep, Withdrawal};
use crate::trader::TradeReason;

use super::ExecutionContext;

/// The size of an entry: the decision's own size, or the configured trade size, capped
/// at the configured maximum multiple of the trade size.
pub(in crate::trader) fn entry_size_native(decision: &TradeDecision) -> f64 {
    entry_size_of(decision.size_native)
}

/// The size an entry carrying `size_native` buys: that size, or the configured trade size
/// when it carries none, capped at the configured maximum multiple of the trade size.
pub(in crate::trader) fn entry_size_of(size_native: Option<f64>) -> f64 {
    let configured = config::get_trade_size_native();
    size_native
        .unwrap_or(configured)
        .min(configured * crate::trader::constants::MAX_TRADE_SIZE_MULTIPLIER)
}

/// The entry strategy that decided a buy opened with `context`, when one did.
pub(super) fn deciding_strategy(context: &ExecutionContext) -> Option<&str> {
    match &context.origin {
        PositionOrigin::Auto { strategy_id } => strategy_id.as_deref(),
        _ => None,
    }
}

/// Why a live buy decided at `decided_at` no longer stands, judged by the deciding
/// strategy as read from the store just before the buy, or `None` while it does: the
/// strategy's own rule ([`Strategy::decision_lapse`] on the live book, for a decision that
/// opens a round), with a removed strategy counted as one that left the book.
pub(super) fn live_buy_lapse(
    strategy: Option<&Strategy>,
    decided_at: chrono::DateTime<chrono::Utc>,
) -> Option<DecisionLapse> {
    match strategy {
        Some(strategy) => strategy.decision_lapse(TradingMode::Live, true, decided_at),
        None => Some(DecisionLapse::LeftBook),
    }
}

/// The refusal of a live strategy buy whose decision no longer stands, read just before
/// the buy, or `None` when it still does. The rule is the paper executor's for a paper
/// strategy buy: automatic entries must still be on ([`config::is_entry_monitor_enabled`],
/// the trader and its entry switch) and the deciding strategy's decision must still stand
/// on the live book ([`live_buy_lapse`]). Entries switched off, a strategy that left the
/// live book or a rules change after the decision withdraws it; a store that cannot answer
/// fails it.
async fn strategy_buy_refusal(decision: &TradeDecision, strategy_id: &str) -> Option<TradeResult> {
    let refused = |error: String, withdrawn: Option<Withdrawal>| {
        let mut result = TradeResult::failure_at(decision.clone(), TradeStep::Validation, error, 0);
        result.withdrawn = withdrawn;
        Some(result)
    };
    if !config::is_entry_monitor_enabled() {
        return refused(
            format!(
                "Cannot execute buy - the trader was switched off, or its automatic entries were, after strategy {strategy_id} decided the entry"
            ),
            Some(Withdrawal::TraderStopped),
        );
    }
    let strategy = match crate::strategies::db::get_strategy_async(strategy_id).await {
        Ok(strategy) => strategy,
        Err(e) => {
            return refused(
                format!(
                    "Cannot execute buy - strategy {strategy_id} could not be read to confirm it still trades live: {e}"
                ),
                None,
            )
        }
    };
    let Some(lapse) = live_buy_lapse(strategy.as_ref(), decision.timestamp) else {
        return None;
    };
    match lapse {
        DecisionLapse::LeftBook => refused(
            format!(
                "Cannot execute buy - strategy {strategy_id} was switched off, returned to paper or removed after the entry was decided"
            ),
            Some(Withdrawal::StrategyStoppedTrading),
        ),
        DecisionLapse::RulesChanged => refused(
            format!(
                "Cannot execute buy - the rules of strategy {strategy_id} changed after the entry was decided"
            ),
            Some(Withdrawal::RulesChanged),
        ),
    }
}

/// Executes a live buy, opening its round with the provenance and ownership in
/// `context`, persisted independently. A strategy buy is refused before anything is
/// quoted or sent when its decision no longer stands ([`strategy_buy_refusal`]).
pub(super) async fn execute_buy(
    decision: &TradeDecision,
    context: ExecutionContext,
) -> crate::trader::Result<TradeResult> {
    super::require_live(decision)?;
    if let Some(strategy_id) = deciding_strategy(&context) {
        if let Some(refusal) = strategy_buy_refusal(decision, strategy_id).await {
            return Ok(refusal);
        }
    }
    let Some(_active_trade) = crate::global::begin_trade() else {
        return Ok(TradeResult::failure_at(
            decision.clone(),
            TradeStep::Validation,
            "Cannot execute buy - an application update is restarting the trading engine"
                .to_owned(),
            0,
        ));
    };
    // Check connectivity before executing trade - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute buy - Unhealthy endpoints: {unhealthy}");
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
            "Executing buy for token {} (reason: {:?}, strategy: {:?})",
            decision.mint, decision.reason, decision.strategy_id
        ),
    );

    let trade_size_native = entry_size_native(decision);

    match positions::open_position_with_size(
        &decision.mint,
        trade_size_native,
        context.origin,
        context.management,
        decision.slippage_pct,
    )
    .await
    {
        Ok(submission) => {
            let transaction_signature = submission.transaction_signature;
            logger::info(
                LogTag::Trader,
                &format!(
                    "Buy executed: {} | ~{} SOL | TX: {}",
                    decision.mint, trade_size_native, transaction_signature
                ),
            );

            let mut result = TradeResult::success(
                decision.clone(),
                transaction_signature,
                Some(submission.entry_price_native),
                Some(trade_size_native),
                submission.position_id.map(|id| id.to_string()),
            );
            result.confirmation_pending = submission.confirmation_pending;
            Ok(result)
        }
        Err(positions::Error::SlotUnavailable { remaining }) => {
            let guard_msg = format!(
                "position_slots_unavailable: global position limit reached (remaining permits: {remaining})"
            );

            logger::debug(
                LogTag::Trader,
                &format!(
                    "Entry guard engaged for {} – max open positions reached (permits left: {})",
                    decision.mint, remaining
                ),
            );

            let mut result =
                TradeResult::failure_at(decision.clone(), TradeStep::Validation, guard_msg, 0);
            result.capacity_guard_remaining = Some(remaining);
            Ok(result)
        }
        Err(e) => {
            let error = format!("Buy execution failed: {e}");
            logger::error(LogTag::Trader, &error);
            Ok(TradeResult::failure_from(decision.clone(), &e, error))
        }
    }
}

/// Execute a live DCA (dollar cost averaging) buy
pub(super) async fn execute_dca(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
    super::require_live(decision)?;
    // Every caller sizes its add: the DCA evaluator from the ladder, a manual add from
    // its request or the ladder's default. A decision without a size is refused before
    // anything else runs, rather than sized here from settings that do not describe it.
    let Some(dca_amount_native) = decision.size_native else {
        return Err(crate::trader::Error::AddSizeRequired {
            mint: decision.mint.clone(),
        });
    };
    let Some(_active_trade) = crate::global::begin_trade() else {
        return Ok(TradeResult::failure_at(
            decision.clone(),
            TradeStep::Validation,
            "Cannot execute DCA - an application update is restarting the trading engine"
                .to_owned(),
            0,
        ));
    };
    // Check connectivity before executing DCA - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute DCA - Unhealthy endpoints: {unhealthy}");
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure_at(
            decision.clone(),
            TradeStep::Validation,
            error,
            0,
        ));
    }

    // A manual "Add to Position" is the user overriding the bot, so the auto-trader's
    // DCA policy (enabled / max count / cooldown) does not gate it. Everything else —
    // strategies, the DCA evaluator — is the bot acting on its own and stays bound by
    // that config.
    let origin = if matches!(
        decision.reason,
        TradeReason::ManualEntry | TradeReason::ForceBuy
    ) {
        TradeOrigin::Manual
    } else {
        TradeOrigin::Auto
    };

    logger::info(
        LogTag::Trader,
        &format!(
            "Executing DCA for position {} token {} (origin: {:?})",
            decision.position_id.as_deref().unwrap_or("unknown"),
            decision.mint,
            origin
        ),
    );

    // Call positions::add_to_position to handle DCA entry
    match positions::add_to_position(
        &decision.mint,
        dca_amount_native,
        decision.slippage_pct,
        origin,
    )
    .await
    {
        Ok(transaction_signature) => {
            logger::info(
                LogTag::Trader,
                &format!(
                    "DCA executed: {} | {} SOL | TX: {}",
                    decision.mint, dca_amount_native, transaction_signature
                ),
            );

            Ok(TradeResult::success(
                decision.clone(),
                transaction_signature,
                None,
                Some(dca_amount_native),
                decision.position_id.clone(),
            ))
        }
        Err(e) => {
            let error = format!("DCA execution failed: {e}");
            logger::error(LogTag::Trader, &error);
            Ok(TradeResult::failure_from(decision.clone(), &e, error))
        }
    }
}
