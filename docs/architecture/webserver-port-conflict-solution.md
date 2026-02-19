# Webserver Port Conflict Solution - Design Document

## Executive Summary

**Problem**: Bot continues running when webserver port is already in use. `tokio::spawn()` swallows errors, service.start() returns Ok even when binding fails.

**Solution**: Implement synchronous pre-flight port check before spawning, add CLI arguments for --port and --host with proper validation, and ensure bot exits immediately on port conflict.

**Impact**: Critical bug fix that prevents silent failures and improves user experience.

---

## 1. Design Decision: Error Handling Approach

### Recommended: **Option A - Pre-flight Check Before Spawn**

**Why this is the best approach:**

1. **Immediate feedback**: User knows instantly if port is unavailable
2. **Clean error path**: No need for channels or complex synchronization
3. **Matches architecture**: Service.start() already returns Result<\_, String>
4. **No breaking changes**: Keeps existing spawn pattern intact
5. **Simple implementation**: Test bind → drop listener → spawn if successful

**Implementation strategy:**

```rust
// In webserver_service.rs start() method:
async fn start(&mut self, ...) -> Result<Vec<JoinHandle<()>>, String> {
    // 1. Get port/host from CLI args or config
    let (port, host) = resolve_webserver_config();

    // 2. PRE-FLIGHT CHECK: Try to bind BEFORE spawning
    test_port_binding(&host, port).await?; // Returns error if port in use

    // 3. Only spawn if test succeeded
    let handle = tokio::spawn(monitor.instrument(async move {
        // This will succeed because we already tested
        crate::webserver::start_server().await
    }));

    Ok(vec![handle])
}
```

**Why NOT Option B or C:**

- **Option B (Synchronous start)**: Would block ServiceManager startup sequence, violating architecture
- **Option C (Channel-based)**: Adds complexity with channels, timeouts, and race conditions

---

## 2. CLI Arguments Design

### Arguments Structure

Add two new CLI arguments in `src/arguments.rs`:

```rust
/// Get webserver port from CLI (--port <number>)
pub fn get_webserver_port() -> Option<u16> {
    get_arg_value("--port").and_then(|s| s.parse().ok())
}

/// Get webserver host from CLI (--host <address>)
pub fn get_webserver_host() -> Option<String> {
    get_arg_value("--host")
}
```

### Validation Rules

**Port validation:**

- Must be 1-65535 (valid port range)
- Ports 1-1023: Warn that elevated privileges may be needed
- Port 0: Special case - use OS-assigned random available port
- Invalid/unparseable: Error and exit

**Host validation:**

- Must be valid IP address or hostname
- Common values: `127.0.0.1`, `0.0.0.0`, `localhost`
- Invalid format: Error and exit

### Precedence Order (Config Resolution)

```
Priority (highest to lowest):
1. CLI --port flag          → Overrides everything
2. Config file port         → From config.toml
3. Default value (8080)     → Hardcoded in schema

Same for host:
1. CLI --host flag          → Overrides everything
2. Config file host         → From config.toml
3. Default value (127.0.0.1) → Hardcoded in schema
```

**GUI mode exception**: CLI args are IGNORED in GUI mode (always uses dynamic port for security).

### Help Text Additions

```
WEBSERVER OPTIONS:
    --port <number>             Override webserver port (default: 8080)
    --host <address>            Override webserver host (default: 127.0.0.1)
                                Use 0.0.0.0 for remote access (VPS mode)

EXAMPLES:
    screenerbot --port 3000                      # Use port 3000
    screenerbot --host 0.0.0.0 --port 8080       # Allow remote access
    screenerbot --port 0                         # Use random available port
```

---

## 3. Code Changes Required

### File: `src/arguments.rs`

**Changes:**

1. Add `get_webserver_port()` function
2. Add `get_webserver_host()` function
3. Add validation helper: `validate_port(port: u16) -> Result<(), String>`
4. Add validation helper: `validate_host(host: &str) -> Result<(), String>`
5. Update `print_help()` to include new options
6. Update examples section

