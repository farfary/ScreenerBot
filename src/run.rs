// New simplified run implementation using ServiceManager

use crate::{
    arguments::{
        get_profile_duration, is_profile_cpu_enabled, is_profile_tokio_console_enabled,
        is_profile_tracing_enabled,
    },
    global,
    logger::{self, LogTag},
    services::ServiceManager,
};
use solana_sdk::signature::Signer;

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
pub async fn run_bot() -> Result<(), String> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    init_profiling();

    // 1. Ensure all required directories exist (safety backup, already done in main.rs)
    crate::paths::ensure_all_directories()
        .map_err(|e| format!("Failed to create required directories: {}", e))?;

    // 2. Acquire process lock to prevent multiple instances
    let _process_lock = crate::process_lock::ProcessLock::acquire()?;

    logger::info(LogTag::System, "🚀 ScreenerBot starting up...");

    // 3. Check if config.toml exists (determines initialization mode)
    let config_path = crate::paths::get_config_path();
    let config_exists = config_path.exists();

    if !config_exists {
        logger::info(
            LogTag::System,
            "📋 No config.toml found - starting in initialization mode",
        );
        logger::info(
            LogTag::System,
            "🌐 Webserver will start on http://localhost:8080 for initial setup",
        );

        // Set initialization flag to false (services will be gated)
        global::INITIALIZATION_COMPLETE.store(false, std::sync::atomic::Ordering::SeqCst);

        // Create service manager with only webserver enabled
        let mut service_manager = ServiceManager::new().await?;
        logger::info(LogTag::System, "Service manager initialized");

        // Register all services (but only webserver will be enabled)
        register_all_services(&mut service_manager);

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
        service_manager.start_all().await?;

        // Put it back for webserver access
        {
            let mut guard = manager_ref.write().await;
            *guard = Some(service_manager);
        }

        logger::info(
            LogTag::System,
            "✅ Webserver started - complete initialization at http://localhost:8080",
        );
        logger::info(
            LogTag::System,
            "⏳ Waiting for initialization to complete...",
        );

        // Wait for initialization to complete or shutdown signal
        wait_for_initialization_or_shutdown().await?;

        logger::info(
            LogTag::System,
            "✅ Initialization complete - all services running",
        );
    } else {
        logger::info(
            LogTag::System,
            "📋 Config.toml found - starting in normal mode",
        );

        // 4. Load configuration
        crate::config::load_config().map_err(|e| format!("Failed to load config: {}", e))?;

        logger::info(LogTag::System, "Configuration loaded successfully");

        // 5. Verify license
        logger::info(LogTag::System, "🔐 Verifying ScreenerBot license...");

        let wallet_keypair = crate::config::utils::get_wallet_keypair()
            .map_err(|e| format!("Failed to get wallet keypair: {}", e))?;
        let wallet_pubkey = wallet_keypair.pubkey();

        let license_status = crate::license::verify_license_for_wallet(&wallet_pubkey)
            .await
            .map_err(|e| format!("License verification failed: {}", e))?;

        if !license_status.valid {
            let reason = license_status.reason.as_deref().unwrap_or("Unknown reason");
            logger::error(
                LogTag::System,
                &format!(
                    "❌ Invalid license for wallet {}: {}",
                    wallet_pubkey, reason
                ),
            );
            return Err(format!(
                "Cannot start bot: Invalid license ({}). Visit https://screenerbot.com to purchase or renew your license.",
                reason
            ));
        }

        logger::info(
            LogTag::System,
            &format!(
                "✅ License verified successfully: tier={}, expiry={}",
                license_status.tier.as_deref().unwrap_or("Unknown"),
                license_status
                    .expiry_ts
                    .map(|ts| {
                        use chrono::{DateTime, TimeZone, Utc};
                        let dt = Utc
                            .timestamp_opt(ts as i64, 0)
                            .single()
                            .map(|dt| dt.format("%Y-%m-%d").to_string())
                            .unwrap_or_else(|| "N/A".to_string());
                        dt
                    })
                    .unwrap_or_else(|| "N/A".to_string())
            ),
        );

        // 6. Validate wallet consistency
        logger::info(LogTag::System, "🔍 Validating wallet consistency...");

        match crate::wallet_validation::WalletValidator::validate_wallet_consistency().await? {
            crate::wallet_validation::WalletValidationResult::Valid => {
                logger::info(LogTag::System, "✅ Wallet validation passed");
            }
            crate::wallet_validation::WalletValidationResult::FirstRun => {
                logger::info(LogTag::System, "✅ First run - no existing data");
            }
            crate::wallet_validation::WalletValidationResult::Mismatch {
                current,
                stored,
                affected_systems,
            } => {
                logger::error(
                    LogTag::System,
                    &format!(
                        "❌ WALLET MISMATCH DETECTED!\n\
                         \n\
                         Current wallet: {}\n\
                         Stored wallet:  {}\n\
                         Affected systems: {}\n\
                         \n\
                         ⚠️  You MUST clean existing data before starting with a new wallet.\n\
                         Run: cargo run --bin screenerbot -- --clean-wallet-data\n\
                         Or manually delete: data/transactions.db data/positions.db data/wallet.db",
                        current,
                        stored,
                        affected_systems.join(", ")
                    ),
                );

                return Err(format!(
                    "Wallet mismatch detected - current wallet {} does not match stored wallet {}. Clean data before proceeding.",
                    current, stored
                ));
            }
        }

        // Set initialization flag to true (all services enabled)
        global::INITIALIZATION_COMPLETE.store(true, std::sync::atomic::Ordering::SeqCst);

        // 7. Initialize strategy system
        crate::strategies::init_strategy_system(crate::strategies::engine::EngineConfig::default())
            .await
            .map_err(|e| format!("Failed to initialize strategy system: {}", e))?;

        logger::info(LogTag::System, "Strategy system initialized successfully");

        // 8. Create service manager
        let mut service_manager = ServiceManager::new().await?;

        logger::info(LogTag::System, "Service manager initialized");

        // 9. Register all services
        register_all_services(&mut service_manager);

        // 10. Initialize global ServiceManager for webserver access
        crate::services::init_global_service_manager(service_manager).await;

        // 11. Get mutable reference to continue
        let manager_ref = crate::services::get_service_manager()
            .await
            .ok_or("Failed to get ServiceManager reference")?;

        let mut service_manager = {
            let mut guard = manager_ref.write().await;
            guard.take().ok_or("ServiceManager was already taken")?
        };

        // 12. Start all enabled services
        service_manager.start_all().await?;

        // 13. Put it back for webserver access
        {
            let mut guard = manager_ref.write().await;
            *guard = Some(service_manager);
        }

        logger::info(
            LogTag::System,
            "✅ All services started - ScreenerBot is running",
        );
    }

    // 14. Wait for shutdown signal
    wait_for_shutdown_signal().await?;

    // 15. Stop all services gracefully
    logger::info(LogTag::System, "🛑 Initiating graceful shutdown...");

    let manager_ref = crate::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference for shutdown")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard
            .take()
            .ok_or("ServiceManager was already taken during shutdown")?
    };

    service_manager.stop_all().await?;

    logger::info(LogTag::System, "✅ ScreenerBot shut down successfully");

    Ok(())
}

