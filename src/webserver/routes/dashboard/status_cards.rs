// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Home Auto Trader and Copy Trading status cards, read from the trader and copy owners.

use chrono::{DateTime, Utc};

use crate::positions::AutoTradeActivity;
use crate::trader::copy::control::{self, CopyTradingStatus};
use crate::trader::copy::workspace::{self, ActivityFilter};
use crate::trader::safety::PositionCapacity;
use crate::trader::TraderRunState;

use super::types::{AutoTraderCard, CopyTradingCard, FeatureState};

/// The Auto Trader card from the trader's run state, its position capacity and the
/// positions it opened and closed today (`None` when they could not be read).
/// `setup_incomplete` is the trader's live gate answer; Explore Mode reads as unavailable
/// too.
pub fn auto_trader_card(
    run_state: TraderRunState,
    setup_incomplete: bool,
    capacity: PositionCapacity,
    activity_today: Option<AutoTradeActivity>,
) -> AutoTraderCard {
    let state = match run_state {
        _ if setup_incomplete => FeatureState::Unavailable,
        TraderRunState::Explore => FeatureState::Unavailable,
        TraderRunState::ForceStopped | TraderRunState::Stopped => FeatureState::Stopped,
        TraderRunState::Waiting
        | TraderRunState::Idle
        | TraderRunState::EntryPaused
        | TraderRunState::Running => FeatureState::Running,
    };
    if state == FeatureState::Unavailable {
        return AutoTraderCard {
            state,
            open_positions: None,
            max_open_positions: None,
            entries_today: None,
            exits_today: None,
        };
    }
    AutoTraderCard {
        state,
        open_positions: Some(capacity.used),
        max_open_positions: Some(capacity.limit),
        entries_today: activity_today.map(|activity| activity.opened),
        exits_today: activity_today.map(|activity| activity.closed),
    }
}

/// The Copy Trading card. `setup_incomplete` is the copy gate's own answer; `status` and
/// `followed_wallets` are absent when the copy store could not be read.
pub fn copy_trading_card(
    setup_incomplete: bool,
    status: Option<&CopyTradingStatus>,
    followed_wallets: Option<usize>,
    last_copy_at: Option<DateTime<Utc>>,
) -> CopyTradingCard {
    if setup_incomplete {
        return CopyTradingCard {
            state: Some(FeatureState::Unavailable),
            followed_wallets: None,
            active_tasks: None,
            total_tasks: None,
            last_copy_at: None,
        };
    }
    CopyTradingCard {
        state: status.map(|status| {
            if status.is_copying() {
                FeatureState::Running
            } else {
                FeatureState::Stopped
            }
        }),
        followed_wallets,
        active_tasks: status.map(|status| status.active_tasks),
        total_tasks: status.map(|status| status.total_tasks),
        last_copy_at: last_copy_at.map(|at| at.to_rfc3339()),
    }
}

