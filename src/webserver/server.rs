/// Axum webserver implementation
///
/// Main server lifecycle management including startup, shutdown, and graceful termination.
///
/// Security features (GUI mode):
/// - Dynamic port selection to avoid conflicts
/// - Security token validation for all requests
/// - Binding to 127.0.0.1 only (localhost, no external access)
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tower_http::compression::CompressionLayer;

use crate::{
  global,
  logger::{self, LogTag},
  webserver::{routes, state::AppState},
};

pub(crate) const DEFAULT_HOST: &str = "127.0.0.1";
pub(crate) const DEFAULT_PORT: u16 = 8080;

/// Port range for dynamic port selection in GUI mode
const DYNAMIC_PORT_START: u16 = 49152;
const DYNAMIC_PORT_END: u16 = 65535;

/// Global shutdown notifier
static SHUTDOWN_NOTIFY: once_cell::sync::Lazy<Arc<Notify>> =
  once_cell::sync::Lazy::new(|| Arc::new(Notify::new()));

/// Find an available port in the dynamic range
async fn find_available_port() -> Result<u16, String> {
  // Generate random ports to try (do RNG sync to avoid Send issues)
  let ports_to_try: Vec<u16> = {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..100)
      .map(|_| rng.gen_range(DYNAMIC_PORT_START..=DYNAMIC_PORT_END))
      .collect()
  };

  for (attempt, port) in ports_to_try.into_iter().enumerate() {
    let addr: SocketAddr = format!("{}:{}", DEFAULT_HOST, port)
      .parse()
      .map_err(|e| format!("Invalid address: {}", e))?;

    // Try to bind - if successful, the port is available
    match TcpListener::bind(&addr).await {
      Ok(listener) => {
        // Drop the listener to release the port
        drop(listener);
        logger::debug(
          LogTag::Webserver,
          &format!(
            "Found available port {} after {} attempts",
            port,
            attempt + 1
          ),
        );
        return Ok(port);
      }
      Err(_) => continue, // Port in use, try another
    }
  }

  Err("Could not find an available port after 100 attempts".to_string())
}

/// Start the webserver
///
/// In GUI mode:
/// - Uses a random available port (49152-65535)
/// - Generates a security token for request validation
/// - Only accepts requests with valid X-ScreenerBot-Token header
///
/// In CLI mode:
/// - Uses fixed port 8080
/// - No security token required (accessible via browser)
pub async fn start_server() -> Result<(), String> {
  let is_gui = global::is_gui_mode();

  // Determine port to use
  let port = if is_gui {
    // GUI mode: find available port and generate security token
    let dynamic_port = find_available_port().await?;
    global::set_webserver_port(dynamic_port);

    // Generate security token for GUI mode
    let token = global::generate_security_token();
    logger::info(
      LogTag::Webserver,
      &format!(
        "GUI mode: using dynamic port {} with security token",
        dynamic_port
      ),
    );
    logger::debug(
      LogTag::Webserver,
      &format!("Security token generated: {}...", &token[..8]),
    );

    dynamic_port
  } else {
    // CLI mode: use fixed port, no security token
    global::set_webserver_port(DEFAULT_PORT);
    logger::debug(
      LogTag::Webserver,
      &format!(
        "CLI mode: using fixed port {} (accessible via browser)",
        DEFAULT_PORT
      ),
    );
    DEFAULT_PORT
  };

  logger::debug(
    LogTag::Webserver,
    &format!("Starting webserver on {}:{}", DEFAULT_HOST, port),
  );

  // Create application state
  let state = Arc::new(AppState::new());

  // Set global app state
  crate::webserver::state::set_global_app_state(Arc::clone(&state));
  logger::debug(LogTag::Webserver, "Global app state configured");

  // Build the router
  let app = build_app(state.clone());

  // Parse bind address
  let addr: SocketAddr = format!("{}:{}", DEFAULT_HOST, port)
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
          addr, port
        )
      }
      _ => format!("Failed to bind to {}: {}", addr, e),
    }
  })?;

  logger::debug(
    LogTag::Webserver,
    &format!("Webserver listening on http://{}", addr),
  );
  logger::debug(
    LogTag::Webserver,
    &format!("API endpoints available at http://{}/api", addr),
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

  logger::debug(LogTag::Webserver, "Webserver stopped gracefully");

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

  // Add middleware layers
  // Order: security_gate -> initialization_gate -> compression
  let app = app
    .layer(axum::middleware::from_fn(
      crate::webserver::middleware::initialization_gate,
    ))
    .layer(axum::middleware::from_fn(
      crate::webserver::middleware::security_gate,
    ))
    .layer(CompressionLayer::new());

  app
}
