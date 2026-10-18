// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a manual paper close is one inline, reconciled operation.
 *
 * Closing a holding shows "Close pending" on its row at once, keeps the row and refuses a
 * second close until the backend has a result. A response lost to the transport is not a
 * failed fill: while the workspace reports the close pending it stays pending, and the
 * final result is read from the workspace and the recorded activity, never from elapsed
 * time. Every request and answer is scripted, so no timer or network is involved; the
 * workspace is refreshed by toggling the compare view, the same path a poll takes.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { copyFixture, openCopyPage, plainText as plain } from "../lib/copy_page.mjs";

const browser = await chromium.launch({ headless: true });
after(() => browser.close());

const MINT = "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm";
const FILL_PRICE = 0.0081;

function deferred() {
  let resolve;
  const promise = new Promise((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

/** The holdings tab on a scripted backend: `state` is what the next workspace read returns. */
async function openHoldings(t) {
  const state = { workspace: copyFixture("copy_workspace.json"), activity: [], workspaces: {} };
  const closes = [];
  const resets = [];
  const arrived = deferred();
  const page = await openCopyPage(browser, t, {
    routes: [
      [
        "**/api/copy-trading/tasks/*/workspace",
        (route) => {
          const id = Number(/tasks\/(\d+)\/workspace/.exec(route.request().url())[1]);
          return route.fulfill({ json: state.workspaces[id] ?? state.workspace });
        },
      ],
      [
        "**/api/copy-trading/tasks/*/reset",
        (route) => {
          resets.push(route.request().url());
          return route.fulfill({ json: { removed_decisions: 0 } });
        },
      ],
      [
        "**/api/copy-trading/tasks/*/activity**",
        (route) => route.fulfill({ json: { activity: state.activity, next_before: null } }),
      ],
      [
        "**/api/copy-trading/tasks/*/holdings/*/close",
        (route) => {
          closes.push(route);
          arrived.resolve(route);
        },
      ],
    ],
  });
  await page.locator("#copy-tab-holdings").click();
  await page.waitForSelector("#copy-ws-panel .copy-table tbody tr");
  return { page, state, closes, resets, request: arrived.promise };
}

/** The same mint held by another paper task, opened as a different round. */
function otherTask(id) {
  const workspace = copyFixture("copy_workspace.json");
  workspace.id = id;
  workspace.paper_holdings[0].opened_at = "2026-10-08T08:00:00.000Z";
  return workspace;
}

/** Selects a task from the list without a pointer, so an open dialog cannot intercept it. */
const selectTask = (page, id) =>
  page.evaluate((taskId) => document.querySelector(`[data-task-id="${taskId}"]`).click(), id);

const dialogOf = (page) => page.locator(".confirmation-dialog--visible");

async function until(predicate) {
  for (let attempt = 0; attempt < 400; attempt += 1) {
    if (predicate()) return;
    await new Promise((done) => setTimeout(done, 25));
  }
  assert.fail("the condition was not reached");
}

const closeButton = (page) => page.locator(`[data-holding-action="close"][data-mint="${MINT}"]`);

/**
 * The close button once its row has re-rendered idle. A settled close shows its toast before
 * the page reloads, so a read taken right after the toast can still see the pending row.
 */
async function idleCloseButton(page) {
  const button = page.locator(
    `[data-holding-action="close"][data-mint="${MINT}"]:not([disabled])`,
    { hasText: /^\s*Close\s*$/ }
  );
  await button.waitFor();
  return button;
}

async function confirmClose(page) {
  await closeButton(page).click();
  const dialog = page.locator(".confirmation-dialog--visible");
  await dialog.waitFor();
  const message = plain(await dialog.locator(".confirmation-dialog__message").innerText());
  await dialog.locator('[data-action="confirm"]').click();
  return message;
}

/** The first holding as the workspace reports it after the close finished. */
function closedHolding(state) {
  const holding = state.workspace.paper_holdings[0];
  holding.open = false;
  holding.close_pending = false;
  holding.closed_at = "2026-10-07T04:00:00.000Z";
}

const soldRow = (id) => ({
  id,
  task_id: 1,
  kind: "paper_sell_observed",
  outcome: {
    outcome: "paper_sell_observed",
    task_id: 1,
    target_address: "CopyTargetAAA1111111111111111111111111111111",
    target_signature: `paper-exit-${id}`,
    mint: MINT,
    target_token_amount: 0,
    target_sol_amount: 0,
    exit_percentage: null,
    transaction_signature: null,
    error: null,
    telemetry: { fill_price_sol: FILL_PRICE, backfill: false },
    paper_fill: {
      token_amount: 12.638,
      market_price_sol: 0.00821,
      priced_from_pool: false,
      fill_price_sol: FILL_PRICE,
      gross_sol: 0.1024,
      referral_fee_sol: 0,
      network_fee_sol: 0.00006,
      priority_fee_sol: 0,
      net_proceeds_sol: 0.10234,
      model: "quote",
    },
    exit_rule: "manual",
  },
  created_at: "2026-10-07T04:00:00.000Z",
});

const skippedRow = (id, kind, signature = `manual-close-skip:${MINT}:${id}`) => ({
  id,
  task_id: 1,
  kind: "skipped",
  outcome: {
    outcome: "skipped",
    task_id: 1,
    signature,
    mint: MINT,
    reason: { kind },
    telemetry: null,
    paper: { model: "quote", epoch: 1 },
  },
  created_at: "2026-10-07T04:00:00.000Z",
});

/** A sale a policy rule or the mirrored wallet produced, never a manual close. */
const policySaleRow = (id, exitRule) => {
  const row = soldRow(id);
  row.outcome.exit_rule = exitRule;
  row.outcome.exit_percentage = 50;
  return row;
};

/** Re-reads the workspace the way a poll does. */
async function refresh(page) {
  await page.locator("#copy-compare-open").click();
  await page.locator("#copy-compare-open").click();
}

const toast = (page, text) => page.locator(".toast", { hasText: text });
const toastText = async (page) => plain((await page.locator(".toast").allInnerTexts()).join(" "));

test("a close is pending at once, refuses a duplicate and reports the recorded fill", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  assert.equal(plain(await closeButton(page).innerText()), "Close");

  const message = await confirmClose(page);
  assert.match(message, /two router quotes about 1\.0s apart/);
  assert.match(message, /stays open and unchanged/);
  assert.doesNotMatch(message, /write.?off/i);

  const route = await request;
  const button = closeButton(page);
  assert.equal(plain(await button.innerText()), "Close pending");
  assert.ok(await button.isDisabled(), "the row stays and cannot be closed twice");
  await button.click({ force: true });
  assert.equal(await page.locator(".confirmation-dialog--visible").count(), 0);
  assert.equal(closes.length, 1, "no second request is sent");
  assert.equal(await page.locator("#copy-ws-panel .copy-table tbody tr").count(), 3);

  closedHolding(state);
  await route.fulfill({
    json: { task_id: 1, mint: MINT, fill_price_native: FILL_PRICE },
  });
  const sold = toast(page, "Sold at");
  await sold.waitFor();
  assert.match(plain(await sold.innerText()), /closed.*Sold at .*0\.0081/);
  await page.waitForFunction(
    () => document.querySelectorAll("#copy-ws-panel .copy-table tbody tr").length === 2
  );
});

test("a lost response stays pending while the backend says so, then reports the result", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;

  state.workspace.paper_holdings[0].close_pending = true;
  await route.abort("timedout");
  const notice = toast(page, "queued or running");
  await notice.waitFor();
  assert.match(
    plain(await notice.innerText()),
    /Close pending.*Its result will appear in Activity/
  );
  assert.equal(plain(await closeButton(page).innerText()), "Close pending");
  assert.ok(await closeButton(page).isDisabled(), "no new close is offered for a lost response");
  assert.equal(closes.length, 1, "the lost close is never sent again");
  assert.doesNotMatch(await toastText(page), /could not be closed|Sold at|not confirmed/);

  closedHolding(state);
  state.activity = [soldRow(315)];
  await refresh(page);
  const sold = toast(page, "Sold at");
  await sold.waitFor();
  assert.match(plain(await sold.innerText()), /Sold at .*0\.0081/);
  assert.equal(closes.length, 1);
});

test("a lost response whose recorded result is a skip reports that reason", async (t) => {
  const { page, state, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;

  state.workspace.paper_holdings[0].close_pending = true;
  await route.abort("timedout");
  await toast(page, "queued or running").waitFor();

  state.workspace.paper_holdings[0].close_pending = false;
  state.activity = [skippedRow(316, "paper_quote_unfillable")];
  await refresh(page);
  const failed = toast(page, "could not be closed");
  await failed.waitFor();
  assert.match(
    plain(await failed.innerText()),
    /Holding could not be closed.*The quote cannot produce a usable paper fill/
  );
  assert.ok(
    await (await idleCloseButton(page)).isEnabled(),
    "the unchanged holding can be closed again"
  );
});

test("a lost response with no recorded result is reported as not confirmed", async (t) => {
  const { page, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;
  await route.abort("connectionreset");

  const unconfirmed = toast(page, "Close result not confirmed");
  await unconfirmed.waitFor();
  assert.match(plain(await unconfirmed.innerText()), /Check Activity before closing again/);
  assert.doesNotMatch(await toastText(page), /could not be closed|Sold at/);
  assert.ok(await (await idleCloseButton(page)).isEnabled());
});

test("a refusal from the backend is a failed close and leaves the holding open", async (t) => {
  const { page, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;
  await route.fulfill({
    status: 409,
    json: { error: { message: "Paper execution is unavailable" } },
  });

  const failed = toast(page, "could not be closed");
  await failed.waitFor();
  assert.match(plain(await failed.innerText()), /Paper execution is unavailable/);
  assert.ok(await (await idleCloseButton(page)).isEnabled());
});

test("a holding without a pool mark can still be closed by quote", async (t) => {
  const { page, state } = await openHoldings(t);
  Object.assign(state.workspace.paper_holdings[0], {
    mark_price_native: null,
    market_value_native: null,
    unrealized_pnl_native: null,
    unrealized_pnl_pct: null,
  });
  await refresh(page);
  await page.getByText("No pool price").first().waitFor();
  assert.ok(await closeButton(page).isEnabled());
  assert.equal(await page.getByRole("button", { name: /write off/i }).count(), 0);
  const message = await confirmClose(page);
  assert.match(message, /two router quotes/);
});

test("a close confirmed after another task was selected closes the captured task's holding", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  state.workspaces[3] = otherTask(3);
  await closeButton(page).click();
  await dialogOf(page).waitFor();
  await selectTask(page, 3);
  await closeButton(page).waitFor();
  await dialogOf(page).locator('[data-action="confirm"]').click();

  const route = await request;
  assert.match(route.request().url(), /\/tasks\/1\/holdings\//, "the captured task is closed");
  assert.equal(closes.length, 1);
  assert.equal(
    plain(await closeButton(page).innerText()),
    "Close",
    "the same mint held by the selected task is not pending"
  );
  assert.ok(await closeButton(page).isEnabled());
  await route.fulfill({ json: { task_id: 1, mint: MINT, fill_price_native: FILL_PRICE } });
  await toast(page, "Sold at").waitFor();
  assert.ok(await closeButton(page).isEnabled());
});

test("a pending close stays with its task and round through rerenders and task switches", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  state.workspaces[3] = otherTask(3);
  await confirmClose(page);
  const route = await request;

  await selectTask(page, 3);
  await closeButton(page).waitFor();
  assert.equal(plain(await closeButton(page).innerText()), "Close");
  assert.ok(await closeButton(page).isEnabled(), "another task's round is its own");

  await selectTask(page, 1);
  await closeButton(page).waitFor();
  assert.equal(plain(await closeButton(page).innerText()), "Close pending");
  await refresh(page);
  await closeButton(page).waitFor();
  assert.equal(plain(await closeButton(page).innerText()), "Close pending");
  assert.ok(await closeButton(page).isDisabled());
  assert.equal(closes.length, 1);
  await route.abort("connectionreset");
  await toast(page, "Close result not confirmed").waitFor();
});

test("a reset confirmed after another task was selected resets the captured task", async (t) => {
  const { page, state, resets } = await openHoldings(t);
  state.workspaces[3] = otherTask(3);
  await page.locator('[data-holding-action="reset"]').click();
  await dialogOf(page).waitFor();
  await selectTask(page, 3);
  await closeButton(page).waitFor();
  await dialogOf(page).locator('[data-action="confirm"]').click();

  await until(() => resets.length > 0);
  assert.equal(resets.length, 1);
  assert.match(resets[0], /\/tasks\/1\/reset$/, "the task shown at the click is reset");
});

test("clicking close repeatedly during its confirmation sends one request", async (t) => {
  const { page, closes, request } = await openHoldings(t);
  await closeButton(page).click();
  await dialogOf(page).waitFor();
  await page.evaluate((mint) => {
    const button = document.querySelector(`[data-holding-action="close"][data-mint="${mint}"]`);
    button.click();
    button.click();
  }, MINT);
  assert.equal(await dialogOf(page).count(), 1, "one confirmation is open");
  await dialogOf(page).locator('[data-action="confirm"]').click();
  const route = await request;
  assert.equal(closes.length, 1);
  await route.abort("connectionreset");
  await toast(page, "Close result not confirmed").waitFor();
  assert.equal(closes.length, 1);
});

test("a reopened round of the mint does not inherit the old close or its result", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;
  state.workspace.paper_holdings[0].close_pending = true;
  await route.abort("timedout");
  await toast(page, "queued or running").waitFor();

  const old = state.workspace.paper_holdings[0];
  closedHolding(state);
  state.workspace.paper_holdings.push({
    ...old,
    open: true,
    close_pending: false,
    opened_at: "2026-10-07T05:00:00.000Z",
    closed_at: null,
  });
  state.activity = [soldRow(315)];
  await refresh(page);
  const unconfirmed = toast(page, "Close result not confirmed");
  await unconfirmed.waitFor();
  assert.doesNotMatch(await toastText(page), /Sold at/);
  assert.ok(
    await (await idleCloseButton(page)).isEnabled(),
    "the new round is its own, not pending"
  );
  assert.equal(closes.length, 1);
});

test("a timeout that finishes while another tab is shown is reconciled", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;
  state.workspace.paper_holdings[0].close_pending = true;
  await route.abort("timedout");
  await toast(page, "queued or running").waitFor();

  await page.locator("#copy-tab-activity").click();
  closedHolding(state);
  state.activity = [soldRow(315)];
  await refresh(page);
  await toast(page, "Sold at").waitFor();
  assert.equal(await page.locator("#copy-tab-activity").getAttribute("aria-selected"), "true");
  assert.equal(closes.length, 1);
});

test("only a manual close result of this task and mint answers a lost close", async (t) => {
  const ambiguous = [
    ["an unrelated buy skip", [skippedRow(317, "paper_queue_full", "target-signature-317")]],
    ["a policy partial sale", [policySaleRow(318, "take_profit")]],
    ["a mirrored partial sale", [policySaleRow(319, null)]],
    [
      "a manual sale of another mint",
      [{ ...soldRow(320), outcome: { ...soldRow(320).outcome, mint: "OtherMint" } }],
    ],
  ];
  for (const [label, rows] of ambiguous) {
    const { page, state, request } = await openHoldings(t);
    await confirmClose(page);
    const route = await request;
    state.activity = rows;
    await route.abort("connectionreset");
    await toast(page, "Close result not confirmed").waitFor();
    assert.doesNotMatch(await toastText(page), /could not be closed|Sold at/, label);
    assert.ok(
      await (await idleCloseButton(page)).isEnabled(),
      `${label}: the open holding stays open`
    );
  }
});

test("each manual close skip reports its reason and a holding gone without a sale is not a result", async (t) => {
  for (const prefix of ["manual-close-skip", "manual-close-queue", "manual-close-unavailable"]) {
    const { page, state, request } = await openHoldings(t);
    await confirmClose(page);
    const route = await request;
    state.activity = [skippedRow(316, "paper_quote_unfillable", `${prefix}:${MINT}:316`)];
    await route.abort("connectionreset");
    const failed = toast(page, "could not be closed");
    await failed.waitFor();
    assert.match(plain(await failed.innerText()), /usable paper fill/, prefix);
  }

  const { page, state, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;
  state.workspace.paper_holdings.shift();
  await route.abort("connectionreset");
  await toast(page, "Close result not confirmed").waitFor();
  assert.doesNotMatch(
    await toastText(page),
    /Sold at|could not be closed|Activity has the recorded fill/
  );
});

test("a round seen closed with no recorded fill is reported without a price", async (t) => {
  const { page, state, request } = await openHoldings(t);
  await confirmClose(page);
  const route = await request;
  closedHolding(state);
  await route.abort("connectionreset");
  const closed = toast(page, "Activity has the recorded fill");
  await closed.waitFor();
  assert.doesNotMatch(await toastText(page), /Sold at/);
});

test("a close confirmed after the round reopened sends the confirmed round and sells nothing", async (t) => {
  const { page, state, closes, request } = await openHoldings(t);
  const confirmed = state.workspace.paper_holdings[0].opened_at;
  const reopened = "2026-10-09T09:00:00.123456Z";
  await closeButton(page).click();
  await dialogOf(page).waitFor();

  state.workspace.paper_holdings[0].opened_at = reopened;
  await page.evaluate(() => {
    const toggle = document.querySelector("#copy-compare-open");
    toggle.click();
    toggle.click();
  });
  await page.waitForFunction(
    ([mint, openedAt]) =>
      document.querySelector(`[data-holding-action="close"][data-mint="${mint}"]`)?.dataset
        .openedAt === openedAt,
    [MINT, reopened]
  );
  await dialogOf(page).locator('[data-action="confirm"]').click();

  const route = await request;
  assert.equal(closes.length, 1);
  assert.deepEqual(JSON.parse(route.request().postData()), { opened_at: confirmed });
  assert.notEqual(confirmed, reopened);

  await route.fulfill({
    status: 400,
    json: { error: { message: "Paper execution is unavailable" } },
  });
  await toast(page, "could not be closed").waitFor();
  assert.doesNotMatch(await toastText(page), /Sold at/);
  assert.ok(await (await idleCloseButton(page)).isEnabled(), "the reopened round stays open");
  assert.equal(closes.length, 1, "the refused close is not sent again");
});
