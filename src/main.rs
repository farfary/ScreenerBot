use screenerbot::logger::{ log, LogTag, init_file_logging };

use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    // Initialize file logging system first
    init_file_logging();

    // Initialize centralized blacklist system with system/stable tokens
    screenerbot::tokens::initialize_system_stable_blacklist();

    log(LogTag::System, "INFO", "Starting ScreenerBot background tasks");
    
    // Create shared shutdown notification for all background tasks
    let shutdown = Arc::new(Notify::new());

    // Set up emergency shutdown handler (second Ctrl+C will force kill)
    let emergency_shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let emergency_shutdown_clone = emergency_shutdown.clone();

    tokio::spawn(async move {
        // Wait for second Ctrl+C
        if tokio::signal::ctrl_c().await.is_ok() {
            if emergency_shutdown_clone.load(std::sync::atomic::Ordering::SeqCst) {
                // Check for critical operations before force killing
                let critical_ops = screenerbot::trader::CriticalOperationGuard::get_active_count();
                if critical_ops > 0 {
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        &format!("🚨 SECOND Ctrl+C DETECTED BUT {} CRITICAL TRADING OPERATIONS STILL ACTIVE!", critical_ops)
                    );
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        "⚠️  FORCE KILL BLOCKED - Would cause financial loss!"
                    );
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        "🔒 Waiting for trading operations to complete..."
                    );
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        "💡 Press Ctrl+C a THIRD time to override (DANGEROUS!)"
                    );

                    // Wait for third Ctrl+C to override protection
                    if tokio::signal::ctrl_c().await.is_ok() {
                        log(
                            LogTag::System,
                            "EMERGENCY",
                            "💀 THIRD Ctrl+C - FORCE KILLING DESPITE ACTIVE OPERATIONS!"
                        );
                        log(
                            LogTag::System,
                            "EMERGENCY",
                            "⚠️  THIS MAY CAUSE FINANCIAL LOSS OR INCOMPLETE TRADES!"
                        );
                        std::process::abort();
                    }
                } else {
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        "Second Ctrl+C detected - FORCE KILLING APPLICATION"
                    );
                    std::process::abort(); // Immediate termination
                }
            }
        }
    });

    // Initialize tokens system
    let mut tokens_system = match screenerbot::tokens::initialize_tokens_system().await {
        Ok(system) => system,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to initialize tokens system: {}", e));
            std::process::exit(1);
        }
    };

    // Initialize price service for thread-safe price access
    if let Err(e) = screenerbot::tokens::initialize_price_service().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize price service: {}", e));
        std::process::exit(1);
    }

    log(LogTag::System, "INFO", "Thread-safe price service initialized successfully");

    // Initialize and start pool service for real-time price calculations and history caching
    let pool_service = screenerbot::tokens::pool::init_pool_service();
    pool_service.start_monitoring().await;
    log(
        LogTag::System,
        "INFO",
        "Pool price service with disk caching initialized and monitoring started"
    );

    // Initialize wallet tracker for balance and value monitoring
    if let Err(e) = screenerbot::wallet_tracker::init_wallet_tracker().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize wallet tracker: {}", e));
        std::process::exit(1);
    }
    log(LogTag::System, "INFO", "Wallet tracker initialized successfully");

    // Initialize wallet transaction manager for efficient transaction caching
    if let Err(e) = screenerbot::wallet_transactions::initialize_wallet_transaction_manager().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize wallet transaction manager: {}", e));
        std::process::exit(1);
    }
    log(LogTag::System, "INFO", "Wallet transaction manager initialized successfully");

    // Start wallet transaction periodic sync background service
    let shutdown_wallet_transactions = shutdown.clone();
    let wallet_transactions_handle = match screenerbot::wallet_transactions::start_wallet_transaction_sync_task(shutdown_wallet_transactions).await {
        Ok(handle) => handle,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to start wallet transaction sync task: {}", e));
            std::process::exit(1);
        }
    };
    log(LogTag::System, "INFO", "Wallet transaction periodic sync task started successfully");

    // Initialize position verifier for automatic transaction verification
    if let Err(e) = screenerbot::position_verifier::init_position_verifier().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize position verifier: {}", e));
        std::process::exit(1);
    }
    log(LogTag::System, "INFO", "Position verifier initialized successfully");

    // Start position verification monitoring background service
    let shutdown_position_verifier = shutdown.clone();
    let position_verifier_handle = match screenerbot::position_verifier::start_position_verification(shutdown_position_verifier).await {
        Ok(handle) => handle,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to start position verification service: {}", e));
            std::process::exit(1);
        }
    };
    log(LogTag::System, "INFO", "Position verification monitoring service started successfully");

    let shutdown_tokens = shutdown.clone();
    let shutdown_pricing = shutdown.clone();

    // Initialize global rugcheck service
    let database = match screenerbot::tokens::TokenDatabase::new() {
        Ok(db) => db,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to create database for rugcheck: {}", e));
            std::process::exit(1);
        }
    };

    let shutdown_rugcheck = shutdown.clone();
    if
        let Err(e) = screenerbot::tokens::initialize_global_rugcheck_service(
            database,
            shutdown_rugcheck
        ).await
    {
        log(
            LogTag::System,
            "ERROR",
            &format!("Failed to initialize global rugcheck service: {}", e)
        );
        std::process::exit(1);
    }
    log(LogTag::System, "INFO", "Global rugcheck service initialized successfully");

    // Start tokens system background tasks (includes rugcheck service)
    let tokens_handles = match tokens_system.start_background_tasks(shutdown_tokens).await {
        Ok(handles) => handles,
        Err(e) => {
            log(
                LogTag::System,
                "WARN",
                &format!("Some tokens system tasks failed to start: {}", e)
            );
            Vec::new()
        }
    };

    // Start pricing background tasks
    let pricing_handles = match
        screenerbot::tokens::start_pricing_background_tasks(shutdown_pricing).await
    {
        Ok(handles) => handles,
        Err(e) => {
            log(
                LogTag::System,
                "WARN",
                &format!("Pricing background tasks failed to start: {}", e)
            );
            Vec::new()
        }
    };

    // Start RPC stats auto-save background service
    let shutdown_rpc_stats = shutdown.clone();
    let rpc_stats_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "RPC stats auto-save service task started");
        screenerbot::rpc::start_rpc_stats_auto_save_service(shutdown_rpc_stats).await;
        log(LogTag::System, "INFO", "RPC stats auto-save service task ended");
    });

    // Start ATA cleanup background service
    let shutdown_ata_cleanup = shutdown.clone();
    let ata_cleanup_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "ATA cleanup service task started");
        screenerbot::ata_cleanup::start_ata_cleanup_service(shutdown_ata_cleanup).await;
        log(LogTag::System, "INFO", "ATA cleanup service task ended");
    });

    // Start reinforcement learning background service
    let shutdown_rl_learning = shutdown.clone();
    let rl_learning_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "Reinforcement learning service task started");
        screenerbot::rl_learning::start_learning_service(shutdown_rl_learning).await;
        log(LogTag::System, "INFO", "Reinforcement learning service task ended");
    });

    // Start RL auto-save background service
    let shutdown_rl_autosave = shutdown.clone();
    let rl_autosave_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "RL auto-save service task started");
        screenerbot::rl_learning::start_rl_auto_save_service(shutdown_rl_autosave).await;
        log(LogTag::System, "INFO", "RL auto-save service task ended");
    });

    // Start trader tasks (moved from trader() function for centralized management)
    let shutdown_entries = shutdown.clone();
    let entries_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "New entries monitor task started");
        screenerbot::trader::monitor_new_entries(shutdown_entries).await;
        log(LogTag::Trader, "INFO", "New entries monitor task ended");
    });

    let shutdown_positions = shutdown.clone();
    let positions_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "Open positions monitor task started");
        screenerbot::trader::monitor_open_positions(shutdown_positions).await;
        log(LogTag::Trader, "INFO", "Open positions monitor task ended");
    });

    let shutdown_display = shutdown.clone();
    let display_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "Positions display task started");
        screenerbot::summary::monitor_positions_display(shutdown_display).await;
        log(LogTag::Trader, "INFO", "Positions display task ended");
    });

    // Start wallet tracker background service
    let shutdown_wallet = shutdown.clone();
    let wallet_tracker_handle = tokio::spawn(async move {
        log(LogTag::Wallet, "INFO", "Wallet tracker service task started");
        match screenerbot::wallet_tracker::start_wallet_tracking(shutdown_wallet).await {
            Ok(handle) => {
                if let Err(e) = handle.await {
                    log(LogTag::Wallet, "ERROR", &format!("Wallet tracker task error: {:?}", e));
                }
            }
            Err(e) => {
                log(LogTag::Wallet, "ERROR", &format!("Failed to start wallet tracker: {}", e));
            }
        }
        log(LogTag::Wallet, "INFO", "Wallet tracker service task ended");
    });

    log(
        LogTag::System,
        "INFO",
        "Waiting for Ctrl+C to shutdown (press Ctrl+C twice for immediate kill)"
    );

    // Set up Ctrl+C signal handler with better error handling
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            emergency_shutdown.store(true, std::sync::atomic::Ordering::SeqCst);
            log(
                LogTag::System,
                "INFO",
                "Shutdown signal received, initiating graceful shutdown..."
            );
            log(
                LogTag::System,
                "INFO",
                "Press Ctrl+C again within 5 seconds to force immediate termination"
            );
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to listen for shutdown signal: {}", e));
            std::process::exit(1);
        }
    }

    // Notify all tasks to shutdown
    shutdown.notify_waiters();
    log(LogTag::System, "INFO", "Shutdown notification sent to all background tasks");

    // CRITICAL PROTECTION: Check for active trading operations
    let critical_ops_count = screenerbot::trader::CriticalOperationGuard::get_active_count();
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
        while screenerbot::trader::CriticalOperationGuard::get_active_count() > 0 {
            if critical_ops_timeout.elapsed() > std::time::Duration::from_secs(60) {
                log(
                    LogTag::System,
                    "EMERGENCY",
                    "⚠️  CRITICAL OPERATIONS TIMEOUT - Some trades may be incomplete!"
                );
                break;
            }

            let remaining = screenerbot::trader::CriticalOperationGuard::get_active_count();
            if remaining > 0 {
                log(
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

        if screenerbot::trader::CriticalOperationGuard::get_active_count() == 0 {
            log(LogTag::System, "CRITICAL", "✅ All critical trading operations completed safely");
        }
    }

    // Cleanup price service on shutdown (with timeout)
    let cleanup_result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        // Stop pool monitoring service and save price history caches
        let pool_service = screenerbot::tokens::pool::get_pool_service();
        pool_service.stop_monitoring().await;
        log(LogTag::System, "INFO", "Pool monitoring service stopped and price history saved");

        screenerbot::tokens::cleanup_price_cache().await;
        screenerbot::tokens::decimals::save_decimal_cache();

        // Save RPC statistics to disk
        if let Err(e) = screenerbot::rpc::save_global_rpc_stats() {
            log(LogTag::System, "WARN", &format!("Failed to save RPC statistics: {}", e));
        } else {
            log(LogTag::System, "INFO", "RPC statistics saved to disk");
        }
    }).await;

    match cleanup_result {
        Ok(_) => log(LogTag::System, "INFO", "Cleanup completed successfully"),
        Err(_) => log(LogTag::System, "WARN", "Cleanup timed out after 3 seconds"),
    }

    // Wait for background tasks to finish with timeout that respects critical operations
    let final_critical_ops = screenerbot::trader::CriticalOperationGuard::get_active_count();
    let task_timeout_seconds = if final_critical_ops > 0 {
        log(
            LogTag::System,
            "CRITICAL",
            &format!("🚨 {} CRITICAL OPERATIONS STILL ACTIVE - Extending task shutdown timeout to 120 seconds", final_critical_ops)
        );
        120 // Extended timeout when critical operations are active
    } else {
        5 // Normal timeout when no critical operations
    };

    log(
        LogTag::System,
        "INFO",
        &format!("Waiting for background tasks to shutdown (max {} seconds)...", task_timeout_seconds)
    );
    let shutdown_timeout = tokio::time::timeout(
        std::time::Duration::from_secs(task_timeout_seconds),
        async {
            // Wait for trader tasks
            if let Err(e) = entries_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("New entries monitor task failed to shutdown cleanly: {}", e)
                );
            }

            if let Err(e) = positions_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Open positions monitor task failed to shutdown cleanly: {}", e)
                );
            }

            if let Err(e) = display_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Positions display task failed to shutdown cleanly: {}", e)
                );
            }

            if let Err(e) = wallet_tracker_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Wallet tracker task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for wallet transactions sync task
            if let Err(e) = wallet_transactions_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Wallet transactions sync task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for position verification service
            if let Err(e) = position_verifier_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Position verification task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for RPC stats auto-save service
            if let Err(e) = rpc_stats_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("RPC stats auto-save task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for ATA cleanup service
            if let Err(e) = ata_cleanup_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("ATA cleanup task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for RL learning service
            if let Err(e) = rl_learning_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("RL learning task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for RL auto-save service
            if let Err(e) = rl_autosave_handle.await {
                log(
                    LogTag::System,
                    "WARN",
                    &format!("RL auto-save task failed to shutdown cleanly: {}", e)
                );
            }

            // Wait for tokens system tasks (includes rugcheck service)
            for (i, handle) in tokens_handles.into_iter().enumerate() {
                if let Err(e) = handle.await {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Tokens task {} failed to shutdown cleanly: {}", i, e)
                    );
                }
            }

            // Wait for pricing tasks
            for (i, handle) in pricing_handles.into_iter().enumerate() {
                if let Err(e) = handle.await {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Pricing task {} failed to shutdown cleanly: {}", i, e)
                    );
                }
            }
        }
    );

    match shutdown_timeout.await {
        Ok(_) => {
            log(LogTag::System, "INFO", "All background tasks finished gracefully. Exiting.");
        }
        Err(_) => {
            let final_critical_check =
                screenerbot::trader::CriticalOperationGuard::get_active_count();
            if final_critical_check > 0 {
                log(
                    LogTag::System,
                    "EMERGENCY",
                    &format!("🚨 CRITICAL: {} trading operations still active during forced shutdown! This may cause data loss!", final_critical_check)
                );
                log(
                    LogTag::System,
                    "EMERGENCY",
                    "⚠️  Waiting additional 30 seconds for critical operations to complete before force exit..."
                );

                // Last ditch effort - wait another 30 seconds for critical operations
                let emergency_start = std::time::Instant::now();
                while
                    screenerbot::trader::CriticalOperationGuard::get_active_count() > 0 &&
                    emergency_start.elapsed() < std::time::Duration::from_secs(30)
                {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    let remaining = screenerbot::trader::CriticalOperationGuard::get_active_count();
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        &format!("🔒 Emergency wait: {} critical operations remaining...", remaining)
                    );
                }

                if screenerbot::trader::CriticalOperationGuard::get_active_count() > 0 {
                    log(
                        LogTag::System,
                        "EMERGENCY",
                        "💥 FORCE SHUTDOWN WITH ACTIVE TRADES - POTENTIAL DATA LOSS!"
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
                &format!("Tasks did not finish within {} second timeout, forcing immediate exit.", task_timeout_seconds)
            );
            // Force immediate termination
            std::process::abort();
        }
    }
}

// Access CMD_ARGS anywhere via CMD_ARGS.lock().unwrap()
