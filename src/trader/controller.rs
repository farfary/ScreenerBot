// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Trader controller for starting/stopping trading.

use super::admission::{self, EntryBlock};
use super::paper::TradingMode;
use crate::config::update_config_section;
use crate::logger::{self, LogTag};
use crate::trader::error::Error;
use std::time::Duration;

/// Check if the trader is currently running
pub fn is_trader_running() -> bool {
    super::config::is_trader_enabled()
}

/// Start the trader by enabling trader operations; every caller goes through
/// [`start_trader_checked`].
async fn start_trader() -> Result<(), Error> {
    if super::config::is_trader_enabled() {
        return Err(Error::AlreadyRunning);
    }

    logger::info(LogTag::Trader, "Enabling trader operations...");

    // Update config to enable trader
    update_config_section(
        |cfg| {
            cfg.trader.enabled = true;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: e.to_string(),
    })?;

    logger::info(LogTag::Trader, "Trader operations enabled");
    Ok(())
}

/// Stop the trader gracefully by signaling shutdown and waiting for tasks to complete
pub async fn stop_trader_gracefully() -> Result<(), Error> {
    if !super::config::is_trader_enabled() {
        return Err(Error::AlreadyStopped);
    }

    logger::info(LogTag::Trader, "Disabling trader operations...");

    // Update config to disable trader
    update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: e.to_string(),
    })?;

    // Wait a moment for graceful shutdown
    tokio::time::sleep(Duration::from_secs(2)).await;

    logger::info(LogTag::Trader, "Trader operations disabled");
    Ok(())
}

/// Whether the auto trader can run at all, and whether it does, with the enabled
/// strategies of each trading mode.
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
}

/// Why the Auto Trader cannot trade live right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBlock {
    /// Wallet and RPC setup is unfinished, or the app runs in Explore Mode.
    SetupIncomplete,
    /// The emergency stop is engaged.
    ForceStop,
}

fn is_available() -> bool {
    crate::global::is_initialization_complete() && !crate::global::is_explore_mode()
}

/// The gate in front of live trading: the setup gate, then the emergency stop.
pub fn live_block() -> Option<LiveBlock> {
    if !is_available() {
        Some(LiveBlock::SetupIncomplete)
    } else if crate::global::is_force_stopped() {
        Some(LiveBlock::ForceStop)
    } else {
        None
    }
}

/// What the Auto Trader is doing right now, in the order the checks decide it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TraderRunState {
    /// Explore Mode: the app has no trader.
    Explore,
    /// The emergency stop is engaged.
    ForceStopped,
    /// The trader is switched off.
    Stopped,
    /// The trader is on and waits for the core services.
    Waiting,
    /// The trader is on with both monitors switched off.
    Idle,
    /// The entry monitor is on and the loss limit pauses live entries.
    EntryPaused,
    /// The trader runs at least one monitor.
    Running,
}

/// The in-memory flags the run state is decided from.
#[derive(Debug, Clone, Copy, Default)]
struct RunFlags {
    explore: bool,
    force_stopped: bool,
    trader_enabled: bool,
    core_services_ready: bool,
    entry_monitor_enabled: bool,
    exit_monitor_enabled: bool,
    live_entries_loss_limited: bool,
}

impl RunFlags {
    fn read() -> Self {
        let (trader_enabled, entry_monitor_enabled, exit_monitor_enabled) =
            crate::config::with_config(|cfg| {
                (
                    cfg.trader.enabled,
                    cfg.trader.entry_monitor_enabled,
                    cfg.trader.exit_monitor_enabled,
                )
            });
        Self {
            explore: crate::global::is_explore_mode(),
            force_stopped: crate::global::is_force_stopped(),
            trader_enabled,
            core_services_ready: crate::global::are_core_services_ready(),
            entry_monitor_enabled,
            exit_monitor_enabled,
            live_entries_loss_limited: super::safety::loss_limit::is_entry_blocked_by_loss_limit(),
        }
    }

    fn run_state(self) -> TraderRunState {
        if self.explore {
            TraderRunState::Explore
        } else if self.force_stopped {
            TraderRunState::ForceStopped
        } else if !self.trader_enabled {
            TraderRunState::Stopped
        } else if !self.core_services_ready {
            TraderRunState::Waiting
        } else if !self.entry_monitor_enabled && !self.exit_monitor_enabled {
            TraderRunState::Idle
        } else if self.entry_monitor_enabled && self.live_entries_loss_limited {
            TraderRunState::EntryPaused
        } else {
            TraderRunState::Running
        }
    }
}

/// The trader's run state, read from in-memory flags and config only.
pub fn run_state() -> TraderRunState {
    RunFlags::read().run_state()
}

