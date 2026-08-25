//! Main run loop — starts the service manager and runs until shutdown signal.
//!
//! Orchestrates bot lifecycle: process lock, configuration, wallet setup,
//! service registration, and graceful shutdown handling.

mod boot;
mod bootstrap;
mod error;
pub mod services;
mod shutdown;

pub use boot::boot;
pub use error::{Error, Result};

use bootstrap::{initialize_ai_runtime_if_enabled, initialize_full_runtime};

use crate::{
    errors::StartupError,
    global,
    logger::{self, LogTag},
    process::lock::ProcessLock,
    process::profiling,
};

/// Main bot execution function — handles the full bot lifecycle with ServiceManager.
pub async fn run_bot() -> Result<()> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    profiling::init_profiling();

    // 1. Ensure all required directories exist (safety backup, already done in main.rs)
    crate::paths::ensure_all_directories().map_err(|error| {
        Error::Startup(StartupError::generic(format!(
            "Failed to create required directories: {error}"
        )))
    })?;

    // 2. Acquire process lock to prevent multiple instances
    let process_lock = ProcessLock::acquire().map_err(StartupError::from)?;

    // Run bot with the acquired lock
    run_bot_internal(process_lock).await
}

/// Internal bot execution with pre-acquired lock.
async fn run_bot_internal(_process_lock: ProcessLock) -> Result<()> {
    logger::info(LogTag::System, "ScreenerBot starting up...");

    // 1. Set GUI mode if --gui flag is present (must be done early for webserver security)
    if crate::arguments::is_gui_enabled() {
        global::set_gui_mode(true);
        logger::info(LogTag::System, "GUI mode enabled");
    }

    // 2. Validate CLI arguments early (before any processing)
    if let Err(e) = crate::arguments::validate_port_argument() {
        return Err(Error::Startup(StartupError::new(
            crate::errors::StartupErrorCode::ConfigInvalid,
            "Invalid startup option",
            e,
            "A command-line option is invalid. Start ScreenerBot without that option, or \
             correct it and try again.",
        )));
    }

    if let Err(e) = crate::arguments::validate_host_argument() {
        return Err(Error::Startup(StartupError::new(
            crate::errors::StartupErrorCode::ConfigInvalid,
            "Invalid startup option",
            e,
            "A command-line option is invalid. Start ScreenerBot without that option, or \
             correct it and try again.",
        )));
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

        // Register every service; only the webserver is enabled before setup.
        services::create_and_start_services("").await?;

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
            crate::config::load_config().map_err(|error| {
                StartupError::new(
                    crate::errors::StartupErrorCode::ConfigInvalid,
                    "Configuration could not be read",
                    format!("Failed to load config: {error}"),
                    "Restore a valid configuration or complete setup again.",
                )
            })?;
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

            services::create_and_start_services("Explore Mode").await?;

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

            // Every service is enabled in full mode.
            services::create_and_start_services("").await?;

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

    services::stop_all_services().await?;

    logger::info(LogTag::System, "ScreenerBot shut down successfully");

    Ok(())
}