/// Register all available services
fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    logger::info(LogTag::System, "Registering services...");

    // Register all services (order doesn't matter - manager handles dependencies and priority)

    // Core infrastructure services
    manager.register(Box::new(crate::connectivity::ConnectivityService::new())); // Priority 5 - Foundation service
    manager.register(Box::new(EventsService));
    manager.register(Box::new(TransactionsService));
    manager.register(Box::new(SolPriceService));

    // Pool services (4 sub-services + 1 helper coordinator)
    manager.register(Box::new(PoolDiscoveryService)); // 100
    manager.register(Box::new(PoolFetcherService)); // 101
    manager.register(Box::new(PoolCalculatorService)); // 102
    manager.register(Box::new(PoolAnalyzerService)); // 103
    manager.register(Box::new(PoolsService)); // 35 - helper tasks (health, cleanup)

    // Centralized Tokens service (replaces token discovery/monitoring/security/blacklist services)
    manager.register(Box::new(TokensService::default()));

    // Other application services
    manager.register(Box::new(FilteringService::new()));
    manager.register(Box::new(OhlcvService));
    manager.register(Box::new(PositionsService));
    manager.register(Box::new(WalletService));
    manager.register(Box::new(RpcStatsService));
    manager.register(Box::new(AtaCleanupService));
    manager.register(Box::new(TraderService));
    manager.register(Box::new(WebserverService));

    logger::info(
        LogTag::System,
        "All services registered (21 total - includes ConnectivityService)",
    );
}

/// Wait for shutdown signal (Ctrl+C)
async fn wait_for_shutdown_signal() -> Result<(), String> {
    logger::info(
        LogTag::System,
        "Waiting for Ctrl+C (press twice to force kill)",
    );

    // First Ctrl+C triggers graceful shutdown
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| format!("Failed to listen for shutdown signal: {}", e))?;

    logger::warning(
        LogTag::System,
        "Shutdown signal received. Press Ctrl+C again to force kill.",
    );

    // Spawn a background listener for a second Ctrl+C to exit immediately
    tokio::spawn(async move {
        // If another Ctrl+C is received during graceful shutdown, exit immediately
        if tokio::signal::ctrl_c().await.is_ok() {
            logger::error(
                LogTag::System,
                "Second Ctrl+C detected — forcing immediate exit.",
            );
            // 130 is the conventional exit code for SIGINT
            std::process::exit(130);
        }
    });

    Ok(())
}

