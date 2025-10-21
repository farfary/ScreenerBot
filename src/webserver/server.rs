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
    arguments::is_debug_webserver_enabled,
    logger::{log, LogTag},
    webserver::{routes, state::AppState},
};

pub(crate) const DEFAULT_HOST: &str = "127.0.0.1";
pub(crate) const DEFAULT_PORT: u16 = 8080;

/// Global shutdown notifier
static SHUTDOWN_NOTIFY: once_cell::sync::Lazy<Arc<Notify>> =
    once_cell::sync::Lazy::new(|| Arc::new(Notify::new()));

/// Start the webserver
///
/// This function blocks until the server is shut down
pub async fn start_server() -> Result<(), String> {
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            &format!("🌐 Starting webserver on {}:{}", DEFAULT_HOST, DEFAULT_PORT),
        );
    }

    // Create application state
    let state = Arc::new(AppState::new());

    // Set global app state
    crate::webserver::state::set_global_app_state(Arc::clone(&state));
    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "✅ Global app state configured");
    }

    // Build the router
    let app = build_app(state.clone());

    // Parse bind address
    let addr: SocketAddr = format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT)
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Create TCP listener
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            &format!("✅ Webserver listening on http://{}", addr),
        );
        log(
            LogTag::Webserver,
            "INFO",
            &format!("📊 API endpoints available at http://{}/api", addr),
        );
    }

    // Run the server with graceful shutdown
    let shutdown_signal = async {
        SHUTDOWN_NOTIFY.notified().await;
        if is_debug_webserver_enabled() {
            log(
                LogTag::Webserver,
                "INFO",
                "Received shutdown signal, stopping webserver...",
            );
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    if is_debug_webserver_enabled() {
        log(LogTag::Webserver, "INFO", "✅ Webserver stopped gracefully");
    }

    Ok(())
}

/// Trigger webserver shutdown
pub fn shutdown() {
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "INFO",
            "Triggering webserver shutdown...",
        );
    }
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
    fn test_default_address_parses() {
        assert!(format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT)
            .parse::<SocketAddr>()
            .is_ok());
    }
}
