//! Service registration — registers all available services with the service manager.

use crate::{
    logger::{self, LogTag},
    run::{Error, Result},
    services::ServiceManager,
};

/// Register all available services with the service manager.
pub fn register_all_services(manager: &mut ServiceManager) {
    use crate::services::implementations::*;

    logger::info(LogTag::System, "Registering services...");

    // Select the Solana swap router set before any service can trigger a swap.
    crate::swaps::registry::set_router_factory(
        crate::chains::solana::swaps::routers::build_routers,
    );

    // Select the Solana wallet-watch runtime before WalletWatchService can start.
    crate::wallets::watch::runtime::set_runtime_factory(
        crate::chains::solana::wallets::runtime::build_runtime,
    );

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
    manager.register(Box::new(LlmAnalysisService::default()));
    manager.register(Box::new(AssistantScheduledTasksService::default()));

    // Telegram service (notifications + commands + discovery)
    manager.register(Box::new(TelegramService::new()));

    // Background utility services
    manager.register(Box::new(UpdateCheckService));

    logger::info(
        LogTag::System,
        &format!(
            "All services registered ({} total)",
            manager.registered_count()
        ),
    );
}

/// Create the service manager, register every service, publish it globally,
/// and start the enabled ones. `mode_label` only annotates the log line.
pub(super) async fn create_and_start_services(mode_label: &str) -> Result<()> {
    let mut service_manager = crate::services::ServiceManager::new()
        .await
        .map_err(|source| Error::Core {
            source: Box::new(source),
        })?;

    if mode_label.is_empty() {
        logger::info(LogTag::System, "Service manager initialized");
    } else {
        logger::info(
            LogTag::System,
            &format!("Service manager initialized ({mode_label})"),
        );
    }

    register_all_services(&mut service_manager);

    crate::services::init_global_service_manager(service_manager).await;

    let manager_ref =
        crate::services::get_service_manager()
            .await
            .ok_or(Error::ServiceManagerUnavailable {
                operation: "start services",
            })?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard.take().ok_or(Error::ServiceManagerTaken {
            operation: "start services",
        })?
    };

    service_manager
        .start_all()
        .await
        .map_err(|source| Error::Core {
            source: Box::new(source),
        })?;

    {
        let mut guard = manager_ref.write().await;
        *guard = Some(service_manager);
    }

    Ok(())
}

/// Take the global service manager back and stop every running service.
pub(super) async fn stop_all_services() -> Result<()> {
    let manager_ref =
        crate::services::get_service_manager()
            .await
            .ok_or(Error::ServiceManagerUnavailable {
                operation: "stop services",
            })?;

    let mut service_manager = {
        let mut guard = manager_ref.write().await;
        guard.take().ok_or(Error::ServiceManagerTaken {
            operation: "stop services",
        })?
    };

    service_manager
        .stop_all()
        .await
        .map_err(|source| Error::Core {
            source: Box::new(source),
        })?;

    Ok(())
}
