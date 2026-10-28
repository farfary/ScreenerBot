// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The Auto Trader status: whether it can run, whether it does, its run state, and why
//! each book takes no new entry while it runs.

use super::admission::EntryBlock;
use super::controller::{self, LiveBlock, TraderRunState};
use super::paper::TradingMode;
use crate::database::catalog;
use crate::logger::{self, LogTag};
use crate::strategies::db::EnabledStrategyCounts;

/// Whether the auto trader can run at all, and whether it does, with the enabled
/// strategies of each trading mode and why each book takes no new entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraderStatus {
    pub enabled: bool,
    pub running: bool,
    pub available: bool,
    /// Why the trader cannot run, while it is unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<crate::i18n::UiText>,
    /// Enabled strategies that trade live.
    pub live_strategies: usize,
    /// Enabled strategies that trade on the paper book.
    pub paper_strategies: usize,
    pub run_state: TraderRunState,
    /// Why the live book takes no new entry; `None` while entries are open or the trader
    /// does not run.
    pub live_entry_idle: Option<EntryIdle>,
    /// Why the paper book takes no new entry; `None` while entries are open or the trader
    /// does not run.
    pub paper_entry_idle: Option<EntryIdle>,
}

/// Why a book takes no new entry while the trader runs.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryIdle {
    /// The entry monitor is switched off.
    EntryMonitorOff,
    /// No enabled entry strategy trades on the book.
    NoEntryStrategy,
    /// A token-independent admission check refuses every entry on the book.
    Blocked { block: EntryBlock },
}

/// What the entry question of one book needs, decided before any store read.
#[derive(Debug, PartialEq, Eq)]
enum EntryGate {
    /// The trader does not run, so the question does not apply.
    NotApplicable,
    /// The book takes no entry for this reason.
    Idle(EntryIdle),
    /// An entry strategy could enter: the admission checks decide.
    CheckBlocks,
}

/// The entry gate of one book under `run_state`, with the entry monitor switch and the
/// count of enabled entry strategies of the book (`None` when the count could not be read).
fn entry_gate(
    run_state: TraderRunState,
    entry_monitor_enabled: bool,
    entry_strategies: Option<usize>,
) -> EntryGate {
    if !matches!(
        run_state,
        TraderRunState::Running | TraderRunState::EntryPaused
    ) {
        return EntryGate::NotApplicable;
    }
    if !entry_monitor_enabled {
        return EntryGate::Idle(EntryIdle::EntryMonitorOff);
    }
    match entry_strategies {
        None => EntryGate::Idle(EntryIdle::Blocked {
            block: EntryBlock::StorageUnavailable(format!(
                "{}: unreadable",
                catalog::STRATEGIES.id
            )),
        }),
        Some(0) => EntryGate::Idle(EntryIdle::NoEntryStrategy),
        Some(_) => EntryGate::CheckBlocks,
    }
}

/// Why `mode` takes no new entry under `run_state`; `None` while entries are open or the
/// trader does not run. The admission checks run only while an entry strategy of the mode
/// could enter, so a stopped trader or a mode nobody trades on costs no store read.
pub async fn entry_idle(
    mode: TradingMode,
    run_state: TraderRunState,
    entry_monitor_enabled: bool,
    entry_strategies: Option<usize>,
) -> Option<EntryIdle> {
    match entry_gate(run_state, entry_monitor_enabled, entry_strategies) {
        EntryGate::NotApplicable => None,
        EntryGate::Idle(idle) => Some(idle),
        EntryGate::CheckBlocks => controller::entry_blocks(mode)
            .await
            .map(|block| EntryIdle::Blocked { block }),
    }
}

/// The enabled-strategy counts, read on the blocking pool rather than on the async worker
/// that polls the status; `None` when they could not be read.
async fn enabled_strategy_counts() -> Option<EnabledStrategyCounts> {
    match tokio::task::spawn_blocking(crate::strategies::db::enabled_strategy_counts).await {
        Ok(Ok(counts)) => Some(counts),
        Ok(Err(error)) => {
            logger::warning(
                LogTag::Trader,
                &format!("Enabled strategy counts unavailable: {error}"),
            );
            None
        }
        Err(error) => {
            logger::warning(
                LogTag::Trader,
                &format!("Enabled strategy count task failed: {error}"),
            );
            None
        }
    }
}

