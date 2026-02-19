# Webserver Port Conflict - Solution Flow Diagram

## Current Flow (BROKEN)

```
┌─────────────────────────────────────────────────────────────┐
│ run_bot_internal()                                           │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ ServiceManager::start_all()                                  │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ WebserverService::start()                                    │
│   - service.start() returns Result<Vec<Handle>, String>      │
│   - Immediately spawns: tokio::spawn(...)                    │
│   - Returns Ok(vec![handle])  ← ALWAYS succeeds!            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          │ Returns Ok() even if port fails
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ ServiceManager continues...                                  │
│   ✅ Service marked as "started"                             │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ Meanwhile, in spawned task...
                          ▼
        ┌─────────────────────────────────────┐
        │ async move {                         │
        │   crate::webserver::start_server()   │
        │   └─> TcpListener::bind()            │
        │       └─> ERROR: Address in use!     │  ← Error here!
        │   Logs error but task just exits     │
        └──────────────────────────────────────┘
                          │
                          │ Error swallowed by spawn
                          ▼
        ┌─────────────────────────────────────┐
        │ Bot continues running                 │
        │ ❌ Webserver not working              │
        │ ❌ User doesn't know                  │
        └───────────────────────────────────────┘
```

**Problem**: Error happens INSIDE spawned task, never propagates back to service.start()

---

## New Flow (FIXED)

```
┌─────────────────────────────────────────────────────────────┐
│ run_bot_internal()                                           │
│   1. Parse CLI args                                          │
│   2. validate_cli_arguments() ← NEW!                         │
│      └─> validate_port()?                                    │
│      └─> validate_host()?                                    │
│      └─> Returns Err() if invalid                            │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          │ If validation fails, exit here
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ ServiceManager::start_all()                                  │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ WebserverService::start()                                    │
│                                                              │
│   1. Resolve config with precedence:                         │
│      ┌──────────────────────────────────┐                   │
│      │ CLI --port > Config > Default    │                   │
│      │ CLI --host > Config > Default    │                   │
│      └──────────────────────────────────┘                   │
│                                                              │
│   2. PRE-FLIGHT CHECK: ← NEW!                                │
│      test_port_binding(&host, port).await?                   │
│      ┌──────────────────────────────────┐                   │
│      │ Try TcpListener::bind()          │                   │
│      │ If Ok: drop listener, continue   │                   │
│      │ If Err: return error immediately │                   │
│      └──────────────────────────────────┘                   │
│                                                              │
│   3. Store values in global state                            │
│      set_webserver_port(port)                                │
│      set_webserver_host(host)                                │
│                                                              │
│   4. NOW spawn (we know it will succeed):                    │
│      tokio::spawn(...)                                       │
│                                                              │
│   5. Return Ok(vec![handle])                                 │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          │ Only returns Ok() if port is available
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ IF pre-flight check FAILED:                                  │
│   - Returns Err() from service.start()                       │
│   - ServiceManager::start_all() propagates error             │
│   - run_bot_internal() receives error                        │
│   - Bot exits with exit code 1                               │
│   - User sees clear error message                            │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ If pre-flight check SUCCEEDED:
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ ServiceManager continues normally                            │
│   ✅ Service marked as "started"                             │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          │ Spawned task runs...
                          ▼
        ┌─────────────────────────────────────┐
        │ async move {                         │
        │   crate::webserver::start_server()   │
        │   └─> TcpListener::bind()            │
        │       └─> SUCCESS (already tested)   │
        │   Server runs normally               │
        └──────────────────────────────────────┘
                          │
                          ▼
        ┌─────────────────────────────────────┐
        │ Bot runs normally                     │
        │ ✅ Webserver working                  │
        │ ✅ User happy                         │
        └───────────────────────────────────────┘
```

**Key Insight**: Test binding BEFORE spawning, so error can propagate synchronously.

---

## Config Precedence Flow

