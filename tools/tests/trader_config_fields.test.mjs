// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: what the Auto Trader rule fields compute from the loaded config.
 *
 * - The line under a field states its computed effect in one plain format, with no
 *   upper-case prefix ("IMPACT:", "CURRENT:", "READABLE:"); the DCA tab states both
 *   waits and the most a position can hold once every step has bought.
 * - Every worked example starts from one position: a buy at the configured trade size
 *   (`trader.trade_size_sol`), never a fixed amount of its own.
 * - A loss threshold reads as a magnitude after a fixed "−" (Stop Loss, Time Exit
 *   loss) while the config keeps its own sign: Stop Loss stores a positive percent,
 *   the time exit a negative one. Load, display, edit and save round-trip at both
 *   bounds of each field.
 * - Sell Part First shows its Part to Sell field only while on; the field takes its bounds
 *   and unit from the metadata, saves through its card, and the stop-loss example sells
 *   that part at the threshold and the rest if the price stays below the stop.
 * - The take-profit ladder edits its steps in place up to the metadata's step limit,
 *   names what it cannot save and keeps Save disabled until it can, and saves its
 *   levels and shares as their two config lists. Its group takes its accessible name
 *   from its label, and decimal shares that add up to the whole position save.
 * - A step added to the ladder doubles the last level, capped at the profit maximum, and
 *   keeps the ladder valid: it splits the last share in half when the steps already sell
 *   the whole position and otherwise sells the smallest share. Add is disabled while the
 *   last level is at the profit maximum.
 * - A caption row names the profit and share columns over the inputs, hidden from
 *   assistive technology, since each input carries its own name, and is not shown while
 *   the ladder has no row.
 * - A ladder that keeps a runner warns that only the stop loss or the user can close it
 *   while the trailing stop, the break-even stop, both time exits and every exit
 *   strategy are off.
 * - The DCA ladder is the same editor: each step's drop reads as a magnitude after a
 *   fixed "−" within the metadata bounds, states what it buys from the configured trade
 *   size and flags a step under the minimum trade; a step added doubles the last drop
 *   and keeps the last multiple, up to the metadata's step limit. Measured from the
 *   first entry, a step that is not deeper than the one before it keeps Save disabled.
 *   The ladder saves as its two config lists, and the basis as its stored name.
 * - The trailing stop states the minimum it locks when it arms, and warns when that
 *   stop sits at or below the entry. Its distance bands are the same editor: empty by
 *   default, a band added doubles the last threshold (the activation for the first) at
 *   the last distance (the base distance for the first), up to the metadata's limit;
 *   thresholds must rise above the activation, re-checked as the activation is typed,
 *   each band trails by less than its threshold, and the last band can be removed.
 * - The break-even stop and the flat-position exit state their rule in one line, take
 *   their bounds and units from the metadata, and save through their cards.
 * - The time limit fields take their bounds from the metadata, not the markup, and a
 *   blank threshold or duration shows a dash in its example instead of a stand-in figure.
 * - A line the page script writes carries no static catalog id, so a re-localization pass
 *   cannot restore a placeholder over it.
 *
 * The config served here uses a trade size other than the default, so a figure that
 * follows the default instead of the loaded config fails.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { FIXTURES_ROOT, createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";

const CONFIG = JSON.parse(readFileSync(`${FIXTURES_ROOT}/trader/config_full.json`, "utf8"));
const OVERVIEW = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/trader/strategies_overview.json`, "utf8")
);
const TRADE_SIZE = 0.02;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

/**
 * The trader page on `tab`, served `trader` merged over the recorded trader config.
 * `exitStrategiesOff` serves the recorded strategies overview with every exit strategy off.
 */
async function openTrader(tab, trader = {}, { exitStrategiesOff = false } = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const config = { ...CONFIG, trader: { ...CONFIG.trader, trade_size_sol: TRADE_SIZE, ...trader } };
  const onApi = (request) => {
    if (request.url.pathname === "/api/config" && request.method === "GET") {
      return { body: JSON.stringify(config) };
    }
    if (exitStrategiesOff && request.url.pathname === "/api/strategies/overview") {
      const counts = { ...OVERVIEW.counts, exit: { ...OVERVIEW.counts.exit, enabled: 0 } };
      return { body: JSON.stringify({ ...OVERVIEW, counts }) };
    }
    if (request.method === "PATCH") return { body: JSON.stringify({ success: true }) };
    return api.onApi(request);
  };
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/trader`);
  await page.waitForSelector(PAGE_READY);
  await page.click(`#subTabsContainer [data-tab-id="${tab}"]`);
  await page.locator(`#${tab}-tab`).waitFor();
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

