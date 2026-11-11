// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: live row changes move rows smoothly and never under a pointer that guards its row.
 *
 * Positions arrive and leave while the viewer watches the Open book. A live change once
 * detached the whole body, dropped a leaving row at the top of the table and pushed the rows
 * below by a full row in one frame. Now:
 *
 * - A row that arrives fades in, and the rows it displaces slide; every row keeps its node.
 * - A row that leaves fades where it stands, then the rows below slide up into its gap.
 * - A change above a moving pointer waits; it lands once the pointer rests on plain cells,
 *   so a pointer parked over the table never hides new rows.
 * - A change above a pointer resting on a row control (sell, add, remove) waits until the
 *   pointer leaves the control: a click there must reach the row it was aimed at.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { longestIncreasingRun } from "../../src/webserver/templates/scripts/ui/data_table/row_motion.js";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, positions] = await Promise.all([loadIndex("shell"), loadIndex("positions")]);
const OPEN = JSON.parse(readFileSync(`${FIXTURES_ROOT}/positions/positions_open.json`, "utf8"));
const NO_ACTIONS = { actions: [], count: 0, in_progress: 0, completed: 0, failed: 0, unread: 0 };

/** A position opened now, so it sorts first in the newest-first Open book. */
const arrival = (id) => ({
  ...OPEN[0],
  id,
  mint: `ArrivalMint${id}`,
  symbol: `NEW${id}`,
  name: `Arrival ${id}`,
  entry_time: Math.floor(Date.now() / 1000),
});

test("the rows that keep their order are the longest increasing run", () => {
  assert.deepEqual([...longestIncreasingRun([0, 1, 2])].sort(), [0, 1, 2]);
  assert.deepEqual([...longestIncreasingRun([-1, 0, 1, 2])].sort(), [1, 2, 3]);
  assert.deepEqual([...longestIncreasingRun([2, 0, 1])].sort(), [1, 2]);
  assert.deepEqual([...longestIncreasingRun([])], []);
  assert.deepEqual([...longestIncreasingRun([-1, -1])], []);
});

/** The Open book on a server whose open list the test changes; polls pick the change up. */
async function openBook(t) {
  const book = { open: OPEN.map((row) => ({ ...row })) };
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 } });
  const fixtures = createApiHandler([positions, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      const { pathname, searchParams } = request.url;
      if (pathname === "/api/positions" && searchParams.get("status") === "open") {
        return { body: JSON.stringify(book.open) };
      }
      if (pathname === "/api/actions/active") return { body: JSON.stringify(NO_ACTIONS) };
      if (pathname === "/api/actions/stream") {
        return { contentType: "text/event-stream", body: ": connected\n\n" };
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
  await page.locator('#positions-root tr[data-row-id="102"]').waitFor();
  await page.evaluate(() => {
    window.__animated = [];
    const animate = window.Element.prototype.animate;
    window.Element.prototype.animate = function (keyframes, options) {
      if (this.matches?.("#positions-root tbody tr")) {
        const id = this.getAttribute("data-row-id") ?? this.getAttribute("data-dt-leaving-id");
        const properties = Object.keys(keyframes?.[0] ?? {}).filter((key) => key !== "offset");
        window.__animated.push({ id, properties });
      }
      return animate.call(this, keyframes, options);
    };
  });
  return { page, book };
}

const animated = (page) => page.evaluate(() => window.__animated.splice(0));
const rowIds = (page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("#positions-root tbody tr")].map(
      (tr) => tr.getAttribute("data-row-id") ?? `leaving:${tr.getAttribute("data-dt-leaving-id")}`
    )
  );

/** Centre of `selector` inside the Open table, for the pointer. */
async function centre(page, selector) {
  const box = await page.locator(`#positions-root ${selector}`).first().boundingBox();
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

/** Keep the pointer moving around `point` for `ms`, as a viewer aiming at a row does. */
async function aimAt(page, point, ms) {
  for (let elapsed = 0; elapsed < ms; elapsed += 250) {
    await page.mouse.move(point.x + (elapsed % 500 === 0 ? 1 : -1), point.y);
    await page.waitForTimeout(250);
  }
}

test("a row that arrives fades in and the rows below slide, keeping their nodes", async (t) => {
  const { page, book } = await openBook(t);
  await page.evaluate(() => {
    window.__kept = [...document.querySelectorAll("#positions-root tbody tr[data-row-id]")];
  });

  book.open.unshift(arrival(9001));
  await page.locator('#positions-root tr[data-row-id="9001"]').waitFor();

  assert.equal(
    await page.evaluate(() => window.__kept.every((tr) => tr.isConnected)),
    true,
    "every row kept its node"
  );
  const motion = await animated(page);
  const slid = motion.filter((m) => m.properties.includes("translate")).map((m) => m.id);
  assert.deepEqual(slid.sort(), ["100", "101", "102"], "the displaced rows slide");
  assert.ok(
    motion.some((m) => m.id === "9001" && m.properties.includes("opacity")),
    "the new row fades in"
  );
  assert.deepEqual(await rowIds(page), ["9001", "100", "101", "102"]);
});

test("a row that leaves fades in place, then the rows below close its gap", async (t) => {
  const { page, book } = await openBook(t);

  book.open = book.open.filter((row) => row.id !== 101);
  await page.locator('#positions-root tr[data-dt-leaving-id="101"]').waitFor({ state: "attached" });
  assert.deepEqual(await rowIds(page), ["100", "leaving:101", "102"], "it fades where it stood");
  assert.equal(
    await page.getAttribute('#positions-root tr[data-dt-leaving-id="101"]', "aria-hidden"),
    "true"
  );

  await page.waitForFunction(() => !document.querySelector("#positions-root tr.dt-row-leaving"));
  assert.deepEqual(await rowIds(page), ["100", "102"]);
  const slid = (await animated(page)).filter((m) => m.properties.includes("translate"));
  assert.ok(
    slid.some((m) => m.id === "102"),
    "the row below slides up into the gap"
  );
});

test("a change above a moving pointer waits and lands once the pointer rests", async (t) => {
  const { page, book } = await openBook(t);
  const plainCell = await centre(page, 'tr[data-row-id="102"] td:nth-child(5)');
  await page.mouse.move(plainCell.x, plainCell.y);

  book.open.unshift(arrival(9002));
  await aimAt(page, plainCell, 3000);
  assert.deepEqual(await rowIds(page), ["100", "101", "102"], "held while the pointer moves");

  await page.locator('#positions-root tr[data-row-id="9002"]').waitFor({ timeout: 5000 });
  assert.deepEqual(await rowIds(page), ["9002", "100", "101", "102"]);
});

test("a change above a pointer resting on a row control waits until it leaves the control", async (t) => {
  const { page, book } = await openBook(t);
  const plainCell = await centre(page, 'tr[data-row-id="102"] td:nth-child(5)');
  await page.mouse.move(plainCell.x, plainCell.y);
  const sell = await centre(page, 'tr[data-row-id="102"] .row-action[data-action="sell"]');
  await page.mouse.move(sell.x, sell.y);

  book.open.unshift(arrival(9003));
  await page.waitForTimeout(3500);
  assert.deepEqual(await rowIds(page), ["100", "101", "102"], "held while resting on the control");

  await page.mouse.move(plainCell.x, plainCell.y);
  await page.locator('#positions-root tr[data-row-id="9003"]').waitFor({ timeout: 5000 });
  assert.deepEqual(await rowIds(page), ["9003", "100", "101", "102"]);
});
