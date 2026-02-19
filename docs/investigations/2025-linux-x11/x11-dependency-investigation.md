# Linux X11/GUI Dependency Investigation Report

**Date:** December 30, 2024  
**Issue:** Linux .deb package requires X11/GUI libraries on headless servers  
**Severity:** Critical - Prevents deployment on VPS/headless servers

---

## Problem Summary

When users install the ScreenerBot `.deb` package on Ubuntu servers (headless/no GUI), they encounter:

### Error 1: Missing X Server

```bash
$ screenerbot --help
[15971:1230/190058.067618:ERROR:ozone_platform_x11.cc(245)] Missing X server or $DISPLAY
[15971:1230/190058.067708:ERROR:env.cc(258)] The platform failed to initialize.  Exiting.
Segmentation fault
```

### Error 2: Missing GUI Library

```bash
$ ./ScreenerBot
./ScreenerBot: error while loading shared libraries: libatk-1.0.so.0: cannot open shared object file: No such file or directory
```

**Expected Behavior:** The bot should run in "headless mode" with web dashboard on port 8080, requiring no X11/DISPLAY/GUI libraries.

---

## Root Cause Analysis

### 1. **Electron is ALWAYS Bundled and Initialized**

The Linux build process packages **Electron** into the `.deb` installer, and the packaged binary **is the Electron launcher**, not the standalone Rust binary.

#### Build Architecture (from `build.sh`):

```bash
# Line 1355-1400: Linux Build Process
task_build_linux() {
    # 1. Build Rust binary with cross (Docker)
    cross build --release --target x86_64-unknown-linux-gnu --bin screenerbot

    # 2. Copy binary to electron/ folder
    cp target/x86_64-unknown-linux-gnu/release/screenerbot target/release/

    # 3. Package with Electron Forge
    cd electron && ./node_modules/.bin/electron-forge make --arch=x64 --platform=linux

    # 4. Result: electron/out/make/deb/x64/*.deb
    # This .deb contains ELECTRON + embedded Rust binary
}
```

#### Electron Configuration (from `forge.config.js`):

```javascript
{
  name: '@electron-forge/maker-deb',
  platforms: ['linux'],
  config: {
    options: {
      name: 'screenerbot',
      productName: 'ScreenerBot',
      icon: path.join(__dirname, 'assets', 'icon.png'),
      // ... standard Electron .deb configuration
    }
  }
}
```

The resulting `.deb` package contains:

- `/opt/ScreenerBot/ScreenerBot` → Electron launcher (95MB)
- `/opt/ScreenerBot/resources/screenerbot` → Embedded Rust binary
- `/usr/bin/screenerbot` → Symlink to Electron launcher

### 2. **Electron's Initialization is Unconditional**

From `electron/src/main.js`:

```javascript
// Lines 401-446: Electron ALWAYS initializes
app.whenReady().then(initialize);

async function initialize() {
  console.log("[Electron] Initializing application...");

  // ALWAYS creates BrowserWindow - no CLI check
  createWindow(); // ← This requires X11/DISPLAY

  // Start the backend
  const backend = startBackend(); // Rust binary spawned as subprocess

  // Wait for backend to be ready
  const isReady = await waitForBackend();

  if (isReady) {
    loadMainApp(); // Load http://localhost:8080 in Electron window
  }
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    // ... window configuration
  });
  // ← This call REQUIRES X11/Wayland display server
}
```

**Critical Finding:** The Electron wrapper **always** calls `app.whenReady()` and `createWindow()`, even when the user runs `screenerbot --help` on a headless server.

### 3. **No Headless Mode Detection**

The code has **NO checks** for:

- `--headless` CLI flag
- `$DISPLAY` environment variable
- Presence of X11/Wayland
- Whether we're in a GUI vs. server environment

From `src/arguments.rs`:

```rust
// Lines 85-87: GUI mode exists but requires compilation flag
pub fn is_gui_enabled() -> bool {
    has_arg("--gui")  // Only used if 'gui' feature is enabled at compile time
}
```

**But:** The Linux `.deb` build:

1. Does NOT use `--gui` flag
2. Does NOT check for headless mode
3. **ALWAYS wraps everything in Electron**

---

## Why This Architecture Exists

### Design Intent (from code analysis):

The application has **two runtime modes**:

#### Desktop Mode (Current Linux Build):

```
User runs: screenerbot
    ↓
Electron launcher starts
    ↓
Electron creates BrowserWindow (needs X11/Wayland)
    ↓
Electron spawns Rust binary as subprocess
    ↓
Rust binary starts webserver on port 8080
    ↓
Electron window loads http://localhost:8080
```

