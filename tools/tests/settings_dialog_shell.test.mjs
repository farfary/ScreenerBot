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
 * - The external links stay inside the sidebar and every divider is drawn.
 * - The Save button keeps one place in the header on every section, and reads
 *   "Saved" when nothing is pending.
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
    const links = [...document.querySelectorAll(".settings-nav-link")].map((link) =>
      link.getBoundingClientRect()
    );
    return {
      overflows: document.querySelector(".settings-nav").scrollHeight > nav.height,
      linksInside: links.every((link) => link.top >= nav.top && link.bottom <= nav.bottom),
      dividers: [...document.querySelectorAll(".settings-nav-divider")].map(
        (divider) => divider.getBoundingClientRect().height
      ),
    };
  });
  assert.ok(sidebar.overflows, "the window is short enough for the sidebar to scroll");
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
