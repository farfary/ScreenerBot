// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Monitoring loops for automated trading
//!
//! This module contains orchestration-only code:
//! - Entry monitor: Loops through available tokens, calls evaluators, executes trades
//! - Exit monitor: Loops through open positions, calls evaluators, executes trades
//!
//! All business logic (safety checks, strategy evaluation, exit conditions) is in evaluators module.

mod access_pause;
mod entry;
mod exit;
mod paper_batch;

pub use entry::monitor_entries;
pub use exit::monitor_positions;

use crate::events::{record_trader_event, Severity};
use crate::i18n::{ids, UiText};
use crate::logger::{self, LogTag};
use serde_json::json;

/// Wait out a monitor's pause, ending it early when shutdown is signalled. The
/// monitor's loop head reads the signal, so a stop never waits for a pause to run out.
async fn pause_or_shutdown(
    shutdown: &mut tokio::sync::watch::Receiver<bool>,
    pause: tokio::time::Duration,
) {
    tokio::select! {
        _ = tokio::time::sleep(pause) => {}
        _ = shutdown.changed() => {}
    }
}

/// Start the auto trading monitors
pub async fn start_automated_trading(
    shutdown: tokio::sync::watch::Receiver<bool>,
) -> crate::trader::Result<()> {
    // The monitors act on the wallet's loaded history: the positions synced from it
    // and the signatures already recorded. That history loads after service startup,
    // so the monitors start once it has.
    if !crate::global::TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        logger::info(
            LogTag::Trader,
            "Automated trading monitors wait for the wallet history to load",
        );
    }
    let mut history_shutdown = shutdown.clone();
    tokio::select! {
        _ = crate::global::wait_for_transactions_system_ready() => {}
        _ = history_shutdown.wait_for(|stop| *stop) => return Ok(()),
    }

    logger::info(LogTag::Trader, "Starting automated trading monitors...");

    // Record auto trading start event
    record_trader_event(
        "auto_trading_started",
        Severity::Info,
        None,
        None,
        crate::events::with_text(
            json!({
                "system": "auto_trading",
            }),
            &UiText::new(ids::EVENTS_MONITORS_STARTING),
        ),
    )
    .await;

    // Clone shutdown receiver for multiple tasks
    let entry_shutdown = shutdown.clone();
    let exit_shutdown = shutdown.clone();

    // Spawn entry monitor
    let entry_task = tokio::spawn(async move {
        if let Err(e) = monitor_entries(entry_shutdown).await {
            logger::error(LogTag::Trader, &format!("Entry monitor error: {e}"));

            // Record entry monitor error
            record_trader_event(
                "entry_monitor_error",
                Severity::Error,
                None,
                None,
                json!({
                    "monitor": "entry",
                    "error": e.to_string(),
                }),
            )
            .await;
        }
    });

    // Spawn exit monitor
    let exit_task = tokio::spawn(async move {
        if let Err(e) = monitor_positions(exit_shutdown).await {
            logger::error(LogTag::Trader, &format!("Exit monitor error: {e}"));

            // Record exit monitor error
            record_trader_event(
                "exit_monitor_error",
                Severity::Error,
                None,
                None,
                json!({
                    "monitor": "exit",
                    "error": e.to_string(),
                }),
            )
            .await;
        }
    });

    // Wait for both tasks
    let _ = tokio::try_join!(entry_task, exit_task);

    logger::info(LogTag::Trader, "Automated trading monitors stopped");

    // Record auto trading stop event
    record_trader_event(
        "auto_trading_stopped",
        Severity::Info,
        None,
        None,
        crate::events::with_text(
            json!({
                "system": "auto_trading",
            }),
            &UiText::new(ids::EVENTS_MONITORS_STOPPED),
        ),
    )
    .await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::pause_or_shutdown;
    use std::time::Duration;

    #[tokio::test]
    async fn a_pause_ends_when_shutdown_is_signalled() {
        let (stop, mut shutdown) = tokio::sync::watch::channel(false);
        let pause = tokio::spawn(async move {
            pause_or_shutdown(&mut shutdown, Duration::from_secs(3600)).await;
        });
        tokio::task::yield_now().await;
        stop.send(true).expect("monitor receiver alive");
        tokio::time::timeout(Duration::from_secs(5), pause)
            .await
            .expect("a shutdown must end the pause")
            .expect("pause task");
    }
}