/** Text without the bidi isolation marks Fluent puts around each placeable. */
const plain = (text) => text.replace(/[\u2068\u2069]/g, "").trim();

/** Waits until the element `id` contains `text` once those isolation marks are dropped. */
const waitForPlain = (page, id, text) =>
  page.waitForFunction(
    ([id, text]) =>
      document.getElementById(id)?.textContent.replace(/[\u2068\u2069]/g, "").includes(text),
    [id, text]
  );

/** The computed line under the field `id`. */
const line = async (page, id) =>
  plain(
    await page.$eval(
      `#${id}`,
      (input) => input.closest(".config-group").querySelector(".config-impact").textContent
    )
  );

test("a computed line is plain text, and the DCA tab states its waits and its most", async () => {
  const { page, close } = await openTrader("dca");
  try {
    await page.locator("#dca-capacity", { hasText: "SOL" }).waitFor();
    const prefixes = await page.$$eval(".trader-tab-content .config-impact", (lines) =>
      lines
        .map((el) => el.textContent.trim())
        .filter((text) => /^\p{Lu}{3,}\s*:/u.test(text) || text === "")
    );
    assert.deepEqual(prefixes, [], "no shouted prefix and no empty line");
    assert.equal(await page.locator(".impact-label").count(), 0);

    assert.equal(
      await line(page, "dca-levels"),
      "Most this position can hold: 0.04 SOL (2× the first buy)"
    );
    assert.match(await line(page, "dca-cooldown"), /^At least 30\D.*between extra entries$/);
    assert.equal(await line(page, "dca-min-hold"), "The first add may follow the entry at once");

    await page.fill("#dca-min-hold", "15");
    assert.match(
      await line(page, "dca-min-hold"),
      /^The first add waits at least 15\D.*after the entry$/
    );
  } finally {
    await close();
  }
});

test("every worked example starts from the configured trade size", async () => {
  const { page, close } = await openTrader("stop-loss");
  try {
    await page.locator("#stop-loss-example-entry", { hasText: "SOL" }).waitFor();
    const text = async (selector) => plain(await page.locator(selector).textContent());
    const expected = {
      "stop-loss": {
        "#stop-loss-example-entry": "0.02 SOL",
        "#stop-loss-example-exit": "0.01 SOL",
      },
      "trailing-stop": { "#example-entry": "0.02 SOL" },
      "take-profit": {
        "#take-profit-example-entry": "0.02 SOL",
        "#take-profit-example-step-1": "0.024 SOL",
      },
    };
    for (const [tab, figures] of Object.entries(expected)) {
      await page.click(`#subTabsContainer [data-tab-id="${tab}"]`);
      for (const [selector, figure] of Object.entries(figures)) {
        assert.equal(await text(selector), figure, `${tab} ${selector}`);
      }
    }
    await page.click('#subTabsContainer [data-tab-id="dca"]');
    const bought = await page
      .locator("#dca-levels .level-list-amount")
      .evaluateAll((cells) => cells.map((cell) => cell.textContent));
    assert.deepEqual(bought.map(plain), ["= 0.01 SOL", "= 0.01 SOL"]);
  } finally {
    await close();
  }
});

// Each loss field: its tab, config key, and stored bounds with the value shown for each.
const LOSS_FIELDS = [
  {
    tab: "stop-loss",
    id: "stop-loss-threshold",
    key: "stop_loss_threshold_pct",
    bounds: [
      [1, "1"],
      [100, "100"],
    ],
  },
  {
    tab: "time-rules",
    id: "time-loss-threshold",
    key: "time_override_loss_threshold_percent",
    bounds: [
      [-100, "100"],
      [0, "0"],
    ],
  },
];

for (const { tab, id, key, bounds } of LOSS_FIELDS) {
  for (const [from, to] of [bounds, [...bounds].reverse()]) {
    test(`${key} round-trips from ${from[0]} to ${to[0]} through its magnitude`, async () => {
      const { page, close } = await openTrader(tab, { [key]: from[0] });
      try {
        const input = page.locator(`#${id}`);
        await page.waitForFunction(
          ([selector, shown]) => document.querySelector(selector).value === shown,
          [`#${id}`, from[1]]
        );
        assert.deepEqual(
          await input.evaluate((el) => ({
            min: el.min,
            max: el.max,
            prefix: el.closest(".number-field").querySelector(".input-prefix")?.textContent,
          })),
          { min: key === "stop_loss_threshold_pct" ? "1" : "0", max: "100", prefix: "−" }
        );

        await input.fill(to[1]);
        const save = page.locator(`.config-card:has(#${id}) .config-card-save`);
        const [request] = await Promise.all([
          page.waitForRequest(
            (candidate) =>
              candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
          ),
          save.click(),
        ]);
        assert.equal(request.postDataJSON()[key], to[0]);
      } finally {
        await close();
      }
    });
  }
}

