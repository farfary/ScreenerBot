const { app, BrowserWindow, ipcMain, shell, Tray, Menu, dialog, nativeImage } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const http = require('http');
const fs = require('fs');
const os = require('os');
const { pathToFileURL } = require('url');
const appPaths = require('./paths');
const coreResolver = require('./core_resolver');

// ============================================================================
// EPIPE PROTECTION - Prevent crashes when stdout/stderr pipes break
// ============================================================================
process.stdout?.on('error', (err) => { if (err.code === 'EPIPE') return; });
process.stderr?.on('error', (err) => { if (err.code === 'EPIPE') return; });

// ============================================================================
// SINGLE INSTANCE LOCK - Must be checked FIRST before any other initialization
// ============================================================================
const gotTheLock = app.requestSingleInstanceLock();
if (!gotTheLock) {
  console.log('[Electron] Another instance is already running, quitting...');
  app.quit();
  // Exit immediately to prevent any further code execution
  process.exit(0);
}

// Configuration
const CONFIG = {
  port: null, // Dynamic - set when backend reports ready
  host: '127.0.0.1',
  healthEndpoint: '/api/health',
  pollInterval: 1000,
  maxWaitTime: 120000, // 2 minutes max wait
  windowWidth: 1400,
  windowHeight: 900,
  minWidth: 1200,
  minHeight: 700
};

// Rust uses this exit code only after a requested restart has completed its
// graceful service shutdown. Electron must relaunch so it remains the owner of
// the backend child (and can still stop it when the desktop app quits).
const BACKEND_RESTART_EXIT_CODE = 75;

// Promo Studio: the owner-only capture driver sets this before launching the app. Nothing
// about the capture bridge is loaded, listening or reachable without it.
const PROMO_CONTROL = process.env.SCREENERBOT_PROMO_CONTROL === '1';
const promoBridge = PROMO_CONTROL ? require('./promo_bridge') : null;
let promoBridgePort = null;

let mainWindow = null;
let backendProcess = null;
let isQuitting = false;
let tray = null;
let isExitDialogOpen = false; // Guard flag to prevent multiple exit dialogs
let backendReadyResolve = null; // Promise resolver for SCREENERBOT_READY signal
let backendReadySignal = false; // Latches readiness if stdout wins the waitForBackend race
let startupError = null; // Structured fatal startup error (SCREENERBOT_ERROR payload), if any
let bootErrorShown = false; // True once a boot-error screen has actually been rendered
let isRecovering = false; // True while a one-click recovery (e.g. wallet reset) is in progress
let dashboardLoaded = false; // True once the dashboard URL has been loaded successfully
let currentTheme = 'dark'; // Last-run UI theme ('light'|'dark'), persisted in window-state.json
let backendRestartRequested = false;
let backendRestartTarget = '/home';

// Which core binary the current backend child was launched from. A staged core
// that never reports ready is quarantined and the bundled binary takes over, so
// a bad silent update can cost one extra launch and nothing more.
let activeCore = { path: null, version: null, staged: false, reason: '' };

// Every backend spawn takes the next generation. A boot sequence that is still
// awaiting an older generation has been superseded (by a restart, a recovery, or
// a staged-core rollback) and must not report its own failure — otherwise its
// 120s timeout later overwrites the screen the newer launch already painted.
let launchGeneration = 0;

/**
 * Revision of this shell build, generated at package time by
 * `electron/tools/shell-revision.js`. The backend compares it against the
 * revision a release was built with: equal means Electron did not change and the
 * update is core-only. Read directly (not through tools/) so it works from the
 * packaged asar.
 */
const SHELL_REVISION = (() => {
  try {
    const value = JSON.parse(
      fs.readFileSync(path.join(__dirname, 'shell_revision.json'), 'utf8')
    ).revision;
    return typeof value === 'string' && /^[0-9a-f]{8,64}$/.test(value) ? value : 'dev';
  } catch (_) {
    return 'dev';
  }
})();

const SCREENERBOT_BASE_DIR = appPaths.resolveBaseDirectory();
const CORE_DIR = appPaths.coreDirectory(SCREENERBOT_BASE_DIR);

function openScreenerBotLogs() {
  const logsPath = appPaths.logsDirectory(SCREENERBOT_BASE_DIR);
  shell.openPath(fs.existsSync(logsPath) ? logsPath : SCREENERBOT_BASE_DIR);
}

function isSafeExternalUrl(rawUrl) {
  try {
    const protocol = new URL(rawUrl).protocol;
    return protocol === 'https:' || protocol === 'http:';
  } catch (_) {
    return false;
  }
}

function isDashboardUrl(rawUrl) {
  if (!CONFIG.port) return false;
  try {
    const url = new URL(rawUrl);
    return url.protocol === 'http:'
      && url.hostname === CONFIG.host
      && url.port === String(CONFIG.port);
  } catch (_) {
    return false;
  }
}

function currentDashboardRoute() {
  try {
    const url = new URL(mainWindow?.webContents.getURL() || '');
    if (!isDashboardUrl(url.href)) return '/home';
    url.searchParams.delete('electron');
    url.searchParams.delete('theme');
    const search = url.searchParams.toString();
    return `${url.pathname}${search ? `?${search}` : ''}${url.hash}`;
  } catch (_) {
    return '/home';
  }
}

function openExternalUrl(rawUrl) {
  if (!isSafeExternalUrl(rawUrl)) {
    console.warn('[Electron] Blocked unsupported external URL:', rawUrl);
    return;
  }
  shell.openExternal(rawUrl).catch(err => {
    console.error('[Electron] Failed to open external URL:', err.message);
  });
}

function isLoadingPageUrl(rawUrl) {
  return rawUrl === pathToFileURL(path.join(__dirname, 'index.html')).href;
}

function getBackendExtraArgs() {
  const raw = process.env.SCREENERBOT_EXTRA_ARGS || '';
  return raw.split(/\s+/).map(arg => arg.trim()).filter(Boolean);
}

// Window/splash base color per theme — matches the dashboard's title-bar/header
// surface (header.css --titlebar-bg / --header-solid-bg: light #f1f5f9, dark
// #0d1117). titleBarStyle 'hiddenInset' makes the native title bar transparent,
// so this color is what shows under the traffic lights in any unpainted frame
// (pre-paint, renderer reload, resize) — keeping it identical to the title-bar
// element means the top of the window never changes color, in either theme.
function themeBackgroundColor(theme) {
  return theme === 'light' ? '#f1f5f9' : '#0d1117';
}

/**
 * Get the path to tray icon based on platform
 */
function getTrayIconPath() {
  // Use different sizes based on platform
  // Windows: 16x16 or 32x32 ICO
  // macOS: 16x16 or 22x22 PNG (template image)
  // Linux: 16x16 or 22x22 PNG
  const iconName = process.platform === 'win32' ? 'icon.ico' : 'icon.png';
  return path.join(__dirname, '..', 'assets', iconName);
}

