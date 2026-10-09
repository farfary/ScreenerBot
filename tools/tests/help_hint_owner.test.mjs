// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every help hint is the one shared trigger from `ui/hint_popover.js`, a bare
 * "?" glyph. The Tools header once drew its own help button with an info glyph and a
 * separate click path, so the same help read as "i" on one page and "?" on every
 * other. No dashboard source spells a help glyph outside the owner, and the Tools
 * header renders the shared trigger beside the tool title, which opens the hint.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { loadMarkupSources } from "../lib/dashboard_ui.mjs";
import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const OWNER = "scripts/ui/hint_popover.js";
const HELP_GLYPH =
  /icon-(?:circle-question-mark|circle-help|help-circle|badge-question-mark)\b|help-btn\b/;
const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

test("only the hint owner draws a help glyph", async () => {
  const found = [];
  for (const { path, source } of await loadMarkupSources()) {
    if (path.endsWith(OWNER)) continue;
    source.split("\n").forEach((line, index) => {
      if (HELP_GLYPH.test(line)) found.push(`${path}:${index + 1}`);
    });
  }
  assert.deepEqual(found, []);
});

test("the Tools header shows the shared help trigger beside the title", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
  const api = createApiHandler([await loadIndex("tools"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  await page.goto(`${host.origin}/tools`);
  await page.waitForSelector(READY);
  await page.click('#tools-nav .nav-item[data-tool="wallet-cleanup"]');

  const trigger = page.locator(".tool-title-row #tool-hint .hint-trigger");
  await trigger.waitFor();
  assert.equal(await trigger.locator("i.icon-circle-question-mark").count(), 1);
  await trigger.click();
  await page.locator(".hint-popover").waitFor();
});
