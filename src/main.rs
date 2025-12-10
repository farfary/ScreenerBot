// Prevents additional console window on Windows in release builds
#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use screenerbot::{
  arguments::{
    is_clean_wallet_data_enabled, is_force_enabled, is_gui_enabled,
    is_reset_default_configs_enabled, is_reset_enabled, patterns, print_debug_info, print_help,
  },
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
 eprintln!("Failed to create required directories: {}", e);
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
 logger::info(LogTag::System, "ScreenerBot starting up...");

  // Print debug information if any debug modes are enabled
  print_debug_info();

  // =========================================================================
  // SPECIAL MODES (execute and exit)
  // =========================================================================

  // Clean wallet data mode - execute and exit
  if is_clean_wallet_data_enabled() {
 logger::info(LogTag::System, "Clean wallet data mode enabled");

    println!("\n WARNING: This will DELETE all stored data:");
    println!(
 "- Transaction history ({})",
      screenerbot::paths::get_transactions_db_path().display()
    );
    println!(
 "- Position history ({})",
      screenerbot::paths::get_positions_db_path().display()
    );
    println!(
 "- Wallet snapshots ({})",
      screenerbot::paths::get_wallet_db_path().display()
    );
    println!("\nThis action is required when switching to a different wallet.");
 print!("\nType 'yes'to confirm: ");

    use std::io::{self, Write};
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

 if input.trim().to_lowercase() == "yes"{
      match screenerbot::wallet_validation::WalletValidator::clean_all_databases().await {
        Ok(_) => {
          logger::info(
            LogTag::System,
 "All databases cleaned successfully. You can now start the bot.",
          );
          std::process::exit(0);
        }
        Err(e) => {
 logger::error(LogTag::System, &format!("Cleanup failed: {}", e));
          std::process::exit(1);
        }
      }
    } else {
 logger::info(LogTag::System, "Cleanup cancelled");
      std::process::exit(0);
    }
  }

  // Reset config to defaults mode - execute and exit
  if is_reset_default_configs_enabled() {
 logger::info(LogTag::System, "Reset config to defaults mode enabled");

    // Load current config first (to get wallet + RPC)
    if let Err(e) = screenerbot::config::load_config() {
      logger::error(
        LogTag::System,
 &format!("Failed to load current config: {}", e),
      );
      std::process::exit(1);
    }

    // Execute reset
    match screenerbot::config::reset_config_to_defaults_preserving_credentials() {
      Ok(_) => {
        logger::info(
          LogTag::System,
 "Config reset completed successfully. Restart the bot to apply changes.",
        );
        std::process::exit(0);
      }
      Err(e) => {
 logger::error(LogTag::System, &format!("Config reset failed: {}", e));
        std::process::exit(1);
      }
    }
  }

  // Reset mode - execute and exit
  if is_reset_enabled() {
 logger::info(LogTag::System, "Reset mode enabled");

    let config = screenerbot::reset::ResetConfig {
      force: is_force_enabled(),
      ..Default::default()
    };

    match screenerbot::reset::execute_extended_reset(config) {
      Ok(()) => {
 logger::info(LogTag::System, "Reset completed successfully");
        std::process::exit(0);
      }
      Err(e) => {
 logger::error(LogTag::System, &format!("Reset failed: {}", e));
        std::process::exit(1);
      }
    }
  }

  // =========================================================================
  // MAIN BOT EXECUTION
  // =========================================================================

  // Check if we're running from an app bundle or if --gui flag is provided
  // Detection patterns:
  // - macOS: .app/Contents/MacOS (app bundle)
  // - Windows: .exe (any executable)
  // - Linux: /usr/bin/ (installed from .deb), AppImage, /opt/ (common install locations)
  let is_bundled = std::env::current_exe()
    .ok()
    .and_then(|exe| {
      exe.to_str().map(|s| {
        // macOS app bundle
        s.contains(".app/Contents/MacOS")
          // Windows executable
          || s.contains(".exe")
          // Linux: installed from .deb to /usr/bin
          || s.starts_with("/usr/bin/")
          // Linux: AppImage (runs from /tmp/.mount_* or similar)
          || s.contains(".mount_") || s.contains("AppImage")
          // Linux: installed to /opt (common for third-party apps)
          || s.starts_with("/opt/")
      })
    })
    .unwrap_or(false);

  let should_run_gui = is_gui_enabled() || is_bundled;

  if should_run_gui {
 logger::info(LogTag::System, "Launching in GUI mode");
    if let Err(e) = screenerbot::gui::run_gui_mode().await {
 logger::error(LogTag::System, &format!("GUI mode failed: {}", e));
      std::process::exit(1);
    }
  } else {
    // Headless mode (default)
 logger::info(LogTag::System, "Launching in headless mode");
    logger::info(
      LogTag::System,
 "Webserver will be available at http://localhost:8080",
    );

    match screenerbot::run::run_bot().await {
      Ok(_) => {
 logger::info(LogTag::System, "ScreenerBot completed successfully");
      }
      Err(e) => {
 logger::error(LogTag::System, &format!("ScreenerBot failed: {}", e));
        std::process::exit(1);
      }
    }
  }
}
