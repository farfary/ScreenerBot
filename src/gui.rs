/// GUI module for ScreenerBot desktop application
///
/// Handles Tauri window management and integration with the headless ScreenerBot backend.
///
/// Security features:
/// - Dynamic port selection (prevents conflicts, not guessable)
/// - Security token validation (prevents external browser access)
/// - 127.0.0.1 binding only (no network access)
use crate::global;
use crate::logger::{self, LogTag};
use crate::process_lock::ProcessLock;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

/// Run bot in GUI mode with Tauri window
///
/// This function:
/// 1. **Sets GUI mode flag** - enables security token validation
/// 2. **Acquires process lock** - prevents multiple instances
/// 3. Spawns the ScreenerBot backend (with dynamic port + security token)
/// 4. Builds and runs the Tauri desktop application
/// 5. Waits for webserver to be ready and injects security token into webview
/// 6. Shows the window with the dashboard loaded
///
/// Security: In GUI mode, the webserver uses a random port and requires
/// a security token header for all requests, preventing browser access.
pub async fn run_gui_mode() -> Result<(), String> {
  logger::info(LogTag::System, "Initializing Tauri desktop application");

  // 1. Set GUI mode FIRST - this enables security features in webserver
  global::set_gui_mode(true);
  logger::info(
    LogTag::System,
    "GUI mode enabled - webserver will use secure random port",
  );

  // 2. Acquire process lock BEFORE starting anything
  let process_lock = ProcessLock::acquire()?;
  logger::info(
    LogTag::System,
    "Process lock acquired - no other instance running",
  );

  // Start the ScreenerBot backend in a background task (pass the lock to keep it alive)
  tokio::spawn(async move {
    logger::info(LogTag::System, "Starting ScreenerBot backend services...");

    // Start the full ScreenerBot system (includes webserver with dynamic port)
    match crate::run::run_bot_with_lock(process_lock).await {
      Ok(_) => {
        logger::info(
          LogTag::System,
          "ScreenerBot backend started successfully",
        );
      }
      Err(e) => {
        logger::error(
          LogTag::System,
          &format!("Failed to start ScreenerBot backend: {}", e),
        );
      }
    }
  });

  // Start with default zoom (config will be loaded later by backend)
  let initial_zoom = 1.0;
  logger::info(LogTag::System, "Starting with default zoom level: 100%");

  // Shared zoom level state (used by keyboard shortcuts)
  let zoom_level = Arc::new(Mutex::new(initial_zoom));

  // Build and run Tauri application
  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_global_shortcut::Builder::new().build())
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .invoke_handler(tauri::generate_handler![smart_maximize, enable_window_drag])
    .setup({
      let zoom_level_clone = Arc::clone(&zoom_level);
      move |app| {
        let app_handle = app.handle().clone();

        // Configure window theme and title bar style programmatically
        if let Some(window) = app.get_webview_window("main") {
          logger::info(LogTag::System, "Configuring window theme and title bar...");

          // Set initial theme to dark
          if let Err(e) = window.set_theme(Some(tauri::Theme::Dark)) {
            logger::warning(
              LogTag::System,
              &format!("Failed to set window theme: {}", e),
            );
          } else {
            logger::info(LogTag::System, "Window theme set to Dark");
          }

          // macOS: Set overlay title bar style
          #[cfg(target_os = "macos")]
          {
            if let Err(e) = window.set_title_bar_style(tauri::TitleBarStyle::Overlay) {
              logger::warning(
                LogTag::System,
                &format!("Failed to set title bar style: {}", e),
              );
            } else {
              logger::info(LogTag::System, "macOS title bar set to Overlay");
            }
          }

          logger::info(LogTag::System, "Window configuration complete");

          // Expose devtools flag to frontend
          // When devtools feature is enabled, the "Inspect Element" option shows in context menu
          #[cfg(feature = "devtools")]
          {
            if let Err(e) = window.eval("window.__SCREENERBOT_DEVTOOLS__ = true;") {
              logger::warning(
                LogTag::System,
                &format!("Failed to set devtools flag: {}", e),
              );
            } else {
              logger::info(LogTag::System, "Devtools feature enabled - Inspect Element available in context menu");
            }
          }

          #[cfg(not(feature = "devtools"))]
          {
            if let Err(e) = window.eval("window.__SCREENERBOT_DEVTOOLS__ = false;") {
              logger::warning(
                LogTag::System,
                &format!("Failed to set devtools flag: {}", e),
              );
            }
            logger::info(LogTag::System, "Production build - Inspect Element hidden from context menu");
          }

          logger::info(LogTag::System, "Window configuration complete");
        } else {
          logger::warning(LogTag::System, "Main window not found during setup");
        }

        // Register global keyboard shortcuts for zoom + reload
        register_window_shortcuts(app, Arc::clone(&zoom_level_clone))?;

        logger::info(
          LogTag::System,
          "Tauri setup started - window created but hidden",
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

/// Get the dashboard base URL (waits for port to be assigned)
fn get_dashboard_url() -> String {
  // Wait for webserver to start and assign port
  let start = Instant::now();
  let timeout = Duration::from_secs(60);

  loop {
    let port = global::get_webserver_port();
    if port != 0 {
      let url = format!("http://127.0.0.1:{}", port);
      logger::info(
        LogTag::System,
        &format!("Dashboard URL: {} (port assigned after {:?})", url, start.elapsed()),
      );
      return url;
    }

    if start.elapsed() > timeout {
      logger::error(
        LogTag::System,
        "Timeout waiting for webserver port assignment",
      );
      // Fall back to a default (will likely fail, but better than hanging)
      return "http://127.0.0.1:8080".to_string();
    }

    std::thread::sleep(Duration::from_millis(50));
  }
}

/// Get security token (waits for it to be generated)
fn get_security_token() -> Option<String> {
  let start = Instant::now();
  let timeout = Duration::from_secs(60);

  loop {
    if let Some(token) = global::get_security_token() {
      logger::debug(
        LogTag::System,
        &format!(
          "Security token obtained after {:?}: {}...",
          start.elapsed(),
          &token[..8]
        ),
      );
      return Some(token);
    }

    if start.elapsed() > timeout {
      logger::error(
        LogTag::System,
        "Timeout waiting for security token generation",
      );
      return None;
    }

    std::thread::sleep(Duration::from_millis(50));
  }
}

/// Security token header name
const SECURITY_TOKEN_HEADER: &str = "X-ScreenerBot-Token";

/// Create HTTP client with security token header
fn create_secure_client() -> reqwest::blocking::Client {
  let token = get_security_token().unwrap_or_default();

  let mut headers = reqwest::header::HeaderMap::new();
  headers.insert(
    SECURITY_TOKEN_HEADER,
    reqwest::header::HeaderValue::from_str(&token).unwrap_or_else(|_| {
      reqwest::header::HeaderValue::from_static("")
    }),
  );

  reqwest::blocking::Client::builder()
    .timeout(Duration::from_millis(500))
    .default_headers(headers)
    .build()
    .unwrap()
}

/// Register global keyboard shortcuts for zoom control + reload
fn register_window_shortcuts(
  app: &mut tauri::App,
  zoom_level: Arc<Mutex<f64>>,
) -> Result<(), Box<dyn std::error::Error>> {
  let app_handle = app.handle().clone();

  // Determine modifier key based on platform
  let modifier = if cfg!(target_os = "macos") {
    Modifiers::META
  } else {
    Modifiers::CONTROL
  };

  // Register Zoom In (Cmd/Ctrl + Plus or =)
  let zoom_in_shortcut = Shortcut::new(Some(modifier), Code::Equal);
  let zoom_level_in = Arc::clone(&zoom_level);
  let app_handle_in = app_handle.clone();
  let last_zoom_in = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(1000)));

  app.global_shortcut()
    .on_shortcut(zoom_in_shortcut, move |_app, _event, _shortcut| {
      let mut last = last_zoom_in.lock().unwrap();
      if last.elapsed() < Duration::from_millis(300) {
        return;
      }
      *last = Instant::now();
      drop(last);

      if let Some(window) = app_handle_in.get_webview_window("main") {
        let mut zoom = zoom_level_in.lock().unwrap();
        *zoom = (*zoom + 0.1).min(3.0);
        let zoom_val = *zoom;
        drop(zoom);

        if let Err(e) = window.set_zoom(zoom_val) {
          logger::warning(LogTag::System, &format!("Failed to set zoom: {}", e));
        } else {
          logger::info(
            LogTag::System,
            &format!("Zoom in: {:.0}%", zoom_val * 100.0),
          );
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
      let mut last = last_zoom_out.lock().unwrap();
      if last.elapsed() < Duration::from_millis(300) {
        return;
      }
      *last = Instant::now();
      drop(last);

      if let Some(window) = app_handle_out.get_webview_window("main") {
        let mut zoom = zoom_level_out.lock().unwrap();
        *zoom = (*zoom - 0.1).max(0.5);
        let zoom_val = *zoom;
        drop(zoom);

        if let Err(e) = window.set_zoom(zoom_val) {
          logger::warning(LogTag::System, &format!("Failed to set zoom: {}", e));
        } else {
          logger::info(
            LogTag::System,
            &format!("Zoom out: {:.0}%", zoom_val * 100.0),
          );
          save_zoom_to_config(zoom_val);
        }
      }
    })?;

  // Register Reset Zoom (Cmd/Ctrl + 0)
  let zoom_reset_shortcut = Shortcut::new(Some(modifier), Code::Digit0);
  let zoom_level_reset = Arc::clone(&zoom_level);
  let last_zoom_reset = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(1000)));
  let app_handle_reset = app_handle.clone();

  app.global_shortcut()
    .on_shortcut(zoom_reset_shortcut, move |_app, _event, _shortcut| {
      let mut last = last_zoom_reset.lock().unwrap();
      if last.elapsed() < Duration::from_millis(300) {
        return;
      }
      *last = Instant::now();
      drop(last);

      if let Some(window) = app_handle_reset.get_webview_window("main") {
        let mut zoom = zoom_level_reset.lock().unwrap();
        *zoom = 1.0;
        let zoom_val = *zoom;
        drop(zoom);

        if let Err(e) = window.set_zoom(zoom_val) {
          logger::warning(LogTag::System, &format!("Failed to reset zoom: {}", e));
        } else {
          logger::info(LogTag::System, "Zoom reset: 100%");
          save_zoom_to_config(zoom_val);
        }
      }
    })?;

  // Register Reload (Cmd/Ctrl + R)
  let reload_shortcut = Shortcut::new(Some(modifier), Code::KeyR);
  let reload_handle = app_handle.clone();
  let last_reload = Arc::new(Mutex::new(Instant::now() - Duration::from_millis(1000)));

  app.global_shortcut()
    .on_shortcut(reload_shortcut, move |_app, _event, _shortcut| {
      let mut last = last_reload.lock().unwrap();
      if last.elapsed() < Duration::from_millis(300) {
        return;
      }
      *last = Instant::now();
      drop(last);

      if let Some(window) = reload_handle.get_webview_window("main") {
        match window.eval("window.location.reload()") {
          Ok(_) => logger::info(LogTag::System, "Reloaded dashboard (Cmd/Ctrl + R)"),
          Err(e) => logger::warning(
            LogTag::System,
            &format!("Failed to reload dashboard via shortcut: {}", e),
          ),
        }
      } else {
        logger::warning(LogTag::System, "Reload shortcut triggered without window");
      }
    })?;

  logger::info(
    LogTag::System,
    "Registered window shortcuts (Ctrl/Cmd +/-/0/R)",
  );

  Ok(())
}

/// Save zoom level to config file
fn save_zoom_to_config(zoom: f64) {
  std::thread::spawn(move || {
    if let Err(e) = crate::config::update_config_section(
      |config| {
        config.gui.zoom_level = zoom;
      },
      true,
    ) {
      logger::warning(
        LogTag::System,
        &format!("Failed to save zoom to config: {}", e),
      );
    }
  });
}

/// Wait for dashboard to be ready and show the window
fn wait_for_dashboard_and_show_window(app_handle: tauri::AppHandle, zoom_level: Arc<Mutex<f64>>) {
  logger::info(
    LogTag::System,
    "Waiting for webserver port and security token...",
  );

  // Get dashboard URL (waits for port assignment)
  let base_url = get_dashboard_url();

  // Create client with security token
  let client = create_secure_client();

  // Wait for bootstrap to be ready
  wait_for_bootstrap_ready(&client, &base_url);

  let mut poll_count = 0;

  // Poll dashboard root endpoint until it returns HTML content
  loop {
    poll_count += 1;

    match client.get(&format!("{}/", base_url)).send() {
      Ok(response) => {
        let status = response.status();
        logger::debug(
          LogTag::System,
          &format!("Poll #{}: HTTP {} from dashboard", poll_count, status),
        );

        if status.is_success() {
          if let Ok(text) = response.text() {
            if text.contains("<!doctype html") || text.contains("<html") {
              logger::info(
                LogTag::System,
                &format!("Dashboard HTML ready after {} polls", poll_count),
              );
              break;
            } else {
              logger::warning(
                LogTag::System,
                &format!(
                  "Got 200 response but no HTML content (got {} bytes)",
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

    std::thread::sleep(Duration::from_millis(50));
  }

  logger::info(LogTag::System, "Looking for main window to show...");

  match app_handle.get_webview_window("main") {
    Some(window) => {
      logger::info(LogTag::System, "Found main window, showing it now");
      navigate_and_show_window(window, zoom_level, &base_url);
    }
    None => {
      logger::error(
        LogTag::System,
        "Failed to find main window with label 'main'",
      );

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
        navigate_and_show_window(window.clone(), zoom_level, &base_url);
      } else {
        logger::error(LogTag::System, "No windows available at all");
      }
    }
  }
}

#[derive(Debug, Deserialize)]
struct BootstrapStatus {
  ready_for_requests: bool,
  initialization_required: bool,
  message: Option<String>,
}

fn wait_for_bootstrap_ready(client: &reqwest::blocking::Client, base_url: &str) {
  logger::info(
    LogTag::System,
    "Waiting for core services to report ready state...",
  );
  let mut attempts = 0;
  loop {
    attempts += 1;
    match client
      .get(&format!("{}/api/system/bootstrap", base_url))
      .send()
    {
      Ok(response) if response.status().is_success() => {
        match response.json::<BootstrapStatus>() {
          Ok(status) => {
            if status.initialization_required || status.ready_for_requests {
              logger::info(
                LogTag::System,
                &format!(
                  "Bootstrap ready after {} checks ({})",
                  attempts,
                  status.message.unwrap_or_else(|| "no message".to_string())
                ),
              );
              break;
            }
          }
          Err(err) => {
            logger::debug(
              LogTag::System,
              &format!("Bootstrap status parse failed: {}", err),
            );
          }
        }
      }
      Ok(response) => {
        logger::debug(
          LogTag::System,
          &format!(
            "Bootstrap check #{} returned HTTP {}",
            attempts,
            response.status()
          ),
        );
      }
      Err(err) => {
        if attempts == 1 || attempts % 20 == 0 {
          logger::debug(
            LogTag::System,
            &format!("Bootstrap endpoint not ready yet: {}", err),
          );
        }
      }
    }

    std::thread::sleep(Duration::from_millis(200));
  }
}

/// Wait for frontend JavaScript to initialize
fn wait_for_frontend_ready(window: &tauri::WebviewWindow, base_url: &str) {
  logger::info(LogTag::System, "Waiting for frontend to initialize...");

  let start = Instant::now();
  let min_wait = Duration::from_millis(2500);
  let timeout = Duration::from_secs(30);
  let mut attempts = 0;
  let mut url_confirmed = false;

  // Extract port from base_url for URL checking
  let port = global::get_webserver_port();

  loop {
    attempts += 1;

    if !url_confirmed {
      if let Ok(result) = window.url() {
        let url_str = result.to_string();
        // Check for dynamic port in URL
        if url_str.contains(&format!("127.0.0.1:{}", port))
          || url_str.contains(&format!("localhost:{}", port))
        {
          url_confirmed = true;
          logger::debug(
            LogTag::System,
            &format!("Navigation confirmed at attempt {}", attempts),
          );
        }
      }
    }

    if url_confirmed && start.elapsed() >= min_wait {
      logger::info(
        LogTag::System,
        &format!("Frontend ready after {:?}", start.elapsed()),
      );
      break;
    }

    if start.elapsed() > timeout {
      logger::warning(
        LogTag::System,
        &format!(
          "Frontend ready timeout after {:?}, proceeding anyway",
          start.elapsed()
        ),
      );
      break;
    }

    std::thread::sleep(Duration::from_millis(100));
  }
}

/// Navigate the window to the dashboard URL and show it
fn navigate_and_show_window(
  window: tauri::WebviewWindow,
  zoom_level: Arc<Mutex<f64>>,
  base_url: &str,
) {
  logger::info(
    LogTag::System,
    &format!(
      "Navigating window '{}' to dashboard URL: {}",
      window.label(),
      base_url
    ),
  );

  // Navigate to the dynamic URL
  // Security token is now injected via HTML template, not JavaScript eval
  match window.navigate(format!("{}/", base_url).parse().unwrap()) {
    Ok(_) => {
      logger::info(LogTag::System, "Window navigation triggered");
    }
    Err(e) => {
      logger::warning(
        LogTag::System,
        &format!("Navigation failed (may already be at URL): {}", e),
      );
    }
  }

  // Wait for frontend to initialize
  wait_for_frontend_ready(&window, base_url);

  // Load saved zoom level from config
  let mut zoom = *zoom_level.lock().unwrap();
  if crate::config::is_config_initialized() {
    let saved_zoom = crate::config::with_config(|cfg| cfg.gui.zoom_level);
    if saved_zoom != zoom && saved_zoom >= 0.5 && saved_zoom <= 3.0 {
      zoom = saved_zoom;
      *zoom_level.lock().unwrap() = zoom;
      logger::info(
        LogTag::System,
        &format!(
          "Loaded saved zoom level from config: {:.0}%",
          zoom * 100.0
        ),
      );
    }
  }

  // Apply zoom level
  if zoom != 1.0 {
    match window.set_zoom(zoom) {
      Ok(_) => {
        logger::info(
          LogTag::System,
          &format!("Applied zoom level: {:.0}%", zoom * 100.0),
        );
      }
      Err(e) => {
        logger::warning(LogTag::System, &format!("Failed to apply zoom: {}", e));
      }
    }
  }

  // Show the window
  match window.show() {
    Ok(_) => {
      logger::info(LogTag::System, "GUI window shown with dashboard loaded");

      if let Err(e) = window.set_focus() {
        logger::warning(
          LogTag::System,
          &format!("Could not focus window: {}", e),
        );
      } else {
        logger::info(LogTag::System, "Window focused and brought to front");
      }
    }
    Err(e) => {
      logger::error(LogTag::System, &format!("Failed to show window: {}", e));
    }
  }
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

/// Smart maximize command
#[tauri::command]
fn smart_maximize(window: tauri::WebviewWindow) -> Result<(), String> {
  logger::info(LogTag::System, "Smart maximize command invoked");

  #[cfg(target_os = "macos")]
  {
    crate::macos_window::smart_maximize_macos(&window)
  }

  #[cfg(not(target_os = "macos"))]
  {
    let is_maximized = window
      .is_maximized()
      .map_err(|e| format!("Failed to check maximize state: {}", e))?;

    if is_maximized {
      window
        .unmaximize()
        .map_err(|e| format!("Failed to unmaximize: {}", e))
    } else {
      window
        .maximize()
        .map_err(|e| format!("Failed to maximize: {}", e))
    }
  }
}

/// Enable window drag handling for macOS
/// Note: We no longer use setMovableByWindowBackground because it makes the
/// ENTIRE window draggable. Instead, we rely on the JS-based drag handling
/// in theme.js combined with CSS -webkit-app-region properties.
/// This command now only sets acceptsFirstMouse for better UX.
#[tauri::command]
fn enable_window_drag(window: tauri::WebviewWindow) -> Result<(), String> {
  #[cfg(target_os = "macos")]
  {
    // Only enable accepts first mouse for better dragging UX when window is unfocused
    // DO NOT use set_window_draggable() as it makes entire window draggable
    crate::macos_window::set_accepts_first_mouse(&window, true)?;
    Ok(())
  }

  #[cfg(not(target_os = "macos"))]
  {
    // On other platforms, dragging works normally
    let _ = window;
    Ok(())
  }
}
