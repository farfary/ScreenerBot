const { app, BrowserWindow, ipcMain, shell } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const http = require('http');
const fs = require('fs');
const os = require('os');

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
  port: 8080,
  host: '127.0.0.1',
  healthEndpoint: '/api/health',
  pollInterval: 1000,
  maxWaitTime: 120000, // 2 minutes max wait
  windowWidth: 1400,
  windowHeight: 900,
  minWidth: 1200,
  minHeight: 700
};

let mainWindow = null;
let backendProcess = null;
let isQuitting = false;

/**
 * Get the path to the screenerbot binary
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
 */
async function waitForBackend() {
  const startTime = Date.now();
  let checkCount = 0;
  
  console.log('[Electron] Waiting for backend to be ready...');
  
  while (Date.now() - startTime < CONFIG.maxWaitTime) {
    checkCount++;
    const isReady = await checkBackendHealth();
    
    if (isReady) {
      console.log(`[Electron] Backend is ready after ${checkCount} checks (${Date.now() - startTime}ms)`);
      return true;
    }
    
    // Log progress every 10 checks
    if (checkCount % 10 === 0) {
      console.log(`[Electron] Still waiting... (${checkCount} checks, ${Math.round((Date.now() - startTime) / 1000)}s)`);
    }
    
    await new Promise(resolve => setTimeout(resolve, CONFIG.pollInterval));
  }
  
  console.error('[Electron] Backend failed to start within timeout');
  return false;
}

/**
 * Start the screenerbot backend process
 */
function startBackend() {
  const binaryPath = getBinaryPath();
  
  // Check if binary exists
  if (!fs.existsSync(binaryPath)) {
    console.error('[Electron] Binary not found at:', binaryPath);
    return null;
  }
  
  console.log('[Electron] Starting backend:', binaryPath);

  try {
    // Spawn the backend process
    backendProcess = spawn(binaryPath, [], {
      stdio: ['ignore', 'pipe', 'pipe'],
      detached: false,
      env: {
        ...process.env,
        RUST_BACKTRACE: '1'
      }
    });

    backendProcess.stdout.on('data', (data) => {
      const lines = data.toString().trim().split('\n');
      lines.forEach(line => {
        if (line.trim()) {
          console.log('[Backend]', line);
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
      
      // If we're not quitting, the backend crashed - show error
      if (!isQuitting && mainWindow) {
        updateLoadingStatus(`Backend process exited unexpectedly (code: ${code})`);
      }
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
    
    // Force kill after 5 seconds if still running
    setTimeout(() => {
      if (backendProcess) {
        console.log('[Electron] Force killing backend...');
        backendProcess.kill('SIGKILL');
      }
    }, 5000);
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
    isMaximized: false
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
      isMaximized: mainWindow.isMaximized()
    };
    
    const statePath = getWindowStatePath();
    fs.writeFileSync(statePath, JSON.stringify(state, null, 2));
  } catch (err) {
    console.error('[Electron] Failed to save window state:', err);
  }
}

/**
 * Create the main window
 */
function createWindow() {
  // Load saved window state
  const windowState = loadWindowState();
  
  mainWindow = new BrowserWindow({
    width: windowState.width,
    height: windowState.height,
    x: windowState.x,
    y: windowState.y,
    minWidth: CONFIG.minWidth,
    minHeight: CONFIG.minHeight,
    show: false,
    backgroundColor: '#0d1117',
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
  
  mainWindow.on('maximize', saveWindowState);
  mainWindow.on('unmaximize', saveWindowState);

  // Handle external links
  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    shell.openExternal(url);
    return { action: 'deny' };
  });

  // macOS: Hide window instead of closing (dock behavior)
  mainWindow.on('close', (event) => {
    if (process.platform === 'darwin' && !isQuitting) {
      event.preventDefault();
      mainWindow.hide();
    }
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  return mainWindow;
}

/**
 * Load the main application URL
 */
function loadMainApp() {
  // Add ?electron=1 to tell the dashboard to skip its splash screen
  const appUrl = `http://${CONFIG.host}:${CONFIG.port}?electron=1`;
  console.log('[Electron] Loading main app:', appUrl);
  mainWindow.loadURL(appUrl);
}

/**
 * Load the loading page
 */
function loadLoadingPage() {
  mainWindow.loadFile(path.join(__dirname, 'index.html'));
}

/**
 * Initialize the application
 */
async function initialize() {
  console.log('[Electron] Initializing application...');
  console.log('[Electron] Packaged:', app.isPackaged);
  console.log('[Electron] Process arch:', process.arch);
  
  // Create window first
  createWindow();
  
  // Load loading page
  loadLoadingPage();
  
  // Send initial status after page loads
  mainWindow.webContents.once('did-finish-load', () => {
    updateLoadingStatus('Initializing...');
  });

  // Start the backend
  updateLoadingStatus('Starting backend services...');
  const backend = startBackend();
  
  if (!backend) {
    console.error('[Electron] Failed to start backend process');
    updateLoadingStatus('Failed to start backend process');
    return;
  }

  // Wait for backend to be ready
  updateLoadingStatus('Waiting for backend...');
  const isReady = await waitForBackend();

  if (isReady) {
    updateLoadingStatus('Loading dashboard...');
    // Small delay to ensure everything is ready
    await new Promise(resolve => setTimeout(resolve, 500));
    // Load the main application
    loadMainApp();
  } else {
    // Show error in the loading page
    updateLoadingStatus('Backend failed to start. Please check logs.');
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
  if (backendProcess) {
    console.log('[Electron] Will quit - stopping backend');
    event.preventDefault();
    stopBackend();
    
    // Wait a bit then quit
    setTimeout(() => {
      app.exit(0);
    }, 1000);
  }
});

// macOS: Don't quit when all windows are closed
app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

// ============================================================================
// IPC Handlers
// ============================================================================

ipcMain.handle('app:minimize', () => {
  if (mainWindow) mainWindow.minimize();
});

ipcMain.handle('app:maximize', () => {
  if (mainWindow) {
    if (mainWindow.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow.maximize();
    }
  }
});

ipcMain.handle('app:close', () => {
  if (mainWindow) mainWindow.close();
});

ipcMain.handle('app:zoom-in', () => {
  if (mainWindow) {
    const currentZoom = mainWindow.webContents.getZoomLevel();
    mainWindow.webContents.setZoomLevel(currentZoom + 0.5);
  }
});

ipcMain.handle('app:zoom-out', () => {
  if (mainWindow) {
    const currentZoom = mainWindow.webContents.getZoomLevel();
    mainWindow.webContents.setZoomLevel(currentZoom - 0.5);
  }
});

ipcMain.handle('app:zoom-reset', () => {
  if (mainWindow) {
    mainWindow.webContents.setZoomLevel(0);
  }
});

ipcMain.handle('app:get-version', () => {
  return app.getVersion();
});