/// The trader's status. The entry question of each book is answered only while the
/// trader is available.
pub async fn trader_status() -> TraderStatus {
    let available = controller::is_available();
    let run_state = controller::run_state();
    let counts = if available {
        enabled_strategy_counts().await
    } else {
        None
    };
    let (live_entry_idle, paper_entry_idle) = if available {
        let entry_monitor = super::config::is_entry_monitor_enabled_standalone();
        tokio::join!(
            entry_idle(
                TradingMode::Live,
                run_state,
                entry_monitor,
                counts.map(|counts| counts.live_entry),
            ),
            entry_idle(
                TradingMode::Paper,
                run_state,
                entry_monitor,
                counts.map(|counts| counts.paper_entry),
            ),
        )
    } else {
        (None, None)
    };
    let counts = counts.unwrap_or_default();
    TraderStatus {
        enabled: available && crate::config::with_config(|cfg| cfg.trader.enabled),
        running: available && controller::is_trader_running(),
        available,
        unavailable_reason: (!available)
            .then(|| crate::strategies::readiness::live_block_text(LiveBlock::SetupIncomplete)),
        live_strategies: counts.live,
        paper_strategies: counts.paper,
        run_state,
        live_entry_idle,
        paper_entry_idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_RUN_STATES: [TraderRunState; 7] = [
        TraderRunState::Explore,
        TraderRunState::ForceStopped,
        TraderRunState::Stopped,
        TraderRunState::Waiting,
        TraderRunState::Idle,
        TraderRunState::EntryPaused,
        TraderRunState::Running,
    ];

    #[test]
    fn entry_gate_applies_only_while_the_trader_runs() {
        for run_state in ALL_RUN_STATES {
            let gate = entry_gate(run_state, true, Some(1));
            let runs = matches!(
                run_state,
                TraderRunState::Running | TraderRunState::EntryPaused
            );
            if runs {
                assert_eq!(gate, EntryGate::CheckBlocks, "{run_state:?}");
            } else {
                assert_eq!(gate, EntryGate::NotApplicable, "{run_state:?}");
                assert_eq!(
                    entry_gate(run_state, false, None),
                    EntryGate::NotApplicable,
                    "{run_state:?}"
                );
            }
        }
    }

    #[test]
    fn entry_gate_names_a_disabled_entry_monitor_first() {
        for entry_strategies in [None, Some(0), Some(3)] {
            assert_eq!(
                entry_gate(TraderRunState::Running, false, entry_strategies),
                EntryGate::Idle(EntryIdle::EntryMonitorOff),
                "{entry_strategies:?}"
            );
        }
    }

    #[test]
    fn entry_gate_reports_unreadable_counts_as_storage_unavailable() {
        let gate = entry_gate(TraderRunState::EntryPaused, true, None);
        assert!(
            matches!(
                gate,
                EntryGate::Idle(EntryIdle::Blocked {
                    block: EntryBlock::StorageUnavailable(_)
                })
            ),
            "{gate:?}"
        );
    }

    #[test]
    fn entry_gate_checks_blocks_when_a_strategy_can_enter() {
        assert_eq!(
            entry_gate(TraderRunState::Running, true, Some(0)),
            EntryGate::Idle(EntryIdle::NoEntryStrategy)
        );
        assert_eq!(
            entry_gate(TraderRunState::Running, true, Some(2)),
            EntryGate::CheckBlocks
        );
    }

    #[test]
    fn entry_idle_serializes_as_a_tagged_kind() {
        let json = |idle: EntryIdle| serde_json::to_value(idle).unwrap();
        assert_eq!(
            json(EntryIdle::EntryMonitorOff),
            serde_json::json!({ "kind": "entry_monitor_off" })
        );
        assert_eq!(
            json(EntryIdle::NoEntryStrategy),
            serde_json::json!({ "kind": "no_entry_strategy" })
        );
        assert_eq!(
            json(EntryIdle::Blocked {
                block: EntryBlock::PositionLimit
            }),
            serde_json::json!({ "kind": "blocked", "block": { "kind": "position_limit" } })
        );
    }

    #[test]
    fn trader_status_serializes_run_state_and_both_idles() {
        let status = TraderStatus {
            enabled: true,
            running: true,
            available: true,
            unavailable_reason: None,
            live_strategies: 2,
            paper_strategies: 1,
            run_state: TraderRunState::Running,
            live_entry_idle: Some(EntryIdle::Blocked {
                block: EntryBlock::PositionLimit,
            }),
            paper_entry_idle: None,
        };
        assert_eq!(
            serde_json::to_value(status).unwrap(),
            serde_json::json!({
                "enabled": true,
                "running": true,
                "available": true,
                "live_strategies": 2,
                "paper_strategies": 1,
                "run_state": "running",
                "live_entry_idle": { "kind": "blocked", "block": { "kind": "position_limit" } },
                "paper_entry_idle": null,
            })
        );
    }
}
