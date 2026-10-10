// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the event details dialog uses the shared dialog chrome.
 *
 * The dialog drew its own "×" close with a red hover, a second Close button in the
 * footer, and a pill-shaped blue Copy button with inline SVG glyphs, so it looked
 * and behaved unlike every other dialog.
 *
 * - One close control, the shared `.modal-close` with the `icon-x` glyph.
 * - The footer holds the Copy action alone, as a shared secondary button with the
 *   same corner radius as any other `.btn`.
 * - The dialog stylesheet gives the Copy action no frame of its own.
 * - Nothing is said twice: a message that fits the title is not repeated in the
 *   body, the metadata is a frameless key-value list without the severity,
 *   category and subtype the header already shows, and one time is listed.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";
import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const DIALOG = ".events-dialog-overlay.is-visible";

test("the event dialog has one shared close and a standard Copy button", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([await loadIndex("events"), await loadIndex("shell")], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(() => host.close());
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/events`);
  await page.waitForSelector(PAGE_READY);
  await page.locator("#events-root tbody tr[data-row-id]").first().click();
  await page.waitForSelector(DIALOG);

  const chrome = await page.evaluate((selector) => {
    const dialog = document.querySelector(selector);
    const closes = [...dialog.querySelectorAll('[data-action="close"]')];
    const footerButtons = [...dialog.querySelectorAll(".events-dialog-footer button")];
    const reference = document.createElement("button");
    reference.className = "btn btn-secondary";
    document.body.append(reference);
    const radius = getComputedStyle(reference).borderStartStartRadius;
    reference.remove();
    return {
      closes: closes.map((node) => ({
        shared: node.classList.contains("modal-close"),
        glyph: Boolean(node.querySelector("i.icon-x")),
        label: node.getAttribute("aria-label"),
      })),
      footer: footerButtons.map((node) => ({
        classes: node.className,
        radius: getComputedStyle(node).borderStartStartRadius,
        glyph: Boolean(node.querySelector("i.icon-copy")),
      })),
      radius,
      svg: dialog.querySelectorAll("header svg, footer svg").length,
      title: dialog.querySelector(".events-dialog-title").textContent,
      message: dialog.querySelector(".events-dialog-message").checkVisibility()
        ? dialog.querySelector(".events-dialog-message").textContent
        : null,
      fields: [...dialog.querySelectorAll(".events-dialog-field")].map((field) => ({
        label: field.querySelector("dt").textContent,
        value: field.querySelector("dd").textContent.trim(),
        display: getComputedStyle(field).display,
      })),
    };
  }, DIALOG);

  assert.deepEqual(chrome.closes, [{ shared: true, glyph: true, label: "Close dialog" }]);
  assert.equal(chrome.footer.length, 1);
  assert.match(chrome.footer[0].classes, /\bbtn btn-secondary\b/);
  assert.equal(chrome.footer[0].glyph, true);
  assert.equal(chrome.footer[0].radius, chrome.radius);
  assert.equal(chrome.svg, 0);

  assert.notEqual(chrome.message, chrome.title, "the title message is not repeated");
  const labels = chrome.fields.map((field) => field.label);
  assert.deepEqual(
    labels.filter((label) => ["Severity", "Category", "Subtype", "Age"].includes(label)),
    []
  );
  assert.ok(labels.includes("Event Time"));
  const times = chrome.fields
    .filter((field) => field.label === "Event Time" || field.label === "Created")
    .map((field) => field.value.slice(0, 24));
  assert.equal(new Set(times).size, times.length, `one time per moment: ${times}`);
  for (const field of chrome.fields) {
    assert.equal(field.display, "contents", `${field.label} has no card of its own`);
  }

  await page.locator(`${DIALOG} .modal-close`).click();
  await page.waitForSelector(DIALOG, { state: "detached" });
});

test("the dialog stylesheet gives the Copy action no frame of its own", () => {
  const rules = rulesIn(readFileSync(`${STYLES_ROOT}/ui/events_dialog.css`, "utf8")).filter(
    ({ selector }) => /\.events-dialog-(?:copy|dismiss)\b/.test(selector)
  );
  for (const { selector, body } of rules) {
    assert.doesNotMatch(
      body,
      /(?:^|;)\s*(?:background|border|border-radius|padding|box-shadow|outline)\b/,
      selector
    );
  }
});