test("Sell Part First shows its part field and a two-step stop-loss example", async () => {
  const { page, close } = await openTrader("stop-loss", {
    stop_loss_allow_partial: true,
    stop_loss_partial_pct: 40,
  });
  try {
    const group = page.locator("#stop-loss-partial-pct-group");
    const part = page.locator("#stop-loss-partial-pct");
    const text = async (selector) => plain(await page.locator(selector).textContent());
    await page.waitForFunction(
      () => document.querySelector("#stop-loss-partial-pct").value === "40"
    );
    assert.equal(await group.isVisible(), true);
    assert.deepEqual(
      await part.evaluate((el) => ({
        min: el.min,
        max: el.max,
        unit: el.closest(".config-group").querySelector(".input-unit").textContent,
      })),
      { min: "10", max: "90", unit: "%" }
    );

    await page.locator("#stop-loss-example-exit", { hasText: "SOL" }).waitFor();
    assert.equal(await text("#stop-loss-exit-title"), "Sell 40%");
    assert.equal(await text("#stop-loss-exit-detail"), "Part of the position");
    assert.equal(await text("#stop-loss-example-exit"), "0.004 SOL");
    assert.equal(await page.locator("#stop-loss-rest-step").isVisible(), true);
    assert.equal(await text("#stop-loss-example-rest"), "0.006 SOL");

    await part.fill("25");
    const [request] = await Promise.all([
      page.waitForRequest(
        (candidate) =>
          candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
      ),
      page.locator(".config-card:has(#stop-loss-partial-pct) .config-card-save").click(),
    ]);
    assert.equal(request.postDataJSON().stop_loss_partial_pct, 25);

    await page.locator('label[for="stop-loss-allow-partial"]').click();
    assert.equal(await group.isVisible(), false);
    assert.equal(await page.locator("#stop-loss-rest-step").isVisible(), false);
    assert.equal(await text("#stop-loss-exit-title"), "Auto Exit");
    assert.equal(await text("#stop-loss-exit-detail"), "Full position exit");
    assert.equal(await text("#stop-loss-example-exit"), "0.01 SOL");
  } finally {
    await close();
  }
});

test("the take-profit ladder adds steps up to its limit and saves only a valid ladder", async () => {
  const { page, close } = await openTrader("take-profit", {
    take_profit_levels_pct: [25, 60],
    take_profit_sell_pct: [40, 30],
  });
  try {
    const ladder = page.locator("#take-profit-levels");
    const rows = ladder.locator(".level-list-row");
    const add = ladder.locator(".level-list-add");
    const error = ladder.locator(".level-list-error");
    const save = page.locator(".config-card:has(#take-profit-levels) .config-card-save");
    const text = async (selector) => plain(await page.locator(selector).textContent());

    await page.waitForFunction(
      () => document.querySelector("#take-profit-levels .level-list-add")?.disabled === false
    );
    assert.equal(await rows.count(), 2);
    assert.equal(await text("#take-profit-runner"), "30% of the position stays open as a runner");
    assert.equal(await text("#take-profit-example-step-2"), "0.032 SOL");

    await add.click();
    assert.equal(await rows.nth(2).locator(".level-list-profit").inputValue(), "120");
    assert.equal(
      await rows.nth(2).locator(".level-list-sell").inputValue(),
      "5",
      "with a runner left, a new step sells the smallest share"
    );
    assert.equal(await rows.nth(1).locator(".level-list-sell").inputValue(), "30");
    await add.click();
    assert.equal(await rows.count(), 4);
    assert.equal(await add.isDisabled(), true, "the metadata allows four steps");
    assert.equal(await error.isVisible(), false);

    await rows.nth(3).locator(".level-list-sell").fill("50");
    assert.equal(await error.isVisible(), true);
    assert.equal(plain(await error.textContent()), "The steps together can sell at most 100%");
    assert.equal(await save.isDisabled(), true);

    await rows.nth(1).locator(".level-list-profit").fill("20");
    assert.equal(
      plain(await error.textContent()),
      "Each step needs a higher profit level than the one before it"
    );

    await rows.nth(3).locator(".level-list-remove").click();
    await rows.nth(1).locator(".level-list-profit").fill("60");
    assert.equal(await rows.count(), 3);
    assert.equal(await error.isVisible(), false);
    assert.equal(await save.isDisabled(), false);

    const [request] = await Promise.all([
      page.waitForRequest(
        (candidate) =>
          candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
      ),
      save.click(),
    ]);
    const body = request.postDataJSON();
    assert.deepEqual(body.take_profit_levels_pct, [25, 60, 120]);
    assert.deepEqual(body.take_profit_sell_pct, [40, 30, 5]);
  } finally {
    await close();
  }
});

