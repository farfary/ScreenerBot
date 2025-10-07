/// Axum webserver implementation
///
/// Main server lifecycle management including startup, shutdown, and graceful termination
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tower_http::compression::CompressionLayer;

use crate::{
    config::WebserverConfig,
    logger::{ log, LogTag },
    webserver::{ routes, state::AppState },
};

/// Global shutdown notifier
static SHUTDOWN_NOTIFY: once_cell::sync::Lazy<Arc<Notify>> = once_cell::sync::Lazy::new(||
    Arc::new(Notify::new())
);

/// Start the webserver
///
/// This function blocks until the server is shut down
pub async fn start_server(config: WebserverConfig) -> Result<(), String> {
    // Validate configuration
    config.validate().map_err(|e| format!("Invalid webserver config: {}", e))?;

    log(LogTag::Webserver, "INFO", &format!("🌐 Starting webserver on {}", config.bind_address()));

    // Create application state
    let state = Arc::new(AppState::new(config.clone()));

    // Initialize WebSocket broadcast systems
    log(LogTag::Webserver, "INFO", "Initializing WebSocket broadcast systems...");

    // Initialize positions broadcaster
    crate::positions::initialize_positions_broadcaster();
    log(LogTag::Webserver, "INFO", "✅ Positions broadcast system initialized");

    // Initialize prices broadcaster
    crate::pools::initialize_prices_broadcaster();
    log(LogTag::Webserver, "INFO", "✅ Prices broadcast system initialized");

    // Initialize status broadcaster
    crate::webserver::initialize_status_broadcaster();
    log(LogTag::Webserver, "INFO", "✅ Status broadcast system initialized");

    // Start status broadcaster task (every 2 seconds)
    let _status_handle = crate::webserver::start_status_broadcaster(2);
    log(LogTag::Webserver, "INFO", "✅ Status broadcast task started (interval: 2s)");

    // Set global app state for WebSocket connection tracking
    crate::webserver::state::set_global_app_state(Arc::clone(&state));
    log(LogTag::Webserver, "INFO", "✅ Global app state configured");

    // Build the router
    let app = build_app(state.clone());

    // Parse bind address
    let addr: SocketAddr = config
        .bind_address()
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Create TCP listener
    let listener = TcpListener::bind(&addr).await.map_err(|e|
        format!("Failed to bind to {}: {}", addr, e)
    )?;

    log(LogTag::Webserver, "INFO", &format!("✅ Webserver listening on http://{}", addr));
    log(LogTag::Webserver, "INFO", &format!("📊 API endpoints available at http://{}/api", addr));

    // Run the server with graceful shutdown
    let shutdown_signal = async {
        SHUTDOWN_NOTIFY.notified().await;
        log(LogTag::Webserver, "INFO", "Received shutdown signal, stopping webserver...");
    };

    axum
        ::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal).await
        .map_err(|e| format!("Server error: {}", e))?;

    log(LogTag::Webserver, "INFO", "✅ Webserver stopped gracefully");

    Ok(())
}

/// Trigger webserver shutdown
pub fn shutdown() {
    log(LogTag::Webserver, "INFO", "Triggering webserver shutdown...");
    SHUTDOWN_NOTIFY.notify_one();
}

/// Build the Axum application with all routes and middleware
fn build_app(state: Arc<AppState>) -> Router {
    // Create main router
    let app = routes::create_router(state);

    // Add middleware layers (future)
    let app = app.layer(CompressionLayer::new());

    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = WebserverConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bind_address() {
        let config = WebserverConfig::default();
        assert_eq!(config.bind_address(), "127.0.0.1:8080");
    }
}
