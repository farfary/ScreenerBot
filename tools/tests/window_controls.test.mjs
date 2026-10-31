// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a scrim darkens the whole window, the status bar and the macOS traffic lights
 * included, and a full-screen view darkens neither.
 *
 * - Every layer that dims the dashboard (a dialog, search, the actions drawer) covers the
 *   status bar and reports the title bar as dimmed to the Electron shell
 *   (`scripts/core/window_controls.js`); closing it reports it restored.
 * - A full-screen view (token details) ends at the status bar's top edge: the bar stays
 *   visible and hit-testable under it, and the traffic lights stay as they are. A dialog
 *   opened over the view dims both again.
 * - Outside the macOS shell nothing is reported.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

/** Longer than any overlay's open or close transition. */
const SETTLE_MS = 400;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

/** The preload bridge of the Electron shell, recording every dim report. */
function shellBridge(platform) {
  window.__dimReports = [];
  window.electronAPI = {
    isElectron: true,
    platform,
    setWindowControlsDimmed: (dimmed) => {
      window.__dimReports.push(dimmed);
      return Promise.resolve(true);
    },
    saveTheme: () => Promise.resolve(),
    setLanguage: (code) => Promise.resolve(code),
    onCheckForUpdates: () => () => {},
  };
}

async function openTokens(t, platform = "darwin") {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await context.addInitScript(shellBridge, platform);
  const api = createApiHandler([tokens, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#splashScreen", { state: "hidden" });
  return page;
}

const lastReport = (page) => page.evaluate(() => window.__dimReports.at(-1) ?? false);

async function expectReport(page, dimmed) {
  await page.waitForFunction((expected) => (window.__dimReports.at(-1) ?? false) === expected, dimmed);
}

/** Whether the status bar's centre answers a hit test, i.e. nothing covers it. */
const statusBarOnTop = (page) =>
  page.evaluate(() => {
    const bar = document.getElementById("statusBar");
    const rect = bar.getBoundingClientRect();
    return bar.contains(document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2));
  });

test("a dialog, search and the actions drawer each dim the status bar and the traffic lights", async (t) => {
  const page = await openTokens(t);
  await page.waitForTimeout(SETTLE_MS);
  assert.equal(await lastReport(page), false);
  assert.equal(await statusBarOnTop(page), true);

  for (const trigger of ["#settingsBtn", "#searchBtn", "#notificationBtn"]) {
    await page.click(trigger);
    await expectReport(page, true);
    assert.equal(await statusBarOnTop(page), false, `${trigger}: the scrim covers the status bar`);

    await page.keyboard.press("Escape");
    await expectReport(page, false);
    await page.waitForTimeout(SETTLE_MS);
    assert.equal(await lastReport(page), false, `${trigger}: closed stays restored`);
    assert.equal(await statusBarOnTop(page), true, `${trigger}: closed uncovers the status bar`);
  }
});

test("a full-screen view keeps the status bar usable and dims nothing until a dialog opens over it", async (t) => {
  const page = await openTokens(t);
  await page.click("#tokens-root tr[data-row-id] .ti-row-cell__symbol");
  await page.waitForSelector(".token-details-dialog.active");
  await page.waitForTimeout(SETTLE_MS);

  assert.equal(await lastReport(page), false);
  assert.equal(await statusBarOnTop(page), true);
  const edges = await page.evaluate(() => ({
    view: document.querySelector(".token-details-dialog").getBoundingClientRect().bottom,
    bar: document.getElementById("statusBar").getBoundingClientRect().top,
  }));
  assert.equal(edges.view, edges.bar, "the view ends at the status bar's top edge");

  await page.evaluate(() => document.getElementById("settingsBtn").click());
  await expectReport(page, true);
  assert.equal(await statusBarOnTop(page), false);
});

test("outside the macOS shell nothing is reported", async (t) => {
  const page = await openTokens(t, "win32");
  await page.click("#settingsBtn");
  await page.waitForTimeout(SETTLE_MS);
  assert.deepEqual(await page.evaluate(() => window.__dimReports), []);
  assert.equal(await statusBarOnTop(page), false, "the scrim still covers the status bar");
});
