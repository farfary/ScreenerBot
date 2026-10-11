// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Home portfolio overview states every figure legibly and once.
 *
 * - The balance trend is drawn at least at its CSS minimum width, or not at all; it is
 *   never squeezed into a sliver beside a wide balance.
 * - A calendar day cell shows its P&L only; every day with a detail opens it on hover
 *   and on keyboard focus, the popover names the end balance and never covers the
 *   month navigation, and a past day without trades is classed as such, never empty.
 * - Every week row holds seven day cells and a week total that sums its days.
 * - A day change between samples that priced different holdings is a dash that explains
 *   itself on hover and keyboard focus, with no trend line beside it.
 * - The exposure ledger names a single position instead of averaging it, and its
 *   symbol opens the position details.
 * - The open count beside the ledger is unknown or the count the ledger lists, never a
 *   0 or a value counted up from one while a position is listed.
 * - No width scrolls the page sideways, and the hero's figure labels stay in their cells.
 * - A phone stacks every section at its natural height, nothing overlapping or clipped.
 * - The calendar fills the workspace height, and its summary wraps into full rows.
 * - A poll that changed nothing replaces no exposure control and closes no day detail.
 * - The day-change percent has one precision in the header Worth card and the hero.
 * - Every pipeline label fits inside its own cell at every desktop width and locale.
 * - Today's P&L in the header bot card and ticker takes the shared SOL precision (4
 *   decimals, like the Home hero), not a precision of its own.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

// Inside the fixture month, so past, today and future days all render.
const FIXED_NOW = new Date("2026-10-15T12:00:00Z");

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [home, shell] = await Promise.all([loadIndex("home"), loadIndex("shell")]);

