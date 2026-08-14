/**
 * Promo capture bridge.
 *
 * Present only when the app is launched with SCREENERBOT_PROMO_CONTROL set — the
 * promo studio driver does that; a normal launch never loads any of this.
 *
 * It exposes one local HTTP endpoint the driver posts commands to, and routes
 * each command to whoever can actually observe its completion:
 *
 *   - window shape and identity                 -> this process
 *   - navigation, interaction, overlays, audio  -> the dashboard's promo runtime
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
const { app, ipcMain, screen } = require('electron');

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
    mainWindow.webContents.send('promo:command', { id, name, args });
  });
}

ipcMain.on('promo:result', (_event, { id, ok, result, error }) => {
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
 * Largest CONTENT size that still leaves the whole window frame inside the
 * display's work area (the screen minus the menu bar and Dock).
 *
 * `screencapture -l` composites the window layer, so a window hanging off the
 * screen edge is captured as a cropped or blank strip — and the operator cannot
 * see what is being captured either. The chrome delta is measured from the live
 * window rather than assumed, because the title bar height depends on
 * titleBarStyle and on the OS version.
 */
function maxContentSize() {
  const { workArea } = screen.getDisplayMatching(mainWindow.getBounds());
  const [contentWidth, contentHeight] = mainWindow.getContentSize();
  const bounds = mainWindow.getBounds();
  return {
    width: workArea.width - (bounds.width - contentWidth),
    height: workArea.height - (bounds.height - contentHeight),
    workArea,
  };
}

/** Zoom levels tried by `zoom: 'fit'`, in order. -1 is roughly 83%. */
const FIT_ZOOM_STEPS = [0, -1, -2];

/**
 * Choose the largest zoom level at which the app's chrome is not clipped.
 *
 * The main nav is a single non-wrapping row of tabs that needs more width than a
 * laptop display has, so at zoom 0 the last tabs are cut off and every
 * screenshot looks like the product is broken. Rather than hardcode a zoom that
 * happens to suit one Mac, each step is applied and then MEASURED in the page —
 * the same "resolve on an observed signal" rule the rest of the runtime follows.
 */
async function fitZoom() {
  let last = null;

  for (const level of FIT_ZOOM_STEPS) {
    mainWindow.webContents.setZoomLevel(level);
    await callRenderer('wait.viewport', {});
    last = await callRenderer('promo.measureChrome', {});
    if (!last.clipped) return level;
  }

  // Nothing fit. Keep the smallest zoom rather than failing the run: a slightly
  // clipped nav is still a usable capture, and the state is reported either way.
  return FIT_ZOOM_STEPS[FIT_ZOOM_STEPS.length - 1];
}

/**
 * Put the window in an exactly known state: CONTENT size (not frame size, which
 * varies with the title bar) and zoom level. A capture run that does not pin
 * both produces frames that differ between machines.
 *
 * The requested size is clamped to the display and the window is centred in it,
 * so a scene written for a large screen still produces a whole, visible window on
 * a smaller one. Pass width/height 0 (or omit them) to ask for the largest window
 * this display can show. The applied size and the clamp are reported back, so the
 * run's manifest records what was really captured rather than what was asked for.
 */
