// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the display language can be chosen on the first-run screens, before the
 * configuration API is open.
 *
 * - The onboarding header offers the system default and every registered locale by
 *   its native name, beside a bare language glyph. Its list, and the toast of a refused
 *   save, open above the full-screen first-run layer (Playwright's click refuses an
 *   option another layer covers).
 * - Choosing a language posts its code to `/api/i18n/language` and reloads into it;
 *   the introduction reopens on the slide it was showing.
 * - The setup wizard's picker locks, saying where the language can be changed, once
 *   a wallet source is chosen: the reload would discard the created or typed key.
 * - A refused save keeps the current language, reports the server's reason and does
 *   not reload.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { formatMessage } from "../html/l10n_transform.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");
const BOOTSTRAP = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/system_bootstrap.json`, "utf8"));
const LOCALES = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/i18n_locales.json`, "utf8"));
const STATUS = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/shell/initialization_status.json`, "utf8")
);
const FIRST_RUN = {
  ...BOOTSTRAP,
  initialization_required: true,
  initialization_complete: false,
  onboarding_complete: false,
  ui_ready: false,
};
const FIRST_RUN_STATUS = {
  ...STATUS,
  required: true,
  config_exists: false,
  initialization_complete: false,
  onboarding_complete: false,
};
const REFUSED = {
  success: false,
  error: {
    code: "CONFIG_ERROR",
    message: "Language could not be saved",
    details: "disk full",
  },
};

/** Opens the first-run introduction; `language` answers `POST /api/i18n/language`. */
async function openFirstRun(t, { language = (body) => ({ body: JSON.stringify(body) }) } = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([shell], "populated");
  const onApi = (request) => {
    const path = request.url.pathname;
    if (path === "/api/system/bootstrap") return { body: JSON.stringify(FIRST_RUN) };
    if (path === "/api/initialization/status") return { body: JSON.stringify(FIRST_RUN_STATUS) };
    if (path === "/api/i18n/language") return language({ language: "fa" });
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/`);
  await page.locator("#onboardingScreen").waitFor({ state: "visible" });
  await page.locator("#onboarding-language .custom-select").waitFor({ state: "visible" });
  return page;
}

/** Opens the picker under `host` and chooses the option labelled `name`. */
async function choose(page, host, name) {
  await page.locator(`${host} .custom-select`).click();
  await page.locator(".cs-dropdown .cs-option", { hasText: name }).click();
}

test("the introduction offers every language by its native name and reopens on its slide", async (t) => {
  const page = await openFirstRun(t);

  const picker = await page.evaluate(() => {
    const host = document.getElementById("onboarding-language");
    const glyph = host.querySelector(".language-picker-icon");
    const style = getComputedStyle(glyph);
    return {
      inHeader: host.parentElement.classList.contains("onboarding-hero"),
      options: [...host.querySelectorAll("select option")].map((option) => ({
        code: option.value,
        name: option.textContent,
        lang: option.lang,
      })),
      selected: host.querySelector("select").value,
      label: host.querySelector(".custom-select").getAttribute("aria-label"),
      glyph: { background: style.backgroundColor, border: style.borderStyle },
    };
  });
  assert.equal(picker.inHeader, true);
  assert.deepEqual(
    picker.options,
    [
      { code: "system", name: formatMessage("common-language-system", undefined, "en"), lang: "" },
      ...LOCALES.locales.map(({ code, name }) => ({ code, name, lang: code })),
    ]
  );
  assert.equal(picker.selected, LOCALES.setting);
  assert.equal(picker.label, formatMessage("common-language-label", undefined, "en"));
  assert.deepEqual(picker.glyph, { background: "rgba(0, 0, 0, 0)", border: "none" });

  await page.locator("#onboardingNext").click();
  await page.locator("#onboardingNext").click();
  await page.locator('.onboarding-slide.active[data-slide="2"]').waitFor();

  const posted = page.waitForRequest(
    (request) => request.url().endsWith("/api/i18n/language") && request.method() === "POST"
  );
  const reloaded = page.waitForEvent("load");
  await choose(page, "#onboarding-language", "فارسی");
  assert.deepEqual((await posted).postDataJSON(), { language: "fa" });
  await reloaded;

  await page.locator('.onboarding-slide.active[data-slide="2"]').waitFor();
  assert.equal(
    await page.evaluate(() => window.sessionStorage.getItem("screenerbot.onboarding.slide")),
    null,
    "the kept slide is read once"
  );
});

test("the setup wizard locks the language once a wallet source is chosen", async (t) => {
  const page = await openFirstRun(t);
  await page.evaluate(() => {
    document.getElementById("onboardingScreen").style.display = "none";
    document.getElementById("setupScreenWrapper").style.display = "block";
    window.SetupController.init();
  });
  const picker = page.locator("#setup-language");
  await picker.locator(".custom-select").waitFor({ state: "visible" });

  const state = () =>
    picker.evaluate((host) => ({
      disabled: host.querySelector(".custom-select").getAttribute("aria-disabled") === "true",
      title: host.getAttribute("title"),
    }));
  assert.deepEqual(await state(), { disabled: false, title: null });

  await page.locator('label[for="setup-wallet-source-import"]').click();
  assert.deepEqual(await state(), {
    disabled: true,
    title: formatMessage("common-language-locked", undefined, "en"),
  });
});

test("a refused language keeps the current one and says why", async (t) => {
  const page = await openFirstRun(t, {
    language: () => ({ status: 500, body: JSON.stringify(REFUSED) }),
  });
  let reloaded = false;
  page.on("load", () => (reloaded = true));

  await choose(page, "#onboarding-language", "Deutsch");
  await page.locator(".toast", { hasText: REFUSED.error.message }).waitFor();
  assert.equal(await page.locator("#onboarding-language select").inputValue(), LOCALES.setting);
  assert.equal(reloaded, false);
});