**Estimated lines**: +80 lines

---

### File: `src/services/implementations/webserver_service.rs`

**Changes:**

1. Add `resolve_webserver_config()` helper that implements precedence logic
2. Add `test_port_binding(host: &str, port: u16) -> Result<(), String>` pre-flight check
3. Update `start()` method to:
   - Resolve config from CLI/config/defaults
   - Call pre-flight test BEFORE spawning
   - Return error immediately if test fails
   - Store resolved values in global state
4. Update logging to show actual vs configured values

**Estimated lines**: +100 lines

**Pseudocode:**

```rust
async fn start(&mut self, shutdown: Arc<Notify>, monitor: TaskMonitor)
    -> Result<Vec<JoinHandle<()>>, String>
{
    // Skip all this in GUI mode (uses dynamic port)
    if crate::global::is_gui_mode() {
        // Existing GUI logic unchanged
        return existing_gui_start();
    }

    // HEADLESS MODE: Resolve config with precedence
    let port = resolve_port()?; // CLI > Config > Default
    let host = resolve_host()?; // CLI > Config > Default

    // PRE-FLIGHT: Test port binding BEFORE spawning
    test_port_binding(&host, port).await?;

    // Store resolved values
    crate::global::set_webserver_port(port);
    crate::global::set_webserver_host(host.clone());

    // Log what we're using
    logger::info(LogTag::Webserver,
        &format!("Using port {} from {}", port, source));

    // NOW spawn (we know it will succeed)
    let handle = tokio::spawn(monitor.instrument(async move {
        if let Err(e) = crate::webserver::start_server().await {
            logger::error(LogTag::System,
                &format!("Webserver failed: {}", e));
        }
    }));

    // Brief delay for initialization
    tokio::time::sleep(Duration::from_millis(200)).await;

    log_service_notice(self.name(), "ready",
        Some(&format!("endpoint=http://{}:{}", host, port)), true);

    Ok(vec![handle])
}

// Helper: Resolve port with precedence
fn resolve_port() -> Result<u16, String> {
    // 1. CLI flag (highest priority)
    if let Some(port) = crate::arguments::get_webserver_port() {
        validate_port(port)?;
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

// Helper: Resolve host with precedence
fn resolve_host() -> Result<String, String> {
    // 1. CLI flag
    if let Some(host) = crate::arguments::get_webserver_host() {
        validate_host(&host)?;
        return Ok(host);
    }

    // 2. Config file
    if crate::global::is_initialization_complete() {
        let host = with_config(|cfg| cfg.webserver.host.clone());
        if !host.is_empty() {
            return Ok(host);
        }
    }

    // 3. Default
    Ok(crate::webserver::DEFAULT_HOST.to_string())
}

// Helper: Pre-flight port test
async fn test_port_binding(host: &str, port: u16) -> Result<(), String> {
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .map_err(|e| format!("Invalid address {}:{} - {}", host, port, e))?;

    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            drop(listener); // Release immediately
            Ok(())
        }
        Err(e) => {
            Err(create_binding_error_message(addr, e))
        }
    }
}
```

---

### File: `src/webserver/server.rs`

**Changes:**

1. Update `start_server()` to use already-resolved port/host from global state
2. Remove internal port resolution logic (now done in service layer)
3. Keep existing bind error handling but simplified
4. Update comments to reflect new flow

**Estimated lines**: -30 lines (simplification)

**Key insight**: Config resolution moves UP to service layer, server.rs becomes dumber.

---

### File: `src/run.rs`

**Changes:**

1. Add CLI validation early in `run_bot_internal()`
2. Exit immediately if validation fails
3. No other changes (error propagation already works)

**Estimated lines**: +15 lines

**Pseudocode:**

