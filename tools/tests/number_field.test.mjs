// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the numeric field's step arithmetic (`ui/number_field.js`).
 *
 * The stepper cannot delegate to `HTMLInputElement.stepUp()`: it throws on
 * `step="any"` and on an empty value, and float addition walks a trade size to
 * 0.30000000000000004. Since a stepped field writes a real config value — a
 * trade size, a slippage, a stop loss — the arithmetic is asserted here rather
 * than trusted to a browser API we do not use.
 *
 * Layout: the suffix and the gutter reserved for it sit on the same physical side
 * in both page directions, also when a page rule sets the `padding` shorthand and
 * when the direction changes in place; a field with a prefix stays left to right.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { chromium } from "playwright";

import { SCRIPTS_ROOT, STYLES_ROOT } from "../lib/dashboard_ui.mjs";
import { stepNumber } from "../../src/webserver/templates/scripts/ui/number_field.js";

test("steps by the field's own step, in both directions", () => {
  assert.equal(stepNumber("5", { step: "0.5" }, 1), 5.5);
  assert.equal(stepNumber("5", { step: "0.5" }, -1), 4.5);
});

test("a field without a step moves by one", () => {
  assert.equal(stepNumber("41", {}, 1), 42);
  assert.equal(stepNumber("41", { step: "any" }, -1), 40);
});

test("keeps the step's precision instead of accumulating float dust", () => {
  assert.equal(stepNumber("0.1", { step: "0.2" }, 1), 0.3);
  assert.equal(stepNumber("0.005", { step: "0.001" }, 1), 0.006);
});

test("clamps to the field's own bounds", () => {
  assert.equal(stepNumber("100", { step: "1", max: "100" }, 1), 100);
  assert.equal(stepNumber("0.5", { step: "1", min: "0.5" }, -1), 0.5);
});

test("an empty field starts at its floor, not at zero", () => {
  assert.equal(stepNumber("", { step: "0.001", min: "0.001" }, 1), 0.001);
  assert.equal(stepNumber("", { step: "1" }, 1), 0);
  assert.equal(stepNumber("", { step: "1", max: "-5" }, 1), -5);
});

test("an unparseable value is treated as unset rather than as NaN", () => {
  assert.equal(stepNumber("abc", { step: "1", min: "10" }, 1), 10);
  assert.equal(Number.isFinite(stepNumber("abc", { step: "1" }, -1)), true);
});

test("a zero or negative step attribute cannot freeze the field", () => {
  assert.equal(stepNumber("7", { step: "0" }, 1), 8);
  assert.equal(stepNumber("7", { step: "-3" }, 1), 8);
});

const FIELD_STYLES = await Promise.all(
  ["base/direction.css", "components/form_controls.css"].map((path) =>
    readFile(resolve(STYLES_ROOT, path), "utf8")
  )
);
const ENHANCER = `${await readFile(resolve(SCRIPTS_ROOT, "ui/number_field.js"), "utf8")}
installGlobalNumberFieldEnhancer();
window.__numberFieldReady = true;`;

/** A page rule that sets the `padding` shorthand on its own number inputs. */
const PAGE_RULE = ".probe input { padding: 6px 12px; width: 100%; }";
const FIELDS = `<div class="probe" style="width: 240px">
  <input id="unit" type="number" value="-0.5"><span class="input-unit">SOL</span>
  <input id="prefixed-unit" type="number" value="12"><span class="input-unit">%</span>
</div>`;

async function openFields(browser, dir) {
  const page = await browser.newPage({ viewport: { width: 800, height: 400 } });
  await page.setContent(`<!doctype html><html dir="${dir}"><body>${FIELDS}</body></html>`);
  await page.evaluate(() => {
    const field = document.getElementById("prefixed-unit");
    const prefix = document.createElement("span");
    prefix.className = "input-prefix";
    prefix.textContent = "\u2212";
    field.before(prefix);
  });
  for (const content of [...FIELD_STYLES, PAGE_RULE]) await page.addStyleTag({ content });
  await page.addScriptTag({ type: "module", content: ENHANCER });
  await page.waitForFunction(() => window.__numberFieldReady === true);
  return page;
}

/** Where a field's suffix sits and which side of its input carries the gutter. */
function fieldLayout(page, id) {
  return page.evaluate(async (id) => {
    await new Promise((done) => requestAnimationFrame(() => done()));
    const input = document.getElementById(id);
    const shell = input.closest(".number-field");
    const box = input.getBoundingClientRect();
    const suffix = shell.querySelector(".number-field-suffix").getBoundingClientRect();
    const style = getComputedStyle(input);
    const left = parseFloat(style.paddingLeft);
    const right = parseFloat(style.paddingRight);
    return {
      suffixSide: suffix.left + suffix.width / 2 < box.left + box.width / 2 ? "left" : "right",
      gutterSide: left > right ? "left" : "right",
      shellDirection: getComputedStyle(shell).direction,
      inputDirection: style.direction,
      textAlign: style.textAlign,
    };
  }, id);
}

test("the suffix and its gutter share a side in both page directions", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());

  for (const [dir, side] of [
    ["ltr", "right"],
    ["rtl", "left"],
  ]) {
    const page = await openFields(browser, dir);
    const layout = await fieldLayout(page, "unit");
    assert.equal(layout.suffixSide, side, `${dir}: the suffix side`);
    assert.equal(layout.gutterSide, side, `${dir}: the gutter side`);
    assert.equal(layout.inputDirection, "ltr", `${dir}: the value keeps left-to-right order`);
    if (dir === "rtl") assert.equal(layout.textAlign, "right", "the value sits against the right edge");
    await page.close();
  }
});

test("a field with a prefix stays left to right in a right-to-left page", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());

  const page = await openFields(browser, "rtl");
  const layout = await fieldLayout(page, "prefixed-unit");
  assert.equal(layout.shellDirection, "ltr");
  assert.equal(layout.suffixSide, "right");
  assert.equal(layout.gutterSide, "right");
  await page.close();
});

test("the gutter follows a direction changed in place", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());

  const page = await openFields(browser, "ltr");
  await page.evaluate(() => {
    document.documentElement.dir = "rtl";
  });
  const layout = await fieldLayout(page, "unit");
  assert.equal(layout.suffixSide, "left");
  assert.equal(layout.gutterSide, "left");
  await page.close();
});
