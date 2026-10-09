// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a switch has two sizes and one off state. The normal size is the
 * `:root` geometry in `components/form_controls.css`; the only other size is the
 * section master switch, `data-level="root"`. No markup declares another level
 * or a size class, and no stylesheet outside the owner resizes or restyles the
 * track, so a settings section can no longer show three switch sizes side by
 * side.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { loadMarkupSources, loadStylesheets } from "../lib/dashboard_ui.mjs";

const OWNER = "components/form_controls.css";

test("switch markup declares only the master size", async () => {
  const offenders = [];
  for (const { path, source } of await loadMarkupSources()) {
    source.split("\n").forEach((line, index) => {
      for (const match of line.matchAll(/data-level="([^"]*)"/g)) {
        if (match[1] !== "root") offenders.push(`${path}:${index + 1} data-level="${match[1]}"`);
      }
      if (/\btoggle-(?:sm|lg|xs)\b/.test(line)) offenders.push(`${path}:${index + 1} size class`);
    });
  }
  assert.deepEqual(offenders, [], 'a switch is normal or data-level="root"');
});

test("only the owner sizes a switch, with exactly two sizes", async () => {
  const sheets = await loadStylesheets();
  const outside = [];
  const sizes = [];
  for (const { path, rules } of sheets) {
    for (const { selector, body, line } of rules) {
      const sizing = /--toggle-(?:width|height|knob-size)\s*:/.test(body);
      const restyle = /\.toggle-track\b/.test(selector);
      if (path !== OWNER) {
        if (sizing || restyle) outside.push(`${path}:${line} ${selector}`);
        continue;
      }
      if (sizing) sizes.push(selector);
    }
  }
  assert.deepEqual(outside, [], "switch geometry and the track skin live in the owner");
  assert.deepEqual(sizes.sort(), ['.toggle[data-level="root"]', ":root"].sort());
});
