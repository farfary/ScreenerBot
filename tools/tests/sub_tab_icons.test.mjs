// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every page's sub-tabs carry an icon, drawn by the tab bar.
 *
 * Each page once built its own icon markup into the tab label, and the Assistant page
 * built none, so its sub-tabs were the only ones without icons. A tab now passes its
 * `icon` class and a text `label`, and `ui/tab_bar.js` renders both.
 *
 * - Every sub-tab of every page with a tab bar shows one decorative icon before its name.
 * - No page script writes icon markup into a tab label.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { SCRIPTS_ROOT, walk } from "../lib/dashboard_ui.mjs";

const PAGES = ["tokens", "positions", "wallets", "trader", "filtering", "assistant"];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");

for (const name of PAGES) {
  test(`the ${name} sub-tabs each show an icon before the name`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    const api = createApiHandler([await loadIndex(name), shell], "populated");
    const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/${name}`);
    await page.waitForSelector("#subTabsContainer .sub-tab");

    const tabs = await page.$$eval("#subTabsContainer .sub-tab", (buttons) =>
      buttons.map((button) => ({
        id: button.dataset.tabId,
        children: [...button.children].map((child) => ({
          tag: child.tagName,
          icon: [...child.classList].some((token) => token.startsWith("icon-")),
          hidden: child.getAttribute("aria-hidden"),
        })),
        firstIsIcon: button.firstElementChild === button.querySelector("i"),
        text: button.textContent.trim(),
      }))
    );
    assert.ok(tabs.length >= 2, `${name} has a tab bar`);
    for (const tab of tabs) {
      assert.ok(tab.text, `${name}/${tab.id} has a name`);
      assert.ok(tab.firstIsIcon, `${name}/${tab.id} starts with its icon`);
      assert.deepEqual(
        tab.children.filter((child) => child.tag === "I"),
        [{ tag: "I", icon: true, hidden: "true" }],
        `${name}/${tab.id} has one decorative icon`
      );
    }
  });
}

test("no page writes icon markup into a tab label", async () => {
  const found = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(/label:\s*`<i\b/g)) {
      const line = source.slice(0, match.index).split("\n").length;
      found.push(`${file.slice(SCRIPTS_ROOT.length + 1)}:${line}`);
    }
  }
  assert.deepEqual(found, []);
});
