// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the shared select's menu shows every option in full.
 *
 * The portaled menu is at least the trigger's width and grows to its longest option,
 * clamped to the viewport. Its width once came from the labels' `scrollWidth`, which
 * rounds: "More menu" in Settings > Navigation measured 70px, rendered 70.1px, and was
 * cut to "More me…" in a menu sized to the rounded figure.
 *
 * Its height holds every option inside the window, above the status bar that stays on
 * top of every menu: it continues the field below, or above when only that side has
 * room, and floats over the field when neither side does. A list capped at 280px once
 * opened above a mid-screen field with its first option cut off by the window's top.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const LOCALES = ["en", "de", "fa"];
const VIEWPORT_MARGIN = 8;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/** The open menu's box, the trigger's box and each option's text against its label box. */
function measureOpenMenu(page, triggerSelector) {
  return page.evaluate((selector) => {
    const trigger = document.querySelector(selector);
    const menu = document.querySelector(".cs-dropdown.cs-portal.is-open");
    const range = document.createRange();
    return {
      trigger: trigger.getBoundingClientRect().width,
      menu: menu.getBoundingClientRect().toJSON(),
      viewport: window.innerWidth,
      options: [...menu.querySelectorAll(".cs-option-label")].map((label) => {
        range.selectNodeContents(label);
        return {
          text: label.textContent,
          text_width: range.getBoundingClientRect().width,
          box_width: label.getBoundingClientRect().width,
        };
      }),
    };
  }, triggerSelector);
}

async function openDashboard(locale, width = 1200, height = 800) {
  const context = await browser.newContext({ viewport: { width, height } });
  const api = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, { locale, onApi: api.onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector(PAGE_READY);
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

for (const locale of LOCALES) {
  test(`Settings > Navigation shows every overflow option in full (${locale})`, async () => {
    const { page, close } = await openDashboard(locale);
    try {
      await page.locator("#settingsBtn").dispatchEvent("click");
      await page.waitForSelector(".settings-dialog.active .settings-nav");
      await page.locator('.settings-nav-item[data-tab="navigation"]').click();
      const trigger = "#settingNavOverflow-custom";
      await page.locator(trigger).click();
      await page.waitForSelector(".cs-dropdown.cs-portal.is-open");
      const { trigger: triggerWidth, menu, options } = await measureOpenMenu(page, trigger);

      assert.ok(
        menu.width >= triggerWidth - 0.5,
        `menu ${menu.width}px < trigger ${triggerWidth}px`
      );
      assert.equal(options.length, 2);
      for (const option of options) {
        assert.ok(
          option.text_width <= option.box_width + 0.01,
          `"${option.text}" needs ${option.text_width}px in a ${option.box_width}px label`
        );
      }
    } finally {
      await close();
    }
  });
}

test("a menu wider than the window is clamped to it", async () => {
  const { page, close } = await openDashboard("en", 900);
  try {
    await page.evaluate(() => {
      const host = document.createElement("div");
      host.id = "select-width-probe";
      host.style.cssText =
        "position: fixed; inset-block-start: 120px; inset-inline-start: 40px; width: 140px;";
      host.innerHTML = `<select id="probeSelect" data-custom-select>
        <option value="a">Short</option>
        <option value="b">${"A very long option label ".repeat(12).trim()}</option>
      </select>`;
      document.body.append(host);
    });
    const trigger = "#probeSelect-custom";
    await page.waitForSelector(trigger);
    await page.locator(trigger).click();
    await page.waitForSelector(".cs-dropdown.cs-portal.is-open");
    const { menu, viewport, options } = await measureOpenMenu(page, trigger);

    assert.ok(menu.left >= VIEWPORT_MARGIN - 0.5, `menu starts at ${menu.left}px`);
    assert.ok(
      menu.right <= viewport - VIEWPORT_MARGIN + 0.5,
      `menu ends at ${menu.right}px of ${viewport}px`
    );
    assert.equal(Math.round(menu.width), viewport - VIEWPORT_MARGIN * 2);
    const short = options.find((option) => option.text === "Short");
    assert.ok(short.text_width <= short.box_width + 0.01, "the short option is whole");
  } finally {
    await close();
  }
});

/** Where an eight-option list must open for a field at `top` in a 500px-high window. */
const PLACEMENTS = [
  { top: 40, above: false, detached: false },
  { top: 420, above: true, detached: false },
  { top: 200, above: false, detached: true },
];

for (const placement of PLACEMENTS) {
  test(`every option is in the window for a field at ${placement.top}px`, async () => {
    const { page, close } = await openDashboard("en", 1200, 500);
    try {
      await page.evaluate((top) => {
        const host = document.createElement("div");
        host.style.cssText = `position: fixed; inset-block-start: ${top}px; inset-inline-start: 40px; width: 160px;`;
        const options = Array.from(
          { length: 8 },
          (_, i) => `<option value="${i}">Option ${i}</option>`
        );
        host.innerHTML = `<select id="probeSelect" data-custom-select>${options.join("")}</select>`;
        document.body.append(host);
      }, placement.top);
      // The header covers a field this high, so the field is opened by its own event.
      const trigger = "#probeSelect-custom";
      await page.waitForSelector(trigger);
      await page.locator(`${trigger} .cs-trigger`).dispatchEvent("click");
      await page.waitForSelector(".cs-dropdown.cs-portal.is-open");
      await page.waitForTimeout(300);
      const open = await page.evaluate(() => {
        const menu = document.querySelector(".cs-dropdown.cs-portal.is-open");
        const statusBar = document.getElementById("statusBar").getBoundingClientRect();
        return {
          floor: Math.min(window.innerHeight, statusBar.top),
          scrolls: menu.scrollHeight > menu.clientHeight,
          above: menu.classList.contains("cs-dropdown--above"),
          detached: menu.classList.contains("cs-dropdown--detached"),
          options: [...menu.querySelectorAll(".cs-option")].map((option) => {
            const { top, bottom } = option.getBoundingClientRect();
            return { text: option.textContent.trim(), top, bottom };
          }),
        };
      });

      assert.equal(open.options.length, 8);
      assert.equal(open.scrolls, false, "the list scrolls although the window holds it");
      for (const option of open.options) {
        assert.ok(option.top >= VIEWPORT_MARGIN - 0.5, `${option.text} starts at ${option.top}px`);
        assert.ok(
          option.bottom <= open.floor - VIEWPORT_MARGIN + 0.5,
          `${option.text} ends at ${option.bottom}px, the status bar at ${open.floor}px`
        );
      }
      assert.equal(open.above, placement.above, "opens above the field");
      assert.equal(open.detached, placement.detached, "floats apart from the field");
    } finally {
      await close();
    }
  });
}