/// Read the Copy Trading card: the task set, the status built from it and the newest
/// copied fill, through the copy owners. Nothing is read while setup is incomplete.
pub async fn read_copy_trading_card() -> CopyTradingCard {
    let setup_incomplete = control::live_block_reason()
        == Some(control::CopyLiveBlock::Trader(
            crate::trader::LiveBlock::SetupIncomplete,
        ));
    if setup_incomplete {
        return copy_trading_card(true, None, None, None);
    }
    let tasks = match control::open_database().await {
        Ok(database) => database.list_tasks().await.ok(),
        Err(_) => None,
    };
    let status = tasks.as_deref().map(control::build_status);
    let followed = tasks.as_deref().map(control::followed_wallets);
    let last_copy_at = workspace::activity_page(
        None,
        ActivityFilter {
            filter: Some("fills".to_owned()),
            limit: Some(1),
            ..ActivityFilter::default()
        },
    )
    .await
    .ok()
    .and_then(|page| page.activity.first().map(|row| row.created_at));
    copy_trading_card(false, status.as_ref(), followed, last_copy_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today(opened: i64, closed: i64) -> AutoTradeActivity {
        AutoTradeActivity { opened, closed }
    }

    fn copy(
        enabled: bool,
        blocked_reason: Option<&'static str>,
        active: usize,
    ) -> CopyTradingStatus {
        CopyTradingStatus {
            enabled,
            live_available: true,
            blocked_reason,
            default_mode: "paper".to_owned(),
            default_slippage_pct: 1.0,
            force_stop_blocks: true,
            total_tasks: 4,
            active_tasks: active,
            paper_tasks: active,
            live_tasks: 0,
            resumable_tasks: 0,
            resumable_live_tasks: 0,
        }
    }

    const CAPACITY: PositionCapacity = PositionCapacity { used: 2, limit: 5 };

    #[test]
    fn a_running_trader_reports_capacity_and_the_day() {
        let card = auto_trader_card(TraderRunState::Running, false, CAPACITY, Some(today(3, 1)));
        assert_eq!(card.state, FeatureState::Running);
        assert_eq!(card.open_positions, Some(2));
        assert_eq!(card.max_open_positions, Some(5));
        assert_eq!(card.entries_today, Some(3));
        assert_eq!(card.exits_today, Some(1));
    }

    #[test]
    fn a_stopped_trader_with_unreadable_stats_reports_them_absent() {
        let card = auto_trader_card(TraderRunState::Stopped, false, CAPACITY, None);
        assert_eq!(card.state, FeatureState::Stopped);
        assert_eq!(card.open_positions, Some(2));
        assert_eq!(card.entries_today, None);
        assert_eq!(card.exits_today, None);
    }

    #[test]
    fn an_unavailable_trader_reports_no_figures() {
        for (run_state, setup_incomplete) in [
            (TraderRunState::Stopped, true),
            (TraderRunState::Running, true),
            (TraderRunState::Explore, false),
        ] {
            let card = auto_trader_card(run_state, setup_incomplete, CAPACITY, Some(today(3, 1)));
            assert_eq!(card.state, FeatureState::Unavailable, "{run_state:?}");
            assert_eq!(card.open_positions, None);
            assert_eq!(card.max_open_positions, None);
            assert_eq!(card.entries_today, None);
        }
    }

    /// The card reads the trader's one run state: an enabled trader runs whatever its
    /// monitors or the loss limit do, and the emergency stop stops it.
    #[test]
    fn the_card_state_follows_the_trader_run_state() {
        for (run_state, expected) in [
            (TraderRunState::ForceStopped, FeatureState::Stopped),
            (TraderRunState::Stopped, FeatureState::Stopped),
            (TraderRunState::Waiting, FeatureState::Running),
            (TraderRunState::Idle, FeatureState::Running),
            (TraderRunState::EntryPaused, FeatureState::Running),
            (TraderRunState::Running, FeatureState::Running),
        ] {
            let card = auto_trader_card(run_state, false, CAPACITY, None);
            assert_eq!(card.state, expected, "{run_state:?}");
            assert_eq!(card.open_positions, Some(2), "{run_state:?}");
        }
    }

    #[test]
    fn copy_states_follow_the_copy_owner() {
        let at = Utc::now();
        let running = copy_trading_card(false, Some(&copy(true, None, 2)), Some(2), Some(at));
        assert_eq!(running.state, Some(FeatureState::Running));
        assert_eq!(running.followed_wallets, Some(2));
        assert_eq!(running.active_tasks, Some(2));
        assert_eq!(running.total_tasks, Some(4));
        assert_eq!(running.last_copy_at, Some(at.to_rfc3339()));

        let halted = copy_trading_card(
            false,
            Some(&copy(true, Some("force_stop"), 2)),
            Some(2),
            None,
        );
        assert_eq!(halted.state, Some(FeatureState::Stopped));
        assert_eq!(halted.last_copy_at, None);
        let off = copy_trading_card(false, Some(&copy(false, None, 2)), Some(2), None);
        assert_eq!(off.state, Some(FeatureState::Stopped));
    }

    #[test]
    fn unavailable_or_unreadable_copy_reports_no_figures() {
        let unavailable = copy_trading_card(true, Some(&copy(true, None, 2)), Some(2), None);
        assert_eq!(unavailable.state, Some(FeatureState::Unavailable));
        assert_eq!(unavailable.followed_wallets, None);
        assert_eq!(unavailable.active_tasks, None);

        let unreadable = copy_trading_card(false, None, None, None);
        assert_eq!(unreadable.state, None);
        assert_eq!(unreadable.total_tasks, None);
    }

    /// The serialized card keeps every absent figure as null, so the page renders "—".
    #[test]
    fn absent_figures_serialize_as_null() {
        let card = serde_json::to_value(auto_trader_card(
            TraderRunState::Stopped,
            false,
            CAPACITY,
            None,
        ))
        .unwrap();
        assert_eq!(card["state"], "stopped");
        assert!(card["entries_today"].is_null());
        let card = serde_json::to_value(copy_trading_card(false, None, None, None)).unwrap();
        assert!(card["state"].is_null());
        assert!(card["last_copy_at"].is_null());
    }
}
