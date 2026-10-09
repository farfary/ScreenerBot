// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading summary figure is one line. A value with its unit
 * ("2.54 / 8 SOL") never wraps the unit under the amount and never runs out of its
 * card, at the narrowest desktop window and wider, in left-to-right and right-to-left
 * locales. The task list's budget line follows the same rule.
 *
 * A free-standing SOL amount follows the one trim rule of core/format.js: a budget beside a
 * P&L reads "0 / 2 SOL" next to "0 SOL", never "0.00 / 2.00 SOL".
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";

import { chromium } from "playwright";

import { SCRIPTS_ROOT } from "../lib/dashboard_ui.mjs";
import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const VIEWS = [
  [1200, "en"],
  [1440, "en"],
  [1200, "de"],
  [1200, "fa"],
];

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [copy, shell] = await Promise.all([loadIndex("copy"), loadIndex("shell")]);

for (const [width, locale] of VIEWS) {
  test(`copy figures stay on one line at ${width}px (${locale})`, async (t) => {
    const context = await browser.newContext({ viewport: { width, height: 900 }, locale });
    const api = createApiHandler([copy, shell], "populated");
    const host = await serveDashboard(context, { locale, onApi: api.onApi });
    t.after(async () => {
      await context.close();
      await host.close();
    });
    const page = await context.newPage();
    await page.goto(`${host.origin}/copy`);
    await page.waitForSelector("#copy-figures .copy-figure");
    const values = await page.$$eval(
      "#copy-figures .copy-figure-value, .copy-row-budget > span:last-child",
      (nodes) =>
        nodes.map((node) => {
          const style = getComputedStyle(node);
          const line = parseFloat(style.lineHeight) || parseFloat(style.fontSize) * 1.6;
          return {
            text: node.textContent,
            height: node.getBoundingClientRect().height,
            line,
            overflow: node.scrollWidth - node.clientWidth,
          };
        })
    );
    assert.ok(
      values.some((value) => value.text.includes("/")),
      "the budget figure is covered"
    );
    for (const value of values) {
      assert.ok(
        value.height <= value.line * 1.5,
        `"${value.text}" wraps: ${JSON.stringify(value)}`
      );
      assert.ok(value.overflow <= 0, `"${value.text}" runs out of its card`);
    }
  });
}

test("every copy P&L figure has one SOL precision", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([copy, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(".copy-row-pnl");
  await page.waitForSelector("#copy-figures .copy-figure");
  const figures = await page.$$eval(".copy-row-pnl, #copy-figures .copy-figure-value", (nodes) =>
    // Signed figures are the P&L ones; a budget ("2.54 / 8 SOL") is not a P&L.
    nodes.map((node) => node.textContent.trim()).filter((text) => /^[+\-−].*SOL/.test(text))
  );
  const decimals = new Set(figures.map((text) => /[.,](\d+)\s*SOL/.exec(text)?.[1].length ?? 0));
  assert.ok(
    figures.some((text) => text),
    "the list and the cards print SOL figures"
  );
  assert.deepEqual([...decimals], [4], `one precision across ${figures.join(" | ")}`);
});

test("no copy P&L call picks its own precision", async () => {
  const directory = resolve(SCRIPTS_ROOT, "pages/copy");
  const offenders = [];
  for (const name of await readdir(directory)) {
    const source = await readFile(resolve(directory, name), "utf8");
    for (const match of source.matchAll(/signedSol\([^()]*(?:\([^()]*\)[^()]*)*,/g)) {
      offenders.push(`${name}: ${match[0]}`);
    }
  }
  assert.deepEqual(offenders, [], "signedSol takes the one P&L precision");
});

test("a free-standing copy SOL amount carries no padded fraction zeros", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([copy, shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  await page.goto(`${host.origin}/copy`);
  await page.waitForSelector(".copy-row-budget");
  const amounts = await page.$$eval(".copy-row-budget > span:last-child", (nodes) =>
    nodes.map((node) => node.textContent.replace(/[\u2066-\u2069]/g, "").trim())
  );
  assert.ok(amounts.length > 0, "the task list prints budget lines");
  const padded = amounts.filter((text) => /\d[.,]\d*0(?!\d)/.test(text));
  assert.deepEqual(padded, [], `budget lines ${amounts.join(" | ")}`);
});

test("no copy SOL amount is formatted as a plain fixed number", async () => {
  const directory = resolve(SCRIPTS_ROOT, "pages/copy");
  const offenders = [];
  for (const name of await readdir(directory)) {
    const source = await readFile(resolve(directory, name), "utf8");
    for (const match of source.matchAll(/\bfixed\([^()]*(?:native|spent|budget)[^()]*\)/g)) {
      offenders.push(`${name}: ${match[0]}`);
    }
  }
  assert.deepEqual(offenders, [], "a SOL amount takes solNumber, sol or a cell formatter");
});