/// The trader-wide halt in front of entries on `mode`: the emergency stop, then the loss
/// limit, which protects real funds and so pauses live entries only, never paper.
fn entry_halt_of(mode: TradingMode, force_stopped: bool, loss_limited: bool) -> Option<EntryBlock> {
    if force_stopped {
        Some(EntryBlock::ForceStopped)
    } else if mode == TradingMode::Live && loss_limited {
        Some(EntryBlock::LossLimit)
    } else {
        None
    }
}

/// The trader-wide halt in front of entries on `mode`, read from the emergency stop and
/// the loss limit state.
pub fn entry_halt(mode: TradingMode) -> Option<EntryBlock> {
    entry_halt_of(
        mode,
        crate::global::is_force_stopped(),
        mode == TradingMode::Live && super::safety::loss_limit::is_entry_blocked_by_loss_limit(),
    )
}

/// The token-independent part of entry admission on `mode`: what refuses every entry on
/// that mode right now, whatever the token. Live: emergency stop, loss limit, position
/// limit. Paper: emergency stop, paper position limit, paper cash short of one default
/// entry. The paper cash is read on the blocking pool and never starts a paper run.
pub async fn entry_blocks(mode: TradingMode) -> Option<EntryBlock> {
    if let Some(block) = entry_halt(mode) {
        return Some(block);
    }
    if let Err(block) = admission::check_slots(mode).await {
        return Some(block);
    }
    match mode {
        TradingMode::Live => None,
        TradingMode::Paper => {
            let cash = admission::standing_paper_cash().await;
            admission::paper_cash_covers(Some(&cash), super::executors::entry_size_of(None)).err()
        }
    }
}

/// The trader's status. The enabled-strategy counts are a SQLite read, so they run on
/// the blocking pool rather than on the async worker that polls the status.
pub async fn trader_status() -> TraderStatus {
    let available = is_available();
    let counts = if available {
        match tokio::task::spawn_blocking(crate::strategies::db::enabled_strategy_counts).await {
            Ok(Ok(counts)) => counts,
            Ok(Err(error)) => {
                logger::warning(
                    LogTag::Trader,
                    &format!("Enabled strategy counts unavailable: {error}"),
                );
                Default::default()
            }
            Err(error) => {
                logger::warning(
                    LogTag::Trader,
                    &format!("Enabled strategy count task failed: {error}"),
                );
                Default::default()
            }
        }
    } else {
        Default::default()
    };
    TraderStatus {
        enabled: available && crate::config::with_config(|cfg| cfg.trader.enabled),
        running: available && is_trader_running(),
        available,
        unavailable_reason: (!available)
            .then(|| crate::strategies::readiness::live_block_text(LiveBlock::SetupIncomplete)),
        live_strategies: counts.live,
        paper_strategies: counts.paper,
    }
}

/// Start the trader after the setup and emergency-stop gates every caller needs.
pub async fn start_trader_checked() -> Result<TraderStatus, Error> {
    match live_block() {
        Some(LiveBlock::SetupIncomplete) => return Err(Error::TraderUnavailable),
        Some(LiveBlock::ForceStop) => return Err(Error::ForceStopActive),
        None => {}
    }
    start_trader().await?;
    Ok(trader_status().await)
}

