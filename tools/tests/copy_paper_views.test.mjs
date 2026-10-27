// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Copy Trading page tells the quote-based paper model from legacy estimates.
 *
 * - Activity names each fill's execution model from its recorded `model`, not from the
 *   pool flag: a quote fill with `priced_from_pool: false` still shows its price, its
 *   target-price difference, the recorded router, quote times, waits and fee assumption,
 *   while a legacy row keeps its historical wording.
 * - Overview and Execution show the model card for a paper task only, from the
 *   workspace's own settings, and Overview lists legacy estimates read-only with labeled
 *   columns and a saved-price time.
 * - Settings load the stored milliseconds as seconds, save them back exactly, and refuse
 *   a delay outside 0..30 s or a queue size that is not an integer in 1..256.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { copyFixture as fixture, openCopyPage, plainText as plain } from "../lib/copy_page.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

async function openCopy(t, { workspace = null, routes = [] } = {}) {
  const served = workspace
    ? [
        ["**/api/copy-trading/tasks/*/workspace", (route) => route.fulfill({ json: workspace })],
        ...routes,
      ]
    : routes;
  return openCopyPage(browser, t, { routes: served });
}

/** The card of the open panel whose heading is `title`, or null. */
async function card(page, title) {
  // The panel shows a loading message until the workspace is read; a card means it is.
  await page.locator("#copy-ws-panel .copy-card").first().waitFor();
  const cards = page.locator("#copy-ws-panel .copy-card", {
    has: page.locator("h4", { hasText: new RegExp(`^${title}$`) }),
  });
  return (await cards.count()) ? cards.first() : null;
}

test("activity names each fill's model from its record, not from the pool flag", async (t) => {
  const page = await openCopy(t);
  await page.locator("#copy-tab-activity").click();
  await page.waitForSelector("#copy-ws-panel .copy-event");
  const rows = (
    await page.$$eval("#copy-ws-panel .copy-event:not(.copy-event-group)", (nodes) =>
      nodes.map((node) => ({
        title: node.querySelector(".copy-event-line strong")?.textContent ?? "",
        detail: node.querySelector(".copy-event-detail")?.textContent ?? "",
        meta: node.querySelector(".copy-event-meta")?.textContent ?? "",
      }))
    )
  ).map((row) => ({ title: plain(row.title), detail: plain(row.detail), meta: plain(row.meta) }));

  const quoteBuy = rows.find((row) => row.title === "Quote-based paper fill");
  assert.ok(quoteBuy, JSON.stringify(rows.map((row) => row.title)));
  assert.match(quoteBuy.detail, /^.+ at .+ · wallet bought .+ · target price difference \+1\.20%$/);
  assert.doesNotMatch(quoteBuy.detail, /slippage|no pool price/i);
  assert.match(quoteBuy.meta, /Router jupiter · quotes received .+ and .+/);
  assert.match(quoteBuy.meta, /Assumed wait 1\.0s · queued 0\.24s/);
  assert.match(quoteBuy.meta, /Estimated network and priority fees .*0\.00006.*counted once/);
  assert.match(quoteBuy.meta, /platform fee already in the quoted output/);

  const quoteExit = rows.find((row) => row.title.startsWith("Quote-based paper exit · "));
  assert.ok(quoteExit, "an exit keeps both its model and its rule");
  assert.match(quoteExit.detail, /^2,?416\.04 tokens for .+ at .+/);
  assert.match(quoteExit.meta, /Router jupiter/);

  const legacyBuy = rows.find((row) => row.title === "Legacy paper estimate");
  assert.ok(legacyBuy, "a historical fill keeps its legacy title");
  assert.match(legacyBuy.detail, /slippage \+0\.94%/);
  assert.doesNotMatch(legacyBuy.meta, /Router|Assumed wait/);

  const legacyExit = rows.find((row) => row.title.startsWith("Legacy paper exit estimate · "));
  assert.ok(legacyExit, "a historical rule exit keeps its legacy title and its rule");
  assert.ok(rows.some((row) => row.title === "Legacy paper exit estimate"));
});

