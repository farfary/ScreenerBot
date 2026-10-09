// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading summary figure is one line. A value with its unit
 * ("2.54 / 8.00 SOL") never wraps the unit under the amount and never runs out of its
 * card, at the narrowest desktop window and wider, in left-to-right and right-to-left
 * locales. The task list's budget line follows the same rule.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const VIEWS = [
  [1200, "en"],
  [1440, "en"],
  [1200, "de"],
  [1200, "fa"],
];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [copy, shell] = await Promise.all([loadIndex("copy"), loadIndex("shell")]);

for (const [width, locale] of VIEWS) {
  test(`copy figures stay on one line at ${width}px (${locale})`, async (t) => {
    const context = await browser.newContext({ viewport: { width, height: 900 }, locale });
    const api = createApiHandler([copy, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    await page.goto(`${host.origin}/copy`);
    await page.waitForSelector("#copy-figures .copy-figure");
    const values = await page.$$eval(
      "#copy-figures .copy-figure-value, .copy-row-budget > span:last-child",
      (nodes) =>
        nodes.map((node) => {
          const style = getComputedStyle(node);
          const line = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.6;
          return {
            text: node.textContent,
            height: node.getBoundingClientRect().height,
            line,
            overflow: node.scrollWidth - node.clientWidth,
          };
        })
    );
    assert.ok(
      values.some((value) => value.text.includes("/")),
      "the budget figure is covered"
    );
    for (const value of values) {
      assert.ok(
        value.height <= value.line * 1.5,
        `"${value.text}" wraps: ${JSON.stringify(value)}`
      );
      assert.ok(value.overflow <= 0, `"${value.text}" runs out of its card`);
    }
  });
}
