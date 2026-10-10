// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading switch sits on its title line.
 *
 * The switch rows centred the toggle against the title, hint and warning together,
 * so in Settings the Require Filter Pass switch floated beside the warning, two
 * lines below the setting it controls.
 *
 * Every `.copy-switch-row` in the Settings dialog shares its top edge with its
 * title, whatever text runs under the title.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

test("Settings switches align with their titles", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.locator("#copy-settings-open").click();
  await page.waitForSelector("#copy-settings-filter:not(:disabled)", { state: "attached" });

  const rows = await page.$$eval("#copy-settings .copy-switch-row", (nodes) =>
    nodes.map((row) => ({
      title: row.querySelector("strong").textContent,
      titleTop: row.querySelector("strong").getBoundingClientRect().top,
      toggleTop: row.querySelector(".toggle").getBoundingClientRect().top,
      height: row.getBoundingClientRect().height,
    }))
  );
  assert.ok(rows.length >= 2);
  assert.ok(
    rows.some((row) => row.height > 60),
    "a row with a hint and warning is covered"
  );
  for (const row of rows) {
    assert.ok(Math.abs(row.toggleTop - row.titleTop) <= 1, `${row.title}: ${JSON.stringify(row)}`);
  }
});