```rust
async fn run_bot_internal(_process_lock: ProcessLock) -> Result<(), String> {
    logger::info(LogTag::System, "ScreenerBot starting up...");

    // EARLY VALIDATION: Check CLI args before anything else
    validate_cli_arguments()?; // Returns error if invalid

    // ... rest of existing startup logic
}

fn validate_cli_arguments() -> Result<(), String> {
    // Validate --port if provided
    if let Some(port) = crate::arguments::get_webserver_port() {
        validate_port(port)?;
    }

    // Validate --host if provided
    if let Some(host) = crate::arguments::get_webserver_host() {
        validate_host(&host)?;
    }

    Ok(())
}
```

---

## 4. Error Message Design

### When Port is Already in Use

**Exit code**: `1` (standard error exit)

**Error message** (displayed to user):

```
ERROR: Failed to start webserver

Cannot bind to 127.0.0.1:8080 - Address already in use

This usually means:
  • Another instance of ScreenerBot is running
  • Another application is using port 8080

Solutions:
  1. Stop other instances:
     ps aux | grep screenerbot | grep -v grep
     pkill -f screenerbot

  2. Use a different port:
     screenerbot --port 3000

  3. Edit config.toml:
     [webserver]
     port = 3000

For help: screenerbot --help
```

### When Port Number is Invalid

**Error message:**

```
ERROR: Invalid port number

Port must be between 1 and 65535, got: <value>

Use: screenerbot --port <valid_port>
Example: screenerbot --port 8080
```

### When Port Requires Privileges (1-1023)

**Warning message** (not error, still attempts):

```
WARNING: Port 80 may require elevated privileges

Ports below 1024 typically require administrator/root access.
If binding fails, try:
  • Use port above 1024: --port 8080
  • Run with sudo (not recommended)
  • Configure port forwarding (recommended for production)
```

### When Host is Invalid

**Error message:**

```
ERROR: Invalid host address

Host must be a valid IP address or hostname, got: '<value>'

Common values:
  • 127.0.0.1  - Localhost only (secure, local access)
  • 0.0.0.0    - All interfaces (VPS/remote access)
  • localhost  - Hostname for local machine

Use: screenerbot --host <address>
Example: screenerbot --host 0.0.0.0 --port 8080
```

---

## 5. Edge Cases & Solutions

### Edge Case 1: Port 0 (Random Available Port)

**Behavior**: Use OS-assigned ephemeral port

**Implementation:**

```rust
if port == 0 {
    // Bind to :0 to get OS-assigned port
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let actual_port = listener.local_addr()?.port();
    drop(listener);

    logger::info(LogTag::Webserver,
        &format!("Port 0 requested, OS assigned port {}", actual_port));

    crate::global::set_webserver_port(actual_port);
    return Ok(actual_port);
}
```

**Use case**: Useful for running multiple instances on same machine for testing.

---

### Edge Case 2: Port Below 1024 (Privileged Ports)

**Behavior**: Show warning but attempt binding

**Implementation:**

```rust
if port < 1024 && port > 0 {
    logger::warning(LogTag::Webserver,
        &format!("Port {} requires elevated privileges. \
                  If binding fails, use port above 1024.", port));
}

// Proceed with normal binding (will fail with PermissionDenied if lacking privileges)
```

**Error handling**: If bind fails with `PermissionDenied`, show specific help message.

---

### Edge Case 3: Invalid Hostname Resolution

**Scenario**: User provides hostname that doesn't resolve (e.g., `--host myserver.local`)

**Behavior**:

- Validate format only (not DNS resolution)
- Let TcpListener handle actual resolution
- If bind fails, show error with hostname included

**Why**: Avoid blocking startup on DNS lookups.

---

### Edge Case 4: Both CLI and Config Set (Precedence)

**Scenario**:

```bash
# config.toml has port = 9000
screenerbot --port 8080
```

**Behavior**: CLI wins (port 8080 used)

**Logging:**