```
┌─────────────────────────────────────────────────────────────┐
│ resolve_port() / resolve_host()                              │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
        ┌─────────────────────────────────────┐
        │ Check CLI arguments first            │
        │ arguments::get_webserver_port()      │
        │ arguments::get_webserver_host()      │
        └─────────────┬──────────────┬─────────┘
                      │              │
                 Found │              │ Not found
                      ▼              ▼
        ┌─────────────────┐   ┌─────────────────────────────┐
        │ Validate & Use  │   │ Check config file            │
        │ CLI value       │   │ with_config(|cfg| ...)       │
        └─────────────────┘   └──────────┬──────────────────┘
                │                        │              │
                │                   Found │              │ Not found
                │                        ▼              ▼
                │              ┌─────────────┐   ┌─────────────┐
                │              │ Use config  │   │ Use default │
                │              │ value       │   │ 8080/127.0  │
                │              └──────┬──────┘   └──────┬──────┘
                │                     │                 │
                └─────────────────────┴─────────────────┘
                                      │
                                      ▼
                        ┌──────────────────────────┐
                        │ Final port/host value    │
                        └──────────────────────────┘
```

---

## Pre-flight Test Flow

```
┌─────────────────────────────────────────────────────────────┐
│ test_port_binding(host, port)                                │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
        ┌─────────────────────────────────────┐
        │ Is port == 0?                        │
        │ (Special case: random port)          │
        └─────────┬────────────────┬───────────┘
                  │                │
             Yes  │                │  No
                  ▼                ▼
    ┌──────────────────────┐   ┌──────────────────────────┐
    │ Bind to :0           │   │ Bind to host:port        │
    │ Get assigned port    │   │ TcpListener::bind(addr)  │
    │ Drop listener        │   │                          │
    │ Return Ok()          │   └──────────┬───────────────┘
    └──────────────────────┘              │
                                          ▼
                        ┌─────────────────────────────────┐
                        │ Bind result?                     │
                        └─────┬──────────────────┬─────────┘
                              │                  │
                         Ok() │                  │ Err(e)
                              ▼                  ▼
            ┌──────────────────────┐   ┌─────────────────────────┐
            │ Drop listener        │   │ Match error kind:        │
            │ Log debug            │   │  • AddrInUse             │
            │ Return Ok()          │   │  • PermissionDenied      │
            └──────────────────────┘   │  • Other                 │
                                       │                          │
                                       │ Create detailed error    │
                                       │ with actionable steps    │
                                       │ Return Err(message)      │
                                       └──────────────────────────┘
```

---

## Error Message Flow

```
Port conflict detected:

┌─────────────────────────────────────────────────────────────┐
│ TcpListener::bind() fails with AddrInUse                     │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ create_binding_error_message()                               │
│   - Matches error.kind()                                     │
│   - Creates context-specific message                         │
│   - Includes actionable solutions                            │
│   - Returns formatted String                                 │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ test_port_binding() returns Err(message)                     │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ WebserverService::start() returns Err(message)               │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ ServiceManager::start_all() propagates error                 │
│   - Logs error with context                                  │
│   - Returns Err() from start_all()                           │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ run_bot_internal() receives error                            │
│   - Returns Err(message)                                     │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│ main() handles error                                         │
│   - Prints error message to stderr                           │
│   - Exits with code 1                                        │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
        ┌─────────────────────────────────────┐
        │ User sees:                           │
        │                                      │
        │ ERROR: Failed to start webserver    │
        │                                      │
        │ Cannot bind to 127.0.0.1:8080 -     │
        │ Address already in use              │
        │                                      │
        │ Solutions:                           │
        │   1. Stop other instances...        │
        │   2. Use different port...          │
        │   3. Edit config...                 │
        └─────────────────────────────────────┘
```

---

## GUI Mode Exception Flow

```
┌─────────────────────────────────────────────────────────────┐
│ WebserverService::start()                                    │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
        ┌─────────────────────────────────────┐
        │ Check: is_gui_mode()?                │
        └─────────┬────────────────┬───────────┘
                  │                │
             Yes  │                │  No (headless)
                  ▼                ▼
    ┌──────────────────────┐   ┌──────────────────────────┐
    │ GUI MODE:            │   │ HEADLESS MODE:           │
    │                      │   │                          │
    │ 1. Check if CLI args │   │ 1. Resolve CLI/Config    │
    │    provided          │   │ 2. Pre-flight test       │
    │    └─> Log warning   │   │ 3. Spawn server          │
    │        "ignored"     │   │                          │
    │                      │   └──────────────────────────┘
    │ 2. Find random port  │
    │    in 49152-65535    │
    │                      │
    │ 3. Generate security │
    │    token             │
    │                      │
    │ 4. Spawn server      │
    │    (always 127.0.0.1)│
    │                      │
    └──────────────────────┘
```