/**
 * Create the system tray icon (Windows/Linux only)
 */
function createTray() {
  // macOS uses dock, not system tray
  if (process.platform === 'darwin') {
    return;
  }
  
  const iconPath = getTrayIconPath();
  
  // Validate icon file exists
  if (!fs.existsSync(iconPath)) {
    console.error('[Electron] Tray icon not found:', iconPath);
    return;
  }
  
  // Create tray icon - resize for better visibility
  let trayIcon;
  try {
    trayIcon = nativeImage.createFromPath(iconPath);
    // Check if icon loaded successfully
    if (trayIcon.isEmpty()) {
      console.error('[Electron] Tray icon is empty/invalid:', iconPath);
      return;
    }
    // Resize to appropriate size for system tray
    trayIcon = trayIcon.resize({ width: 16, height: 16 });
  } catch (err) {
    console.error('[Electron] Failed to load tray icon:', err);
    return;
  }
  
  tray = new Tray(trayIcon);
  tray.setToolTip('ScreenerBot - Solana Trading Bot');
  
  const contextMenu = Menu.buildFromTemplate([
    {
      label: 'Show ScreenerBot',
      click: () => {
        if (mainWindow) {
          mainWindow.show();
          mainWindow.focus();
        }
      }
    },
    { type: 'separator' },
    {
      label: 'Open Dashboard',
      click: () => {
        if (mainWindow) {
          mainWindow.show();
          mainWindow.focus();
        }
      }
    },
    {
      label: 'Open Data Folder',
      click: () => {
        shell.openPath(appPaths.dataDirectory(SCREENERBOT_BASE_DIR));
      }
    },
    {
      label: 'Open Logs Folder',
      click: openScreenerBotLogs
    },
    { type: 'separator' },
    {
      label: 'Documentation',
      click: async () => {
        await shell.openExternal('https://screenerbot.io/docs');
      }
    },
    {
      label: 'Telegram Support',
      click: async () => {
        await shell.openExternal('https://t.me/screenerbotio_support');
      }
    },
    { type: 'separator' },
    {
      label: 'Quit ScreenerBot',
      click: () => {
        isQuitting = true;
        app.quit();
      }
    }
  ]);
  
  tray.setContextMenu(contextMenu);
  
  // Double-click to show window (Windows)
  tray.on('double-click', () => {
    if (mainWindow) {
      mainWindow.show();
      mainWindow.focus();
    }
  });
  
  console.log('[Electron] System tray created');
}

/**
 * Show exit confirmation dialog (Windows/Linux only)
 * Returns: 'minimize' | 'quit' | 'cancel'
 */
async function showExitDialog() {
  // Prevent multiple dialogs from stacking
  if (isExitDialogOpen) {
    return 'cancel';
  }
  isExitDialogOpen = true;
  
  try {
    const result = await dialog.showMessageBox(mainWindow, {
      type: 'question',
      buttons: ['Minimize to Tray', 'Quit Completely', 'Cancel'],
      defaultId: 0,
      cancelId: 2,
      title: 'Close ScreenerBot',
      message: 'What would you like to do?',
      detail: 'ScreenerBot can continue running in the background. The trading bot will keep monitoring and trading while minimized to the system tray.',
      icon: nativeImage.createFromPath(path.join(__dirname, '..', 'assets', 'icon.png'))
    });
    
    switch (result.response) {
      case 0: return 'minimize';
      case 1: return 'quit';
      default: return 'cancel';
    }
  } finally {
    isExitDialogOpen = false;
  }
}

/**
 * Pick the core binary for this launch: the one staged by a silent update when
 * it verifies and is newer, otherwise the one shipped inside this bundle.
 *
 * Resolution happens immediately before every spawn, which is what makes a
 * backend restart the activation step of a core update.
 */
async function resolveBackendBinary() {
  const bundledPath = getBinaryPath();
  try {
    const resolved = await coreResolver.resolveCore({
      coreDir: CORE_DIR,
      bundledPath,
      bundledVersion: app.getVersion(),
    });
    if (resolved.staged) {
      console.log(`[Electron] Launching staged core v${resolved.version} (${resolved.path})`);
    } else if (resolved.reason) {
      console.log(`[Electron] Launching bundled core: ${resolved.reason}`);
    }
    return resolved;
  } catch (err) {
    console.error('[Electron] Core resolution failed, using bundled binary:', err.message);
    return { path: bundledPath, version: app.getVersion(), staged: false, reason: 'resolution failed' };
  }
}

/**
 * Get the path to the screenerbot binary shipped inside this application bundle
 */
function getBinaryPath() {
  const binaryName = process.platform === 'win32' ? 'screenerbot.exe' : 'screenerbot';
  
  if (app.isPackaged) {
    // In production, binary is in resources folder
    const resourcePath = path.join(process.resourcesPath, binaryName);
    console.log('[Electron] Production binary path:', resourcePath);
    return resourcePath;
  } else {
    // In development, check for debug override
    if (process.env.USE_DEBUG_BINARY === 'true') {
      const debugPath = path.join(__dirname, '..', '..', 'target', 'debug', binaryName);
      console.log('[Electron] Development binary path (DEBUG):', debugPath);
      return debugPath;
    }
    // In development, use the release build
    const devPath = path.join(__dirname, '..', '..', 'target', 'release', binaryName);
    console.log('[Electron] Development binary path:', devPath);
    return devPath;
  }
}

/**
 * Check if the backend webserver is ready
 */
function checkBackendHealth() {
  return new Promise((resolve) => {
    const options = {
      hostname: CONFIG.host,
      port: CONFIG.port,
      path: CONFIG.healthEndpoint,
      method: 'GET',
      timeout: 3000
    };

    const req = http.request(options, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        console.log('[Electron] Health check response:', res.statusCode);
        resolve(res.statusCode === 200);
      });
    });

    req.on('error', (err) => {
      // Only log occasionally to avoid spam
      resolve(false);
    });
    
    req.on('timeout', () => {
      req.destroy();
      resolve(false);
    });

    req.end();
  });
}

/**
 * Wait for the backend to be ready
 * First waits for SCREENERBOT_READY signal (to get dynamic port), then health checks
 */
