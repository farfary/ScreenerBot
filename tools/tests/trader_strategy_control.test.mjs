// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Strategy Control sub-tab of the Auto Trader page.
 *
 * - A lane with no strategy shows the standard empty state with a New Strategy action,
 *   which opens the new-strategy dialog with that lane's kind chosen; its count reads
 *   neutral instead of in the lane's colour.
 * - A card leaves out a description it does not have, keeps its own height, and has an
 *   Edit action that opens the strategy in the editor.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { leaf } from "../lib/strategy_rules.mjs";

const ID = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c01";
const STORED = {
  id: ID,
  name: "Momentum breakout",
  description: "Buys a 1h breakout.",
  strategy_type: "ENTRY",
  enabled: true,
  priority: 7,
  timeframe: "15m",
  rules: leaf("Rsi", { period: 21, timeframe: "1h" }),
  parameters: {},
  created_at: "2026-09-10T08:30:00+00:00",
  updated_at: "2026-10-01T12:15:00+00:00",
  author: null,
  version: 1,
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const indexes = [await loadIndex("trader"), await loadIndex("shell")];

/** The Strategy Control sub-tab over the populated or the empty fixtures. */
async function openStrategyControl(mode) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler(indexes, mode);
  const onApi = (request) =>
    request.url.pathname === `/api/strategies/${ID}`
      ? { body: JSON.stringify(STORED) }
      : api.onApi(request);
  const host = await serveDashboard(context, { locale: "en", onApi });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/trader`);
  await page.waitForSelector(PAGE_READY);
  await page.click('#subTabsContainer [data-tab-id="strategy-control"]');
  const close = async () => {
    await context.close();
    await host.close();
  };
  return { page, close };
}

test("an empty lane offers New Strategy for its kind, with a neutral count", async () => {
  const { page, close } = await openStrategyControl("empty");
  try {
    const action = page.locator("#exit-strategies .state-view-empty .state-view-action");
    await action.waitFor();
    assert.equal((await action.textContent()).trim(), "New Strategy");

    assert.ok(
      await page.$eval("#strategy-exit-enabled-label", (el) => el.classList.contains("is-empty")),
      "the count of an empty lane is marked empty"
    );
    // Read once any colour transition has finished.
    await page.waitForFunction(() => {
      const count = document.getElementById("strategy-exit-enabled-label");
      return count.getAnimations().length === 0;
    });
    const colors = await page.$eval("#strategy-exit-enabled-label", (el) => [
      getComputedStyle(el).color,
      getComputedStyle(document.querySelector(".card-subtitle")).color,
    ]);
    assert.equal(colors[0], colors[1], "the count of an empty lane reads neutral");

    await action.click();
    await page.locator("#create-strategy-modal:not([hidden])").waitFor();
    assert.deepEqual(
      await page.$$eval("#create-strategy-modal .type-card", (cards) =>
        cards.map((card) => [card.dataset.type, card.getAttribute("aria-pressed")])
      ),
      [
        ["ENTRY", "false"],
        ["EXIT", "true"],
      ]
    );
  } finally {
    await close();
  }
});

test("a card shows what it has, keeps its height and opens in the editor", async () => {
  const { page, close } = await openStrategyControl("populated");
  try {
    await page.locator("#entry-strategies .strategy-control-item").first().waitFor();
    const cards = await page.$$eval(".strategy-control-item", (items) =>
      items.map((item) => {
        // The same card laid out on its own, outside the lane's grid.
        const alone = item.cloneNode(true);
        alone.style.width = `${item.getBoundingClientRect().width}px`;
        document.body.appendChild(alone);
        const natural = alone.getBoundingClientRect().height;
        alone.remove();
        return {
          name: item.querySelector(".strategy-control-name").textContent.trim(),
          description: item.querySelector(".strategy-control-description")?.textContent.trim(),
          edit: item.querySelector("[data-edit-strategy]")?.textContent.trim(),
          height: Math.round(item.getBoundingClientRect().height),
          natural: Math.round(natural),
        };
      })
    );
    const guard = cards.find((card) => card.name === "Low liquidity guard");
    assert.equal(guard.description, undefined, "no placeholder for a missing description");
    for (const card of cards) {
      assert.equal(card.edit, "Edit", `${card.name}: an Edit action`);
      assert.equal(card.height, card.natural, `${card.name}: sized to its content`);
    }

    await page
      .locator(".strategy-control-item", { hasText: "Momentum breakout" })
      .locator("[data-edit-strategy]")
      .click();
    await page.locator("#strategies-tab").waitFor();
    await page.waitForFunction(
      () => document.getElementById("strategy-name")?.value === "Momentum breakout"
    );
  } finally {
    await close();
  }
});
