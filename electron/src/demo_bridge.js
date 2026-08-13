/**
 * Demo capture bridge.
 *
 * Present only when the app is launched with SCREENERBOT_DEMO_CONTROL set — the
 * demo studio driver does that; a normal launch never loads any of this.
 *
 * It exposes one local HTTP endpoint the driver posts commands to, and routes
 * each command to whoever can actually observe its completion:
 *
 *   - window shape and identity                 -> this process
 *   - navigation, interaction, overlays, audio  -> the dashboard's demo runtime
 *
 * It deliberately does NOT capture anything. Screenshots and video are taken by
 * macOS itself (`screencapture`, targeted at this window's CoreGraphics id), so
 * the captures are true native window frames rather than a copy of the web
 * contents. All this side has to do is say which window that is.
 *
 * Nothing here resolves on a timer: a window resize resolves on the window's own
 * resize event, and a renderer command resolves when the runtime reports back.
 */

const http = require('http');
const fsp = require('fs/promises');
const path = require('path');
const { app, ipcMain } = require('electron');

const HOST = '127.0.0.1';
const RENDERER_TIMEOUT_MS = 60000;

let server = null;
let mainWindow = null;
let mediaRoot = null;

const pending = new Map();
let sequence = 0;

// ---------------------------------------------------------------------------
// Renderer channel
// ---------------------------------------------------------------------------

/** Send a command to the dashboard runtime and await the result it reports. */
function callRenderer(name, args) {
  return new Promise((resolve, reject) => {
    if (!mainWindow || mainWindow.webContents.isDestroyed()) {
      reject(new Error('No dashboard window'));
      return;
    }

    sequence += 1;
    const id = sequence;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`Renderer command "${name}" did not answer within ${RENDERER_TIMEOUT_MS}ms`));
    }, RENDERER_TIMEOUT_MS);

    pending.set(id, { resolve, reject, timer });
    mainWindow.webContents.send('demo:command', { id, name, args });
  });
}

ipcMain.on('demo:result', (_event, { id, ok, result, error }) => {
  const entry = pending.get(id);
  if (!entry) return;
  clearTimeout(entry.timer);
  pending.delete(id);
  if (ok) entry.resolve(result);
  else entry.reject(new Error(error || 'Renderer command failed'));
});

// ---------------------------------------------------------------------------
// Window-scope commands
// ---------------------------------------------------------------------------

/**
 * Put the window in an exactly known state: CONTENT size (not frame size, which
 * varies with the title bar) and zoom level. A capture run that does not pin
 * both produces frames that differ between machines.
 */
async function configureWindow({ width, height, zoom = 0 }) {
  if (!mainWindow) throw new Error('No dashboard window');

  mainWindow.setFullScreen(false);
  mainWindow.unmaximize();
  mainWindow.setResizable(true);

  if (width && height) {
    const [currentWidth, currentHeight] = mainWindow.getContentSize();
    if (currentWidth !== width || currentHeight !== height) {
      const resized = new Promise((resolve) => mainWindow.once('resize', resolve));
      mainWindow.setContentSize(Math.round(width), Math.round(height));
      await Promise.race([resized, new Promise((resolve) => setTimeout(resolve, 2000))]);
    }
  }

  mainWindow.webContents.setZoomLevel(zoom);
  mainWindow.show();
  mainWindow.focus();

  // One presented frame with the new geometry before anything is measured.
  await new Promise((resolve) => setImmediate(resolve));
  await callRenderer('wait.stable', {});

  return windowState();
}

function windowState() {
  const [contentWidth, contentHeight] = mainWindow.getContentSize();
  const bounds = mainWindow.getBounds();
  return {
    content: { width: contentWidth, height: contentHeight },
    bounds,
    zoom: mainWindow.webContents.getZoomLevel(),
    fullScreen: mainWindow.isFullScreen(),
  };
}

/**
 * Who this window is, in the terms macOS uses to find it: the owning process and
 * the window title. The driver turns that into a CoreGraphics window id and hands
 * it to `screencapture`. Reporting identity rather than pixels is what keeps the
 * capture native — the app is never in the imaging path.
 */
function windowIdentity() {
  if (!mainWindow) throw new Error('No dashboard window');
  return {
    pid: process.pid,
    title: mainWindow.getTitle(),
    visible: mainWindow.isVisible(),
    minimized: mainWindow.isMinimized(),
    ...windowState(),
  };
}

/** Raise the window so macOS composites it unobscured before a capture. */
async function focusWindow() {
  if (!mainWindow) throw new Error('No dashboard window');
  if (mainWindow.isMinimized()) mainWindow.restore();
  mainWindow.show();
  mainWindow.focus();
  app.focus({ steal: true });
  await callRenderer('wait.stable', {});
  return windowIdentity();
}

