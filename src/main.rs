use screenerbot::trader::trader;
use screenerbot::logger::{ log, LogTag, init_file_logging };

use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    // Initialize file logging system first
    init_file_logging();

    log(LogTag::System, "INFO", "Starting ScreenerBot background tasks");

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

    let shutdown = Arc::new(Notify::new());
    let shutdown_trader = shutdown.clone();
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

    // Start ATA cleanup background service
    let shutdown_ata_cleanup = shutdown.clone();
    let ata_cleanup_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "ATA cleanup service task started");
        screenerbot::ata_cleanup::start_ata_cleanup_service(shutdown_ata_cleanup).await;
        log(LogTag::System, "INFO", "ATA cleanup service task ended");
    });

    let trader_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "Trader task started");
        trader(shutdown_trader).await;
        log(LogTag::System, "INFO", "Trader task ended");
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
        screenerbot::tokens::cleanup_price_cache().await;
        screenerbot::tokens::decimals::save_decimal_cache();
    }).await;

    match cleanup_result {
        Ok(_) => log(LogTag::System, "INFO", "Cleanup completed successfully"),
        Err(_) => log(LogTag::System, "WARN", "Cleanup timed out after 3 seconds"),
    }

    // Wait for background tasks to finish with shorter timeout and better handling
    log(LogTag::System, "INFO", "Waiting for background tasks to shutdown (max 5 seconds)...");
    let shutdown_timeout = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        // Wait for trader task
        if let Err(e) = trader_handle.await {
            log(LogTag::System, "WARN", &format!("Trader task failed to shutdown cleanly: {}", e));
        }

        // Wait for ATA cleanup service
        if let Err(e) = ata_cleanup_handle.await {
            log(
                LogTag::System,
                "WARN",
                &format!("ATA cleanup task failed to shutdown cleanly: {}", e)
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
    });

    match shutdown_timeout.await {
        Ok(_) => {
            log(LogTag::System, "INFO", "All background tasks finished gracefully. Exiting.");
        }
        Err(_) => {
            log(
                LogTag::System,
                "WARN",
                "Tasks did not finish within 5 second timeout, forcing immediate exit."
            );
            // Force immediate termination
            std::process::abort();
        }
    }
}

// Access CMD_ARGS anywhere via CMD_ARGS.lock().unwrap()
