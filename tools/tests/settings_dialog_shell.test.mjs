// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Settings dialog shell keeps its frame steady between sections.
 *
 * Switching sections kept the previous section's scroll offset, so a section
 * could open mid-page. In a short window the sidebar was taller than the dialog:
 * the external links fell below the edge and the dividers collapsed to blank
 * gaps. The header Save button vanished on sections that save on their own, and
 * on first open read "Save Changes" with nothing pending.
 *
 * - Every section opens at the top.
 * - The section list scrolls on its own; the external links sit below it, outside
 *   the scroller, at the foot of the sidebar, and every divider is drawn. A sticky
 *   footer inside the scroller drew over the last sections instead.
 * - The Save button keeps one place in the header on every section, and reads
 *   "Saved" when nothing is pending.
 * - Every select in one settings list has one width (Theme, Language and Refresh
 *   were each as wide as their own value), and no select cuts its value.
 * - The Navigation list shows every tab row at its full height inside the dialog's own
 *   scroll, each row's glyph bare, with the switch as the only visibility control and
 *   no note asking for a page refresh (saving rebuilds the navigation bar).
 * - A field's status badge sits on its title's line, not stacked under the hint.
 * - The Data section sets both stored paths at one start edge inside their cards, and
 *   every button and input in it stands at one height.
 * - A disabled Security action states in its row what it waits for.
 * - Every boolean in Settings is a switch, never a bare checkbox, and every section
 *   title carries its glyph. A field whose control stands far taller than its label
 *   block heads that control from the top edge. No Settings action is a frameless
 *   ghost button, whose label reads as indented text; the account panel shared with
 *   the setup screen keeps its own button set and is the one exemption.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

test("Settings sections share one steady frame in a short window", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1200, height: 700 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  // At this width the header action sits in the overflow menu.
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");
  await page
    .locator(".settings-container")
    .evaluate((node) => Promise.all(node.getAnimations().map((animation) => animation.finished)));

  const save = page.locator("#settingsSaveBtn");
  assert.equal((await save.textContent()).trim(), "Saved");

  const sidebar = await page.evaluate(() => {
    const nav = document.querySelector(".settings-nav").getBoundingClientRect();
    const list = document.querySelector(".settings-nav-list");
    const footer = document.querySelector(".settings-nav-footer");
    const links = [...document.querySelectorAll(".settings-nav-link")].map((link) =>
      link.getBoundingClientRect()
    );
    list.scrollTop = list.scrollHeight;
    const last = [...list.querySelectorAll(".settings-nav-item")].at(-1).getBoundingClientRect();
    return {
      overflows: list.scrollHeight > list.clientHeight,
      footerOutsideScroller:
        !list.contains(footer) && getComputedStyle(footer).position === "static",
      footerAtFoot: Math.abs(footer.getBoundingClientRect().bottom - nav.bottom) <= 1,
      lastSectionClear: last.bottom <= footer.getBoundingClientRect().top + 0.5,
      linksInside: links.every((link) => link.top >= nav.top && link.bottom <= nav.bottom),
      dividers: [...document.querySelectorAll(".settings-nav-divider")].map(
        (divider) => divider.getBoundingClientRect().height
      ),
    };
  });
  assert.ok(sidebar.overflows, "the window is short enough for the section list to scroll");
  assert.ok(sidebar.footerOutsideScroller, "the external links are not part of the scroller");
  assert.ok(sidebar.footerAtFoot, "the external links sit at the foot of the sidebar");
  assert.ok(sidebar.lastSectionClear, "the last section scrolls clear of the external links");
  assert.ok(sidebar.linksInside, "the external links stay in view");
  for (const height of sidebar.dividers) {
    assert.ok(height >= 1, `divider height ${height}`);
  }

  const content = page.locator(".settings-content");
  const slot = async (tab) => {
    const box = await save.boundingBox();
    return { tab, x: Math.round(box.x), width: Math.round(box.width) };
  };
  const slots = [await slot("interface")];
  for (const tab of ["data", "security", "startup", "interface"]) {
    await content.evaluate((node) => {
      node.scrollTop = node.scrollHeight;
    });
    await page.locator(`.settings-nav-item[data-tab="${tab}"]`).click();
    assert.equal(await content.evaluate((node) => node.scrollTop), 0, `${tab} opens at the top`);
    slots.push(await slot(tab));
  }
  for (const { tab, x, width } of slots) {
    assert.deepEqual({ x, width }, { x: slots[0].x, width: slots[0].width }, tab);
  }
});

