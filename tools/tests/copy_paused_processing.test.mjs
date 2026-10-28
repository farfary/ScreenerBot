// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: while copy processing is paused for every task, the task wizard never
 * promises that a new task processes trades once saved.
 *
 * The Wallet step and the Review say a saved task copies nothing until processing
 * resumes and offer Resume in place. Resuming saves the global switch, keeps the
 * draft, and the Review then reads that the task processes trades once saved.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex, FIXTURES_ROOT } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const VALID_ADDRESS = "So11111111111111111111111111111111111111112";
const NOTE = "#copy-editor-body .copy-paused-note";
const strip = (text) => text.replace(/[⁨⁩]/g, "").trim();

test("a paused book says so in the wizard and resumes from it", async (t) => {
  const raw = JSON.parse(await readFile(join(FIXTURES_ROOT, "copy/copy_overview.json"), "utf8"));
  const overview = raw.data ?? raw;
  overview.status.enabled = false;
  // Paper tasks only: resuming asks first while a live task would submit swaps.
  overview.tasks = overview.tasks.map((task) => ({ ...task, mode: "paper" }));
  overview.status.live_tasks = 0;

  const patches = [];
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const path = request.url.pathname;
      if (path === "/api/copy-trading/overview") return { body: JSON.stringify(raw) };
      if (path === "/api/config/copy_trading" && request.method === "PATCH") {
        patches.push(path);
        overview.status.enabled = true;
        return { body: JSON.stringify({ success: true }) };
      }
      return api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector("#copy-main:not([hidden])");
  await page.locator("#copy-add").click();
  await page.waitForSelector("#copy-editor:not([hidden]) [data-field='target_address']");

  const walletNote = page.locator(NOTE);
  assert.match(strip(await walletNote.textContent()), /copies nothing until it resumes/);
  assert.equal(strip(await walletNote.locator("button").textContent()), "Resume copying");

  await page.locator("[data-field='target_address']").fill(VALID_ADDRESS);
  const next = page.locator("#copy-editor-next");
  while (await next.isVisible()) await next.click();
  const head = page.locator("#copy-editor-body .copy-review-head");
  await head.waitFor();
  assert.match(strip(await head.textContent()), /waits until copy processing resumes/);
  assert.doesNotMatch(strip(await head.textContent()), /processes trades once saved/);

  await page.locator(`${NOTE} button`).click();
  await page.waitForFunction(() => !document.querySelector("#copy-editor-body .copy-paused-note"));
  assert.deepEqual(patches, ["/api/config/copy_trading"]);
  assert.match(strip(await head.textContent()), /processes trades once saved/);
});
