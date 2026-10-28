// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: from 1200 px up the seven header actions stay inline in every locale.
 *
 * The actions once folded behind their "..." toggle at 1200 and 1280 px as soon as the
 * SOL/USD price arrived, while a narrower window, whose wordmark had already given way
 * to the logo glyph, kept them all. The row now makes room cheapest first (the Explore
 * setup control drops to one line, then the wordmark gives way to the glyph) and folds
 * the actions only when that is not enough.
 *
 * - A trading wallet with realistic figures and an Explore Mode first run, each at 1200
 *   and 1280 px: every action is visible inside the window, the row does not overflow,
 *   and in Explore Mode the setup control is one line.
 * - Every metric figure reserves its width: the metric cards measure the same with "—"
 *   and with the values that replace it, so an arriving value never reflows the row.
 * - A "—" sits next to its label as a value would; the width it reserves trails its row
 *   instead of opening a gap after the dash.
 * - Hiding the Assistant action (on the Assistant page) does not move the metric cards.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { chromium } from "playwright";

import { serveDashboard } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const LOCALES_ROOT = new URL("../../locales/", import.meta.url).pathname;
const LOCALES = readdirSync(LOCALES_ROOT).filter((name) =>
  statSync(join(LOCALES_ROOT, name)).isDirectory()
);
const WIDTHS = [1200, 1280];
// Assistant, search, announcements, notifications, refresh, theme and settings.
const ACTION_COUNT = 7;

const fixture = (path) => JSON.parse(readFileSync(`${FIXTURES_ROOT}/${path}`, "utf8"));
const METRICS = fixture("shell/header_metrics.json");
const INITIALIZATION = fixture("shell/initialization_status.json");
const BOOTSTRAP = fixture("shell/system_bootstrap.json");
const NO_COPY = { enabled: true, total_tasks: 0, paper_tasks: 0, live_tasks: 0, notices: [] };

// A first run has no copy task; a trading wallet shows four-decimal figures.
const STATES = {
  trading: { metrics: { ...METRICS, copy: NO_COPY }, explore: false },
  explore: {
    metrics: {
      ...METRICS,
      trader: { enabled: false, state: "explore" },
      wallet: Object.fromEntries(Object.keys(METRICS.wallet).map((key) => [key, null])),
      copy: NO_COPY,
    },
    explore: true,
  },
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [shell, home] = await Promise.all([loadIndex("shell"), loadIndex("home")]);

/** Resolves after two frames, once the measured fit has run. */
function settled(page) {
  return page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve(true)))
      )
  );
}

