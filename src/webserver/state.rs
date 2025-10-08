/// Shared application state for the webserver
///
/// Contains references to core ScreenerBot systems and shared resources
/// that need to be accessed by route handlers.
use crate::config::WebserverConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state passed to all route handlers
#[derive(Clone)]
pub struct AppState {
    /// Webserver configuration
    pub config: Arc<WebserverConfig>,

    /// Server startup time
    pub startup_time: chrono::DateTime<chrono::Utc>,

    /// Central WebSocket hub
    pub ws_hub: Arc<crate::webserver::ws::WsHub>,
}

impl AppState {
    /// Create new application state with WsHub
    pub fn new(config: WebserverConfig, ws_hub: Arc<crate::webserver::ws::WsHub>) -> Self {
        Self {
            config: Arc::new(config),
            startup_time: chrono::Utc::now(),
            ws_hub,
        }
    }

    /// Get WsHub reference
    pub fn ws_hub(&self) -> &Arc<crate::webserver::ws::WsHub> {
        &self.ws_hub
    }

    /// Get current WebSocket connection count (from hub)
    pub async fn ws_connection_count(&self) -> usize {
        self.ws_hub.active_connections().await
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        (chrono::Utc::now() - self.startup_time)
            .num_seconds()
            .max(0) as u64
    }

    /// Get all service names from ServiceManager
    pub async fn get_all_services(&self) -> Vec<&'static str> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                return manager.get_all_service_names();
            }
        }
        vec![]
    }

    /// Get service health status
    pub async fn get_service_health(&self, name: &str) -> Option<crate::services::ServiceHealth> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                if let Some(service) = manager.get_service(name) {
                    return Some(service.health().await);
                }
            }
        }
        None
    }

    /// Get all services health
    pub async fn get_all_services_health(
        &self,
    ) -> std::collections::HashMap<&'static str, crate::services::ServiceHealth> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                return manager.get_health().await;
            }
        }
        std::collections::HashMap::new()
    }

    /// Get service metrics (optimized - uses read lock, not write lock)
    pub async fn get_service_metrics(
        &self,
    ) -> std::collections::HashMap<&'static str, crate::services::ServiceMetrics> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                return manager.get_metrics().await;
            }
        }
        std::collections::HashMap::new()
    }

    /// Get service details (priority, dependencies, enabled status)
    pub async fn get_service_details(&self, name: &str) -> Option<ServiceDetails> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                if let Some(service) = manager.get_service(name) {
                    return Some(ServiceDetails {
                        name: name.to_string(),
                        priority: service.priority(),
                        dependencies: service
                            .dependencies()
                            .iter()
                            .map(|s| s.to_string())
                            .collect(),
                        enabled: manager.is_service_enabled(name),
                    });
                }
            }
        }
        None
    }
}

/// Service details for webserver responses
#[derive(Debug, Clone)]
pub struct ServiceDetails {
    pub name: String,
    pub priority: i32,
    pub dependencies: Vec<String>,
    pub enabled: bool,
}

// Global state accessor
static GLOBAL_APP_STATE: once_cell::sync::OnceCell<Arc<AppState>> =
    once_cell::sync::OnceCell::new();

/// Set global app state (called during webserver initialization)
pub fn set_global_app_state(state: Arc<AppState>) {
    GLOBAL_APP_STATE.set(state).ok();
}

/// Get global app state
pub async fn get_app_state() -> Option<Arc<AppState>> {
    GLOBAL_APP_STATE.get().cloned()
}
