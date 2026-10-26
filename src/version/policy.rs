// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The rules that decide when an update may act on its own.
//!
//! Downloading in the background is cheap and always safe. *Applying* an update
//! is not: it restarts the trading engine. So a staged core is only activated by
//! itself when the owner allows automatic installation and nothing is riding on
//! the current process. Otherwise the update simply waits — a staged core costs
//! nothing to hold and activates at the next launch regardless.

use serde::Serialize;

use super::types::*;
use crate::config::with_config;

/// The update-relevant settings, read once so no lock is held across an await.
#[derive(Debug, Clone, Copy)]
pub(super) struct UpdatePolicy {
    pub auto_check: bool,
    pub check_interval_secs: u64,
    pub auto_download: bool,
    pub auto_install: bool,
    pub defer_while_trading: bool,
    pub notify_telegram: bool,
}

impl UpdatePolicy {
    pub(super) fn load() -> Self {
        let (
            auto_check,
            check_interval_hours,
            auto_download,
            auto_install,
            defer_while_trading,
            notify_telegram,
        ) = with_config(|config| {
            (
                config.updates.auto_check,
                config.updates.check_interval_hours,
                config.updates.auto_download,
                config.updates.auto_install,
                config.updates.defer_while_trading,
                config.updates.notify_telegram,
            )
        });

        Self {
            auto_check,
            // Clamp defensively: a hand-edited 0 would spin the check loop.
            check_interval_secs: check_interval_hours.clamp(1, 168) * 3600,
            auto_download,
            auto_install,
            defer_while_trading,
            notify_telegram,
        }
    }
}

/// Whether a verified update may be applied right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyReadiness {
    /// Apply it — restart the backend onto the staged core.
    Ready,
    /// Hold it, for a reason the dashboard shows verbatim.
    Deferred(DeferReason),
}

impl ApplyReadiness {
    pub fn is_ready(self) -> bool {
        matches!(self, ApplyReadiness::Ready)
    }

    pub fn reason(self) -> Option<DeferReason> {
        match self {
            ApplyReadiness::Ready => None,
            ApplyReadiness::Deferred(reason) => Some(reason),
        }
    }
}

/// Decide whether the automatic path may apply an update of this kind.
pub async fn apply_readiness(kind: UpdateKind) -> ApplyReadiness {
    let policy = UpdatePolicy::load();
    let open_positions = crate::positions::state::get_open_positions_count().await;
    decide(
        kind,
        policy.auto_install,
        policy.defer_while_trading,
        open_positions,
        trade_in_flight(),
    )
}

/// A trade submission or a running tool that a restart would cut short.
fn trade_in_flight() -> bool {
    crate::global::are_tools_active() || crate::global::are_trades_active()
}

/// Why restarting the process now would interrupt something.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum RestartBlocker {
    /// A trade submission is between its start and its booked outcome.
    TradeInFlight,
    /// A tool is running.
    ToolInFlight,
    /// Positions are open, counted in memory or in the positions store.
    OpenPositions { count: usize },
    /// Setup has completed but the positions store is not open yet.
    PositionsLoading,
    /// The positions store could not be read, so open positions cannot be ruled out.
    PositionsUnreadable,
}

impl std::fmt::Display for RestartBlocker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TradeInFlight => write!(f, "a trade is in flight"),
            Self::ToolInFlight => write!(f, "a tool is running"),
            Self::OpenPositions { count } => write!(f, "{count} position(s) are open"),
            Self::PositionsLoading => write!(f, "positions are still loading"),
            Self::PositionsUnreadable => write!(f, "the positions store could not be read"),
        }
    }
}

/// What the positions store says about open positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StoredOpenPositions {
    Count(usize),
    NotLoaded,
    Unreadable,
}

/// The inputs of the restart-safety decision, explicit so the decision is testable.
#[derive(Debug, Clone, Copy)]
pub(super) struct RestartInputs {
    pub trades_active: bool,
    pub tools_active: bool,
    pub open_in_memory: usize,
    pub stored: StoredOpenPositions,
    pub initialization_complete: bool,
}

/// The first reason a restart would interrupt something, if any: no trade or tool in
/// flight and no open position, counted both in memory and in the positions store,
/// whatever the update settings say. A store that is not open yet is safe only while
/// setup has not completed (no wallet, so nothing can be open); after setup it means the
/// process is still loading its positions.
pub async fn restart_blocker() -> Option<RestartBlocker> {
    let trades_active = crate::global::are_trades_active();
    let tools_active = crate::global::are_tools_active();
    let open_in_memory = crate::positions::state::get_open_positions_count().await;
    let stored = match crate::positions::db::get_open_positions().await {
        Ok(open) => StoredOpenPositions::Count(open.len()),
        Err(crate::positions::Error::NotInitialised) => StoredOpenPositions::NotLoaded,
        Err(_) => StoredOpenPositions::Unreadable,
    };
    blocker(RestartInputs {
        trades_active,
        tools_active,
        open_in_memory,
        stored,
        initialization_complete: crate::global::is_initialization_complete(),
    })
}