```
INFO [Webserver] Using port 8080 from CLI (config.toml has 9000)
```

**Implementation:**

```rust
let source = if cli_port.is_some() {
    "CLI argument"
} else if config_port > 0 {
    "config.toml"
} else {
    "default"
};

logger::info(LogTag::Webserver,
    &format!("Using port {} from {}", final_port, source));
```

---

### Edge Case 5: GUI Mode with CLI Args

**Scenario**:

```bash
screenerbot --gui --port 3000
```

**Behavior**: CLI args are IGNORED (GUI always uses dynamic port)

**Logging:**

```
INFO [Webserver] GUI mode: CLI port/host arguments ignored (using dynamic port for security)
DEBUG [Webserver] GUI mode: found available port 54321
```

**Rationale**: GUI mode requires security token and dynamic port. Allowing CLI override would break security model.

---

### Edge Case 6: Config Reload While Running

**Scenario**: User reloads config via API, changes webserver.port

**Behavior**: No effect until restart

**Implementation:**

```rust
// In config reload endpoint
if config_changed.contains("webserver") {
    response.warnings.push(
        "Webserver config changes require restart to take effect"
    );
}
```

**UI notification**: Show toast message: "Webserver settings require restart"

---

### Edge Case 7: Pre-initialization Mode (No config.toml)

**Scenario**: First run, no config file exists yet

**Behavior**:

- Use hardcoded defaults (127.0.0.1:8080)
- CLI args still work and override defaults
- After initialization completes, config values take effect on next start

**Implementation:**

```rust
let port = if crate::global::is_initialization_complete() {
    // Config loaded, use precedence: CLI > Config > Default
    resolve_port_with_config()?
} else {
    // Pre-init: CLI > Default (no config yet)
    resolve_port_no_config()?
};
```

---

## 6. Test Scenarios

### Test 1: Basic Port Conflict

**Setup**: Start bot on port 8080, try starting another instance
**Expected**: Second instance exits immediately with error message
**Verification**: `echo $?` returns 1, error logged, no zombie processes

---

### Test 2: CLI Override

**Setup**: Config has port 9000, run with `--port 3000`
**Expected**: Bot uses port 3000, log shows "from CLI"
**Verification**: `curl localhost:3000/api/status` succeeds

---

### Test 3: Port Validation

**Setup**: Run with `--port 99999` (invalid)
**Expected**: Bot exits immediately with validation error
**Verification**: Error message shows valid range, exit code 1

---

### Test 4: Host Validation

**Setup**: Run with `--host 999.999.999.999` (invalid IP)
**Expected**: Bot exits immediately or bind fails with clear error
**Verification**: Error mentions invalid format

---

### Test 5: Port 0 (Random)

**Setup**: Run with `--port 0`
**Expected**: Bot binds to OS-assigned port, logs actual port number
**Verification**: Log shows "OS assigned port XXXXX", curl works on that port

---

### Test 6: Privileged Port

**Setup**: Run with `--port 80` without sudo
**Expected**: Warning logged, bind fails with PermissionDenied
**Verification**: Error message suggests using port above 1024

---

### Test 7: GUI Mode Ignores CLI

**Setup**: Run with `--gui --port 3000`
**Expected**: GUI uses dynamic port, CLI arg ignored
**Verification**: Log shows "CLI arguments ignored", security token generated

---

### Test 8: Config Precedence

**Setup**: Set config port=5000, no CLI args
**Expected**: Bot uses port 5000 from config
**Verification**: Log shows "from config.toml"

---

### Test 9: Remote Access (0.0.0.0)

**Setup**: Run with `--host 0.0.0.0 --port 8080`
**Expected**: Bot binds to all interfaces
**Verification**: Accessible from remote machine via VPS IP

---

### Test 10: Pre-initialization Mode

**Setup**: Delete config.toml, run bot
**Expected**: Uses defaults (127.0.0.1:8080), shows init screen
**Verification**: Localhost:8080 shows initialization UI

