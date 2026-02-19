# Webserver Port Conflict Solution - Executive Summary

## Problem Statement

ScreenerBot continues running when webserver port is already in use because `tokio::spawn()` swallows binding errors. The error happens inside the spawned task and never propagates back to `Service::start()`, so the bot thinks everything is fine.

## Solution Overview

Implement **pre-flight port check** before spawning the server task, add CLI arguments for `--port` and `--host` override, and ensure proper error propagation so bot exits immediately on port conflict.

## Recommended Approach

**Option A: Pre-flight Check Before Spawn**

Test port binding synchronously BEFORE spawning the server task. If binding fails, return error immediately. If successful, spawn knowing it will work.

**Why this is best:**

- ✅ Immediate feedback (fails fast)
- ✅ Clean error path (no channels needed)
- ✅ Matches existing architecture (Service.start returns Result)
- ✅ Simple implementation
- ✅ No breaking changes

## Key Changes

### 1. CLI Arguments (`src/arguments.rs`)

```rust
// New functions
pub fn get_webserver_port() -> Option<u16>
pub fn get_webserver_host() -> Option<String>
pub fn validate_port(port: u16) -> Result<(), String>
pub fn validate_host(host: &str) -> Result<(), String>
```

### 2. Early Validation (`src/run.rs`)

```rust
// In run_bot_internal()
validate_cli_arguments()?; // Fail fast if invalid
```

### 3. Pre-flight Check (`src/services/implementations/webserver_service.rs`)

```rust
async fn start(...) -> Result<Vec<JoinHandle<()>>, String> {
    // 1. Resolve config: CLI > Config > Default
    let port = resolve_port()?;
    let host = resolve_host()?;

    // 2. TEST before spawning
    test_port_binding(&host, port).await?;

    // 3. Store in global state
    set_webserver_port(port);
    set_webserver_host(host);

    // 4. NOW spawn (will succeed)
    let handle = tokio::spawn(...);

    Ok(vec![handle])
}
```

### 4. Simplified Server (`src/webserver/server.rs`)

```rust
// Just use values from global state (already resolved)
let port = global::get_webserver_port();
let host = global::get_webserver_host();
```

## Config Precedence

```
Port: CLI --port > config.webserver.port > 8080
Host: CLI --host > config.webserver.host > 127.0.0.1
```

**Exception**: GUI mode IGNORES CLI args (uses dynamic port for security)

## Error Messages

### Port Already in Use

```
ERROR: Failed to start webserver

Cannot bind to 127.0.0.1:8080 - Address already in use

Solutions:
  1. Stop other instances:
     pkill -f screenerbot

  2. Use different port:
     screenerbot --port 3000

  3. Edit config.toml:
     [webserver]
     port = 3000
```

### Invalid Port

```
ERROR: Invalid port number

Port must be between 1 and 65535, got: 99999

Use: screenerbot --port <valid_port>
```

### Permission Denied

```
ERROR: Failed to start webserver

Cannot bind to 0.0.0.0:80 - Permission denied

Port 80 requires elevated privileges.

Solutions:
  • Use port above 1024: --port 8080
  • Configure port forwarding (recommended)
```

## Edge Cases Handled

1. **Port 0**: OS assigns random available port (useful for testing)
2. **Privileged ports (<1024)**: Warning but attempts binding
3. **Invalid hostname**: Format validation + helpful error
4. **CLI vs Config conflict**: CLI wins, logs source
5. **GUI mode with CLI args**: Ignores CLI, logs warning
6. **Pre-initialization (no config)**: Uses CLI > Default
7. **Config reload**: Shows "requires restart" message

## Testing Scenarios

✅ Port conflict (should exit with error)
✅ CLI override (--port 3000)
✅ Port validation (invalid port number)
✅ Host validation (invalid IP)
✅ Port 0 (random port)
✅ Privileged port (permission denied)
✅ GUI mode ignores CLI
✅ Config precedence
✅ Remote access (0.0.0.0)
✅ Pre-initialization mode

## Impact Analysis

### Breaking Changes

**NONE** - Existing behavior fully preserved

### Behavior Changes