async function waitForBackend() {
  const startTime = Date.now();
  
  console.log('[Electron] Waiting for backend to report ready (SCREENERBOT_READY signal)...');
  
  // Phase 1: Wait for SCREENERBOT_READY with port and token. The latch covers
  // fast restarts where stdout reports ready before this function installs its
  // resolver.
  let gotReadySignal = backendReadySignal;
  if (!gotReadySignal && startupError) return false;
  if (!gotReadySignal) {
    gotReadySignal = await new Promise((resolve) => {
      backendReadyResolve = resolve;

      setTimeout(() => {
        if (backendReadyResolve) {
          console.error('[Electron] Timeout waiting for SCREENERBOT_READY signal');
          resolve(false);
          backendReadyResolve = null;
        }
      }, CONFIG.maxWaitTime);
    });
  }
  
  if (!gotReadySignal || !CONFIG.port) {
    console.error('[Electron] Backend did not send SCREENERBOT_READY signal or port is missing');
    return false;
  }
  
  console.log(`[Electron] Got SCREENERBOT_READY signal, port=${CONFIG.port}, elapsed=${Date.now() - startTime}ms`);
  
  // Phase 2: Health check loop to verify backend is fully operational
  let checkCount = 0;
  const healthStartTime = Date.now();
  
  console.log('[Electron] Starting health checks...');
  
  while (Date.now() - startTime < CONFIG.maxWaitTime) {
    checkCount++;
    const isReady = await checkBackendHealth();
    
    if (isReady) {
      console.log(`[Electron] Backend is ready after ${checkCount} health checks (${Date.now() - healthStartTime}ms)`);
      return true;
    }
    
    // Log progress every 10 checks
    if (checkCount % 10 === 0) {
      console.log(`[Electron] Still waiting for health check... (${checkCount} checks, ${Math.round((Date.now() - healthStartTime) / 1000)}s)`);
    }
    
    await new Promise(resolve => setTimeout(resolve, CONFIG.pollInterval));
  }
  
  console.error('[Electron] Backend health check failed within timeout');
  return false;
}

/**
 * Start the screenerbot backend process
 */
function startBackend(extraArgs = [], core = null) {
  activeCore = core || { path: getBinaryPath(), version: app.getVersion(), staged: false, reason: '' };
  const binaryPath = activeCore.path;

  // Check if binary exists
  if (!fs.existsSync(binaryPath)) {
    console.error('[Electron] Binary not found at:', binaryPath);
    return null;
  }

  const args = ['--gui', ...extraArgs];
  console.log('[Electron] Starting backend:', binaryPath, args.join(' '));
  backendReadySignal = false;
  launchGeneration += 1;

  try {
    // Spawn the backend process
    backendProcess = spawn(binaryPath, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: false,
      env: {
        ...process.env,
        RUST_BACKTRACE: '1',
        // The backend cannot discover these on its own: the shell revision is
        // what makes a core-only update decidable, and the staged flag tells the
        // dashboard it is running an update that was applied without an installer.
        SCREENERBOT_SHELL_REVISION: SHELL_REVISION,
        SCREENERBOT_CORE_STAGED: activeCore.staged ? '1' : '0'
      }
    });

    backendProcess.stdout.on('error', (err) => { if (err.code === 'EPIPE') return; });
    backendProcess.stderr.on('error', (err) => { if (err.code === 'EPIPE') return; });

    backendProcess.stdout.on('data', (data) => {
      const lines = data.toString().trim().split('\n');
      lines.forEach(line => {
        if (line.trim()) {
          console.log('[Backend]', line);
          // Parse SCREENERBOT_ERROR signal — a structured fatal startup failure.
          // Symmetric with SCREENERBOT_READY: base64-encoded JSON on one line.
          if (line.startsWith('SCREENERBOT_ERROR:')) {
            try {
              const encoded = line.slice('SCREENERBOT_ERROR:'.length).trim();
              const payload = JSON.parse(Buffer.from(encoded, 'base64').toString('utf8'));
              console.error('[Electron] Backend reported fatal startup error:', payload.code);
              startupError = payload;
              // Unblock waitForBackend() so the boot sequence stops waiting.
              if (backendReadyResolve) {
                backendReadyResolve(false);
                backendReadyResolve = null;
              }
            } catch (err) {
              console.error('[Electron] Failed to parse SCREENERBOT_ERROR payload:', err.message);
            }
            return;
          }
          if (line.trim() === 'SCREENERBOT_RESTART') {
            backendRestartRequested = true;
            try {
              const currentRoute = currentDashboardRoute();
              backendRestartTarget = currentRoute && !currentRoute.startsWith('/initialization')
                ? currentRoute
                : '/home';
            } catch (_) {
              backendRestartTarget = '/home';
            }
            console.log(`[Electron] Backend requested graceful restart; restore=${backendRestartTarget}`);
            return;
          }
          // Parse SCREENERBOT_READY message for port and token
          if (line.startsWith('SCREENERBOT_READY:')) {
            const parts = line.split(':');
            if (parts.length >= 3) {
              const port = parseInt(parts[1], 10);
              const token = parts[2];
              console.log(`[Electron] Backend ready on port ${port} with token ${token.substring(0, 8)}...`);
              CONFIG.port = port;
              global.SCREENERBOT_TOKEN = token;
              backendReadySignal = true;
              
              // Resolve the waitForBackend() Promise
              if (backendReadyResolve) {
                backendReadyResolve(true);
                backendReadyResolve = null;
              }
            }
          }
        }
      });
    });

    backendProcess.stderr.on('data', (data) => {
      const lines = data.toString().trim().split('\n');
      lines.forEach(line => {
        if (line.trim()) {
          console.error('[Backend]', line);
        }
      });
    });

    backendProcess.on('error', (err) => {
      console.error('[Electron] Failed to start backend:', err.message);
      console.error('[Electron] Error code:', err.code);
    });

    backendProcess.on('exit', (code, signal) => {
      console.log(`[Electron] Backend exited with code ${code}, signal ${signal}`);
      backendProcess = null;

      if (isQuitting || !mainWindow) {
        return;
      }

      if (backendRestartRequested || code === BACKEND_RESTART_EXIT_CODE) {
        const target = backendRestartTarget;
        backendRestartRequested = false;
        setImmediate(() => restartBackendFromDashboard(target));
        return;
      }

      // A staged core that dies before reporting ready is not usable — the most
      // likely causes (a truncated file, a binary the OS refuses to execute) are
      // exactly the ones a digest cannot catch. Quarantine it so neither this
      // launch nor the updater picks it up again, and fall back to the bundle.
      if (activeCore.staged && !backendReadySignal && !isRecovering) {
        const failedVersion = activeCore.version;
        console.error(`[Electron] Staged core v${failedVersion} failed to start; rolling back`);
        // Release whoever is still awaiting this launch so it stops waiting out
        // its timeout; the generation check keeps it from reporting a failure.
        if (backendReadyResolve) {
          backendReadyResolve(false);
          backendReadyResolve = null;
        }
        setImmediate(async () => {
          await coreResolver.quarantineStagedCore(CORE_DIR, failedVersion);
          restartBackendFromDashboard(backendRestartTarget, `Update v${failedVersion} did not start — restoring the previous version...`);
        });
        return;
      }

      // Ignore exits while a separate recovery relaunch is in flight.
      if (isRecovering) return;

      // An error screen already on display is the terminal state — leave it.
      if (bootErrorShown) {
        return;
      }

      // A structured startup error is only rendered by the boot sequence when
      // waitForBackend() fails. The webserver reports SCREENERBOT_READY before
      // the services that can still fail, so a fatal error after that point let
      // the dashboard load and then left it in its skeleton with the backend
      // gone. Render the reported cause here instead of dropping it.
      if (startupError) {
        showBootError(startupError);
        return;
      }

      const phase = dashboardLoaded ? 'while the dashboard was running' : 'before the dashboard was ready';
      showBootError(genericBootError(`The backend stopped ${phase} (exit code ${code}).`));
    });

    console.log('[Electron] Backend process spawned with PID:', backendProcess.pid);
    return backendProcess;
    
  } catch (err) {
    console.error('[Electron] Exception starting backend:', err);
    return null;
  }
}

