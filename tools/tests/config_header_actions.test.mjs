// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Config section header has one primary action, Save, and every
 * other action (Reload, Compare, Revert Section) shares the secondary style.
 * Revert discards only unsaved edits, so it is never a solid danger button
 * beside its neighbours.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

test("the Config header has one primary action and secondary siblings", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("config"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  await page.goto(`${host.origin}/config`);
  await page.waitForSelector(READY);
  await page.waitForSelector(".config-header-actions .btn");
  const variants = await page.$$eval(".config-header-actions .btn", (buttons) =>
    buttons.map((button) =>
      [...button.classList].filter((name) => name.startsWith("btn-") && name !== "btn-sm")
    )
  );
  assert.equal(variants.length, 4, "Save, Reload, Compare and Revert Section");
  assert.deepEqual(variants[0], ["btn-primary"]);
  for (const variant of variants.slice(1)) assert.deepEqual(variant, ["btn-secondary"]);
});
