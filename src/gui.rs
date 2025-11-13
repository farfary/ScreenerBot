/// GUI module for ScreenerBot desktop application
///
/// Handles Tauri window management and integration with the headless ScreenerBot backend.
/// The GUI mode embeds the webserver dashboard (localhost:8080) in a native window.
use crate::config::with_config;
use crate::logger::{self, LogTag};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

/// Run bot in GUI mode with Tauri window
///
/// This function:
/// 1. Spawns the ScreenerBot backend in a background task
/// 2. Builds and runs the Tauri desktop application
/// 3. Registers global keyboard shortcuts for zoom (Ctrl/Cmd +/-/0)
/// 4. Waits for the webserver to be ready
/// 5. Shows the window with the dashboard loaded
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

    // Start with default zoom (config will be loaded later by backend)
    let initial_zoom = 1.0;
    logger::info(
        LogTag::System,
        "Starting with default zoom level: 100%",
    );

    // Shared zoom level state (used by keyboard shortcuts)
    let zoom_level = Arc::new(Mutex::new(initial_zoom));

    // Build and run Tauri application
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup({
            let zoom_level_clone = Arc::clone(&zoom_level);
            move |app| {
                let app_handle = app.handle().clone();

                // Register global keyboard shortcuts for zoom
                register_zoom_shortcuts(app, Arc::clone(&zoom_level_clone))?;

                logger::info(
                    LogTag::System,
                    "🔧 Tauri setup started - window created but hidden",
                );

                // Spawn thread to wait for dashboard to be fully loaded, then show window
                std::thread::spawn(move || {
                    wait_for_dashboard_and_show_window(app_handle, zoom_level_clone);
                });

                Ok(())
            }
        })
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri application error: {}", e))?;

    Ok(())
}

/// Register global keyboard shortcuts for zoom control
///
/// Registers:
/// - Cmd/Ctrl + Plus: Zoom in
/// - Cmd/Ctrl + Minus: Zoom out
/// - Cmd/Ctrl + 0: Reset zoom to 100%
fn register_zoom_shortcuts(
    app: &mut tauri::App,
    zoom_level: Arc<Mutex<f64>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // Determine modifier key based on platform
    let modifier = if cfg!(target_os = "macos") {
        Modifiers::META // Command key on macOS
    } else {
        Modifiers::CONTROL // Ctrl key on Windows/Linux
    };

    // Register Zoom In (Cmd/Ctrl + Plus or =)
    let zoom_in_shortcut = Shortcut::new(Some(modifier), Code::Equal); // Equal key (where + is)
    let zoom_level_in = Arc::clone(&zoom_level);
    let app_handle_in = app_handle.clone();
    let last_zoom_in = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(1000)));

    app.global_shortcut()
        .on_shortcut(zoom_in_shortcut, move |_app, _event, _shortcut| {
            // Debounce: ignore if called within 300ms (prevents double-trigger on key hold)
            let mut last = last_zoom_in.lock().unwrap();
            if last.elapsed() < Duration::from_millis(300) {
                return;
            }
            *last = Instant::now();
            drop(last);

            if let Some(window) = app_handle_in.get_webview_window("main") {
                let mut zoom = zoom_level_in.lock().unwrap();
                *zoom = (*zoom + 0.1).min(3.0); // Max 300%
                let zoom_val = *zoom;
                drop(zoom); // Release lock before window operations

                if let Err(e) = window.set_zoom(zoom_val) {
                    logger::warning(LogTag::System, &format!("Failed to set zoom: {}", e));
                } else {
                    logger::info(
                        LogTag::System,
                        &format!("🔍 Zoom in: {:.0}%", zoom_val * 100.0),
                    );
                    // Save to config
                    save_zoom_to_config(zoom_val);
                }
            }
        })?;

    // Register Zoom Out (Cmd/Ctrl + Minus)
    let zoom_out_shortcut = Shortcut::new(Some(modifier), Code::Minus);
    let zoom_level_out = Arc::clone(&zoom_level);
    let app_handle_out = app_handle.clone();
    let last_zoom_out = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(1000)));

    app.global_shortcut()
        .on_shortcut(zoom_out_shortcut, move |_app, _event, _shortcut| {
            // Debounce: ignore if called within 300ms (prevents double-trigger on key hold)
            let mut last = last_zoom_out.lock().unwrap();
            if last.elapsed() < Duration::from_millis(300) {
                return;
            }
            *last = Instant::now();
            drop(last);

            if let Some(window) = app_handle_out.get_webview_window("main") {
                let mut zoom = zoom_level_out.lock().unwrap();
                *zoom = (*zoom - 0.1).max(0.5); // Min 50%
                let zoom_val = *zoom;
                drop(zoom); // Release lock before window operations

                if let Err(e) = window.set_zoom(zoom_val) {
                    logger::warning(LogTag::System, &format!("Failed to set zoom: {}", e));
                } else {
                    logger::info(
                        LogTag::System,
                        &format!("🔍 Zoom out: {:.0}%", zoom_val * 100.0),
                    );
                    // Save to config
                    save_zoom_to_config(zoom_val);
                }
            }
        })?;

    // Register Reset Zoom (Cmd/Ctrl + 0)
    let zoom_reset_shortcut = Shortcut::new(Some(modifier), Code::Digit0);
    let zoom_level_reset = Arc::clone(&zoom_level);
    let last_zoom_reset = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(1000)));

    app.global_shortcut()
        .on_shortcut(zoom_reset_shortcut, move |_app, _event, _shortcut| {
            // Debounce: ignore if called within 300ms (prevents double-trigger on key hold)
            let mut last = last_zoom_reset.lock().unwrap();
            if last.elapsed() < Duration::from_millis(300) {
                return;
            }
            *last = Instant::now();
            drop(last);

            if let Some(window) = app_handle.get_webview_window("main") {
                let mut zoom = zoom_level_reset.lock().unwrap();
                *zoom = 1.0;
                let zoom_val = *zoom;
                drop(zoom); // Release lock before window operations

                if let Err(e) = window.set_zoom(zoom_val) {
                    logger::warning(LogTag::System, &format!("Failed to reset zoom: {}", e));
                } else {
                    logger::info(LogTag::System, "🔍 Zoom reset: 100%");
                    // Save to config
                    save_zoom_to_config(zoom_val);
                }
            }
        })?;

    logger::info(
        LogTag::System,
        "✅ Registered zoom shortcuts (Ctrl/Cmd +/-/0)",
    );

    Ok(())
}

