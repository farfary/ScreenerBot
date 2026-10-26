// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Telegram settings act only on a connection that can work, and say when a
 * change waits for a restart.
 *
 * Discover Chat ID needs a bot token; Send Test also needs a chat and Telegram on. Each
 * stays disabled until then, with the missing piece named in its hint and tooltip. A
 * save the running service cannot take (the server answers `restart_required`) shows
 * the Restart row; a save that started the service does not.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const SETTINGS_PATH = "/api/telegram/settings";
const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("home"), await loadIndex("shell"), await loadIndex("config")];

/** Opens Settings > Telegram with `settings` saved and `service` answered to every save. */
async function openTelegram(settings, service) {
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const onApi = async (request) => {
    if (request.url.pathname === SETTINGS_PATH) {
      return request.method === "POST"
        ? { body: JSON.stringify({ service }) }
        : { body: JSON.stringify(settings) };
    }
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(PAGE_READY);
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");
  await page.locator('.settings-nav-item[data-tab="telegram"]').click();
  await page.waitForSelector("#tgEnabled", { state: "attached" });
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

const action = (page, id) =>
  page.$eval(id, (button) => ({
    disabled: button.disabled,
    title: button.title.replace(/[\u2068\u2069]/g, ""),
  }));

test("the connection actions wait for a token and name what is missing", async () => {
  const { page, close } = await openTelegram(
    { enabled: false, bot_token: "", chat_id: "" },
    "restart_required"
  );
  try {
    assert.deepEqual(await action(page, "#tgDiscoverBtn"), {
      disabled: true,
      title: "Add a bot token first",
    });
    assert.deepEqual(await action(page, "#tgTestBtn"), {
      disabled: true,
      title: "Add a bot token first",
    });
    assert.equal((await page.textContent("#tgDiscoverHint")).trim(), "Add a bot token first");
    assert.equal(await page.isVisible("#tgRestartField"), false);

    await page.fill("#tgBotToken", "123456:abcdef");
    await page.dispatchEvent("#tgBotToken", "change");
    await page.waitForFunction(() => !document.querySelector("#tgDiscoverBtn").disabled);
    assert.deepEqual(await action(page, "#tgDiscoverBtn"), { disabled: false, title: "" });
    assert.deepEqual(await action(page, "#tgTestBtn"), {
      disabled: true,
      title: "Connect a chat first",
    });
    assert.equal(await page.isVisible("#tgRestartField"), true);
  } finally {
    await close();
  }
});

test("Send Test waits for Telegram to be on, and a live start needs no restart", async () => {
  const { page, close } = await openTelegram(
    { enabled: false, bot_token: "1234567890...", chat_id: "42" },
    "started"
  );
  try {
    assert.deepEqual(await action(page, "#tgTestBtn"), {
      disabled: true,
      title: "Turn on Telegram first",
    });
    await page.locator("#tgEnabled").evaluate((toggle) => {
      toggle.checked = true;
      toggle.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await page.waitForFunction(() => !document.querySelector("#tgTestBtn").disabled);
    assert.equal(
      (await page.textContent("#tgTestHint")).trim(),
      "Send a test message to verify configuration"
    );
    assert.equal(await page.isVisible("#tgRestartField"), false);
  } finally {
    await close();
  }
});