test("every select in one settings list has one width", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");

  let lists = 0;
  for (const tab of ["interface", "navigation", "startup", "security", "telegram"]) {
    await page.locator(`.settings-nav-item[data-tab="${tab}"]`).click();
    await page.waitForFunction(() =>
      [
        ...document.querySelectorAll(".settings-content .settings-field-control .custom-select"),
      ].some((select) => select.getClientRects().length > 0)
    );
    const groups = await page.$$eval(".settings-content .settings-group", (nodes) =>
      nodes
        .filter((group) => group.getClientRects().length > 0)
        .map((group) =>
          [...group.querySelectorAll(".settings-field-control .custom-select")].map((select) => {
            const value = select.querySelector(".cs-value");
            return {
              width: Math.round(select.getBoundingClientRect().width),
              cut: value ? value.scrollWidth > value.clientWidth : false,
            };
          })
        )
        .filter((selects) => selects.length > 0)
    );
    for (const selects of groups) {
      lists += selects.length > 1 ? 1 : 0;
      const widths = [...new Set(selects.map((select) => select.width))];
      assert.equal(widths.length, 1, `${tab}: one select width per list, got ${widths}`);
      assert.ok(
        selects.every((select) => !select.cut),
        `${tab}: no select cuts its value`
      );
    }
  }
  assert.ok(lists > 0, "a settings list holds more than one select");
});

test("the Navigation list shows every row with bare glyphs and one visibility control", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");
  await page.locator('.settings-nav-item[data-tab="navigation"]').click();
  await page.waitForFunction(
    () => document.querySelector("#navTabsList .settings-nav-tab-item")?.getClientRects().length > 0
  );

  const list = await page.evaluate(() => {
    const root = document.querySelector("#navTabsList");
    const rows = [...root.querySelectorAll(".settings-nav-tab-item")];
    const painted = (element) => {
      const style = getComputedStyle(element);
      return (
        style.backgroundColor !== "rgba(0, 0, 0, 0)" ||
        style.borderStyle !== "none" ||
        style.boxShadow !== "none"
      );
    };
    return {
      rows: rows.length,
      scrolls: root.scrollHeight > root.clientHeight + 1,
      boxed: rows
        .flatMap((row) => [
          ...row.querySelectorAll(".settings-nav-tab-icon, .settings-nav-tab-position"),
        ])
        .filter(painted).length,
      controls: rows.map((row) => row.querySelectorAll("input, .settings-nav-tab-status").length),
      refreshNote: /refresh/i.test(root.closest(".settings-section").textContent),
    };
  });

  assert.ok(list.rows > 8, "the fixture lists every navigation tab");
  assert.equal(list.scrolls, false, "the list takes its own height");
  assert.equal(list.boxed, 0, "row glyphs and positions are bare");
  assert.ok(
    list.controls.every((count) => count === 1),
    "the switch is each row's only visibility control"
  );
  assert.equal(list.refreshNote, false, "no note asks for a page refresh");
});

test("Startup badges sit on their title line and Data controls share one frame", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");

  await page.locator('.settings-nav-item[data-tab="startup"]').click();
  await page.waitForFunction(
    () =>
      document.querySelector(".settings-content .settings-field-badge")?.getClientRects().length > 0
  );
  const badges = await page.$$eval(".settings-content .settings-field-badge", (nodes) =>
    nodes
      .filter((badge) => badge.getClientRects().length > 0)
      .map((badge) => {
        const info = badge.closest(".settings-field-info");
        const title = info.querySelector("label").getBoundingClientRect();
        const box = badge.getBoundingClientRect();
        return box.top < title.bottom && box.bottom > title.top;
      })
  );
  assert.ok(badges.length > 0 && badges.every(Boolean), "each badge shares its title's line");

  await page.locator('.settings-nav-item[data-tab="data"]').click();
  await page.waitForFunction(
    () => document.querySelector("#dataPathDisplay")?.getClientRects().length > 0
  );
  const data = await page.evaluate(() => {
    const content = document.querySelector(".settings-content");
    const visible = (node) => node.getClientRects().length > 0;
    return {
      paths: ["#dataPathDisplay", "#configPathDisplay"].map((selector) => ({
        start: Math.round(content.querySelector(selector).getBoundingClientRect().left),
        carded: Boolean(content.querySelector(selector).closest(".settings-group")),
      })),
      heights: [
        ...new Set(
          [...content.querySelectorAll("button.btn, .settings-field-control input")]
            .filter(visible)
            .map((node) => Math.round(node.getBoundingClientRect().height))
        ),
      ],
    };
  });
  assert.ok(
    data.paths.every((path) => path.carded),
    "both paths sit inside a card"
  );
  assert.equal(data.paths[0].start, data.paths[1].start, "both paths share one start edge");
  assert.deepEqual(data.heights, [32], "every Data control stands at one height");
});

