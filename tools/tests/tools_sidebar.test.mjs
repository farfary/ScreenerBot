// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Tools sidebar states each fact once and lays nothing over anything.
 *
 * - The sidebar title glyph is bare: no tile behind it.
 * - A tool's status badge ("Coming soon") sits in the row beside the status dot, inside
 *   its item, never laid over the dot or the item edge.
 * - The "select a tool" hint leaves with its footer once a tool is selected.
 *
 * Checked at the narrowest desktop width in a left-to-right and a right-to-left locale.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tools, shell] = await Promise.all([loadIndex("tools"), loadIndex("shell")]);

for (const locale of ["en", "fa"]) {
  test(`the Tools sidebar keeps badges, dots and hint apart (${locale})`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1200, height: 900 } });
    const api = createApiHandler([tools, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    await page.goto(`${host.origin}/tools`);
    await page.waitForSelector(READY);
    await page.click('#tools-nav .nav-item[data-tool="wallet-cleanup"]');
    await page.waitForSelector("#tools-nav .status-badge");

    const report = await page.evaluate(() => {
      const box = (element) => element.getBoundingClientRect();
      const brand = getComputedStyle(document.querySelector(".sidebar-brand i"));
      const badges = [...document.querySelectorAll("#tools-nav .nav-item .status-badge")].map(
        (badge) => {
          const item = badge.closest(".nav-item");
          const [b, d, i] = [box(badge), box(item.querySelector(".nav-item-status")), box(item)];
          return {
            tool: item.dataset.tool,
            overDot: b.left < d.right && b.right > d.left && b.top < d.bottom && b.bottom > d.top,
            inside: b.left >= i.left - 0.5 && b.right <= i.right + 0.5,
          };
        }
      );
      return {
        brandPaint: [brand.backgroundColor, brand.borderStyle],
        badges,
        footer: getComputedStyle(document.querySelector(".sidebar-footer")).display,
      };
    });

    assert.deepEqual(report.brandPaint, ["rgba(0, 0, 0, 0)", "none"], "the title glyph is bare");
    assert.ok(report.badges.length > 0, "the fixture shows status badges");
    for (const badge of report.badges) {
      assert.ok(!badge.overDot, `${badge.tool}: the badge does not cover the status dot`);
      assert.ok(badge.inside, `${badge.tool}: the badge stays inside its item`);
    }
    assert.equal(report.footer, "none", "the hint leaves once a tool is selected");
  });
}
