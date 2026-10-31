// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Tests for the macOS traffic-light dimming owner (`electron/src/window_controls.js`): the buttons
// dim and restore in place, repeated states cost nothing, a missing helper never breaks the shell,
// a page that goes away restores them, a full-screen transition keeps them dimmed, and the native
// helper's recipe matches the bundle.

import test from "node:test";
import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import { createRequire } from "node:module";
import path from "node:path";

const require = createRequire(import.meta.url);
const {
  TRAFFIC_LIGHT_POSITION,
  DIMMED_ALPHA,
  NATIVE_HELPER_PATH,
  createWindowControls,
} = require("../../electron/src/window_controls.js");
const { clangArguments } = require("../../electron/tools/macos-native.js");

const HANDLE = Buffer.alloc(8);

function fakeWindow() {
  const calls = [];
  const webContents = new EventEmitter();
  const events = new EventEmitter();
  return {
    calls,
    webContents,
    on: (event, listener) => events.on(event, listener),
    emit: (event) => events.emit(event),
    destroyed: false,
    isDestroyed() {
      return this.destroyed;
    },
    getNativeWindowHandle: () => HANDLE,
    setWindowButtonPosition: (position) => calls.push(["position", position]),
  };
}

function fakeHelper(win) {
  return { setAlpha: (handle, alpha) => win.calls.push(["alpha", alpha, handle === HANDLE]) };
}

function recordingLog() {
  const warnings = [];
  return { warnings, warn: (...args) => warnings.push(args.join(" ")) };
}

test("dimming sets the button alpha and then puts the buttons back at their position", () => {
  const win = fakeWindow();
  const controls = createWindowControls(() => fakeHelper(win));

  assert.equal(controls.setDimmed(win, true), true);
  assert.equal(controls.setDimmed(win, false), true);
  // AppKit returns the buttons to the system position after an alpha change, so every
  // alpha change is followed by the position, in that order.
  assert.deepEqual(win.calls, [
    ["alpha", DIMMED_ALPHA, true],
    ["position", TRAFFIC_LIGHT_POSITION],
    ["alpha", 1, true],
    ["position", TRAFFIC_LIGHT_POSITION],
  ]);
});

test("a repeated state touches nothing native", () => {
  const win = fakeWindow();
  const controls = createWindowControls(() => fakeHelper(win));

  assert.equal(controls.setDimmed(win, false), true, "restored is the starting state");
  controls.setDimmed(win, true);
  controls.setDimmed(win, true);
  assert.equal(win.calls.length, 2);
});

test("only a literal true dims", () => {
  const win = fakeWindow();
  const controls = createWindowControls(() => fakeHelper(win));

  controls.setDimmed(win, "true");
  controls.setDimmed(win, 1);
  assert.deepEqual(win.calls, []);
});

test("a destroyed window is left alone", () => {
  const win = fakeWindow();
  win.destroyed = true;
  const controls = createWindowControls(() => fakeHelper(win));

  assert.equal(controls.setDimmed(win, true), false);
  assert.deepEqual(win.calls, []);
});

test("a helper that cannot load is reported once and never throws", () => {
  const win = fakeWindow();
  const log = recordingLog();
  let loads = 0;
  const controls = createWindowControls(() => {
    loads += 1;
    throw new Error("image not found");
  }, log);

  assert.equal(controls.setDimmed(win, true), false);
  assert.equal(controls.setDimmed(win, true), false);
  assert.equal(loads, 1);
  assert.equal(log.warnings.length, 1);
  assert.match(log.warnings[0], /image not found/);
  assert.deepEqual(win.calls, []);
});

test("a platform without the helper is a silent no-op", () => {
  const win = fakeWindow();
  const log = recordingLog();
  const controls = createWindowControls(() => null, log);

  assert.equal(controls.setDimmed(win, true), false);
  assert.deepEqual(log.warnings, []);
});

test("a main-frame navigation or a renderer crash restores dimmed buttons", () => {
  for (const [event, details] of [
    ["did-start-navigation", { isMainFrame: true, isSameDocument: false }],
    ["render-process-gone", {}],
  ]) {
    const win = fakeWindow();
    const controls = createWindowControls(() => fakeHelper(win));
    controls.attach(win);
    controls.setDimmed(win, true);
    win.calls.length = 0;

    win.webContents.emit(event, details);
    assert.deepEqual(win.calls[0], ["alpha", 1, true], event);
  }
});

test("in-page and subframe navigations keep the buttons dimmed", () => {
  const win = fakeWindow();
  const controls = createWindowControls(() => fakeHelper(win));
  controls.attach(win);
  controls.setDimmed(win, true);
  win.calls.length = 0;

  win.webContents.emit("did-start-navigation", { isMainFrame: true, isSameDocument: true });
  win.webContents.emit("did-start-navigation", { isMainFrame: false, isSameDocument: false });
  assert.deepEqual(win.calls, []);
});

test("a full-screen transition dims the rebuilt buttons again, and leaves restored ones alone", () => {
  for (const event of ["enter-full-screen", "leave-full-screen"]) {
    const win = fakeWindow();
    const controls = createWindowControls(() => fakeHelper(win));
    controls.attach(win);

    win.emit(event);
    assert.deepEqual(win.calls, [], `${event}: restored buttons need nothing`);

    controls.setDimmed(win, true);
    win.calls.length = 0;
    // AppKit rebuilds the title bar on the transition with opaque buttons, while the
    // remembered state still reads dimmed.
    win.emit(event);
    assert.deepEqual(win.calls, [
      ["alpha", DIMMED_ALPHA, true],
      ["position", TRAFFIC_LIGHT_POSITION],
    ], event);

    win.destroyed = true;
    win.calls.length = 0;
    win.emit(event);
    assert.deepEqual(win.calls, [], `${event}: a destroyed window is left alone`);
  }
});

test("the helper is built universal, for the bundle's minimum macOS, where the shell loads it", () => {
  const args = clangArguments({ minimumVersion: "13.0", includeDirectory: "/node/include" });
  const arches = args.flatMap((arg, i) => (arg === "-arch" ? [args[i + 1]] : []));

  assert.deepEqual(arches.sort(), ["arm64", "x86_64"]);
  assert.ok(args.includes("-mmacosx-version-min=13.0"));
  assert.equal(args[args.indexOf("-o") + 1], NATIVE_HELPER_PATH);
  // Outside src/: a compiled binary in the revision inputs would make every build a shell change.
  assert.equal(path.basename(path.dirname(NATIVE_HELPER_PATH)), "build");
});
