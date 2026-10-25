// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: contracts between dashboard scripts, styles and the data they read.
 *
 * - A position's exit price is the exit's fill price (`effective_exit_price`), the
 *   counterpart of the average entry; `exit_price` is the market price at the decision and
 *   stands in only when no fill is recorded.
 * - A DataTable column searches the text its cell shows (`searchValue`), not the raw field.
 * - A timestamp shown on the Wallets page and in Settings reads like the Positions table:
 *   short month, day, hour and minute, without year or seconds.
 * - The Security tab's fallback status and the Telegram tab's lockscreen read use the field
 *   and error envelope the API sends.
 * - A transaction instruction without a program id shows plain text, never an address.
 * - A range field sizes each bound from the shell's own gutter and wraps instead of
 *   clipping a long value under its unit.
 * - The Actions drawer starts in a loading state, never "No actions".
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

import { BASE_HTML, SCRIPTS_ROOT, STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

globalThis.window = globalThis.window || { addEventListener() {} };
globalThis.document = {
  addEventListener() {},
  readyState: "complete",
  createElement() {
    let text = "";
    return {
      set textContent(value) {
        text = String(value);
      },
      get innerHTML() {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
      },
    };
  },
};

const script = (relative) => `${SCRIPTS_ROOT}/${relative}`;
const read = (file) => fs.readFileSync(file, "utf8");

test("a position's exit price is the fill price, with the decision price as the last resort", async () => {
  const { applyUtilitiesMixin } = await import(script("ui/position_details/utilities.js"));
  class Dialog {}
  applyUtilitiesMixin(Dialog);
  const dialog = new Dialog();

  assert.equal(
    dialog._exitFillPrice({ effective_exit_price: 0.9, average_exit_price: 0.95, exit_price: 1 }),
    0.9
  );
  assert.equal(dialog._exitFillPrice({ average_exit_price: 0.95, exit_price: 1 }), 0.95);
  assert.equal(dialog._exitFillPrice({ exit_price: 1 }), 1);

  for (const file of ["header.js", "summary.js", "chart.js"]) {
    assert.doesNotMatch(
      read(script(`ui/position_details/${file}`)),
      /pos\.exit_price|pos\.average_exit_price/,
      `${file} reads the exit price through _exitFillPrice`
    );
  }
});

test("a DataTable column is searched by the text its cell shows", async () => {
  const { columnSearchText, rowMatchesSearch } = await import(
    script("ui/data_table/search.js")
  );
  const columns = [
    { id: "name", searchValue: (row) => (row.name === "ohlcv" ? "Candle Service" : row.name) },
    { id: "health" },
  ];
  const row = { name: "ohlcv", health: "healthy" };

  assert.equal(columnSearchText(columns[0], row), "candle service");
  assert.equal(columnSearchText(columns[1], row), "healthy");
  assert.equal(rowMatchesSearch(row, columns, "candle"), true);
  assert.equal(rowMatchesSearch(row, columns, "ohlcv"), false);
  assert.equal(rowMatchesSearch(row, columns, "heal"), true);
});

test("Wallets and Settings timestamps read like the Positions table", async () => {
  const { formatTimeFromSeconds, formatTimestamp } = await import(script("core/format.js"));
  const seconds = 1_790_000_000;
  const shared = { includeYear: false, includeSeconds: false };
  assert.equal(formatTimestamp(seconds * 1000, shared), formatTimeFromSeconds(seconds));

  for (const file of ["pages/wallets/watched.js", "pages/wallets/renderers.js", "ui/settings/updates_view.js"]) {
    const source = read(script(file));
    const calls = [...source.matchAll(/formatTimestamp\(([\s\S]*?)\}\s*\)/g)].map((m) => m[1]);
    assert.ok(calls.length > 0, `${file} formats a timestamp`);
    for (const call of calls) {
      assert.match(call, /includeYear:\s*false/, `${file} drops the year`);
      assert.match(call, /includeSeconds:\s*false/, `${file} drops the seconds`);
    }
  }
});

test("the settings tabs read the fields and envelope the API sends", () => {
  const security = read(script("ui/settings/security_tab.js"));
  assert.doesNotMatch(security, /auto_lock_timeout_minutes/);
  assert.match(security, /auto_lock_timeout_secs: 0/);

  const telegram = read(script("ui/settings/telegram_tab.js"));
  assert.doesNotMatch(telegram, /lockscreenData\.error/);
  assert.match(telegram, /apiErrorMessage\(lockscreenData,/);
});

test("an instruction without a program id shows plain text, not an address", async () => {
  const { TransactionDetailsDialog } = await import(script("ui/transaction_details_dialog.js"));
  const dialog = Object.create(TransactionDetailsDialog.prototype);
  const html = dialog._buildInstructionCard({ instruction_type: "Transfer", accounts: [] }, 0);

  assert.doesNotMatch(html, /ti-address/);
  assert.match(html, /instruction-program">Unknown</);
  assert.match(html, /detail-value">Unknown</);

  const withProgram = dialog._buildInstructionCard(
    { instruction_type: "Transfer", program_id: "11111111111111111111111111111111", accounts: [] },
    0
  );
  assert.match(withProgram, /ti-address/);
});

test("a range bound is sized from the shell gutter and wraps instead of clipping", () => {
  const rules = rulesIn(read(`${STYLES_ROOT}/components/config_fields.css`));
  const body = (selector) => rules.find((rule) => rule.selector === selector)?.body ?? "";

  assert.match(body(".config-field-range"), /flex-wrap:\s*wrap/);
  const shell = body(".config-field-range .config-field-bound .number-field");
  assert.match(shell, /width:\s*calc\(var\(--number-field-gutter\)\s*\+\s*12px\s*\+\s*9ch\)/);
  assert.doesNotMatch(shell, /flex:\s*1 1 0/);
});

test("the Actions drawer starts in a loading state", () => {
  const html = read(BASE_HTML);
  const list = /id="notificationList"[^>]*>([\s\S]*?)<\/div>\s*<\/div>/.exec(html)?.[1] ?? "";
  assert.match(list, /state-view-loading/);
  assert.doesNotMatch(list, /shell-notification-empty/);
});
