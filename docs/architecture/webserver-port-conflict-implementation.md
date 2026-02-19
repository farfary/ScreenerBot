# Webserver Port Conflict - Implementation Guide

Quick reference for implementing the solution. See `WEBSERVER_PORT_CONFLICT_SOLUTION.md` for full design rationale.

---

## Quick Summary

**Problem**: Bot continues when port binding fails (tokio::spawn swallows errors)

**Solution**: Pre-flight port check before spawning + CLI args for port/host override

**Error Handling**: **Option A** (Pre-flight check) - Test bind before spawn, return error if fails

**Precedence**: CLI > Config > Default (for both port and host)

---

## Files to Modify

1. ✅ `src/arguments.rs` - Add CLI arg getters and validation (+80 lines)
2. ✅ `src/services/implementations/webserver_service.rs` - Add pre-flight check (+100 lines)
3. ✅ `src/webserver/server.rs` - Simplify, use global state values (-30 lines)
4. ✅ `src/run.rs` - Add early CLI validation (+15 lines)

---

## Implementation Steps

### Step 1: Add CLI Arguments (src/arguments.rs)

```rust
// Add these functions to arguments.rs

/// Get webserver port from CLI (--port <number>)
pub fn get_webserver_port() -> Option<u16> {
    get_arg_value("--port").and_then(|s| s.parse().ok())
}

/// Get webserver host from CLI (--host <address>)
pub fn get_webserver_host() -> Option<String> {
    get_arg_value("--host")
}

/// Validate port number (1-65535)
pub fn validate_port(port: u16) -> Result<(), String> {
    if port == 0 {
        // Port 0 is special: OS assigns random available port
        return Ok(());
    }

    if port < 1 || port > 65535 {
        return Err(format!(
            "Invalid port number: {}. Port must be between 1 and 65535",
            port
        ));
    }

    // Warn about privileged ports but don't error
    if port < 1024 {
        logger::warning(
            LogTag::Webserver,
            &format!(
                "Port {} requires elevated privileges. \
                 If binding fails, use port above 1024.",
                port
            ),
        );
    }

    Ok(())
}

/// Validate host address (basic format check)
pub fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("Host cannot be empty".to_string());
    }

    // Accept common patterns without deep validation
    // Let TcpListener handle actual resolution/binding
    Ok(())
}

// Update print_help() to add:
println!("WEBSERVER OPTIONS:");
println!("    --port <number>             Override webserver port (default: 8080)");
println!("    --host <address>            Override webserver host (default: 127.0.0.1)");
println!("                                Use 0.0.0.0 for remote access (VPS mode)");
println!();
println!("EXAMPLES:");
println!("    screenerbot --port 3000                      # Use port 3000");
println!("    screenerbot --host 0.0.0.0 --port 8080       # Allow remote access");
println!("    screenerbot --port 0                         # Use random available port");
```

---

### Step 2: Add Early Validation (src/run.rs)

```rust
// Add to run_bot_internal() at the start

async fn run_bot_internal(_process_lock: ProcessLock) -> Result<(), String> {
    logger::info(LogTag::System, "ScreenerBot starting up...");

    // EARLY VALIDATION: Check CLI args before anything else
    validate_cli_arguments()?;

    // ... rest of existing code unchanged
}

/// Validate CLI arguments early to fail fast
fn validate_cli_arguments() -> Result<(), String> {
    // Validate --port if provided
    if let Some(port) = crate::arguments::get_webserver_port() {
        crate::arguments::validate_port(port)?;
    }

    // Validate --host if provided
    if let Some(host) = crate::arguments::get_webserver_host() {
        crate::arguments::validate_host(&host)?;
    }

    Ok(())
}
```

---

### Step 3: Pre-flight Check in Service (src/services/implementations/webserver_service.rs)

