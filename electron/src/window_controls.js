// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Dims the native macOS traffic-light buttons while the dashboard is under a modal scrim, and owns their position.

'use strict';

const path = require('path');

// The traffic lights are AppKit views above the web contents, so a dialog's
// scrim darkens everything in the window except them. The dashboard reports
// when its title bar is under a scrim (scripts/core/window_controls.js) and
// this module fades the real buttons to match, through the native helper built
// by tools/macos-native.js. The buttons keep working while dimmed: hover glyphs,
// close, minimize and the zoom button's tiling menu are unaffected by opacity.
//
// This module never imports `electron`; callers pass the window, so plain node
// tests can exercise it.

/** Where `titleBarStyle: 'hiddenInset'` places the buttons; the header leaves room for them. */
const TRAFFIC_LIGHT_POSITION = Object.freeze({ x: 16, y: 16 });

/** Button opacity under a scrim; over the darkened header it reads as the same dim. */
const DIMMED_ALPHA = 0.35;

/** The compiled helper, outside `src/` so the build output never enters the shell revision. */
const NATIVE_HELPER_PATH = path.join(__dirname, '..', 'build', 'window_buttons.node');

/**
 * @typedef {{ setAlpha: (handle: Buffer, alpha: number) => void }} NativeHelper
 * @typedef {{
 *   isDestroyed: () => boolean,
 *   getNativeWindowHandle: () => Buffer,
 *   setWindowButtonPosition: (position: { x: number, y: number }) => void,
 *   on: (event: string, listener: () => void) => void,
 *   webContents: { on: (event: string, listener: (details: object) => void) => void },
 * }} ControlledWindow
 */

/**
 * Build the window-controls owner around a native helper loader.
 *
 * @param {() => NativeHelper | null} loadHelper - returns the helper, or null where it cannot load
 * @param {{ warn: (...args: unknown[]) => void }} [log]
 */
function createWindowControls(loadHelper, log = console) {
  const dimmed = new WeakMap();
  let helper;

  function nativeHelper() {
    if (helper === undefined) {
      try {
        helper = loadHelper();
      } catch (err) {
        log.warn('[Electron] Window button helper unavailable; traffic lights will not dim:', err.message);
        helper = null;
      }
    }
    return helper;
  }

  function apply(win, native, dim) {
    native.setAlpha(win.getNativeWindowHandle(), dim ? DIMMED_ALPHA : 1);
    // A button opacity change makes AppKit lay the title bar out again, which
    // returns the buttons to the system position until Electron is told again.
    win.setWindowButtonPosition(TRAFFIC_LIGHT_POSITION);
  }

  /**
   * Dim or restore the window's traffic lights. A repeated state is a no-op.
   *
   * @param {ControlledWindow} win
   * @param {boolean} dim
   * @returns {boolean} whether the buttons now show `dim`
   */
  function setDimmed(win, dim) {
    if (!win || win.isDestroyed()) return false;
    const next = dim === true;
    if ((dimmed.get(win) ?? false) === next) return true;
    const native = nativeHelper();
    if (!native) return false;

    apply(win, native, next);
    dimmed.set(win, next);
    return true;
  }

  /**
   * Keep the buttons in step with the window: restore them whenever the page
   * that dimmed them goes away, since a reloaded or crashed renderer can no
   * longer report its scrim closing, and dim them again after a full-screen
   * transition, which rebuilds the title bar with opaque buttons.
   *
   * @param {ControlledWindow} win
   */
  function attach(win) {
    win.webContents.on('did-start-navigation', (details) => {
      if (details.isMainFrame && !details.isSameDocument) setDimmed(win, false);
    });
    win.webContents.on('render-process-gone', () => setDimmed(win, false));
    const redim = () => {
      const native = dimmed.get(win) && !win.isDestroyed() ? nativeHelper() : null;
      if (native) apply(win, native, true);
    };
    win.on('enter-full-screen', redim);
    win.on('leave-full-screen', redim);
  }

  return { setDimmed, attach };
}

const windowControls = createWindowControls(() =>
  process.platform === 'darwin' ? require(NATIVE_HELPER_PATH) : null
);

module.exports = {
  TRAFFIC_LIGHT_POSITION,
  DIMMED_ALPHA,
  NATIVE_HELPER_PATH,
  createWindowControls,
  setWindowControlsDimmed: windowControls.setDimmed,
  attachWindowControls: windowControls.attach,
};
