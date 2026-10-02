// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard for directional icons under `dir="rtl"`.
 *
 * Every Lucide icon class whose name contains `left` or `right` and is used by a
 * dashboard template must be classified: mirrored by
 * `styles/base/direction.css`, or listed in NOT_MIRRORED below.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const TEMPLATES = new URL("../../src/webserver/templates/", import.meta.url).pathname;
const DIRECTION_CSS = join(TEMPLATES, "styles/base/direction.css");

/*
 * Not mirrored by decision: trend and diagonal arrows and chart icons keep
 * their meaning (up/down, gain/loss) in every language; exchange arrows
 * (left-right, right-left) are symmetric; vertical arrows and media play are not
 * reading-order glyphs; toggle icons depict a switch state, not a direction.
 */
const NOT_MIRRORED = new Set([
  "icon-arrow-left-right",
  "icon-arrow-right-left",
  "icon-arrow-up-right",
  "icon-arrow-down-left",
  "icon-arrow-up-left",
  "icon-arrow-down-right",
  "icon-trending-up-right",
  "icon-toggle-left",
  "icon-toggle-right",
]);

function walk(dir, files = []) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path, files);
    else if (/\.(js|html)$/.test(name)) files.push(path);
  }
  return files;
}

function mirroredIcons() {
  const css = readFileSync(DIRECTION_CSS, "utf8");
  return new Set(css.match(/\.icon-[a-z0-9-]+(?=[,\s)])/g)?.map((name) => name.slice(1)));
}

test("direction.css mirrors the glyph pseudo-element under RTL", () => {
  const css = readFileSync(DIRECTION_CSS, "utf8");
  assert.match(css, /\[dir="rtl"\]/);
  assert.match(css, /\)::before\s*\{[^}]*transform:\s*scaleX\(-1\)/);
  assert.ok(mirroredIcons().size > 0);
});

test("every left/right icon class in a template is mirrored or explicitly not", () => {
  const mirrored = mirroredIcons();
  const unclassified = new Map();
  for (const file of walk(TEMPLATES)) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(/\bicon-[a-z0-9-]*(?:left|right)[a-z0-9-]*/g)) {
      const name = match[0];
      if (mirrored.has(name) || NOT_MIRRORED.has(name)) continue;
      unclassified.set(name, file.slice(TEMPLATES.length));
    }
  }
  assert.deepEqual(
    [...unclassified].map(([name, file]) => `${name} (${file})`),
    [],
    "classify in direction.css or NOT_MIRRORED",
  );
});
