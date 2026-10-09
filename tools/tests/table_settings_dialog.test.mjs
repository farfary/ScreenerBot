// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the DataTable settings dialog lines its content up on one start edge and
 * closes with the shared modal close control. The column filter once sat inside a
 * second gutter, inset from the Visibility and Ordering cards and the column list
 * around it, and the dialog drew its own undersized close glyph.
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
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

for (const locale of ["en", "fa"]) {
  test(`the table settings dialog shares one start edge and the modal close (${locale})`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler([tokens, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    await page.goto(`${host.origin}/tokens`);
    await page.waitForSelector(READY);
    await page.click("#tokens-root .dt-btn-columns");
    await page.waitForSelector(".table-settings-dialog .table-settings-column-item");

    const layout = await page.evaluate(() => {
      const dialog = document.querySelector(".table-settings-dialog");
      const rtl = getComputedStyle(dialog).direction === "rtl";
      const start = (selector) => {
        const rect = dialog.querySelector(selector).getBoundingClientRect();
        return Math.round(rtl ? rect.right : rect.left);
      };
      return {
        controls: start(".table-settings-controls"),
        search: start(".table-settings-search"),
        list: start(".table-settings-column-list"),
        close: dialog.querySelector('[data-action="close"]').className,
      };
    });

    assert.equal(layout.search, layout.controls, "the column filter starts on the cards' edge");
    assert.equal(layout.list, layout.controls, "the column list starts on the cards' edge");
    assert.equal(layout.close, "modal-close");
  });
}