/**
 * Stop the backend process
 */
function stopBackend() {
  if (backendProcess) {
    console.log('[Electron] Stopping backend (PID:', backendProcess.pid, ')...');
    
    // Send SIGTERM for graceful shutdown
    backendProcess.kill('SIGTERM');
    
    // Force kill after 30 seconds if still running
    // (ServiceManager has 10s per-task timeout, needs time for graceful shutdown)
    setTimeout(() => {
      if (backendProcess) {
        console.log('[Electron] Force killing backend...');
        backendProcess.kill('SIGKILL');
      }
    }, 30000);
  }
}

/**
 * Send loading status to the renderer
 */
function updateLoadingStatus(status) {
  if (mainWindow && mainWindow.webContents) {
    mainWindow.webContents.send('loading:status', status);
  }
}

/**
 * Drive the splash screen's update panel.
 *
 * A silent update is applied by restarting the backend, which means the only
 * moment the owner sees it is the splash. Rather than a bare spinner, the splash
 * shows what is being installed and how far along it is.
 *
 * @param {{phase:'applying'|'verifying'|'done', version?:string, percent?:number}} payload
 */
function sendUpdateProgress(payload) {
  if (mainWindow && mainWindow.webContents && !mainWindow.webContents.isDestroyed()) {
    mainWindow.webContents.send('update:progress', payload);
  }
}

/**
 * Get the window state file path
 */
function getWindowStatePath() {
  const userDataPath = app.getPath('userData');
  return path.join(userDataPath, 'window-state.json');
}

/**
 * Load saved window state
 */
function loadWindowState() {
  try {
    const statePath = getWindowStatePath();
    if (fs.existsSync(statePath)) {
      const data = fs.readFileSync(statePath, 'utf8');
      return JSON.parse(data);
    }
  } catch (err) {
    console.error('[Electron] Failed to load window state:', err);
  }
  
  // Return defaults if no saved state
  return {
    width: CONFIG.windowWidth,
    height: CONFIG.windowHeight,
    x: undefined,
    y: undefined,
    isMaximized: false,
    zoomLevel: 0,
    theme: 'dark'
  };
}

/**
 * Save current window state
 */
function saveWindowState() {
  if (!mainWindow) return;
  
  try {
    const bounds = mainWindow.getBounds();
    const state = {
      width: bounds.width,
      height: bounds.height,
      x: bounds.x,
      y: bounds.y,
      isMaximized: mainWindow.isMaximized(),
      zoomLevel: mainWindow.webContents.getZoomLevel(),
      theme: currentTheme
    };
    
    const statePath = getWindowStatePath();
    fs.writeFileSync(statePath, JSON.stringify(state, null, 2));
  } catch (err) {
    console.error('[Electron] Failed to save window state:', err);
  }
}

function getZoomWebContents() {
  const focusedWindow = BrowserWindow.getFocusedWindow();
  if (focusedWindow && focusedWindow.webContents && !focusedWindow.webContents.isDestroyed()) {
    return focusedWindow.webContents;
  }
  if (mainWindow && mainWindow.webContents && !mainWindow.webContents.isDestroyed()) {
    return mainWindow.webContents;
  }
  return null;
}

function adjustZoomLevel(delta) {
  const webContents = getZoomWebContents();
  if (!webContents) return 0;
  const currentZoom = webContents.getZoomLevel();
  const nextZoom = Math.max(-5, Math.min(5, currentZoom + delta));
  webContents.setZoomLevel(nextZoom);
  saveWindowState();
  return webContents.getZoomLevel();
}

function resetZoomLevel() {
  const webContents = getZoomWebContents();
  if (!webContents) return 0;
  webContents.setZoomLevel(0);
  saveWindowState();
  return 0;
}

/**
 * Check and install Visual C++ Redistributable if missing (Windows Only)
 * Returns true if safe to proceed, false if we should stop/restart
 */
async function checkAndInstallVCRedist() {
  if (process.platform !== 'win32') return true;

  // 1. Quick Check: Look for the DLL in System32
  // This is a reliable heuristic for standard users
  const system32 = path.join(process.env.SystemRoot || 'C:\\Windows', 'System32');
  const dllPath = path.join(system32, 'vcruntime140.dll');
  
  if (fs.existsSync(dllPath)) {
    console.log('[Electron] VCRedist check: Found vcruntime140.dll');
    return true; // Exists, proceed
  }

  console.log('[Electron] VCRedist check: DLL missing, prompting user...');
  updateLoadingStatus('Checking dependencies...');

  // 2. Prompt User
  const { response } = await dialog.showMessageBox(mainWindow, {
    type: 'warning',
    title: 'Missing Dependency',
    message: 'Visual C++ Redistributable is missing',
    detail: 'ScreenerBot requires Microsoft Visual C++ Redistributable to run. Would you like to install it now?',
    buttons: ['Install & Fix', 'Exit'],
    defaultId: 0,
    cancelId: 1,
    icon: nativeImage.createFromPath(path.join(__dirname, '..', 'assets', 'icon.png'))
  });

  if (response !== 0) {
    app.quit();
    return false;
  }

  // 3. Locate Bundled Installer
  let redistPath;
  const isArm64 = process.arch === 'arm64';
  const redistName = isArm64 ? 'vc_redist.arm64.exe' : 'vc_redist.x64.exe';

  if (app.isPackaged) {
    redistPath = path.join(process.resourcesPath, redistName);
  } else {
    // Dev path
    redistPath = path.join(__dirname, '..', 'redist', redistName);
  }

  if (!fs.existsSync(redistPath)) {
    dialog.showErrorBox('Installer Not Found', `Could not locate ${redistName} correctly.`);
    const downloadUrl = isArm64 
      ? 'https://aka.ms/vs/17/release/vc_redist.arm64.exe'
      : 'https://aka.ms/vs/17/release/vc_redist.x64.exe';
    shell.openExternal(downloadUrl);
    return false;
  }

  // 4. Run Installer
  // /install /passive /norestart -> Installs with progress bar but no user interaction required
  updateLoadingStatus('Installing system dependencies...');
  
  try {
    await new Promise((resolve, reject) => {
      const installer = spawn(redistPath, ['/install', '/passive', '/norestart'], {
        detached: true,
        stdio: 'ignore'
      });
      
      installer.on('exit', (code) => {
        // Code 0 = Success, 3010 = Success (Restart Required)
        if (code === 0 || code === 3010) resolve();
        else reject(new Error(`Installer exited with code ${code}`));
      });
      
      installer.on('error', reject);
    });

    dialog.showMessageBox(mainWindow, {
      type: 'info',
      title: 'Installation Complete',
      message: 'Dependencies installed successfully.',
      detail: 'ScreenerBot will now start.',
      buttons: ['OK']
    });
    
    return true; // Proceed to start backend

  } catch (err) {
    console.error('[Electron] Redist installation failed:', err);
    dialog.showErrorBox('Installation Failed', 'Please install Visual C++ Redistributable manually.');
    shell.openExternal('https://aka.ms/vs/17/release/vc_redist.x64.exe');
    app.quit();
    return false;
  }
}

