// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Exit column of a copy task's holdings shows the break-even stop and the
 * stagnation exit from the holding's exit watch.
 *
 * A holding watched only by those two rules lists the break-even floor (with the profit
 * that arms it while it is not armed yet) and the flat band (with the wait before the
 * stagnation exit can act), and never reads as having no exit rule.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { copyFixture as fixture, openCopyPage, plainText as plain } from "../lib/copy_page.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const SIGN = "[-+−]";

/** An exit watch with every rule off. */
function emptyWatch() {
  return {
    stop_loss_price_native: null,
    stop_loss_armed_at: null,
    take_profit_price_native: null,
    trailing_activation_price_native: null,
    trailing_armed: false,
    trailing_stop_price_native: null,
    break_even_activation_price_native: null,
    break_even_armed: false,
    break_even_floor_price_native: null,
    time_rule_from: null,
    time_rule_price_native: null,
    stagnation_from: null,
    stagnation_band_low_price_native: null,
    stagnation_band_high_price_native: null,
  };
}

test("the break-even floor and the flat band show in the Exit column", async (t) => {
  const workspace = fixture("copy_workspace.json");
  workspace.exit_mode = "buy_only";
  const [waiting, armed, none] = workspace.paper_holdings;
  const at = (holding, pct) => holding.entry_price_native * (1 + pct / 100);

  waiting.exit_watch = {
    ...emptyWatch(),
    break_even_activation_price_native: at(waiting, 15),
    break_even_floor_price_native: at(waiting, 1),
    stagnation_from: new Date(Date.now() + 2 * 3600 * 1000).toISOString(),
    stagnation_band_low_price_native: at(waiting, -5),
    stagnation_band_high_price_native: at(waiting, 5),
  };
  armed.exit_watch = {
    ...emptyWatch(),
    break_even_activation_price_native: at(armed, 15),
    break_even_armed: true,
    break_even_floor_price_native: at(armed, 1),
  };
  none.exit_watch = emptyWatch();

  const page = await openCopyPage(browser, t, {
    routes: [
      ["**/api/copy-trading/tasks/*/workspace", (route) => route.fulfill({ json: workspace })],
    ],
  });
  await page.locator("#copy-tab-holdings").click();
  await page.waitForSelector("#copy-ws-panel .copy-table tbody tr");
  const exits = (
    await page.locator("#copy-ws-panel .copy-table tbody tr").evaluateAll((rows) =>
      rows.map((row) => row.querySelector(".copy-exit-list, .copy-warning-text")?.textContent || "")
    )
  ).map(plain);

  const waitingLine = exits.find((text) => text.includes("Floor") && text.includes("arms"));
  assert.ok(waitingLine, exits.join(" | "));
  assert.match(waitingLine, new RegExp(`Floor ${SIGN}1(\\.0)?% arms ${SIGN}15(\\.0)?%`));
  assert.match(waitingLine, new RegExp(`Flat ${SIGN}5(\\.0)?% to ${SIGN}5(\\.0)?% in \\S+`));
  assert.ok(!waitingLine.includes("No exit rule"), waitingLine);

  const armedLine = exits.find((text) => text.includes("Floor") && !text.includes("arms"));
  assert.ok(armedLine, exits.join(" | "));
  assert.match(armedLine, new RegExp(`^Floor ${SIGN}1(\\.0)?%$`));

  assert.equal(exits.filter((text) => text.includes("No exit rule")).length, 1, exits.join(" | "));
});
