//! Main run loop — starts the service manager and runs until shutdown signal.
//!
//! Orchestrates bot lifecycle: process lock, configuration, wallet setup,
//! service registration, and graceful shutdown handling.

mod bootstrap;
mod llm_init;
pub mod services;
mod shutdown;

use bootstrap::{initialize_ai_runtime_if_enabled, initialize_full_runtime};

use crate::{
    arguments::{print_banner, print_help, print_version, set_cmd_args},
    config::utils::load_config,
    errors::StartupError,
    global,
    logger::{self, LogTag},
    process::lock::ProcessLock,
    process::profiling,
    services::ServiceManager,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Full process boot sequence: CLI early-exits, banner, logger, config, panic
/// hook, ctrl-c shutdown signal, one-shot wallet reset, run the bot, and
/// restart-after-graceful-shutdown handling. Called once from `main()`.
pub async fn boot() {
    // Store command line arguments
    set_cmd_args(std::env::args().collect());

    // Handle help flag
    if std::env::args().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    // Handle version flag
    if std::env::args().any(|arg| arg == "--version" || arg == "-v") {
        print_version();
        return;
    }

    print_banner();

    // Initialize logger
    crate::logger::init();
    logger::info(
        LogTag::System,
        "Logger initialized, attempting to load config...",
    );

    // Load configuration
    if let Err(e) = load_config() {
        logger::error(
            LogTag::System,
            &format!("Failed to load configuration: {e}"),
        );
        return;
    }
    logger::info(LogTag::System, "Configuration loaded successfully");

    // Detect and log the system/network proxy once (used by HTTP, RPC and WS).
    // Must run AFTER config load so `[network] proxy` is available.
    crate::net::log_detected_proxy();

    // Set up panic hook for crash notifications (after config is loaded)
    crate::process::panic_hook::install();

    logger::info(LogTag::System, "ScreenerBot starting...");

    // Set up shutdown signal handler
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = shutdown_flag.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl+c");
        logger::info(LogTag::System, "Shutdown signal received");
        shutdown_flag_clone.store(true, Ordering::SeqCst);
        crate::process::shutdown::request_shutdown();
    });

    // Handle one-shot wallet-data reset (safe: backs up before clearing) then
    // continue into a normal boot, so a single relaunch fixes a wallet mismatch.
    // This is what the GUI's "Reset wallet data & restart" and the documented
    // `screenerbot --clean-wallet-data` both trigger.
    if crate::arguments::is_clean_wallet_data_enabled() {
        match crate::wallets::recovery::backup_and_clean_wallet_data() {
            Ok(backup_dir) => logger::info(
                LogTag::System,
                &format!(
                    "Wallet data reset complete (backup: {}). Continuing startup.",
                    backup_dir.display()
                ),
            ),
            Err(e) => logger::error(
                LogTag::System,
                &format!("Wallet data reset failed: {e}. Continuing startup anyway."),
            ),
        }
    }

    // Run the bot in headless mode. On a fatal startup failure, surface a
    // structured error to the terminal/log file and the GUI shell, then exit
    // with a non-zero code so callers (Electron, systemd) can tell a failed
    // boot apart from a clean shutdown.
    if let Err(e) = run_bot().await {
        e.emit();
        std::process::exit(1);
    }

    if crate::global::is_restart_requested() {
        crate::process::restart::restart_after_graceful_shutdown();
    }

    logger::info(LogTag::System, "ScreenerBot shutdown complete");
}

/// Main bot execution function — handles the full bot lifecycle with ServiceManager.
pub async fn run_bot() -> Result<(), StartupError> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    profiling::init_profiling();

    // 1. Ensure all required directories exist (safety backup, already done in main.rs)
    crate::paths::ensure_all_directories()
        .map_err(|e| format!("Failed to create required directories: {e}"))?;

    // 2. Acquire process lock to prevent multiple instances
    let process_lock = ProcessLock::acquire()?;

    // Run bot with the acquired lock
    run_bot_internal(process_lock).await
}