test("a disabled Security action names what it waits for", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");
  await page.locator('.settings-nav-item[data-tab="security"]').click();
  await page.waitForFunction(
    () => document.querySelector("#securityLockNowBtn")?.getClientRects().length > 0
  );
  const rows = await page.$$eval(".settings-content .settings-field", (fields) =>
    fields
      .filter((field) => field.querySelector(".settings-field-control button.btn:disabled"))
      .map((field) => field.querySelector(".settings-field-info").textContent)
  );
  assert.ok(rows.length >= 2, "the fixture has no password, so actions wait for one");
  for (const text of rows) assert.match(text, /first to use this/);
});

test("every Settings boolean is a switch, every title has its glyph, tall fields align to the top", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("home"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#settingsBtn").dispatchEvent("click");
  await page.waitForSelector(".settings-dialog.active .settings-nav");
  const tabs = await page.$$eval(".settings-nav-item[data-tab]", (items) =>
    items.map((item) => item.dataset.tab)
  );
  // Sections whose per-page group titles have no glyph owner yet (the hint
  // categories are page names, and page glyphs live in the navigation config).
  // This list only shrinks.
  const unglyphedTabs = new Set(["hints"]);
  const bare = [];
  const untitled = [];
  const floating = [];
  const ghosts = [];
  for (const tab of tabs) {
    await page.locator(`.settings-nav-item[data-tab="${tab}"]`).click();
    await page.waitForTimeout(400);
    ghosts.push(
      ...(await page.$$eval(
        ".settings-content :is(.btn-ghost, .account-btn-ghost)",
        (buttons, name) =>
          buttons
            .filter((button) => button.getClientRects().length > 0)
            .filter((button) => !button.closest("#settingsAccountPanel"))
            .map((button) => `${name}#${button.id || button.textContent.trim()}`),
        tab
      ))
    );
    floating.push(
      ...(await page.$$eval(
        ".settings-content .settings-field",
        (fields, name) =>
          fields
            .filter((field) => field.getClientRects().length > 0)
            .map((field) => [
              field,
              field.querySelector(":scope > .settings-field-info")?.getBoundingClientRect(),
              field.querySelector(":scope > .settings-field-control")?.getBoundingClientRect(),
            ])
            .filter(([, info, control]) => info && control && control.height > info.height + 40)
            .filter(([, info, control]) => Math.abs(info.top - control.top) > 2)
            .map(
              ([field]) =>
                `${name}: ${field.querySelector(".settings-field-info")?.textContent.trim().slice(0, 40)}`
            ),
        tab
      ))
    );
    if (!unglyphedTabs.has(tab))
      untitled.push(
        ...(await page.$$eval(
          ".settings-content .settings-section-title",
          (titles, name) =>
            titles
              .filter((title) => title.getClientRects().length > 0 && !title.querySelector("i"))
              .map((title) => `${name}: ${title.textContent.trim()}`),
          tab
        ))
      );
    bare.push(
      ...(await page.$$eval(
        '.settings-content input[type="checkbox"]',
        (inputs, name) =>
          inputs
            .filter((input) => !input.closest(".toggle"))
            .filter((input) => input.closest(".settings-content > *")?.getClientRects().length > 0)
            .map((input) => `${name}#${input.id}`),
        tab
      ))
    );
  }
  assert.ok(tabs.includes("account"), "the Account section is listed");
  assert.deepEqual(bare, []);
  assert.deepEqual(untitled, [], "every section title carries its glyph");
  assert.deepEqual(floating, [], "a tall control's label sits on its top edge");
  assert.deepEqual(ghosts, [], "no Settings action is a frameless ghost button");
});
