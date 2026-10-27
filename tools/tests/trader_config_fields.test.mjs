// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: what the Auto Trader rule fields compute from the loaded config.
 *
 * - The line under a field states its computed effect in one plain format, with no
 *   upper-case prefix ("IMPACT:", "CURRENT:", "READABLE:"); every DCA field has one.
 * - Every worked example starts from one position: a buy at the configured trade size
 *   (`trader.trade_size_sol`), never a fixed amount of its own.
 * - A loss threshold reads as a magnitude after a fixed "−" (Stop Loss, Time Exit
 *   loss, DCA threshold) while the config keeps its own sign: Stop Loss stores a
 *   positive percent, the other two a negative one. Load, display, edit and save
 *   round-trip at both bounds of each field.
 * - Sell Part First shows its Part to Sell field only while on; the field takes its bounds
 *   and unit from the metadata, saves through its card, and the stop-loss example sells
 *   that part at the threshold and the rest if the price stays below the stop.
 * - The take-profit ladder edits its steps in place up to the metadata's step limit,
 *   names what it cannot save and keeps Save disabled until it can, and saves its
 *   levels and shares as their two config lists. Its group takes its accessible name
 *   from its label, and decimal shares that add up to the whole position save.
 * - A ladder that keeps a runner warns that only the stop loss or the user can close it
 *   while the trailing stop, the time exit and every exit strategy are off.
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
const EXIT_STRATEGIES = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/trader/strategies_exit.json`, "utf8")
);
const TRADE_SIZE = 0.02;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

/**
 * The trader page on `tab`, served `trader` merged over the recorded trader config.
 * `exitStrategiesOff` serves the recorded exit strategies with every one switched off.
 */
async function openTrader(tab, trader = {}, { exitStrategiesOff = false } = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const config = { ...CONFIG, trader: { ...CONFIG.trader, trade_size_sol: TRADE_SIZE, ...trader } };
  const onApi = (request) => {
    if (request.url.pathname === "/api/config" && request.method === "GET") {
      return { body: JSON.stringify(config) };
    }
    if (
      exitStrategiesOff &&
      request.url.pathname === "/api/strategies" &&
      request.url.searchParams.get("type") === "EXIT"
    ) {
      const items = EXIT_STRATEGIES.items.map((strategy) => ({ ...strategy, enabled: false }));
      return { body: JSON.stringify({ ...EXIT_STRATEGIES, items }) };
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

/** The computed line under the field `id`. */
const line = async (page, id) =>
  plain(
    await page.$eval(
      `#${id}`,
      (input) => input.closest(".config-group").querySelector(".config-impact").textContent
    )
  );

test("a field's computed line is plain text, and every DCA field has one", async () => {
  const { page, close } = await openTrader("dca");
  try {
    await page.locator("#dca-size-impact", { hasText: "SOL" }).waitFor();
    const prefixes = await page.$$eval(".trader-tab-content .config-impact", (lines) =>
      lines
        .map((el) => el.textContent.trim())
        .filter((text) => /^\p{Lu}{3,}\s*:/u.test(text) || text === "")
    );
    assert.deepEqual(prefixes, [], "no shouted prefix and no empty line");
    assert.equal(await page.locator(".impact-label").count(), 0);

    assert.equal(
      await line(page, "dca-threshold"),
      "Buys more when the position is down 10% from entry"
    );
    assert.equal(await line(page, "dca-max-count"), "At most 2 extra entries per position");
    assert.equal(await line(page, "dca-size"), "Each extra entry buys 0.01 SOL");
    assert.match(await line(page, "dca-cooldown"), /^At least 30\D.*between extra entries$/);

    await page.fill("#dca-max-count", "1");
    assert.equal(await line(page, "dca-max-count"), "At most one extra entry per position");
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
    assert.equal(
      await text("#dca-example"),
      "Buy 0.02 SOL → each further -10% adds 0.01 SOL, up to 2 times"
    );
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
  {
    tab: "dca",
    id: "dca-threshold",
    key: "dca_threshold_pct",
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
    await page.waitForFunction(() => document.querySelector("#stop-loss-partial-pct").value === "40");
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
        (candidate) => candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
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
    await add.click();
    assert.equal(await rows.count(), 4);
    assert.equal(await add.isDisabled(), true, "the metadata allows four steps");
    assert.equal(await error.isVisible(), false);

    await rows.nth(3).locator(".level-list-sell").fill("50");
    assert.equal(await error.isVisible(), true);
    assert.equal(plain(await error.textContent()), "The steps together can sell at most 100%");
    assert.equal(await save.isDisabled(), true);

    await rows.nth(1).locator(".level-list-profit").fill("20");
    assert.equal(plain(await error.textContent()), "Each step needs a higher profit level than the one before it");

    await rows.nth(3).locator(".level-list-remove").click();
    await rows.nth(1).locator(".level-list-profit").fill("60");
    assert.equal(await rows.count(), 3);
    assert.equal(await error.isVisible(), false);
    assert.equal(await save.isDisabled(), false);

    const [request] = await Promise.all([
      page.waitForRequest(
        (candidate) => candidate.method() === "PATCH" && candidate.url().endsWith("/api/config/trader")
      ),
      save.click(),
    ]);
    const body = request.postDataJSON();
    assert.deepEqual(body.take_profit_levels_pct, [25, 60, 61]);
    assert.deepEqual(body.take_profit_sell_pct, [40, 30, 5]);
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
      time_override_enabled: false,
    },
    { exitStrategiesOff: true }
  );
  try {
    const warning = page.locator("#take-profit-runner-warning");
    await warning.waitFor();
    assert.equal(
      plain(await warning.textContent()),
      "With the trailing stop, the time exit and every exit strategy off, the runner closes only by the stop loss or by hand"
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
