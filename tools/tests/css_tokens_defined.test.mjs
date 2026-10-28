// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every custom property a dashboard stylesheet reads without a fallback is defined.
 *
 * A `var(--name)` with no definition makes the whole declaration invalid at computed-value
 * time, so the property silently falls back to its inherited or initial value: a hover
 * border that never appears, a tint that renders transparent. A definition is any
 * `--name:` declaration under the templates (a stylesheet, an inline style or a script
 * that writes one) or a `setProperty("--name", …)` call.
 *
 * The files below still read a property nothing defines; each fix removes its entry, and
 * the list only shrinks.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { relative } from "node:path";

import { STYLES_ROOT, TEMPLATES_ROOT, walk } from "../lib/dashboard_ui.mjs";

/** Stylesheet (under `styles/`) to the undefined properties it still reads. */
const UNDEFINED_ALLOWED = {
  "config_import_export_dialog.css": ["--primary-alpha-05"],
  "pages/assistant/automation.css": ["--info"],
  "pages/assistant/instructions.css": ["--info"],
  "pages/assistant/providers.css": ["--danger-color"],
  "pages/filtering/results_explorer.css": ["--accent-bg", "--accent-color", "--border-hover"],
  "pages/tokens.css": ["--badge-warning"],
  "token_details/base.css": ["--accent-primary"],
};

/** The source with its block comments blanked, so a commented-out rule counts for nothing. */
const withoutComments = (source) =>
  source.replace(/\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\n]/g, " "));

test("every custom property read without a fallback is defined under the templates", async () => {
  const sources = (await walk(TEMPLATES_ROOT)).filter((path) => /\.(?:css|html|js|mjs)$/.test(path));
  const defined = new Set();
  for (const path of sources) {
    const source = withoutComments(readFileSync(path, "utf8"));
    for (const match of source.matchAll(/(--[\w-]+)\s*:/g)) defined.add(match[1]);
    for (const match of source.matchAll(/setProperty\(\s*["'`](--[\w-]+)/g)) defined.add(match[1]);
  }

  const undefinedReads = {};
  for (const path of sources.filter((file) => file.startsWith(STYLES_ROOT) && file.endsWith(".css"))) {
    const source = withoutComments(readFileSync(path, "utf8"));
    const names = new Set();
    for (const match of source.matchAll(/var\(\s*(--[\w-]+)\s*\)/g)) {
      if (!defined.has(match[1])) names.add(match[1]);
    }
    if (names.size > 0) undefinedReads[relative(STYLES_ROOT, path)] = [...names].sort();
  }

  assert.deepEqual(
    undefinedReads,
    UNDEFINED_ALLOWED,
    "define the property or give the read a fallback; a fixed file leaves the list"
  );
});
