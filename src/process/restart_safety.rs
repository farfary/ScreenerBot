// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Whether restarting the process now would interrupt a trade, a tool or an open position.

use serde::Serialize;

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
enum StoredOpenPositions {
    Count(usize),
    NotLoaded,
    Unreadable,
}

/// The inputs of the restart-safety decision, explicit so the decision is testable.
#[derive(Debug, Clone, Copy)]
struct RestartInputs {
    trades_active: bool,
    tools_active: bool,
    open_in_memory: usize,
    stored: StoredOpenPositions,
    initialization_complete: bool,
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
fn blocker(inputs: RestartInputs) -> Option<RestartBlocker> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