pub async fn stop_trader_checked() -> Result<TraderStatus, Error> {
    stop_trader_gracefully().await?;
    Ok(trader_status().await)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorState {
    pub enabled: bool,
    pub running: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorsStatus {
    pub entry_monitor: MonitorState,
    pub exit_monitor: MonitorState,
    pub master_enabled: bool,
    pub force_stopped: bool,
    pub available: bool,
    /// Whether a force stop has anything to halt right now: a running monitor, a
    /// copy task the stop blocks, or a trade in flight.
    pub anything_running: bool,
}

pub async fn monitors_status() -> MonitorsStatus {
    use super::config;
    let available = is_available();
    let entry_monitor = MonitorState {
        enabled: config::is_entry_monitor_enabled_standalone(),
        running: available && config::is_entry_monitor_enabled(),
    };
    let exit_monitor = MonitorState {
        enabled: config::is_exit_monitor_enabled_standalone(),
        running: available && config::is_exit_monitor_enabled(),
    };
    let copy = super::copy::control::status().await.ok();
    let anything_running = anything_to_halt(
        entry_monitor.running || exit_monitor.running,
        crate::global::are_trades_active(),
        copy.as_ref(),
    );
    MonitorsStatus {
        entry_monitor,
        exit_monitor,
        master_enabled: available && config::is_trader_enabled(),
        force_stopped: crate::global::is_force_stopped(),
        available,
        anything_running,
    }
}

/// Whether a force stop would halt anything. Copy tasks count only while copy
/// trading is on and the stop blocks it; a copy store that could not be read
/// counts as running, so the stop is never withheld on a missing answer.
fn anything_to_halt(
    monitor_running: bool,
    trade_in_flight: bool,
    copy: Option<&super::copy::control::CopyTradingStatus>,
) -> bool {
    let copy_running = copy
        .is_none_or(|status| status.enabled && status.force_stop_blocks && status.active_tasks > 0);
    monitor_running || trade_in_flight || copy_running
}

/// Which trader monitor a toggle addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Monitor {
    Entry,
    Exit,
}

pub fn set_monitor_enabled(monitor: Monitor, enabled: bool) -> Result<(), Error> {
    update_config_section(
        |cfg| match monitor {
            Monitor::Entry => cfg.trader.entry_monitor_enabled = enabled,
            Monitor::Exit => cfg.trader.exit_monitor_enabled = enabled,
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: e.to_string(),
    })?;
    logger::info(
        LogTag::Trader,
        &format!(
            "{} monitor {}",
            if monitor == Monitor::Entry {
                "Entry"
            } else {
                "Exit"
            },
            if enabled { "enabled" } else { "disabled" }
        ),
    );
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct LossLimitSnapshot {
    pub enabled: bool,
    pub limit_native: f64,
    pub current_loss_native: f64,
    pub is_limited: bool,
    pub limited_at: Option<chrono::DateTime<chrono::Utc>>,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_remaining_secs: i64,
    pub progress_percent: f64,
}

pub fn loss_limit_snapshot() -> LossLimitSnapshot {
    use super::config;
    use super::safety::loss_limit;
    let status = loss_limit::get_loss_limit_status();
    let limit = config::get_loss_limit_native();
    LossLimitSnapshot {
        enabled: config::is_loss_limit_enabled(),
        limit_native: limit,
        current_loss_native: status.cumulative_loss_native,
        is_limited: status.is_limited,
        limited_at: status.limited_at,
        period_start: status.period_start,
        period_remaining_secs: status.period_remaining_secs,
        progress_percent: if limit > 0.0 {
            (status.cumulative_loss_native / limit * 100.0).min(100.0)
        } else {
            0.0
        },
    }
}

/// Engage the emergency stop and switch the trader off so the stop survives a
/// restart. Recorded in the event log whoever triggers it.
pub async fn engage_force_stop(
    reason: Option<&str>,
) -> Result<crate::global::ForceStopStatus, Error> {
    crate::global::set_force_stopped(true, reason);
    update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: format!("force stop activated but disabling the trader failed: {e}"),
    })?;
    logger::warning(
        LogTag::Trader,
        &format!(
            "FORCE STOP activated: {}",
            reason.unwrap_or("no reason given")
        ),
    );
    record_force_stop_event("ForceStop", serde_json::json!({ "reason": reason })).await;
    Ok(crate::global::get_force_stop_status())
}

/// Clear the emergency stop. Like the dashboard resume, the trader stays off
/// until it is started explicitly. Returns whether a stop was active.
pub async fn clear_force_stop(reason: Option<&str>) -> bool {
    let was_stopped = crate::global::is_force_stopped();
    crate::global::set_force_stopped(false, None);
    logger::info(
        LogTag::Trader,
        "Force stop cleared - trading can be resumed",
    );
    record_force_stop_event(
        "ForceStopCleared",
        serde_json::json!({ "reason": reason, "was_stopped": was_stopped }),
    )
    .await;
    was_stopped
}

async fn record_force_stop_event(subtype: &str, payload: serde_json::Value) {
    let _ = crate::events::record(crate::events::Event {
        id: None,
        event_time: chrono::Utc::now(),
        category: crate::events::EventCategory::System,
        subtype: Some(subtype.to_owned()),
        severity: crate::events::Severity::Warn,
        mint: None,
        reference_id: None,
        payload,
        created_at: None,
    })
    .await;
}

#[cfg(test)]
mod tests {
    use super::{anything_to_halt, entry_halt_of, RunFlags, TraderRunState};
    use crate::trader::admission::EntryBlock;
    use crate::trader::copy::control::CopyTradingStatus;
    use crate::trader::paper::TradingMode;

    /// A trader that is on, with core services ready and both monitors running.
    fn running() -> RunFlags {
        RunFlags {
            trader_enabled: true,
            core_services_ready: true,
            entry_monitor_enabled: true,
            exit_monitor_enabled: true,
            ..RunFlags::default()
        }
    }

