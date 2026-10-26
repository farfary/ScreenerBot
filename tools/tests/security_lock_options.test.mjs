// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Security lock options act only while the lock screen is on.
 *
 * Auto-lock, Lock on blur and Lock Now do nothing while the lock screen is off, so
 * they stay disabled then, each hint saying to turn the lock screen on. Turning it on
 * enables them in place and restores their own hints; a refused save leaves the switch
 * and the options as they were.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("home"), await loadIndex("shell"), await loadIndex("config")];
const NEEDS_LOCKSCREEN = "Turn on the lock screen first to use this";

/** Opens Settings > Security with the lock screen off and saves answering `saveStatus`. */
async function openSecurity(saveStatus) {
  const context = await browser.newContext({ viewport: { width: 1400, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const onApi = async (request) => {
    if (request.url.pathname === "/api/lockscreen/status") {
      return {
        body: JSON.stringify({
          has_password: true,
          enabled: false,
          password_type: "pin6",
          auto_lock_timeout_secs: 300,
          lock_on_blur: false,
          totp_enabled: false,
        }),
      };
    }
    if (request.url.pathname === "/api/lockscreen/settings") {
      return { status: saveStatus, body: JSON.stringify({}) };
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
  await page.locator('.settings-nav-item[data-tab="security"]').click();
  await page.waitForSelector("#securityEnableLockscreen", { state: "attached" });
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

function lockOptions(page) {
  return page.evaluate(() => {
    const text = (id) => document.querySelector(id).textContent.trim();
    return {
      autoLock: document.querySelector("#securityAutoLockTimeout").disabled,
      blur: document.querySelector("#securityLockOnBlur").disabled,
      lockNow: document.querySelector("#securityLockNowBtn").disabled,
      hints: [
        text("#securityAutoLockHint"),
        text("#securityLockBlurHint"),
        text("#securityLockNowHint"),
      ],
    };
  });
}

const toggleLockscreen = (page, on) =>
  page.locator("#securityEnableLockscreen").evaluate((toggle, checked) => {
    toggle.checked = checked;
    toggle.dispatchEvent(new Event("change", { bubbles: true }));
  }, on);

test("the lock options wait for the lock screen and follow its switch", async () => {
  const { page, close } = await openSecurity(200);
  try {
    assert.deepEqual(await lockOptions(page), {
      autoLock: true,
      blur: true,
      lockNow: true,
      hints: [NEEDS_LOCKSCREEN, NEEDS_LOCKSCREEN, NEEDS_LOCKSCREEN],
    });

    await toggleLockscreen(page, true);
    await page.waitForFunction(() => !document.querySelector("#securityLockOnBlur").disabled);
    const enabled = await lockOptions(page);
    assert.deepEqual([enabled.autoLock, enabled.blur, enabled.lockNow], [false, false, false]);
    for (const hint of enabled.hints) assert.notEqual(hint, NEEDS_LOCKSCREEN);
  } finally {
    await close();
  }
});

test("a refused switch leaves the lock options as they were", async () => {
  const { page, close } = await openSecurity(500);
  try {
    await toggleLockscreen(page, true);
    await page.waitForFunction(() => !document.querySelector("#securityEnableLockscreen").checked);
    const options = await lockOptions(page);
    assert.deepEqual([options.autoLock, options.blur, options.lockNow], [true, true, true]);
  } finally {
    await close();
  }
});
