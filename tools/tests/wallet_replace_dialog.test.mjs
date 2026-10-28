// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the trading wallet is replaced from the Wallets page through the setup
 * credential form in replace mode. The "Replace wallet" action sits in the trading
 * wallet toolbar beside the key export and on no other tab. The dialog first states
 * what the replacement stops, moves and keeps, and shows the credential fields only
 * after that step is acknowledged; the key field shows once "Import a wallet" is chosen,
 * since nothing is preselected. The RPC field may stay empty: the request then carries
 * no endpoints and the server keeps the configured ones. A refused replacement shows the server's own reason,
 * with its arguments, and a staged one waits for the restart. Other Wallets marks a
 * replaced wallet with the date and backup directory its stores were moved to. A
 * replacement a restart abandoned is shown once with its reason and then dismissed.
 *
 * Run with `npm run test:js`.
 */

import test, { after } from "node:test";
import assert from "node:assert/strict";

import { chromium } from "playwright";

import { serveDashboard, PAGE_READY } from "../lib/dashboard_host.mjs";
import { readFileSync } from "node:fs";

import { createApiHandler, FIXTURES_ROOT, loadIndex } from "../lib/dashboard_fixtures.mjs";

const REPLACE = '[data-btn-id="wt-replace"]';
const tab = (id) => `#subTabsContainer [data-tab-id="${id}"]`;
const IMPORT = "#setup-dialog-wallet-source-import";
const KEY = "1".repeat(88);
const RPC = "https://rpc.example.com";
const VALIDATION = { valid: true, validation_id: "v-1", wallet_address: "NewWa11et1111" };

const browser = await chromium.launch({ headless: true });
after(() => browser.close());
const [wallets, shell] = await Promise.all([loadIndex("wallets"), loadIndex("shell")]);

/**
 * Opens the Wallets page; `answers` maps "METHOD /path" to a `{ status, body }` reply, and
 * every API request is appended to `seen` as "METHOD /path".
 */
async function openWallets(t, answers = {}, seen = []) {
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const api = createApiHandler([wallets, shell], "populated");
  const host = await serveDashboard(context, {
    locale: "en",
    onApi: (request) => {
      seen.push(`${request.method} ${request.url.pathname}`);
      const answer = answers[`${request.method} ${request.url.pathname}`];
      return answer
        ? { status: answer.status ?? 200, body: JSON.stringify(answer.body) }
        : api.onApi(request);
    },
  });
  t.after(async () => {
    await context.close();
    await host.close();
  });
  const page = await context.newPage();
  page.setDefaultTimeout(15000);
  await page.goto(`${host.origin}/wallets`);
  await page.waitForSelector(PAGE_READY);
  return page;
}

const visible = (page, selector) =>
  page.$$eval(selector, (found) => found.some((node) => node.getClientRects().length > 0));

/** Opens the replace dialog, leaves its confirmation step and submits valid credentials. */
async function submitReplacement(page) {
  await page.click(tab("main"));
  await page.click(REPLACE);
  await page.click('.setup-dialog [data-action="submit"]');
  await page.check(IMPORT);
  await page.fill("#setup-dialog-wallet-private-key", KEY);
  await page.fill("#setup-dialog-rpc-urls", RPC);
  await page.click('.setup-dialog [data-action="submit"]');
}

test("the replace action sits in the trading wallet toolbar only", async (t) => {
  const page = await openWallets(t);
  await page.click(tab("main"));
  await page.waitForSelector(REPLACE, { state: "visible" });
  assert.equal(await visible(page, '[data-btn-id="wt-export-key"]'), true);

  for (const other of ["other", "watched"]) {
    await page.click(tab(other));
    assert.equal(await visible(page, REPLACE), false, `the replace action shows on ${other}`);
  }
});

