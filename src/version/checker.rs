//! Update checking — polls the screenerbot.io API for new versions.

use super::types::*;
use super::{UPDATE_AVAILABLE, UPDATE_STATE, VERSION};
use crate::logger::{self, LogTag};
use chrono::Utc;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// Start the background update checking service.
///
/// Periodically checks for updates every 6 hours.
/// Runs in the background and logs when updates are found.
pub fn start_update_check_service(
    shutdown: std::sync::Arc<tokio::sync::Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(monitor.instrument(async move {
        logger::info(
            LogTag::System,
            &format!(
                "Update check service started (interval: {} hours)",
                super::UPDATE_CHECK_INTERVAL_SECS / 3600
            ),
        );

        // Initial check after 30 seconds (allow bot to fully start)
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(30)) => {}
            _ = shutdown.notified() => {
                logger::debug(LogTag::System, "Update check service shutdown during initial delay");
                return;
            }
        }

        // Perform initial check
        if let Err(e) = check_for_update().await {
            logger::warning(LogTag::System, &format!("Initial update check failed: {e}"));
        }

        // Periodic check loop
        let mut interval =
            tokio::time::interval(Duration::from_secs(super::UPDATE_CHECK_INTERVAL_SECS));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await; // Skip first immediate tick

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Skip the update check while the network is confirmed offline.
                    if crate::connectivity::is_network_offline() {
                        continue;
                    }
                    logger::debug(LogTag::System, "Running periodic update check...");
                    match check_for_update().await {
                        Ok(Some(update)) => {
                            logger::info(
                                LogTag::System,
                                &format!(
                                    "Update available: v{} (current: v{})",
                                    update.version, VERSION
                                ),
                            );
                        }
                        Ok(None) => {
                            logger::debug(LogTag::System, "No updates available");
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::System,
                                &format!("Periodic update check failed: {e}"),
                            );
                        }
                    }
                }
                _ = shutdown.notified() => {
                    logger::debug(LogTag::System, "Update check service shutting down");
                    break;
                }
            }
        }
    }))
}

/// Check for updates from the server.
pub async fn check_for_update() -> Result<Option<UpdateInfo>, String> {
    let platform = super::get_platform();
    let server_url = super::get_update_server_url();
    let url = format!(
        "{}/releases/check?version={}&platform={}",
        server_url, VERSION, platform
    );

    logger::debug(
        LogTag::System,
        &format!("Checking for updates at: {server_url}"),
    );
    logger::debug(LogTag::System, &format!("Update check URL: {url}"));

    let client = crate::net::client();
    let response = client
        .get(&url)
        .header("User-Agent", format!("ScreenerBot/{VERSION}"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to check for updates: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Update check failed: HTTP {}", response.status()));
    }

    let api_response: ApiResponse<UpdateCheckData> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse update response: {e}"))?;

    if !api_response.success {
        return Err(api_response
            .error
            .unwrap_or_else(|| "Unknown error".to_owned()));
    }

    let check_data = api_response.data.ok_or("No data in response")?;

    // Update global state
    let mut state = UPDATE_STATE.write().await;
    let update_state = state.get_or_insert_with(UpdateState::default);
    update_state.last_check = Some(Utc::now());

    if check_data.update_available {
        if let Some(ref update_data) = check_data.update {
            let update_info = UpdateInfo {
                version: update_data.version.clone(),
                download_url: update_data.download_url.clone(),
                file_size: update_data.file_size,
                checksum: update_data.checksum.clone(),
                release_notes: update_data.release_notes.clone(),
                release_date: update_data.published_at.clone().unwrap_or_default(),
            };

            UPDATE_AVAILABLE.store(true, Ordering::SeqCst);
            update_state.available_update = Some(update_info.clone());

            logger::info(
                LogTag::System,
                &format!("Update available: v{} → v{}", VERSION, update_info.version),
            );

            return Ok(Some(update_info));
        }
    }

    UPDATE_AVAILABLE.store(false, Ordering::SeqCst);
    update_state.available_update = None;

    logger::debug(LogTag::System, "No updates available");

    Ok(None)
}