test("a step added to a ladder that sells the whole position splits the last share", async () => {
  const { page, close } = await openTrader("take-profit");
  try {
    const ladder = page.locator("#take-profit-levels");
    const rows = ladder.locator(".level-list-row");
    const add = ladder.locator(".level-list-add");
    await page.waitForFunction(
      () => document.querySelector("#take-profit-levels .level-list-add")?.disabled === false
    );
    assert.equal(await rows.count(), 1);

    await add.click();
    const values = async (selector) =>
      rows.locator(selector).evaluateAll((inputs) => inputs.map((input) => input.value));
    assert.deepEqual(await values(".level-list-profit"), ["20", "40"]);
    assert.deepEqual(await values(".level-list-sell"), ["50", "50"]);
    assert.equal(await ladder.locator(".level-list-error").isVisible(), false);
    const save = page.locator(".config-card:has(#take-profit-levels) .config-card-save");
    assert.equal(await save.isDisabled(), false);
  } finally {
    await close();
  }
});

test("Add is disabled while the last level is at the profit maximum", async () => {
  const { page, close } = await openTrader("take-profit", {
    take_profit_levels_pct: [500, 1000],
    take_profit_sell_pct: [50, 50],
  });
  try {
    const ladder = page.locator("#take-profit-levels");
    await ladder.locator(".level-list-row").first().waitFor();
    const add = ladder.locator(".level-list-add");
    assert.equal(await add.isDisabled(), true);

    await ladder.locator(".level-list-row").nth(1).locator(".level-list-profit").fill("900");
    assert.equal(await add.isDisabled(), false, "a level below the maximum leaves room");
  } finally {
    await close();
  }
});

test("a caption row names the ladder's input columns, hidden from assistive technology", async () => {
  const { page, close } = await openTrader("take-profit", {
    take_profit_levels_pct: [25, 60],
    take_profit_sell_pct: [40, 30],
  });
  try {
    const ladder = page.locator("#take-profit-levels");
    const caption = ladder.locator(".level-list-rows > .level-list-caption");
    await caption.waitFor();
    assert.equal(await caption.count(), 1);
    assert.equal(await caption.getAttribute("aria-hidden"), "true");
    const cells = caption.locator(":scope > span");
    const texts = (await cells.allTextContents()).map(plain);
    assert.deepEqual(texts, ["", "Profit Level", "Share to Sell", ""]);

    const row = ladder.locator(".level-list-row").first();
    const start = async (locator) => (await locator.boundingBox()).x;
    assert.equal(await start(cells.nth(1)), await start(row.locator(".level-list-field").nth(0)));
    assert.equal(await start(cells.nth(2)), await start(row.locator(".level-list-field").nth(1)));
  } finally {
    await close();
  }
});

test("the ladder group is named by its label, and decimal shares of the whole position save", async () => {
  const { page, close } = await openTrader("take-profit", {
    take_profit_levels_pct: [20, 50, 100],
    take_profit_sell_pct: [30, 30, 40],
  });
  try {
    const ladder = page.getByRole("group", { name: "Take-Profit Levels" });
    await ladder.locator(".level-list-row").first().waitFor();
    assert.equal(await ladder.getAttribute("id"), "take-profit-levels");

    const rows = ladder.locator(".level-list-row");
    const shares = ["5.2", "78.9", "15.9"];
    for (const [index, share] of shares.entries()) {
      await rows.nth(index).locator(".level-list-sell").fill(share);
    }
    const save = page.locator(".config-card:has(#take-profit-levels) .config-card-save");
    assert.equal(await ladder.locator(".level-list-error").isVisible(), false);
    assert.equal(await save.isDisabled(), false, "shares that add up to 100% save");
  } finally {
    await close();
  }
});

