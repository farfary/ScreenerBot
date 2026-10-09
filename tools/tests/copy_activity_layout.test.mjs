// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading activity headline sits on the line of the token it names. The
 * token chip is two lines (symbol, then the mint), so a headline centred on the chip
 * floated between them; it shares the symbol line's baseline instead, in a
 * left-to-right and a right-to-left locale.
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
const [copy, shell] = await Promise.all([loadIndex("copy"), loadIndex("shell")]);

for (const locale of ["en", "fa"]) {
  test(`an activity headline shares the token symbol's baseline (${locale})`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler([copy, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    await page.goto(`${host.origin}/copy`);
    await page.waitForSelector(READY);
    await page.click("#copy-list-rows .copy-row");
    await page.click("#copy-tab-activity");
    await page.waitForSelector(".copy-event .copy-token-link");
    const offsets = await page.$$eval(
      ".copy-event > .copy-event-main > .copy-event-line",
      (lines) => {
        // An empty inline-block's top edge is the baseline of the line it sits on.
        const baseline = (element) => {
          const marker = document.createElement("span");
          marker.style.cssText = "display:inline-block;width:0;height:0";
          element.append(marker);
          const top = marker.getBoundingClientRect().top;
          marker.remove();
          return top;
        };
        return lines
          .filter((line) => line.querySelector(".ti-chip-symbol"))
          .map((line) => ({
            text: line.querySelector("strong").textContent.trim(),
            offset:
              baseline(line.querySelector("strong")) -
              baseline(line.querySelector(".ti-chip-symbol")),
          }));
      }
    );
    assert.ok(offsets.length > 0, "activity rows name a token");
    for (const { text, offset } of offsets) {
      assert.ok(Math.abs(offset) <= 1, `"${text}" sits ${offset}px off the symbol line`);
    }
  });
}
