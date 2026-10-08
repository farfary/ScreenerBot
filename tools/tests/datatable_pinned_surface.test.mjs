// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a pinned DataTable cell paints exactly one opaque surface,
 * `--dt-sticky-surface`, and the pinned edge fade starts from that same value.
 *
 * A row state that sets the pinned cell's `background` directly (a tint layered as
 * an image, a bare low-alpha colour) leaves the edge fade on the old colour or lets
 * the columns scrolling beneath show through, which reads as glyph slivers beside
 * the pinned column. Row states set the variable instead.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const rules = (css) =>
  [...css.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => ({
    selector: match[1].trim(),
    body: match[2],
  }));

test("pinned cells take their background only from --dt-sticky-surface", async () => {
  const found = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    for (const { selector, body } of rules(readFileSync(file, "utf8"))) {
      if (!/\bdt-col-sticky\b(?!-last::after)/.test(selector) || selector.includes("::after")) continue;
      for (const declaration of body.matchAll(/(?:^|;)\s*(background(?:-color|-image)?)\s*:\s*([^;]+)/g)) {
        if (declaration[2].trim() !== "var(--dt-sticky-surface)")
          found.push(`${repoPath(file)}: ${selector.split("\n")[0]} -> ${declaration[1]}`);
      }
    }
  }
  assert.deepEqual(found, []);
});

test("the pinned edge fade starts from the row's pinned surface", () => {
  const css = readFileSync(`${STYLES_ROOT}/ui/data_table/core.css`, "utf8");
  const edge = rules(css).find(({ selector }) => selector.includes("td.dt-col-sticky-last::after"));
  assert.ok(edge, "no pinned edge rule in core.css");
  assert.match(edge.body, /var\(--dt-sticky-surface\)/);
});