/// Wait for initialization to complete or shutdown signal during pre-init mode
async fn wait_for_initialization_or_shutdown() -> Result<(), String> {
    use tokio::time::{sleep, Duration, Instant};

    const MAX_WAIT_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes
    const WARNING_INTERVAL: Duration = Duration::from_secs(5 * 60); // Warn every 5 minutes

    let start = Instant::now();
    let mut last_warning = start;

    loop {
        // Check if initialization is complete
        if global::is_initialization_complete() {
            logger::info(
                LogTag::System,
                "✅ Initialization complete - services started successfully",
            );
            return Ok(());
        }

        // Check elapsed time
        let elapsed = start.elapsed();
        if elapsed >= MAX_WAIT_DURATION {
            logger::error(
                LogTag::System,
                &format!(
                    "⏱️ Initialization timeout after {} minutes - initialization never completed",
                    MAX_WAIT_DURATION.as_secs() / 60
                ),
            );
            return Err(format!(
                "Initialization timeout after {} minutes",
                MAX_WAIT_DURATION.as_secs() / 60
            ));
        }

        // Periodic warning logs
        if elapsed - (last_warning - start) >= WARNING_INTERVAL {
            logger::warning(
                LogTag::System,
                &format!(
                    "⏳ Still waiting for initialization... ({} minutes elapsed)",
                    elapsed.as_secs() / 60
                ),
            );
            last_warning = Instant::now();
        }

        // Check for Ctrl+C (non-blocking)
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                logger::warning(
                    LogTag::System,
                    "Shutdown signal received during initialization",
                );
                return Err("Shutdown during initialization".to_string());
            }
            _ = sleep(Duration::from_millis(500)) => {
                // Continue polling
            }
        }
    }
}

/// Initialize CPU profiling based on command-line flags
fn init_profiling() {
    // Tokio console profiling (async task inspector)
    #[cfg(feature = "console")]
    if is_profile_tokio_console_enabled() {
        console_subscriber::init();
        crate::logger::info(
            crate::logger::LogTag::System,
            &"🔍 Tokio console enabled - connect with: tokio-console".to_string(),
        );
        crate::logger::info(
            crate::logger::LogTag::System,
            &"   Install: cargo install tokio-console".to_string(),
        );
        crate::logger::info(
            crate::logger::LogTag::System,
            &"   Connect: tokio-console".to_string(),
        );
        return;
    }

    // Tracing-based profiling
    if is_profile_tracing_enabled() {
        use tracing_subscriber::{fmt, EnvFilter};

        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
            )
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(true)
            .with_line_number(true)
            .init();

        crate::logger::info(
            crate::logger::LogTag::System,
            &"🔍 Tracing profiling enabled".to_string(),
        );
        crate::logger::info(
            crate::logger::LogTag::System,
            &"   View detailed traces with thread IDs and timing".to_string(),
        );
        return;
    }

    // CPU profiling with pprof (will generate flamegraph on exit)
    #[cfg(feature = "flamegraph")]
    if is_profile_cpu_enabled() {
        let duration = get_profile_duration();
        crate::logger::info(
            crate::logger::LogTag::System,
            &"🔥 CPU profiling enabled with pprof".to_string(),
        );
        crate::logger::info(
            crate::logger::LogTag::System,
            &format!("   Duration: {} seconds", duration),
        );
        crate::logger::info(
            crate::logger::LogTag::System,
            &"   Flamegraph will be generated on exit".to_string(),
        );
        crate::logger::info(
            crate::logger::LogTag::System,
            &"   Press Ctrl+C to stop and generate flamegraph".to_string(),
        );

        // Note: pprof profiling is initialized later in the async context
        // This is just a notification
        return;
    }
}

/// Start CPU profiling guard (pprof-based)
/// Returns a guard that will generate flamegraph when dropped
#[cfg(feature = "flamegraph")]
pub fn start_cpu_profiling() -> Option<pprof::ProfilerGuard<'static>> {
    if !is_profile_cpu_enabled() {
        return None;
    }

    match pprof::ProfilerGuardBuilder::default()
        .frequency(997) // Sample at ~1000 Hz
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
    {
        Ok(guard) => {
            logger::info(LogTag::System, "🔥 CPU profiling started (pprof)");
            Some(guard)
        }
        Err(e) => {
            logger::error(
                LogTag::System,
                &format!("Failed to start CPU profiling: {}", e),
            );
            None
        }
    }
}

#[cfg(not(feature = "flamegraph"))]
pub fn start_cpu_profiling() -> Option<()> {
    None
}
