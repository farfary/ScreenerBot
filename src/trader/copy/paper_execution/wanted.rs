// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Whether a paper trade of a copy round is still wanted when the paper executor fills it.

use crate::trader::paper::TradingMode;
use crate::trader::types::{TradeDecision, TradeReason};

use super::super::{CopyTask, ExitMode};

/// Whether copy task `task_id` still wants the paper trade `decision` of one of its
/// rounds. A copy buy needs copy trading on and the task enabled in Paper with the
/// slippage the decision was sized with; a mirror sell needs the task in Paper with
/// mirrored exits and the same slippage; any other trade of a copy round (exit rules,
/// automatic additions) follows the Auto Trader. A copy store read failure refuses.
pub async fn paper_trade_wanted(task_id: i64, decision: &TradeDecision) -> bool {
    let task = match decision.reason {
        TradeReason::CopyBuy | TradeReason::CopySell => {
            match super::super::control::open_database().await {
                Ok(database) => match database.get_task(task_id).await {
                    Ok(task) => task,
                    Err(_) => return false,
                },
                Err(_) => return false,
            }
        }
        _ => None,
    };
    let (copy_enabled, trader_enabled) = (
        crate::config::with_config(|config| config.copy_trading.enabled),
        crate::trader::config::is_trader_enabled(),
    );
    paper_trade_wanted_with(decision, task.as_ref(), copy_enabled, trader_enabled)
}

/// [`paper_trade_wanted`] over the task as stored (`None` when it is gone), whether
/// copy trading is on, and whether the Auto Trader runs.
pub(super) fn paper_trade_wanted_with(
    decision: &TradeDecision,
    task: Option<&CopyTask>,
    copy_enabled: bool,
    trader_enabled: bool,
) -> bool {
    let same_slippage = |task: &CopyTask| {
        decision
            .slippage_pct
            .is_some_and(|slippage| slippage == task.slippage_pct)
    };
    match decision.reason {
        TradeReason::CopyBuy => {
            copy_enabled
                && task.is_some_and(|task| {
                    task.enabled && task.mode == TradingMode::Paper && same_slippage(task)
                })
        }
        TradeReason::CopySell => task.is_some_and(|task| {
            task.mode == TradingMode::Paper
                && task.exit_mode != ExitMode::BuyOnly
                && same_slippage(task)
        }),
        _ => trader_enabled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::positions::Book;
    use crate::trader::types::{TradeAction, TradePriority};

    fn task() -> CopyTask {
        CopyTask {
            id: 1,
            chain: crate::chains::ChainId::Solana,
            target_address: "target".to_owned(),
            label: None,
            enabled: true,
            mode: TradingMode::Paper,
            sizing: crate::trader::copy::SizingMode::Fixed { sol: 0.1 },
            exit_mode: ExitMode::Mirror,
            exit_policy_overrides: Default::default(),
            max_native_per_trade: 0.2,
            max_native_per_token: 1.0,
            total_budget_native: 5.0,
            min_target_trade_native: None,
            max_target_trade_native: None,
            buy_once_per_token: false,
            slippage_pct: 3.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            require_filter_pass: None,
            copy_stable_quoted: None,
            pause_reason: None,
            paused_at: None,
            paper_started_at: None,
        }
    }

    fn decision(reason: TradeReason, slippage_pct: Option<f64>) -> TradeDecision {
        TradeDecision {
            position_id: None,
            mint: "Mint".to_owned(),
            action: match reason {
                TradeReason::CopyBuy => TradeAction::Buy,
                _ => TradeAction::Sell,
            },
            reason,
            strategy_id: None,
            book: Book::Paper,
            timestamp: chrono::Utc::now(),
            priority: TradePriority::High,
            price_native: None,
            size_native: None,
            exit_percentage: None,
            slippage_pct,
            take_profit_level: None,
            paper_leg_key: None,
        }
    }

    #[test]
    fn a_copy_buy_needs_copy_trading_and_the_task_enabled_in_paper_at_its_slippage() {
        let buy = decision(TradeReason::CopyBuy, Some(3.0));
        let task = task();
        assert!(paper_trade_wanted_with(&buy, Some(&task), true, false));
        assert!(!paper_trade_wanted_with(&buy, Some(&task), false, true));
        assert!(!paper_trade_wanted_with(&buy, None, true, true));
        let disabled = CopyTask {
            enabled: false,
            ..task.clone()
        };
        assert!(!paper_trade_wanted_with(&buy, Some(&disabled), true, true));
        let live = CopyTask {
            mode: TradingMode::Live,
            ..task.clone()
        };
        assert!(!paper_trade_wanted_with(&buy, Some(&live), true, true));
        let other_slippage = decision(TradeReason::CopyBuy, Some(5.0));
        assert!(!paper_trade_wanted_with(
            &other_slippage,
            Some(&task),
            true,
            true
        ));
        let without_size = decision(TradeReason::CopyBuy, None);
        assert!(!paper_trade_wanted_with(
            &without_size,
            Some(&task),
            true,
            true
        ));
    }

    #[test]
    fn a_mirror_sell_needs_the_task_in_paper_with_mirrored_exits_at_its_slippage() {
        let sell = decision(TradeReason::CopySell, Some(3.0));
        let task = task();
        assert!(paper_trade_wanted_with(&sell, Some(&task), false, false));
        let paused = CopyTask {
            enabled: false,
            ..task.clone()
        };
        assert!(paper_trade_wanted_with(&sell, Some(&paused), false, false));
        let buy_only = CopyTask {
            exit_mode: ExitMode::BuyOnly,
            ..task.clone()
        };
        assert!(!paper_trade_wanted_with(&sell, Some(&buy_only), true, true));
        let live = CopyTask {
            mode: TradingMode::Live,
            ..task.clone()
        };
        assert!(!paper_trade_wanted_with(&sell, Some(&live), true, true));
        assert!(!paper_trade_wanted_with(&sell, None, true, true));
        let other_slippage = decision(TradeReason::CopySell, Some(1.0));
        assert!(!paper_trade_wanted_with(
            &other_slippage,
            Some(&task),
            true,
            true
        ));
    }

    #[test]
    fn any_other_trade_of_a_copy_round_follows_the_auto_trader() {
        for reason in [
            TradeReason::StopLoss,
            TradeReason::TakeProfit,
            TradeReason::DCAScheduled,
        ] {
            let other = decision(reason, None);
            assert!(paper_trade_wanted_with(&other, None, false, true));
            assert!(!paper_trade_wanted_with(&other, Some(&task()), true, false));
        }
    }
}
