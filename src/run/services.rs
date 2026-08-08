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
    // Observation starts after TransactionsService initializes transactions.db;
    // its consumer loop is already subscribed before this producer comes online.
    manager.register(Box::new(WalletWatchService));
    manager.register(Box::new(CopyTradingService));
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
    // Opt-in and inert until the user sets a referral code in Settings. Its
    // is_enabled() is the consent gate — with no code the task never spawns.
    manager.register(Box::new(ReferralService));
    manager.register(Box::new(AccountService));
    manager.register(Box::new(AtaCleanupService));
    manager.register(Box::new(TraderService::new()));
    manager.register(Box::new(WebserverService));

    // AI service (background auto-blacklisting)
    manager.register(Box::new(AiService::default()));
    manager.register(Box::new(ScheduledAiTasksService::default()));

    // Telegram service (notifications + commands + discovery)
    manager.register(Box::new(TelegramService::new()));

    // Background utility services
    manager.register(Box::new(UpdateCheckService));

    let service_count = 24; // connectivity, events, wallet_watch, copy_trading, transactions, sol_price, pool_discovery,
                            // pool_fetcher, pool_calculator, pool_analyzer, pools, tokens, filtering, ohlcv,
                            // positions, wallet, rpc_stats, ata_cleanup, trader, webserver, ai,
                            // scheduled_ai_tasks, telegram, update_check
    logger::info(
        LogTag::System,
        &format!("All services registered ({service_count} total)"),
    );
}
