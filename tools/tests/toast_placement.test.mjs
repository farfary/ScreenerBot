// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: toasts stack in one corner that never covers navigation. The top of the
 * window holds the header, the navigation and each page's sub-tab and status strips,
 * so the toast container is anchored to the bottom inline-end corner, above the fixed
 * status bar, and only its owner (`styles/components/toast.css`) places it.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const OWNER = "src/webserver/templates/styles/components/toast.css";
const PLACEMENT = /(?:^|;)\s*(top|bottom|left|right|inset(?:-[a-z-]+)?|transform)\s*:\s*([^;]+)/g;

async function containerPlacements() {
  const placements = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    const css = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const selector = match[1]
        .trim()
        .split(/\s*\n\s*/)
        .join(" ");
      if (!/\.toast-container(?![\w-])/.test(selector)) continue;
      for (const [, property, value] of match[2].matchAll(PLACEMENT)) {
        placements.push({ file: repoPath(file), selector, property, value: value.trim() });
      }
    }
  }
  return placements;
}

test("only the toast owner places the toast container", async () => {
  const outside = (await containerPlacements())
    .filter((placement) => placement.file !== OWNER)
    .map((placement) => `${placement.file} ${placement.selector} -> ${placement.property}`);
  assert.deepEqual(outside, []);
});

test("the toast container sits in the bottom inline-end corner", async () => {
  const placements = await containerPlacements();
  const properties = new Set(placements.map((placement) => placement.property));
  assert.ok(properties.has("inset-block-end"), "anchored to the block end");
  assert.ok(properties.has("inset-inline-end"), "anchored to the inline end");
  const toward = placements
    .filter((placement) => !["inset-block-end", "inset-inline-end"].includes(placement.property))
    .map((placement) => `${placement.selector} -> ${placement.property}: ${placement.value}`);
  assert.deepEqual(toward, []);
});