test("replace mode states the consequences before it asks for the new key", async (t) => {
  const page = await openWallets(t);
  await page.click(tab("main"));
  await page.click(REPLACE);
  await page.waitForSelector(".setup-dialog .setup-dialog-confirm", { state: "visible" });

  const before = await page.evaluate(() => ({
    items: document.querySelectorAll(".setup-dialog-confirm-list li").length,
    fields: document.querySelector(".setup-dialog-fields").getClientRects().length,
    title: document.querySelector("#setup-dialog-title span").textContent.trim(),
    submit: document.querySelector('.setup-dialog [data-action="submit"]').textContent.trim(),
    expected: {
      title: window.I18n.t("setup-dialog-replace-title"),
      submit: window.I18n.t("common-action-continue"),
    },
  }));
  assert.equal(before.items, 6);
  assert.equal(before.fields, 0, "the credential fields show before the confirmation");
  assert.equal(before.title, before.expected.title);
  assert.equal(before.submit, before.expected.submit);

  await page.click('.setup-dialog [data-action="submit"]');
  await page.waitForSelector(".setup-dialog-source .setup-choice-input", { state: "visible" });
  assert.equal(
    await page.evaluate(() => document.querySelectorAll(".setup-dialog-source :checked").length),
    0,
    "no wallet source is preselected"
  );
  await page.check(IMPORT);
  await page.waitForSelector("#setup-dialog-wallet-private-key", { state: "visible" });
  const after = await page.evaluate(() => ({
    confirm: document.querySelector(".setup-dialog-confirm").getClientRects().length,
    submit: document.querySelector('.setup-dialog [data-action="submit"]').textContent.trim(),
    expected: window.I18n.t("setup-dialog-replace-submit"),
  }));
  assert.equal(after.confirm, 0, "the confirmation is hidden once the credential fields show");
  assert.equal(after.submit, after.expected);
});

test("replace mode submits without an RPC value so the configured endpoints are kept", async (t) => {
  const page = await openWallets(t, {
    "POST /api/initialization/validate": { body: VALIDATION },
    "POST /api/wallets/replace": {
      body: {
        restart_required: true,
        instance_id: "instance-1",
        previous_wallet: "MainWa11etAAA1111111111111111111111111111111",
        new_wallet: VALIDATION.wallet_address,
        backup_directory: "/data/backups/wallet-reset-20261027T100000Z-MainWa11",
      },
    },
  });
  const sent = {};
  page.on("request", (request) => {
    const path = new URL(request.url()).pathname;
    if (request.method() === "POST" && path.startsWith("/api/")) {
      sent[path] = request.postDataJSON();
    }
  });
  await page.evaluate(() => {
    window.waitForScreenerBotRestart = () => new Promise(() => {});
  });
  await page.click(tab("main"));
  await page.click(REPLACE);
  await page.click('.setup-dialog [data-action="submit"]');
  await page.check(IMPORT);
  await page.fill("#setup-dialog-wallet-private-key", KEY);
  await page.click('.setup-dialog [data-action="submit"]');
  await page.waitForSelector("#setup-dialog-status.success", { state: "visible" });

  assert.equal(
    await page.getAttribute("#setup-dialog-rpc-urls", "aria-invalid"),
    null,
    "an empty RPC field is not an error in replace mode"
  );
  assert.deepEqual(sent["/api/initialization/validate"]?.rpc_urls, []);
  assert.deepEqual(sent["/api/wallets/replace"]?.rpc_urls, []);
});

test("a refused replacement shows the server's reason with its arguments", async (t) => {
  const refusal = {
    error: {
      code: "OPEN_POSITIONS",
      message: "2 open positions",
      text: {
        id: "errors-wallets-replace-open-positions",
        args: { count: { type: "number", value: 2 } },
      },
    },
  };
  const page = await openWallets(t, {
    "POST /api/initialization/validate": { body: VALIDATION },
    "POST /api/wallets/replace": { status: 409, body: refusal },
  });
  await submitReplacement(page);
  await page.waitForSelector("#setup-dialog-status.error", { state: "visible" });

  const shown = await page.evaluate(() => ({
    status: document.querySelector("#setup-dialog-status").textContent.trim(),
    expected: window.I18n.t("errors-wallets-replace-open-positions", { count: 2 }),
    open: Boolean(document.querySelector(".setup-dialog")),
  }));
  assert.equal(shown.status, shown.expected);
  assert.match(shown.status, /2/);
  assert.equal(shown.open, true, "a refusal keeps the dialog open");
});