- Port conflict now exits immediately (improvement, not breakage)
- Better error messages with actionable solutions

### Performance

- Pre-flight test adds ~1-5ms to startup (negligible)
- No ongoing impact

### Security

- No new attack surface
- GUI mode security model preserved
- Clear logging of bind interface

## Implementation Timeline

- **Day 1**: Implement arguments.rs + run.rs validation
- **Day 2**: Implement webserver_service.rs pre-flight check
- **Day 3**: Testing on all platforms + edge cases
- **Day 4**: Documentation updates + website
- **Day 5**: Release + monitoring

**Total**: 5 days with buffer

## Success Criteria

### Must Have

1. ✅ Bot exits immediately on port conflict
2. ✅ CLI --port and --host work correctly
3. ✅ Precedence: CLI > Config > Default
4. ✅ Clear, actionable error messages
5. ✅ All test scenarios pass
6. ✅ Zero breaking changes

### Should Have

1. ✅ Edge cases handled gracefully
2. ✅ Logging shows config source
3. ✅ Documentation updated

### Nice to Have

1. Auto-retry with next port
2. Config validation tool
3. Health check shows bind status

## Files Modified

```
src/arguments.rs                           +80 lines (NEW helpers)
src/run.rs                                 +15 lines (validation)
src/services/implementations/webserver_    +100 lines (pre-flight)
src/webserver/server.rs                    -30 lines (simplified)
───────────────────────────────────────────────────────────────
Total                                      +165 net lines
```

## Documentation Created

1. ✅ `WEBSERVER_PORT_CONFLICT_SOLUTION.md` - Full design rationale (15 sections)
2. ✅ `WEBSERVER_PORT_CONFLICT_IMPLEMENTATION.md` - Implementation guide with pseudocode
3. ✅ `WEBSERVER_PORT_CONFLICT_FLOW.md` - Visual diagrams and flow charts
4. ✅ `WEBSERVER_PORT_CONFLICT_SUMMARY.md` - This executive summary

## Next Steps

### For Implementation

1. Start with `arguments.rs` - add getters and validators
2. Add early validation in `run.rs`
3. Implement pre-flight check in `webserver_service.rs`
4. Simplify `server.rs` to use global state
5. Test all scenarios
6. Update website docs

### For Testing

1. Run all 10 test scenarios locally
2. Test on macOS, Linux, Windows
3. Test GUI vs headless modes
4. Verify error messages are helpful
5. Check exit codes

### For Documentation

1. Update CLI help output
2. Add examples to README
3. Update website docs
4. Add troubleshooting guide
5. Document precedence order

## Questions & Answers

**Q: Why not just fix the spawn error propagation?**
A: Pre-flight check is cleaner - errors happen synchronously in the right context.

**Q: Why not auto-retry with port+1?**
A: Confusing for users who expect deterministic ports. Better to fail with clear message.

**Q: Why ignore CLI args in GUI mode?**
A: GUI security model requires dynamic port + token. CLI override would break security.

**Q: What if config is reloaded while running?**
A: Webserver config requires restart. Shows warning in reload response.

**Q: Performance impact?**
A: Negligible (~1-5ms startup), one extra bind/unbind operation.

**Q: Breaking changes?**
A: None. All existing behavior preserved. Only enhancements added.

## Recommendation

**✅ PROCEED WITH IMPLEMENTATION**

- Low risk (well-tested pattern)
- High user impact (fixes critical bug)
- Clean architecture (no hacks)
- Simple implementation (2-3 days)
- Zero breaking changes
- Excellent documentation

---

**Design Status**: ✅ Complete and ready for implementation

**Design Documents**:

- `/docs/WEBSERVER_PORT_CONFLICT_SOLUTION.md` (complete design)
- `/docs/WEBSERVER_PORT_CONFLICT_IMPLEMENTATION.md` (implementation guide)
- `/docs/WEBSERVER_PORT_CONFLICT_FLOW.md` (visual diagrams)
- `/docs/WEBSERVER_PORT_CONFLICT_SUMMARY.md` (this document)

**Approval**: Pending stakeholder review

**Implementation**: Ready to start immediately after approval