/// Whether restarting the process now would interrupt nothing.
pub async fn is_restart_safe() -> bool {
    restart_blocker().await.is_none()
}

/// The restart-safety decision itself.
pub(super) fn blocker(inputs: RestartInputs) -> Option<RestartBlocker> {
    if inputs.trades_active {
        return Some(RestartBlocker::TradeInFlight);
    }
    if inputs.tools_active {
        return Some(RestartBlocker::ToolInFlight);
    }
    if inputs.open_in_memory > 0 {
        return Some(RestartBlocker::OpenPositions {
            count: inputs.open_in_memory,
        });
    }
    match inputs.stored {
        StoredOpenPositions::Count(0) => None,
        StoredOpenPositions::Count(count) => Some(RestartBlocker::OpenPositions { count }),
        StoredOpenPositions::NotLoaded if inputs.initialization_complete => {
            Some(RestartBlocker::PositionsLoading)
        }
        StoredOpenPositions::NotLoaded => None,
        StoredOpenPositions::Unreadable => Some(RestartBlocker::PositionsUnreadable),
    }
}

/// The decision itself, with every input explicit so it is testable.
pub(super) fn decide(
    kind: UpdateKind,
    auto_install: bool,
    defer_while_trading: bool,
    open_positions: usize,
    tools_active: bool,
) -> ApplyReadiness {
    if !kind.is_silent() {
        return ApplyReadiness::Deferred(DeferReason::NeedsInstaller);
    }
    if !auto_install {
        return ApplyReadiness::Deferred(DeferReason::AutomaticInstallDisabled);
    }
    if defer_while_trading && (open_positions > 0 || tools_active) {
        return ApplyReadiness::Deferred(DeferReason::TradingActive);
    }
    ApplyReadiness::Ready
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_update_is_never_applied_automatically() {
        assert_eq!(
            decide(UpdateKind::Full, true, false, 0, false),
            ApplyReadiness::Deferred(DeferReason::NeedsInstaller)
        );
    }

    #[test]
    fn automatic_installation_can_be_switched_off() {
        assert_eq!(
            decide(UpdateKind::Core, false, true, 0, false),
            ApplyReadiness::Deferred(DeferReason::AutomaticInstallDisabled)
        );
    }

    #[test]
    fn open_positions_and_running_tools_hold_the_restart() {
        assert_eq!(
            decide(UpdateKind::Core, true, true, 2, false),
            ApplyReadiness::Deferred(DeferReason::TradingActive)
        );
        assert_eq!(
            decide(UpdateKind::Core, true, true, 0, true),
            ApplyReadiness::Deferred(DeferReason::TradingActive)
        );
        // Deferring is itself optional; without it the restart goes ahead.
        assert_eq!(
            decide(UpdateKind::Core, true, false, 2, true),
            ApplyReadiness::Ready
        );
    }

    #[test]
    fn an_idle_process_applies_a_core_update_on_its_own() {
        assert_eq!(
            decide(UpdateKind::Core, true, true, 0, false),
            ApplyReadiness::Ready
        );
    }

    const IDLE: RestartInputs = RestartInputs {
        trades_active: false,
        tools_active: false,
        open_in_memory: 0,
        stored: StoredOpenPositions::Count(0),
        initialization_complete: true,
    };

    #[test]
    fn an_idle_process_has_no_restart_blocker() {
        assert_eq!(blocker(IDLE), None);
    }

    #[test]
    fn work_in_flight_blocks_a_restart_with_its_reason() {
        assert_eq!(
            blocker(RestartInputs {
                trades_active: true,
                tools_active: true,
                ..IDLE
            }),
            Some(RestartBlocker::TradeInFlight)
        );
        assert_eq!(
            blocker(RestartInputs {
                tools_active: true,
                ..IDLE
            }),
            Some(RestartBlocker::ToolInFlight)
        );
    }

    #[test]
    fn open_positions_in_memory_or_in_the_store_block_a_restart() {
        assert_eq!(
            blocker(RestartInputs {
                open_in_memory: 3,
                ..IDLE
            }),
            Some(RestartBlocker::OpenPositions { count: 3 })
        );
        assert_eq!(
            blocker(RestartInputs {
                stored: StoredOpenPositions::Count(2),
                ..IDLE
            }),
            Some(RestartBlocker::OpenPositions { count: 2 })
        );
    }

    #[test]
    fn an_unopened_store_blocks_only_after_setup_completed() {
        assert_eq!(
            blocker(RestartInputs {
                stored: StoredOpenPositions::NotLoaded,
                ..IDLE
            }),
            Some(RestartBlocker::PositionsLoading)
        );
        assert_eq!(
            blocker(RestartInputs {
                stored: StoredOpenPositions::NotLoaded,
                initialization_complete: false,
                ..IDLE
            }),
            None
        );
    }

    #[test]
    fn an_unreadable_store_blocks_a_restart() {
        assert_eq!(
            blocker(RestartInputs {
                stored: StoredOpenPositions::Unreadable,
                ..IDLE
            }),
            Some(RestartBlocker::PositionsUnreadable)
        );
    }
}