const WINDOW_COMMANDS = {
  'window.configure': configureWindow,
  'window.state': async () => windowState(),
  'window.identity': async () => windowIdentity(),
  'window.focus': focusWindow,
  'app.quit': async () => {
    setTimeout(() => app.quit(), 100);
    return { quitting: true };
  },
};

// ---------------------------------------------------------------------------
// HTTP control endpoint
// ---------------------------------------------------------------------------

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', () => {
      const raw = Buffer.concat(chunks).toString('utf8');
      if (!raw) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(raw));
      } catch (err) {
        reject(new Error(`Invalid JSON body: ${err.message}`));
      }
    });
    req.on('error', reject);
  });
}

function sendJson(res, status, payload) {
  const body = JSON.stringify(payload);
  res.writeHead(status, {
    'Content-Type': 'application/json',
    'Content-Length': Buffer.byteLength(body),
    'Access-Control-Allow-Origin': '*',
  });
  res.end(body);
}

/** Serve a scene's media file to the renderer, confined to the media root. */
async function serveMedia(req, res, name) {
  if (!mediaRoot) {
    sendJson(res, 404, { ok: false, error: 'No media directory configured' });
    return;
  }

  const target = path.resolve(mediaRoot, decodeURIComponent(name));
  // The renderer is trusted, but the path still comes from scene text: confine
  // it to the media directory so a stray "../" cannot read the rest of the disk.
  if (!target.startsWith(path.resolve(mediaRoot) + path.sep)) {
    sendJson(res, 403, { ok: false, error: 'Media path escapes the media directory' });
    return;
  }

  try {
    const data = await fsp.readFile(target);
    res.writeHead(200, {
      'Content-Type': 'application/octet-stream',
      'Content-Length': data.length,
      'Access-Control-Allow-Origin': '*',
    });
    res.end(data);
  } catch (err) {
    sendJson(res, 404, { ok: false, error: `Media not found: ${name} (${err.code})` });
  }
}

async function handleRequest(req, res) {
  if (req.method === 'OPTIONS') {
    res.writeHead(204, {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Headers': 'Content-Type',
      'Access-Control-Allow-Methods': 'POST, GET, OPTIONS',
    });
    res.end();
    return;
  }

  const url = new URL(req.url, `http://${HOST}`);

  if (req.method === 'GET' && url.pathname === '/ready') {
    // The driver polls this before its first command, so a scene never runs
    // against a dashboard that has not finished loading its runtime.
    let runtimeReady = false;
    try {
      runtimeReady = Boolean(
        mainWindow && (await mainWindow.webContents.executeJavaScript('window.__SB_DEMO_READY__ === true'))
      );
    } catch (_) {
      runtimeReady = false;
    }
    sendJson(res, 200, { ok: true, ready: runtimeReady });
    return;
  }

  if (req.method === 'GET' && url.pathname.startsWith('/media/')) {
    await serveMedia(req, res, url.pathname.slice('/media/'.length));
    return;
  }

  if (req.method !== 'POST' || url.pathname !== '/command') {
    sendJson(res, 404, { ok: false, error: 'Not found' });
    return;
  }

  try {
    const { name, args } = await readBody(req);
    if (!name) throw new Error('Command name is required');

    const windowCommand = WINDOW_COMMANDS[name];
    const result = windowCommand ? await windowCommand(args || {}) : await callRenderer(name, args || {});
    sendJson(res, 200, { ok: true, result: result === undefined ? null : result });
  } catch (err) {
    sendJson(res, 200, { ok: false, error: err.message });
  }
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/**
 * Start the bridge. Returns the port it is listening on, which is also printed
 * to stdout so the driver can find it without a fixed port.
 *
 * @param {BrowserWindow} window
 * @param {object} options
 * @param {number} [options.port]      0 lets the OS choose
 * @param {string} [options.mediaDir]  directory the scene's audio is read from
 */
function startDemoBridge(window, { port = 0, mediaDir = null } = {}) {
  mainWindow = window;
  mediaRoot = mediaDir;

  return new Promise((resolve, reject) => {
    server = http.createServer((req, res) => {
      handleRequest(req, res).catch((err) => sendJson(res, 500, { ok: false, error: err.message }));
    });
    server.on('error', reject);
    server.listen(port, HOST, () => {
      const actual = server.address().port;
      console.log(`SCREENERBOT_DEMO_BRIDGE:${actual}`);
      resolve(actual);
    });
  });
}

function stopDemoBridge() {
  if (server) {
    server.close();
    server = null;
  }
  pending.forEach(({ reject, timer }) => {
    clearTimeout(timer);
    reject(new Error('Demo bridge stopped'));
  });
  pending.clear();
}

module.exports = { startDemoBridge, stopDemoBridge };
