# Webserver Port Conflict - Quick Reference Card

One-page reference for implementing the fix.

---

## The Problem (30 seconds)

```rust
// BEFORE (broken):
async fn start(...) -> Result<...> {
    tokio::spawn(async {
        // Error happens here but never propagates back
        crate::webserver::start_server().await?; // ❌ Error swallowed
    });
    Ok(vec![handle]) // ✅ Always returns Ok!
}
```

**Result**: Bot continues running with broken webserver.

---

## The Solution (30 seconds)

```rust
// AFTER (fixed):
async fn start(...) -> Result<...> {
    let port = resolve_port()?;
    let host = resolve_host()?;

    // TEST FIRST (synchronous error propagation)
    test_port_binding(&host, port).await?; // ✅ Error returns here

    // Only spawn if test succeeded
    tokio::spawn(async { ... });
    Ok(vec![handle])
}
```

**Result**: Bot exits immediately on port conflict with clear error.

---

## Implementation Checklist

```
□ arguments.rs
  □ Add get_webserver_port() -> Option<u16>
  □ Add get_webserver_host() -> Option<String>
  □ Add validate_port(u16) -> Result<(), String>
  □ Add validate_host(&str) -> Result<(), String>
  □ Update print_help() with new flags

□ run.rs
  □ Add validate_cli_arguments() -> Result<(), String>
  □ Call it at start of run_bot_internal()

□ webserver_service.rs
  □ Add resolve_port() -> Result<u16, String>
  □ Add resolve_host() -> Result<String, String>
  □ Add test_port_binding(&str, u16) -> Result<(), String>
  □ Add create_binding_error_message(...) -> String
  □ Update start() method with pre-flight check
  □ Extract GUI logic to start_gui_mode()

□ server.rs
  □ Remove config resolution logic
  □ Use global::get_webserver_port()
  □ Use global::get_webserver_host()

□ Testing
  □ Run all 10 test scenarios
  □ Test on macOS/Linux/Windows
  □ Verify exit codes
  □ Check error messages

□ Documentation
  □ Update CLI help
  □ Update website docs
  □ Add troubleshooting section
```

---

## Code Snippets (Copy-Paste Ready)

### 1. arguments.rs

```rust
/// Get webserver port from CLI (--port <number>)
pub fn get_webserver_port() -> Option<u16> {
    get_arg_value("--port").and_then(|s| s.parse().ok())
}

/// Get webserver host from CLI (--host <address>)
pub fn get_webserver_host() -> Option<String> {
    get_arg_value("--host")
}

/// Validate port number
pub fn validate_port(port: u16) -> Result<(), String> {
    if port == 0 { return Ok(()); } // Special: random port
    if port < 1 || port > 65535 {
        return Err(format!("Invalid port: {}. Must be 1-65535", port));
    }
    if port < 1024 {
        logger::warning(LogTag::Webserver,
            &format!("Port {} may require privileges", port));
    }
    Ok(())
}

/// Validate host address
pub fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty() {
        return Err("Host cannot be empty".to_string());
    }
    Ok(())
}
```

### 2. run.rs

```rust
async fn run_bot_internal(_process_lock: ProcessLock) -> Result<(), String> {
    logger::info(LogTag::System, "ScreenerBot starting up...");

    // EARLY VALIDATION: Check CLI args first
    validate_cli_arguments()?;

    // ... rest of code unchanged
}

fn validate_cli_arguments() -> Result<(), String> {
    if let Some(port) = crate::arguments::get_webserver_port() {
        crate::arguments::validate_port(port)?;
    }
    if let Some(host) = crate::arguments::get_webserver_host() {
        crate::arguments::validate_host(&host)?;
    }
    Ok(())
}
```

### 3. webserver_service.rs - Pre-flight Test

```rust
/// Pre-flight check: Test port binding before spawning
async fn test_port_binding(host: &str, port: u16) -> Result<(), String> {
    use tokio::net::TcpListener;
    use std::net::SocketAddr;

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;

    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            drop(listener);
            Ok(())
        }
        Err(e) => Err(create_binding_error_message(&addr, e))
    }
}
```

### 4. webserver_service.rs - Config Resolution

