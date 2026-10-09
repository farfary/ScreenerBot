// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a panel whose content cannot load shows the one shared state view
 * (`scripts/ui/state_view.js`). Settings once rendered a failed load three ways - an
 * italic grey line in the Data card, a red tinted box in Security, Telegram and Agent
 * Connections, and red text in Account - while Token Details tabs had their own
 * state block. No dashboard source spells a private state block any more, and a
 * failed Settings load renders the shared error state.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { loadMarkupSources, loadStylesheets } from "../lib/dashboard_ui.mjs";
import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const PRIVATE_STATES = /(?<![\w-])(?:tdd-state[\w-]*|settings-error|renderTabState)(?![\w-])/;

test("no dashboard source spells a private state block", async () => {
  const found = [];
  for (const { path, source } of await loadMarkupSources()) {
    source.split("\n").forEach((line, index) => {
      if (PRIVATE_STATES.test(line)) found.push(`${path}:${index + 1}`);
    });
  }
  for (const { path, rules } of await loadStylesheets()) {
    for (const { selector, line } of rules) {
      if (PRIVATE_STATES.test(selector)) found.push(`${path}:${line} ${selector}`);
    }
  }
  assert.deepEqual(found, []);
});

test("a failed Settings load renders the shared error state", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const onApi = (request) =>
    request.url.pathname === "/api/system/data-stats"
      ? { status: 500, body: JSON.stringify({ error: "unavailable" }) }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");

  await page.locator('.settings-nav-item[data-tab="data"]').click();
  const failed = page.locator("#dataOverviewCard .state-view-error");
  await failed.waitFor();
  assert.equal(await failed.getAttribute("role"), "alert");
  assert.equal(
    (await failed.locator(".state-view-title").textContent()).trim(),
    "Failed to load database statistics"
  );
  assert.equal(await failed.locator(".state-view-icon").count(), 1);
});