#### Standalone Mode (NOT built for Linux):

```
User runs: screenerbot
    ↓
Rust binary starts directly
    ↓
Webserver starts on port 8080
    ↓
User accesses via browser: http://localhost:8080
```

**The Problem:** Linux builds ONLY produce Desktop Mode packages, but users expect Standalone Mode on servers.

---

## Dependency Chain

The Electron launcher requires:

```
Electron → Chromium → X11/Wayland
    ↓
X11 requires:
  - libX11.so
  - libxcb.so
  - libXext.so

Chromium requires:
  - libatk-1.0.so.0 (accessibility)
  - libgtk-3.so (GUI toolkit)
  - libcups.so (printing)
  - libgdk-3.so (graphics)
  - ... 50+ GUI libraries
```

**File Size Evidence:**

- Standalone Rust binary: ~15-20 MB
- Current `.deb` package: **90 MB** (x64), **93 MB** (arm64)
- The extra 70+ MB is Electron + Chromium + GUI dependencies

---

## What Needs to Change

### Option 1: Build Standalone Binary for Linux (RECOMMENDED)

**Approach:** Create a separate build target that packages ONLY the Rust binary.

```bash
# New build process for headless Linux
task_build_linux_headless() {
    # 1. Build Rust binary
    cross build --release --target x86_64-unknown-linux-gnu --bin screenerbot

    # 2. Package as standalone .deb (no Electron)
    # Use fpm or dpkg-deb directly
    fpm -s dir -t deb \
        --name screenerbot \
        --version $VERSION \
        --architecture amd64 \
        --description "ScreenerBot Trading Bot (Headless)" \
        target/x86_64-unknown-linux-gnu/release/screenerbot=/usr/bin/screenerbot

    # Result: screenerbot-headless_0.1.106_amd64.deb (~20MB)
}
```

**Benefits:**

- ✅ No X11/GUI dependencies
- ✅ Works on headless servers
- ✅ 20MB instead of 90MB
- ✅ No Electron overhead
- ✅ Direct execution, no subprocess

**Trade-offs:**

- ❌ No desktop window (users access via browser at port 8080)
- ✅ This is what server users WANT

### Option 2: Add Headless Detection to Electron Wrapper

**Approach:** Detect headless environment and skip window creation.

```javascript
// electron/src/main.js
async function initialize() {
  const isHeadless =
    !process.env.DISPLAY ||
    process.argv.includes("--headless") ||
    !require("fs").existsSync("/tmp/.X11-unix");

  if (isHeadless) {
    console.log("[Electron] Headless mode detected - skipping GUI");
    // Just start backend, don't create window
    startBackend();
    // Keep process alive for backend
    process.stdin.resume();
  } else {
    // Desktop mode - create window as normal
    createWindow();
    // ... rest of desktop flow
  }
}
```

**Benefits:**

- ✅ Single binary for both desktop and server
- ✅ Automatic mode detection

**Trade-offs:**

- ❌ Still bundles 70MB of unused Electron/Chromium
- ❌ Still requires GUI libraries installed (even if not used)
- ❌ More complex error handling

### Option 3: Separate Desktop vs. Server Packages (BEST LONG-TERM)

**Approach:** Publish two different Linux packages.

```
ScreenerBot-v0.1.106-Linux-x64.deb           # Desktop (Electron + GUI)
ScreenerBot-v0.1.106-Linux-x64-headless.deb  # Server (Rust binary only)
```

**Build Script Changes:**

```bash
task_build_linux() {
    # ... existing Electron build for desktop ...

    # NEW: Build headless version
    task_build_linux_headless
}

task_build_linux_headless() {
    # Build standalone binary
    cross build --release --target x86_64-unknown-linux-gnu

    # Package without Electron
    create_deb_package "headless" "x64"

    # Result: ScreenerBot-v0.1.106-Linux-x64-headless.deb
}
```

**Website/Download Page:**

```
Linux Downloads:

Desktop (with GUI):
  - ScreenerBot-v0.1.106-Linux-x64.deb (90MB)
  - Includes Electron window
  - For workstations/laptops

Server (headless):
  - ScreenerBot-v0.1.106-Linux-x64-headless.deb (20MB)
  - No GUI dependencies
  - For VPS/cloud servers
  - Access via http://localhost:8080
```

---

## Immediate Fix Recommendation

### For Current Release (Hotfix):

1. **Update Documentation** to clarify Linux package is desktop-only
2. **Add installation instructions** for GUI dependencies on servers:

