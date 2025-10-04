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
        (chrono::Utc::now() - self.startup_time)
            .num_seconds()
            .max(0) as u64
    }
}
