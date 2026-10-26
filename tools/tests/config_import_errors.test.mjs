// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a config import the server refuses names the reason and its cause.
 *
 * The import preview answers a validation failure with the localized reason and the
 * technical cause in `details` (which field, which limit). The dialog's notice shows
 * both, never the transport status of the request.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const CAUSE = "trader.max_open_positions: must be at least 1";
const REFUSAL = JSON.stringify({
  error: {
    code: "INVALID_INPUT",
    message: "Config validation failed. No changes were applied.",
    text: { id: "errors-config-import-validation-failed" },
    details: CAUSE,
  },
});

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

test("a refused import preview shows the reason and the validation cause", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("config"), await loadIndex("shell")], "populated");
  const onApi = async (request) =>
    request.url.pathname === "/api/config/import/preview"
      ? { status: 400, body: REFUSAL }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/config`);
    await page.waitForSelector(PAGE_READY);
    await page.evaluate(async () => {
      const { ConfigImportDialog } = await import("/scripts/ui/config_import_export_dialog.js");
      void ConfigImportDialog.show();
    });
    await page.setInputFiles("#importFileInput", {
      name: "config.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify({ trader: { max_open_positions: 0 } })),
    });

    const toast = page.locator('.toast[data-type="error"]').last();
    await toast.waitFor();
    const title = (await toast.locator(".toast__title").textContent()).trim();
    const message = (await toast.locator(".toast__message").textContent()).trim();
    assert.equal(title, "Config validation failed. No changes were applied.");
    assert.equal(message, CAUSE);
    assert.doesNotMatch(`${title} ${message}`, /HTTP \d{3}/);
  } finally {
    await context.close();
    await host.close();
  }
});
