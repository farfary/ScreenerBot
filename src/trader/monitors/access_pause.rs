// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Live trading pause on chain access: whether live trades may run now, and one notification per pause while live positions are open.

use crate::chains::access::{service_state, ServiceState};
use crate::chains::ChainId;
use crate::events::{record_trader_event, with_text, Severity};
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use crate::telegram::notifier::queue_notification;
use crate::telegram::types::Notification;
use serde_json::json;

/// What one observation of the access state asks of the monitor.
#[derive(Debug, PartialEq, Eq)]
enum PauseStep {
    /// Nothing changed that needs telling.
    Quiet,
    /// The pause began, or live positions appeared during it: notify once.
    Notify,
    /// Access is ready again after a pause.
    Resumed,
}

/// Tracks one chain's pause across monitor cycles.
#[derive(Debug, Default)]
pub(super) struct AccessPause {
    paused: bool,
    notified: bool,
}

impl AccessPause {
    /// The step for an observation: `ready` is the access state, `live_open`
    /// whether the live book holds an open position.
    fn step(&mut self, ready: bool, live_open: bool) -> PauseStep {
        if ready {
            let resumed = self.paused;
            *self = Self::default();
            return if resumed {
                PauseStep::Resumed
            } else {
                PauseStep::Quiet
            };
        }
        self.paused = true;
        if live_open && !self.notified {
            self.notified = true;
            return PauseStep::Notify;
        }
        PauseStep::Quiet
    }

    /// Whether live trades on `chain` may run now: only while its chain access
    /// is ready (always under own-RPC access). The first paused observation
    /// with live positions open records an event and queues one notification.
    pub(super) async fn live_trades_allowed(&mut self, chain: ChainId, live_open: bool) -> bool {
        let state = service_state(chain);
        match self.step(state.ready(), live_open) {
            PauseStep::Quiet => {}
            PauseStep::Notify => notify_paused(chain, &state).await,
            PauseStep::Resumed => logger::info(
                LogTag::Trader,
                &format!("Live trading on {chain} resumes: chain access is ready"),
            ),
        }
        state.ready()
    }
}

async fn notify_paused(chain: ChainId, state: &ServiceState) {
    logger::warning(
        LogTag::Trader,
        &format!(
            "Live trading on {chain} paused with open positions: chain access is {}",
            state.key()
        ),
    );
    record_trader_event(
        "trading_paused",
        Severity::Warn,
        None,
        None,
        with_text(
            json!({
                "chain": chain.to_string(),
                "state": state.key(),
            }),
            &UiText::new(ids::EVENTS_TRADER_TRADING_PAUSED),
        ),
    )
    .await;
    queue_notification(Notification::trading_paused(state.ui_text()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pause_notifies_once_and_only_with_live_positions_open() {
        let mut pause = AccessPause::default();
        assert_eq!(pause.step(true, true), PauseStep::Quiet);

        assert_eq!(pause.step(false, false), PauseStep::Quiet);
        assert_eq!(
            pause.step(false, true),
            PauseStep::Notify,
            "a position opened during the pause is told once"
        );
        assert_eq!(pause.step(false, true), PauseStep::Quiet);
        assert_eq!(pause.step(false, false), PauseStep::Quiet);

        assert_eq!(pause.step(true, true), PauseStep::Resumed);
        assert_eq!(pause.step(true, true), PauseStep::Quiet);

        assert_eq!(
            pause.step(false, true),
            PauseStep::Notify,
            "every new pause is told again"
        );
    }
}
