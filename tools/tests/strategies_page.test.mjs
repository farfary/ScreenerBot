// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Strategies page sends and receives a strategy whole.
 *
 * The list's enable toggle once sent a body without the strategy's timeframe, so the
 * route reset a strategy saved on another timeframe to the default one. Import put
 * the parsed file into memory but never loaded its rules into the editor.
 *
 * - The list toggle changes only `enabled`, through the route that touches nothing
 *   else of the stored strategy.
 * - Importing an export opens it as a new unsaved strategy, switched off, with its
 *   rules in the editor, and exporting that strategy writes the same file back.
 * - An import file whose condition does not apply to the strategy type is refused
 *   with the reason and leaves the editor as it was.
 * - Deleting the strategy in the editor empties the editor rather than opening an
 *   unsaved new strategy in its place.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { createApiHandler, loadIndex } from "../lib/dashboard_fixtures.mjs";
import { branch, leaf } from "../lib/strategy_rules.mjs";

const ID = "8f3c2a1e-4b6d-4c7a-9e21-3a5f7b9d1c01";
const RULES = branch(
  "AND",
  leaf("Rsi", { period: 21, timeframe: "1h" }),
  { ...leaf("VolumeSpike"), disabled: true },
  branch("NOT", branch("OR", leaf("CandleSize"), leaf("PriceToMA")))
);
const STORED = {
  id: ID,
  name: "Momentum breakout",
  description: "Buys a 1h breakout.",
  strategy_type: "ENTRY",
  enabled: true,
  priority: 7,
  timeframe: "15m",
  rules: RULES,
  parameters: {},
  created_at: "2026-09-10T08:30:00+00:00",
  updated_at: "2026-10-01T12:15:00+00:00",
  author: null,
  version: 1,
};
const EXPORTED = {
  format: "screenerbot.strategy",
  version: 1,
  strategy: {
    name: "Imported momentum",
    description: "Imported from a file.",
    strategy_type: "ENTRY",
    timeframe: "1h",
    priority: 12,
    rules: RULES,
    parameters: {},
  },
};

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
const fixtures = createApiHandler([await loadIndex("trader"), await loadIndex("shell")], "populated");
const host = await serveDashboard(context, {
  locale: "en",
  onApi: async (request) => {
    if (request.url.pathname === `/api/strategies/${ID}`) {
      return { body: JSON.stringify(request.method === "GET" ? STORED : {}) };
    }
    if (request.url.pathname === `/api/strategies/${ID}/enabled`) {
      return { body: JSON.stringify({ id: ID, enabled: false, version: 2 }) };
    }
    return fixtures.onApi(request);
  },
});
after(async () => {
  await context.close();
  await host.close();
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
await page.goto(`${host.origin}/trader`);
await page.waitForSelector(PAGE_READY);
await page.click('#subTabsContainer [data-tab-id="strategies"]');
await page.waitForSelector("#strategy-list .strategy-item");

/** Text without the Unicode isolates Fluent places around arguments. */
const plain = (text) => text.replace(/[\u2068\u2069]/g, "");

async function importFile(file) {
  const [chooser] = await Promise.all([page.waitForEvent("filechooser"), page.click("#import-strategy")]);
  await chooser.setFiles({
    name: "strategy.json",
    mimeType: "application/json",
    buffer: Buffer.from(typeof file === "string" ? file : JSON.stringify(file)),
  });
}

test("the list toggle changes only enabled, through the enabled route", async () => {
  const writes = [];
  // Only the strategy resource counts: the shell also POSTs reads such as
  // `/api/ui-state/load`, which a slow page start can issue inside this window.
  const record = (request) => {
    if (request.method() !== "GET" && new URL(request.url()).pathname.startsWith("/api/strategies")) {
      writes.push(request);
    }
  };
  page.on("request", record);
  const [request] = await Promise.all([
    page.waitForRequest((candidate) => candidate.method() === "PATCH"),
    page.click(`[data-strategy-id="${ID}"] [data-action="toggle"]`),
  ]);
  await page.getByText("Strategy disabled").first().waitFor();
  page.off("request", record);
  assert.equal(new URL(request.url()).pathname, `/api/strategies/${ID}/enabled`);
  assert.deepEqual(request.postDataJSON(), { enabled: false });
  assert.deepEqual(writes, [request], "no other write reaches the stored strategy");
});

test("an imported export opens as a new unsaved strategy and exports back unchanged", async () => {
  await importFile(EXPORTED);
  await page.waitForSelector("#editor-header.dirty");

  assert.equal(await page.inputValue("#strategy-name"), "Imported momentum");
  assert.equal(await page.inputValue("#strategy-priority"), "12");
  assert.equal(await page.inputValue("#strategy-timeframe"), "1h");
  assert.equal(await page.inputValue("#strategy-description"), "Imported from a file.");
  assert.equal(await page.isChecked("#strategy-enabled-toggle"), false, "an import arrives switched off");
  assert.equal(await page.locator("#conditions-list .rule-row").count(), 4);
  assert.equal(await page.locator("#conditions-list .rule-row.is-off").count(), 1);
  assert.equal(await page.locator("#conditions-list .rule-group.is-inverted").count(), 1);

  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.click("#export-strategy"),
  ]);
  assert.equal(download.suggestedFilename(), "imported-momentum.json");
  assert.deepEqual(JSON.parse(readFileSync(await download.path(), "utf8")), EXPORTED);
});