/// Save zoom level to config file
fn save_zoom_to_config(zoom: f64) {
    // Update in-memory config and save to disk
    std::thread::spawn(move || {
        if let Err(e) = crate::config::update_config_section(
            |config| {
                config.gui.zoom_level = zoom;
            },
            true, // save to disk
        ) {
            logger::warning(
                LogTag::System,
                &format!("Failed to save zoom to config: {}", e),
            );
        }
    });
}

/// Wait for dashboard to be ready and show the window
///
/// Polls the dashboard endpoint until HTML content is returned, then:
/// 1. Navigates the window to ensure fresh content
/// 2. Applies saved zoom level
/// 3. Shows the window
fn wait_for_dashboard_and_show_window(app_handle: tauri::AppHandle, zoom_level: Arc<Mutex<f64>>) {
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
    logger::info(LogTag::System, "🔍 Looking for main window to show...");

    match app_handle.get_webview_window("main") {
        Some(window) => {
            logger::info(LogTag::System, "✅ Found main window, showing it now");
            navigate_and_show_window(window, zoom_level);
        }
        None => {
            logger::error(
                LogTag::System,
                "❌ Failed to find main window with label 'main'",
            );

            // Try to get all windows and show the first one
            let webview_windows = app_handle.webview_windows();
            logger::info(
                LogTag::System,
                &format!(
                    "Available windows: {:?}",
                    webview_windows.keys().collect::<Vec<_>>()
                ),
            );

            if let Some((label, window)) = webview_windows.iter().next() {
                logger::info(
                    LogTag::System,
                    &format!("Showing window with label: {}", label),
                );
                navigate_and_show_window(window.clone(), zoom_level);
            } else {
                logger::error(LogTag::System, "❌ No windows available at all");
            }
        }
    }
}

/// Navigate the window to the dashboard URL and show it
fn navigate_and_show_window(window: tauri::WebviewWindow, zoom_level: Arc<Mutex<f64>>) {
    logger::info(
        LogTag::System,
        &format!(
            "🔄 Navigating window '{}' to dashboard URL...",
            window.label()
        ),
    );

    // Use Tauri's native navigation instead of JavaScript reload
    // This ensures the webview actually loads the URL with fresh content
    match window.navigate("http://localhost:8080/".parse().unwrap()) {
        Ok(_) => {
            logger::info(LogTag::System, "✅ Window navigation triggered");
        }
        Err(e) => {
            logger::warning(
                LogTag::System,
                &format!("⚠️  Navigation failed (may already be at URL): {}", e),
            );
        }
    }

    // Small delay to let navigation start before showing
    std::thread::sleep(Duration::from_millis(200));

    // Try to load saved zoom level from config (if available)
    let mut zoom = *zoom_level.lock().unwrap();
    if crate::config::is_config_initialized() {
        let saved_zoom = crate::config::with_config(|cfg| cfg.gui.zoom_level);
        if saved_zoom != zoom && saved_zoom >= 0.5 && saved_zoom <= 3.0 {
            zoom = saved_zoom;
            *zoom_level.lock().unwrap() = zoom;
            logger::info(
                LogTag::System,
                &format!("📋 Loaded saved zoom level from config: {:.0}%", zoom * 100.0),
            );
        }
    }

    // Apply zoom level
    if zoom != 1.0 {
        match window.set_zoom(zoom) {
            Ok(_) => {
                logger::info(
                    LogTag::System,
                    &format!("✅ Applied zoom level: {:.0}%", zoom * 100.0),
                );
            }
            Err(e) => {
                logger::warning(
                    LogTag::System,
                    &format!("⚠️  Failed to apply zoom: {}", e),
                );
            }
        }
    }

    // Now show the window
    match window.show() {
        Ok(_) => {
            logger::info(LogTag::System, "✅ GUI window shown with dashboard loaded");

            // Also try to focus/raise the window
            if let Err(e) = window.set_focus() {
                logger::warning(
                    LogTag::System,
                    &format!("⚠️  Could not focus window: {}", e),
                );
            } else {
                logger::info(LogTag::System, "✅ Window focused and brought to front");
            }
        }
        Err(e) => {
            logger::error(LogTag::System, &format!("❌ Failed to show window: {}", e));
        }
    }
}