---

## 7. Implementation Checklist

### Phase 1: Validation & CLI (No Breaking Changes)

- [ ] Add `get_webserver_port()` to arguments.rs
- [ ] Add `get_webserver_host()` to arguments.rs
- [ ] Add `validate_port()` helper
- [ ] Add `validate_host()` helper
- [ ] Update `print_help()` with new options
- [ ] Add early validation in `run_bot_internal()`
- [ ] Test CLI parsing with various inputs

### Phase 2: Service Layer Changes (Core Fix)

- [ ] Add `resolve_port()` helper in webserver_service.rs
- [ ] Add `resolve_host()` helper in webserver_service.rs
- [ ] Add `test_port_binding()` pre-flight check
- [ ] Update `start()` method with pre-flight test
- [ ] Add detailed error messages for binding failures
- [ ] Update logging to show config source
- [ ] Handle GUI mode exception (ignore CLI args)

### Phase 3: Server Layer Simplification

- [ ] Remove config resolution from server.rs
- [ ] Use global state values only
- [ ] Simplify bind error handling
- [ ] Update comments and documentation

### Phase 4: Edge Cases

- [ ] Implement port 0 (random port) logic
- [ ] Add privileged port warning
- [ ] Handle pre-initialization mode correctly
- [ ] Add config reload warning for webserver changes
- [ ] Test all edge cases from section 5

### Phase 5: Documentation & Website

- [ ] Update CLI help output
- [ ] Add examples to README
- [ ] Update website docs for --port and --host
- [ ] Add troubleshooting section for port conflicts
- [ ] Document precedence order clearly

### Phase 6: Testing

- [ ] Run all 10 test scenarios
- [ ] Test on macOS, Linux, Windows
- [ ] Test with process lock + port conflict
- [ ] Test GUI mode vs headless mode
- [ ] Verify error messages are helpful
- [ ] Check exit codes are correct

---

## 8. Migration Path (Zero Breaking Changes)

**Existing behavior preserved:**

- Default: 127.0.0.1:8080 (unchanged)
- Config file: Still works as before (unchanged)
- GUI mode: Still uses dynamic port (unchanged)
- Error handling: Enhanced but backward compatible

**New features added:**

- CLI override capability (new, optional)
- Pre-flight port check (new, prevents silent failures)
- Better error messages (enhancement)

**User migration:**

1. Existing users: No action needed, everything works as before
2. VPS users: Can now use `--host 0.0.0.0` without editing config
3. Docker users: Can easily override port per container
4. Development: Can run multiple instances with different ports

**Rollback plan**: If issues arise, can temporarily disable pre-flight check with feature flag.

---

## 9. Performance Impact

**Startup time**: +1-5ms for port test (negligible)

**Memory**: No additional allocations

**CPU**: One extra bind/unbind during startup (negligible)

**Network**: One extra socket operation (negligible)

**Verdict**: Zero measurable performance impact.

---

## 10. Security Considerations

### Positive Security Impacts:

1. **Prevents silent failures**: User knows immediately if port is compromised
2. **Clear host binding**: Explicit logging of which interface is used
3. **GUI mode protected**: CLI args can't override GUI security model

### No New Attack Surface:

- Pre-flight test uses same TcpListener as main server
- No new network operations introduced
- Validation happens before any binding

### Recommendations:

- Keep default 127.0.0.1 (localhost only) for security
- Log warning when binding to 0.0.0.0 (all interfaces)
- Document security implications of remote access

---

## 11. Backward Compatibility Matrix

