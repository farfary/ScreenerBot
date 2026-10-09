// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a panel whose content cannot load shows the one shared state view
 * (`scripts/ui/state_view.js`). Settings once rendered a failed load three ways - an
 * italic grey line in the Data card, a red tinted box in Security, Telegram and Agent
 * Connections, and red text in Account - while Token Details tabs had their own
 * state block. No dashboard source spells a private state block any more, and a
 * failed Settings load renders the shared error state. The state view sets its glyph
 * inline with its heading; a file that still builds an empty state of its own, glyph
 * first, is listed and the list only shrinks.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { loadMarkupSources, loadStylesheets } from "../lib/dashboard_ui.mjs";
import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const PRIVATE_STATES =
  /(?<![\w-])(?:tdd-state[\w-]*|settings-error|renderTabState|dt-empty[\w-]*|notification-empty)(?![\w-])/;

/**
 * An element whose class names an empty state with a glyph as its first child: the
 * shape that parks the glyph alone on a row above the heading. Only the shared state
 * view places its glyph, inline with the heading.
 */
const GLYPH_FIRST_EMPTY_STATE =
  /class="([^"]*empty[^"]*)"[^>]*>\s*(?:<[a-z]+[^>]*>\s*)?<(?:i|span) class="[^"]*icon/g;

/**
 * Files that still build an empty state of their own instead of the shared state view.
 * This list only shrinks: each file leaves it once its states render through
 * `renderStateView`.
 */
const PRIVATE_EMPTY_STATE_FILES = [
  "src/webserver/templates/pages/assistant.html",
  "src/webserver/templates/pages/strategies.html",
  "src/webserver/templates/scripts/core/chat_widget.js",
  "src/webserver/templates/scripts/pages/assistant/automation_tab.js",
  "src/webserver/templates/scripts/pages/assistant/instructions_tab.js",
  "src/webserver/templates/scripts/pages/strategies.js",
  "src/webserver/templates/scripts/pages/strategies/condition_editor.js",
  "src/webserver/templates/scripts/pages/tools/multi_wallet_tools.js",
  "src/webserver/templates/scripts/pages/tools/token_tools.js",
  "src/webserver/templates/scripts/pages/tools/trading_tools.js",
  "src/webserver/templates/scripts/pages/tools/wallet_tools.js",
  "src/webserver/templates/scripts/pages/trader.js",
  "src/webserver/templates/scripts/ui/featured_dialog.js",
  "src/webserver/templates/scripts/ui/position_details/chart.js",
  "src/webserver/templates/scripts/ui/position_details_dialog.js",
  "src/webserver/templates/scripts/ui/setup_gate.js",
  "src/webserver/templates/scripts/ui/token_details/pools_links_tab.js",
  "src/webserver/templates/scripts/ui/token_details/security_tab.js",
  "src/webserver/templates/scripts/ui/transaction_details_dialog.js",
];

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

test("an empty state with a glyph is the shared state view, outside the shrinking list", async () => {
  const found = new Set();
  for (const { path, source } of await loadMarkupSources()) {
    for (const [, classes] of source.matchAll(GLYPH_FIRST_EMPTY_STATE)) {
      if (!/(?<![\w-])state-view(?![\w-])/.test(classes)) found.add(path);
    }
  }
  const unlisted = [...found].filter((path) => !PRIVATE_EMPTY_STATE_FILES.includes(path));
  const cleared = PRIVATE_EMPTY_STATE_FILES.filter((path) => !found.has(path));
  assert.deepEqual(unlisted, [], "a new private empty state: render it with renderStateView");
  assert.deepEqual(cleared, [], "these files no longer need their entry: remove it");
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
  const [icon, title] = await Promise.all([
    failed.locator(".state-view-icon").boundingBox(),
    failed.locator(".state-view-title").boundingBox(),
  ]);
  const centre = icon.y + icon.height / 2;
  assert.ok(
    centre >= title.y && centre <= title.y + title.height,
    "the glyph sits on the title's row, not above it"
  );
});
