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
    tokio::spawn(async {
        // The manager is transiently taken OUT of the global (Option=None) while
        // start_all() runs during boot — that take-out/start/put-back window lasts
        // several seconds. The loop must NOT terminate on that transient None
        // (doing so left no task to refresh the cache, freezing the whole services
        // table at its pre-startup snapshot — uptime stuck at 0). Skip those ticks
        // and only give up after a sustained absence, which only happens at
        // shutdown (when the manager is taken out for good and the process exits).
        const MAX_CONSECUTIVE_MISSES: u32 = 24; // ~2 minutes at 5s cadence
        let mut consecutive_misses: u32 = 0;

        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;

            let manager_ref = match get_service_manager().await {
                Some(m) => m,
                None => {
                    consecutive_misses += 1;
                    if consecutive_misses >= MAX_CONSECUTIVE_MISSES {
                        logger::debug(
                            LogTag::System,
                            "ServiceManager: Cache update task terminating - manager gone",
                        );
                        break;
                    }
                    continue;
                }
            };

            let manager_guard = manager_ref.read().await;
            let manager = match manager_guard.as_ref() {
                Some(m) => m,
                None => {
                    consecutive_misses += 1;
                    if consecutive_misses >= MAX_CONSECUTIVE_MISSES {
                        logger::debug(
                            LogTag::System,
                            "ServiceManager: Cache update task terminating - manager cleared",
                        );
                        break;
                    }
                    continue;
                }
            };

            consecutive_misses = 0;

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
