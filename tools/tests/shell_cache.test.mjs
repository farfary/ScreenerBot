// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Tests for the shell cache reset (`electron/src/shell_cache.js`): both caches are cleared, a
// failure is logged and never stops the launch, and the shell clears them before its first page
// load.

import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";

const require = createRequire(import.meta.url);
const { clearShellCaches } = require("../../electron/src/shell_cache.js");

function fakeSession({ failHttp = false, failCode = false } = {}) {
  const calls = [];
  return {
    calls,
    async clearCache() {
      calls.push(["clearCache"]);
      if (failHttp) throw new Error("http cache locked");
    },
    async clearCodeCaches(options) {
      calls.push(["clearCodeCaches", options]);
      if (failCode) throw new Error("code cache locked");
    },
  };
}

function recordingLog() {
  const warnings = [];
  return { warnings, warn: (...args) => warnings.push(args.join(" ")) };
}

test("the HTTP cache and every code cache are cleared", async () => {
  const session = fakeSession();
  const log = recordingLog();
  const result = await clearShellCaches(session, log);

  assert.deepEqual(result, { httpCache: true, codeCaches: true });
  assert.deepEqual(session.calls, [["clearCache"], ["clearCodeCaches", {}]]);
  assert.deepEqual(log.warnings, []);
});

test("a cache that cannot be cleared is logged and the other is still cleared", async () => {
  const session = fakeSession({ failHttp: true });
  const log = recordingLog();
  const result = await clearShellCaches(session, log);

  assert.deepEqual(result, { httpCache: false, codeCaches: true });
  assert.equal(session.calls.length, 2);
  assert.equal(log.warnings.length, 1);
  assert.match(log.warnings[0], /HTTP cache could not be cleared: http cache locked/);
});

test("a session method that throws synchronously never rejects the launch", async () => {
  const log = recordingLog();
  const result = await clearShellCaches(
    {
      clearCache() {
        throw new Error("no session");
      },
      clearCodeCaches() {
        throw new Error("no session");
      },
    },
    log
  );
  assert.deepEqual(result, { httpCache: false, codeCaches: false });
  assert.equal(log.warnings.length, 2);
});

test("the shell clears the caches before it creates the window or loads any page", () => {
  const main = readFileSync(new URL("../../electron/src/main.js", import.meta.url), "utf8");
  const start = main.indexOf("async function initialize()");
  assert.ok(start >= 0, "initialize() exists");
  assert.match(main, /app\.whenReady\(\)\.then\(initialize\)/);
  const body = main.slice(start);
  const cleared = body.indexOf("await clearShellCaches(session.defaultSession)");
  assert.ok(cleared >= 0, "initialize() awaits clearShellCaches");
  for (const first of ["createWindow()", "loadLoadingPage()"]) {
    assert.ok(body.indexOf(first) > cleared, `${first} runs after the caches are cleared`);
  }
});
