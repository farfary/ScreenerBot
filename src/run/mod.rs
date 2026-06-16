//! Main run loop — starts the service manager and runs until shutdown signal.
//!
//! Orchestrates bot lifecycle: process lock, configuration, wallet setup,
//! service registration, and graceful shutdown handling.

mod llm_init;
mod services;
mod shutdown;

use crate::{
    errors::StartupError,
    global,
    logger::{self, LogTag},
    process_lock::ProcessLock,
    profiling,
    services::ServiceManager,
};
use solana_sdk::signature::Signer;

/// Main bot execution function — handles the full bot lifecycle with ServiceManager.
///
/// Acquires process lock and runs the bot. For GUI mode, use `run_bot_with_lock()` instead.
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

/// Run bot with a pre-acquired process lock.
///
/// Used by Electron GUI mode which acquires the lock before starting to ensure
/// the window doesn't open if another instance is running.
pub async fn run_bot_with_lock(process_lock: ProcessLock) -> Result<(), StartupError> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    profiling::init_profiling();

    // 1. Ensure all required directories exist (safety backup, already done in main.rs)
    crate::paths::ensure_all_directories()
        .map_err(|e| format!("Failed to create required directories: {e}"))?;

    // Lock already acquired, run bot directly
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
        logger::info(LogTag::System, "Waiting for initialization to complete...");

        // Wait for initialization to complete or shutdown signal
        shutdown::wait_for_initialization_or_shutdown().await?;

        logger::info(
            LogTag::System,
            "Initialization complete - all services running",
        );
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

        // 4b. Detect preview mode: the user skipped wallet + RPC setup at first
        // run. The durable marker is `setup_skipped`; an empty encrypted wallet is a
        // safety fallback (treat as preview rather than hard-failing later).
        let preview = crate::config::with_config(|cfg| {
            cfg.gui.dashboard.startup.setup_skipped || cfg.wallet_encrypted.trim().is_empty()
        });

        if preview {
            logger::info(
                LogTag::System,
                "preview mode: wallet + RPC not configured - starting preview tier only",
            );

            // preview mode: only the preview tier runs. Wallet/RPC-dependent
            // services stay disabled and are filtered out of the startup order.
            global::set_preview_mode(true);
            global::INITIALIZATION_COMPLETE.store(false, std::sync::atomic::Ordering::SeqCst);

            let mut service_manager = ServiceManager::new().await.map_err(|e| e.to_string())?;
            logger::info(LogTag::System, "Service manager initialized (preview)");

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
                "preview mode active - complete wallet + RPC setup in the dashboard to enable trading",
            );
        } else {
            // 5. Initialize wallets module (migrates from config.toml if needed)
            crate::wallets::initialize()
                .await
                .map_err(|e| format!("Failed to initialize wallets: {e}"))?;

            logger::info(LogTag::System, "Wallets module initialized");

            // 6. Validate wallet consistency
            logger::info(LogTag::System, "Validating wallet consistency...");

            match crate::wallet_validation::WalletValidator::validate_wallet_consistency().await? {
                crate::wallet_validation::WalletValidationResult::Valid => {
                    logger::info(LogTag::System, "Wallet validation passed");
                }
                crate::wallet_validation::WalletValidationResult::FirstRun => {
                    logger::info(LogTag::System, "First run - no existing data");
                }
                crate::wallet_validation::WalletValidationResult::Mismatch {
                    current,
                    stored,
                    affected_systems,
                } => {
                    // Surfaced uniformly to terminal/log and the GUI via the
                    // StartupError emitted at the process boundary (main.rs).
                    // Remedy text is binary-user-appropriate (no `cargo`) and a
                    // safe one-click recovery is attached for the GUI.
                    return Err(StartupError::wallet_mismatch(
                        &current,
                        &stored,
                        &affected_systems,
                    ));
                }
            }

            // Set initialization flag to true (all services enabled)
            global::INITIALIZATION_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);

            // 7. Initialize strategy system
            crate::strategies::init_strategy_system(
                crate::strategies::engine::EngineConfig::default(),
            )
            .await
            .map_err(|e| format!("Failed to initialize strategy system: {e}"))?;

            logger::info(LogTag::System, "Strategy system initialized successfully");

            // 8. Initialize actions database
            crate::actions::init_database()
                .await
                .map_err(|e| format!("Failed to initialize actions database: {e}"))?;

            logger::info(LogTag::System, "Actions database initialized successfully");

            // Sync recent incomplete actions from database to memory
            crate::actions::sync_from_db()
                .await
                .map_err(|e| format!("Failed to sync actions from database: {e}"))?;

            // Start periodic cleanup of old completed actions (30-day retention)
            crate::actions::spawn_cleanup_task();

            // Start database maintenance (auto-vacuum migration + periodic incremental vacuum)
            tokio::spawn(crate::database::start_db_maintenance_task());

            // 8.5. Initialize AI database (always) and AI engine (if enabled)
            // Database is always initialized so dashboard can view/edit instructions
            if let Err(e) = crate::ai::init_ai_database() {
                logger::warning(
                LogTag::System,
                &format!(
                    "Failed to initialize AI database: {} - AI instructions and history will not be available",
                    e
                ),
            );
            }

            // Initialize AI chat database (always, for chat history persistence)
            if let Err(e) = crate::ai::init_chat_db() {
                logger::warning(
                LogTag::System,
                &format!(
                    "Failed to initialize AI chat database: {} - Chat history will not be available",
                    e
                ),
            );
            }

            // Initialize AI engine only if enabled
            let ai_enabled = crate::config::with_config(|cfg| cfg.ai.enabled);
            if ai_enabled {
                logger::info(LogTag::System, "Initializing AI engine...");

                crate::ai::init_ai_engine()
                    .await
                    .map_err(|e| format!("Failed to initialize AI engine: {e}"))?;
                logger::info(LogTag::System, "AI engine initialized successfully");

                // Initialize AI chat engine
                crate::ai::init_chat_engine()
                    .await
                    .map_err(|e| format!("Failed to initialize AI chat engine: {e}"))?;
                logger::info(LogTag::System, "AI chat engine initialized successfully");

                // Initialize LLM manager with configured providers
                llm_init::initialize_llm_providers().await?;
            }

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
