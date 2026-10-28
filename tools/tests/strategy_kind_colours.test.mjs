// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a strategy's kind has one colour pair on every surface. Entry is the
 * primary blue and Exit the warning amber, declared once as the
 * `--strategy-kind-*` properties; no kind selector picks a success or error
 * colour of its own, so an Exit strategy never reads as a failure. The Entry and Exit
 * view switch of the strategies roster is a shared segmented control: its bare glyph
 * takes the option's colour like every other one, never a kind colour of its own.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const KIND_SELECTOR = /\.(?:entry|exit)\b|data-type="(?:ENTRY|EXIT)"|data-seg-value="(?:entry|exit)"/;
const SEMANTIC_COLOUR = /--(?:success|error|danger)-|rgb\(/;

test("strategy kinds take one blue and amber pair", async () => {
  const pairs = new Map();
  const offenders = [];
  for (const file of (await walk(`${STYLES_ROOT}/pages`)).filter((path) => path.endsWith(".css"))) {
    if (!/strateg/.test(file)) continue;
    for (const rule of rulesIn(readFileSync(file, "utf8"))) {
      if (!KIND_SELECTOR.test(rule.selector)) continue;
      if (SEMANTIC_COLOUR.test(rule.body)) offenders.push(`${repoPath(file)}: ${rule.selector}`);
      const color = /--strategy-kind-color:\s*([^;]+);/.exec(rule.body)?.[1];
      // One declaration per kind: a selector list yields one rule per selector.
      if (color)
        pairs.set(`${repoPath(file)}:${rule.line}`, [
          /entry|ENTRY/.test(rule.selector) ? "entry" : "exit",
          color.trim(),
        ]);
    }
  }
  assert.deepEqual(offenders, [], "kind selectors carry no colour of their own");
  assert.deepEqual([...pairs.values()].sort(), [
    ["entry", "var(--primary-color)"],
    ["exit", "var(--warning-color)"],
  ]);
});

test("each roster view's bare glyph takes its option's colour", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("trader"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/trader`);
  await page.click('#subTabsContainer [data-tab-id="strategies"]');
  await page.waitForSelector("#strategies-roster tbody tr[data-row-id]");
  const views = await page.evaluate(() => {
    const doc = globalThis.document;
    const view = (kind) => {
      const option = doc
        .querySelector(`#strategies-roster input[data-seg-value="${kind}"]`)
        .closest(".segmented-option");
      const glyph = option.querySelector(".segmented-icon");
      const style = getComputedStyle(glyph);
      return {
        color: style.color === getComputedStyle(option.querySelector(".segmented-label")).color,
        bare:
          style.backgroundColor === "rgba(0, 0, 0, 0)" &&
          style.borderStyle === "none" &&
          style.boxShadow === "none",
        inline: glyph.parentElement === option.querySelector(".segmented-label"),
      };
    };
    return { entry: view("entry"), exit: view("exit") };
  });
  assert.deepEqual(views.entry, { color: true, bare: true, inline: true });
  assert.deepEqual(views.exit, { color: true, bare: true, inline: true });
});
