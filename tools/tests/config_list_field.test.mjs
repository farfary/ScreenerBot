// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Config list field (one value per line, such as Entry Sizes) shows every
 * line. The textarea had a fixed 84px height, so a four-entry list cut its last line
 * at the box edge; it now grows with its content.
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

test("every Config list field shows all of its lines", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("config"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/config`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector(".config-field-control textarea");
  const fields = await page.$$eval(".config-field-control textarea", (areas) =>
    areas.map((area) => ({
      id: area.id,
      lines: area.value.split("\n").length,
      hidden: area.scrollHeight - area.clientHeight,
    }))
  );
  assert.ok(
    fields.some((field) => field.lines >= 4),
    "the fixture holds a list of four or more lines"
  );
  for (const field of fields) {
    assert.ok(field.hidden <= 0, `${field.id}: ${field.hidden}px of its lines are cut`);
  }
});
