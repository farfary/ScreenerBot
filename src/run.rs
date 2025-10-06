// New simplified run implementation using ServiceManager
// The old implementation is preserved in run_old.rs

use crate::{
    global,
    logger::{init_file_logging, log, LogTag},
    services::ServiceManager,
};

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
pub async fn run_bot() -> Result<(), String> {
    // 1. Initialize file logging system first
    init_file_logging();

    log(LogTag::System, "INFO", "🚀 ScreenerBot starting up...");

    // 2. Load configuration
    crate::config::load_config().map_err(|e| format!("Failed to load config: {}", e))?;

    log(LogTag::System, "INFO", "Configuration loaded successfully");

    // 3. Create service manager
    let mut service_manager = ServiceManager::new().await?;

    log(LogTag::System, "INFO", "Service manager initialized");

    // 4. Register all services
    register_all_services(&mut service_manager);

    // 5. Initialize global ServiceManager for webserver access
    crate::services::init_global_service_manager(service_manager).await;

    // 6. Get mutable reference to continue
    let manager_ref = crate::services::get_service_manager()
        .await
        .ok_or("Failed to get ServiceManager reference")?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard.take().ok_or("ServiceManager was already taken")?
    };

    // 7. Start all enabled services
    service_manager.start_all().await?;

    // 8. Put it back for webserver access
    {
        let mut guard = manager_ref.write().await;
        *guard = Some(service_manager);
    }

    log(
        LogTag::System,
        "SUCCESS",
        "✅ All services started - ScreenerBot is running",
    );

    // 6. Wait for shutdown signal
    wait_for_shutdown_signal().await?;

    // 9. Stop all services gracefully
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
    manager.register(Box::new(BlacklistService));
    manager.register(Box::new(SolPriceService));

    // Pool services (4 sub-services + 1 helper coordinator)
    manager.register(Box::new(PoolDiscoveryService)); // 31
    manager.register(Box::new(PoolFetcherService)); // 32
    manager.register(Box::new(PoolCalculatorService)); // 33
    manager.register(Box::new(PoolAnalyzerService)); // 34
    manager.register(Box::new(PoolsService)); // 35 - helper tasks (health, cleanup)

    // Token services (2 sub-services, no empty coordinator)
    manager.register(Box::new(TokenDiscoveryService)); // 41 - includes initialization
    manager.register(Box::new(TokenMonitoringService)); // 42

    // Other application services
    manager.register(Box::new(SecurityService));
    manager.register(Box::new(OhlcvService));
    manager.register(Box::new(PositionsService));
    manager.register(Box::new(WalletService));
    manager.register(Box::new(RpcStatsService));
    manager.register(Box::new(AtaCleanupService));
    manager.register(Box::new(LearningService));
    manager.register(Box::new(SummaryService));
    manager.register(Box::new(TraderService));
    manager.register(Box::new(WebserverService));

    log(
        LogTag::System,
        "INFO",
        "All services registered (22 total - removed empty TokensService)",
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
