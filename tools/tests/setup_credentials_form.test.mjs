// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the first-run setup step and the Explore Mode setup dialog ask for the
 * wallet and RPC with one form.
 *
 * Both mount the shared credential fields, so their titles, placeholders, hints,
 * reveal toggle and provider links read the same, the RPC hint says public endpoints
 * are not supported, and the dialog's key icon is a bare glyph without a tile. The
 * dialog sizes to its content: with no status to show, the RPC card is followed by
 * the body's own padding and then the buttons, with no empty band between them.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");
const BOOTSTRAP = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/system_bootstrap.json`, "utf8"));
const FIRST_RUN = {
  ...BOOTSTRAP,
  initialization_required: true,
  initialization_complete: false,
  onboarding_complete: false,
  ui_ready: false,
};

/** The visible text and attributes that make up a credential form under `root`. */
function readForm(root) {
  const text = (node) => (node ? node.textContent.replace(/\s+/g, " ").trim() : null);
  const form = document.querySelector(root);
  return {
    titles: [...form.querySelectorAll(".setup-card-title")].map(text),
    placeholders: [...form.querySelectorAll("textarea")].map((input) => input.placeholder),
    notes: [...form.querySelectorAll(".setup-security-note, .setup-rpc-guidance")].map(text),
    toggles: form.querySelectorAll(".setup-input-toggle").length,
    links: [...form.querySelectorAll("a[href]")].map((link) => link.href),
  };
}

test("onboarding and the Explore Mode dialog render the same credential form", async () => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([shell], "populated");
  const onApi = (request) =>
    request.url.pathname === "/api/system/bootstrap"
      ? { body: JSON.stringify(FIRST_RUN) }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  try {
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/`);
    await page.locator("#onboardingScreen").waitFor({ state: "visible" });
    await page.evaluate(() => {
      document.getElementById("setupScreenWrapper").style.display = "block";
      window.SetupController.init();
    });
    await page.waitForSelector("#setup-credential-fields textarea");
    await page.waitForFunction(
      () => document.querySelector("#setup-credential-fields .setup-card-title")?.textContent
    );
    const onboarding = await page.evaluate(readForm, "#setup-credential-fields");

    await page.evaluate(async () => {
      const { SetupDialog } = await import("/scripts/ui/setup_dialog.js");
      void SetupDialog.show();
    });
    await page.waitForFunction(
      () => document.querySelector(".setup-dialog-fields .setup-card-title")?.textContent
    );
    const dialog = await page.evaluate(readForm, ".setup-dialog-fields");

    assert.deepEqual(dialog, onboarding);
    assert.equal(onboarding.titles.length, 2);
    assert.equal(onboarding.toggles, 1);
    assert.ok(
      onboarding.notes.some((note) => note.includes("public endpoints are not supported")),
      onboarding.notes.join(" | ")
    );

    const ids = await page.evaluate(() =>
      [...document.querySelectorAll("textarea")].map((input) => input.id)
    );
    assert.equal(new Set(ids).size, ids.length, `credential ids collide: ${ids.join(", ")}`);

    const icon = await page.evaluate(() => {
      const glyph = document.querySelector(".setup-dialog-title i");
      const style = getComputedStyle(glyph);
      const parent = getComputedStyle(glyph.parentElement);
      return {
        inTitle: glyph.parentElement.classList.contains("setup-dialog-title"),
        background: style.backgroundColor,
        border: style.borderStyle,
        parentBackground: parent.backgroundColor,
      };
    });
    assert.equal(icon.inTitle, true, "the key glyph sits inline in the title");
    assert.equal(icon.background, "rgba(0, 0, 0, 0)");
    assert.equal(icon.border, "none");
    assert.equal(icon.parentBackground, "rgba(0, 0, 0, 0)");

    const band = await page.evaluate(() => {
      const body = document.querySelector(".setup-dialog-body");
      const fields = document.querySelector(".setup-dialog-fields").getBoundingClientRect();
      const footer = document.querySelector(".setup-dialog-footer").getBoundingClientRect();
      return {
        statusBoxes: document.querySelector("#setup-dialog-status").getClientRects().length,
        gap: Math.round(footer.top - fields.bottom),
        padding: Math.round(parseFloat(getComputedStyle(body).paddingBottom)),
      };
    });
    assert.equal(band.statusBoxes, 0, "an idle status line takes no space");
    assert.equal(band.gap, band.padding, "the buttons follow the RPC card after the body padding");
  } finally {
    await context.close();
    await host.close();
  }
});
