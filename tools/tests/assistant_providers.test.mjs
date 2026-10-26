// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Assistant reads its provider set-up the same way everywhere.
 *
 * A provider is set up when it is enabled with an API key and a model. The chat answers
 * through the default provider, so without a set-up default it shows a call to connect
 * one that opens Providers, and keeps the input closed. The Providers list shows the
 * default only on a set-up provider, offers the default choice only there, and keeps
 * every Configure action secondary.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const WAIT_MS = 15000;
const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("assistant"), await loadIndex("shell")];
const providers = JSON.parse(
  await readFile(join(FIXTURES_ROOT, "assistant/llm_providers.json"), "utf8")
);

/** Opens the Assistant at `hash` with the providers list answering `defaultProvider`. */
async function openAssistant(hash, defaultProvider) {
  const context = await browser.newContext({ viewport: { width: 1200, height: 780 } });
  const api = createApiHandler(indexes, "populated");
  const onApi = async (request) => {
    if (request.url.pathname === "/api/llm/providers") {
      return { body: JSON.stringify({ ...providers, default_provider: defaultProvider }) };
    }
    if (request.url.pathname === "/api/llm/config") {
      return { body: JSON.stringify({ enabled: true, default_provider: defaultProvider }) };
    }
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(WAIT_MS);
  await page.goto(`${host.origin}/assistant#${hash}`);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

test("without a set-up default provider the chat points to Providers", async () => {
  const { page, close } = await openAssistant("chat", "anthropic");
  try {
    await page.click("#chat-panel .cw-new-session-btn");
    await page.waitForSelector("#chat-panel .cw-welcome-provider:not([hidden])");
    assert.equal(await page.isHidden("#chat-panel .cw-welcome-ready"), true);
    assert.equal(await page.isDisabled("#chat-panel .cw-chat-input"), true);
    assert.equal(await page.isDisabled("#chat-panel .cw-send-btn"), true);
    assert.match(await page.textContent("#chat-panel .cw-input-status"), /Set one up in Providers/);

    await page.click("#chat-panel .cw-open-providers");
    await page.waitForSelector('#subTabsContainer [data-tab-id="providers"].active');
    await page.waitForSelector("#providers-list .provider-item");
  } finally {
    await close();
  }
});

test("with a set-up default provider the chat offers its prompts", async () => {
  const { page, close } = await openAssistant("chat", "openai");
  try {
    await page.click("#chat-panel .cw-new-session-btn");
    await page.waitForSelector("#chat-panel .cw-welcome-ready:not([hidden])");
    assert.equal(await page.isHidden("#chat-panel .cw-welcome-provider"), true);
    assert.equal(await page.isEnabled("#chat-panel .cw-chat-input"), true);
  } finally {
    await close();
  }
});

test("the Providers list marks the default only on a set-up provider", async () => {
  for (const [defaultProvider, expectedDefault] of [
    ["openai", "openai"],
    ["anthropic", null],
  ]) {
    const { page, close } = await openAssistant("providers", defaultProvider);
    try {
      await page.waitForSelector(
        '#providers-list .provider-item[data-provider="openai"] .status-badge.configured'
      );
      const rows = await page.$$eval("#providers-list .provider-item", (items) =>
        items.map((item) => ({
          id: item.dataset.provider,
          isDefault: item.classList.contains("is-default"),
          badge: Boolean(item.querySelector(".status-badge.default")),
          checked: item.querySelector(".provider-select input").checked,
          radioDisabled: item.querySelector(".provider-select input").disabled,
          status: item.querySelector(".provider-status .status-badge").textContent.trim(),
          model: item.querySelector(".provider-model").textContent.trim(),
          primaryAction: Boolean(item.querySelector(".provider-btn.primary, .btn-primary")),
        }))
      );
      const byId = Object.fromEntries(rows.map((row) => [row.id, row]));

      for (const row of rows) {
        const marked = row.id === expectedDefault;
        assert.equal(row.isDefault, marked, `${row.id} default row`);
        assert.equal(row.badge, marked, `${row.id} default badge`);
        assert.equal(row.checked, marked, `${row.id} default choice`);
        assert.equal(row.primaryAction, false, `${row.id} actions stay secondary`);
      }
      assert.deepEqual(
        [byId.openai.status, byId.openai.radioDisabled, byId.openai.model],
        ["Ready", false, "gpt-4o-mini"]
      );
      assert.deepEqual(
        [byId.anthropic.status, byId.anthropic.radioDisabled, byId.anthropic.model],
        ["Not set up", true, "—"]
      );
    } finally {
      await close();
    }
  }
});
