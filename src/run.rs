use crate::{
    arguments::{
        is_dashboard_enabled,
        is_debug_system_enabled,
        is_dry_run_enabled,
        is_run_enabled,
        is_summary_enabled,
        patterns,
        print_help,
    },
    ata_cleanup,
    dashboard::{ self, Dashboard },
    global,
    logger::{ init_file_logging, log, LogTag },
    positions,
    rpc,
    summary,
    tokens::{ self, monitor, pool, TokenDatabase },
    trader::{ self, CriticalOperationGuard },
    transactions,
    wallet,
};

use solana_sdk::signer::Signer;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

// Constants for timeouts and limits
const CRITICAL_OPS_TIMEOUT_SECS: u64 = 60;
const CLEANUP_TIMEOUT_SECS: u64 = 3;
const EMERGENCY_WAIT_SECS: u64 = 30;
const DASHBOARD_MONITOR_INTERVAL_MS: u64 = 500;
const PROGRESS_UPDATE_INTERVAL_SECS: u64 = 2;
const DEFAULT_TASK_TIMEOUT_SECS: u64 = 10;
const DASHBOARD_TASK_TIMEOUT_SECS: u64 = 20;
const EXTENDED_TASK_TIMEOUT_SECS: u64 = 120;
const DASHBOARD_MIN_TIMEOUT_SECS: u64 = 22;

// Helper function for debug logging to reduce repetition
fn debug_log(tag: LogTag, level: &str, message: &str) {
    if is_debug_system_enabled() {
        log(tag, level, message);
    }
}

