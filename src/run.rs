// New simplified run implementation using ServiceManager
// The old implementation is preserved in run_old.rs

use crate::{ global, logger::{ init_file_logging, log, LogTag }, services::ServiceManager };

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
pub async fn run_bot() -> Result<(), String> {
    // 1. Initialize file logging system first
    init_file_logging();

    log(LogTag::System, "INFO", "🚀 ScreenerBot starting up...");

    // 2. Load configuration
    let config = global::read_configs().map_err(|e| format!("Failed to load config: {:?}", e))?;

    log(LogTag::System, "INFO", "Configuration loaded successfully");

    // 3. Create service manager
    let mut service_manager = ServiceManager::new(config).await?;

    log(LogTag::System, "INFO", "Service manager initialized");

    // 4. Register all services
    register_all_services(&mut service_manager);

    // 5. Start all enabled services
    service_manager.start_all().await?;

    log(LogTag::System, "SUCCESS", "✅ All services started - ScreenerBot is running");

    // 6. Wait for shutdown signal
    wait_for_shutdown_signal().await?;

    // 7. Stop all services gracefully
    log(LogTag::System, "INFO", "🛑 Initiating graceful shutdown...");
    service_manager.stop_all().await?;

    log(LogTag::System, "SUCCESS", "✅ ScreenerBot shut down successfully");

    Ok(())
}

/// Register all available services
fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    log(LogTag::System, "INFO", "Registering services...");

    // Register all services (order doesn't matter - manager handles dependencies and priority)
    manager.register(Box::new(EventsService));
    manager.register(Box::new(BlacklistService));
    manager.register(Box::new(WebserverService));
    manager.register(Box::new(TokensService));
    manager.register(Box::new(PositionsService));
    manager.register(Box::new(PoolsService));
    manager.register(Box::new(SecurityService));
    manager.register(Box::new(TransactionsService));
    manager.register(Box::new(WalletService));
    manager.register(Box::new(RpcStatsService));
    manager.register(Box::new(AtaCleanupService));
    manager.register(Box::new(SolPriceService));
    manager.register(Box::new(LearningService));
    manager.register(Box::new(TraderService));

    log(LogTag::System, "INFO", "All services registered");
}

/// Wait for shutdown signal (Ctrl+C)
async fn wait_for_shutdown_signal() -> Result<(), String> {
    log(LogTag::System, "INFO", "Waiting for Ctrl+C to shutdown");

    tokio::signal
        ::ctrl_c().await
        .map_err(|e| format!("Failed to listen for shutdown signal: {}", e))?;

    log(LogTag::System, "INFO", "Shutdown signal received");
    Ok(())
}
