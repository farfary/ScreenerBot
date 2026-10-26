// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the unsaved-changes question of Settings reads cleanly in every locale.
 *
 * In a 440px dialog "Keep editing" and "Save Changes" wrapped to two lines each, and
 * Save wore the warning amber of the dialog's variant.
 *
 * - At a 1200px window, in every shipped locale, no action label wraps: the three
 *   actions share one line, or the dialog stacks them one per row at full width.
 * - Save fills with the shared primary button colour.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readdir } from "node:fs/promises";
import { resolve } from "node:path";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { REPO_ROOT } from "../lib/dashboard_ui.mjs";

const LOCALES = (await readdir(resolve(REPO_ROOT, "locales"), { withFileTypes: true }))
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name)
  .sort();

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const indexes = [await loadIndex("home"), await loadIndex("shell")];

for (const locale of LOCALES) {
  test(`${locale}: the unsaved-changes actions never wrap and Save is primary`, async (t) => {
    const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
    const api = createApiHandler(indexes, "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    page.setDefaultTimeout(15000);
    await page.goto(`${host.origin}/home`);
    await page.waitForSelector(PAGE_READY);

    await page.evaluate(async () => {
      const { ConfirmationDialog } = await import("/scripts/ui/confirmation_dialog.js");
      void ConfirmationDialog.show({
        title: I18n.t("settings-dialog-unsaved-title"),
        message: I18n.t("settings-dialog-unsaved-message"),
        confirmLabel: I18n.t("settings-dialog-save"),
        alternateLabel: I18n.t("settings-dialog-unsaved-discard"),
        cancelLabel: I18n.t("settings-dialog-unsaved-keep"),
        variant: "warning",
        confirmTone: "primary",
      });
    });
    await page.waitForSelector(".confirmation-dialog--visible");

    const layout = await page.evaluate(() => {
      const footer = document.querySelector(".confirmation-dialog__footer");
      const buttons = [...footer.querySelectorAll(".confirmation-dialog__button")];
      const lines = (button) => {
        const range = document.createRange();
        range.selectNodeContents(button);
        return new Set([...range.getClientRects()].map((rect) => Math.round(rect.top))).size;
      };
      const probe = document.createElement("button");
      probe.className = "btn btn-primary";
      document.body.appendChild(probe);
      const primary = getComputedStyle(probe).backgroundColor;
      probe.remove();
      return {
        stacked: footer.classList.contains("confirmation-dialog__footer--stacked"),
        overflow: footer.scrollWidth > footer.clientWidth,
        footerWidth: Math.round(footer.getBoundingClientRect().width),
        buttons: buttons.map((button) => ({
          label: button.textContent.trim(),
          lines: lines(button),
          top: Math.round(button.getBoundingClientRect().top),
          width: Math.round(button.getBoundingClientRect().width),
        })),
        save: getComputedStyle(footer.querySelector('[data-action="confirm"]')).backgroundColor,
        primary,
      };
    });

    for (const button of layout.buttons) {
      assert.equal(button.lines, 1, `${locale}: "${button.label}" wraps`);
    }
    assert.equal(layout.overflow, false, `${locale}: the actions overflow the dialog`);
    if (layout.stacked) {
      for (const button of layout.buttons) {
        assert.equal(button.width, layout.footerWidth, `${locale}: a stacked action is full width`);
      }
    } else {
      assert.equal(
        new Set(layout.buttons.map((button) => button.top)).size,
        1,
        `${locale}: the actions share one line`
      );
    }
    assert.equal(layout.save, layout.primary, `${locale}: Save is the primary button`);
  });
}
