// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: Settings > Connection names the active chain connection and switches it
 * only after a confirmation.
 *
 * - Both modes render as one radio group; the active one is checked and named
 *   "Active", and Apply stays disabled until another mode is chosen.
 * - On the ScreenerBot service, sending transactions through ScreenerBot is on and
 *   locked; on your own RPC the toggle is free.
 * - A paused service row names the pause and its detail (when trading resumes).
 * - Choosing the service while signed out keeps Apply disabled and offers the sign-in,
 *   which opens the Account tab.
 * - Choosing the other mode and applying opens a confirmation first; cancelling it
 *   sends no switch request.
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
const [home, shell] = await Promise.all([loadIndex("home"), loadIndex("shell")]);

const READY = { state: "ready", ready: true, text: { id: "connectivity-service-ready", args: {} } };

function accessStatus(mode, condition = READY) {
  return {
    chain: "solana",
    mode,
    ...condition,
    actionable: false,
    capabilities: {},
    account: { signed_in: true, chain_scope: true, submit_scope: true },
  };
}

/**
 * Opens Settings > Connection with the chain on `mode` in `condition` (ready by default);
 * `posts` collects switch requests.
 */
async function openConnection(
  t,
  mode,
  condition = READY,
  { signedIn = true, gateway = false } = {}
) {
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const posts = [];
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      if (request.url.pathname === "/api/chain/access") {
        if (request.method === "POST") posts.push(request);
        return { body: JSON.stringify(accessStatus(mode, condition)) };
      }
      if (request.url.pathname === "/api/account/status" && !signedIn) {
        return {
          body: JSON.stringify({ success: true, data: { signed_in: false, online: true } }),
        };
      }
      if (request.url.pathname === "/api/account/gateway" && request.method === "POST") {
        // The host passes no request body; the tests only ever turn the switch on.
        posts.push(request);
        const data = { signed_in: signedIn, use_gateway_rpc: true };
        return { body: JSON.stringify({ success: true, data }) };
      }
      if (request.url.pathname === "/api/config/account") {
        // Config GET routes wrap the section in { data, timestamp }.
        const section = { use_gateway_rpc: gateway };
        return { body: JSON.stringify({ data: section, timestamp: "2026-10-28T00:00:00Z" }) };
      }
      return fixtures.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(PAGE_READY);
  await page.evaluate(async () => {
    const { showSettingsDialog } = await import("/scripts/ui/settings_dialog.js");
    await showSettingsDialog({ tab: "connection" });
  });
  await page.waitForSelector('.connection-modes:not([aria-busy="true"])');
  return { page, posts };
}

const readTab = (page) =>
  page.evaluate(() => ({
    checked: document.querySelector(".connection-mode-input:checked")?.value ?? null,
    active: [...document.querySelectorAll(".connection-mode")]
      .filter((row) => !row.querySelector(".connection-mode-active").hidden)
      .map((row) => row.dataset.mode),
    apply: document.querySelector("#settingsConnectionApply").disabled,
    gateway: {
      checked: document.querySelector("#settingsUseGateway").checked,
      disabled: document.querySelector("#settingsUseGateway").disabled,
    },
  }));

test("the service mode is active and keeps sending through ScreenerBot on", async (t) => {
  const { page } = await openConnection(t, "service");
  assert.deepEqual(await readTab(page), {
    checked: "service",
    active: ["service"],
    apply: true,
    gateway: { checked: true, disabled: true },
  });
});

test("a paused service names the pause and when trading resumes", async (t) => {
  const { page } = await openConnection(t, "service", {
    state: "daily_limit",
    ready: false,
    text: { id: "connectivity-service-daily-limit", args: {} },
  });
  const line = page.locator('.connection-mode[data-mode="service"] .connection-mode-state');
  assert.match(await line.innerText(), /^Trading paused · Daily limit reached/);
  assert.match(await line.locator(".connection-mode-state-detail").innerText(), /00:00 UTC/);
});

test("choosing the service while signed out asks for a sign-in before Apply", async (t) => {
  const { page, posts } = await openConnection(t, "own_rpc", READY, { signedIn: false });
  await page.locator("#settings-connection-service").check();
  assert.equal(await page.locator("#settingsConnectionSignIn").isVisible(), true);
  assert.equal(await page.locator("#settingsConnectionApply").isDisabled(), true);
  await page.locator("#settingsConnectionSignInBtn").click();
  await page.locator('.settings-nav-item[data-tab="account"].active').waitFor();
  assert.equal(posts.length, 0);
});

test("your own RPC is active and the sending toggle is free and shows the saved choice", async (t) => {
  const { page } = await openConnection(t, "own_rpc", READY, { gateway: true });
  assert.deepEqual(await readTab(page), {
    checked: "own_rpc",
    active: ["own_rpc"],
    apply: true,
    gateway: { checked: true, disabled: false },
  });
});

test("turning sending through ScreenerBot on keeps the switch on", async (t) => {
  const { page, posts } = await openConnection(t, "own_rpc");
  await page.click(`label[for="settingsUseGateway"]`);
  await page.waitForFunction(() => !document.querySelector("#settingsUseGateway").disabled);
  assert.equal(posts.length, 1);
  assert.deepEqual((await readTab(page)).gateway, { checked: true, disabled: false });
});

test("switching asks for confirmation and cancelling sends nothing", async (t) => {
  const { page, posts } = await openConnection(t, "own_rpc");
  await page.check("#settings-connection-service");
  assert.equal((await readTab(page)).apply, false, "Apply enables once another mode is chosen");

  await page.click("#settingsConnectionApply");
  const dialog = page.locator(".confirmation-dialog");
  await dialog.waitFor();
  assert.equal(
    (await dialog.locator("#confirmation-dialog-title").textContent()).trim(),
    await page.evaluate(() => window.I18n.t("settings-connection-to-service-title"))
  );
  await page.keyboard.press("Escape");
  await dialog.waitFor({ state: "detached" });
  assert.equal(posts.length, 0, "no switch request without confirmation");
});