| Scenario                    | Before                | After                | Compatible?                 |
| --------------------------- | --------------------- | -------------------- | --------------------------- |
| No CLI args, default config | 127.0.0.1:8080        | 127.0.0.1:8080       | ✅ Yes                      |
| Config port=9000            | Uses 9000             | Uses 9000            | ✅ Yes                      |
| GUI mode                    | Dynamic port          | Dynamic port         | ✅ Yes                      |
| Port conflict               | Logs error, continues | Exits immediately    | ⚠️ Better (behavior change) |
| Pre-init (no config)        | 127.0.0.1:8080        | 127.0.0.1:8080       | ✅ Yes                      |
| New: --port 3000            | N/A                   | Uses 3000            | ➕ New feature              |
| New: --host 0.0.0.0         | N/A                   | Binds all interfaces | ➕ New feature              |

**Breaking changes**: NONE

**Behavior changes**: Only port conflict handling (improvement, not breakage)

---

## 12. Success Criteria

### Must Have (Required for completion):

1. ✅ Bot exits immediately on port conflict (no silent failure)
2. ✅ CLI --port and --host arguments work correctly
3. ✅ Precedence order implemented: CLI > Config > Default
4. ✅ Error messages are clear and actionable
5. ✅ All 10 test scenarios pass
6. ✅ Zero breaking changes for existing users

### Should Have (Important but not blocking):

1. ✅ Edge cases handled gracefully
2. ✅ Logging shows config source
3. ✅ Documentation updated
4. ✅ Website updated with examples

### Nice to Have (Future enhancements):

1. Auto-retry with next available port
2. Config validation tool
3. Health check endpoint shows bind status

---

## 13. Rollout Plan

### Step 1: Development (Days 1-2)

- Implement Phase 1 & 2 (core functionality)
- Local testing with all test scenarios
- Code review with focus on error paths

### Step 2: Testing (Day 3)

- Test on all platforms (macOS, Linux, Windows)
- Test with real VPS deployment
- Test GUI vs headless modes
- Verify backward compatibility

### Step 3: Documentation (Day 4)

- Update CLI help
- Update website docs
- Add troubleshooting section
- Create migration guide (even though no breaking changes)

### Step 4: Release (Day 5)

- Merge to main branch
- Tag release with clear changelog
- Monitor for issues in first 24 hours
- Prepare hotfix branch just in case

---

## 14. Monitoring & Observability

### New Metrics to Track:

1. Port conflict errors (should be rare)
2. CLI argument usage (--port, --host)
3. Config source distribution (CLI vs Config vs Default)
4. Pre-flight test failures

### Logging Strategy:

- INFO: Config source, final bind address
- WARNING: Privileged ports, remote access (0.0.0.0)
- ERROR: Validation failures, binding failures
- DEBUG: Pre-flight test results, precedence resolution

### User Feedback Channels:

- Error messages with actionable solutions
- Log file contains full context
- Website docs have troubleshooting guide

---

## 15. Alternatives Considered (And Why Rejected)

### Alternative 1: Just fix error propagation (no CLI args)

**Rejected**: Doesn't solve VPS use case, still requires config edit

### Alternative 2: Use port range instead of single port

**Rejected**: Adds complexity, users want deterministic ports

### Alternative 3: Automatic port fallback (try 8080, 8081, 8082...)

**Rejected**: Confusing for users, hard to predict which port is used

### Alternative 4: Config file only (no CLI args)

**Rejected**: Poor UX for Docker/VPS, requires manual file editing

### Alternative 5: Environment variables instead of CLI args

**Rejected**: CLI args are more discoverable and conventional

---

## Conclusion

This solution provides:

- ✅ **Immediate error detection** via pre-flight check
- ✅ **User control** via CLI arguments
- ✅ **Clear precedence** (CLI > Config > Default)
- ✅ **Zero breaking changes** for existing users
- ✅ **Excellent error messages** with actionable solutions
- ✅ **Simple implementation** with minimal code changes
- ✅ **Complete edge case handling** for production readiness

**Estimated implementation time**: 2-3 days (including testing)

**Risk level**: Low (pre-flight pattern is well-tested in the industry)

**User impact**: High (fixes critical silent failure bug)

**Recommendation**: Proceed with implementation immediately.
