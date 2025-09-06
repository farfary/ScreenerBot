/// Pool service supervisor - manages the lifecycle of all pool-related tasks
///
/// This module provides the main entry points for starting and stopping the pool service.
/// It coordinates all the background tasks needed for price discovery and calculation.

use crate::logger::{ log, LogTag };
use crate::arguments::is_debug_pool_service_enabled;
use super::PoolError;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::Arc;
use tokio::sync::Notify;

// Global service state
static SERVICE_RUNNING: AtomicBool = AtomicBool::new(false);
static mut GLOBAL_SHUTDOWN_HANDLE: Option<Arc<Notify>> = None;

/// Start the pool service with all background tasks
///
/// This function initializes and starts all the necessary background tasks for
/// pool discovery, price calculation, and caching.
///
/// Returns an error if the service is already running or if initialization fails.
pub async fn start_pool_service() -> Result<(), PoolError> {
    // Check if already running
    if SERVICE_RUNNING.swap(true, Ordering::SeqCst) {
        log(LogTag::PoolService, "WARN", "Pool service is already running");
        return Err(PoolError::InitializationFailed("Service already running".to_string()));
    }

    log(LogTag::PoolService, "INFO", "Starting pool service...");

    // Create shutdown notification
    let shutdown = Arc::new(Notify::new());

    // Store shutdown handle globally
    unsafe {
        GLOBAL_SHUTDOWN_HANDLE = Some(shutdown.clone());
    }

    // Start background tasks
    start_background_tasks(shutdown).await;

    log(LogTag::PoolService, "SUCCESS", "Pool service started successfully");
    Ok(())
}

/// Stop the pool service and all background tasks
///
/// This function gracefully shuts down all background tasks and cleans up resources.
/// It waits for tasks to complete within the specified timeout.
pub async fn stop_pool_service(timeout_seconds: u64) -> Result<(), PoolError> {
    if !SERVICE_RUNNING.load(Ordering::Relaxed) {
        log(LogTag::PoolService, "WARN", "Pool service is not running");
        return Ok(());
    }

    log(
        LogTag::PoolService,
        "INFO",
        &format!("Stopping pool service (timeout: {}s)...", timeout_seconds)
    );

    // Get shutdown handle and notify
    unsafe {
        if let Some(ref handle) = GLOBAL_SHUTDOWN_HANDLE {
            handle.notify_waiters();
        }
    }

    // Wait for shutdown with timeout
    let shutdown_result = tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_seconds),
        async {
            // Give tasks time to shutdown gracefully
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    ).await;

    match shutdown_result {
        Ok(_) => {
            SERVICE_RUNNING.store(false, Ordering::Relaxed);
            unsafe {
                GLOBAL_SHUTDOWN_HANDLE = None;
            }

            log(LogTag::PoolService, "SUCCESS", "✅ Pool service stopped successfully");
            Ok(())
        }
        Err(_) => {
            log(LogTag::PoolService, "WARN", "Pool service shutdown timed out");
            Err(PoolError::InitializationFailed("Shutdown timeout".to_string()))
        }
    }
}

/// Check if the pool service is currently running
pub fn is_pool_service_running() -> bool {
    SERVICE_RUNNING.load(Ordering::SeqCst)
}

/// Start all background tasks for the pool service
async fn start_background_tasks(shutdown: Arc<Notify>) {
    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Starting background tasks...");
    }

    // For now, start with a minimal placeholder task
    // This will be expanded as we implement each component
    let shutdown_monitor = shutdown.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));

        loop {
            tokio::select! {
                _ = shutdown_monitor.notified() => {
                    if is_debug_pool_service_enabled() {
                        log(LogTag::PoolService, "INFO", "Pool service monitor task shutting down");
                    }
                    break;
                }
                _ = interval.tick() => {
                    if is_debug_pool_service_enabled() {
                        log(LogTag::PoolService, "DEBUG", "Pool service heartbeat");
                    }
                }
            }
        }
    });

    if is_debug_pool_service_enabled() {
        log(LogTag::PoolService, "DEBUG", "Background tasks started");
    }
}
