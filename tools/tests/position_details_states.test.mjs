// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the position details dialog recovers from a failed load, and its header
 * favourite is a bare glyph.
 *
 * A failed details request left a centred "Failed to load position details" with
 * no way forward but closing the dialog. The active favourite star sat on an
 * amber-tinted, amber-bordered tile.
 *
 * - The load failure is the shared empty state with a Retry action, and Retry
 *   replaces it with the position once the request succeeds.
 * - The shared header cluster marks an active favourite by the glyph colour alone:
 *   no background, border, shadow or filter of its own.
 * - At the narrowest desktop window, where the trade actions lose their labels, each
 *   action is a bare glyph at rest and on hover.
 * - The chart's hover card stays inside a chart area shorter than its natural height,
 *   and the candles drawn are those of the selected timeframe.
 * - Every value the dialog labels has a label (no raw id reaches the screen), and every
 *   time on screen is to the minute.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";
import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const ROW_SYMBOL = "#positions-root tr[data-row-id] .ti-row-cell__symbol";
const BODY_STATE = ".position-details-dialog #pddBodyState";

test("a failed details load offers Retry and recovers", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(
    [await loadIndex("positions"), await loadIndex("shell")],
    "populated"
  );
  let failDetails = true;
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (failDetails && /^\/api\/positions\/[^/]+\/details$/.test(request.url.pathname)) {
        return {
          status: 500,
          body: JSON.stringify({ error: { code: "INTERNAL", message: "Internal error" } }),
        };
      }
      return api.onApi(request);
    },
  });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator(ROW_SYMBOL).first().click();

  const state = page.locator(`${BODY_STATE}:not([hidden]) .empty-state`);
  await state.waitFor();
  assert.equal(
    (await state.locator(".empty-state-title").textContent()).trim(),
    "Failed to load position details"
  );
  const retry = state.locator("button.empty-state-action");
  assert.equal((await retry.textContent()).trim(), "Retry");

  failDetails = false;
  await retry.click();
  await page.waitForSelector(BODY_STATE, { state: "hidden" });
  assert.ok(await page.locator(".position-details-dialog .header-metric").count());
});

test("an active favourite is a bare glyph", () => {
  const rules = rulesIn(readFileSync(`${STYLES_ROOT}/ui/dialog_header_actions.css`, "utf8")).filter(
    ({ selector }) => /\.favorite-btn\.active\b/.test(selector)
  );
  assert.ok(rules.length > 0);
  for (const { selector, body } of rules) {
    assert.doesNotMatch(
      body,
      /(?:^|;)\s*(?:background|border|box-shadow|filter|outline)\b/,
      selector
    );
  }
});

test("the open position dialog at the narrowest desktop window", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1200, height: 700 } });
  const api = createApiHandler(
    [await loadIndex("positions"), await loadIndex("shell")],
    "populated"
  );
  const candleTimeframes = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (/\/ohlcv$/.test(request.url.pathname)) {
        candleTimeframes.push(request.url.searchParams.get("timeframe"));
      }
      return api.onApi(request);
    },
  });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  const missingLabels = [];
  page.on("console", (message) => {
    if (message.text().includes("[I18n] No label for value")) missingLabels.push(message.text());
  });
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator(ROW_SYMBOL).first().click();
  const dialog = page.locator(".position-details-dialog");
  await dialog.locator(".pdd-act-milestone").first().waitFor();
  const area = dialog.locator(".advanced-chart-area");
  await page.waitForFunction(
    () =>
      !document.querySelector(".position-details-dialog #pddChartSection.is-empty") &&
      document.querySelector(".position-details-dialog .advanced-chart-area canvas")
  );

  // Trade actions without labels are bare glyphs, at rest and on hover.
  const actions = dialog.locator(".pdd-trade-btn");
  assert.ok((await actions.count()) > 0, "the open position shows its trade actions");
  for (let index = 0; index < (await actions.count()); index += 1) {
    const action = actions.nth(index);
    for (const phase of ["rest", "hover"]) {
      if (phase === "hover") await action.hover();
      const paint = await action.evaluate((element) => {
        const style = getComputedStyle(element);
        return {
          label: getComputedStyle(element.querySelector("span")).display,
          border: style.borderTopColor,
          background: style.backgroundColor,
        };
      });
      const name = `${await action.getAttribute("data-trade-action")} at ${phase}`;
      assert.equal(paint.label, "none", `${name}: the label is hidden at 1200px`);
      assert.equal(paint.border, "rgba(0, 0, 0, 0)", `${name}: no border`);
      assert.equal(paint.background, "rgba(0, 0, 0, 0)", `${name}: no background`);
    }
  }

  // The hover card stays inside the chart area and names the selected timeframe.
  const box = await area.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height * 0.8);
  const tooltip = dialog.locator(".advanced-chart-tooltip.visible");
  await tooltip.waitFor();
  const card = await tooltip.boundingBox();
  assert.ok(
    card.y >= box.y && card.y + card.height <= box.y + box.height + 0.5,
    `the hover card (${card.y}-${card.y + card.height}) stays inside the chart area (${box.y}-${box.y + box.height})`
  );
  const selected = await dialog
    .locator("#pddTimeframes .timeframe-btn.active")
    .getAttribute("data-tf");
  assert.equal(
    (await tooltip.locator(".tooltip-interval").textContent()).trim().toLowerCase(),
    selected,
    "the candles drawn are the selected timeframe's"
  );
  assert.deepEqual(
    [...new Set(candleTimeframes)],
    [selected],
    "candles are read for the selected timeframe only"
  );

  // No raw id and no seconds on screen.
  assert.deepEqual(missingLabels, []);
  assert.equal(
    (await dialog.locator(".pdd-act-milestone strong").first().textContent()).trim(),
    "Position open"
  );
  const text = await dialog.evaluate((element) => element.innerText);
  assert.doesNotMatch(text, /\b\d{1,2}:\d{2}:\d{2}\b/, "times on screen are to the minute");
});
