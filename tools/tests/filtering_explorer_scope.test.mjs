// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Filtering explorer always describes the current state.
 *
 * The explorer's per-reason lists read the current rejections, so its tree and overview
 * counts must come from an analytics payload fetched with no time range. The Analytics
 * tab alone follows `state.timeRange`.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT } from "../lib/dashboard_ui.mjs";

const PAGE = readFileSync(`${SCRIPTS_ROOT}/pages/filtering.js`, "utf8");
const RENDERERS = readFileSync(`${SCRIPTS_ROOT}/pages/filtering/renderers.js`, "utf8");

/** Source of a top-level async function of the page module, up to the next top-level block. */
function pageFunction(name) {
  const start = PAGE.indexOf(`async function ${name}(`);
  assert.ok(start >= 0, `filtering.js defines ${name}`);
  const end = PAGE.indexOf("\n}\n", start);
  return PAGE.slice(start, end + 3);
}

/** Source of a renderer function, up to the next renderer. */
function rendererFunction(name) {
  const start = RENDERERS.indexOf(`function ${name}(`);
  assert.ok(start >= 0, `renderers.js defines ${name}`);
  const end = RENDERERS.indexOf("\n  function ", start + 1);
  return RENDERERS.slice(start, end < 0 ? undefined : end);
}

test("fetchAnalytics takes its time range from the caller, never from page state", () => {
  const source = pageFunction("fetchAnalytics");
  assert.doesNotMatch(source, /state\.timeRange/);
  assert.match(source, /fetchAnalytics\(range\)/);
});

test("only the analytics loader sends the time range", () => {
  assert.match(pageFunction("loadAnalytics"), /fetchAnalytics\(state\.timeRange\)/);

  const explorer = pageFunction("loadExplorerOverview");
  assert.match(explorer, /fetchAnalytics\(\)/);
  assert.doesNotMatch(explorer, /timeRange/);
  assert.doesNotMatch(explorer, /state\.analytics\b/);

  const calls = [...PAGE.matchAll(/fetchAnalytics\(([^)]*)\)/g)].map((m) => m[1]);
  assert.deepEqual(calls.sort(), ["", "range", "state.timeRange"]);
});

test("the explorer renders only its own overview payload", () => {
  const explorerView = rendererFunction("renderExplorerView");
  assert.doesNotMatch(explorerView, /state\.analytics\b/);
  assert.match(explorerView, /state\.explorerOverview/);

  const dashboardCalls = [...PAGE.matchAll(/renderExplorerDashboard\(([^)]*)\)/g)].map((m) => m[1]);
  assert.ok(dashboardCalls.length >= 1, "the page repaints the explorer overview");
  for (const argument of dashboardCalls) {
    assert.equal(argument, "state.explorerOverview");
  }
});

test("explorer loading is guarded by its own request id and hash", () => {
  const source = pageFunction("loadExplorerOverview");
  assert.match(source, /state\.explorerRequestId/);
  assert.match(source, /_lastExplorerKey/);
  assert.doesNotMatch(source, /analyticsRequestId|_lastAnalyticsKey/);
});