```rust
/// Resolve port: CLI > Config > Default
fn resolve_port() -> Result<u16, String> {
    if let Some(port) = crate::arguments::get_webserver_port() {
        crate::arguments::validate_port(port)?;
        return Ok(port);
    }
    if crate::global::is_initialization_complete() {
        let port = with_config(|cfg| cfg.webserver.port);
        if port > 0 { return Ok(port); }
    }
    Ok(crate::webserver::DEFAULT_PORT)
}

/// Resolve host: CLI > Config > Default
fn resolve_host() -> Result<String, String> {
    if let Some(host) = crate::arguments::get_webserver_host() {
        crate::arguments::validate_host(&host)?;
        return Ok(host);
    }
    if crate::global::is_initialization_complete() {
        let host = with_config(|cfg| cfg.webserver.host.clone());
        if !host.is_empty() { return Ok(host); }
    }
    Ok(crate::webserver::DEFAULT_HOST.to_string())
}
```

---

## Test Commands

```bash
# 1. Port conflict
terminal1$ cargo run --bin screenerbot
terminal2$ cargo run --bin screenerbot  # Should exit with error

# 2. CLI override
$ cargo run --bin screenerbot -- --port 3000
$ curl localhost:3000/api/status

# 3. Invalid port
$ cargo run --bin screenerbot -- --port 99999

# 4. Port 0 (random)
$ cargo run --bin screenerbot -- --port 0

# 5. Privileged port
$ cargo run --bin screenerbot -- --port 80

# 6. Remote access
$ cargo run --bin screenerbot -- --host 0.0.0.0

# 7. GUI ignores CLI
$ cargo run --bin screenerbot -- --gui --port 3000

# 8. Help text
$ cargo run --bin screenerbot -- --help
```

---

## Error Message Template

```rust
match error.kind() {
    std::io::ErrorKind::AddrInUse => {
        format!(
            "Failed to start webserver\n\
             \n\
             Cannot bind to {} - Address already in use\n\
             \n\
             Solutions:\n\
             1. Stop other instances: pkill -f screenerbot\n\
             2. Use different port: screenerbot --port 3000\n\
             3. Edit config.toml: [webserver] port = 3000",
            addr
        )
    }
    std::io::ErrorKind::PermissionDenied => {
        format!(
            "Failed to start webserver\n\
             \n\
             Port {} requires elevated privileges.\n\
             \n\
             Solutions:\n\
             • Use port above 1024: screenerbot --port 8080\n\
             • Configure port forwarding (recommended)",
            addr.port()
        )
    }
    _ => format!("Failed to bind to {}: {}", addr, error)
}
```

---

## Precedence Logic (Decision Tree)

```
GUI mode?
├─ Yes → Dynamic port (ignore CLI)
└─ No  → Headless mode:
         ├─ --port flag? → Use CLI
         ├─ config.port? → Use config
         └─ else         → Use 8080
```

---

## Common Mistakes

❌ Test port AFTER spawn → Race condition
✅ Test port BEFORE spawn → Error propagates

❌ Override GUI port with CLI → Breaks security
✅ Ignore CLI in GUI mode → Security preserved

❌ Generic error message → User confused
✅ Detailed error + solutions → User can fix

❌ Forget to drop test listener → Port locked
✅ Drop immediately after test → Port released

---

## Key Insights

1. **Pre-flight pattern**: Test synchronously before spawning async task
2. **Error propagation**: Return Result from pre-flight, not from spawned task
3. **Config precedence**: CLI > Config > Default (but GUI always ignores CLI)
4. **Fail fast**: Validate early in run.rs before services start
5. **Helpful errors**: Include context, cause, and actionable solutions

---

## Performance

- Pre-flight test: ~1-5ms (one bind/unbind)
- No ongoing impact
- Zero memory overhead
- One extra socket operation at startup only

---

## Files & Lines

```
arguments.rs           +80 lines
run.rs                 +15 lines
webserver_service.rs   +100 lines
server.rs              -30 lines (simplified)
───────────────────────────────────
Total                  +165 net lines
```

---

## Full Documentation

For complete details, see:

- `WEBSERVER_PORT_CONFLICT_SOLUTION.md` (15-section design)
- `WEBSERVER_PORT_CONFLICT_IMPLEMENTATION.md` (pseudocode guide)
- `WEBSERVER_PORT_CONFLICT_FLOW.md` (visual diagrams)
- `WEBSERVER_PORT_CONFLICT_SUMMARY.md` (executive summary)

---

## Timeline

- Day 1: arguments.rs + run.rs
- Day 2: webserver_service.rs + server.rs
- Day 3: Testing (all platforms)
- Day 4: Documentation
- Day 5: Release

---

**Status**: Ready for implementation
**Risk**: Low
**Impact**: High (fixes critical bug)
**Breaking changes**: None
