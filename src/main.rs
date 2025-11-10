// Prevents additional console window on Windows in release builds
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use screenerbot::{
    arguments::{is_clean_wallet_data_enabled, is_force_enabled, is_gui_enabled, is_reset_enabled, patterns, print_debug_info, print_help},
    logger::{self as logger, LogTag},
};

/// Main entry point for ScreenerBot
///
/// Unified entry point that handles:
/// - Special modes (--reset, --clean-wallet-data, --help)
/// - GUI mode (--gui): Desktop window with embedded webserver
/// - Headless mode (default): Background service with webserver on :8080
///
/// Bot ALWAYS runs unless a special mode is specified.
/// No --run flag needed - simplified UX.
#[tokio::main]
async fn main() {
    // Ensure all directories exist BEFORE logger initialization
    // (Logger needs logs directory to create log files)
    if let Err(e) = screenerbot::paths::ensure_all_directories() {
        eprintln!("❌ Failed to create required directories: {}", e);
        std::process::exit(1);
    }

    // Initialize logger system (now safe to create log files)
    logger::init();

    // Check for help request first (before any other processing)
    if patterns::is_help_requested() {
        print_help();
        std::process::exit(0);
    }

    // Log startup information
    logger::info(LogTag::System, "🚀 ScreenerBot starting up...");

    // Print debug information if any debug modes are enabled
    print_debug_info();

    // =========================================================================
    // SPECIAL MODES (execute and exit)
    // =========================================================================

    // Clean wallet data mode - execute and exit
    if is_clean_wallet_data_enabled() {
        logger::info(LogTag::System, "🧹 Clean wallet data mode enabled");

        println!("\n⚠️  WARNING: This will DELETE all stored data:");
        println!(
            "   - Transaction history ({})",
            screenerbot::paths::get_transactions_db_path().display()
        );
        println!(
            "   - Position history ({})",
            screenerbot::paths::get_positions_db_path().display()
        );
        println!(
            "   - Wallet snapshots ({})",
            screenerbot::paths::get_wallet_db_path().display()
        );
        println!("\nThis action is required when switching to a different wallet.");
        print!("\nType 'yes' to confirm: ");

        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_lowercase() == "yes" {
            match screenerbot::wallet_validation::WalletValidator::clean_all_databases().await {
                Ok(_) => {
                    logger::info(
                        LogTag::System,
                        "✅ All databases cleaned successfully. You can now start the bot.",
                    );
                    std::process::exit(0);
                }
                Err(e) => {
                    logger::error(LogTag::System, &format!("❌ Cleanup failed: {}", e));
                    std::process::exit(1);
                }
            }
        } else {
            logger::info(LogTag::System, "❌ Cleanup cancelled");
            std::process::exit(0);
        }
    }

    // Reset mode - execute and exit
    if is_reset_enabled() {
        logger::info(LogTag::System, "🔄 Reset mode enabled");

        let config = screenerbot::reset::ResetConfig {
            force: is_force_enabled(),
            ..Default::default()
        };

        match screenerbot::reset::execute_extended_reset(config) {
            Ok(()) => {
                logger::info(LogTag::System, "✅ Reset completed successfully");
                std::process::exit(0);
            }
            Err(e) => {
                logger::error(LogTag::System, &format!("❌ Reset failed: {}", e));
                std::process::exit(1);
            }
        }
    }

    // =========================================================================
    // MAIN BOT EXECUTION
    // =========================================================================

    // Check if we're running from an app bundle or if --gui flag is provided
    let is_bundled = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.to_str().map(|s| s.contains(".app/Contents/MacOS") || s.contains(".exe")))
        .unwrap_or(false);
    
    let should_run_gui = is_gui_enabled() || is_bundled;

    if should_run_gui {
        logger::info(LogTag::System, "🖥️  Launching in GUI mode");
        if let Err(e) = run_gui_mode().await {
            logger::error(LogTag::System, &format!("❌ GUI mode failed: {}", e));
            std::process::exit(1);
        }
    } else {
        // Headless mode (default)
        logger::info(LogTag::System, "🚀 Launching in headless mode");
        logger::info(LogTag::System, "🌐 Webserver will be available at http://localhost:8080");
        
        match screenerbot::run::run_bot().await {
            Ok(_) => {
                logger::info(LogTag::System, "✅ ScreenerBot completed successfully");
            }
            Err(e) => {
                logger::error(LogTag::System, &format!("❌ ScreenerBot failed: {}", e));
                std::process::exit(1);
            }
        }
    }
}

/// Run bot in GUI mode with Tauri window
async fn run_gui_mode() -> Result<(), String> {
    use std::time::Duration;
    use tauri::Manager;

    logger::info(LogTag::System, "🖥️  Initializing Tauri desktop application");

    // Start the ScreenerBot backend in a background task
    tokio::spawn(async move {
        logger::info(
            LogTag::System,
            "Starting ScreenerBot backend services...",
        );

        // Start the full ScreenerBot system (includes webserver on :8080)
        match screenerbot::run::run_bot().await {
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
                                            &format!("⚠️  Got 200 response but no HTML content (got {} bytes)", text.len()),
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
                    logger::info(
                        LogTag::System,
                        "🔄 Navigating window to dashboard URL...",
                    );
                    
                    // Use Tauri's native navigation instead of JavaScript reload
                    // This ensures the webview actually loads the URL with fresh content
                    if let Err(e) = window.navigate("http://localhost:8080/".parse().unwrap()) {
                        logger::error(
                            LogTag::System,
                            &format!("❌ Failed to navigate window: {}", e),
                        );
                        return;
                    }
                    
                    logger::info(
                        LogTag::System,
                        "✅ Window navigation triggered",
                    );

                    // Adjust zoom based on monitor scale factor so retina displays render correctly
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
                    
                    // Small delay to let navigation start before showing
                    std::thread::sleep(Duration::from_millis(200));
                    
                    // Now show the window
                    match window.show() {
                        Ok(_) => {
                            logger::info(
                                LogTag::System,
                                "✅ GUI window shown with dashboard loaded",
                            );
                        }
                        Err(e) => {
                            logger::error(
                                LogTag::System,
                                &format!("❌ Failed to show window: {}", e),
                            );
                        }
                    }
                } else {
                    logger::error(
                        LogTag::System,
                        "❌ Failed to find main window",
                    );
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri application error: {}", e))?;

    Ok(())
}
