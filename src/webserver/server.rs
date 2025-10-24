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
    logger::{self, LogTag},
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
    logger::debug(
        LogTag::Webserver,
        &format!("🌐 Starting webserver on {}:{}", DEFAULT_HOST, DEFAULT_PORT),
    );

    // Create application state
    let state = Arc::new(AppState::new());

    // Set global app state
    crate::webserver::state::set_global_app_state(Arc::clone(&state));
    logger::debug(LogTag::Webserver, "✅ Global app state configured");

    // Build the router
    let app = build_app(state.clone());

    // Parse bind address
    let addr: SocketAddr = format!("{}:{}", DEFAULT_HOST, DEFAULT_PORT)
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Create TCP listener
    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        // Provide helpful error message for common cases
        match e.kind() {
            std::io::ErrorKind::AddrInUse => {
                format!(
                    "Failed to bind to {}: Address already in use\n\
                     \n\
                     This usually means another instance of ScreenerBot is running.\n\
                     The process lock should have prevented this - please report this issue.\n\
                     \n\
                     To verify and stop other instances:\n\
                       1. Check: ps aux | grep screenerbot | grep -v grep\n\
                       2. Stop: pkill -f screenerbot\n\
                       3. Verify: ps aux | grep screenerbot | grep -v grep",
                    addr
                )
            }
            std::io::ErrorKind::PermissionDenied => {
                format!(
                    "Failed to bind to {}: Permission denied\n\
                     \n\
                     Port {} requires elevated privileges on this system.\n\
                     Consider using a port above 1024 or running with appropriate permissions.",
                    addr, DEFAULT_PORT
                )
            }
            _ => format!("Failed to bind to {}: {}", addr, e),
        }
    })?;

    logger::debug(
        LogTag::Webserver,
        &format!("✅ Webserver listening on http://{}", addr),
    );
    logger::debug(
        LogTag::Webserver,
        &format!("📊 API endpoints available at http://{}/api", addr),
    );

    // Run the server with graceful shutdown
    let shutdown_signal = async {
        SHUTDOWN_NOTIFY.notified().await;
        logger::debug(
            LogTag::Webserver,
            "Received shutdown signal, stopping webserver...",
        );
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    logger::debug(LogTag::Webserver, "✅ Webserver stopped gracefully");

    Ok(())
}

/// Trigger webserver shutdown
pub fn shutdown() {
    logger::debug(LogTag::Webserver, "Triggering webserver shutdown...");
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