/** Opens Home in `state`; the first metrics response waits for `release()`. */
async function openHeader(locale, state) {
  const { metrics, explore } = STATES[state];
  let release;
  const held = new Promise((resolve) => {
    release = resolve;
  });
  let first = true;
  const context = await browser.newContext({ viewport: { width: WIDTHS[0], height: 800 } });
  const fixtures = createApiHandler([home, shell], "populated");
  const host = await serveDashboard(context, {
    locale,
    onApi: async (request) => {
      const path = request.url.pathname;
      if (path === "/api/header/metrics") {
        if (first) {
          first = false;
          await held;
        }
        return { body: JSON.stringify(metrics) };
      }
      if (path === "/api/system/bootstrap") {
        return { body: JSON.stringify({ ...BOOTSTRAP, explore_mode: explore }) };
      }
      if (path === "/api/initialization/status") {
        return {
          body: JSON.stringify({
            ...INITIALIZATION,
            explore_mode: explore,
            explore_mode_enabled: explore,
          }),
        };
      }
      return fixtures.onApi(request);
    },
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/home`);
  await page.waitForSelector("#navTabs > a.tab.active");
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, release, close };
}

/** Each visible metric card's width. */
function cardWidths(page) {
  return page.evaluate(() =>
    Object.fromEntries(
      [...document.querySelectorAll("#headerCards .header-card")]
        .filter((card) => !card.hidden && card.getClientRects().length > 0)
        .map((card) => [card.id, card.getBoundingClientRect().width])
    )
  );
}

/** The actions' fold, how many show inside the window, and the row's overflow. */
function actionLayout(page) {
  return page.evaluate(() => {
    const row = document.querySelector(".header-row-1");
    const detail = document.querySelector("#accessControl .access-control-detail");
    const buttons = [...document.querySelectorAll("#headerActionsItems .header-action-btn")];
    return {
      folded: document.getElementById("headerActions").classList.contains("is-folded"),
      overflow: row.scrollWidth - row.clientWidth,
      exploreDetail: Boolean(detail && detail.getClientRects().length > 0),
      total: buttons.length,
      shown: buttons.filter((button) => {
        const box = button.getBoundingClientRect();
        return (
          button.getClientRects().length > 0 &&
          getComputedStyle(button).visibility === "visible" &&
          box.left >= 0 &&
          box.right <= window.innerWidth
        );
      }).length,
    };
  });
}

for (const locale of LOCALES) {
  test(`the seven header actions stay inline at 1200 and 1280 px (${locale})`, async () => {
    for (const state of Object.keys(STATES)) {
      const { page, release, close } = await openHeader(locale, state);
      try {
        await settled(page);
        const placeholder = await cardWidths(page);
        release();
        await page.waitForSelector("#solPriceValue:not(:empty)");
        await page.waitForFunction(
          () => document.getElementById("solPriceValue").textContent !== "—"
        );
        await settled(page);
        const filled = await cardWidths(page);
        for (const [id, width] of Object.entries(placeholder)) {
          assert.ok(
            Math.abs(filled[id] - width) <= 0.5,
            `${locale} ${state}: #${id} is ${width}px with "—" and ${filled[id]}px with its value`
          );
        }

        for (const width of WIDTHS) {
          await page.setViewportSize({ width, height: 800 });
          await settled(page);
          const where = `${locale} ${state} at ${width}px`;
          const layout = await actionLayout(page);
          assert.equal(layout.folded, false, `${where}: the actions fold`);
          assert.equal(layout.total, ACTION_COUNT, `${where}: the action group`);
          assert.equal(layout.shown, ACTION_COUNT, `${where}: actions in view`);
          assert.equal(layout.overflow, 0, `${where}: the row overflows`);
          if (state === "explore") {
            assert.equal(layout.exploreDetail, false, `${where}: the setup control is one line`);
          }
        }
      } finally {
        await close();
      }
    }
  });
}

/** Offset of each "—" from where a value would start after its label (0 = in place). */
function placeholderGaps(page) {
  return page.evaluate(() =>
    [...document.querySelectorAll("#headerCards .header-card [data-empty]")]
      .filter(
        (figure) =>
          !figure.closest("[hidden]") &&
          figure.previousElementSibling?.matches(".card-label, .card-sub")
      )
      .map((figure) => {
        const range = document.createRange();
        range.selectNodeContents(figure);
        const dash = range.getBoundingClientRect();
        const label = figure.previousElementSibling.getBoundingClientRect();
        const gap = parseFloat(getComputedStyle(figure.parentElement).columnGap);
        return { id: figure.id, offset: dash.left - label.right - gap };
      })
  );
}

test("an unknown figure sits next to its label and the cards ignore a hidden action", async () => {
  const { page, release, close } = await openHeader("en", "explore");
  try {
    release();
    await page.waitForFunction(() => document.getElementById("solPriceValue").textContent !== "—");
    await settled(page);
    const gaps = await placeholderGaps(page);
    assert.ok(gaps.length >= 3, `only ${gaps.length} empty figures were measured`);
    for (const { id, offset } of gaps) {
      assert.ok(Math.abs(offset) <= 1, `#${id}: "—" sits ${offset}px from where a value starts`);
    }

    const cardsAt = () =>
      page.evaluate(() => document.getElementById("headerCards").getBoundingClientRect().left);
    const before = await cardsAt();
    await page.evaluate(() =>
      document.querySelector(".header-action-btn.global-chat-btn").classList.add("hidden")
    );
    await settled(page);
    assert.ok(
      Math.abs((await cardsAt()) - before) <= 0.5,
      "the metric cards move when the Assistant action is hidden"
    );
  } finally {
    await close();
  }
});