test("a staged replacement waits for the restart into the new wallet", async (t) => {
  const page = await openWallets(t, {
    "POST /api/initialization/validate": { body: VALIDATION },
    "POST /api/wallets/replace": {
      body: {
        restart_required: true,
        instance_id: "instance-1",
        previous_wallet: "MainWa11etAAA1111111111111111111111111111111",
        new_wallet: VALIDATION.wallet_address,
        backup_directory: "/data/backups/wallet-reset-20261027T100000Z-MainWa11",
      },
    },
  });
  await page.evaluate(() => {
    window.restartWaits = [];
    window.waitForScreenerBotRestart = (instanceId) => {
      window.restartWaits.push(instanceId);
      return new Promise(() => {});
    };
  });
  await submitReplacement(page);
  await page.waitForSelector("#setup-dialog-status.success", { state: "visible" });

  const shown = await page.evaluate(() => ({
    waits: window.restartWaits,
    status: document.querySelector("#setup-dialog-status").textContent.trim(),
    expected: window.I18n.t("setup-dialog-replace-saved"),
    key: document.querySelector("#setup-dialog-wallet-private-key").value,
  }));
  assert.deepEqual(shown.waits, ["instance-1"]);
  assert.equal(shown.status, shown.expected);
  assert.equal(shown.key, "", "the key is cleared from the form once it was sent");
});

test("other wallets mark a replaced wallet with its backup", async (t) => {
  const page = await openWallets(t);
  await page.click(tab("other"));
  await page.waitForSelector("#other-wallets-table-container .wallet-replaced-path");
  const paths = await page.$$eval("#other-wallets-table-container .wallet-replaced-path", (found) =>
    found.map((node) => node.textContent.trim())
  );
  assert.deepEqual(paths, ["/data/backups/wallet-reset-20261020T091500Z-SecondWa"]);
});

test("a replacement a restart abandoned is shown once with its reason, then dismissed", async (t) => {
  const backups = JSON.parse(readFileSync(`${FIXTURES_ROOT}/wallets/wallets_backups.json`, "utf8"));
  backups.abandoned = {
    previous_wallet: "MainWa11etAAA1111111111111111111111111111111",
    new_wallet: VALIDATION.wallet_address,
    reason: "open_positions",
    count: 2,
    abandoned_at: "2026-10-27T10:00:00Z",
  };
  const seen = [];
  const page = await openWallets(
    t,
    {
      "GET /api/wallets/backups": { body: backups },
      "POST /api/wallets/replacement/dismiss": { body: { dismissed: true } },
    },
    seen
  );
  const toast = page.locator(".toast", { hasText: "2" }).first();
  await toast.waitFor({ state: "visible" });

  const shown = await page.evaluate(() => ({
    title: window.I18n.t("wallets-replace-abandoned-title"),
    message: window.I18n.t("wallets-replace-abandoned-open-positions", { count: 2 }),
  }));
  assert.equal((await toast.locator(".toast__title").textContent()).trim(), shown.title);
  assert.equal((await toast.locator(".toast__message").textContent()).trim(), shown.message);
  const DISMISS = "POST /api/wallets/replacement/dismiss";
  for (let attempt = 0; attempt < 50 && !seen.includes(DISMISS); attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.deepEqual(
    seen.filter((request) => request === DISMISS),
    [DISMISS],
    "the page asks the server once to forget the shown outcome"
  );
});