/// Main bot execution function - handles the full bot lifecycle
pub async fn run_bot() -> Result<(), String> {
    // Initialize file logging system first
    init_file_logging();

    // Check for dry-run mode and log it prominently
    if is_dry_run_enabled() {
        log(LogTag::System, "CRITICAL", "🚫 DRY-RUN MODE ENABLED - NO ACTUAL TRADING WILL OCCUR");
        log(
            LogTag::System,
            "CRITICAL",
            "📊 All trading signals and analysis will be logged but not executed"
        );
    }

    // Create shared shutdown notification for all background tasks
    let shutdown = Arc::new(Notify::new());
    // Local trigger for initiating shutdown (from dashboard exit or OS Ctrl+C)
    let shutdown_trigger = Arc::new(Notify::new());
    // Service completion tracker - dashboard waits for this before exiting
    let services_completed = Arc::new(Notify::new());

    // Check for dashboard mode
    let dashboard_mode = is_dashboard_enabled();
    // Keep a handle so we can await dashboard shutdown at the end
    let mut dashboard_handle_opt: Option<tokio::task::JoinHandle<()>> = None;

    if dashboard_mode {
        log(LogTag::System, "INFO", "🖥️ Dashboard mode enabled - Starting terminal UI");

        // Create dashboard instance and set it globally for log forwarding
        let dashboard = std::sync::Arc::new(Dashboard::new());
        dashboard::set_global_dashboard(dashboard.clone());

        // Start dashboard in a separate task
        let shutdown_dashboard = shutdown.clone();
        let dashboard_running = dashboard.running.clone();
        let services_completed_dashboard = services_completed.clone();

        let dashboard_handle = tokio::spawn(async move {
            if
                let Err(e) = dashboard::run_dashboard(
                    shutdown_dashboard,
                    services_completed_dashboard
                ).await
            {
                // Avoid stderr prints in dashboard context; route to file logger
                debug_log(LogTag::System, "ERROR", &format!("Dashboard error: {}", e));
            }
            // Clear global dashboard on exit
            dashboard::clear_global_dashboard();
        });
        dashboard_handle_opt = Some(dashboard_handle);

        // Monitor dashboard state and trigger main shutdown when dashboard exits
        let shutdown_monitor = shutdown.clone();
        let shutdown_trigger_for_monitor = shutdown_trigger.clone();
        tokio::spawn(async move {
            loop {
                if let Ok(running) = dashboard_running.lock() {
                    if !*running {
                        // Dashboard has exited, trigger main shutdown
                        shutdown_monitor.notify_waiters();
                        shutdown_trigger_for_monitor.notify_waiters();
                        break;
                    }
                }
                tokio::time::sleep(Duration::from_millis(DASHBOARD_MONITOR_INTERVAL_MS)).await;
            }
        });

        // Also watch for OS Ctrl+C and trigger unified shutdown in dashboard mode
        let shutdown_trigger_os = shutdown_trigger.clone();
        tokio::spawn(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                debug_log(LogTag::System, "INFO", "Shutdown signal received (Ctrl+C)");
            }
            shutdown_trigger_os.notify_waiters();
        });

        // In dashboard mode, we'll run a simplified background version
        log(LogTag::System, "INFO", "Running in dashboard mode with terminal UI");
    } else {
        debug_log(LogTag::System, "INFO", "Running in console mode");
    }

    // Initialize centralized blacklist system with system/stable tokens
    tokens::initialize_system_stable_blacklist();

    debug_log(LogTag::System, "INFO", "Starting ScreenerBot background tasks");

    // Emergency shutdown flag (used below after first Ctrl+C)
    let emergency_shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));

    // Initialize tokens system (includes price service initialization)
    let mut tokens_system = match tokens::initialize_tokens_system().await {
        Ok(system) => system,
        Err(e) => {
            debug_log(
                LogTag::System,
                "ERROR",
                &format!("Failed to initialize tokens system: {}", e)
            );
            return Err(format!("Failed to initialize tokens system: {}", e));
        }
    };

    // Initialize and start pool service for real-time price calculations and history caching
    let pool_service = pool::init_pool_service();
    pool_service.start_monitoring().await;
    debug_log(
        LogTag::System,
        "INFO",
        "Pool price service with disk caching initialized and monitoring started"
    );

    let shutdown_tokens = shutdown.clone();

    // Initialize global rugcheck service
    let database = match TokenDatabase::new() {
        Ok(db) => db,
        Err(e) => {
            debug_log(
                LogTag::System,
                "ERROR",
                &format!("Failed to create database for rugcheck: {}", e)
            );
            return Err(format!("Failed to create database for rugcheck: {}", e));
        }
    };

    let shutdown_rugcheck = shutdown.clone();
    let rugcheck_handle = match
        tokens::initialize_global_rugcheck_service(database, shutdown_rugcheck).await
    {
        Ok(handle) => handle,
        Err(e) => {
            debug_log(
                LogTag::System,
                "ERROR",
                &format!("Failed to initialize global rugcheck service: {}", e)
            );
            return Err(format!("Failed to initialize global rugcheck service: {}", e));
        }
    };
    debug_log(LogTag::System, "INFO", "Global rugcheck service initialized successfully");

    // Start token monitoring service for database updates
    let shutdown_monitor = shutdown.clone();
    let _token_monitor_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "Token monitoring service task started");
        match monitor::start_token_monitoring(shutdown_monitor).await {
            Ok(handle) => {
                if let Err(e) = handle.await {
                    debug_log(
                        LogTag::System,
                        "ERROR",
                        &format!("Token monitoring task failed: {:?}", e)
                    );
                }
            }
            Err(e) => {
                debug_log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to start token monitoring: {}", e)
                );
            }
        }
        debug_log(LogTag::System, "INFO", "Token monitoring service task ended");
    });

    // Start tokens system background tasks (includes rugcheck service)
    let tokens_handles = match tokens_system.start_background_tasks(shutdown_tokens).await {
        Ok(handles) => handles,
        Err(e) => {
            debug_log(
                LogTag::System,
                "WARN",
                &format!("Some tokens system tasks failed to start: {}", e)
            );
            Vec::new()
        }
    };

    // Start RPC stats auto-save background service
    let shutdown_rpc_stats = shutdown.clone();
    let rpc_stats_handle = tokio::spawn(async move {
        debug_log(LogTag::System, "INFO", "RPC stats auto-save service task started");
        rpc::start_rpc_stats_auto_save_service(shutdown_rpc_stats).await;
        debug_log(LogTag::System, "INFO", "RPC stats auto-save service task ended");
    });

    // Start ATA cleanup background service
    let shutdown_ata_cleanup = shutdown.clone();
    let ata_cleanup_handle = tokio::spawn(async move {
        debug_log(LogTag::System, "INFO", "ATA cleanup service task started");
        ata_cleanup::start_ata_cleanup_service(shutdown_ata_cleanup).await;
        debug_log(LogTag::System, "INFO", "ATA cleanup service task ended");
    });

    // Start wallet monitoring background service
    let shutdown_wallet = shutdown.clone();
    let wallet_monitor_handle = wallet::start_wallet_monitoring_service(shutdown_wallet).await;

    // Start trader tasks (moved from trader() function for centralized management)

    // Initialize global transaction manager FIRST (before reconciliation)
    // Load wallet address from config for transaction monitoring
    match global::read_configs() {
        Ok(configs) =>
            match global::load_wallet_from_config(&configs) {
                Ok(keypair) => {
                    let wallet_pubkey = keypair.pubkey();
                    if
                        let Err(e) =
                            transactions::initialize_global_transaction_manager(wallet_pubkey).await
                    {
                        debug_log(
                            LogTag::System,
                            "ERROR",
                            &format!("Failed to initialize global transaction manager: {}", e)
                        );
                        return Err(
                            format!("Failed to initialize global transaction manager: {}", e)
                        );
                    }
                    debug_log(
                        LogTag::System,
                        "INFO",
                        "Global transaction manager initialized for swap monitoring"
                    );
                }
                Err(e) => {
                    debug_log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to load wallet keypair for transaction manager: {}", e)
                    );
                    return Err(
                        format!("Failed to load wallet keypair for transaction manager: {}", e)
                    );
                }
            }
        Err(e) => {
            debug_log(
                LogTag::System,
                "ERROR",
                &format!("Failed to read configs for transaction manager: {}", e)
            );
            return Err(format!("Failed to read configs for transaction manager: {}", e));
        }
    }

    // Start PositionsManager background service
    let shutdown_positions_manager = shutdown.clone();
    let positions_manager_handle = tokio::spawn(async move {
        debug_log(LogTag::System, "INFO", "PositionsManager service task started");
        let _sender = positions::start_positions_manager_service(shutdown_positions_manager).await;
        debug_log(LogTag::System, "INFO", "PositionsManager service task ended");
    });

    let shutdown_entries = shutdown.clone();
    let entries_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "New entries monitor task started");
        trader::monitor_new_entries(shutdown_entries).await;
        log(LogTag::Trader, "INFO", "New entries monitor task ended");
    });

    let shutdown_positions = shutdown.clone();
    let positions_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "Open positions monitor task started");
        trader::monitor_open_positions(shutdown_positions).await;
        log(LogTag::Trader, "INFO", "Open positions monitor task ended");
    });

    let shutdown_stale_refresh = shutdown.clone();
    let stale_refresh_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "Stale price history refresh task started");
        trader::refresh_stale_price_history(shutdown_stale_refresh).await;
        log(LogTag::Trader, "INFO", "Stale price history refresh task ended");
    });

    let shutdown_display = shutdown.clone();
    let display_handle = if is_summary_enabled() {
        tokio::spawn(async move {
            // Add a small delay to ensure reconcile function completes first and avoid deadlock
            tokio::time::sleep(Duration::from_secs(2)).await;
            log(LogTag::Trader, "INFO", "Positions display task started");
            summary::summary_loop(shutdown_display).await;
            log(LogTag::Trader, "INFO", "Positions display task ended");
        })
    } else {
        // Create a dummy handle that does nothing when summary is disabled
        tokio::spawn(async move {
            // Wait for shutdown signal without doing any work
            shutdown_display.notified().await;
        })
    };

    // Start transaction manager background service
    let shutdown_transactions = shutdown.clone();
    let transaction_manager_handle = tokio::spawn(async move {
        debug_log(LogTag::System, "INFO", "Transaction manager service task started");
        transactions::start_transactions_service(shutdown_transactions).await;
        debug_log(LogTag::System, "INFO", "Transaction manager service task ended");
    });

    if dashboard_mode {
        debug_log(LogTag::System, "INFO", "Waiting for exit (q/Esc/Ctrl+C) to shutdown");
        // Wait until dashboard requests shutdown or OS Ctrl+C arrives
        shutdown_trigger.notified().await;
        emergency_shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
        debug_log(LogTag::System, "INFO", "Shutdown requested, initiating graceful shutdown...");
    } else {
        debug_log(LogTag::System, "INFO", "Waiting for Ctrl+C to shutdown");
        // Set up Ctrl+C signal handler with better error handling
        match tokio::signal::ctrl_c().await {
            Ok(_) => {
                emergency_shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
                debug_log(
                    LogTag::System,
                    "INFO",
                    "Shutdown signal received, initiating graceful shutdown..."
                );
            }
            Err(e) => {
                debug_log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to listen for shutdown signal: {}", e)
                );
                return Err(format!("Failed to listen for shutdown signal: {}", e));
            }
        }
    }

    // Notify all tasks to shutdown
    debug_log(
        LogTag::System,
        "INFO",
        "📢 Starting shutdown notification to all background tasks..."
    );
    shutdown.notify_waiters();
    debug_log(LogTag::System, "INFO", "✅ Shutdown notification sent to all background tasks");
    let shutdown_start_time = std::time::Instant::now();

    // CRITICAL PROTECTION: Check for active trading operations
    let critical_ops_count = CriticalOperationGuard::get_active_count();
    if critical_ops_count > 0 {
        log(
            LogTag::System,
            "CRITICAL",
            &format!("🚨 WAITING FOR {} CRITICAL TRADING OPERATIONS TO COMPLETE BEFORE SHUTDOWN", critical_ops_count)
        );
        log(
            LogTag::System,
            "CRITICAL",
            "⚠️  DO NOT FORCE KILL - Financial operations in progress!"
        );

        // Wait for critical operations to complete (max 60 seconds)
        let critical_ops_timeout = std::time::Instant::now();
        while CriticalOperationGuard::get_active_count() > 0 {
            if
                critical_ops_timeout.elapsed() >
                std::time::Duration::from_secs(CRITICAL_OPS_TIMEOUT_SECS)
            {
                log(
                    LogTag::System,
                    "EMERGENCY",
                    "⚠️  CRITICAL OPERATIONS TIMEOUT - Some trades may be incomplete!"
                );
                break;
            }

            let remaining = CriticalOperationGuard::get_active_count();
            if remaining > 0 {
                debug_log(
                    LogTag::System,
                    "CRITICAL",
                    &format!(
                        "🔒 Still waiting for {} critical operations... ({}s elapsed)",
                        remaining,
                        critical_ops_timeout.elapsed().as_secs()
                    )
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        if CriticalOperationGuard::get_active_count() == 0 {
            debug_log(
                LogTag::System,
                "CRITICAL",
                "✅ All critical trading operations completed safely"
            );
        }
    }

    // Cleanup price service on shutdown (with timeout)
    let cleanup_result = tokio::time::timeout(
        std::time::Duration::from_secs(CLEANUP_TIMEOUT_SECS),
        async {
            // Stop pool monitoring service
            let pool_service = pool::get_pool_service();
            pool_service.stop_monitoring().await;
            debug_log(LogTag::System, "INFO", "Pool monitoring service stopped");

            // Decimals are now automatically saved to database
            debug_log(LogTag::System, "INFO", "Decimals database persists automatically");

            // Save RPC statistics to disk
            if let Err(e) = rpc::save_global_rpc_stats() {
                debug_log(LogTag::System, "WARN", &format!("Failed to save RPC statistics: {}", e));
            } else {
                debug_log(LogTag::System, "INFO", "RPC statistics saved to disk");
            }
        }
    ).await;

    match cleanup_result {
        Ok(_) => {
            debug_log(LogTag::System, "INFO", "Cleanup completed successfully");
        }
        Err(_) => {
            debug_log(LogTag::System, "WARN", "Cleanup timed out after 3 seconds");
        }
    }

    // Wait for background tasks to finish with timeout that respects critical operations
    let final_critical_ops = CriticalOperationGuard::get_active_count();
    let mut task_timeout_seconds = if final_critical_ops > 0 {
        log(
            LogTag::System,
            "CRITICAL",
            &format!("🚨 {} CRITICAL OPERATIONS STILL ACTIVE - Extending task shutdown timeout to 120 seconds", final_critical_ops)
        );
        EXTENDED_TASK_TIMEOUT_SECS
    } else if dashboard_mode {
        DASHBOARD_TASK_TIMEOUT_SECS
    } else {
        DEFAULT_TASK_TIMEOUT_SECS
    };

    // If in dashboard mode, ensure timeout is at least as long as dashboard's max wait window
    if dashboard_mode {
        task_timeout_seconds = task_timeout_seconds.max(DASHBOARD_MIN_TIMEOUT_SECS);
    }

    debug_log(
        LogTag::System,
        "INFO",
        &format!("Waiting for background tasks to shutdown (max {} seconds)...", task_timeout_seconds)
    );

    // Start a progress monitor task that runs in parallel
    let progress_shutdown = shutdown.clone();
    let progress_task = tokio::spawn(async move {
        let mut progress_interval = tokio::time::interval(
            Duration::from_secs(PROGRESS_UPDATE_INTERVAL_SECS)
        );
        let mut elapsed = 0u64;

        loop {
            tokio::select! {
                _ = progress_shutdown.notified() => break,
                _ = progress_interval.tick() => {
                    elapsed += PROGRESS_UPDATE_INTERVAL_SECS;
                    debug_log(LogTag::System, "INFO", &format!("⏳ Shutdown progress: {}s elapsed, still waiting for tasks...", elapsed));
                }
            }
        }
    });

    let shutdown_timeout = tokio::time::timeout(
        std::time::Duration::from_secs(task_timeout_seconds),
        async {
            // Wait for trader tasks
            debug_log(LogTag::System, "INFO", "🔄 Waiting for entries monitor task to shutdown...");
            if let Err(e) = entries_handle.await {
                debug_log(
                    LogTag::System,
                    "WARN",
                    &format!("New entries monitor task failed to shutdown cleanly: {}", e)
                );
            } else {
                debug_log(LogTag::System, "INFO", "✅ Entries monitor task shutdown completed");
            }

            debug_log(
                LogTag::System,
                "INFO",
                "🔄 Waiting for positions monitor task to shutdown..."
            );
            if let Err(e) = positions_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Open positions monitor task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ Positions monitor task shutdown completed");
            }

            debug_log(
                LogTag::System,
                "INFO",
                "🔄 Waiting for stale price refresh task to shutdown..."
            );
            if let Err(e) = stale_refresh_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Stale price refresh task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ Stale price refresh task shutdown completed");
            }

            log(LogTag::System, "INFO", "🔄 Waiting for positions display task to shutdown...");
            if let Err(e) = display_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Positions display task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ Positions display task shutdown completed");
            }

            // Wait for PositionsManager service
            log(LogTag::System, "INFO", "🔄 Waiting for PositionsManager task to shutdown...");
            if let Err(e) = positions_manager_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("PositionsManager task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ PositionsManager task shutdown completed");
            }

            // Wait for RPC stats auto-save service
            log(LogTag::System, "INFO", "🔄 Waiting for RPC stats auto-save task to shutdown...");
            if let Err(e) = rpc_stats_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("RPC stats auto-save task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ RPC stats auto-save task shutdown completed");
            }

            // Wait for ATA cleanup service
            log(LogTag::System, "INFO", "🔄 Waiting for ATA cleanup task to shutdown...");
            if let Err(e) = ata_cleanup_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("ATA cleanup task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ ATA cleanup task shutdown completed");
            }

            // Wait for wallet monitoring service
            log(LogTag::System, "INFO", "🔄 Waiting for wallet monitoring task to shutdown...");
            if let Err(e) = wallet_monitor_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Wallet monitoring task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ Wallet monitoring task shutdown completed");
            }

            // Wait for transaction manager service
            log(LogTag::System, "INFO", "🔄 Waiting for transaction manager task to shutdown...");
            if let Err(e) = transaction_manager_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Transaction manager task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ Transaction manager task shutdown completed");
            }

            // Wait for tokens system tasks (includes rugcheck-related tasks)
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "🔄 Waiting for {} tokens system tasks to shutdown...",
                    tokens_handles.len()
                )
            );
            for (i, handle) in tokens_handles.into_iter().enumerate() {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("🔄 Waiting for tokens task {} to shutdown...", i)
                );
                if let Err(e) = handle.await {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Tokens task {} failed to shutdown cleanly: {}", i, e)
                    );
                } else {
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("✅ Tokens task {} shutdown completed", i)
                    );
                }
            }

            // Wait for Rugcheck service task explicitly
            log(LogTag::System, "INFO", "🔄 Waiting for Rugcheck service task to shutdown...");
            if let Err(e) = rugcheck_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Rugcheck task failed to shutdown cleanly: {}", e)
                );
            } else {
                log(LogTag::System, "INFO", "✅ Rugcheck service task shutdown completed");
            }
        }
    );

    // Stop the progress monitor
    progress_task.abort();

    match shutdown_timeout.await {
        Ok(_) => {
            let total_shutdown_time = shutdown_start_time.elapsed();
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "All background tasks finished gracefully in {:.2}s.",
                    total_shutdown_time.as_secs_f64()
                )
            );
            // Notify dashboard that all services have completed
            services_completed.notify_waiters();
        }
        Err(_) => {
            let total_shutdown_time = shutdown_start_time.elapsed();
            let final_critical_check = CriticalOperationGuard::get_active_count();
            if final_critical_check > 0 {
                log(
                    LogTag::System,
                    "EMERGENCY",
                    &format!(
                        "🚨 CRITICAL: {} trading operations still active during forced shutdown after {:.2}s! This may cause data loss!",
                        final_critical_check,
                        total_shutdown_time.as_secs_f64()
                    )
                );
                log(
                    LogTag::System,
                    "EMERGENCY",
                    "⚠️  Waiting additional 30 seconds for critical operations to complete before force exit..."
                );

                // Last ditch effort - wait another 30 seconds for critical operations
                let emergency_start = std::time::Instant::now();
                while
                    CriticalOperationGuard::get_active_count() > 0 &&
                    emergency_start.elapsed() < std::time::Duration::from_secs(EMERGENCY_WAIT_SECS)
                {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    let remaining = CriticalOperationGuard::get_active_count();
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        &format!("🔒 Emergency wait: {} critical operations remaining...", remaining)
                    );
                }

                if CriticalOperationGuard::get_active_count() > 0 {
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        "💥 FORCE SHUTDOWN WITH ACTIVE TRADES - POTENTIAL DATA LOSS!"
                    );
                    return Err(
                        "Force shutdown with active trades - potential data loss".to_string()
                    );
                } else {
                    log(
                        LogTag::System,
                        "INFO",
                        "✅ Emergency wait successful - all critical operations completed"
                    );
                }
            }

            log(
                LogTag::System,
                "WARN",
                &format!(
                    "⚠️ Tasks did not finish within {} second timeout (total time: {:.2}s). Some tasks may still be running.",
                    task_timeout_seconds,
                    total_shutdown_time.as_secs_f64()
                )
            );

            // Even on timeout, notify dashboard that we're done trying
            services_completed.notify_waiters();

            if dashboard_mode {
                log(
                    LogTag::System,
                    "WARN",
                    "Exiting without abort to preserve terminal state (dashboard mode)"
                );
                return Err("Dashboard mode timeout".to_string());
            } else {
                return Err("Task shutdown timeout".to_string());
            }
        }
    }

    // Finally, if dashboard was running, wait briefly for it to restore the terminal
    if let Some(handle) = dashboard_handle_opt.take() {
        let _ = tokio::time::timeout(std::time::Duration::from_secs(3), handle).await;
    }

    Ok(())
}