```rust
use tokio::net::TcpListener;
use std::net::SocketAddr;
use crate::config::with_config;

#[async_trait]
impl Service for WebserverService {
    // ... existing methods unchanged ...

    async fn start(
        &mut self,
        shutdown: Arc<Notify>,
        monitor: tokio_metrics::TaskMonitor,
    ) -> Result<Vec<JoinHandle<()>>, String> {
        // GUI MODE: Use existing dynamic port logic (unchanged)
        if crate::global::is_gui_mode() {
            return start_gui_mode(shutdown, monitor).await;
        }

        // HEADLESS MODE: Resolve config with precedence + pre-flight check
        let port = resolve_port()?;
        let host = resolve_host()?;

        // PRE-FLIGHT: Test port binding BEFORE spawning
        test_port_binding(&host, port).await?;

        // Store resolved values in global state
        crate::global::set_webserver_port(port);
        crate::global::set_webserver_host(host.clone());

        // Log what we're using with source
        let source = get_config_source();
        logger::info(
            LogTag::Webserver,
            &format!("Using {}:{} from {}", host, port, source),
        );

        // NOW spawn server (we know it will succeed)
        let handle = tokio::spawn(monitor.instrument(async move {
            if let Err(e) = crate::webserver::start_server().await {
                logger::error(
                    LogTag::System,
                    &format!("Webserver failed to start: {}", e)
                );
            }
        }));

        // Brief delay for initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        log_service_notice(
            self.name(),
            "ready",
            Some(&format!("endpoint=http://{}:{}", host, port)),
            true,
        );

        Ok(vec![handle])
    }
}

/// Resolve port with precedence: CLI > Config > Default
fn resolve_port() -> Result<u16, String> {
    // 1. CLI flag (highest priority)
    if let Some(port) = crate::arguments::get_webserver_port() {
        crate::arguments::validate_port(port)?;
        return Ok(port);
    }

    // 2. Config file (if initialized)
    if crate::global::is_initialization_complete() {
        let port = with_config(|cfg| cfg.webserver.port);
        if port > 0 {
            return Ok(port);
        }
    }

    // 3. Default
    Ok(crate::webserver::DEFAULT_PORT)
}

/// Resolve host with precedence: CLI > Config > Default
fn resolve_host() -> Result<String, String> {
    // 1. CLI flag (highest priority)
    if let Some(host) = crate::arguments::get_webserver_host() {
        crate::arguments::validate_host(&host)?;
        return Ok(host);
    }

    // 2. Config file (if initialized)
    if crate::global::is_initialization_complete() {
        let host = with_config(|cfg| cfg.webserver.host.clone());
        if !host.is_empty() {
            return Ok(host);
        }
    }

    // 3. Default
    Ok(crate::webserver::DEFAULT_HOST.to_string())
}

/// Get human-readable config source for logging
fn get_config_source() -> &'static str {
    if crate::arguments::get_webserver_port().is_some()
       || crate::arguments::get_webserver_host().is_some() {
        "CLI arguments"
    } else if crate::global::is_initialization_complete() {
        "config.toml"
    } else {
        "defaults"
    }
}

/// Pre-flight check: Test port binding before spawning server
async fn test_port_binding(host: &str, port: u16) -> Result<(), String> {
    // Handle port 0 (OS-assigned random port) specially
    if port == 0 {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("Failed to bind to random port: {}", e))?;

        let actual_port = listener.local_addr()
            .map_err(|e| format!("Failed to get assigned port: {}", e))?
            .port();

        drop(listener);

        logger::info(
            LogTag::Webserver,
            &format!("Port 0 requested, OS assigned port {}", actual_port)
        );

        return Ok(());
    }

    // Normal port: try to bind
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid address {}:{} - {}", host, port, e))?;

    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            // Success! Drop listener to release port
            drop(listener);
            logger::debug(
                LogTag::Webserver,
                &format!("Pre-flight check: port {} is available", port)
            );
            Ok(())
        }
        Err(e) => {
            // Binding failed - create helpful error message
            Err(create_binding_error_message(&addr, e))
        }
    }
}

/// Create detailed error message for binding failures
fn create_binding_error_message(addr: &SocketAddr, error: std::io::Error) -> String {
    match error.kind() {
        std::io::ErrorKind::AddrInUse => {
            format!(
                "Failed to start webserver\n\
                 \n\
                 Cannot bind to {} - Address already in use\n\
                 \n\
                 This usually means:\n\
                 • Another instance of ScreenerBot is running\n\
                 • Another application is using port {}\n\
                 \n\
                 Solutions:\n\
                 1. Stop other instances:\n\
                    ps aux | grep screenerbot | grep -v grep\n\
                    pkill -f screenerbot\n\
                 \n\
                 2. Use a different port:\n\
                    screenerbot --port 3000\n\
                 \n\
                 3. Edit config.toml:\n\
                    [webserver]\n\
                    port = 3000\n\
                 \n\
                 For help: screenerbot --help",
                addr,
                addr.port()
            )
        }
        std::io::ErrorKind::PermissionDenied => {
            format!(
                "Failed to start webserver\n\
                 \n\
                 Cannot bind to {} - Permission denied\n\
                 \n\
                 Port {} requires elevated privileges.\n\
                 \n\
                 Solutions:\n\
                 • Use a port above 1024: screenerbot --port 8080\n\
                 • Configure port forwarding (recommended for production)\n\
                 • Run with elevated privileges (not recommended)\n\
                 \n\
                 For help: screenerbot --help",
                addr,
                addr.port()
            )
        }
        _ => {
            format!(
                "Failed to start webserver\n\
                 \n\
                 Cannot bind to {}: {}\n\
                 \n\
                 For help: screenerbot --help",
                addr, error
            )
        }
    }
}

/// GUI mode startup (existing logic, extracted for clarity)
async fn start_gui_mode(
    shutdown: Arc<Notify>,
    monitor: tokio_metrics::TaskMonitor,
) -> Result<Vec<JoinHandle<()>>, String> {
    // Ignore CLI args in GUI mode (log if provided)
    if crate::arguments::get_webserver_port().is_some()
       || crate::arguments::get_webserver_host().is_some() {
        logger::info(
            LogTag::Webserver,
            "GUI mode: CLI port/host arguments ignored (using dynamic port for security)"
        );
    }

    // ... existing GUI mode logic unchanged ...
    // (dynamic port selection, security token generation, etc.)
}
```

