/// Axum webserver implementation
///
/// Main server lifecycle management including startup, shutdown, and graceful termination.
///
/// Security features (GUI mode):
/// - Dynamic port selection to avoid conflicts
/// - Security token validation for all requests
/// - Binding to 127.0.0.1 only (localhost, no external access)
///
/// Headless/CLI mode:
/// - Uses port from config (default 8080)
/// - Uses host from config (default 127.0.0.1, use 0.0.0.0 for remote access)
/// - No security token required (accessible via browser)
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tower_http::compression::CompressionLayer;

use crate::{
  config::with_config,
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
/// - Always binds to 127.0.0.1 (localhost only) for security
///
/// In CLI/Headless mode:
/// - Uses port from config.webserver.port (default 8080)
/// - Uses host from config.webserver.host (default 127.0.0.1, use 0.0.0.0 for remote)
/// - No security token required (accessible via browser)
pub async fn start_server(
  port_override: Option<u16>,
  host_override: Option<String>,
) -> Result<(), String> {
  let is_gui = global::is_gui_mode();

  // Get config values for headless mode (use defaults if config not loaded yet)
  let (config_port, config_host) = if crate::global::is_initialization_complete() {
    with_config(|cfg| (cfg.webserver.port, cfg.webserver.host.clone()))
  } else {
    // Use defaults during initialization (will fall back to defaults below anyway)
    (0, String::new())
  };

  // Determine port and host to use
  let (port, host) = if is_gui {
    // GUI mode: find available port, always bind to localhost for security
    let dynamic_port = find_available_port().await?;
    global::set_webserver_port(dynamic_port);
    global::set_webserver_host(DEFAULT_HOST.to_string());

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

    (dynamic_port, DEFAULT_HOST.to_string())
  } else {
    // CLI/Headless mode: implement precedence logic (CLI > config > default)
    let (port, port_source) = if let Some(cli_port) = port_override {
      (cli_port, "CLI")
    } else if config_port > 0 {
      (config_port, "config")
    } else {
      (DEFAULT_PORT, "default")
    };

    let (host, host_source) = if let Some(cli_host) = host_override {
      (cli_host, "CLI")
    } else if !config_host.is_empty() {
      (config_host, "config")
    } else {
      (DEFAULT_HOST.to_string(), "default")
    };
    
    global::set_webserver_port(port);
    global::set_webserver_host(host.clone());
    
    // Log effective values with source information
    let source_info = if port_source == host_source {
      format!("source: {}", port_source)
    } else {
      format!("port source: {}, host source: {}", port_source, host_source)
    };

    if host == "0.0.0.0" {
      logger::info(
        LogTag::Webserver,
        &format!(
          "Starting webserver on {}:{} (accessible from any network interface) [{}]",
          host, port, source_info
        ),
      );
    } else {
      logger::info(
        LogTag::Webserver,
        &format!(
          "Starting webserver on {}:{} (localhost only) [{}]",
          host, port, source_info
        ),
      );
    }

    (port, host)
  };

  logger::debug(
    LogTag::Webserver,
    &format!("Starting webserver on {}:{}", host, port),
  );

  // Create application state
  let state = Arc::new(AppState::new());

  // Set global app state
  crate::webserver::state::set_global_app_state(Arc::clone(&state));
  logger::debug(LogTag::Webserver, "Global app state configured");

  // Build the router
  let app = build_app(state.clone());

  // Parse bind address
  let addr: SocketAddr = format!("{}:{}", host, port)
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
  // Order matters - layers are applied in reverse order (last added runs first):
  // 1. Compression runs first (outermost)
  // 2. Security gate checks token (GUI mode only)
  // 3. Auth gate checks session cookie (headless mode only)
  // 4. Initialization gate checks init status  
  // 5. Cache control adds no-cache headers (innermost, runs last on response)
  let app = app
    .layer(axum::middleware::from_fn(
      crate::webserver::middleware::cache_control,
    ))
    .layer(axum::middleware::from_fn(
      crate::webserver::middleware::initialization_gate,
    ))
    .layer(axum::middleware::from_fn(
      crate::webserver::middleware::auth_gate,
    ))
    .layer(axum::middleware::from_fn(
      crate::webserver::middleware::security_gate,
    ))
    .layer(CompressionLayer::new());

  app
}

/// Test port binding before spawning background task
///
/// This pre-flight check ensures the port is available before the webserver
/// service spawns the background task. If binding fails here, the error is
/// propagated to ServiceManager, which stops initialization immediately.
pub async fn test_port_binding(
  port_override: Option<u16>,
  host_override: Option<String>,
) -> Result<(), String> {
  logger::debug(
    LogTag::Webserver,
    "[TEST-BIND] test_port_binding() entry",
  );
  
  let is_gui = global::is_gui_mode();
  
  logger::debug(
    LogTag::Webserver,
    &format!("[TEST-BIND] Checking GUI mode: is_gui={}", is_gui),
  );

  if is_gui {
    // GUI mode will find its own port dynamically, skip pre-flight check
    logger::debug(
      LogTag::Webserver,
      "[TEST-BIND] SKIPPING pre-flight check (GUI mode uses dynamic port selection)",
    );
    return Ok(());
  }
  
  logger::debug(
    LogTag::Webserver,
    "[TEST-BIND] Running pre-flight check (CLI/headless mode)",
  );

  // Get config values (use defaults if config not loaded yet)
  let init_complete = crate::global::is_initialization_complete();
  logger::debug(
    LogTag::Webserver,
    &format!("[TEST-BIND] Initialization complete: {}", init_complete),
  );
  
  let (config_port, config_host) = if init_complete {
    with_config(|cfg| (cfg.webserver.port, cfg.webserver.host.clone()))
  } else {
    (0, String::new())
  };
  
  logger::debug(
    LogTag::Webserver,
    &format!(
      "[TEST-BIND] Config values: port={}, host={}",
      config_port,
      if config_host.is_empty() { "<empty>" } else { &config_host }
    ),
  );

  // Use same precedence logic as start_server (CLI > config > default)
  let effective_port = port_override
    .or_else(|| if config_port > 0 { Some(config_port) } else { None })
    .unwrap_or(DEFAULT_PORT);

  let effective_host = host_override
    .or_else(|| if !config_host.is_empty() { Some(config_host) } else { None })
    .unwrap_or_else(|| DEFAULT_HOST.to_string());

  let addr = format!("{}:{}", effective_host, effective_port);
  
  logger::debug(
    LogTag::Webserver,
    &format!(
      "[TEST-BIND] Resolved address: {} (port={}, host={})",
      addr, effective_port, effective_host
    ),
  );

  // Try to bind and immediately drop the listener
  logger::debug(
    LogTag::Webserver,
    &format!("[TEST-BIND] Attempting TcpListener::bind({})...", addr),
  );
  
  match TcpListener::bind(&addr).await {
    Ok(listener) => {
      logger::debug(
        LogTag::Webserver,
        &format!("[TEST-BIND] ✅ Bind SUCCESSFUL for {}", addr),
      );
      drop(listener);
      logger::debug(
        LogTag::Webserver,
        &format!("[TEST-BIND] Listener dropped, port {} released", effective_port),
      );
      logger::debug(
        LogTag::System,
        &format!("Pre-flight port check passed for {}", addr),
      );
      Ok(())
    }
    Err(e) => {
      logger::error(
        LogTag::Webserver,
        &format!(
          "[TEST-BIND] ❌ Bind FAILED for {}: kind={:?}, error={}",
          addr,
          e.kind(),
          e
        ),
      );
      
      // Provide helpful error messages for common cases
      let error_msg = match e.kind() {
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
            addr, effective_port
          )
        }
        _ => format!("Failed to bind to {}: {}", addr, e),
      };

      logger::error(LogTag::System, &error_msg);
      Err(error_msg)
    }
  }
}
