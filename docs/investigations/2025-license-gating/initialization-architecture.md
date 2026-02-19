# License-Gated Initialization Architecture

**Date:** November 4, 2025  
**Status:** Complete & Production-Ready  
**Phase:** 1 (Backend) + 3 (Frontend) - Fully Implemented

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture Components](#architecture-components)
3. [File Structure](#file-structure)
4. [Backend Implementation](#backend-implementation)
5. [Frontend Implementation](#frontend-implementation)
6. [State Management](#state-management)
7. [API Endpoints](#api-endpoints)
8. [Security Features](#security-features)
9. [Bug Fixes Applied](#bug-fixes-applied)
10. [Testing Guide](#testing-guide)
11. [Troubleshooting](#troubleshooting)

---

## Overview

The License-Gated Initialization Architecture provides a secure, user-friendly first-time setup wizard for ScreenerBot with **continuous license verification**. When the bot starts without a configuration file, it launches in **pre-initialization mode** with only the webserver active. Users complete a 3-step wizard to configure their wallet, RPC endpoints, and license verification before the bot begins trading.

**CRITICAL SECURITY:** License verification occurs on **EVERY bot startup**, not just during initial setup. This ensures:

- Expired licenses are detected and prevent bot operation
- Revoked licenses cannot bypass verification by restarting
- License status is always current and enforced

### Key Features

- **Zero-config First Launch:** Bot starts immediately, no manual config file editing
- **Progressive Disclosure:** 3-step wizard (Credentials → Verification → Service Startup)
- **Security First:** Ephemeral RPC clients, 0o600 file permissions, no credential storage in browser
- **License Validation:** WebSocket-based verification with Solana blockchain **on every startup**
- **Service Orchestration:** Conditional startup with dependency resolution
- **Auto-detection:** Redirects to setup if initialization required
- **Memory Safe:** All pollers and event listeners properly cleaned up

---

## Architecture Components

### 1. Global State Flags

**Location:** `src/global.rs`

```rust
pub static INITIALIZATION_COMPLETE: AtomicBool = AtomicBool::new(false);
pub static CONNECTIVITY_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static TOKENS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POSITIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
```

**Purpose:** Thread-safe coordination between systems during startup

### 2. Pre-initialization Mode

**Location:** `src/run.rs`

**Logic:**

```rust
if !config_path.exists() {
    // Pre-init mode: Only webserver starts
    INITIALIZATION_COMPLETE.store(false, ...);
    // Register all services but only enable webserver
    service_manager.start_all().await?;
    // Wait for initialization completion or Ctrl+C
    wait_for_initialization_or_shutdown().await?;
} else {
    // Normal mode: All services start
    INITIALIZATION_COMPLETE.store(true, ...);
    // Full bot startup
}
```

### 3. Middleware Protection

**Location:** `src/webserver/middleware.rs`

**Rules:**

- Checks `INITIALIZATION_COMPLETE` flag
- **Allows:** `/api/initialization/*`, `/scripts/*`, `/styles/*`, `/api/pages/*`, root path
- **Blocks:** All other `/api/*` endpoints with 503 SERVICE_UNAVAILABLE
- Prevents API access until initialization complete

### 4. Service Manager Integration

**Location:** `src/services/mod.rs`, `src/services/implementations/webserver_service.rs`

**Features:**

- `start_newly_enabled()` method returns a `ServiceStartupReport` with counts/failures for post-init service startup
- Topological sort ensures dependency order
- Webserver has no dependencies (always enabled)
- All other services gated behind `INITIALIZATION_COMPLETE` flag

---

## File Structure

### Backend Files (Rust)

```
src/
├── global.rs                          [Modified] Global flags + helpers
├── run.rs                             [Modified] Conditional startup flow
├── config/
│   └── utils.rs                       [Modified] save_config_to_file with 0o600
├── rpc.rs                             [Modified] test_rpc_endpoint[s] functions
├── license/
│   └── mod.rs                         [Modified] verify_license_for_wallet_with_endpoints
├── services/
│   ├── mod.rs                         [Modified] start_newly_enabled method
│   └── implementations/
│       └── webserver_service.rs       [Modified] No dependencies, always enabled
├── webserver/
│   ├── middleware.rs                  [New] initialization_gate middleware
│   └── routes/
│       ├── mod.rs                     [Modified] Added initialization routes
│       └── initialization.rs          [New] 4 API endpoints (493 lines)
```

### Frontend Files (JavaScript/HTML/CSS)

```
src/webserver/templates/
├── base.html                          [Modified] Auto-detection script
├── templates.rs                       [Modified] Embedded new templates
├── pages/
│   └── initialization.html            [New] Wizard UI structure
├── scripts/
│   ├── core/
│   │   ├── router.js                  [Modified] /initialization route
│   │   └── lifecycle.js               [Used] Page lifecycle pattern
│   └── pages/
│       └── initialization.js          [New] Full wizard logic (565 lines)
└── styles/
    └── pages/
        └── initialization.css         [New] Complete styling (900+ lines)
```

---

## Backend Implementation

### 1. RPC Endpoint Testing

**File:** `src/rpc.rs`

**Function:** `test_rpc_endpoint(url: &str) -> Result<bool, String>`

```rust
// Creates ephemeral RpcClient (not stored globally)
// 3-second timeout on getHealth() call
// Tests connectivity + responsiveness
```

**Function:** `test_rpc_endpoints(urls: Vec<String>) -> Vec<String>`

```rust
// Concurrent testing of multiple URLs
// Returns only working endpoints
// Used in validation step
```

### 5. License Validation

**File:** `src/license/mod.rs`

**Function:** `verify_license_for_wallet(&wallet_pubkey) -> Result<LicenseStatus, String>`

```rust
// Uses global RPC client (initialized from config)
// WebSocket verification with Solana blockchain
// Checks for valid ScreenerBot NFT license
// Returns LicenseStatus { valid, tier, expiry_ts, reason }
// 13+ second operation (blockchain latency)
// Uses cache for 5-minute validity
```

**Verification Flow:**

1. Check cache (5-minute TTL)
2. Fetch wallet's NFT token accounts (decimals=0, amount=1)
3. For each NFT, fetch metadata account
4. Parse Metaplex metadata
5. Verify creator = LICENSE_ISSUER_PUBKEY
6. Validate expiry timestamp
7. Cache result and return

**Called In:**

- **Normal Mode Startup:** `src/run.rs` - Verifies on EVERY bot restart (NEW)
- **Initialization Wizard:** `src/webserver/routes/initialization.rs` - Verifies during first-time setup
- **Test Binary:** `src/bin/test_license_verification.rs` - Manual testing

**Security:** Bot exits with error if license is invalid or expired. No bypass possible.

### 3. Config Persistence

**File:** `src/config/utils.rs`

**Function:** `save_config_to_file(config_path: &Path, config_toml: &str) -> Result<(), String>`

```rust
// Writes config.toml with Unix permissions 0o600
// Ensures secure storage of wallet private key
// Called after successful license verification
```

### 4. Service Manager Extensions

**File:** `src/services/mod.rs`

**Method:** `start_newly_enabled(&mut self) -> Result<ServiceStartupReport, String>`

```rust
// Starts services that were disabled during pre-init
// Respects dependency order via topological sort
// Called after initialization completes
// Returns ServiceStartupReport { started, failures, already_running, total_enabled, duration_ms }
```

### 5. API Endpoints

**File:** `src/webserver/routes/initialization.rs`

#### **GET** `/api/initialization/status`

```rust
StatusResponse {
    required: bool,      // true if config.toml missing
    completed: bool,     // INITIALIZATION_COMPLETE flag
    message: String
}
```

#### **POST** `/api/initialization/validate`

```rust
Request {
    wallet_private_key: String,  // base58 or JSON array
    rpc_urls: Vec<String>
}

Response {
    wallet_address: String,
    working_rpc_urls: Vec<String>,  // Only URLs that passed test
    license_valid: Option<bool>     // None if RPC test failed
}
```

#### **POST** `/api/initialization/complete`

```rust
Request {
    wallet_private_key: String,
    rpc_urls: Vec<String>
}

Response {
    success: bool,
    services_started: usize
}

// Workflow:
// 1. Parse wallet private key
// 2. Test RPC endpoints (parallel)
// 3. Verify license via WebSocket (13s+)
// 4. Return 403 FORBIDDEN if invalid license
// 5. Build config.toml with working URLs only
// 6. Save with 0o600 permissions
// 7. Set INITIALIZATION_COMPLETE = true
// 8. Start newly enabled services
```

#### **GET** `/api/initialization/progress`

```rust
ProgressResponse {
    services_started: usize,
    total_services: usize,
    healthy_services: Vec<String>
}
```

### 6. Middleware

**File:** `src/webserver/middleware.rs`

**Function:** `initialization_gate() -> Middleware`

```rust
// Checks INITIALIZATION_COMPLETE flag
// Allows: /api/initialization/*, /scripts/*, /styles/*, /api/pages/*, root
// Blocks: All other /api/* with 503 SERVICE_UNAVAILABLE
// Message: "System is initializing. Please complete setup at /initialization"
```

---

## Frontend Implementation

### 1. HTML Structure

**File:** `src/webserver/templates/pages/initialization.html`

**Components:**

- Full-screen overlay with gradient background
- 3-step progress indicator (Credentials → Verification → Services)
- Step 1: Wallet private key (textarea with toggle visibility) + RPC URLs (textarea)
- Step 2: Real-time verification status (wallet ✓/⏳/❌, RPC ✓/⏳/❌, license ✓/⏳/❌)
- Step 3: Service startup progress bar with count
- Error display with retry button
- Navigation: Back, Next, Retry buttons

### 2. JavaScript Logic

**File:** `src/webserver/templates/scripts/pages/initialization.js` (565 lines)

#### **State Management**

```javascript
const state = {
  currentStep: 1,
  credentials: { walletPrivateKey: "", rpcUrls: [] },
  validation: { wallet: null, rpc: null },
  verification: { wallet: null, rpc: null, license: null },
  errors: [],
};

// Cleanup references (memory leak prevention)
let servicesPoller = null;
let eventListeners = [];
```

#### **Helper Functions**

```javascript
addTrackedListener(element, event, handler); // Track for cleanup
removeAllListeners(); // Clean all tracked listeners
debounce(func, wait); // 500ms debounce for input validation
show(el) / hide(el); // Element visibility
setStep(step); // Step navigation with UI updates
showValidation(fieldId, type, message); // Real-time validation feedback
resetVerificationStates(); // Clean verification indicators
```

#### **Validation Functions**

```javascript
validateWalletKey(); // Debounced: Check base58 or JSON array format
validateRpcUrls(); // Debounced: Check https://, warn if default Solana RPC
```

#### **API Integration**

```javascript
async validateCredentials() {
  // POST /api/initialization/validate
  // Returns: wallet_address, working_rpc_urls, license_valid
}

async completeInitialization() {
  // POST /api/initialization/complete
  // Sets INITIALIZATION_COMPLETE flag
  // Starts all services
}

async runVerification() {
  // Step 2: Animated verification sequence
  // 1. Wallet parsing (500ms delay)
  // 2. RPC testing (API call)
  // 3. License verification (API call, 13s+)
  // Progress through ⏳ → ✓/❌ for each step
}

async startServicesProgress() {
  // Step 3: Poll /api/services every 1s
  // Update progress bar (servicesStarted/totalServices)
  // Redirect to /services when >= totalServices - 1
  // Cleans up poller on completion or dispose()
}
```

#### **Lifecycle Hooks**

```javascript
init() {
  // Clean up existing listeners first
  // Add tracked listeners for all inputs and buttons
  // Setup password toggle
  // Initialize at step 1
}

dispose() {
  // Remove ALL tracked event listeners
  // Clear services poller
  // Prevent memory leaks
}
```

#### **Button State Management**

```javascript
// Next button during validation:
nextBtn.disabled = true;
nextBtn.textContent = "Validating...";
try {
  await runVerification();
} finally {
  nextBtn.disabled = false;
  nextBtn.textContent = "Next →";
}
```

### 3. CSS Styling

**File:** `src/webserver/templates/styles/pages/initialization.css` (900+ lines)

**Features:**

- Full-screen overlay with gradient background
- Glassmorphism dialog design (backdrop blur, transparent backgrounds)
- Animations: fadeIn, slideUp, fadeInContent, scaleIn (spinner)
- Dark mode support via CSS custom properties
- Responsive design (mobile breakpoints at 768px)
- Password toggle icon button
- Progress bar with smooth transitions
- Verification status indicators with color coding (blue=pending, green=success, red=error)
- Button states (hover, active, disabled)
- Error alert styling with retry button

### 4. Auto-Detection Script

**File:** `src/webserver/templates/base.html`

```javascript
(async function checkInitialization() {
  try {
    const response = await fetch("/api/initialization/status");
    if (!response.ok) {
      console.warn("Failed to fetch initialization status:", response.status);
      return;
    }

    let result;
    try {
      result = await response.json();
    } catch (jsonError) {
      console.error("Failed to parse initialization status JSON:", jsonError);
      return;
    }

    if (result.success && result.data && result.data.required) {
      // Redirect to /initialization (unless already there)
      if (
        !window.location.pathname.includes("/initialization") &&
        !window.location.hash.includes("initialization")
      ) {
        console.log("Initialization required - redirecting to setup page");
        window.location.href = "/initialization";
      }
    }
  } catch (error) {
    console.error("Failed to check initialization status:", error);
  }
})();
```

**Features:**

- Runs on every page load
- Explicit JSON error handling (P1-6 fix)
- Prevents redirect loop
- Silent failure on network errors

---

## State Management

### Global Flags (Thread-Safe)

| Flag                        | Type       | Purpose                              | Set By                         |
| --------------------------- | ---------- | ------------------------------------ | ------------------------------ |
| `INITIALIZATION_COMPLETE`   | AtomicBool | Gates service startup and API access | `/api/initialization/complete` |
| `CONNECTIVITY_SYSTEM_READY` | AtomicBool | RPC health monitoring ready          | Connectivity service           |
| `TOKENS_SYSTEM_READY`       | AtomicBool | Token database ready                 | Tokens service                 |
| `POSITIONS_SYSTEM_READY`    | AtomicBool | Positions verified and loaded        | Positions service              |

### Startup Flow

```
1. Bot starts → Check config.toml exists?
   ├─ NO:  Pre-init mode (INITIALIZATION_COMPLETE = false)
   │       └─ Start webserver only
   │       └─ Wait for initialization
   └─ YES: Normal mode (INITIALIZATION_COMPLETE = true)
           └─ Load configuration
           └─ **VERIFY LICENSE** (NEW: checks on every startup)
           └─ Start all services (only if license valid)

2. License Verification (EVERY STARTUP):
   ├─ Load wallet from config
   ├─ Call verify_license_for_wallet() (13s+ blockchain check)
   ├─ Valid: Continue to service startup
   └─ Invalid: Exit with error message

3. User visits any page → Auto-detection script checks status
   └─ If required=true → Redirect to /initialization

4. User completes wizard (first-time only)
   ├─ Step 1: Enter credentials
   ├─ Step 2: Verify (wallet parse → RPC test → license check)
   └─ Step 3: Services start (progress bar → redirect to /services)

5. Config saved → INITIALIZATION_COMPLETE = true
   └─ Middleware unblocks API endpoints
   └─ ServiceManager starts newly enabled services
```

---

## API Endpoints

### Initialization Endpoints

| Method | Path                           | Auth | Purpose                          |
| ------ | ------------------------------ | ---- | -------------------------------- |
| GET    | `/api/initialization/status`   | None | Check if initialization required |
| POST   | `/api/initialization/validate` | None | Test credentials (wallet + RPC)  |
| POST   | `/api/initialization/complete` | None | Save config and start services   |
| GET    | `/api/initialization/progress` | None | Get service startup progress     |

### Request/Response Types

#### Status Endpoint

```json
GET /api/initialization/status

Response:
{
  "success": true,
  "data": {
    "required": false,
    "completed": true,
    "message": "System is fully initialized"
  }
}
```

#### Validate Endpoint

```json
POST /api/initialization/validate
Content-Type: application/json

Request:
{
  "wallet_private_key": "[1,2,3,...]",  // or base58 string
  "rpc_urls": [
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com"
  ]
}

Response (Success):
{
  "success": true,
  "data": {
    "wallet_address": "5A6Eq...",
    "working_rpc_urls": ["https://api.mainnet-beta.solana.com"],
    "license_valid": true
  }
}

Response (Invalid License):
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",
    "message": "Invalid license for wallet 5A6Eq..."
  }
}
```

#### Complete Endpoint

```json
POST /api/initialization/complete
Content-Type: application/json

Request:
{
  "wallet_private_key": "[1,2,3,...]",
  "rpc_urls": ["https://api.mainnet-beta.solana.com"]
}

Response (Success):
{
  "success": true,
  "data": {
    "message": "Initialization complete",
    "services_started": 17
  }
}

Response (Invalid License):
{
  "success": false,
  "error": {
    "code": "FORBIDDEN",
    "message": "Invalid license. Cannot initialize bot."
  }
}
```

#### Progress Endpoint

```json
GET /api/initialization/progress

Response:
{
  "success": true,
  "data": {
    "services_started": 12,
    "total_services": 18,
    "healthy_services": ["rpc_stats", "events", "sol_price", ...]
  }
}
```

---

## Security Features

### 1. Continuous License Verification

- **Every Startup:** License verified on EVERY bot restart (not just first-time)
- **Cache with TTL:** 5-minute cache prevents excessive blockchain calls during development
- **Blockchain-backed:** Verifies NFT ownership via Solana on-chain data
- **No Bypass:** Bot exits immediately if license invalid or expired
- **Clear Messaging:** Error includes expiry reason and link to purchase/renew

### 2. File Permissions

- Config file written with Unix mode `0o600` (read/write owner only)
- Prevents other users from reading wallet private key

### 3. Ephemeral RPC Clients

- RPC testing uses temporary clients (not stored globally)
- License verification creates ephemeral client during initialization
- No RPC credentials stored in memory after validation

### 4. Middleware Protection

- All API endpoints blocked until initialization complete
- Only `/api/initialization/*` accessible pre-init
- Prevents unauthorized access to trading functions

### 5. No Browser Storage

- Wallet private key never stored in localStorage/sessionStorage
- Credentials sent once during validation/completion
- No sensitive data in browser memory after redirect

### 6. License Validation Details

- WebSocket verification with Solana blockchain
- Cannot bypass via API manipulation
- Returns 403 FORBIDDEN for invalid licenses
- Config not saved if license invalid

### 7. Memory Safety

- All event listeners tracked and cleaned up
- Service poller cleared on page exit
- No memory leaks from repeated page activations

---

## Bug Fixes Applied

### P0 Critical Fixes (All 3 Fixed)

#### **P0-1: Services Poller Memory Leak**

**Problem:** `setInterval` poller in `startServicesProgress()` only cleared on completion, causing infinite API calls if user navigates away.

**Fix:**

```javascript
// Global reference
let servicesPoller = null;

async function startServicesProgress() {
  // Clear existing poller
  if (servicesPoller) {
    clearInterval(servicesPoller);
    servicesPoller = null;
  }

  servicesPoller = setInterval(async () => {
    // ... polling logic
  }, 1000);
}

// Cleanup in dispose()
dispose() {
  if (servicesPoller) {
    clearInterval(servicesPoller);
    servicesPoller = null;
  }
}
```

#### **P0-2: Event Listener Stacking**

**Problem:** Event listeners added in `init()` but only 2 removed in `dispose()`, causing duplicates on page re-activation.

**Fix:**

```javascript
let eventListeners = [];

function addTrackedListener(element, event, handler) {
  if (element) {
    element.addEventListener(event, handler);
    eventListeners.push({ element, event, handler });
  }
}

function removeAllListeners() {
  eventListeners.forEach(({ element, event, handler }) => {
    if (element) element.removeEventListener(event, handler);
  });
  eventListeners = [];
}

// Use in init()
addTrackedListener(walletInput, "input", validateWalletKey);
addTrackedListener(nextBtn, "click", nextHandler);
// ... all listeners tracked

// Clean in dispose()
dispose() {
  removeAllListeners();  // Removes ALL listeners
}
```

#### **P0-3: Concurrent API Calls**

**Problem:** User could spam-click "Next" before debounce/API responds, causing race conditions.

**Fix:**

```javascript
const nextHandler = async () => {
  // Disable button immediately
  nextBtn.disabled = true;
  nextBtn.textContent = "Validating...";

  try {
    setStep(2);
    await runVerification();
  } catch (error) {
    showError(error.message);
    setStep(1); // Go back on error
  } finally {
    // Re-enable after completion
    nextBtn.disabled = false;
    nextBtn.textContent = "Next →";
  }
};
```

### P1 High-Priority Fixes (All 4 Fixed)

#### **P1-4: Verification State Cleanup**

**Problem:** Going back to step 1 after verification left animated states active.

**Fix:**

```javascript
function resetVerificationStates() {
  // Reset all verification indicators
  ["wallet-status", "rpc-status", "license-status"].forEach((id) => {
    const el = $(`#${id}`);
    if (el) {
      el.className = "init-verification-status";
      el.innerHTML = '<span class="init-spinner"></span><span>Pending</span>';
    }
  });

  // Reset password visibility
  const input = $("#wallet-private-key");
  if (input) {
    input.style.webkitTextSecurity = "disc";
    input.style.textSecurity = "disc";
  }
}

// Called when going back
const backHandler = () => {
  if (state.currentStep > 1) {
    resetVerificationStates(); // Clean UI
    setStep(state.currentStep - 1);
  }
};
```

#### **P1-5: Hard-coded Service Count**

**Problem:** `const totalServices = 20` was hard-coded, causing inaccurate progress if services changed.

**Fix:**

```javascript
async function startServicesProgress() {
  let servicesStarted = 0;
  let totalServices = 20; // Initial estimate

  servicesPoller = setInterval(async () => {
    const result = await response.json();

    // Update total from first response
    if (totalServices === 20) {
      totalServices = result.data.services.length;
    }

    const progress = (servicesStarted / totalServices) * 100;
    // ... update UI
  }, 1000);
}
```

#### **P1-6: Unsafe JSON Parsing**

**Problem:** Auto-detection script had no specific error handling for JSON parsing failures.

**Fix:**

```javascript
// In base.html
try {
  const response = await fetch("/api/initialization/status");
  if (!response.ok) {
    console.warn("Failed to fetch:", response.status);
    return;
  }

  let result;
  try {
    result = await response.json(); // Explicit try-catch
  } catch (jsonError) {
    console.error("JSON parse error:", jsonError);
    return;
  }

  if (result.success && result.data && result.data.required) {
    // Redirect logic
  }
} catch (error) {
  console.error("Network error:", error);
}
```

#### **P1-7: No Initialization Timeout**

**Problem:** Infinite loop polling flag every 500ms with no timeout, causing deadlock if frontend bugs.

**Fix:**

```rust
async fn wait_for_initialization_or_shutdown() -> Result<(), String> {
    const MAX_WAIT_DURATION: Duration = Duration::from_secs(30 * 60); // 30 min
    const WARNING_INTERVAL: Duration = Duration::from_secs(5 * 60);   // 5 min

    let start = Instant::now();
    let mut last_warning = start;

    loop {
        if global::is_initialization_complete() {
            return Ok(());
        }

        let elapsed = start.elapsed();

        // Timeout after 30 minutes
        if elapsed >= MAX_WAIT_DURATION {
            logger::error(LogTag::System, "Initialization timeout");
            return Err("Initialization timeout after 30 minutes".to_string());
        }

        // Warn every 5 minutes
        if elapsed - (last_warning - start) >= WARNING_INTERVAL {
            logger::warning(
                LogTag::System,
                &format!("Still waiting... ({} min)", elapsed.as_secs() / 60)
            );
            last_warning = Instant::now();
        }

        // Poll every 500ms
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                return Err("Shutdown during initialization".to_string());
            }
            _ = sleep(Duration::from_millis(500)) => {}
        }
    }
}
```

---

## Testing Guide

### 1. Pre-Testing Setup

```bash
# Ensure no config exists
rm -f data/config.toml

# Kill any running instances
pkill -f screenerbot

# Build with latest changes
cargo build

# Start bot in background with logging
nohup cargo run --bin screenerbot -- --run --dry-run > logs/init_test.log 2>&1 &

# Wait for webserver startup (15-20s)
sleep 20
```

### 2. Manual Testing Steps

#### **Test 1: Auto-Detection**

1. Open browser to `http://localhost:8080`
2. Should automatically redirect to `/initialization`
3. Verify 3-step progress indicator visible
4. Verify gradient background and glassmorphism dialog

#### **Test 2: Input Validation**

1. Step 1: Enter invalid wallet key → See error message
2. Enter valid base58 or JSON array → See ✓ success
3. Enter RPC URLs without https:// → See error
4. Enter default Solana RPC → See warning
5. Enter valid premium RPC → See ✓ success

#### **Test 3: Password Toggle**

1. Default: Wallet key should be hidden (dots)
2. Click eye icon → Should reveal text
3. Click again → Should hide text

#### **Test 4: Navigation**

1. Click "Next" without filling fields → See error
2. Fill fields → Click "Next" → Move to step 2
3. Click "Back" → Return to step 1
4. Verify verification states reset (no stale animations)

#### **Test 5: Verification Sequence**

1. Fill valid credentials → Click "Next"
2. Step 2: Watch animated verification:
   - Wallet status: ⏳ → ✓ (500ms)
   - RPC status: ⏳ → ✓ (variable)
   - License status: ⏳ → ✓/❌ (13s+)
3. Invalid license: See ❌, error message, retry button
4. Valid license: Proceed to step 3

#### **Test 6: Service Startup**

1. Step 3: Progress bar should animate
2. Status text: "Initializing services (X/Y)..."
3. Count should increase every second
4. When complete: "Complete! Redirecting..."
5. Auto-redirect to `/services` page

#### **Test 7: Error Handling**

1. Stop bot during verification → See error, retry button
2. Click "Retry" → Return to step 1, clean state
3. Enter invalid license → See 403 error, cannot proceed

#### **Test 8: Memory Leak Prevention**

1. Start wizard, get to step 3 (services polling)
2. Navigate away using browser back button
3. Return to initialization page
4. Verify no duplicate API calls in browser DevTools Network tab
5. Check no memory leak in browser Task Manager

### 3. Automated Testing (Playwright MCP)

```bash
# Start bot in background
nohup cargo run --bin screenerbot -- --run --dry-run > logs/bot_test.log 2>&1 &
sleep 20

# Use Playwright MCP tools:
# 1. mcp_playwright_browser_navigate("http://localhost:8080")
# 2. mcp_playwright_browser_snapshot() - Verify redirect to /initialization
# 3. mcp_playwright_browser_click() - Fill form fields
# 4. mcp_playwright_browser_type() - Enter credentials
# 5. mcp_playwright_browser_console_messages() - Check for JS errors
# 6. mcp_playwright_browser_take_screenshot() - Visual verification
```

### 4. Validation Checklist

- [ ] Config.toml removed before test
- [ ] Bot starts without errors
- [ ] Webserver accessible at :8080
- [ ] Auto-redirect to /initialization works
- [ ] All form validation works
- [ ] Password toggle works
- [ ] Back/Next navigation works
- [ ] Verification sequence completes
- [ ] Invalid license blocked (403)
- [ ] Valid license proceeds
- [ ] Service progress updates
- [ ] Auto-redirect to /services
- [ ] Config.toml created with 0o600
- [ ] All services start successfully
- [ ] No JavaScript errors in console
- [ ] No memory leaks (check DevTools)

---

## Troubleshooting

### Issue: Bot doesn't start

**Symptoms:** `cargo run` fails or exits immediately

**Solutions:**

1. Check logs: `tail -f logs/screenerbot_*.log`
2. Verify no other instance running: `pkill -f screenerbot`
3. Check port 8080 available: `lsof -i :8080`
4. Ensure data/ directory exists: `mkdir -p data/`

### Issue: Webserver not accessible

**Symptoms:** Cannot connect to `http://localhost:8080`

**Solutions:**

1. Wait 15-20s after startup (services take time)
2. Check initialization mode: Should see "starting in initialization mode" in logs
3. Verify no firewall blocking port 8080
4. Check webserver service started: `GET /api/services`

### Issue: Auto-redirect not working

**Symptoms:** Stays on root page instead of redirecting to /initialization

**Solutions:**

1. Check browser console for JavaScript errors
2. Verify `/api/initialization/status` returns `required: true`
3. Clear browser cache and reload
4. Check middleware allows `/api/initialization/*` paths

### Issue: Validation fails

**Symptoms:** Cannot proceed past step 1, validation errors

**Solutions:**

1. Wallet key format: Must be base58 string OR JSON array `[1,2,3,...]`
2. RPC URLs: Must start with `https://`
3. Avoid default Solana RPC (won't work)
4. Test RPC manually: `curl https://your-rpc-url -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'`

### Issue: License verification hangs

**Symptoms:** Stuck on "Verifying license..." for minutes

**Solutions:**

1. License verification takes 13+ seconds (normal)
2. Check RPC endpoints are working (not rate-limited)
3. Verify wallet has license NFT on-chain
4. Check logs for WebSocket errors
5. Timeout after 30s should trigger error state

### Issue: Services not starting

**Symptoms:** Progress bar stuck at 0 or low count

**Solutions:**

1. Check logs for service startup errors
2. Verify `INITIALIZATION_COMPLETE` flag set: Check `/api/initialization/status`
3. Ensure config.toml saved correctly: `cat data/config.toml`
4. Check service dependencies in ServiceManager
5. Look for errors in `/api/services` response

### Issue: Memory leak / Performance degradation

**Symptoms:** Browser slows down, high memory usage

**Solutions:**

1. Check browser DevTools → Performance → Memory
2. Verify services poller cleared: Should see only 1 request/second in Network tab
3. Check event listeners: Should be same count after page reload
4. Clear browser cache and restart
5. Update to latest code (all P0 fixes applied)

### Issue: Config.toml has wrong permissions

**Symptoms:** Warning about file permissions, security concerns

**Solutions:**

1. Check permissions: `ls -la data/config.toml`
2. Should be `-rw-------` (0o600)
3. Fix manually: `chmod 600 data/config.toml`
4. Verify save_config_to_file() called correctly

### Issue: Cannot test without valid license

**Symptoms:** Need to test UI but don't have license

**Solutions:**

1. Temporarily modify `verify_license_for_wallet_with_endpoints()` to return true
2. Or add test license to wallet on devnet
3. Or mock the validation endpoint in development
4. **Note:** Production code requires valid license

---

## Code Statistics

### Lines of Code Added

- **Rust (Backend):** ~800 lines
  - `initialization.rs`: 493 lines
  - `middleware.rs`: 70 lines
  - `run.rs`: +100 lines
  - Other files: +137 lines

- **JavaScript (Frontend):** ~565 lines
  - `initialization.js`: 565 lines (including lifecycle)

- **CSS (Styling):** ~900 lines
  - `initialization.css`: 900+ lines (including animations)

- **HTML (Structure):** ~200 lines
  - `initialization.html`: ~200 lines

**Total:** ~2,500+ lines of new code

### Files Modified

- **Modified:** 10 files
- **Created:** 5 files
- **Total Files Affected:** 15 files

### Testing Coverage

- **Manual Tests:** 8 test scenarios
- **Automated Tests:** Playwright MCP ready
- **Edge Cases:** 11 troubleshooting scenarios documented

---

## Conclusion

The License-Gated Initialization Architecture provides a secure, user-friendly first-time setup experience for ScreenerBot. All critical bugs have been fixed, memory leaks prevented, and the system is production-ready.

**Key Achievements:**
✅ Zero-config first launch  
✅ Progressive 3-step wizard  
✅ License validation with blockchain  
✅ Secure credential handling (0o600)  
✅ Memory-safe (no leaks)  
✅ Auto-detection and redirect  
✅ Service orchestration with dependencies  
✅ Comprehensive error handling  
✅ Mobile-responsive UI  
✅ Full documentation

**Status:** Ready for production deployment and user testing.

---

**Document Version:** 1.0  
**Last Updated:** November 4, 2025  
**Author:** ScreenerBot Development Team