test("an import with a condition the strategy type cannot use is refused with the reason", async () => {
  const refused = JSON.parse(JSON.stringify(EXPORTED));
  refused.strategy.name = "Refused";
  refused.strategy.rules = branch("AND", leaf("PositionHoldingTime"));

  await importFile(refused);
  await page.getByText("cannot be used in this type of strategy").first().waitFor();
  assert.equal(await page.inputValue("#strategy-name"), "Imported momentum");

  await importFile("not json");
  await page.getByText("The file is not valid JSON").first().waitFor();
});

test("a condition is added from the catalog, checked as it is edited and kept out of Save while invalid", async () => {
  const saves = [];
  page.on("request", (request) => {
    if (request.method() === "POST" && new URL(request.url()).pathname === "/api/strategies") {
      saves.push(request);
    }
  });

  await page.click("#create-strategy");
  await page.click('.type-card[data-type="ENTRY"]');
  await page.getByText("Discard unsaved changes?").waitFor();
  await page.getByRole("button", { name: "Discard" }).click();
  await page.waitForSelector("#editor-header.dirty");
  assert.match(plain(await page.textContent("#rule-problems")), /1 problem to fix: Add at least one condition/);

  await page.click("#add-condition");
  assert.equal(await page.evaluate(() => document.activeElement?.id), "condition-search");
  assert.equal(await page.locator("#catalog-list .catalog-entry").count(), 13);
  assert.equal(await page.locator("#catalog-list .catalog-entry:disabled").count(), 3);
  assert.equal(
    await page.locator('.catalog-entry[data-condition-type="PositionProfit"]').isDisabled(),
    true
  );
  await page.fill("#condition-search", "relative strength");
  await page.keyboard.press("Enter");
  await page.locator("#condition-catalog-modal").waitFor({ state: "hidden" });

  const row = page.locator("#conditions-list .rule-row.is-open");
  assert.equal(await row.count(), 1);
  assert.equal(await page.evaluate(() => document.activeElement?.closest(".rule-body") !== null), true);
  assert.equal(await page.locator("#rule-problems").isHidden(), true);

  await page.fill('[data-key="period"]', "");
  assert.equal(await page.getAttribute('[data-key="period"]', "aria-invalid"), "true");
  assert.equal(await row.locator(".param-field-error:not(:empty)").textContent(), "Enter a number");
  assert.match(plain(await page.textContent("#rule-problems")), /1 problem to fix: .*RSI Period: Enter a number/);

  await page.click("#save-strategy");
  assert.equal(saves.length, 0, "Save does not call the API while a field is invalid");
  assert.equal(await page.evaluate(() => document.activeElement?.dataset.key), "period");

  await page.fill('[data-key="period"]', "500");
  assert.match(plain(await page.textContent("#rule-problems")), /Must be between 2 and 100/);
  await page.fill('[data-key="period"]', "14");
  assert.equal(await page.locator("#rule-problems").isHidden(), true);

  await page.locator('#conditions-list [data-action="duplicate"]').first().click();
  assert.equal(await page.locator("#conditions-list .rule-row").count(), 2);
  await page.locator('#conditions-list .rule-row').first().locator('[data-action="move-down"]').click();
  assert.deepEqual(
    await page.evaluate(() => {
      const active = document.activeElement;
      const rows = [...document.querySelectorAll("#conditions-list .rule-row")];
      return [active?.dataset.action, rows.indexOf(active?.closest(".rule-row"))];
    }),
    ["move-up", 1],
    "focus stays on the moved row, on the button that is still enabled"
  );

  await page.click("#add-group");
  const group = page.locator("#conditions-list .rule-group.is-nested");
  assert.equal(await group.count(), 1);
  assert.match(plain(await group.textContent()), /^\s*Match[\s\S]*of these/);
  await group.locator('[data-action="add-condition"]').click();
  await page.fill("#condition-search", "buy");
  await page.keyboard.press("Enter");
  assert.equal(await group.locator(".rule-row").count(), 1);
  assert.equal(await page.locator("#conditions-list > .rule-group > .rule-children > .rule-row").count(), 2);
});

test("deleting the strategy in the editor returns the editor to its empty state", async () => {
  await page.click(`[data-strategy-id="${ID}"] .strategy-item-name`);
  await page.getByText("Discard unsaved changes?").waitFor();
  await page.getByRole("button", { name: "Discard" }).click();
  await page.waitForFunction((name) => document.querySelector("#strategy-name")?.value === name, STORED.name);

  const [request] = await Promise.all([
    page.waitForRequest((candidate) => candidate.method() === "DELETE"),
    (async () => {
      await page.click(`[data-strategy-id="${ID}"] [data-action="delete"]`);
      await page.locator(".confirmation-dialog").getByRole("button", { name: "Delete" }).click();
    })(),
  ]);
  assert.equal(new URL(request.url()).pathname, `/api/strategies/${ID}`);
  await page.locator("#editor-empty").waitFor({ state: "visible" });
  assert.equal(await page.locator("#editor-header").isHidden(), true);
  assert.equal(await page.locator("#editor-header.dirty").count(), 0);
});