---

### Step 4: Simplify Server (src/webserver/server.rs)

```rust
// BEFORE: server.rs had config resolution logic mixed with binding
// AFTER: Just use values from global state (already resolved by service)

pub async fn start_server() -> Result<(), String> {
    let is_gui = global::is_gui_mode();

    // Get port and host from global state (already resolved)
    let port = global::get_webserver_port();
    let host = global::get_webserver_host();

    // Validate we have values (should always be set by service)
    if port == 0 || host.is_empty() {
        return Err("Webserver port/host not initialized".to_string());
    }

    logger::debug(
        LogTag::Webserver,
        &format!("Starting webserver on {}:{}", host, port),
    );

    // Create application state
    let state = Arc::new(AppState::new());
    crate::webserver::state::set_global_app_state(Arc::clone(&state));

    // Build router
    let app = build_app(state.clone());

    // Parse bind address
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // Bind listener (should succeed since we pre-flight tested)
    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        // This should rarely happen (race condition between test and actual bind)
        format!("Failed to bind to {}: {}", addr, e)
    })?;

    logger::debug(
        LogTag::Webserver,
        &format!("Webserver listening on http://{}", addr),
    );

    // Run server with graceful shutdown
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

// ... rest of file unchanged ...
```

---

## Testing Checklist

