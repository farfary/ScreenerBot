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
const TRADE_SIZE = 0.02;

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

/** The trader page on `tab`, served `trader` merged over the recorded trader config. */
async function openTrader(tab, trader = {}) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, "populated");
  const config = { ...CONFIG, trader: { ...CONFIG.trader, trade_size_sol: TRADE_SIZE, ...trader } };
  const onApi = (request) => {
    if (request.url.pathname === "/api/config" && request.method === "GET") {
      return { body: JSON.stringify(config) };
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
      roi: { "#roi-example-entry": "0.02 SOL", "#roi-example-target": "0.024 SOL" },
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
