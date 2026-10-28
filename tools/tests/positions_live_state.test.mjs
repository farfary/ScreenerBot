// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a position row in flight keeps the table's row height.
 *
 * The live state caption (Closing, Selling, Buying, Failed) once rendered as a third
 * line under the token name, and the uniform row height cut it in half. The caption
 * now takes the name line's place: the row's token cell has two lines, the caption
 * sits inside the row, and the row is as tall as a row at rest.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, positions] = await Promise.all([loadIndex("shell"), loadIndex("positions")]);
const OPEN = JSON.parse(readFileSync(`${FIXTURES_ROOT}/positions/positions_open.json`, "utf8"));
// An exit signature not yet verified is the Closing state.
const CLOSING = OPEN.map((row, index) =>
  index === 0 ? { ...row, exit_transaction_signature: "5".repeat(88) } : row
);

test("a closing position shows its state on the name line within the row", async (t) => {
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([positions, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const { pathname, searchParams } = request.url;
      if (pathname === "/api/positions" && searchParams.get("status") === "open") {
        return { body: JSON.stringify(CLOSING) };
      }
      return fixtures.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/positions`);
  await page.waitForSelector(PAGE_READY);
  await page.locator('#subTabsContainer [data-tab-id="open"]').click();
  await page.locator("#positions-root tr[data-row-id] .pos-state-closing").waitFor();

  const rows = await page.evaluate(() =>
    [...document.querySelectorAll("#positions-root tr[data-row-id]")].map((row) => {
      const box = row.getBoundingClientRect();
      const caption = row.querySelector(".pos-state-caption")?.getBoundingClientRect();
      return {
        height: Math.round(box.height),
        lines: row.querySelectorAll(".ti-row-cell__meta > *").length,
        captionInside: caption ? caption.bottom <= box.bottom + 0.5 : null,
      };
    })
  );
  const closing = rows.find((row) => row.captionInside !== null);
  const rest = rows.filter((row) => row !== closing);
  assert.equal(closing.captionInside, true, "the caption sits inside its row");
  assert.equal(closing.lines, 2, "symbol line and caption, no third line");
  for (const row of rest) assert.equal(row.height, closing.height, "one row height");
});
