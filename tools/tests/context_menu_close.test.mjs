// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: closing the right-click menu always finishes, so its full-window overlay never
 * outlives the menu.
 *
 * The overlay that catches outside clicks is removed by the close timer alone. A
 * right-click while the menu was open started the close on mousedown and then cancelled
 * that timer from its contextmenu handler, and pointer movement over the fading menu
 * cancelled it from the submenu hover handlers. The transparent overlay then stayed over
 * the whole window and swallowed every click and wheel event until a reload.
 *
 * - A right-click on another row while the menu is open replaces the menu with the one
 *   for that row; closing it leaves no overlay.
 * - Leaving a submenu row while the menu fades out does not keep its overlay.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const ROWS = "#tokens-root tr[data-row-id]";
const VISIBLE_MENU = ".context-menu.visible";
/** Longer than the menu's 120 ms close animation. */
const SETTLE_MS = 400;
/**
 * Pointer columns in the 1440 px window: the menu opens beside `NEAR` and hangs below
 * it, so a click at `FAR` lands outside the open menu, on its overlay.
 */
const NEAR = 200;
const FAR = 1000;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [tokens, shell] = await Promise.all([loadIndex("tokens"), loadIndex("shell")]);

async function openTokens(t) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([tokens, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/tokens`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector(ROWS);
  // Raw mouse input is hit-tested: the browser splash covers the page until it fades.
  await page.waitForSelector("#splashScreen", { state: "hidden" });
  return page;
}

/** The point at column `x` on the vertical centre of the row at `index`. */
async function rowPoint(page, index, x) {
  const box = await page.locator(ROWS).nth(index).boundingBox();
  return { x, y: box.y + box.height / 2 };
}

function menuState(page) {
  return page.evaluate(() => ({
    overlays: document.querySelectorAll(".context-menu-overlay").length,
    menus: document.querySelectorAll(".context-menu").length,
    symbol: document.querySelector(".context-menu.visible .context-menu-token-symbol")?.textContent,
  }));
}

function rowSymbol(page, index) {
  return page
    .locator(ROWS)
    .nth(index)
    .evaluate((row) =>
      row
        .querySelector(
          ".token-name-group .token-symbol, .dt-symbol, [data-field='symbol'], .token-symbol"
        )
        ?.textContent?.trim()
    );
}

test("a right-click on another row while the menu is open opens that row's menu", async (t) => {
  const page = await openTokens(t);
  const first = await rowPoint(page, 0, NEAR);
  const third = await rowPoint(page, 2, FAR);

  await page.mouse.click(first.x, first.y, { button: "right" });
  await page.waitForSelector(VISIBLE_MENU);
  assert.equal((await menuState(page)).symbol, await rowSymbol(page, 0));

  await page.mouse.click(third.x, third.y, { button: "right" });
  await page.waitForTimeout(SETTLE_MS);
  assert.deepEqual(
    await menuState(page),
    { overlays: 1, menus: 1, symbol: await rowSymbol(page, 2) },
    "the open menu is replaced by the menu of the row under the pointer"
  );

  await page.keyboard.press("Escape");
  await page.waitForTimeout(SETTLE_MS);
  assert.deepEqual(await menuState(page), { overlays: 0, menus: 0, symbol: undefined });
  const hit = await page.evaluate(
    ({ x, y }) => Boolean(document.elementFromPoint(x, y)?.closest("#tokens-root")),
    first
  );
  assert.ok(hit, "the table takes pointer input again once the menu has closed");
});

test("leaving a submenu row while the menu closes does not keep its overlay", async (t) => {
  const page = await openTokens(t);
  const first = await rowPoint(page, 0, NEAR);
  const outside = await rowPoint(page, 4, FAR);

  await page.mouse.click(first.x, first.y, { button: "right" });
  await page.waitForSelector(VISIBLE_MENU);
  const parent = await page
    .locator(`${VISIBLE_MENU} > .context-menu-item:has(> .context-menu-submenu)`)
    .first()
    .boundingBox();
  assert.ok(parent, "the token menu has a submenu row");

  // Rest on the submenu row, close with Escape, and leave the row while the menu fades.
  await page.mouse.move(parent.x + parent.width / 2, parent.y + parent.height / 2);
  await page.keyboard.press("Escape");
  await page.mouse.move(outside.x, outside.y);
  await page.waitForTimeout(SETTLE_MS);

  assert.deepEqual(await menuState(page), { overlays: 0, menus: 0, symbol: undefined });
});
