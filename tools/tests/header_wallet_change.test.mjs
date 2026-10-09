// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the header Worth card's day change takes its arrow and tone from the value as
 * shown. A change that rounds to "0.0%" carries no arrow and a neutral tone, as on the
 * Home hero; a "↓0.0%" pointed at a change the card does not show.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const METRICS = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/header_metrics.json`, "utf8"));
const CASES = [
  { percent: -0.04, text: "0.0%", tone: "neutral" },
  { percent: 0.049, text: "0.0%", tone: "neutral" },
  { percent: -0.06, text: "↓0.1%", tone: "negative" },
  { percent: 2.35, text: "↑2.4%", tone: "positive" },
];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("home"), await loadIndex("shell")];

for (const { percent, text, tone } of CASES) {
  test(`a day change of ${percent}% reads "${text}" with a ${tone} tone`, async () => {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler(indexes, "populated");
    const onApi = (request) =>
      request.url.pathname === "/api/header/metrics"
        ? {
            body: JSON.stringify({
              ...METRICS,
              wallet: { ...METRICS.wallet, change_today_percent: percent },
            }),
          }
        : api.onApi(request);
    const host = await serveDashboard(context, { locale: "en", onApi });
    try {
      const page = await context.newPage();
      await page.goto(`${host.origin}/home`);
      const change = page.locator("#walletChange");
      await page.waitForFunction(
        () => document.querySelector("#walletChange")?.textContent !== "—"
      );
      assert.equal(await change.textContent(), text);
      const classes = await change.evaluate((el) => [...el.classList]);
      assert.ok(classes.includes(tone), `classes ${classes.join(" ")}`);
    } finally {
      await context.close();
      await host.close();
    }
  });
}
