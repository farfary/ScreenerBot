// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Transactions, Positions and Main Wallet tables read correctly at the
 * 1200px window floor.
 *
 * At 1200px the Transactions Router and Instr. columns and the Positions P&L columns
 * sat off-screen behind wide minimums; the Time cell wrapped into "Oct 7, 2026," over
 * "03:15:00 AM"; a failed transaction printed FAILED under both Type and Status; Δ SOL
 * padded six decimals; a bare "Estimate" count sat beside the total; the position
 * origin tag ran into the token name; and the Main Wallet chip printed an absolute
 * timestamp because it passed an option the timestamp formatter does not have.
 *
 * - Transactions: every column but the signature is on screen, the Time cell is one
 *   line, a failed row's Type is absent, Δ SOL carries no padded decimals and the
 *   toolbar has no separate estimate count. Direction is a neutral badge in every row,
 *   so a normal buy never shows a red "Outgoing" beside a green status.
 * - Positions Open and Closed: both P&L columns are on screen and headed "P&L", and
 *   the two prices of a row show the same digits.
 * - A position origin tag is set apart from the name as an uppercase caption.
 * - Main Wallet: "Last used" is relative, the toolbar stays on one row and a token
 *   balance carries no padded zeros.
 * - Copy Trading Compare: a paused task's mode is shown whole, and each curve in the
 *   chart legend is named as the table names its task ("Unnamed task", not a wallet).
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const READY = "body:not(.initialization-mode) main.content:not([data-loading])";
const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const shell = await loadIndex("shell");