**Key**: GUI mode IGNORES --port and --host for security reasons.

---

## Complete Call Stack (Success Case)

```
main()
  └─> run_bot()
        └─> run_bot_internal()
              ├─> validate_cli_arguments()
              │     ├─> get_webserver_port()
              │     ├─> validate_port()?
              │     ├─> get_webserver_host()
              │     └─> validate_host()?
              │
              └─> ServiceManager::start_all()
                    └─> WebserverService::start()
                          ├─> resolve_port()
                          │     ├─> get_webserver_port()
                          │     ├─> with_config(|cfg| cfg.webserver.port)
                          │     └─> Returns final port
                          │
                          ├─> resolve_host()
                          │     ├─> get_webserver_host()
                          │     ├─> with_config(|cfg| cfg.webserver.host)
                          │     └─> Returns final host
                          │
                          ├─> test_port_binding(&host, port)?
                          │     ├─> TcpListener::bind(addr)
                          │     ├─> If Ok: drop(listener), return Ok()
                          │     └─> If Err: create_error_message(), return Err()
                          │
                          ├─> set_webserver_port(port)
                          ├─> set_webserver_host(host)
                          │
                          └─> tokio::spawn(async {
                                    crate::webserver::start_server()
                                      ├─> get_webserver_port()
                                      ├─> get_webserver_host()
                                      ├─> TcpListener::bind(addr)
                                      └─> axum::serve(listener, app)
                                })
```

---

## Complete Call Stack (Failure Case)

```
main()
  └─> run_bot()
        └─> run_bot_internal()
              ├─> validate_cli_arguments()  ← Fails here if invalid CLI args
              │     ├─> get_webserver_port()
              │     └─> validate_port(99999)
              │           └─> Returns Err("Invalid port number: 99999...")
              │
              └─> Returns Err() propagated from validation
                    │
                    └─> main() catches error
                          ├─> eprintln!("Error: {}", e)
                          └─> std::process::exit(1)

OR (port conflict during service start):

main()
  └─> run_bot()
        └─> run_bot_internal()
              ├─> validate_cli_arguments()  ← Passes
              │
              └─> ServiceManager::start_all()
                    └─> WebserverService::start()
                          ├─> resolve_port()  ← Returns 8080
                          ├─> resolve_host()  ← Returns 127.0.0.1
                          │
                          └─> test_port_binding("127.0.0.1", 8080)?
                                ├─> TcpListener::bind(addr)
                                │     └─> Returns Err(AddrInUse)
                                │
                                ├─> create_binding_error_message()
                                │     └─> Returns formatted error
                                │
                                └─> Returns Err(error_message)
                                      │
                                      └─> WebserverService::start() returns Err()
                                            └─> ServiceManager::start_all() returns Err()
                                                  └─> run_bot_internal() returns Err()
                                                        └─> main() catches error
                                                              ├─> eprintln!()
                                                              └─> exit(1)
```

---

## Timing Diagram

```
Time →

Without Fix (BROKEN):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
T0: ServiceManager::start_all()
T1: WebserverService::start() called
T2: tokio::spawn() called → Returns Ok() immediately
T3: ServiceManager continues (thinks service is OK)
T4: Bot continues running normally
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
T5: [In spawned task] TcpListener::bind() fails
T6: [In spawned task] Error logged
T7: [In spawned task] Task exits
    ❌ Error never seen by caller
    ❌ Bot continues with broken webserver


With Fix (WORKING):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
T0: ServiceManager::start_all()
T1: WebserverService::start() called
T2: test_port_binding() called
T3: TcpListener::bind() called (pre-flight test)
T4: Bind fails → Returns Err() immediately
T5: WebserverService::start() returns Err()
T6: ServiceManager::start_all() returns Err()
T7: run_bot_internal() returns Err()
T8: main() catches error, prints, exits with code 1
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ✅ Error caught immediately
    ✅ User sees clear message
    ✅ Bot exits cleanly
    ✅ No zombie processes
```

