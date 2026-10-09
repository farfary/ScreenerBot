// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the main navigation's two overflow layouts.
 *
 * - "scroll" (the default) keeps every tab in the row and shows no More control.
 * - "menu" keeps the row inside its box: the tabs that fit stay in order, the rest are
 *   listed under a More control at the row's inline end, and the list is reachable and
 *   usable from the keyboard (open, move, follow a page, Escape back to the control).
 *   While the page on screen is a folded tab, the More control carries the current mark
 *   and the travelling indicator sits under it.
 *
 * Each case runs in a left-to-right and a right-to-left locale.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const LOCALES = [
  { locale: "en", next: "ArrowRight" },
  { locale: "fa", next: "ArrowLeft" },
];

const browser = await chromium.launch({ headless: true });
const hosts = [];
after(async () => {
  await browser.close();
  await Promise.all(hosts.map((host) => host.close()));
});

const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

async function openHome(locale, width) {
  const context = await browser.newContext({ viewport: { width, height: 800 }, locale });
  const fixtures = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, { locale, onApi: fixtures.onApi });
  hosts.push(host);
  const page = await context.newPage();
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("#navTabs > a.tab.active");
  await page.waitForSelector("#navTabs > .nav-more", { state: "attached" });
  return { context, page };
}

async function useMenuLayout(page) {
  await page.evaluate(() => {
    document.documentElement.dataset.navOverflow = "menu";
  });
  // The layout change is applied on the next animation frame.
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve(true)))
      )
  );
}

/** The row's geometry: its box, each tab's state and box, and the More control. */
function rowState(page) {
  return page.evaluate(() => {
    const rect = (element) => {
      const box = element.getBoundingClientRect();
      return { left: box.left, right: box.right };
    };
    const row = document.querySelector(".header-row-2");
    const more = document.querySelector("#navTabs > .nav-more");
    return {
      row: rect(row),
      tabs: [...document.querySelectorAll("#navTabs > a.tab")].map((tab) => ({
        id: tab.dataset.page,
        hidden: tab.hidden,
        ...rect(tab),
      })),
      more: more.hidden ? null : rect(more),
    };
  });
}

for (const { locale, next } of LOCALES) {
  test(`the scrolling layout keeps every tab in the row (${locale})`, async () => {
    const { context, page } = await openHome(locale, 1200);
    const { tabs, more } = await rowState(page);
    assert.equal(more, null, "no More control in the scrolling layout");
    assert.deepEqual(
      tabs.filter((tab) => tab.hidden).map((tab) => tab.id),
      [],
      "no tab is folded"
    );
    await context.close();
  });

  test(`the More layout keeps the row inside its box (${locale})`, async () => {
    const { context, page } = await openHome(locale, 1200);
    await useMenuLayout(page);
    const { row, tabs, more } = await rowState(page);
    assert.ok(more, "the More control is shown when the tabs do not fit");
    const shown = tabs.filter((tab) => !tab.hidden);
    const folded = tabs.filter((tab) => tab.hidden);
    assert.ok(folded.length > 0, "some tabs are folded at 1200px");
    assert.deepEqual(
      tabs.map((tab) => tab.hidden),
      tabs.map((_, index) => index >= shown.length),
      "the tabs that stay are the first ones, in order"
    );
    for (const box of [...shown, more]) {
      assert.ok(
        box.left >= row.left - 0.5 && box.right <= row.right + 0.5,
        `${box.id ?? "More"} stays inside the row`
      );
    }
    const last = shown.at(-1);
    assert.ok(
      locale === "fa" ? more.right <= last.left + 0.5 : more.left >= last.right - 0.5,
      "the More control sits at the row's inline end"
    );
    await context.close();
  });

  test(`the More list works from the keyboard (${locale})`, async () => {
    const { context, page } = await openHome(locale, 1200);
    await useMenuLayout(page);
    const { tabs } = await rowState(page);
    const folded = tabs.filter((tab) => tab.hidden).map((tab) => tab.id);
    const shown = tabs.filter((tab) => !tab.hidden).map((tab) => tab.id);

    // The arrows follow the reading direction.
    await page.focus(`#navTabs > a.tab[data-page="${shown[0]}"]`);
    await page.keyboard.press(next);
    assert.equal(
      await page.evaluate(() => document.activeElement?.dataset.page),
      shown[1],
      "the next-entry arrow moves to the second tab"
    );

    // End reaches the More control; ArrowDown opens the list on its first page.
    await page.keyboard.press("End");
    assert.equal(await page.evaluate(() => document.activeElement?.id), "navMoreToggle");
    await page.keyboard.press("ArrowDown");
    await page.waitForFunction(() => document.activeElement?.matches("#navMoreList a"));
    // Hit-tested once the list has finished fading in.
    await page.waitForFunction(
      () => document.getElementById("navMoreList").getAnimations().length === 0
    );
    const opened = await page.evaluate(() => {
      const item = document.activeElement;
      const box = item.getBoundingClientRect();
      const top = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
      return {
        expanded: document.getElementById("navMoreToggle").getAttribute("aria-expanded"),
        focused: item.dataset.page,
        items: [...document.querySelectorAll("#navMoreList .dropdown-item")].map(
          (entry) => entry.dataset.page
        ),
        uncovered: item.contains(top),
      };
    });
    assert.deepEqual(opened, {
      expanded: "true",
      focused: folded[0],
      items: folded,
      uncovered: true,
    });

    await page.keyboard.press("End");
    assert.equal(await page.evaluate(() => document.activeElement?.dataset.page), folded.at(-1));

    // Escape closes the list and returns focus to the control.
    await page.keyboard.press("Escape");
    assert.deepEqual(
      await page.evaluate(() => [
        document.getElementById("navMoreToggle").getAttribute("aria-expanded"),
        document.activeElement?.id,
      ]),
      ["false", "navMoreToggle"]
    );

    // Following a folded page from the list marks the More control as current.
    await page.keyboard.press("Enter");
    await page.waitForFunction(() => document.activeElement?.matches("#navMoreList a"));
    await page.keyboard.press("Enter");
    await page.waitForSelector(`#navTabs > a.tab.active[data-page="${folded[0]}"]`, {
      state: "attached",
    });
    await page.waitForSelector("#navMoreToggle.active");
    await page.waitForFunction(() => {
      const toggle = document.getElementById("navMoreToggle").getBoundingClientRect();
      const bar = document.querySelector(".nav-tabs-indicator").getBoundingClientRect();
      return Math.abs(bar.left - toggle.left) < 1 && Math.abs(bar.width - toggle.width) < 1;
    });
    await context.close();
  });
}

test("the More layout shows every tab when they all fit", async () => {
  const { context, page } = await openHome("en", 1920);
  await useMenuLayout(page);
  const { tabs, more } = await rowState(page);
  assert.equal(more, null);
  assert.ok(tabs.every((tab) => !tab.hidden));
  await context.close();
});
