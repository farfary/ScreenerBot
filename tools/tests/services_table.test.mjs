// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Services table shows an absent reading as absent and keeps one row height.
 *
 * - `/api/services/overview` sends `metrics: null` and `uptime_seconds: null` for a
 *   disabled or unsampled service. `pages/services.js` passes those nulls to the shared
 *   formatters (which render "—") and never substitutes a zero, which would read as a
 *   measured value.
 * - The dependency chips sit on one line, so a service with several dependencies is no
 *   taller than one with none; an empty list is "—", not a placeholder chip.
 * - Every rendered row has one height, and the activity reading shares the row's text
 *   line: the bar and its reading sit side by side, not stacked.
 * - A rate column names its unit in the header ("Cycles/sec", "Ops/sec"), since its
 *   cells are bare numbers.
 * - A disabled service is off by choice: its Enabled mark is neutral, never the error
 *   colour, which is reserved for failures.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { SCRIPTS_ROOT, STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

const SCRIPT = readFileSync(`${SCRIPTS_ROOT}/pages/services.js`, "utf8");
const STYLES = rulesIn(readFileSync(`${STYLES_ROOT}/pages/services.css`, "utf8"));

const declaration = (body, property) =>
  body.match(new RegExp(`(?:^|;)\\s*${property}\\s*:\\s*([^;]+)`))?.[1].trim();

test("service readings never default an absent value to zero", () => {
  const defaults = SCRIPT.split("\n")
    .map((line, index) => `${index + 1}: ${line.trim()}`)
    .filter((line) => /(?:\|\||\?\?)\s*0\b/.test(line));
  assert.deepEqual(defaults, []);
});

test("the dependency chips stay on one line", () => {
  assert.doesNotMatch(SCRIPT, /dependency-badge--none|services-dependencies-none/);
  const list = STYLES.find((rule) => rule.selector === ".dependency-list");
  assert.ok(list, "services.css declares .dependency-list");
  assert.equal(declaration(list.body, "white-space"), "nowrap");
  assert.equal(declaration(list.body, "flex-wrap"), "nowrap");
});

test("activity fills take their colour from theme tokens", () => {
  assert.doesNotMatch(SCRIPT, /#[0-9a-f]{3,8}\b/i);
  const fills = STYLES.filter((rule) => rule.selector.startsWith(".activity-fill--"));
  assert.ok(fills.length > 0, "services.css declares the activity tiers");
  for (const rule of fills) {
    assert.match(declaration(rule.body, "background") ?? "", /^var\(--/, rule.selector);
  }
});

test("a disabled service's Enabled mark is neutral", () => {
  const off = STYLES.find((rule) => rule.selector === ".services-enabled-icon.is-off");
  assert.ok(off, "services.css declares the disabled mark");
  assert.doesNotMatch(declaration(off.body, "color") ?? "", /error|danger/);
});

test("every Services row has one height and one text line", async () => {
  const browser = await chromium.launch({ headless: true });
  after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(
    [await loadIndex("services"), await loadIndex("shell")],
    "populated"
  );
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => api.onApi(request),
  });
  try {
    const page = await context.newPage();
    await page.goto(`${host.origin}/services`);
    await page.waitForSelector(".data-table tbody tr .activity-cell", { timeout: 15000 });
    const heights = await page.evaluate(() => [
      ...new Set(
        [...document.querySelectorAll(".data-table tbody tr")].map((row) =>
          Math.round(row.getBoundingClientRect().height)
        )
      ),
    ]);
    assert.equal(heights.length, 1, `row heights ${heights.join(", ")}`);

    // An empty inline-block's top edge is the baseline of the line it sits on.
    const offsets = await page.evaluate(() => {
      const baseline = (element) => {
        const marker = document.createElement("span");
        marker.style.cssText = "display:inline-block;width:0;height:0";
        element.append(marker);
        const top = marker.getBoundingClientRect().top;
        marker.remove();
        return top;
      };
      return [...document.querySelectorAll(".data-table tbody tr")]
        .filter((row) => row.querySelector(".activity-meta span"))
        .map(
          (row) =>
            baseline(row.querySelector(".activity-meta span")) -
            baseline(row.querySelector('td[data-column-id="uptime"]'))
        );
    });
    assert.ok(offsets.length > 0, "the fixture carries activity readings");
    for (const offset of offsets) {
      assert.ok(Math.abs(offset) <= 1, `the activity reading sits ${offset}px off the row line`);
    }

    const rates = await page.$$eval(
      '.data-table thead th:is([data-column-id="cycleRate"], [data-column-id="ops"])',
      (headers) => headers.map((header) => header.textContent.trim())
    );
    assert.equal(rates.length, 2, "both rate columns are shown");
    for (const label of rates) {
      assert.match(label, /\//, `"${label}" names its per-second unit`);
    }
  } finally {
    await context.close();
    await host.close();
  }
});