/**
 * Create the main window
 */
function createWindow() {
  // Load saved window state (incl. last-run theme)
  const windowState = loadWindowState();
  currentTheme = windowState.theme === 'light' ? 'light' : 'dark';

  mainWindow = new BrowserWindow({
    width: windowState.width,
    height: windowState.height,
    x: windowState.x,
    y: windowState.y,
    minWidth: CONFIG.minWidth,
    minHeight: CONFIG.minHeight,
    show: false,
    backgroundColor: themeBackgroundColor(currentTheme),
    icon: path.join(__dirname, '..', 'assets', 'icon.png'),
    autoHideMenuBar: process.platform === 'win32', // Hide menu bar on Windows only
    titleBarStyle: 'hiddenInset',
    trafficLightPosition: { x: 16, y: 16 },
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false // Disable sandbox to allow preload to work properly
    }
  });

  // Show window only after HTML content is fully loaded (prevents flash/double loading screen)
  mainWindow.webContents.once('did-finish-load', () => {
    mainWindow.show();
    // Restore maximized state if it was maximized before
    if (windowState.isMaximized) {
      mainWindow.maximize();
    }
    // Restore zoom level
    if (windowState.zoomLevel !== undefined && windowState.zoomLevel !== 0) {
      mainWindow.webContents.setZoomLevel(windowState.zoomLevel);
    }
  });
  
  // Save window state when it changes
  mainWindow.on('resize', () => {
    if (!mainWindow.isMaximized() && !mainWindow.isMinimized() && !mainWindow.isFullScreen()) {
      saveWindowState();
    }
  });
  
  mainWindow.on('move', () => {
    if (!mainWindow.isMaximized() && !mainWindow.isMinimized() && !mainWindow.isFullScreen()) {
      saveWindowState();
    }
  });
  
  mainWindow.on('maximize', () => {
    saveWindowState();
    if (mainWindow && !mainWindow.webContents.isDestroyed()) {
      mainWindow.webContents.send('window:maximize-change', true);
    }
  });
  
  mainWindow.on('unmaximize', () => {
    saveWindowState();
    if (mainWindow && !mainWindow.webContents.isDestroyed()) {
      mainWindow.webContents.send('window:maximize-change', false);
    }
  });

  // Notify renderer of fullscreen changes
  mainWindow.on('enter-full-screen', () => {
    if (mainWindow && !mainWindow.webContents.isDestroyed()) {
      mainWindow.webContents.send('window:fullscreen-change', true);
    }
  });
  
  mainWindow.on('leave-full-screen', () => {
    if (mainWindow && !mainWindow.webContents.isDestroyed()) {
      mainWindow.webContents.send('window:fullscreen-change', false);
    }
  });

  // Handle external links
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    openExternalUrl(url);
    return { action: 'deny' };
  });

  mainWindow.webContents.on('will-navigate', (event, url) => {
    if (isDashboardUrl(url) || isLoadingPageUrl(url)) return;
    event.preventDefault();
    openExternalUrl(url);
  });

  mainWindow.webContents.on('did-finish-load', () => {
    dashboardLoaded = isDashboardUrl(mainWindow.webContents.getURL());

    // Tell the capture runtime where its scene media is served from. Re-applied
    // on every dashboard load so a reload mid-scene keeps working.
    if (dashboardLoaded && promoBridgePort) {
      mainWindow.webContents
        .executeJavaScript(`window.__SB_PROMO_MEDIA_BASE__ = "http://127.0.0.1:${promoBridgePort}/media";`)
        .catch(err => console.error('[Electron] Failed to publish promo media base:', err.message));
    }
  });

  mainWindow.webContents.on('did-fail-load', (_event, errorCode, errorDescription, validatedURL, isMainFrame) => {
    if (!isMainFrame || errorCode === -3 || !isDashboardUrl(validatedURL) || isQuitting) return;
    dashboardLoaded = false;
    showBootError(genericBootError(`The dashboard failed to load (${errorDescription}, ${errorCode}).`));
  });

  mainWindow.webContents.on('render-process-gone', (_event, details) => {
    if (isQuitting) return;
    dashboardLoaded = false;
    showBootError(genericBootError(`The dashboard renderer stopped (${details.reason}).`));
  });

  mainWindow.on('unresponsive', () => {
    if (isQuitting) return;
    dashboardLoaded = false;
    showBootError(genericBootError('The dashboard became unresponsive.'));
  });

  // Ensure zoom shortcuts work consistently across keyboard layouts.
  mainWindow.webContents.on('before-input-event', (event, input) => {
    const isZoomModifier = input.control || input.meta;
    if (!isZoomModifier || input.alt) return;

    if (input.key === '+' || input.key === '=' || input.code === 'NumpadAdd') {
      event.preventDefault();
      adjustZoomLevel(0.5);
      return;
    }

    if (input.key === '-' || input.code === 'NumpadSubtract') {
      event.preventDefault();
      adjustZoomLevel(-0.5);
      return;
    }

    if (input.key === '0' || input.code === 'Numpad0') {
      event.preventDefault();
      resetZoomLevel();
    }
  });

  // Handle window close event
  mainWindow.on('close', async (event) => {
    // If we're already quitting, let it close
    if (isQuitting) {
      return;
    }
    
    // macOS: Hide window instead of closing (dock behavior)
    if (process.platform === 'darwin') {
      event.preventDefault();
      mainWindow.hide();
      return;
    }
    
    // Windows/Linux: Show exit confirmation dialog
    event.preventDefault();
    
    const choice = await showExitDialog();
    
    if (choice === 'minimize') {
      // Minimize to system tray
      mainWindow.hide();
      console.log('[Electron] Minimized to system tray');
    } else if (choice === 'quit') {
      // Quit the application
      isQuitting = true;
      app.quit();
    }
    // 'cancel' - do nothing, window stays open
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  return mainWindow;
}

/**
 * Create the application menu with keyboard shortcuts
 */
function createApplicationMenu() {
  const isMac = process.platform === 'darwin';
  
  const template = [
    // App menu (macOS only)
    ...(isMac ? [{
      label: app.name,
      submenu: [
        {
          label: 'About ScreenerBot',
          click: () => {
            dialog.showMessageBox(mainWindow, {
              type: 'info',
              title: 'About ScreenerBot',
              message: 'ScreenerBot',
              detail: `Version ${app.getVersion()}\n\nAdvanced Solana wallet management and auto-trading bot.\n\nhttps://screenerbot.io\n\n© 2024-2026 ScreenerBot`,
              buttons: ['OK']
            });
          }
        },
        { type: 'separator' },
        {
          label: 'Check for Updates...',
          click: () => {
            mainWindow?.webContents.send('updates:check');
          }
        },
        { type: 'separator' },
        { role: 'services' },
        { type: 'separator' },
        { role: 'hide' },
        { role: 'hideOthers' },
        { role: 'unhide' },
        { type: 'separator' },
        { role: 'quit' }
      ]
    }] : []),
    
    // File menu
    {
      label: 'File',
      submenu: [
        {
          label: 'Open Data Folder',
          accelerator: isMac ? 'Cmd+Shift+D' : 'Ctrl+Shift+D',
          click: () => {
            shell.openPath(appPaths.dataDirectory(SCREENERBOT_BASE_DIR));
          }
        },
        {
          label: 'Open Logs Folder',
          click: openScreenerBotLogs
        },
        { type: 'separator' },
        isMac ? { role: 'close' } : { role: 'quit' }
      ]
    },
    
    // Edit menu
    {
      label: 'Edit',
      submenu: [
        { role: 'undo' },
        { role: 'redo' },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        ...(isMac ? [
          { role: 'pasteAndMatchStyle' },
          { role: 'delete' },
          { role: 'selectAll' },
        ] : [
          { role: 'delete' },
          { type: 'separator' },
          { role: 'selectAll' }
        ])
      ]
    },
    
    // View menu - CRITICAL FOR ZOOM SHORTCUTS
    {
      label: 'View',
      submenu: [
        { role: 'reload' },
        { role: 'forceReload' },
        { type: 'separator' },
        { label: 'Reset Zoom', accelerator: 'CmdOrCtrl+0', click: () => resetZoomLevel() },
        { label: 'Zoom In', accelerator: 'CmdOrCtrl+=', click: () => adjustZoomLevel(0.5) },
        { label: 'Zoom Out', accelerator: 'CmdOrCtrl+-', click: () => adjustZoomLevel(-0.5) },
        { type: 'separator' },
        { role: 'togglefullscreen' },
        { type: 'separator' },
        { role: 'toggleDevTools' }
      ]
    },
    
    // Window menu
    {
      label: 'Window',
      submenu: [
        { role: 'minimize' },
        { role: 'zoom' },
        ...(isMac ? [
          { type: 'separator' },
          { role: 'front' },
          { type: 'separator' },
          { role: 'window' }
        ] : [
          { role: 'close' }
        ])
      ]
    },
    
    // Help menu
    {
      label: 'Help',
      submenu: [
        {
          label: 'Documentation',
          accelerator: 'F1',
          click: async () => {
            await shell.openExternal('https://screenerbot.io/docs');
          }
        },
        {
          label: 'Keyboard Shortcuts',
          click: () => {
            const shortcuts = isMac ? `
Keyboard Shortcuts:

Window Controls:
  Cmd+M          Minimize
  Cmd+W          Close Window
  Cmd+Q          Quit
  Cmd+Ctrl+F     Toggle Fullscreen

Zoom:
  Cmd++          Zoom In
  Cmd+-          Zoom Out
  Cmd+0          Reset Zoom

Navigation:
  Cmd+R          Reload Dashboard
  Cmd+Shift+D    Open Data Folder

Other:
  F1             Open Documentation
  Cmd+Alt+I      Toggle DevTools
` : `
Keyboard Shortcuts:

Window Controls:
  Alt+F4         Quit
  F11            Toggle Fullscreen

Zoom:
  Ctrl++         Zoom In
  Ctrl+-         Zoom Out
  Ctrl+0         Reset Zoom

Navigation:
  Ctrl+R         Reload Dashboard
  Ctrl+Shift+D   Open Data Folder

Other:
  F1             Open Documentation
  Ctrl+Shift+I   Toggle DevTools
`;
            dialog.showMessageBox(mainWindow, {
              type: 'info',
              title: 'Keyboard Shortcuts',
              message: 'ScreenerBot Keyboard Shortcuts',
              detail: shortcuts.trim(),
              buttons: ['OK']
            });
          }
        },
        { type: 'separator' },
        {
          label: 'Telegram Channel',
          click: async () => {
            await shell.openExternal('https://t.me/screenerbotio');
          }
        },
        {
          label: 'Telegram Community',
          click: async () => {
            await shell.openExternal('https://t.me/screenerbotio_talk');
          }
        },
        {
          label: 'Telegram Support',
          click: async () => {
            await shell.openExternal('https://t.me/screenerbotio_support');
          }
        },
        { type: 'separator' },
        {
          label: 'Follow on X (Twitter)',
          click: async () => {
            await shell.openExternal('https://x.com/screenerbotio');
          }
        },
        {
          label: 'Visit Website',
          click: async () => {
            await shell.openExternal('https://screenerbot.io');
          }
        },
        { type: 'separator' },
        {
          label: 'Check for Updates...',
          click: () => {
            mainWindow?.webContents.send('updates:check');
          }
        },
        ...(!isMac ? [
          { type: 'separator' },
          {
            label: 'About ScreenerBot',
            click: () => {
              dialog.showMessageBox(mainWindow, {
                type: 'info',
                title: 'About ScreenerBot',
                message: 'ScreenerBot',
                detail: `Version ${app.getVersion()}\n\nAdvanced Solana wallet management and auto-trading bot.\n\nhttps://screenerbot.io\n\n© 2024-2026 ScreenerBot`,
                buttons: ['OK']
              });
            }
          }
        ] : [])
      ]
    }
  ];
  
  const menu = Menu.buildFromTemplate(template);
  Menu.setApplicationMenu(menu);
}

/**
 * Load the main application URL
 */
function loadMainApp(route = '/') {
  // Add ?electron=1 to tell the dashboard to skip its splash screen, and pass the
  // last-run theme so the dashboard's pre-paint FOUC script renders the right
  // theme immediately. GUI mode uses a NEW dynamic port each launch, so the
  // dashboard's localStorage (origin-scoped, includes port) is empty on every
  // start — without this it falls back to dark and then flips to light once
  // theme.js fetches the server value (the visible dark→light flash).
  const safeRoute = typeof route === 'string' && route.startsWith('/') && !route.startsWith('//')
    ? route
    : '/';
  const appUrl = new URL(safeRoute, `http://${CONFIG.host}:${CONFIG.port}`);
  appUrl.searchParams.set('electron', '1');
  appUrl.searchParams.set('theme', currentTheme);
  console.log('[Electron] Loading main app:', appUrl.href);
  dashboardLoaded = false;
  mainWindow.loadURL(appUrl.href).catch(err => {
    if (!isQuitting) {
      showBootError(genericBootError(`The dashboard URL could not be loaded (${err.message}).`));
    }
  });
}

/**
 * Relaunch after a dashboard-requested graceful restart. The old backend has
 * already stopped its services and released its process lock before this runs.
 */
async function restartBackendFromDashboard(targetRoute, statusLabel) {
  if (isRecovering || isQuitting || !mainWindow) return;
  isRecovering = true;
  startupError = null;
  bootErrorShown = false;
  dashboardLoaded = false;

  loadLoadingPage();
  if (mainWindow.webContents.isLoading()) {
    await new Promise(resolve => mainWindow.webContents.once('did-finish-load', resolve));
  }

  CONFIG.port = null;
  global.SCREENERBOT_TOKEN = null;
  backendReadyResolve = null;

  // Resolving the core here is what actually applies a staged update: the
  // backend that comes back is the new version.
  const core = await resolveBackendBinary();
  if (statusLabel) {
    updateLoadingStatus(statusLabel);
  } else if (core.staged && core.version !== app.getVersion()) {
    sendUpdateProgress({ phase: 'applying', version: core.version });
    updateLoadingStatus(`Installing update v${core.version}...`);
  } else {
    updateLoadingStatus('Restarting ScreenerBot...');
  }

  const backend = startBackend(getBackendExtraArgs(), core);
  const generation = launchGeneration;
  isRecovering = false;
  if (!backend) {
    showBootError(genericBootError('Could not relaunch the backend after setup.'));
    return;
  }

  updateLoadingStatus('Waiting for backend...');
  const isReady = await waitForBackend();
  if (generation !== launchGeneration) return;
  if (isReady) {
    if (core.staged) sendUpdateProgress({ phase: 'done', version: core.version });
    updateLoadingStatus('Loading dashboard...');
    await new Promise(resolve => setTimeout(resolve, 500));
    loadMainApp(targetRoute || '/home');
  } else if (startupError) {
    showBootError(startupError);
  } else {
    showBootError(genericBootError('The backend did not come back online after restart.'));
  }
}

/**
 * Build a generic boot-error payload for unexplained early failures, mirroring
 * the structured SCREENERBOT_ERROR shape so the renderer can treat both alike.
 */
function genericBootError(detail) {
  const logsPath = path.join(appPaths.logsDirectory(SCREENERBOT_BASE_DIR), 'latest.log');
  return {
    code: 'generic',
    title: 'ScreenerBot could not start',
    detail: detail || 'The backend stopped unexpectedly before the dashboard was ready.',
    remedy: 'Open the logs folder to see what happened, then restart the app. If the problem '
      + 'persists, contact support at t.me/screenerbotio_support.',
    log_path: logsPath
  };
}

/**
 * Render the dedicated boot-error screen by reloading the splash page (if the
 * dashboard URL had already been attempted) and pushing the error payload to it.
 */
function showBootError(payload) {
  if (!mainWindow) return;
  startupError = payload;
  bootErrorShown = true;

  const send = () => {
    if (mainWindow && mainWindow.webContents) {
      mainWindow.webContents.send('boot:error', payload);
    }
  };

  // The splash page (index.html) hosts the error screen. If we've navigated away
  // to the dashboard URL, reload it first; otherwise push the payload directly.
  const currentUrl = mainWindow.webContents.getURL();
  if (!currentUrl.endsWith('index.html') && !currentUrl.startsWith('file://')) {
    mainWindow.webContents.once('did-finish-load', send);
    loadLoadingPage();
  } else {
    send();
  }
}

/**
 * Load the loading page
 */
function loadLoadingPage() {
  // Pass the last-run theme so the splash (boot.js) renders light/dark to match.
  mainWindow.loadFile(path.join(__dirname, 'index.html'), { query: { theme: currentTheme } });
}

/**
 * Initialize the application
 */
async function initialize() {
  console.log('[Electron] Initializing application...');
  console.log('[Electron] Packaged:', app.isPackaged);
  console.log('[Electron] Process arch:', process.arch);
  
  // Create system tray (Windows/Linux only)
  createTray();
  
  // Create window first
  createWindow();

  // Bring the capture bridge up before the backend: the driver polls it for
  // readiness, so it must be able to connect while the app is still booting.
  if (promoBridge) {
    try {
      promoBridgePort = await promoBridge.startPromoBridge(mainWindow, {
        port: Number(process.env.SCREENERBOT_PROMO_PORT || 0),
        mediaDir: process.env.SCREENERBOT_PROMO_MEDIA || null
      });
      console.log('[Electron] Promo Studio bridge listening on port', promoBridgePort);
    } catch (err) {
      console.error('[Electron] Promo Studio bridge failed to start:', err.message);
    }
  }

  // Create application menu with keyboard shortcuts
  createApplicationMenu();
  
  // Load loading page
  loadLoadingPage();
  
  // Wait for loading page to be ready so we can show updates
  if (mainWindow.webContents.isLoading()) {
    await new Promise(resolve => mainWindow.webContents.once('did-finish-load', resolve));
  }
  
  updateLoadingStatus('Initializing...');

  // Check dependencies (Windows)
  const dependenciesOk = await checkAndInstallVCRedist();
  if (!dependenciesOk) return;

  // Resolving the core is where a silently staged update is adopted: an update
  // that finished downloading in a previous session is simply what launches now.
  const core = await resolveBackendBinary();
  if (core.staged && core.version !== app.getVersion()) {
    sendUpdateProgress({ phase: 'applying', version: core.version });
    updateLoadingStatus(`Installing update v${core.version}...`);
  } else {
    updateLoadingStatus('Starting backend services...');
  }
  const backend = startBackend(getBackendExtraArgs(), core);
  const generation = launchGeneration;

  if (!backend) {
    console.error('[Electron] Failed to start backend process');
    showBootError(genericBootError(
      'The backend program could not be started. It may be missing or blocked by security software.'
    ));
    return;
  }

  // Wait for backend to be ready
  updateLoadingStatus('Waiting for backend...');
  const isReady = await waitForBackend();
  // A staged-core rollback (or any relaunch) supersedes this boot sequence.
  if (generation !== launchGeneration) return;

  if (isReady) {
    if (core.staged) sendUpdateProgress({ phase: 'done', version: core.version });
    updateLoadingStatus('Loading dashboard...');
    // Small delay to ensure everything is ready
    await new Promise(resolve => setTimeout(resolve, 500));
    // Load the main application
    loadMainApp();
  } else if (startupError) {
    // Backend reported a structured fatal error — show the dedicated screen.
    showBootError(startupError);
  } else {
    // No structured error but never became ready (e.g. health-check timeout).
    showBootError(genericBootError(
      'The backend did not finish starting in time. This can happen on a slow first run or if '
      + 'another program is blocking the connection.'
    ));
  }
}

// ============================================================================
// App Lifecycle Events
// ============================================================================

app.whenReady().then(initialize);

// macOS: Re-create window when dock icon is clicked
app.on('activate', () => {
  if (mainWindow === null) {
    initialize();
  } else {
    mainWindow.show();
  }
});

// Handle second instance attempt - focus existing window
app.on('second-instance', () => {
  if (mainWindow) {
    if (mainWindow.isMinimized()) mainWindow.restore();
    mainWindow.focus();
  }
});

// Handle quit
app.on('before-quit', () => {
  console.log('[Electron] Before quit - setting isQuitting flag');
  isQuitting = true;
});

app.on('will-quit', (event) => {
  // Clean up tray
  if (tray) {
    tray.destroy();
    tray = null;
  }

  if (promoBridge) {
    promoBridge.stopPromoBridge();
  }

  if (backendProcess) {
    console.log('[Electron] Will quit - stopping backend');
    event.preventDefault();
    
    // Listen for backend exit
    backendProcess.once('exit', () => {
      console.log('[Electron] Backend stopped, exiting app');
      app.exit(0);
    });
    
    stopBackend();
    
    // Fallback: force exit after 35 seconds (after SIGKILL timeout)
    setTimeout(() => {
      console.log('[Electron] Forcing app exit after timeout');
      app.exit(0);
    }, 35000);
  }
});

// Windows/Linux: Don't quit when all windows are closed if minimized to tray
app.on('window-all-closed', () => {
  // On macOS, apps typically stay active until explicitly quit
  // On Windows/Linux, we keep running if minimized to tray
  if (process.platform === 'darwin') {
    // macOS: Don't quit (dock behavior)
    return;
  }
  
  // Windows/Linux: Only quit if isQuitting flag is set
  // Otherwise the app is minimized to tray
  if (isQuitting) {
    app.quit();
  }
});

// ============================================================================
// IPC Handlers
// ============================================================================

// --- Boot-error screen actions -------------------------------------------------

/**
 * Perform a safe one-click recovery for a recoverable startup error by relaunching
 * the backend with the matching CLI flag. The Rust binary backs up the affected
 * data before clearing it, then continues into a normal boot.
 */
async function recoverAndRestart(extraArgs, statusLabel) {
  if (isRecovering) return;
  isRecovering = true;
  startupError = null;
  bootErrorShown = false;
  dashboardLoaded = false;

  // Return to the splash page and show progress.
  loadLoadingPage();
  if (mainWindow.webContents.isLoading()) {
    await new Promise(resolve => mainWindow.webContents.once('did-finish-load', resolve));
  }
  updateLoadingStatus(statusLabel || 'Recovering...');

  // Ensure any prior backend is gone before relaunching.
  if (backendProcess) {
    try { backendProcess.kill(); } catch (_) { /* already gone */ }
    backendProcess = null;
  }

  const backend = startBackend(extraArgs, await resolveBackendBinary());
  const generation = launchGeneration;
  isRecovering = false;
  if (!backend) {
    showBootError(genericBootError('Could not relaunch the backend for recovery.'));
    return;
  }

  updateLoadingStatus('Waiting for backend...');
  const isReady = await waitForBackend();
  if (generation !== launchGeneration) return;
  if (isReady) {
    updateLoadingStatus('Loading dashboard...');
    await new Promise(resolve => setTimeout(resolve, 500));
    loadMainApp();
  } else if (startupError) {
    showBootError(startupError);
  } else {
    showBootError(genericBootError('Recovery finished but the backend did not become ready.'));
  }
}

ipcMain.handle('boot:reset-wallet-data', async () => {
  console.log('[Electron] Boot recovery requested: reset wallet data');
  await recoverAndRestart(['--clean-wallet-data'], 'Backing up and resetting wallet data...');
  return true;
});

ipcMain.handle('boot:open-logs', () => {
  openScreenerBotLogs();
  return true;
});

ipcMain.handle('boot:quit', () => {
  isQuitting = true;
  app.quit();
  return true;
});

// Persist the UI theme chosen in the dashboard so the NEXT launch's splash and
// window background match it (the dashboard renderer calls this via theme.js on
// every theme set/toggle). Stored in window-state.json alongside bounds/zoom.
ipcMain.handle('theme:set', (event, theme) => {
  currentTheme = theme === 'light' ? 'light' : 'dark';
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.setBackgroundColor(themeBackgroundColor(currentTheme));
  }
  saveWindowState();
  return currentTheme;
});