test("a ladder that keeps a runner warns while no other rule can close it", async () => {
  const { page, close } = await openTrader(
    "take-profit",
    {
      take_profit_levels_pct: [25, 60],
      take_profit_sell_pct: [40, 30],
      trailing_stop_enabled: false,
      break_even_enabled: false,
      time_override_enabled: false,
      time_exit_stagnation_enabled: false,
    },
    { exitStrategiesOff: true }
  );
  try {
    const warning = page.locator("#take-profit-runner-warning");
    await warning.waitFor();
    assert.equal(
      plain(await warning.textContent()),
      "With the trailing stop, the break-even stop, both time exits and every exit strategy off, the runner closes only by the stop loss or by hand"
    );

    await page.locator("#trailing-enabled").evaluate((toggle) => {
      toggle.checked = true;
      toggle.dispatchEvent(new Event("change", { bubbles: true }));
    });
    assert.equal(await warning.isVisible(), false, "the trailing stop can close the runner");
  } finally {
    await close();
  }
});

/** The PATCH a click on the Save control of the card holding `id` sends, as JSON. */
async function savedBody(page, id) {
  const [request] = await Promise.all([
    page.waitForRequest(
      (candidate) =>
        candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
    ),
    page.locator(`.config-card:has(#${id}) .config-card-save`).click(),
  ]);
  return request.postDataJSON();
}

test("the DCA ladder shows each drop as a magnitude and saves its steps", async () => {
  const { page, close } = await openTrader("dca", {
    dca_levels_pct: [-10, -25],
    dca_size_multipliers: [0.5, 1],
  });
  try {
    const ladder = page.getByRole("group", { name: "DCA Levels" });
    const rows = ladder.locator(".level-list-row");
    const add = ladder.locator(".level-list-add");
    await page.waitForFunction(
      () => document.querySelector("#dca-levels .level-list-add")?.disabled === false
    );
    assert.equal(await ladder.getAttribute("id"), "dca-levels");
    const values = async (selector) =>
      rows.locator(selector).evaluateAll((inputs) => inputs.map((input) => input.value));
    assert.deepEqual(await values(".level-list-drop"), ["10", "25"]);
    assert.deepEqual(await values(".level-list-size"), ["0.5", "1"]);

    const drop = rows.first().locator(".level-list-drop");
    assert.deepEqual(
      await drop.evaluate((el) => ({
        min: el.min,
        max: el.max,
        prefix: el.closest(".number-field").querySelector(".input-prefix")?.textContent,
      })),
      { min: "1", max: "95", prefix: "−" }
    );
    const caption = ladder.locator(".level-list-rows > .level-list-caption > span");
    assert.deepEqual((await caption.allTextContents()).map(plain), [
      "",
      "Price Drop",
      "Buy × First Buy",
      "",
      "",
    ]);
    const bought = async () =>
      (await rows.locator(".level-list-amount").allTextContents()).map(plain);
    assert.deepEqual(await bought(), ["= 0.01 SOL", "= 0.02 SOL"]);

    await add.click();
    assert.deepEqual(await values(".level-list-drop"), ["10", "25", "50"]);
    assert.deepEqual(await values(".level-list-size"), ["0.5", "1", "1"]);
    await add.click();
    await add.click();
    assert.equal(await rows.count(), 5);
    assert.equal(await add.isDisabled(), true, "the metadata allows five steps");
    assert.deepEqual(await values(".level-list-drop"), ["10", "25", "50", "95", "95"]);
    assert.equal(await ladder.locator(".level-list-error").isVisible(), false);

    await rows.nth(4).locator(".level-list-remove").click();
    await rows.nth(3).locator(".level-list-remove").click();
    const body = await savedBody(page, "dca-levels");
    assert.deepEqual(body.dca_levels_pct, [-10, -25, -50]);
    assert.deepEqual(body.dca_size_multipliers, [0.5, 1, 1]);
    assert.equal(body.dca_measure_from, "average");
  } finally {
    await close();
  }
});

test("a DCA step under the minimum trade is flagged on its row", async () => {
  const { page, close } = await openTrader("dca", {
    dca_levels_pct: [-10, -20],
    dca_size_multipliers: [0.5, 0.1],
    trade_size_sol: 0.005,
  });
  try {
    const rows = page.locator("#dca-levels .level-list-row");
    await rows.nth(1).locator(".level-list-amount", { hasText: "SOL" }).waitFor();
    const flagged = await rows
      .locator(".level-list-amount")
      .evaluateAll((cells) => cells.map((cell) => cell.classList.contains("is-warning")));
    assert.deepEqual(flagged, [false, true], "0.0005 SOL is under the 0.001 SOL minimum");
    assert.equal(
      plain(await rows.nth(1).locator(".level-list-amount").getAttribute("title")),
      "Below the minimum trade of 0.001 SOL: this add is not sent"
    );
  } finally {
    await close();
  }
});