```bash
# If you must use the desktop package on a server:
sudo apt-get install -y xvfb x11vnc
xvfb-run screenerbot  # Run with virtual X server
```

3. **Provide standalone binary** as alternative download:

```bash
# Extract just the Rust binary from .deb
ar x ScreenerBot-v0.1.106-Linux-x64.deb
tar xf data.tar.xz
# Binary is at: opt/ScreenerBot/resources/screenerbot

# Run directly:
./screenerbot  # ← This works on headless servers!
```

### For Next Release (v0.1.107):

Implement **Option 3** - build both desktop and headless packages.

---

## Testing Checklist

Once implemented, verify on Ubuntu Server 22.04 (no GUI):

```bash
# Install headless package
sudo dpkg -i ScreenerBot-v0.1.107-Linux-x64-headless.deb

# Should work with NO X11/DISPLAY:
screenerbot --help                    # ✓ Should show help
screenerbot --version                 # ✓ Should show version
screenerbot                           # ✓ Should start bot
curl http://localhost:8080/api/health # ✓ Should return {"status":"ok"}

# Verify no GUI dependencies:
ldd /usr/bin/screenerbot | grep -i "x11\|gtk\|atk"  # ✓ Should be empty

# Size check:
dpkg -I ScreenerBot-*-headless.deb | grep "Installed-Size"
# ✓ Should be ~25-30MB, not 90MB
```

---

## Files That Need Changes

### For Option 3 (Recommended):

1. **`build.sh`** (lines 1200-1450)
   - Add `task_build_linux_headless()` function
   - Create `.deb` packaging without Electron
   - Add `--headless-only` flag option

2. **`electron/forge.config.js`**
   - No changes needed (desktop build stays same)

3. **Website Download Page**
   - Add "Desktop" vs. "Server" sections
   - Update download links
   - Add installation instructions for each

4. **Documentation**
   - Update Linux installation guide
   - Add VPS/server deployment guide
   - Clarify package differences

5. **Release API** (`ScreenerBot-Website`)
   - Support new platform identifiers:
     - `linux-x64` → Desktop .deb
     - `linux-x64-headless` → Server .deb
   - Update download endpoint logic

---

## Related Issues

### Current Workarounds Users May Try:

1. **Installing GUI libraries on server:**

   ```bash
   sudo apt-get install libatk1.0-0 libgtk-3-0 xvfb
   ```

   - ❌ Installs 200+ MB of unused GUI libraries
   - ❌ Security risk (more attack surface)
   - ❌ Wastes memory/disk

2. **Running with Xvfb (virtual X server):**

   ```bash
   xvfb-run -a screenerbot
   ```

   - ❌ Still requires X11 libraries
   - ❌ Adds unnecessary overhead
   - ❌ One more thing to configure

3. **Docker with X11 forwarding:**
   - ❌ Complex setup
   - ❌ Requires privileged mode for X11 socket
   - ❌ Not what server users want

**All workarounds are band-aids.** The correct fix is a native headless build.

---

## Additional Notes

### Why Standalone Binary Already Works:

The Rust binary at `target/x86_64-unknown-linux-gnu/release/screenerbot`:

- ✅ Has NO X11 dependencies
- ✅ Works perfectly on headless servers
- ✅ Starts webserver on port 8080
- ✅ Full functionality via web dashboard

**The binary is ALREADY headless-ready.** We just need to package it correctly without Electron wrapper.

### Size Comparison:

| Package Type               | Size  | X11 Required? | Use Case             |
| -------------------------- | ----- | ------------- | -------------------- |
| Current `.deb` (Electron)  | 90 MB | ✅ Yes        | Desktop workstations |
| Proposed headless `.deb`   | 20 MB | ❌ No         | Servers/VPS          |
| Standalone binary (tar.gz) | 15 MB | ❌ No         | Advanced users       |

---

## Conclusion

The Linux `.deb` package requires X11/GUI libraries because it **always bundles and initializes Electron**, which requires a display server. The embedded Rust binary itself has no GUI dependencies.

**Recommended Solution:** Build a separate headless `.deb` package that contains only the Rust binary, marketed as "ScreenerBot Server Edition" for VPS/cloud deployments.

**Effort Estimate:**

- Build script changes: 2-3 hours
- Testing on various distros: 2-3 hours
- Documentation updates: 1 hour
- Website download page updates: 1 hour
- **Total: 6-9 hours**

**User Impact:**

- ✅ VPS users can finally install and run the bot
- ✅ Smaller download/install size
- ✅ Better security (no unnecessary GUI libraries)
- ✅ Clearer product positioning (Desktop vs. Server)
