// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: Settings > Wallet controls the trading wallet through the owners Wallets uses.
 *
 * - With a wallet: the section shows the trading wallet's full address with copy and its
 *   balance; Reveal private key opens the shared wallet key dialog above Settings, which
 *   decrypts the key only on its own reveal, and Escape closes that dialog alone.
 * - Replace wallet opens the Replace wallet dialog above Settings.
 * - In Explore Mode the section names what is missing, and Complete setup closes
 *   Settings for the full-screen first-run setup.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const fixture = (path) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/${path}`, "utf8"));
const WALLETS = fixture("wallets/wallets_list.json");
const main = WALLETS.wallets.find((wallet) => wallet.role === "main");
const EXPLORE_BOOTSTRAP = JSON.stringify({
  ...fixture("shell/system_bootstrap.json"),
  initialization_complete: false,
  explore_mode: true,
});
// A placeholder, not a key: the dialog only has to show what the route returns.
const EXPORTED = "TestOnlyExportedKey1111111111111111111111111111";
const plain = (text) => text.replace(/[⁨⁩]/g, "").trim();

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

/** Opens Settings > Wallet on Home; `bootstrap` replaces the shell's bootstrap. */
async function openWalletTab({ bootstrap } = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const requests = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const path = request.url.pathname;
      requests.push(`${request.method} ${path}`);
      if (bootstrap && path === "/api/system/bootstrap") return { body: bootstrap };
      if (path === "/api/wallets") return { body: JSON.stringify(WALLETS) };
      if (path === `/api/wallets/${main.id}/export`) {
        return { body: JSON.stringify({ private_key: EXPORTED }) };
      }
      return api.onApi(request);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(PAGE_READY);
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.locator('.settings-nav-item[data-tab="wallet"]').click();
  await page.waitForSelector('.settings-tab.active[data-tab-content="wallet"] .settings-section');
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, requests, close };
}

/** True when the topmost element at the centre of `selector` belongs to it. */
const onTop = (page, selector) =>
  page.evaluate((target) => {
    const node = document.querySelector(target);
    const box = node.getBoundingClientRect();
    return node.contains(document.elementFromPoint(box.x + box.width / 2, box.y + box.height / 2));
  }, selector);

test("Settings > Wallet shows the trading wallet and reveals its key above Settings", async () => {
  const { page, requests, close } = await openWalletTab();
  try {
    const tab = page.locator('.settings-tab[data-tab-content="wallet"]');
    await tab.locator(`[data-copy="${main.address}"]`).waitFor();
    assert.equal(
      plain(await tab.locator(".settings-wallet .ti-address-value").textContent()),
      main.address
    );
    assert.match(plain(await tab.locator("#settingsWalletBalance").textContent()), /^4\.2187 SOL$/);

    const reveal = tab.locator("#settingsWalletReveal");
    await page.waitForFunction(() => !document.querySelector("#settingsWalletReveal").disabled);
    await reveal.click();
    const dialog = page.locator(".wallet-key-dialog .modal-dialog");
    await dialog.waitFor();
    assert.equal(await onTop(page, ".wallet-key-dialog .modal-dialog"), true, "above Settings");
    assert.equal(
      requests.some((line) => line.endsWith("/export")),
      false,
      "nothing is decrypted before the dialog's own reveal"
    );

    await dialog.locator('[data-action="reveal"]').click();
    await dialog.locator(".wallet-key-value code", { hasText: EXPORTED }).waitFor();
    assert.ok(requests.includes(`POST /api/wallets/${main.id}/export`));

    await page.keyboard.press("Escape");
    await page.locator(".wallet-key-dialog").waitFor({ state: "detached" });
    assert.equal(await page.locator(".settings-dialog.active").count(), 1, "Settings stays open");

    await tab.locator("#settingsWalletReplace").click();
    await page.locator(".setup-dialog").waitFor();
    assert.equal(await onTop(page, ".setup-dialog"), true, "Replace wallet above Settings");
  } finally {
    await close();
  }
});

test("Settings > Wallet in Explore Mode leads to the full-screen setup", async () => {
  const { page, close } = await openWalletTab({ bootstrap: EXPLORE_BOOTSTRAP });
  try {
    const tab = page.locator('.settings-tab[data-tab-content="wallet"]');
    assert.match(
      plain(await tab.locator(".settings-section-description").textContent()),
      /Explore Mode/
    );
    assert.equal(await tab.locator(".settings-wallet").count(), 0, "no wallet to show");

    await tab.locator("#settingsWalletSetup").click();
    await page.locator("body.initialization-mode #setupScreen").waitFor();
    await page.locator(".settings-dialog").waitFor({ state: "detached" });
  } finally {
    await close();
  }
});
