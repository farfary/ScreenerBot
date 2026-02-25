//! Global ServiceManager singleton — provides process-wide access.

use super::ServiceManager;
use crate::logger::{self, LogTag};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::RwLock;

/// Global ServiceManager instance for webserver and other components access
static GLOBAL_SERVICE_MANAGER: LazyLock<Arc<RwLock<Option<ServiceManager>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Initialize global ServiceManager instance
pub async fn init_global_service_manager(manager: ServiceManager) {
    // Do initial cache update
    manager.update_cache().await;

    let mut global = GLOBAL_SERVICE_MANAGER.write().await;
    *global = Some(manager);
    logger::info(LogTag::System, "Global ServiceManager initialized");

    // Spawn background task to update cache every 5 seconds
    // (most services are idle, so less frequent updates reduce CPU overhead)
    // Task auto-terminates when ServiceManager is cleared (during shutdown)
    tokio::spawn(async {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            // Check if ServiceManager still exists - if not, shutdown is in progress
            let manager_ref = match get_service_manager().await {
                Some(m) => m,
                None => {
                    logger::debug(
                        LogTag::System,
                        "ServiceManager: Cache update task terminating - manager not available",
                    );
                    break;
                }
            };

            let manager_guard = manager_ref.read().await;
            let manager = match manager_guard.as_ref() {
                Some(m) => m,
                None => {
                    logger::debug(
                        LogTag::System,
                        "ServiceManager: Cache update task terminating - manager cleared",
                    );
                    break;
                }
            };

            // Add timeout to prevent hanging forever
            let update_future = manager.update_cache();
            match tokio::time::timeout(Duration::from_secs(3), update_future).await {
                Ok(_) => {
                    logger::debug(LogTag::System, "ServiceManager: Cache update completed");
                }
                Err(_) => {
                    logger::warning(
            LogTag::System,
            "ServiceManager: Cache update timed out after 3s - continuing with stale cache",
          );
                }
            }
        }
    });
}

/// Get reference to global ServiceManager
pub async fn get_service_manager() -> Option<Arc<RwLock<Option<ServiceManager>>>> {
    let global = GLOBAL_SERVICE_MANAGER.read().await;
    if global.is_some() {
        Some(GLOBAL_SERVICE_MANAGER.clone())
    } else {
        None
    }
}
