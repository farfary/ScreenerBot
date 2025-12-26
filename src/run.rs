// New simplified run implementation using ServiceManager

use crate::{
  global,
  logger::{self, LogTag},
  process_lock::ProcessLock,
  profiling,
  services::ServiceManager,
};
use solana_sdk::signature::Signer;

/// Main bot execution function - handles the full bot lifecycle with ServiceManager
///
/// Acquires process lock and runs the bot. For GUI mode, use `run_bot_with_lock()` instead.
pub async fn run_bot() -> Result<(), String> {
  // 0. Initialize profiling if requested (must be done before any tokio tasks)
  profiling::init_profiling();

  // 1. Ensure all required directories exist (safety backup, already done in main.rs)
  crate::paths::ensure_all_directories()
    .map_err(|e| format!("Failed to create required directories: {}", e))?;

  // 2. Acquire process lock to prevent multiple instances
  let process_lock = ProcessLock::acquire()?;

  // Run bot with the acquired lock
  run_bot_internal(process_lock).await
}

/// Run bot with a pre-acquired process lock
///
/// Used by Electron GUI mode which acquires the lock before starting to ensure
/// the window doesn't open if another instance is running.
pub async fn run_bot_with_lock(process_lock: ProcessLock) -> Result<(), String> {
  // 0. Initialize profiling if requested (must be done before any tokio tasks)
  profiling::init_profiling();

  // 1. Ensure all required directories exist (safety backup, already done in main.rs)
  crate::paths::ensure_all_directories()
    .map_err(|e| format!("Failed to create required directories: {}", e))?;

  // Lock already acquired, run bot directly
  run_bot_internal(process_lock).await
}

/// Internal bot execution with pre-acquired lock
async fn run_bot_internal(_process_lock: ProcessLock) -> Result<(), String> {
 logger::info(LogTag::System, "ScreenerBot starting up...");

  // 3. Check if config.toml exists (determines initialization mode)
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
 "Webserver started - complete initialization at http://localhost:8080",
    );
    logger::info(
      LogTag::System,
 "Waiting for initialization to complete...",
    );

    // Wait for initialization to complete or shutdown signal
    wait_for_initialization_or_shutdown().await?;

    logger::info(
      LogTag::System,
 "Initialization complete - all services running",
    );
  } else {
    logger::info(
      LogTag::System,
 "Config.toml found - starting in normal mode",
    );

    // 4. Load configuration
    crate::config::load_config().map_err(|e| format!("Failed to load config: {}", e))?;

    logger::info(LogTag::System, "Configuration loaded successfully");

    // 5. Initialize wallets module (migrates from config.toml if needed)
    crate::wallets::initialize()
      .await
      .map_err(|e| format!("Failed to initialize wallets: {}", e))?;

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
        logger::error(
          LogTag::System,
          &format!(
 "WALLET MISMATCH DETECTED!\n\
             \n\
             Current wallet: {}\n\
             Stored wallet: {}\n\
             Affected systems: {}\n\
             \n\
              You MUST clean existing data before starting with a new wallet.\n\
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

    // 8. Initialize actions database
    crate::actions::init_database()
      .await
      .map_err(|e| format!("Failed to initialize actions database: {}", e))?;

    logger::info(LogTag::System, "Actions database initialized successfully");

    // Sync recent incomplete actions from database to memory
    crate::actions::sync_from_db()
      .await
      .map_err(|e| format!("Failed to sync actions from database: {}", e))?;

    // 9. Create service manager
    let mut service_manager = ServiceManager::new().await?;

    logger::info(LogTag::System, "Service manager initialized");

    // 10. Register all services
    register_all_services(&mut service_manager);

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
    service_manager.start_all().await?;

    // 14. Put it back for webserver access
    {
      let mut guard = manager_ref.write().await;
      *guard = Some(service_manager);
    }

    logger::info(
      LogTag::System,
 "All services started - ScreenerBot is running",
    );
  }

  // 15. Wait for shutdown signal
  wait_for_shutdown_signal().await?;

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

  service_manager.stop_all().await?;

 logger::info(LogTag::System, "ScreenerBot shut down successfully");

  Ok(())
}

/// Register all available services
fn register_all_services(manager: &mut ServiceManager) {
  use crate::services::implementations::*;

  logger::info(LogTag::System, "Registering services...");

  // Core infrastructure services
  manager.register(Box::new(crate::connectivity::ConnectivityService::new()));
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
  manager.register(Box::new(crate::trader::TraderService::new()));
  manager.register(Box::new(WebserverService));

  // Notification service (Telegram integration)
  manager.register(Box::new(NotificationService));

  // Background utility services
  manager.register(Box::new(UpdateCheckService));

  let service_count = 21; // connectivity, events, transactions, sol_price, pool_discovery, pool_fetcher,
                           // pool_calculator, pool_analyzer, pool_helpers, tokens, filtering, ohlcv,
                           // positions, wallet, rpc_stats, ata_cleanup, trader, webserver, notifications, update_check
  logger::info(
    LogTag::System,
    &format!("All services registered ({} total)", service_count),
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
 "Initialization complete - services started successfully",
      );
      return Ok(());
    }

    // Check elapsed time
    let elapsed = start.elapsed();
    if elapsed >= MAX_WAIT_DURATION {
      logger::error(
        LogTag::System,
        &format!(
 "Initialization timeout after {} minutes - initialization never completed",
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
 "Still waiting for initialization... ({} minutes elapsed)",
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