async function openPage(t, name) {
  const context = await browser.newContext({ viewport: { width: 1200, height: 800 } });
  const api = createApiHandler([await loadIndex(name), shell], "populated");
  const host = await serveDashboard(context, { locale: "en", onApi: api.onApi });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/${name}`);
  await page.waitForSelector(READY);
  return page;
}

/** Each visible column's header text and whether its right edge is inside the table viewport. */
function columnsOnScreen(page, root) {
  return page.$eval(root, (el) => {
    const viewport = el.querySelector(".data-table-scroll-container").getBoundingClientRect();
    return [...el.querySelectorAll("thead th[data-column-id]")]
      .filter((th) => th.offsetParent !== null)
      .map((th) => ({
        id: th.dataset.columnId,
        label: th.textContent.trim(),
        visible: Math.round(th.getBoundingClientRect().right) <= Math.round(viewport.right),
      }));
  });
}

function assertOnScreen(columns, ids, where) {
  for (const id of ids) {
    const column = columns.find((entry) => entry.id === id);
    assert.ok(column, `${where}: ${id} renders`);
    assert.ok(column.visible, `${where}: ${id} is on screen at 1200px`);
  }
}

/** The digits after the point (or after the subscript zero count) of a price cell. */
function priceDigits(text) {
  const sub = text.match(/^-?0\.0([₀-₉]+)(\d+)$/);
  if (sub) return { notation: `sub${sub[1]}`, digits: sub[2].length };
  const plain = text.match(/^-?[\d,]+(?:\.(\d+))?$/);
  return plain ? { notation: "plain", digits: plain[1]?.length ?? 0 } : null;
}

/** Two prices in one row are padded to the same digits when they share a notation. */
async function assertPricesAligned(page, root, first, second) {
  const pairs = await page.$$eval(
    `${root} tbody tr[data-row-id]`,
    (rows, ids) =>
      rows.map((row) =>
        ids.map((id) => row.querySelector(`td[data-column-id="${id}"]`)?.textContent.trim())
      ),
    [first, second]
  );
  let compared = 0;
  for (const [a, b] of pairs) {
    const [left, right] = [priceDigits(a ?? ""), priceDigits(b ?? "")];
    if (!left || !right || left.notation !== right.notation) continue;
    compared += 1;
    assert.equal(left.digits, right.digits, `"${a}" and "${b}" show the same digits`);
  }
  assert.ok(compared > 0, `${first} and ${second} rows share a notation`);
}

test("Transactions keeps its columns on screen and states each fact once", async (t) => {
  const page = await openPage(t, "transactions");
  const root = "#transactions-root";
  await page.waitForSelector(`${root} tbody tr[data-row-id]`);

  const columns = await columnsOnScreen(page, root);
  assertOnScreen(
    columns,
    columns.filter((column) => column.id !== "signature").map((column) => column.id),
    "Transactions"
  );

  const cells = await page.$$eval(`${root} tbody tr[data-row-id]`, (rows) =>
    rows.map((row) => {
      const cell = (id) => row.querySelector(`td[data-column-id="${id}"]`);
      return {
        timeWrap: getComputedStyle(cell("timestamp")).whiteSpace,
        type: cell("transaction_type").textContent.trim(),
        status: cell("status").textContent.trim(),
        delta: cell("native_delta").textContent.trim(),
        direction: cell("direction").querySelector(".badge")?.className ?? null,
      };
    })
  );
  assert.ok(cells.length > 0, "transaction rows render");
  for (const cell of cells) {
    assert.equal(cell.timeWrap, "nowrap", "the Time cell never wraps");
    assert.doesNotMatch(cell.delta, /\d\.\d{5,}/, `Δ SOL "${cell.delta}" pads no decimals`);
    assert.equal(cell.direction, "badge secondary", "Direction is a neutral badge");
    if (/failed/i.test(cell.status)) {
      assert.equal(cell.type, "—", "a failed row leaves its Type to the Status column");
    }
  }
  assert.ok(
    cells.some((cell) => /failed/i.test(cell.status)),
    "the fixture carries a failed row"
  );
  const summary = await page.$$eval(`${root} [data-summary-id]`, (chips) =>
    chips.map((chip) => chip.dataset.summaryId)
  );
  assert.ok(summary.includes("tx-total"), "the toolbar states the total");
  assert.equal(
    await page.$eval(`${root} [data-summary-id="tx-failed"]`, (chip) => chip.dataset.variant),
    "error",
    "a failure count takes the error tone"
  );
  assert.ok(!summary.includes("tx-estimate"), "no separate estimate count");
});

test("Positions Open and Closed keep P&L on screen under one term", async (t) => {
  const page = await openPage(t, "positions");
  const root = "#positions-root";
  await page.waitForSelector(`${root} tbody tr[data-row-id]`);
  const open = await columnsOnScreen(page, root);
  assertOnScreen(open, ["unrealized_pnl", "unrealized_pnl_percent"], "Positions Open");
  await assertPricesAligned(page, root, "average_entry_price", "current_price");

  await page.click(tab("closed"));
  await page.waitForSelector(`${root} th[data-column-id="pnl"]`);
  await page.waitForSelector(`${root} tbody tr[data-row-id]`);
  const closed = await columnsOnScreen(page, root);
  assertOnScreen(closed, ["pnl", "pnl_percent"], "Positions Closed");
  await assertPricesAligned(page, root, "average_entry_price", "average_exit_price");

  for (const column of [...open, ...closed]) {
    assert.doesNotMatch(column.label, /PnL/i, `${column.id} spells P&L`);
  }
});

test("a position origin tag reads as a tag, not as part of the name", async (t) => {
  const page = await openPage(t, "positions");
  const tag = page.locator("#positions-root .position-origin-label").first();
  await tag.waitFor();
  const style = await tag.evaluate((el) => {
    const own = getComputedStyle(el);
    return { transform: own.textTransform, spacing: own.letterSpacing };
  });
  assert.equal(style.transform, "uppercase");
  assert.notEqual(style.spacing, "normal");
});

test("Main Wallet states last use relatively, on one toolbar row, with unpadded balances", async (t) => {
  const page = await openPage(t, "wallets");
  const root = "#tokens-datatable-root";
  await page.waitForSelector(`${root} tbody tr[data-row-id]`);

  const chip = await page.locator(`${root} [data-summary-id="wt-last-used"]`).textContent();
  assert.doesNotMatch(chip, /\b20\d\d\b/, "Last used carries no absolute year");
  assert.match(chip, /ago|now/i, "Last used is relative");

  const tops = await page.$eval(`${root} .table-toolbar`, (bar) =>
    [".table-toolbar-identity", ".table-toolbar-search", ".table-toolbar__actions"].map((sel) =>
      Math.round(bar.querySelector(sel).getBoundingClientRect().top)
    )
  );
  assert.ok(Math.max(...tops) - Math.min(...tops) < 16, `toolbar stays on one row (${tops})`);

  const balances = await page.$$eval(`${root} td[data-column-id="ui_amount"]`, (tds) =>
    tds.map((td) => td.textContent.trim())
  );
  assert.ok(balances.length > 0, "token balances render");
  for (const balance of balances) {
    assert.doesNotMatch(balance, /\.\d*0$/, `balance "${balance}" pads no zeros`);
  }
});

test("Copy Trading Compare shows each mode whole and names curves as the table does", async (t) => {
  const page = await openPage(t, "copy");
  await page.click("#copy-compare-open");
  const root = "#copy-compare-table";
  await page.waitForSelector(`${root} tbody tr[data-row-id]`);

  const modes = await page.$$eval(`${root} td[data-column-id="mode"]`, (tds) =>
    tds.map((td) => ({
      text: td.textContent.trim().replace(/\s+/g, " "),
      cut: td.scrollWidth > td.clientWidth + 1,
    }))
  );
  assert.ok(
    modes.some((mode) => /paused/i.test(mode.text)),
    "a paused task is in the fixture"
  );
  for (const mode of modes) assert.equal(mode.cut, false, `mode "${mode.text}" is cut`);

  const names = await page.$$eval(`${root} .ti-named-address-name`, (nodes) =>
    nodes.map((node) => node.textContent.trim())
  );
  const legend = await page.$$eval("#copy-compare .copy-legend-item", (nodes) =>
    nodes.map((node) => node.textContent.trim())
  );
  assert.ok(names.includes("Unnamed task"), "an unnamed task is in the fixture");
  assert.deepEqual([...legend].sort(), [...names].sort());
});

test("the Tokens price column keeps one significant-digit count", async (t) => {
  const page = await openPage(t, "tokens");
  const root = "#tokens-root";
  await page.waitForSelector(`${root} tbody tr[data-row-id] td[data-column-id="price_sol"]`);
  const prices = await page.$$eval(`${root} tbody td[data-column-id="price_sol"]`, (cells) =>
    cells.map((cell) => cell.textContent.trim()).filter((text) => /\d/.test(text))
  );
  // Significant digits: the digits after the zero run (and any subscript count).
  const significant = (text) => {
    const digits = text.replace(/[^\d₀-₉]/g, "");
    const plain = digits.replace(/[₀-₉]+/, "").replace(/^0+/, "");
    return plain.length;
  };
  const counts = new Set(prices.map(significant));
  assert.ok(prices.length > 1, "the fixture prices several tokens");
  assert.equal(counts.size, 1, `one digit count across ${prices.join(" | ")}`);
});