test("the paper model card shows the workspace's own settings, for a paper task only", async (t) => {
  const workspace = fixture("copy_workspace.json");
  workspace.paper_execution = { model: "quote", landing_delay_ms: 2500, queue_capacity: 48 };
  const page = await openCopy(t, { workspace });

  const model = await card(page, "Paper execution model");
  assert.ok(model, "a paper task shows the model card");
  const text = plain(await model.textContent());
  assert.match(text, /Assumed landing wait\s*2\.5s/);
  assert.match(text, /Queue size per task\s*48/);
  assert.match(text, /counted once/);

  await page.locator("#copy-tab-execution").click();
  await page.waitForSelector("#copy-ws-panel .copy-metric");
  const execution = await card(page, "Paper execution model");
  assert.ok(execution, "the Execution tab discloses the same assumptions");
  const detail = plain(await execution.textContent());
  assert.match(detail, /Assumed landing wait\s*2\.5s/);
  assert.match(detail, /Detection delay is measured from the wallet's block/);
  const difference = plain(
    await page
      .locator("#copy-ws-panel .copy-card", { hasText: "Target price difference" })
      .first()
      .textContent()
  );
  assert.match(difference, /not the task's slippage tolerance/);
  assert.doesNotMatch(difference, /excluded/);

  const live = { ...workspace, mode: "live", effective_state: "live" };
  const livePage = await openCopy(t, { workspace: live });
  assert.equal(await card(livePage, "Paper execution model"), null);
  assert.equal(await card(livePage, "Legacy paper estimates"), null);
});

test("configured waits with millisecond precision are shown exactly", async (t) => {
  for (const [delay, shown] of [
    [1234, "1.234s"],
    [10_001, "10.001s"],
    [1000, "1.0s"],
    [1250, "1.25s"],
  ]) {
    const workspace = fixture("copy_workspace.json");
    workspace.paper_execution = { model: "quote", landing_delay_ms: delay, queue_capacity: 32 };
    const page = await openCopy(t, { workspace });
    const model = await card(page, "Paper execution model");
    assert.match(plain(await model.textContent()), new RegExp(`Assumed landing wait\\s*${shown}`));

    await page.locator("#copy-tab-holdings").click();
    await page.waitForSelector("#copy-ws-panel .copy-table tbody tr");
    await page.locator('[data-holding-action="close"]').first().click();
    const dialog = page.locator(".confirmation-dialog--visible");
    await dialog.waitFor();
    const message = plain(await dialog.locator(".confirmation-dialog__message").innerText());
    assert.ok(message.includes(`about ${shown} apart`), message);
  }

  const activity = fixture("copy_activity.json");
  for (const row of activity.activity) {
    const quote = row.outcome?.fill?.quote || row.outcome?.paper_fill?.quote;
    if (!quote) continue;
    quote.requested_delay_ms = 1234;
    quote.queue_duration_ms = 10_001;
  }
  const page = await openCopy(t, {
    routes: [
      ["**/api/copy-trading/tasks/*/activity**", (route) => route.fulfill({ json: activity })],
    ],
  });
  await page.locator("#copy-tab-activity").click();
  await page.waitForSelector("#copy-ws-panel .copy-event");
  const meta = plain(
    (await page.locator("#copy-ws-panel .copy-event-meta").allTextContents()).join(" ")
  );
  assert.match(meta, /Assumed wait 1\.234s · queued 10\.001s/);
});

test("legacy estimates are labeled, dated and read-only", async (t) => {
  const page = await openCopy(t);
  const legacy = await card(page, "Legacy paper estimates");
  assert.ok(legacy, "the populated book has legacy history");
  // textContent: the column headers are upper-cased by CSS only.
  const headers = (await legacy.locator("thead th").allTextContents()).map(plain);
  assert.deepEqual(headers, [
    "Token",
    "Tokens",
    "Remaining cost (SOL)",
    "Realized estimate (SOL)",
    "Last saved price (SOL)",
    "Remaining P&L at saved price (SOL)",
  ]);
  const rows = legacy.locator("tbody tr");
  assert.equal(await rows.count(), 2);
  const open = rows.nth(0);
  const closed = rows.nth(1);
  const savedPrice = plain(await open.locator("td").nth(4).innerText());
  assert.doesNotMatch(savedPrice, /^—/, "the saved price is shown");
  assert.match(savedPrice, /\d{1,2}:\d{2}:\d{2}/, "with the time it was saved");
  assert.match(plain(await open.locator("td").nth(5).innerText()), /^\+0\.0101$/);
  assert.equal(
    plain(await closed.locator("td").nth(5).innerText()),
    "—",
    "a closed holding is never valued at its saved price"
  );
  assert.match(
    plain(await legacy.textContent()),
    /not part of the current book's spend or holdings/
  );
  assert.equal(await legacy.locator("button").count(), 0, "the archive offers no action");
});

const CONFIG = {
  enabled: true,
  max_active_tasks: 10,
  default_slippage_pct: 2,
  default_mode: "paper",
  require_filter_pass: true,
  block_on_force_stop: true,
  latency_kill_switch_enabled: true,
  max_arrival_distance_ms: 4000,
  latency_window_size: 10,
  readiness_min_closed_rounds: 10,
  paper_queue_capacity: 24,
};

async function openSettings(t, config = CONFIG) {
  const patches = [];
  const page = await openCopy(t, {
    routes: [
      [
        "**/api/config/copy_trading",
        async (route) => {
          const request = route.request();
          if (request.method() === "PATCH") {
            patches.push(request.postDataJSON());
            await route.fulfill({ json: { success: true } });
          } else {
            await route.fulfill({ json: { data: config } });
          }
        },
      ],
    ],
  });
  const open = async () => {
    await page.locator("#copy-settings-open").click();
    await page.waitForSelector("#copy-settings-paper-queue:not(:disabled)");
  };
  const fill = async (queue) => {
    await page.locator("#copy-settings-paper-queue").fill(queue);
  };
  return { page, patches, open, fill };
}

test("settings save the paper queue and leave the landing delay to the shared paper setting", async (t) => {
  const { page, patches, open, fill } = await openSettings(t);
  await open();
  assert.equal(await page.locator("#copy-settings-paper-delay").count(), 0);
  assert.equal(
    await page.locator("#copy-settings-paper-delay-shared").getAttribute("data-l10n-id"),
    "copy-settings-paper-delay-shared"
  );
  assert.equal(await page.locator("#copy-settings-paper-queue").inputValue(), "24");

  for (const queue of ["256", "1"]) {
    await open();
    await fill(queue);
    await page.locator("#copy-settings-form button[type='submit']").click();
    await page.waitForFunction(() =>
      document.querySelector("#copy-settings")?.classList.contains("hidden")
    );
    const saved = patches[patches.length - 1];
    assert.equal(saved.paper_queue_capacity, Number(queue));
    assert.equal(saved.max_arrival_distance_ms, 4000, "the arrival limit round-trips unchanged");
    assert.equal("paper_landing_delay_ms" in saved, false, "copy saves no landing delay of its own");
  }
});

test("settings refuse a non-integer or out-of-range queue", async (t) => {
  const { page, patches, open, fill } = await openSettings(t);
  await open();
  const invalid = [
    ["2.5", "stepMismatch"],
    ["0", "rangeUnderflow"],
    ["257", "rangeOverflow"],
    ["", "valueMissing"],
  ];
  for (const [queue, reason] of invalid) {
    await fill(queue);
    await page.locator("#copy-settings-form button[type='submit']").click();
    const state = await page.$eval(
      "#copy-settings-paper-queue",
      (input, name) => input.validity[name],
      reason
    );
    assert.equal(state, true, `queue "${queue}" is ${reason}`);
    assert.equal(patches.length, 0, `nothing is saved for ${reason}`);
    assert.equal(
      await page.evaluate(() =>
        document.querySelector("#copy-settings")?.classList.contains("hidden")
      ),
      false,
      "the dialog stays open on invalid input"
    );
  }
});