async function configureWindow({ width = 0, height = 0, zoom = 0 }) {
  if (!mainWindow) throw new Error('No dashboard window');

  mainWindow.setFullScreen(false);
  mainWindow.unmaximize();
  mainWindow.setResizable(true);

  const limit = maxContentSize();
  const requested = {
    width: Math.round(width) || limit.width,
    height: Math.round(height) || limit.height,
  };
  const target = {
    width: Math.min(requested.width, limit.width),
    height: Math.min(requested.height, limit.height),
  };

  const [currentWidth, currentHeight] = mainWindow.getContentSize();
  if (currentWidth !== target.width || currentHeight !== target.height) {
    const resized = new Promise((resolve) => mainWindow.once('resize', resolve));
    mainWindow.setContentSize(target.width, target.height);
    await Promise.race([resized, new Promise((resolve) => setTimeout(resolve, 2000))]);
  }

  // Centre the frame in the work area. Read the bounds back first: the window's
  // own minimum size can refuse part of the resize above, and centring the size
  // we asked for rather than the size we got would push it off the screen again.
  const bounds = mainWindow.getBounds();
  mainWindow.setPosition(
    Math.round(limit.workArea.x + (limit.workArea.width - bounds.width) / 2),
    Math.round(limit.workArea.y + (limit.workArea.height - bounds.height) / 2)
  );

  mainWindow.webContents.setZoomLevel(zoom === 'fit' ? 0 : zoom);
  mainWindow.show();
  mainWindow.focus();

  // Wait for the new geometry to reach the renderer and stop moving — not for
  // full quiescence. Configure runs during app boot, when the dashboard is still
  // loading and every poller is live, so a "nothing in flight" bar here is one
  // the app may never clear. The scene's freeze() is what settles the page.
  await new Promise((resolve) => setImmediate(resolve));
  await callRenderer('wait.viewport', {});

  if (zoom === 'fit') await fitZoom();

  const state = windowState();
  return {
    ...state,
    requested,
    clamped: state.content.width !== requested.width || state.content.height !== requested.height,
    displayLimit: { width: limit.width, height: limit.height },
  };
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

/**
 * Resolve as soon as the window is not the focused window any more.
 *
 * Starting `screencapture -v` activates the recorder and leaves the app's title
 * bar drawn in its INACTIVE state — grey traffic lights for the length of the
 * clip. There is no signal from screencapture itself that recording has begun,
 * but losing focus IS observable from in here, and it is precisely the thing
 * that has to be corrected. The driver waits for it and then re-focuses.
 *
 * Bounded, and returns what actually happened: if focus was never taken, the
 * caller re-focuses anyway and nothing is harmed.
 */
async function awaitBlur({ timeout = 4000 } = {}) {
  if (!mainWindow) throw new Error('No dashboard window');
  if (!mainWindow.isFocused()) return { blurred: true, waited: false };

  return new Promise((resolve) => {
    const finish = (blurred) => {
      clearTimeout(timer);
      mainWindow.removeListener('blur', onBlur);
      resolve({ blurred, waited: true });
    };
    const onBlur = () => finish(true);
    const timer = setTimeout(() => finish(false), timeout);
    mainWindow.once('blur', onBlur);
  });
}

/**
 * The window's rectangle in PHYSICAL pixels, relative to its display's origin.
 *
 * This is what lets a full-display recording be cut down to exactly the window:
 * the numbers come from the window and the display, never from measuring the
 * screen, so the crop is as window-targeted as `-l` is. `isPrimary` matters
 * because `screencapture -D 1` records the primary display.
 */
function captureRect() {
  if (!mainWindow) throw new Error('No dashboard window');

  const display = screen.getDisplayMatching(mainWindow.getBounds());
  const bounds = mainWindow.getBounds();
  const scale = display.scaleFactor;

  return {
    x: Math.round((bounds.x - display.bounds.x) * scale),
    y: Math.round((bounds.y - display.bounds.y) * scale),
    width: Math.round(bounds.width * scale),
    height: Math.round(bounds.height * scale),
    scaleFactor: scale,
    isPrimary: display.id === screen.getPrimaryDisplay().id,
  };
}

const WINDOW_COMMANDS = {
  'window.configure': configureWindow,
  'window.awaitBlur': awaitBlur,
  'window.captureRect': async () => captureRect(),
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
        mainWindow && (await mainWindow.webContents.executeJavaScript('window.__SB_PROMO_READY__ === true'))
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
function startPromoBridge(window, { port = 0, mediaDir = null } = {}) {
  mainWindow = window;
  mediaRoot = mediaDir;

  return new Promise((resolve, reject) => {
    server = http.createServer((req, res) => {
      handleRequest(req, res).catch((err) => sendJson(res, 500, { ok: false, error: err.message }));
    });
    server.on('error', reject);
    server.listen(port, HOST, () => {
      const actual = server.address().port;
      console.log(`SCREENERBOT_PROMO_BRIDGE:${actual}`);
      resolve(actual);
    });
  });
}

function stopPromoBridge() {
  if (server) {
    server.close();
    server = null;
  }
  pending.forEach(({ reject, timer }) => {
    clearTimeout(timer);
    reject(new Error('Promo bridge stopped'));
  });
  pending.clear();
}

module.exports = { startPromoBridge, stopPromoBridge };