ipcMain.handle('app:minimize', () => {
  if (mainWindow) {
    mainWindow.minimize();
    return true;
  }
  return false;
});

ipcMain.handle('app:maximize', () => {
  if (mainWindow) {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize();
      return false;
    } else {
      mainWindow.maximize();
      return true;
    }
  }
  return false;
});

ipcMain.handle('app:close', () => {
  if (mainWindow) {
    mainWindow.close();
    return true;
  }
  return false;
});

ipcMain.handle('app:zoom-in', () => {
  return adjustZoomLevel(0.5);
});

ipcMain.handle('app:zoom-out', () => {
  return adjustZoomLevel(-0.5);
});

ipcMain.handle('app:zoom-reset', () => {
  return resetZoomLevel();
});

ipcMain.handle('app:get-zoom-level', () => {
  const webContents = getZoomWebContents();
  return webContents ? webContents.getZoomLevel() : 0;
});

ipcMain.handle('app:get-version', () => {
  return app.getVersion();
});

// What the dashboard needs to describe this installation: the shell it is
// running inside, and which core actually launched.
ipcMain.handle('app:get-shell-info', () => ({
  shellVersion: app.getVersion(),
  shellRevision: SHELL_REVISION,
  coreVersion: activeCore.version,
  coreStaged: activeCore.staged
}));

ipcMain.handle('app:quit-for-update', () => {
  console.log('[Electron] Verified installer opened; quitting cleanly for update');
  isQuitting = true;
  app.quit();
  return true;
});

ipcMain.handle('app:toggle-fullscreen', () => {
  if (mainWindow) {
    mainWindow.setFullScreen(!mainWindow.isFullScreen());
    return mainWindow.isFullScreen();
  }
  return false;
});

ipcMain.handle('app:is-fullscreen', () => {
  return mainWindow ? mainWindow.isFullScreen() : false;
});

ipcMain.handle('app:is-maximized', () => {
  return mainWindow ? mainWindow.isMaximized() : false;
});
