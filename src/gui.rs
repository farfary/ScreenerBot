/// GUI module for ScreenerBot desktop application
///
/// Handles Tauri window management and integration with the headless ScreenerBot backend.
/// The GUI mode embeds the webserver dashboard (localhost:8080) in a native window.
use crate::logger::{self, LogTag};
use std::time::Duration;
use tauri::Manager;

/// Run bot in GUI mode with Tauri window
///
/// This function:
/// 1. Spawns the ScreenerBot backend in a background task
/// 2. Builds and runs the Tauri desktop application
/// 3. Waits for the webserver to be ready
/// 4. Shows the window with the dashboard loaded
///
/// The window is initially hidden and only shown after the dashboard HTML is ready,
/// ensuring a smooth user experience without showing loading states.
pub async fn run_gui_mode() -> Result<(), String> {
    logger::info(LogTag::System, "🖥️  Initializing Tauri desktop application");

    // Start the ScreenerBot backend in a background task
    tokio::spawn(async move {
        logger::info(LogTag::System, "Starting ScreenerBot backend services...");

        // Start the full ScreenerBot system (includes webserver on :8080)
        match crate::run::run_bot().await {
            Ok(_) => {
                logger::info(
                    LogTag::System,
                    "✅ ScreenerBot backend started successfully",
                );
            }
            Err(e) => {
                logger::error(
                    LogTag::System,
                    &format!("❌ Failed to start ScreenerBot backend: {}", e),
                );
            }
        }
    });

    // Build and run Tauri application
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();

            logger::info(
                LogTag::System,
                "🔧 Tauri setup started - window created but hidden",
            );

            // Spawn thread to wait for dashboard to be fully loaded, then show window
            std::thread::spawn(move || {
                wait_for_dashboard_and_show_window(app_handle);
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri application error: {}", e))?;

    Ok(())
}

/// Wait for dashboard to be ready and show the window
///
/// Polls the dashboard endpoint until HTML content is returned, then:
/// 1. Navigates the window to ensure fresh content
/// 2. Applies zoom correction for retina displays
/// 3. Shows the window
fn wait_for_dashboard_and_show_window(app_handle: tauri::AppHandle) {
    logger::info(
        LogTag::System,
        "⏳ Polling dashboard endpoint until HTML is ready...",
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap();

    let mut poll_count = 0;

    // Poll dashboard root endpoint until it returns HTML content
    // This ensures webserver is fully up and serving content
    loop {
        poll_count += 1;

        match client.get("http://localhost:8080/").send() {
            Ok(response) => {
                let status = response.status();
                logger::debug(
                    LogTag::System,
                    &format!("Poll #{}: HTTP {} from dashboard", poll_count, status),
                );

                if status.is_success() {
                    // Verify we got HTML content, not just any response
                    if let Ok(text) = response.text() {
                        if text.contains("<!doctype html") || text.contains("<html") {
                            logger::info(
                                LogTag::System,
                                &format!("✅ Dashboard HTML ready after {} polls", poll_count),
                            );
                            break;
                        } else {
                            logger::warning(
                                LogTag::System,
                                &format!(
                                    "⚠️  Got 200 response but no HTML content (got {} bytes)",
                                    text.len()
                                ),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                if poll_count == 1 || poll_count % 20 == 0 {
                    logger::debug(
                        LogTag::System,
                        &format!("Poll #{}: Dashboard not ready yet - {}", poll_count, e),
                    );
                }
            }
        }

        // Fast polling - check every 50ms
        std::thread::sleep(Duration::from_millis(50));
    }

    // Get the window and navigate it to ensure fresh content
    if let Some(window) = app_handle.get_webview_window("main") {
        navigate_and_show_window(window);
    } else {
        logger::error(LogTag::System, "❌ Failed to find main window");
    }
}

/// Navigate the window to the dashboard URL and show it
///
/// Applies zoom correction for retina displays before showing the window.
fn navigate_and_show_window(window: tauri::WebviewWindow) {
    logger::info(LogTag::System, "🔄 Navigating window to dashboard URL...");

    // Use Tauri's native navigation instead of JavaScript reload
    // This ensures the webview actually loads the URL with fresh content
    if let Err(e) = window.navigate("http://localhost:8080/".parse().unwrap()) {
        logger::error(
            LogTag::System,
            &format!("❌ Failed to navigate window: {}", e),
        );
        return;
    }

    logger::info(LogTag::System, "✅ Window navigation triggered");

    // Apply zoom correction for retina displays
    apply_zoom_correction(&window);

    // Small delay to let navigation start before showing
    std::thread::sleep(Duration::from_millis(200));

    // Now show the window
    match window.show() {
        Ok(_) => {
            logger::info(LogTag::System, "✅ GUI window shown with dashboard loaded");
        }
        Err(e) => {
            logger::error(LogTag::System, &format!("❌ Failed to show window: {}", e));
        }
    }
}

/// Apply zoom correction based on monitor scale factor
///
/// Retina displays have scale factors > 1.0, which can cause UI rendering issues.
/// This function applies a zoom correction to ensure proper rendering.
fn apply_zoom_correction(window: &tauri::WebviewWindow) {
    match window.scale_factor() {
        Ok(scale_factor) => {
            if (scale_factor - 1.0).abs() > f64::EPSILON {
                let zoom = 1.0 / scale_factor;
                match window.set_zoom(zoom) {
                    Ok(_) => {
                        logger::info(
                            LogTag::System,
                            &format!(
                                "Applied zoom correction for scale factor {:.2} (zoom={:.4})",
                                scale_factor, zoom
                            ),
                        );
                    }
                    Err(e) => {
                        logger::warning(
                            LogTag::System,
                            &format!(
                                "Failed to apply zoom correction (scale factor {:.2}): {}",
                                scale_factor, e
                            ),
                        );
                    }
                }
            } else {
                logger::debug(
                    LogTag::System,
                    "Scale factor is 1.0 - no zoom correction needed",
                );
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::System,
                &format!("Failed to read scale factor for zoom calibration: {}", e),
            );
        }
    }
}