**Key Difference**: Error happens BEFORE spawn, so it can propagate synchronously.

---

## State Machine

```
┌─────────────┐
│  Bot Start  │
└──────┬──────┘
       │
       ▼
┌─────────────────┐
│ Validate CLI    │
│ Arguments       │
└────┬────────────┘
     │
     ├─ Invalid ──> [ERROR EXIT]
     │
     ▼ Valid
┌─────────────────┐
│ Resolve Config  │
│ (CLI>Cfg>Def)   │
└────┬────────────┘
     │
     ▼
┌─────────────────┐
│ Pre-flight Test │
│ Port Binding    │
└────┬────────────┘
     │
     ├─ Fails ────> [ERROR EXIT]
     │
     ▼ Success
┌─────────────────┐
│ Store Global    │
│ Port/Host       │
└────┬────────────┘
     │
     ▼
┌─────────────────┐
│ Spawn Server    │
│ Task            │
└────┬────────────┘
     │
     ▼
┌─────────────────┐
│ Bot Running     │
│ Normally        │
└─────────────────┘
```

**State transitions**:

- Start → ValidateCLI → ResolveConfig → PreflightTest → Store → Spawn → Running
- Any step can fail and cause ERROR EXIT
- No partial states (either fully started or fully stopped)

---

## Comparison Matrix

| Aspect              | Before (Broken)        | After (Fixed)             |
| ------------------- | ---------------------- | ------------------------- |
| Error detection     | Spawned task (async)   | Pre-flight test (sync)    |
| Error propagation   | ❌ Swallowed by spawn  | ✅ Returns Err()          |
| Exit on conflict    | ❌ Continues running   | ✅ Exits immediately      |
| Error message       | ❌ Just logs error     | ✅ Detailed + solutions   |
| CLI override        | ❌ Not supported       | ✅ --port and --host      |
| Config precedence   | ❌ Only config/default | ✅ CLI > Config > Default |
| Port validation     | ❌ No validation       | ✅ Early validation       |
| Test before spawn   | ❌ No                  | ✅ Yes (pre-flight)       |
| GUI mode security   | ✅ Dynamic port        | ✅ Same (CLI ignored)     |
| Startup time impact | None                   | +1-5ms (negligible)       |

---

## Decision Tree for Port Resolution

```
Is GUI mode?
├─ Yes → Use dynamic port (49152-65535)
│        Ignore CLI args
│        Generate security token
│        Bind to 127.0.0.1 only
│
└─ No (headless) → Resolve port/host:
    │
    ├─ Has --port CLI arg?
    │  ├─ Yes → Validate → Use CLI port
    │  └─ No  → Continue
    │
    ├─ Config initialized?
    │  ├─ Yes → Has config.webserver.port?
    │  │        ├─ Yes → Use config port
    │  │        └─ No  → Continue
    │  └─ No  → Continue
    │
    └─ Use default (8080)

Same logic for host:
    --host > config.webserver.host > 127.0.0.1
```

---

## Files Modified Summary

```
src/
├── arguments.rs                       (+80 lines)
│   ├── get_webserver_port()           [NEW]
│   ├── get_webserver_host()           [NEW]
│   ├── validate_port()                [NEW]
│   ├── validate_host()                [NEW]
│   └── print_help()                   [UPDATED]
│
├── run.rs                             (+15 lines)
│   ├── validate_cli_arguments()       [NEW]
│   └── run_bot_internal()             [UPDATED - add validation call]
│
├── services/implementations/
│   └── webserver_service.rs          (+100 lines)
│       ├── resolve_port()             [NEW]
│       ├── resolve_host()             [NEW]
│       ├── get_config_source()        [NEW]
│       ├── test_port_binding()        [NEW]
│       ├── create_binding_error...()  [NEW]
│       ├── start_gui_mode()           [EXTRACTED]
│       └── start()                    [UPDATED - add pre-flight]
│
└── webserver/
    └── server.rs                      (-30 lines)
        └── start_server()             [SIMPLIFIED - use global state]

Total changes: +165 net lines (mostly new helpers)
```

---

This visual guide complements the design and implementation docs.