async function openHome(t, width, locale = "en", answers = {}, device = {}, initScript = null) {
  const context = await browser.newContext({ viewport: { width, height: 900 }, ...device });
  const api = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, {
    locale,
    onApi: (request) => {
      const body = answers[request.url.pathname];
      return body ? { body } : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  if (initScript) await page.addInitScript(initScript);
  await page.clock.setFixedTime(FIXED_NOW);
  await page.goto(`${host.origin}/`);
  await page.waitForSelector(PAGE_READY);
  await page.waitForSelector(".home-dashboard.loaded");
  return page;
}

test("the balance trend is drawn at its minimum width or not at all", async (t) => {
  for (const width of [820, 1000, 1200, 1440]) {
    const page = await openHome(t, width);
    // A wide balance takes the row; the trend must give way whole.
    await page.evaluate(() => {
      document.getElementById("walletBalance").textContent = "12,345,678.9012 SOL";
    });
    await page.waitForFunction(() => {
      const svg = document.getElementById("heroSpark");
      const shown = getComputedStyle(svg).display !== "none";
      return (
        !shown || svg.getBoundingClientRect().width >= parseFloat(getComputedStyle(svg).minWidth)
      );
    });
    const spark = await page.$eval("#heroSpark", (svg) => ({
      shown: getComputedStyle(svg).display !== "none",
      width: svg.getBoundingClientRect().width,
      min: parseFloat(getComputedStyle(svg).minWidth),
    }));
    assert.ok(spark.min > 0, `${width}px: the trend declares a minimum width`);
    if (spark.shown) {
      assert.ok(
        spark.width >= spark.min,
        `${width}px: the trend is ${spark.width}px, below ${spark.min}px`
      );
    }
  }
});

test("calendar days show P&L only and open their detail on hover and focus", async (t) => {
  const page = await openHome(t, 1440);
  await page.waitForSelector(".calendar-cell.no-trades[data-date]");
  const cells = await page.$$eval(".calendar-grid > .calendar-cell", (all) =>
    all.map((cell) => ({
      cls: cell.className,
      pnl: !!cell.querySelector(".cell-pnl"),
      other: cell.querySelectorAll(":scope > span:not(.cell-day, .cell-pnl)").length,
      date: cell.dataset.date ?? null,
      focusable: cell.tabIndex === 0,
    }))
  );
  assert.ok(cells.length > 0, "calendar cells render");
  for (const cell of cells) {
    assert.doesNotMatch(cell.cls, /\bempty\b/, "no day is classed empty");
    assert.equal(cell.other, 0, `a ${cell.cls} cell shows an unlabelled figure`);
    if (cell.pnl) assert.ok(cell.date && cell.focusable, `a ${cell.cls} cell has no detail`);
  }

  await page.hover(".calendar-cell.no-trades[data-date]");
  await page.waitForSelector(".cal-day-popover--visible");
  const labels = await page.$$eval(".cal-day-popover .cal-pop-label", (els) =>
    els.map((el) => el.textContent.trim())
  );
  assert.ok(labels.includes("End balance"), `a no-trade day names its balance: ${labels}`);

  // A top-row traded day, reached from the keyboard: its popover opens and stays off
  // the month navigation.
  await page.mouse.move(0, 0);
  const first = page
    .locator(".calendar-cell.profit[data-date], .calendar-cell.loss[data-date]")
    .first();
  await first.focus();
  await page.waitForSelector(".cal-day-popover--visible");
  const overlap = await page.evaluate(() => {
    const pop = document.querySelector(".cal-day-popover").getBoundingClientRect();
    const nav = document.querySelector(".calendar-header").getBoundingClientRect();
    return pop.top < nav.bottom && pop.bottom > nav.top;
  });
  assert.equal(overlap, false, "the day popover covers the month navigation");
  await page.keyboard.press("Escape");
  await page.waitForSelector(".cal-day-popover--visible", { state: "detached" }).catch(() => {});
  assert.equal(await page.$(".cal-day-popover--visible"), null, "Escape closes the popover");
});

test("every week row holds seven days and a week total that sums them", async (t) => {
  const page = await openHome(t, 1440);
  await page.waitForSelector(".calendar-week .cell-pnl");
  const rows = await page.$$eval(".calendar-grid > *", (all) => {
    const number = (text) => Number(text.replace(/[\u2066-\u2069]/g, "").replace(/[^0-9.-]/g, ""));
    const weeks = [];
    let days = [];
    for (const el of all) {
      if (el.classList.contains("calendar-week")) {
        const total = el.querySelector(".cell-pnl")?.textContent ?? null;
        weeks.push({
          days: days.length,
          total: total === null ? null : number(total),
          sum: days.reduce((sum, day) => sum + (day ? number(day) : 0), 0),
          traded: days.some(Boolean),
        });
        days = [];
      } else {
        days.push(el.querySelector(".cell-pnl")?.textContent ?? null);
      }
    }
    return { weeks, trailing: days.length };
  });
  assert.equal(rows.trailing, 0, "the grid ends on a week cell");
  for (const week of rows.weeks) {
    assert.equal(week.days, 7, "a week row holds seven day cells");
    if (!week.traded) {
      assert.equal(week.total, null, "a week without trades shows no total");
      continue;
    }
    assert.ok(Math.abs(week.total - week.sum) < 0.0005, `week ${week.total} != days ${week.sum}`);
  }
});

test("a day change across differently priced holdings is a dash that says why", async (t) => {
  const body = JSON.parse(readFileSync(`${FIXTURES_ROOT}/home/dashboard_home.json`, "utf8"));
  body.wallet = {
    ...body.wallet,
    change_status: "incomparable",
    change_native: null,
    change_percent: null,
    balance_history: [],
  };
  const page = await openHome(t, 1440, "en", { "/api/dashboard/home": JSON.stringify(body) });
  assert.equal((await page.textContent("#homeWalletChange")).trim(), "—");
  const trend = await page.evaluate(() => ({
    line: document.getElementById("heroSparkLine").getAttribute("points"),
    range: getComputedStyle(document.querySelector(".hero-trend-range")).display,
  }));
  assert.deepEqual(trend, { line: "", range: "none" }, "an unknown change draws a trend");

  await page.focus("#homeWalletChange");
  await page.waitForSelector(".app-tooltip:not([hidden])");
  assert.match(await page.textContent(".app-tooltip"), /today's change isn't shown/);
  await page.keyboard.press("Tab");
  await page.mouse.move(0, 0);
  await page.hover("#homeWalletChange");
  await page.waitForSelector(".app-tooltip:not([hidden])");
});

test("the exposure ledger names a single position and opens it", async (t) => {
  const body = JSON.parse(readFileSync(`${FIXTURES_ROOT}/home/dashboard_home.json`, "utf8"));
  body.positions = {
    ...body.positions,
    open_count: 1,
    avg_position_size_native: 1.1,
    worst_performer: body.positions.best_performer,
  };
  const page = await openHome(t, 1440, "en", { "/api/dashboard/home": JSON.stringify(body) });
  const shown = await page.$$eval("#exposureLedger .metric-row", (rows) =>
    rows.filter((row) => !row.hidden).map((row) => row.querySelector(".position-label").textContent)
  );
  assert.deepEqual(shown, ["Invested", "Current value", "Open P&L", "Position", "Held"]);
  await page.click("#positionsOnly .exposure-position");
  await page.waitForSelector(".position-details-dialog");
});

test("the open count is unknown or the ledger's count while a position is listed", async (t) => {
  const body = JSON.parse(readFileSync(`${FIXTURES_ROOT}/home/dashboard_home.json`, "utf8"));
  body.positions = {
    ...body.positions,
    open_count: 1,
    avg_position_size_native: 1.1,
    worst_performer: body.positions.best_performer,
  };
  // Every text the count shows, from the first paint on.
  const recordCount = () => {
    window.__shownCounts = [];
    new window.MutationObserver(() => {
      const text = document.getElementById("positionsCount")?.textContent;
      if (text !== undefined && text !== window.__shownCounts.at(-1)) {
        window.__shownCounts.push(text);
      }
    }).observe(document, { subtree: true, childList: true, characterData: true });
  };
  const page = await openHome(
    t,
    1440,
    "en",
    { "/api/dashboard/home": JSON.stringify(body) },
    {},
    recordCount
  );
  await page.waitForSelector("#positionsOnly .exposure-position");
  await page.waitForTimeout(600);
  const shown = await page.evaluate(() => window.__shownCounts);
  assert.deepEqual(
    shown.filter((text) => text !== "—"),
    ["1"],
    `the count showed ${JSON.stringify(shown)}`
  );
});

for (const width of [390, 900, 1100, 1440]) {
  test(`no page scrolls sideways and hero labels stay in their cells at ${width}px`, async (t) => {
    const page = await openHome(t, width);
    const layout = await page.evaluate(() => {
      const container = document.querySelector(".home-dashboard").closest(".page-container");
      const spill = [...document.querySelectorAll(".portfolio-stat")].filter((stat) => {
        const box = stat.getBoundingClientRect();
        const label = stat.querySelector(".hero-stat-label").getBoundingClientRect();
        return label.left < box.left - 0.5 || label.right > box.right + 0.5;
      });
      const identity = document.querySelector("#homeWalletIdentity").getBoundingClientRect();
      const stats = document.querySelector(".portfolio-stats").getBoundingClientRect();
      const overlaps =
        identity.right > stats.left + 0.5 &&
        identity.left < stats.right - 0.5 &&
        identity.bottom > stats.top + 0.5 &&
        identity.top < stats.bottom - 0.5;
      return {
        sideways: container.scrollWidth - container.clientWidth,
        page: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        spill: spill.length,
        overlaps,
      };
    });
    assert.ok(layout.sideways <= 0 && layout.page <= 0, `${width}px scrolls sideways`);
    assert.equal(layout.spill, 0, `${width}px: a hero label leaves its cell`);
    assert.equal(layout.overlaps, false, `${width}px: the wallet line overlaps the figures`);
  });
}

test("a phone stacks every section at its natural height", async (t) => {
  const page = await openHome(t, 390, "en", {}, { isMobile: true, hasTouch: true });
  const layout = await page.evaluate(() => {
    const rect = (css) => document.querySelector(css).getBoundingClientRect();
    const order = [
      ".portfolio-stats",
      ".calendar-grid",
      ".calendar-summary",
      ".exposure-section",
      ".pipeline-section",
    ];
    const overlaps = order
      .slice(1)
      .filter((css, i) => rect(css).top < rect(order[i]).bottom - 0.5)
      .map((css, i) => `${order[i]} / ${css}`);
    // Text shown in a calendar cell is never cut to a sliver.
    const clipped = [...document.querySelectorAll(".calendar-grid span")]
      .filter((el) => el.checkVisibility() && el.textContent.trim())
      .filter((el) => el.scrollWidth > el.clientWidth + 1)
      .map((el) => el.textContent.trim());
    const grid = rect(".calendar-grid");
    const cells = [...document.querySelectorAll(".calendar-grid > *")]
      .filter((el) => el.checkVisibility())
      .map((el) => el.getBoundingClientRect());
    return {
      overlaps,
      clipped,
      gridHeight: grid.height,
      // The grid's own fill (its row lines) never reaches past its cells.
      lineOverrun: grid.right - Math.max(...cells.map((cell) => cell.right)),
      calendarWidth: rect(".calendar-workspace").width,
      viewport: document.documentElement.clientWidth,
    };
  });
  assert.deepEqual(layout.overlaps, [], "stacked sections overlap");
  assert.deepEqual(layout.clipped, [], "calendar text is clipped");
  assert.ok(layout.gridHeight > 150, `the calendar grid is ${layout.gridHeight}px tall`);
  assert.ok(layout.lineOverrun < 1, `row lines run ${layout.lineOverrun}px past the cells`);
  assert.ok(
    layout.calendarWidth >= layout.viewport - 1,
    `the calendar is ${layout.calendarWidth}px of ${layout.viewport}px`
  );
});

test("the calendar fills the workspace height with no band under it", async (t) => {
  const page = await openHome(t, 1440);
  const gap = await page.evaluate(() => {
    const section = document.querySelector(".calendar-workspace").getBoundingClientRect();
    const summary = document.querySelector(".calendar-summary").getBoundingClientRect();
    const padding = parseFloat(
      getComputedStyle(document.querySelector(".calendar-workspace")).paddingBottom
    );
    return section.bottom - padding - summary.bottom;
  });
  assert.ok(gap < 2, `${gap}px of empty band under the calendar`);
});

for (const width of [1440, 900, 390]) {
  test(`the calendar summary wraps into full rows at ${width}px`, async (t) => {
    const page = await openHome(t, width);
    const rows = await page.evaluate(() => {
      const summary = document.querySelector(".calendar-summary").getBoundingClientRect();
      const byRow = new Map();
      for (const stat of document.querySelectorAll(".calendar-summary-stat")) {
        const box = stat.getBoundingClientRect();
        byRow.set(Math.round(box.top), (byRow.get(Math.round(box.top)) ?? 0) + box.width);
      }
      return [...byRow.values()].map((width) => summary.width - width);
    });
    for (const short of rows) assert.ok(Math.abs(short) < 2, `a summary row is ${short}px short`);
  });
}

test("a poll that changed nothing keeps the exposure button and the day detail", async (t) => {
  const page = await openHome(t, 1440);
  const button = await page.waitForSelector("#positionsBest .exposure-position");
  await page.focus(".calendar-cell.profit[data-date]");
  await page.waitForSelector(".cal-day-popover--visible");
  // Both pollers fire on one 5s tick.
  await page.waitForResponse((response) => response.url().includes("/api/dashboard/home"));
  await page.waitForResponse((response) =>
    response.url().includes("/api/dashboard/portfolio-calendar")
  );
  await page.waitForTimeout(300);
  assert.equal(await button.evaluate((el) => el.isConnected), true, "the button was replaced");
  assert.ok(await page.$(".cal-day-popover--visible"), "the day detail closed on a poll");
});

test("the day-change percent has one precision in the header and the hero", async (t) => {
  const page = await openHome(t, 1440);
  await page.waitForSelector("#homeWalletChange .change-percent");
  const decimals = (text) => /[.,](\d+)\s*%/.exec(text)?.[1].length ?? 0;
  const hero = await page.$eval("#homeWalletChange .change-percent", (el) => el.textContent);
  const header = await page.$eval("#walletChange", (el) => el.textContent);
  assert.ok(decimals(hero) > 0, `hero percent "${hero}" carries decimals`);
  assert.equal(decimals(hero), decimals(header), `hero "${hero}" and header "${header}"`);
});

for (const locale of ["en", "de"]) {
  test(`every pipeline label fits inside its cell (${locale})`, async (t) => {
    const page = await openHome(t, 1024, locale);
    for (const width of [1024, 1328, 1440, 1920]) {
      await page.setViewportSize({ width, height: 900 });
      const overflowing = await page.$$eval(".pipeline-metric", (cells) =>
        cells
          .map((cell) => {
            const label = cell.querySelector("span");
            const box = cell.getBoundingClientRect();
            const text = label.getBoundingClientRect();
            const fits =
              label.scrollWidth <= label.clientWidth &&
              text.left >= box.left - 0.5 &&
              text.right <= box.right + 0.5;
            return fits ? null : `${label.textContent} (${label.scrollWidth}/${label.clientWidth})`;
          })
          .filter(Boolean)
      );
      assert.deepEqual(overflowing, [], `${width}px: a pipeline label overflows its cell`);
    }
  });
}

test("today's P&L in the header takes the shared SOL precision", async (t) => {
  const header = JSON.parse(readFileSync(`${FIXTURES_ROOT}/shell/header_metrics.json`, "utf8"));
  header.trader.today_pnl_native = 0.01234;
  const page = await openHome(t, 1440, "en", {
    "/api/header/metrics": JSON.stringify(header),
  });
  const text = (selector) =>
    page
      .waitForFunction((css) => {
        const shown = globalThis.document.querySelector(css)?.textContent ?? "";
        return /\d/.test(shown) && shown.replace(/[\u2066-\u2069]/g, "").trim();
      }, selector)
      .then((handle) => handle.jsonValue());
  assert.equal(await text("#botPnL .pnl-num"), "+0.0123");
  assert.match(await text("#tickerTodayPnL"), /^\+0\.0123 SOL /);
});
