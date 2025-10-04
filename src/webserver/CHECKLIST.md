# Webserver Module - Implementation Checklist

## ✅ What Was Created (Completed)

- [x] Module structure in `src/webserver/`
- [x] Server implementation (`server.rs`)
- [x] Configuration system (`config.rs`)
- [x] Application state (`state.rs`)
- [x] Utility functions (`utils.rs`)
- [x] Request/Response models (`models/`)
- [x] WebSocket event types (`models/events.rs`)
- [x] Phase 1 status routes (`routes/status.rs`)
- [x] Route aggregation (`routes/mod.rs`)
- [x] Placeholder files for Phase 2/3
- [x] Comprehensive documentation (5 docs)
- [x] Setup automation script
- [x] Visual architecture diagram

## 📋 Integration Checklist (What YOU Need to Do)

### Step 1: Install Dependencies
- [ ] Run `./scripts/setup_webserver.sh`
  - OR manually: `cargo add axum@0.7 tower@0.4 tower-http@0.5 hyper@1.0 sysinfo@0.30`
- [ ] Verify: `cargo build --lib` succeeds

### Step 2: Add Module to lib.rs
- [ ] Open `src/lib.rs`
- [ ] Add line: `pub mod webserver;`
- [ ] Verify: `cargo check --lib` succeeds

### Step 3: Update Logger
- [ ] Open `src/logger.rs`
- [ ] Add `Webserver` to `LogTag` enum:
  ```rust
  pub enum LogTag {
      // ... existing tags ...
      Webserver,
  }
  ```
- [ ] Add to `as_str()` match:
  ```rust
  LogTag::Webserver => "Webserver",
  ```
- [ ] Verify: `cargo check --lib` succeeds

### Step 4: Update Configs
- [ ] Open `src/configs.rs`
- [ ] Add import at top:
  ```rust
  use crate::webserver::config::WebserverConfig;
  ```
- [ ] Add field to `Configs` struct:
  ```rust
  #[serde(default)]
  pub webserver: WebserverConfig,
  ```
- [ ] Verify: `cargo check --lib` succeeds

### Step 5: Update Config File
- [ ] Open `data/configs.json`
- [ ] Add webserver section:
  ```json
  {
    "webserver": {
      "enabled": true,
      "host": "127.0.0.1",
      "port": 8080
    }
  }
  ```
- [ ] Verify: JSON is valid (use `jq . data/configs.json`)

### Step 6: Add to Startup Flow
- [ ] Open `src/run.rs` (or wherever services start)
- [ ] Find location after core services initialize
- [ ] Add webserver startup code:
  ```rust
  // Start webserver dashboard
  if configs.webserver.enabled {
      log(LogTag::System, "INFO", "🌐 Starting webserver dashboard...");
      
      let webserver_config = configs.webserver.clone();
      tokio::spawn(async move {
          if let Err(e) = crate::webserver::start_server(webserver_config).await {
              log(LogTag::Webserver, "ERROR", &format!("Webserver failed: {}", e));
          }
      });
      
      // Give it a moment to start
      tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
      
      log(LogTag::System, "INFO", "✅ Webserver started successfully");
  }
  ```
- [ ] Verify: `cargo build` succeeds

### Step 7: Add Shutdown Handler (Optional but Recommended)
- [ ] Find shutdown/cleanup code in your main loop
- [ ] Add before final cleanup:
  ```rust
  // Gracefully shutdown webserver
  log(LogTag::System, "INFO", "Stopping webserver...");
  crate::webserver::shutdown();
  tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
  ```

### Step 8: Test Basic Functionality
- [ ] Start the bot: `cargo run -- --run`
- [ ] Look for webserver startup logs:
  - `[System] 🌐 Starting webserver dashboard...`
  - `[Webserver] ✅ Webserver listening on http://127.0.0.1:8080`
