const { app, BrowserWindow, ipcMain, shell } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const http = require('http');
const fs = require('fs');

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
  if (app.isPackaged) {
    // In production, binary is in resources folder
    const resourcePath = path.join(process.resourcesPath, 'screenerbot');
    console.log('[Electron] Production binary path:', resourcePath);
    return resourcePath;
  } else {
    // In development, use the release build
    const devPath = path.join(__dirname, '..', '..', 'target', 'release', 'screenerbot');
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
        mainWindow.webContents.executeJavaScript(`
          document.getElementById('status').textContent = 'Backend process exited unexpectedly (code: ${code})';
          document.getElementById('spinner').style.display = 'none';
        `).catch(() => {});
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
 * Create the main window
 */
function createWindow() {
  mainWindow = new BrowserWindow({
    width: CONFIG.windowWidth,
    height: CONFIG.windowHeight,
    minWidth: CONFIG.minWidth,
    minHeight: CONFIG.minHeight,
    show: false,
    backgroundColor: '#0d1117',
    titleBarStyle: 'hiddenInset',
    trafficLightPosition: { x: 16, y: 16 },
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: false // Disable sandbox to allow preload to work properly
    }
  });

  // Show window when ready
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
    mainWindow.maximize();
  });

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
  const appUrl = `http://${CONFIG.host}:${CONFIG.port}`;
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

  // Start the backend
  const backend = startBackend();
  
  if (!backend) {
    console.error('[Electron] Failed to start backend process');
    mainWindow.webContents.executeJavaScript(`
      document.getElementById('status').textContent = 'Failed to start backend process';
      document.getElementById('spinner').style.display = 'none';
    `).catch(() => {});
    return;
  }

  // Wait for backend to be ready
  const isReady = await waitForBackend();

  if (isReady) {
    // Small delay to ensure everything is ready
    await new Promise(resolve => setTimeout(resolve, 500));
    // Load the main application
    loadMainApp();
  } else {
    // Show error in the loading page
    mainWindow.webContents.executeJavaScript(`
      document.getElementById('status').textContent = 'Backend failed to start. Please check logs.';
      document.getElementById('spinner').style.display = 'none';
    `).catch(() => {});
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

// Prevent multiple instances
const gotTheLock = app.requestSingleInstanceLock();
if (!gotTheLock) {
  console.log('[Electron] Another instance is running, quitting...');
  app.quit();
} else {
  app.on('second-instance', () => {
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore();
      mainWindow.focus();
    }
  });
}

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
