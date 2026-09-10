//! Trader controller for starting/stopping trading

use crate::config::update_config_section;
use crate::logger::{self, LogTag};
use crate::trader::error::Error;
use std::time::Duration;

/// Check if the trader is currently running
pub fn is_trader_running() -> bool {
    super::config::is_trader_enabled()
}

/// Start the trader by enabling trader operations
pub async fn start_trader() -> Result<(), Error> {
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

/// Whether the auto trader can run at all, and whether it does.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TraderStatus {
    pub enabled: bool,
    pub running: bool,
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<&'static str>,
}

fn is_available() -> bool {
    crate::global::is_initialization_complete() && !crate::global::is_explore_mode()
}

pub fn trader_status() -> TraderStatus {
    let available = is_available();
    TraderStatus {
        enabled: available && crate::config::with_config(|cfg| cfg.trader.enabled),
        running: available && is_trader_running(),
        available,
        unavailable_reason: (!available)
            .then_some("Complete wallet and RPC setup to use Auto Trader"),
    }
}

/// Start the trader after the setup and emergency-stop gates every caller needs.
pub async fn start_trader_checked() -> Result<TraderStatus, Error> {
    if !is_available() {
        return Err(Error::TraderUnavailable);
    }
    if crate::global::is_force_stopped() {
        return Err(Error::ForceStopActive);
    }
    start_trader().await?;
    Ok(trader_status())
}

pub async fn stop_trader_checked() -> Result<TraderStatus, Error> {
    stop_trader_gracefully().await?;
    Ok(trader_status())
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
}

pub fn monitors_status() -> MonitorsStatus {
    use super::config;
    let available = is_available();
    MonitorsStatus {
        entry_monitor: MonitorState {
            enabled: config::is_entry_monitor_enabled_standalone(),
            running: available && config::is_entry_monitor_enabled(),
        },
        exit_monitor: MonitorState {
            enabled: config::is_exit_monitor_enabled_standalone(),
            running: available && config::is_exit_monitor_enabled(),
        },
        master_enabled: available && config::is_trader_enabled(),
        force_stopped: crate::global::is_force_stopped(),
        available,
    }
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
    pub limit_sol: f64,
    pub current_loss_sol: f64,
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
    let limit = config::get_loss_limit_sol();
    LossLimitSnapshot {
        enabled: config::is_loss_limit_enabled(),
        limit_sol: limit,
        current_loss_sol: status.cumulative_loss_sol,
        is_limited: status.is_limited,
        limited_at: status.limited_at,
        period_start: status.period_start,
        period_remaining_secs: status.period_remaining_secs,
        progress_percent: if limit > 0.0 {
            (status.cumulative_loss_sol / limit * 100.0).min(100.0)
        } else {
            0.0
        },
    }
}

/// Engage the emergency stop and switch the trader off so the stop survives a
/// restart. Recorded in the event log whoever triggers it.
pub async fn engage_force_stop(reason: &str) -> Result<crate::global::ForceStopStatus, Error> {
    crate::global::set_force_stopped(true, Some(reason));
    update_config_section(
        |cfg| {
            cfg.trader.enabled = false;
        },
        true,
    )
    .map_err(|e| Error::ConfigUpdate {
        detail: format!("force stop activated but disabling the trader failed: {e}"),
    })?;
    logger::warning(LogTag::Trader, &format!("FORCE STOP activated: {reason}"));
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
