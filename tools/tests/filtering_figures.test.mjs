// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Filtering page shows each kind of figure one way.
 *
 * - The explorer's sidebar tree and its overview lists sit on one screen and name the
 *   same rejection counts, so every count there is the full grouped number ("1,964");
 *   a compact form ("1.96K") beside it reads as a different figure. The sidebar label
 *   ellipsizes instead.
 * - Every share on the page (status strip, analytics cards, category and source bars,
 *   the explorer rate) goes through the page's one share formatter at one precision.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT } from "../lib/dashboard_ui.mjs";

const RENDERERS = readFileSync(`${SCRIPTS_ROOT}/pages/filtering/renderers.js`, "utf8");

/** Source of `name` up to the next top-level renderer function. */
function functionSource(name) {
  const start = RENDERERS.indexOf(`function ${name}(`);
  assert.ok(start >= 0, `renderers.js defines ${name}`);
  const end = RENDERERS.indexOf("\n  function ", start + 1);
  return RENDERERS.slice(start, end < 0 ? undefined : end);
}

test("every explorer count is the full grouped number", () => {
  let checked = 0;
  for (const name of ["renderExplorerView", "renderExplorerDashboard"]) {
    const source = functionSource(name);
    assert.doesNotMatch(source, /formatCompactNumber/, `${name} compacts a count`);
    const counts = [
      ...source.matchAll(
        /class="(?:tree-count|tree-reason-count|overview-item-count)">\$\{([^}]*)\}/g
      ),
    ];
    for (const [, expression] of counts) {
      assert.match(expression, /^Utils\.formatNumber\([^,]+, 0\)$/, `${name}: ${expression}`);
      checked += 1;
    }
  }
  assert.equal(checked, 4, "sidebar total, category, reason and overview counts");
});

test("every share on the page goes through one formatter", () => {
  const calls = [...RENDERERS.matchAll(/formatPercent\w*\(/g)];
  assert.equal(calls.length, 1, "only shareText formats a percentage");
  assert.match(
    functionSource("shareText"),
    /formatPercentValue\(rate, \{ includeSign: false, decimals: 1 \}\)/
  );
});
