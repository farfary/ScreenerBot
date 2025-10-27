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

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
pub async fn run_bot() -> Result<(), String> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    init_profiling();

    // 1. Acquire process lock to prevent multiple instances
    let _process_lock = crate::process_lock::ProcessLock::acquire()?;

    logger::info(LogTag::System, "🚀 ScreenerBot starting up...");

    // 3. Load configuration
    crate::config::load_config().map_err(|e| format!("Failed to load config: {}", e))?;

    logger::info(LogTag::System, "Configuration loaded successfully");

    // 4. Initialize strategy system
    crate::strategies::init_strategy_system(crate::strategies::engine::EngineConfig::default())
        .await
        .map_err(|e| format!("Failed to initialize strategy system: {}", e))?;

    logger::info(LogTag::System, "Strategy system initialized successfully");

    // 5. Create service manager
    let mut service_manager = ServiceManager::new().await?;

    logger::info(LogTag::System, "Service manager initialized");

    // 6. Register all services
    register_all_services(&mut service_manager);

    // 7. Initialize global ServiceManager for webserver access
    crate::services::init_global_service_manager(service_manager).await;

    // 8. Get mutable reference to continue
    let manager_ref = crate::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard.take().ok_or("ServiceManager was already taken")?
    };

    // 9. Start all enabled services
    service_manager.start_all().await?;

    // 10. Put it back for webserver access
    {
        let mut guard = manager_ref.write().await;
        *guard = Some(service_manager);
    }

    logger::info(
        LogTag::System,
        "✅ All services started - ScreenerBot is running",
    );

    // 11. Wait for shutdown signal
    wait_for_shutdown_signal().await?;

    // 12. Stop all services gracefully
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
