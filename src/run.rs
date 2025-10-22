// New simplified run implementation using ServiceManager
// The old implementation is preserved in run_old.rs

use crate::{
    arguments::{
        get_profile_duration, is_profile_cpu_enabled, is_profile_tokio_console_enabled,
        is_profile_tracing_enabled,
    },
    global,
    logger::{init_file_logging, log, LogTag},
    services::ServiceManager,
};

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
pub async fn run_bot() -> Result<(), String> {
    // 0. Initialize profiling if requested (must be done before any tokio tasks)
    init_profiling();

    // 1. Initialize file logging system first
    init_file_logging();

    log(LogTag::System, "INFO", "🚀 ScreenerBot starting up...");

    // 2. Load configuration
    crate::config::load_config().map_err(|e| format!("Failed to load config: {}", e))?;

    log(LogTag::System, "INFO", "Configuration loaded successfully");

    // 3. Initialize strategy system
    crate::strategies::init_strategy_system(crate::strategies::engine::EngineConfig::default())
        .await
        .map_err(|e| format!("Failed to initialize strategy system: {}", e))?;

    log(
        LogTag::System,
        "INFO",
        "Strategy system initialized successfully"
    );

    // 4. Create service manager
    let mut service_manager = ServiceManager::new().await?;

    log(LogTag::System, "INFO", "Service manager initialized");

    // 5. Register all services
    register_all_services(&mut service_manager);

    // 6. Initialize global ServiceManager for webserver access
    crate::services::init_global_service_manager(service_manager).await;

    // 7. Get mutable reference to continue
    let manager_ref = crate::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard.take().ok_or("ServiceManager was already taken")?
    };

    // 8. Start all enabled services
    service_manager.start_all().await?;

    // 9. Put it back for webserver access
    {
        let mut guard = manager_ref.write().await;
        *guard = Some(service_manager);
    }

    log(
        LogTag::System,
        "SUCCESS",
        "✅ All services started - ScreenerBot is running",
    );

    // 10. Wait for shutdown signal
    wait_for_shutdown_signal().await?;

    // 11. Stop all services gracefully
    log(LogTag::System, "INFO", "🛑 Initiating graceful shutdown...");

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

    log(
        LogTag::System,
        "SUCCESS",
        "✅ ScreenerBot shut down successfully",
    );

    Ok(())
}

/// Register all available services
fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    log(LogTag::System, "INFO", "Registering services...");

    // Register all services (order doesn't matter - manager handles dependencies and priority)

    // Core infrastructure services
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

    log(
        LogTag::System,
        "INFO",
        "All services registered (20 total - centralized TokensService)",
    );
}

/// Wait for shutdown signal (Ctrl+C)
async fn wait_for_shutdown_signal() -> Result<(), String> {
    log(
        LogTag::System,
        "INFO",
        "Waiting for Ctrl+C (press twice to force kill)",
    );

    // First Ctrl+C triggers graceful shutdown
    tokio::signal::ctrl_c()
        .await
        .map_err(|e| format!("Failed to listen for shutdown signal: {}", e))?;

    log(
        LogTag::System,
        "WARN",
        "Shutdown signal received. Press Ctrl+C again to force kill.",
    );

    // Spawn a background listener for a second Ctrl+C to exit immediately
    tokio::spawn(async move {
        // If another Ctrl+C is received during graceful shutdown, exit immediately
        if tokio::signal::ctrl_c().await.is_ok() {
            log(
                LogTag::System,
                "ERROR",
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
        eprintln!("🔍 Tokio console enabled - connect with: tokio-console");
        eprintln!("   Install: cargo install tokio-console");
        eprintln!("   Connect: tokio-console");
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

        eprintln!("🔍 Tracing profiling enabled");
        eprintln!("   View detailed traces with thread IDs and timing");
        return;
    }

    // CPU profiling with pprof (will generate flamegraph on exit)
    #[cfg(feature = "flamegraph")]
    if is_profile_cpu_enabled() {
        let duration = get_profile_duration();
        eprintln!("🔥 CPU profiling enabled with pprof");
        eprintln!("   Duration: {} seconds", duration);
        eprintln!("   Flamegraph will be generated on exit");
        eprintln!("   Press Ctrl+C to stop and generate flamegraph");

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
            log(LogTag::System, "INFO", "🔥 CPU profiling started (pprof)");
            Some(guard)
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
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