```bash
# Test 1: Basic port conflict
terminal1$ cargo run --bin screenerbot
terminal2$ cargo run --bin screenerbot  # Should exit with error

# Test 2: CLI override
$ cargo run --bin screenerbot -- --port 3000
# Check: curl localhost:3000/api/status

# Test 3: Port validation
$ cargo run --bin screenerbot -- --port 99999
# Should exit with validation error

# Test 4: Host validation
$ cargo run --bin screenerbot -- --host 999.999.999.999
# Should exit with error

# Test 5: Port 0 (random)
$ cargo run --bin screenerbot -- --port 0
# Check logs for "OS assigned port XXXXX"

# Test 6: Privileged port (without sudo)
$ cargo run --bin screenerbot -- --port 80
# Should show warning, then fail with PermissionDenied

# Test 7: GUI mode ignores CLI
$ cargo run --bin screenerbot -- --gui --port 3000
# Check logs for "CLI arguments ignored"

# Test 8: Config precedence
# Edit config.toml: port = 5000
$ cargo run --bin screenerbot
# Should use port 5000, log shows "from config.toml"

# Test 9: Remote access
$ cargo run --bin screenerbot -- --host 0.0.0.0 --port 8080
# From remote: curl <VPS_IP>:8080/api/status

# Test 10: Pre-initialization
$ rm data/config.toml
$ cargo run --bin screenerbot
# Should use defaults, show init screen
```

---

## Key Points

1. **Pre-flight check is critical**: Test port binding BEFORE spawning server
2. **Precedence is clear**: CLI > Config > Default (document this)
3. **GUI mode exception**: Always uses dynamic port (ignore CLI args)
4. **Error messages matter**: Users need actionable solutions
5. **Zero breaking changes**: Existing users see no difference
6. **Port 0 is special**: OS assigns random available port (useful for testing)
7. **Validation happens early**: Fail fast in run.rs before services start

---

## Common Mistakes to Avoid

❌ Don't spawn server first then check - race condition
✅ Do pre-flight test then spawn

❌ Don't override GUI mode port with CLI - breaks security
✅ Do ignore CLI args in GUI mode

❌ Don't use generic error messages - users get confused
✅ Do provide specific error with solutions

❌ Don't forget to drop listener after test - port stays locked
✅ Do drop immediately after bind test

❌ Don't validate hostname resolution - blocks on DNS
✅ Do basic format check only, let TcpListener resolve

---

## Performance Notes

- Pre-flight test adds ~1-5ms to startup (negligible)
- No ongoing performance impact
- One extra bind/unbind during startup only
- No additional memory allocations

---

## Rollback Plan

If issues arise after deployment:

1. **Immediate**: Revert the 4 file changes
2. **Short-term**: Add feature flag to disable pre-flight check
3. **Long-term**: Fix issue and re-deploy

Feature flag approach:

```rust
// In webserver_service.rs
if !cfg!(feature = "skip-port-test") {
    test_port_binding(&host, port).await?;
}
```

Build without test: `cargo build --features skip-port-test`

---

## Documentation Updates Required

1. **CLI help**: Add --port and --host examples
2. **Website docs**: Add troubleshooting section for port conflicts
3. **README**: Update examples with CLI args
4. **Config docs**: Explain precedence order
5. **Deployment guide**: Add VPS setup with 0.0.0.0

---

## Timeline

- **Day 1**: Implement arguments.rs + run.rs validation
- **Day 2**: Implement webserver_service.rs pre-flight check
- **Day 3**: Testing on all platforms + edge cases
- **Day 4**: Documentation updates + website
- **Day 5**: Release + monitoring

**Total**: 5 days with buffer for testing and docs.

---

## Success Metrics

After deployment, track:

- Port conflict errors (should be near zero after fix)
- CLI argument usage (--port, --host)
- User-reported issues (should decrease)
- Startup success rate (should be 100%)

---

## Questions?

See full design rationale in: `WEBSERVER_PORT_CONFLICT_SOLUTION.md`
