// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The system-wide Copy Trading state: config, safety gates and the task set, and
//! the gate that decides whether a task can be armed for live.

use serde::Serialize;

use super::super::{CopyMode, CopyTask};
use super::open_database;
use crate::trader::{Error, Result};

/// System-wide copy-trading state derived from config, safety gates and the task set.
#[derive(Clone, Serialize)]
pub struct CopyTradingStatus {
    pub enabled: bool,
    pub live_available: bool,
    pub blocked_reason: Option<&'static str>,
    pub default_mode: String,
    pub default_slippage_pct: f64,
    pub force_stop_blocks: bool,
    pub total_tasks: usize,
    pub active_tasks: usize,
    pub paper_tasks: usize,
    pub live_tasks: usize,
}

impl CopyTradingStatus {
    /// Whether any task is copying: Copy Trading is on, no force stop halts it, and at
    /// least one task is enabled. A loss-limit block still copies exits, so it counts.
    pub fn is_copying(&self) -> bool {
        self.enabled && self.blocked_reason != Some("force_stop") && self.active_tasks > 0
    }
}

/// The distinct wallets the task set follows, whatever each task's state.
pub fn followed_wallets(tasks: &[CopyTask]) -> usize {
    tasks
        .iter()
        .map(|task| task.target_address.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len()
}

pub fn build_status(tasks: &[CopyTask]) -> CopyTradingStatus {
    let (enabled, default_mode, default_slippage_pct, force_stop_blocks) =
        crate::config::with_config(|config| {
            (
                config.copy_trading.enabled,
                config.copy_trading.default_mode.clone(),
                config.copy_trading.default_slippage_pct,
                config.copy_trading.block_on_force_stop,
            )
        });
    CopyTradingStatus {
        enabled,
        live_available: live_block_reason().is_none(),
        blocked_reason: if crate::global::is_force_stopped() {
            Some("force_stop")
        } else if crate::trader::safety::loss_limit::is_entry_blocked_by_loss_limit() {
            Some("loss_limit")
        } else {
            None
        },
        default_mode,
        default_slippage_pct,
        force_stop_blocks,
        total_tasks: tasks.len(),
        active_tasks: tasks.iter().filter(|task| task.enabled).count(),
        paper_tasks: tasks
            .iter()
            .filter(|task| task.enabled && task.mode == CopyMode::Paper)
            .count(),
        live_tasks: tasks
            .iter()
            .filter(|task| task.enabled && task.mode == CopyMode::Live)
            .count(),
    }
}

/// Why a task cannot be armed for live right now, if it cannot: the one gate
/// behind both the status flag and the paper-to-live transition.
pub fn live_block_reason() -> Option<&'static str> {
    if !crate::global::is_initialization_complete() || crate::global::is_explore_mode() {
        Some("setup_incomplete")
    } else if crate::global::is_force_stopped() {
        Some("force_stop")
    } else if !crate::config::with_config(|config| config.copy_trading.enabled) {
        Some("copy_trading_disabled")
    } else {
        None
    }
}

/// Switch Copy Trading off and persist it, through the same validated config update
/// the config route uses. Already off is not an error.
pub fn stop_copy_trading() -> Result<()> {
    crate::config::update_config_section(|config| config.copy_trading.enabled = false, true)
        .map_err(|e| Error::ConfigUpdate {
            detail: e.to_string(),
        })
}

pub async fn status() -> Result<CopyTradingStatus> {
    let tasks = open_database().await?.list_tasks().await?;
    Ok(build_status(&tasks))
}

pub(in crate::trader::copy) fn effective_state(
    status: &CopyTradingStatus,
    task: &CopyTask,
) -> &'static str {
    if !status.enabled {
        "system_paused"
    } else if status.blocked_reason == Some("force_stop") {
        "force_stopped"
    } else if !task.enabled {
        "paused"
    } else if status.blocked_reason.is_some() {
        "entries_blocked"
    } else if task.mode == CopyMode::Live {
        "live"
    } else {
        "paper"
    }
}