    #[test]
    fn each_run_state_is_decided_in_order() {
        let cases = [
            (
                RunFlags {
                    explore: true,
                    force_stopped: true,
                    ..running()
                },
                TraderRunState::Explore,
            ),
            (
                RunFlags {
                    force_stopped: true,
                    live_entries_loss_limited: true,
                    ..running()
                },
                TraderRunState::ForceStopped,
            ),
            (
                RunFlags {
                    trader_enabled: false,
                    core_services_ready: false,
                    ..running()
                },
                TraderRunState::Stopped,
            ),
            (
                RunFlags {
                    core_services_ready: false,
                    entry_monitor_enabled: false,
                    exit_monitor_enabled: false,
                    ..running()
                },
                TraderRunState::Waiting,
            ),
            (
                RunFlags {
                    entry_monitor_enabled: false,
                    exit_monitor_enabled: false,
                    live_entries_loss_limited: true,
                    ..running()
                },
                TraderRunState::Idle,
            ),
            (
                RunFlags {
                    live_entries_loss_limited: true,
                    ..running()
                },
                TraderRunState::EntryPaused,
            ),
            (
                RunFlags {
                    entry_monitor_enabled: false,
                    live_entries_loss_limited: true,
                    ..running()
                },
                TraderRunState::Running,
            ),
            (running(), TraderRunState::Running),
            (
                RunFlags {
                    entry_monitor_enabled: false,
                    ..running()
                },
                TraderRunState::Running,
            ),
            (
                RunFlags {
                    exit_monitor_enabled: false,
                    ..running()
                },
                TraderRunState::Running,
            ),
        ];
        for (flags, expected) in cases {
            assert_eq!(flags.run_state(), expected, "{flags:?}");
        }
    }

    /// The header and every other reader receive these exact wire values.
    #[test]
    fn run_states_serialize_as_their_snake_case_wire_values() {
        for (state, wire) in [
            (TraderRunState::Explore, "explore"),
            (TraderRunState::ForceStopped, "force_stopped"),
            (TraderRunState::Stopped, "stopped"),
            (TraderRunState::Waiting, "waiting"),
            (TraderRunState::Idle, "idle"),
            (TraderRunState::EntryPaused, "entry_paused"),
            (TraderRunState::Running, "running"),
        ] {
            assert_eq!(serde_json::to_value(state).unwrap(), wire);
        }
    }

    #[test]
    fn the_loss_limit_blocks_live_entries_but_never_paper() {
        assert_eq!(
            entry_halt_of(TradingMode::Live, false, true),
            Some(EntryBlock::LossLimit)
        );
        assert_eq!(entry_halt_of(TradingMode::Paper, false, true), None);
        assert_eq!(entry_halt_of(TradingMode::Live, false, false), None);
        assert_eq!(entry_halt_of(TradingMode::Paper, false, false), None);
    }

    #[test]
    fn the_emergency_stop_blocks_entries_on_every_mode_ahead_of_the_loss_limit() {
        for mode in [TradingMode::Live, TradingMode::Paper] {
            for loss_limited in [false, true] {
                assert_eq!(
                    entry_halt_of(mode, true, loss_limited),
                    Some(EntryBlock::ForceStopped),
                    "{mode:?}"
                );
            }
        }
    }

    fn copy(enabled: bool, force_stop_blocks: bool, active_tasks: usize) -> CopyTradingStatus {
        CopyTradingStatus {
            enabled,
            live_available: false,
            blocked_reason: None,
            default_mode: "paper".to_string(),
            default_slippage_pct: 1.0,
            force_stop_blocks,
            total_tasks: active_tasks,
            active_tasks,
            paper_tasks: active_tasks,
            live_tasks: 0,
            resumable_tasks: 0,
            resumable_live_tasks: 0,
        }
    }

    #[test]
    fn nothing_running_leaves_nothing_to_halt() {
        assert!(!anything_to_halt(false, false, Some(&copy(true, true, 0))));
        assert!(!anything_to_halt(false, false, Some(&copy(false, true, 3))));
        assert!(!anything_to_halt(false, false, Some(&copy(true, false, 3))));
    }

    #[test]
    fn each_running_kind_gives_the_stop_something_to_halt() {
        let idle = copy(true, true, 0);
        assert!(anything_to_halt(true, false, Some(&idle)));
        assert!(anything_to_halt(false, true, Some(&idle)));
        assert!(anything_to_halt(false, false, Some(&copy(true, true, 1))));
    }

    #[test]
    fn an_unreadable_copy_store_never_withholds_the_stop() {
        assert!(anything_to_halt(false, false, None));
    }
}
