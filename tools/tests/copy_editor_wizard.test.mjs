// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Copy Trading wizard reports a problem at its field and keeps its footer.
 *
 * - A step that fails validation marks the field it names as invalid (`aria-invalid`,
 *   the error border, `aria-describedby` on the message) and moves focus to it; typing
 *   in the field clears the mark.
 * - The message sits on its own line above the footer buttons, and the buttons keep
 *   one row whatever the message length.
 * - The dialog is as tall as its step: a short step does not leave the box empty.
 * - Configured amounts and percentages read without padded fraction zeros.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const VALID_ADDRESS = "So11111111111111111111111111111111111111112";

/** The footer's geometry: the message box and each visible button's box. */
function footerState(page) {
  return page.evaluate(() => {
    const error = document.getElementById("copy-editor-error");
    const buttons = [...error.parentElement.querySelectorAll("button")].filter(
      (button) => button.offsetParent !== null
    );
    return {
      message: error.textContent.trim(),
      errorBottom: error.getBoundingClientRect().bottom,
      buttons: buttons.map((button) => {
        const box = button.getBoundingClientRect();
        return { id: button.id, top: box.top, bottom: box.bottom };
      }),
    };
  });
}

const panelHeight = (page) =>
  page.$eval(".copy-editor-panel", (panel) => panel.getBoundingClientRect().height);

test("the wizard marks the invalid field and keeps its footer on one row", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("copy"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector("body:not(.initialization-mode) main.content:not([data-loading])");
  await page.locator("#copy-add").click();
  await page.waitForSelector("#copy-editor:not(.hidden) [data-field='target_address']");

  const address = page.locator("[data-field='target_address']");
  await address.fill("not-an-address");
  await page.locator("#copy-editor-next").click();
  await page.waitForFunction(() => document.getElementById("copy-editor-error").textContent);
  // The border is compared once its colour transition has settled.
  await page.waitForFunction(
    () => document.querySelector("[data-field='target_address']").getAnimations().length === 0
  );

  const marked = await page.evaluate(() => {
    const input = document.querySelector("[data-field='target_address']");
    const probe = document.createElement("div");
    probe.style.borderTop = "1px solid var(--input-border-error)";
    document.body.append(probe);
    const errorColor = getComputedStyle(probe).borderTopColor;
    probe.remove();
    return {
      invalid: input.getAttribute("aria-invalid"),
      describedBy: input.getAttribute("aria-describedby"),
      focused: document.activeElement === input,
      errorBorder: getComputedStyle(input).borderTopColor === errorColor,
    };
  });
  assert.deepEqual(marked, {
    invalid: "true",
    describedBy: "copy-editor-error",
    focused: true,
    errorBorder: true,
  });

  const footer = await footerState(page);
  assert.ok(footer.message, "the step shows its message");
  assert.ok(footer.buttons.length >= 1);
  const firstTop = footer.buttons[0].top;
  for (const button of footer.buttons) {
    assert.ok(Math.abs(button.top - firstTop) <= 1, `${button.id} shares the button row`);
    assert.ok(footer.errorBottom <= button.top + 0.5, `the message sits above ${button.id}`);
  }

  await address.fill(VALID_ADDRESS);
  assert.equal(await address.getAttribute("aria-invalid"), null, "typing clears the mark");

  const shortStep = await panelHeight(page);
  await page.locator("#copy-editor-next").click();
  await page.waitForSelector("#copy-editor-body [data-field='sizing_amount']");
  const sizingStep = await panelHeight(page);
  assert.ok(
    sizingStep > shortStep + 40,
    `the dialog follows its step (${shortStep}px -> ${sizingStep}px)`
  );

  const body = await page.$eval("#copy-editor-body", (node) => node.innerText);
  const padded = body.match(/\d\.\d*0(?!\d)\s*(?:SOL|%)/g) ?? [];
  assert.deepEqual(padded, [], "configured figures carry no padded zeros");
});
