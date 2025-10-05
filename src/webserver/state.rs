/// Shared application state for the webserver
///
/// Contains references to core ScreenerBot systems and shared resources
/// that need to be accessed by route handlers.

use crate::webserver::config::WebserverConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state passed to all route handlers
#[derive(Clone)]
pub struct AppState {
    /// Webserver configuration
    pub config: Arc<WebserverConfig>,

    /// Active WebSocket connection count
    pub ws_connections: Arc<RwLock<usize>>,

    /// Server startup time
    pub startup_time: chrono::DateTime<chrono::Utc>,

    // Phase 2: Add references to bot systems
    // pub transactions_manager: Arc<TransactionsManager>,
    // pub positions_manager: Arc<PositionsManager>,
    // pub token_monitor: Arc<TokenMonitor>,
    // pub security_analyzer: Arc<SecurityAnalyzer>,
}

impl AppState {
    /// Create new application state
    pub fn new(config: WebserverConfig) -> Self {
        Self {
            config: Arc::new(config),
            ws_connections: Arc::new(RwLock::new(0)),
            startup_time: chrono::Utc::now(),
        }
    }

    /// Get current WebSocket connection count
    pub async fn ws_connection_count(&self) -> usize {
        *self.ws_connections.read().await
    }

    /// Increment WebSocket connection count
    pub async fn increment_ws_connections(&self) {
        let mut count = self.ws_connections.write().await;
        *count += 1;
    }

    /// Decrement WebSocket connection count
    pub async fn decrement_ws_connections(&self) {
        let mut count = self.ws_connections.write().await;
        if *count > 0 {
            *count -= 1;
        }
    }

    /// Get server uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        (chrono::Utc::now() - self.startup_time).num_seconds().max(0) as u64
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
        &self
    ) -> std::collections::HashMap<&'static str, crate::services::ServiceHealth> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(manager) = manager_ref.read().await.as_ref() {
                return manager.get_health().await;
            }
        }
        std::collections::HashMap::new()
    }

    /// Get service metrics
    pub async fn get_service_metrics(
        &self
    ) -> std::collections::HashMap<&'static str, crate::services::ServiceMetrics> {
        if let Some(manager_ref) = crate::services::get_service_manager().await {
            if let Some(mut manager) = manager_ref.write().await.as_mut() {
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