- [ ] In another terminal, test endpoints:
  ```bash
  # Test health check
  curl http://localhost:8080/api/v1/health
  
  # Test full status
  curl http://localhost:8080/api/v1/status | jq
  
  # Test services only
  curl http://localhost:8080/api/v1/status/services | jq
  
  # Test metrics only
  curl http://localhost:8080/api/v1/status/metrics | jq
  ```
- [ ] Verify all return valid JSON
- [ ] Verify service status shows correct ready states
- [ ] Verify metrics show actual values (memory, RPC calls, etc.)

### Step 9: Test Error Handling
- [ ] Try invalid endpoint: `curl http://localhost:8080/invalid`
- [ ] Should return 404 (not found)
- [ ] Stop bot and verify webserver shuts down gracefully

### Step 10: Review Logs
- [ ] Check `logs/` directory for webserver entries
- [ ] Should see `[Webserver]` tagged messages
- [ ] No errors should be present (except during testing)

## 🎯 Success Criteria

You know Phase 1 is working when:

- ✅ Bot starts without errors
- ✅ Webserver logs appear correctly
- ✅ `/health` endpoint returns: `{"status":"ok",...}`
- ✅ `/status` endpoint shows all 5 services
- ✅ Service ready flags match bot status
- ✅ Metrics show actual RPC/memory data
- ✅ Bot still trades normally (no performance impact)
- ✅ Graceful shutdown works

## 🐛 Troubleshooting

### Problem: Port 8080 already in use
**Solution:**
```bash
# Find process using port
lsof -ti:8080

# Kill it
kill -9 <PID>

# Or change port in configs.json
```

### Problem: Module not found error
**Solution:**
- Ensure `pub mod webserver;` is in `src/lib.rs`
- Run `cargo clean && cargo build`

### Problem: LogTag error
**Solution:**
- Check `LogTag::Webserver` is in logger enum
- Check `as_str()` has the match case
- Run `cargo check --lib`

### Problem: Config not loading
**Solution:**
- Validate JSON syntax: `jq . data/configs.json`
- Ensure `webserver` field is in `Configs` struct
- Check `#[serde(default)]` is present

### Problem: Services show not ready
**Solution:**
- This is normal during bot startup
- Wait for all services to initialize
- Check main bot logs for readiness

### Problem: No data in metrics
**Solution:**
- Wait for bot to run for a bit
- RPC calls accumulate over time
- Memory/CPU update on each request

## 📚 Reference Documentation

- **Architecture**: `docs/webserver-dashboard-architecture.md`
- **Implementation**: `docs/webserver-implementation-guide.md`
- **Usage**: `src/webserver/README.md`
- **Summary**: `src/webserver/SUMMARY.md`
- **Visual**: `src/webserver/ARCHITECTURE.txt`

## 🚀 After Phase 1 Works

Move on to Phase 2:

1. Implement position routes
2. Implement token routes  
3. Implement transaction routes
4. Add WebSocket support
5. Test with real trading data

See `docs/webserver-implementation-guide.md` for Phase 2 details.

## 📝 Notes

- All created files are in `src/webserver/`
- No existing bot code was modified
- Safe to remove webserver folder if not needed
- Dependencies are optional until you're ready
- Can be disabled via `"enabled": false` in config

## ✨ Tips

1. **Start Simple**: Get Phase 1 working first before adding features
2. **Test Often**: Check endpoints after each integration step
3. **Read Logs**: Webserver logs show what's happening
4. **Use jq**: Makes JSON responses readable
5. **Port Conflict**: Use different port if 8080 is taken

## 🎉 You're Done When...

- [ ] All checkboxes above are checked ✅
- [ ] All tests pass ✅
- [ ] Bot trades normally ✅
- [ ] Webserver responds correctly ✅
- [ ] Logs show no errors ✅
- [ ] You understand the architecture ✅
- [ ] Ready to add Phase 2 features ✅

---

**Estimated Time**: 30-60 minutes for complete Phase 1 setup
**Difficulty**: Easy (just follow the steps)
**Risk**: Low (no existing code modified)
**Reward**: Production-ready monitoring dashboard! 🎉
