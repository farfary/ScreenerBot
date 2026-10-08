// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every button variant shares one disabled state, declared by the button owner.
 * A disabled button reads as unavailable through a muted label and a neutral surface,
 * never through opacity alone, and it keeps pointer events so a title can explain why
 * the action is unavailable. The state is declared once, in `styles/components.css`;
 * a page may style its enabled buttons but never restyles a disabled one.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const OWNER = "src/webserver/templates/styles/components.css";

const BUTTON = /\.btn(?:-icon)?(?![\w-])/;
const DISABLED = /:disabled|\[disabled\]|\[aria-disabled/;

/** True when one comma-separated selector styles a disabled button. */
const stylesDisabledButton = (selector) =>
  selector
    .split(",")
    .some((part) => BUTTON.test(part) && DISABLED.test(part.replace(/:not\([^)]*\)/g, "")));

async function disabledButtonRules() {
  const rules = [];
  for (const file of (await walk(STYLES_ROOT)).filter((path) => path.endsWith(".css"))) {
    const css = readFileSync(file, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    for (const match of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const selector = match[1]
        .trim()
        .split(/\s*\n\s*/)
        .join(" ");
      if (stylesDisabledButton(selector)) {
        rules.push({ file: repoPath(file), selector, body: match[2] });
      }
    }
  }
  return rules;
}

test("only the button owner styles a disabled button", async () => {
  const outside = (await disabledButtonRules())
    .filter((rule) => rule.file !== OWNER)
    .map((rule) => `${rule.file} ${rule.selector}`);
  assert.deepEqual(outside, []);
});

test("the shared disabled state keeps full opacity and pointer events", async () => {
  const owned = (await disabledButtonRules()).filter((rule) => rule.file === OWNER);
  assert.ok(owned.length > 0, "components.css declares the disabled button state");
  const fading = owned
    .filter((rule) => /(?:^|;)\s*(?:opacity|filter)\s*:|pointer-events\s*:\s*none/.test(rule.body))
    .map((rule) => rule.selector);
  assert.deepEqual(fading, []);
});
