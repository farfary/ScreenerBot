// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: `core/brand_text.js` sets the product name in the brand face only in
 * interface copy, never inside a machine value.
 *
 * A path, address or code sample is copied verbatim and read as one value. With the
 * name rewritten into `.brand-name`, the settings data path
 * ".../Application Support/ScreenerBot/data" switched typeface mid-path and read as
 * a broken font.
 *
 * - A name in plain copy gains one `.brand-name` span.
 * - A name inside a left-to-right island (`dir="ltr"`), `code`, `pre`, `kbd`,
 *   `samp` or `.font-mono` stays plain text, whether rendered up front or added later.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { chromium } from "playwright";

import { SCRIPTS_ROOT } from "../lib/dashboard_ui.mjs";

const brandText = await readFile(resolve(SCRIPTS_ROOT, "core/brand_text.js"), "utf8");

const MACHINE_VALUES = [
  '<div dir="ltr">/Users/a/Library/Application Support/ScreenerBot/data</div>',
  "<code>ScreenerBot/data/config.toml</code>",
  "<pre>ScreenerBot --help</pre>",
  "<kbd>ScreenerBot</kbd>",
  "<samp>ScreenerBot ready</samp>",
  '<span class="font-mono">ScreenerBot/logs</span>',
  '<div dir="ltr"><span>~/ScreenerBot</span></div>',
];

test("the brand face stays out of machine values", async (t) => {
  const browser = await chromium.launch({ headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();

  const markup = (values) =>
    `<p id="copy">Open the folder containing all ScreenerBot data</p>` +
    values.map((value, index) => `<section id="v${index}">${value}</section>`).join("");
  await page.setContent(
    `<!doctype html><html><body>${markup(MACHINE_VALUES)}<script>${brandText}</script></body></html>`
  );

  const inspect = () =>
    page.evaluate(
      (count) => ({
        copy: document.querySelectorAll("#copy .brand-name").length,
        values: Array.from({ length: count }, (_, index) => {
          const section = document.getElementById(`v${index}`);
          return section.querySelectorAll(".brand-name").length;
        }),
      }),
      MACHINE_VALUES.length
    );

  const initial = await inspect();
  assert.equal(initial.copy, 1, "plain copy carries the brand face");
  assert.deepEqual(
    initial.values,
    MACHINE_VALUES.map(() => 0)
  );

  await page.evaluate((values) => {
    document.body.innerHTML = values
      .map((value, index) => `<section id="v${index}">${value}</section>`)
      .join("");
    const copy = document.createElement("p");
    copy.id = "copy";
    copy.textContent = "Starting ScreenerBot";
    document.body.prepend(copy);
  }, MACHINE_VALUES);
  await page.evaluate(() => new Promise((done) => setTimeout(done, 0)));

  const added = await inspect();
  assert.equal(added.copy, 1, "copy added later carries the brand face");
  assert.deepEqual(
    added.values,
    MACHINE_VALUES.map(() => 0)
  );
});
