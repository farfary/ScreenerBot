// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the shared search field (`.search-wrapper` with its `.search-icon` and
 * `.search-clear`) is declared once, by `styles/components/form_controls.css`.
 *
 * A second copy in `foundation.css` once declared the same parts with other sizes and
 * a different show/hide mechanism, so what a search field looked like depended on
 * which declaration won each property. A page may place or size a search field from
 * its own selector, never restate the shared one.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const OWNER = "src/webserver/templates/styles/components/form_controls.css";

test("only the form controls stylesheet declares the shared search field", async () => {
  const found = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    for (const rule of rulesIn(readFileSync(file, "utf8"))) {
      if (/^\.search-wrapper(?![\w-])/.test(rule.selector)) {
        found.push({ file: repoPath(file), line: rule.line, selector: rule.selector });
      }
    }
  }
  assert.ok(found.some((rule) => rule.file === OWNER), "form_controls.css declares the field");
  assert.deepEqual(
    found.filter((rule) => rule.file !== OWNER).map((rule) => `${rule.file}:${rule.line}`),
    []
  );
});
