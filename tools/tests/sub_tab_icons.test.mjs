// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the page sub-tabs are labels only.
 *
 * A sub-tab is marked selected by its colour and bar, so a glyph adds no information and
 * costs the row the width it needs to fit at 1200 px in every catalog. The tab bar
 * renders a tab's text `label` and nothing else.
 *
 * - No sub-tab of any page with a tab bar renders a glyph.
 * - No page passes `icon` in a tab bar tab definition (read from the registered bars).
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
  test(`the ${name} sub-tabs render their name and no glyph`, async (t) => {
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
        children: button.children.length,
        text: button.textContent.trim(),
      }))
    );
    assert.ok(tabs.length >= 2, `${name} has a tab bar`);
    for (const tab of tabs) {
      assert.ok(tab.text, `${name}/${tab.id} has a name`);
      assert.equal(tab.children, 0, `${name}/${tab.id} renders only its name`);
    }

    // The definitions themselves carry no glyph: the page's registered bar is read
    // from the same module instance the page imported.
    const definitions = await page.evaluate(async () => {
      const { TabBarManager } = await import("/scripts/ui/tab_bar.js");
      return TabBarManager.getAllTabBars().flatMap((bar) =>
        bar.tabs.map((tab) => ({ id: tab.id, keys: Object.keys(tab) }))
      );
    });
    assert.ok(definitions.length >= 2, `${name} registers its tab bar`);
    for (const { id, keys } of definitions) {
      assert.ok(!keys.includes("icon"), `${name}/${id} passes an icon`);
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