/// Internal bot execution with pre-acquired lock.
async fn run_bot_internal(_process_lock: ProcessLock) -> Result<(), StartupError> {
    logger::info(LogTag::System, "ScreenerBot starting up...");

    // 1. Set GUI mode if --gui flag is present (must be done early for webserver security)
    if crate::arguments::is_gui_enabled() {
        global::set_gui_mode(true);
        logger::info(LogTag::System, "GUI mode enabled");
    }

    // 2. Validate CLI arguments early (before any processing)
    if let Err(e) = crate::arguments::validate_port_argument() {
        return Err(StartupError::new(
            crate::errors::StartupErrorCode::ConfigInvalid,
            "Invalid startup option",
            e,
            "A command-line option is invalid. Start ScreenerBot without that option, or \
             correct it and try again.",
        ));
    }

    if let Err(e) = crate::arguments::validate_host_argument() {
        return Err(StartupError::new(
            crate::errors::StartupErrorCode::ConfigInvalid,
            "Invalid startup option",
            e,
            "A command-line option is invalid. Start ScreenerBot without that option, or \
             correct it and try again.",
        ));
    }

    // 3. Log CLI overrides (if provided)
    if let Some(port) = crate::arguments::get_port_override() {
        if crate::arguments::is_privileged_port(port) {
            logger::warning(
                LogTag::System,
                &format!(
                    "Port {} requires elevated privileges (root/Administrator)",
                    port
                ),
            );
        }

        logger::info(LogTag::System, &format!("CLI override: Using port {port}"));
    }

    if let Some(host) = crate::arguments::get_host_override() {
        logger::info(LogTag::System, &format!("CLI override: Using host {host}"));

        if host == "0.0.0.0" {
            logger::warning(
                LogTag::System,
                "Binding to 0.0.0.0 allows remote access - ensure firewall is configured",
            );
        }
    }

    if crate::arguments::get_port_override().is_none()
        && crate::arguments::get_host_override().is_none()
    {
        logger::debug(
            LogTag::System,
            "No webserver CLI overrides provided, using config/defaults",
        );
    }

    // 4. Check if config.toml exists (determines initialization mode)
    let config_path = crate::paths::get_config_path();
    let config_exists = config_path.exists();

    // Dashboard-owned persistence is independent of wallet/RPC setup. Routes
    // for actions, AI instructions, and chat exist in every dashboard mode.
    bootstrap::initialize_dashboard_persistence().await?;

    if !config_exists {
        logger::info(
            LogTag::System,
            "No config.toml found - starting in initialization mode",
        );
        logger::info(
            LogTag::System,
            "Webserver will start on http://localhost:8080 for initial setup",
        );

        // Set initialization flag to false (services will be gated)
        global::INITIALIZATION_COMPLETE.store(false, std::sync::atomic::Ordering::SeqCst);

        // Create service manager with only webserver enabled
        let mut service_manager = ServiceManager::new().await.map_err(|e| e.to_string())?;
        logger::info(LogTag::System, "Service manager initialized");

        // Register all services (but only webserver will be enabled)
        services::register_all_services(&mut service_manager);

        // Initialize global ServiceManager for webserver access
        crate::services::init_global_service_manager(service_manager).await;

        // Get mutable reference to continue
        let manager_ref = crate::services::get_service_manager()
            .await
            .ok_or("Failed to get ServiceManager reference")?;

        let mut service_manager = {
            let mut guard = manager_ref.write().await;
            guard.take().ok_or("ServiceManager was already taken")?
        };

        // Start only enabled services (webserver only in pre-init mode)
        service_manager
            .start_all()
            .await
            .map_err(|e| e.to_string())?;

        // Put it back for webserver access
        {
            let mut guard = manager_ref.write().await;
            *guard = Some(service_manager);
        }

        logger::info(
            LogTag::System,
            "Webserver started - complete initialization at http://localhost:8080",
        );
        logger::info(LogTag::System, "Waiting for setup choice...");

        // Wait until the user chooses Explore Mode or completes full setup.
        shutdown::wait_for_operational_mode_or_shutdown().await?;

        if global::is_explore_mode() {
            logger::info(LogTag::System, "Explore Mode ready");
        } else {
            logger::info(
                LogTag::System,
                "Initialization complete - all services running",
            );
        }
    } else {
        logger::info(
            LogTag::System,
            "Config.toml found - starting in normal mode",
        );

        // 4. Load configuration (if not already loaded by main.rs)
        if !crate::config::is_config_initialized() {
            crate::config::load_config().map_err(|e| format!("Failed to load config: {e}"))?;
            logger::info(LogTag::System, "Configuration loaded successfully");
        }

        // 4b. Detect Explore Mode: the user selected wallet-free browsing at first
        // run. The durable marker is `explore_mode_enabled`; an empty encrypted wallet is a
        // safety fallback (treat as Explore Mode rather than hard-failing later).
        let explore = crate::config::with_config(|cfg| {
            cfg.gui.dashboard.startup.explore_mode_enabled || cfg.wallet_encrypted.trim().is_empty()
        });

        if explore {
            logger::info(
                LogTag::System,
                "Explore Mode: wallet + RPC not configured - starting Explore tier only",
            );

            // Explore Mode: only the Explore tier runs. Wallet/RPC-dependent
            // services stay disabled and are filtered out of the startup order.
            global::set_explore_mode(true);
            global::INITIALIZATION_COMPLETE.store(false, std::sync::atomic::Ordering::SeqCst);

            // AI chat/providers are wallet-independent. If the saved Explore Mode
            // configuration enables AI, make the assistant usable here too.
            initialize_ai_runtime_if_enabled().await?;

            let mut service_manager = ServiceManager::new().await.map_err(|e| e.to_string())?;
            logger::info(LogTag::System, "Service manager initialized (Explore Mode)");

            services::register_all_services(&mut service_manager);
            crate::services::init_global_service_manager(service_manager).await;

            let manager_ref = crate::services::get_service_manager()
                .await
                .ok_or("Failed to get ServiceManager reference")?;

            let mut service_manager = {
                let mut guard = manager_ref.write().await;
                guard.take().ok_or("ServiceManager was already taken")?
            };

            service_manager
                .start_all()
                .await
                .map_err(|e| e.to_string())?;

            {
                let mut guard = manager_ref.write().await;
                *guard = Some(service_manager);
            }

            logger::info(
                LogTag::System,
                "Explore Mode active - complete wallet + RPC setup in the dashboard to enable trading",
            );
        } else {
            // Full mode has one initialization path whether entered at boot or
            // live from Explore Mode.
            initialize_full_runtime().await?;

            // Only expose wallet/RPC-backed services after every prerequisite
            // above succeeded.
            global::set_explore_mode(false);
            global::INITIALIZATION_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);

            // 9. Create service manager
            let mut service_manager = ServiceManager::new().await.map_err(|e| e.to_string())?;

            logger::info(LogTag::System, "Service manager initialized");

            // 10. Register all services
            services::register_all_services(&mut service_manager);

            // 11. Initialize global ServiceManager for webserver access
            crate::services::init_global_service_manager(service_manager).await;

            // 12. Get mutable reference to continue
            let manager_ref = crate::services::get_service_manager()
                .await
                .ok_or("Failed to get ServiceManager reference")?;

            let mut service_manager = {
                let mut guard = manager_ref.write().await;
                guard.take().ok_or("ServiceManager was already taken")?
            };

            // 13. Start all enabled services
            service_manager
                .start_all()
                .await
                .map_err(|e| e.to_string())?;

            // 14. Put it back for webserver access
            {
                let mut guard = manager_ref.write().await;
                *guard = Some(service_manager);
            }

            logger::info(
                LogTag::System,
                "All services started - ScreenerBot is running",
            );
        } // end normal (full) mode
    }

    // 15. Wait for shutdown signal
    shutdown::wait_for_shutdown_signal().await?;

    // 16. Stop all services gracefully
    logger::info(LogTag::System, "Initiating graceful shutdown...");

    let manager_ref = crate::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference for shutdown")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard
            .take()
            .ok_or("ServiceManager was already taken during shutdown")?
    };

    service_manager
        .stop_all()
        .await
        .map_err(|e| e.to_string())?;

    logger::info(LogTag::System, "ScreenerBot shut down successfully");

    Ok(())
}
