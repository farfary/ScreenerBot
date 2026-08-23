//! Buy operation execution

use crate::logger::{self, LogTag};
use crate::positions::{self, PositionManagement, PositionOrigin, TradeOrigin};
use crate::trader::config;
use crate::trader::types::{TradeDecision, TradeReason, TradeResult};

/// Execute a buy trade (auto/strategy path).
///
/// Manual management is derived from the decision reason: manual/force buys are
/// protected from the auto-trader by default. Dashboard manual buys that need an
/// explicit choice call [`execute_buy_managed`] instead.
pub async fn execute_buy(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
    let is_manual = matches!(
        decision.reason,
        TradeReason::ManualEntry | TradeReason::ForceBuy
    );
    let origin = if is_manual {
        PositionOrigin::Manual
    } else {
        PositionOrigin::Auto {
            strategy_id: decision.strategy_id.clone(),
        }
    };
    let management = if is_manual {
        PositionManagement::UserOnly
    } else {
        PositionManagement::AutoTrader
    };
    execute_buy_managed(decision, origin, management).await
}

/// Execute a buy trade with explicit provenance and ownership.
///
/// The origin and ownership policy are persisted independently.
pub async fn execute_buy_managed(
    decision: &TradeDecision,
    origin: PositionOrigin,
    management: PositionManagement,
) -> crate::trader::Result<TradeResult> {
    // Check connectivity before executing trade - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute buy - Unhealthy endpoints: {unhealthy}");
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure(decision.clone(), error, 0));
    }

    logger::info(
        LogTag::Trader,
        &format!(
            "Executing buy for token {} (reason: {:?}, strategy: {:?})",
            decision.mint, decision.reason, decision.strategy_id
        ),
    );

    // Note: Trade size is read from config by open_position_direct
    // decision.size_sol is informational only - actual size comes from cfg.trader.trade_size_sol
    let trade_size_sol = decision
        .size_sol
        .unwrap_or_else(|| config::get_trade_size_sol());

    // Enforce maximum trade size limit
    let max_allowed =
        config::get_trade_size_sol() * crate::trader::constants::MAX_TRADE_SIZE_MULTIPLIER;
    let trade_size_sol = trade_size_sol.min(max_allowed);

    match positions::open_position_with_size(
        &decision.mint,
        trade_size_sol,
        origin,
        management,
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
                    decision.mint, trade_size_sol, transaction_signature
                ),
            );

            let mut result = TradeResult::success(
                decision.clone(),
                transaction_signature,
                submission.entry_price_sol,
                trade_size_sol,
                None, // Position ID will be set by verification
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

            let mut result = TradeResult::failure(decision.clone(), guard_msg, 0);
            result.capacity_guard_remaining = Some(remaining);
            Ok(result)
        }
        Err(e) => {
            let error = format!("Buy execution failed: {e}");
            logger::error(LogTag::Trader, &error);
            Ok(TradeResult::failure(decision.clone(), error, 0))
        }
    }
}

/// Execute a DCA (dollar cost averaging) buy
pub async fn execute_dca(decision: &TradeDecision) -> crate::trader::Result<TradeResult> {
    // Check connectivity before executing DCA - critical operation
    if let Some(unhealthy) = crate::connectivity::check_endpoints_healthy(&["rpc"]).await {
        let error = format!("Cannot execute DCA - Unhealthy endpoints: {unhealthy}");
        logger::error(LogTag::Trader, &error);
        return Ok(TradeResult::failure(decision.clone(), error, 0));
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

    // Determine DCA amount from decision, else the configured DCA size (a fraction of
    // the trade size). Never hardcode the fraction — `trader.dca_size_percentage` is
    // the single source of truth for it.
    let dca_amount_sol = decision.size_sol.unwrap_or_else(|| {
        config::get_trade_size_sol() * (config::get_dca_size_percentage() / 100.0)
    });

    // Call positions::add_to_position to handle DCA entry
    match positions::add_to_position(
        &decision.mint,
        dca_amount_sol,
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
                    decision.mint, dca_amount_sol, transaction_signature
                ),
            );

            Ok(TradeResult::success(
                decision.clone(),
                transaction_signature,
                decision.price_sol.unwrap_or_default(),
                dca_amount_sol,
                decision.position_id.clone(),
            ))
        }
        Err(e) => {
            let error = format!("DCA execution failed: {e}");
            logger::error(LogTag::Trader, &error);
            Ok(TradeResult::failure(decision.clone(), error, 0))
        }
    }
}
