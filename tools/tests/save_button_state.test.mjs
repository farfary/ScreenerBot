// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Save action with nothing to save looks unavailable, the same way on every
 * surface. The Config page header drew its own button family whose disabled state was
 * the enabled blue at half opacity, so "Save Changes" read as live with no pending
 * change. Both the Config header and the Settings dialog now take the shared `.btn`
 * family, whose disabled state is owned once in components.css.
 *
 * - Each disabled Save is a `.btn.btn-primary` and paints the neutral disabled surface
 *   with the muted label, at full opacity.
 * - The two surfaces paint the disabled Save identically.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const shell = await loadIndex("shell");

/** Computed look of a disabled Save button and the theme's reference colours. */
async function disabledSave(pageName, open, selector) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex(pageName), shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  try {
    const page = await context.newPage();
    await page.goto(`${host.origin}/${pageName}`);
    await page.waitForSelector(READY);
    await open(page);
    await page.waitForSelector(`${selector}:disabled`);
    return await page.$eval(selector, (button) => {
      const probe = document.createElement("span");
      probe.style.cssText = "background: var(--bg-secondary); color: var(--text-muted)";
      document.body.append(probe);
      const reference = getComputedStyle(probe);
      const style = getComputedStyle(button);
      const result = {
        classes: ["btn", "btn-primary"].every((name) => button.classList.contains(name)),
        background: style.backgroundColor,
        color: style.color,
        opacity: style.opacity,
        surface: reference.backgroundColor,
        muted: reference.color,
      };
      probe.remove();
      return result;
    });
  } finally {
    await context.close();
    await host.close();
  }
}

test("a Save with nothing pending looks unavailable on every surface", async () => {
  const config = await disabledSave(
    "config",
    async () => {},
    ".config-header-actions .btn-primary"
  );
  const settings = await disabledSave(
    "home",
    async (page) => {
      await page.locator("#settingsBtn").dispatchEvent("click");
      await page.waitForSelector(".settings-dialog.active .settings-nav");
    },
    "#settingsSaveBtn"
  );
  for (const [name, save] of [
    ["Config", config],
    ["Settings", settings],
  ]) {
    assert.ok(save.classes, `${name}: Save is a shared .btn.btn-primary`);
    assert.equal(save.background, save.surface, `${name}: the neutral disabled surface`);
    assert.equal(save.color, save.muted, `${name}: the muted label`);
    assert.equal(save.opacity, "1", `${name}: full opacity`);
  }
  assert.deepEqual(
    [config.background, config.color],
    [settings.background, settings.color],
    "both surfaces paint the disabled Save the same"
  );
});
