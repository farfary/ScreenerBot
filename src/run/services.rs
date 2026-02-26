//! Service registration — registers all available services with the service manager.

use crate::{
    logger::{self, LogTag},
    services::ServiceManager,
};

/// Register all available services with the service manager.
pub(super) fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    logger::info(LogTag::System, "Registering services...");

    // Core infrastructure services
    manager.register(Box::new(ConnectivityService::new()));
    manager.register(Box::new(EventsService));
    manager.register(Box::new(TransactionsService));
    manager.register(Box::new(SolPriceService));

    // Pool services (4 sub-services + 1 helper coordinator)
    manager.register(Box::new(PoolDiscoveryService));
    manager.register(Box::new(PoolFetcherService));
    manager.register(Box::new(PoolCalculatorService));
    manager.register(Box::new(PoolAnalyzerService));
    manager.register(Box::new(PoolsService));

    // Centralized Tokens service
    manager.register(Box::new(TokensService::default()));

    // Application services
    manager.register(Box::new(FilteringService::new()));
    manager.register(Box::new(OhlcvService));
    manager.register(Box::new(PositionsService));
    manager.register(Box::new(WalletService));
    manager.register(Box::new(RpcStatsService));
    manager.register(Box::new(AtaCleanupService));
    manager.register(Box::new(TraderService::new()));
    manager.register(Box::new(WebserverService));

    // AI service (background auto-blacklisting)
    manager.register(Box::new(AiService::default()));

    // Telegram service (notifications + commands + discovery)
    manager.register(Box::new(TelegramService::new()));

    // Background utility services
    manager.register(Box::new(UpdateCheckService));

    let service_count = 21; // connectivity, events, transactions, sol_price, pool_discovery, pool_fetcher,
                            // pool_calculator, pool_analyzer, pools, tokens, filtering, ohlcv,
                            // positions, wallet, rpc_stats, ata_cleanup, trader, webserver, ai, telegram, update_check
    logger::info(
        LogTag::System,
        &format!("All services registered ({service_count} total)"),
    );
}