test("measured from the first entry, a DCA step must be deeper than the one before", async () => {
  const { page, close } = await openTrader("dca");
  try {
    const ladder = page.locator("#dca-levels");
    const rows = ladder.locator(".level-list-row");
    const error = ladder.locator(".level-list-error");
    await rows.first().waitFor();
    assert.equal(await error.isVisible(), false, "the average basis allows equal drops");

    await page.locator("#dca-measure-from").evaluate((select) => {
      select.value = "first_entry";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await rows.nth(1).locator(".level-list-drop").fill("10");
    assert.equal(
      plain(await error.textContent()),
      "Measured from the first entry, each add needs a deeper drop than the one before it"
    );
    const save = page.locator(".config-card:has(#dca-levels) .config-card-save");
    assert.equal(await save.isDisabled(), true);

    await rows.nth(1).locator(".level-list-drop").fill("20");
    assert.equal(await error.isVisible(), false);
    const body = await savedBody(page, "dca-levels");
    assert.equal(body.dca_measure_from, "first_entry");
    assert.deepEqual(body.dca_levels_pct, [-10, -20]);
  } finally {
    await close();
  }
});

test("the DCA capacity line shows a dash while the ladder is invalid and follows every change", async () => {
  const { page, close } = await openTrader("dca");
  try {
    const ladder = page.locator("#dca-levels");
    const rows = ladder.locator(".level-list-row");
    const capacity = page.locator("#dca-capacity");
    await rows.first().waitFor();
    await capacity.filter({ hasText: "SOL" }).waitFor();
    assert.match(plain(await capacity.textContent()), /^Most this position can hold: 0\.04 SOL/);

    await page.locator("#dca-measure-from").evaluate((select) => {
      select.value = "first_entry";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await ladder.locator(".level-list-add").click();
    await rows.nth(2).waitFor();
    assert.equal(await ladder.locator(".level-list-error").isVisible(), true);
    assert.equal(plain(await capacity.textContent()), "—");

    await page.locator("#dca-measure-from").evaluate((select) => {
      select.value = "average";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    assert.match(plain(await capacity.textContent()), /^Most this position can hold: \d.* SOL/);
  } finally {
    await close();
  }
});

test("the trailing stop states the minimum it locks and warns when it locks none", async () => {
  const { page, close } = await openTrader("trailing-stop", {
    trailing_stop_activation_pct: 10,
    trailing_stop_distance_pct: 5,
  });
  try {
    const locked = page.locator("#trail-locked-minimum");
    await locked.filter({ hasText: "Locked minimum" }).waitFor();
    assert.equal(plain(await locked.textContent()), "Locked minimum when it arms: +4.5%");
    assert.equal(await locked.evaluate((el) => el.classList.contains("is-warning")), false);

    await page.locator("#trail-activation").fill("5");
    await locked.filter({ hasText: "The stop sits" }).waitFor();
    assert.equal(
      plain(await locked.textContent()),
      "The stop sits at or below entry when it arms; it sells only while in profit. Turn on the break-even stop for a floor."
    );
    assert.equal(await locked.evaluate((el) => el.classList.contains("is-warning")), true);
  } finally {
    await close();
  }
});

test("the trailing stop's computed lines follow the typed activation and distance", async () => {
  const { page, close } = await openTrader("trailing-stop", {
    trailing_stop_activation_pct: 10,
    trailing_stop_distance_pct: 5,
  });
  try {
    const activation = page.locator("#activation-impact");
    const distance = page.locator("#distance-impact");
    await waitForPlain(page, "activation-impact", "+10%");
    assert.equal(plain(await distance.textContent()), "Exits at -5% from peak");

    await page.locator("#trail-activation").fill("25");
    await page.locator("#trail-distance").fill("8");
    await waitForPlain(page, "activation-impact", "+25%");
    await waitForPlain(page, "distance-impact", "-8%");
    assert.equal(plain(await activation.textContent()), "Starts trailing at +25% profit");
    assert.equal(plain(await distance.textContent()), "Exits at -8% from peak");

    await page.locator("#trail-activation").fill("");
    await page.locator("#trail-distance").fill("");
    await activation.filter({ hasText: "—" }).waitFor();
    await distance.filter({ hasText: "—" }).waitFor();
    // The worked example's profit lines show the dash alone, never a signed blank.
    for (const line of ["#example-activation-pct", "#example-peak-pct", "#example-exit-pct"]) {
      assert.equal(plain(await page.locator(line).textContent()), "—", line);
    }
  } finally {
    await close();
  }
});

test("the trailing bands list starts empty, adds bands above the activation and saves both lists", async () => {
  const { page, close } = await openTrader("trailing-stop", {
    trailing_stop_activation_pct: 20,
    trailing_stop_distance_pct: 10,
    trailing_stop_band_profit_pct: [],
    trailing_stop_band_distance_pct: [],
  });
  try {
    const bands = page.locator("#trailing-bands");
    const rows = bands.locator(".level-list-row");
    const add = bands.locator(".level-list-add");
    const error = bands.locator(".level-list-error");
    const save = page.locator(".config-card:has(#trailing-bands) .config-card-save");

    await page.waitForFunction(
      () => document.querySelector("#trailing-bands .level-list-add")?.disabled === false
    );
    assert.equal(await rows.count(), 0, "no band: the base distance applies at every profit");
    assert.equal(await bands.locator(".level-list-caption").isHidden(), true);

    await add.click();
    assert.equal(await bands.locator(".level-list-caption").isVisible(), true);
    assert.equal(await rows.nth(0).locator(".level-list-band-profit").inputValue(), "40");
    assert.equal(
      await rows.nth(0).locator(".level-list-band-distance").inputValue(),
      "10",
      "a first band starts from twice the activation at the base distance"
    );
    await add.click();
    await add.click();
    assert.equal(await rows.count(), 3);
    assert.equal(await rows.nth(2).locator(".level-list-band-profit").inputValue(), "160");
    assert.equal(await add.isDisabled(), true, "the metadata allows three bands");
    assert.equal(await error.isVisible(), false);

    await rows.nth(1).locator(".level-list-band-profit").fill("30");
    assert.equal(
      plain(await error.textContent()),
      "Each band needs a higher peak profit than the activation and the band before it"
    );
    assert.equal(await save.isDisabled(), true);
    await rows.nth(1).locator(".level-list-band-profit").fill("80");

    await rows.nth(0).locator(".level-list-band-distance").fill("40");
    assert.equal(plain(await error.textContent()), "Each band must trail by less than its peak profit");
    await rows.nth(0).locator(".level-list-band-distance").fill("8");
    await rows.nth(1).locator(".level-list-band-distance").fill("6");
    await rows.nth(2).locator(".level-list-remove").click();
    assert.equal(await error.isVisible(), false);

    // The bands must sit above the activation: raising it past a band is refused at once.
    await page.locator("#trail-activation").fill("50");
    await error.filter({ hasText: "higher peak profit" }).waitFor();
    await page.locator("#trail-activation").fill("20");
    await error.waitFor({ state: "hidden" });

    const body = await savedBody(page, "trailing-bands");
    assert.deepEqual(body.trailing_stop_band_profit_pct, [40, 80]);
    assert.deepEqual(body.trailing_stop_band_distance_pct, [8, 6]);
  } finally {
    await close();
  }
});

test("the last trailing band can be removed, leaving the base distance", async () => {
  const { page, close } = await openTrader("trailing-stop", {
    trailing_stop_band_profit_pct: [50],
    trailing_stop_band_distance_pct: [4],
  });
  try {
    const rows = page.locator("#trailing-bands .level-list-row");
    await rows.first().waitFor();
    await page.waitForFunction(
      () => document.querySelector("#trailing-bands .level-list-remove")?.disabled === false
    );
    await rows.first().locator(".level-list-remove").click();
    assert.equal(await rows.count(), 0);
    assert.equal(
      await page.locator("#trailing-bands .level-list-caption").isHidden(),
      true,
      "no column captions over an empty list"
    );
    const body = await savedBody(page, "trailing-bands");
    assert.deepEqual(body.trailing_stop_band_profit_pct, []);
    assert.deepEqual(body.trailing_stop_band_distance_pct, []);
  } finally {
    await close();
  }
});

test("the break-even stop states its rule and saves its two levels", async () => {
  const { page, close } = await openTrader("trailing-stop", {
    break_even_trigger_pct: 15,
    break_even_offset_pct: 1,
  });
  try {
    const summary = page.locator("#break-even-summary");
    await summary.filter({ hasText: "sells everything" }).waitFor();
    assert.equal(
      plain(await summary.textContent()),
      "Once the best price since the last buy is +15%, a fall back to +1% sells everything."
    );

    await page.locator("#break-even-offset").fill("2");
    await summary.filter({ hasText: "+2%" }).waitFor();
    assert.equal(
      plain(await summary.textContent()),
      "Once the best price since the last buy is +15%, a fall back to +2% sells everything."
    );
    const [request] = await Promise.all([
      page.waitForRequest(
        (candidate) =>
          candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
      ),
      page.locator(".config-card:has(#break-even-offset) .config-card-save").click(),
    ]);
    assert.equal(request.postDataJSON().break_even_offset_pct, 2);
  } finally {
    await close();
  }
});

test("the flat-position exit takes its bounds from the metadata and saves its fields", async () => {
  const { page, close } = await openTrader("time-rules", {
    time_exit_stagnation_minutes: 240,
    time_exit_stagnation_band_pct: 5,
  });
  try {
    const summary = page.locator("#time-stagnation-summary");
    await summary.filter({ hasText: "closes" }).waitFor();
    assert.equal(
      plain(await summary.textContent()),
      "A position held 4h or longer with its P&L within ±5% closes."
    );
    const bounds = (id) =>
      page.locator(`#${id}`).evaluate((el) => {
        const group = el.closest(".config-group");
        return {
          min: el.min,
          max: el.max,
          prefix: group.querySelector(".input-prefix")?.textContent ?? null,
          unit: group.querySelector(".input-unit").textContent,
        };
      });
    assert.deepEqual(await bounds("time-stagnation-minutes"), {
      min: "5",
      max: "43200",
      prefix: null,
      unit: "min",
    });
    assert.deepEqual(await bounds("time-stagnation-band"), {
      min: "0.5",
      max: "50",
      prefix: "±",
      unit: "%",
    });

    await page.locator("#time-stagnation-minutes").fill("90");
    await page.locator("#time-stagnation-band").fill("3");
    assert.equal(
      plain(await summary.textContent()),
      "A position held 1h 30m or longer with its P&L within ±3% closes."
    );
    const [request] = await Promise.all([
      page.waitForRequest(
        (candidate) =>
          candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
      ),
      page.locator(".config-card:has(#time-stagnation-minutes) .config-card-save").click(),
    ]);
    const body = request.postDataJSON();
    assert.equal(body.time_exit_stagnation_minutes, 90);
    assert.equal(body.time_exit_stagnation_band_pct, 3);
  } finally {
    await close();
  }
});

test("the time limit takes its bounds from the metadata and a blank field shows a dash", async () => {
  const { page, close } = await openTrader("time-rules", {
    time_override_duration: 48,
    time_override_unit: "hours",
    time_override_loss_threshold_percent: -25,
  });
  try {
    const loss = page.locator("#time-loss-impact");
    await loss.filter({ hasText: "25" }).waitFor();
    const bounds = (id) =>
      page.locator(`#${id}`).evaluate((el) => ({ min: el.min, max: el.max, value: el.value }));
    assert.deepEqual(await bounds("time-max-hold"), { min: "1", max: "43200", value: "48" });
    assert.deepEqual(await bounds("time-loss-threshold"), { min: "0", max: "100", value: "25" });

    await page.locator("#time-loss-threshold").fill("");
    assert.equal(plain(await loss.textContent()), "—");
    assert.equal(plain(await page.locator("#time-example-loss").textContent()), "—");
    await page.locator("#time-max-hold").fill("");
    assert.equal(plain(await page.locator("#time-conversion-hint").textContent()), "—");
    assert.equal(plain(await page.locator("#time-example-duration").textContent()), "—");
  } finally {
    await close();
  }
});

test("a line the page script writes carries no static catalog placeholder", async () => {
  const { page, close } = await openTrader("stop-loss");
  try {
    const scriptOwned = [
      "stop-loss-impact",
      "stop-loss-example-loss",
      "activation-impact",
      "example-activation-pct",
      "example-peak-pct",
      "example-exit-pct",
      "example-protected",
      "example-avoided",
      "break-even-summary",
      "time-conversion-hint",
      "time-loss-impact",
      "time-example-loss",
      "time-example-duration",
      "time-stagnation-summary",
    ];
    const placeholders = await page.$$eval(
      scriptOwned.map((id) => `#${id}`).join(","),
      (lines) => lines.filter((el) => el.hasAttribute("data-l10n-id")).map((el) => el.id)
    );
    assert.equal(
      await page.locator(scriptOwned.map((id) => `#${id}`).join(",")).count(),
      scriptOwned.length
    );
    assert.deepEqual(placeholders, []);
  } finally {
    await close();
  }
});
