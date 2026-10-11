// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: `core/dom.js` `paintHtml` / `paintText` write only changed content, so a poll
 * that changed nothing keeps an element's children (and their hover, focus and scroll),
 * and there is one owner of the rule: no page or dialog keeps a private copy.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, walk } from "../lib/dashboard_ui.mjs";

const { paintHtml, paintText, forgetPaint } = await import(
  new URL("../../src/webserver/templates/scripts/core/dom.js", import.meta.url)
);

// An element whose markup writes are counted.
function element() {
  let html = "";
  const node = { writes: 0, textContent: "" };
  Object.defineProperty(node, "innerHTML", {
    get: () => html,
    set: (value) => {
      html = value;
      node.writes += 1;
    },
  });
  return node;
}

test("paintHtml writes only markup that changed, per element", () => {
  const a = element();
  const b = element();
  assert.equal(paintHtml(a, "<b>1</b>"), true);
  assert.equal(paintHtml(a, "<b>1</b>"), false);
  assert.equal(paintHtml(b, "<b>1</b>"), true, "elements are tracked separately");
  assert.equal(paintHtml(a, "<b>2</b>"), true);
  assert.equal(a.writes, 2);
  assert.equal(paintHtml(null, "<b>1</b>"), false);
});

test("forgetPaint makes the next paint write unchanged markup", () => {
  const a = element();
  paintHtml(a, "x");
  forgetPaint(a);
  assert.equal(paintHtml(a, "x"), true);
  assert.equal(a.writes, 2);
});

test("paintText leaves an unchanged text in place", () => {
  let writes = 0;
  let text = "";
  const node = {
    get textContent() {
      return text;
    },
    set textContent(value) {
      text = value;
      writes += 1;
    },
  };
  paintText(node, "5");
  paintText(node, "5");
  paintText(node, "6");
  assert.equal(writes, 2);
  paintText(null, "5");
});

test("no script keeps a private paint-if-changed copy", async () => {
  const owner = new URL("../../src/webserver/templates/scripts/core/dom.js", import.meta.url)
    .pathname;
  const found = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    if (file === owner) continue;
    const source = readFileSync(file, "utf8");
    if (/new WeakMap\(\)[\s\S]{0,400}\.innerHTML\s*=/.test(source)) found.push(file);
    if (/_renderKeys|_paintRegion/.test(source)) found.push(file);
  }
  assert.deepEqual(found, []);
});
