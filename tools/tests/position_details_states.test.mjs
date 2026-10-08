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
