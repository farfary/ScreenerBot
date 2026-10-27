// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the Positions table shows one book at a time.
 *
 * - The live book reads `/api/positions`, the paper book reads `/api/positions/paper` only,
 *   and the paper book has no archived view to read.
 * - A paper row offers only its paper close: no manual add, sell, archive or delete, and a
 *   closed paper round offers nothing.
 * - The toolbar switch names both books, Live first and default, and the Paper option
 *   carries its bare glyph.
 * - The empty paper book says where paper rounds come from.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { installEnglishI18n } from "../lib/dashboard_i18n.mjs";

installEnglishI18n();
const { POSITION_BOOKS, bookViews, paperActionCell, paperEmptyState, positionsUrl } = await import(
  new URL("../../src/webserver/templates/scripts/pages/positions/book.js", import.meta.url)
);
const { TRADING_MODE_ICONS } = await import(
  new URL("../../src/webserver/templates/scripts/ui/trading_mode.js", import.meta.url)
);

const esc = (text) =>
  String(text).replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);
const plain = (text) => String(text).replace(/[⁦-⁩]/g, "");

test("each book reads its own rows and the paper book has no archive", () => {
  assert.deepEqual(POSITION_BOOKS, ["live", "paper"]);
  for (const status of ["open", "closed", "archived"]) {
    assert.equal(positionsUrl("live", status), `/api/positions?status=${status}&limit=0`);
  }
  assert.equal(positionsUrl("paper", "open"), "/api/positions/paper?status=open&limit=0");
  assert.equal(positionsUrl("paper", "closed"), "/api/positions/paper?status=closed&limit=0");
  assert.equal(positionsUrl("paper", "archived"), null);
  for (const status of ["open", "closed"]) {
    assert.doesNotMatch(positionsUrl("paper", status), /^\/api\/positions\?/);
  }
});

test("an open paper row offers only its paper close", () => {
  const cell = paperActionCell({ id: 7, mint: "Mint", transaction_exit_verified: false }, esc);
  const actions = [...cell.matchAll(/data-action="([^"]+)"/g)].map((match) => match[1]);
  assert.deepEqual(actions, ["paper-close"]);
  assert.match(cell, /data-id="7"/);
  assert.match(cell, /title="Close paper position"/);
  assert.match(cell, /aria-label="Close paper position"/);
  for (const live of ["add", "sell", "remove", "restore", "delete"]) {
    assert.doesNotMatch(cell, new RegExp(`data-action="${live}"`));
  }
});

test("a closed or unidentified paper row offers nothing", () => {
  assert.equal(paperActionCell({ id: 7, transaction_exit_verified: true }, esc), "");
  assert.equal(paperActionCell({ mint: "Mint" }, esc), "");
  assert.equal(paperActionCell(null, esc), "");
});

test("the switch names both books, Live first, and Paper carries its bare glyph", () => {
  const changes = [];
  const views = bookViews("live", (book) => changes.push(book));
  assert.equal(views.id, "book");
  assert.equal(views.value, "live");
  assert.equal(views.ariaLabel, "Trading book");
  assert.deepEqual(
    views.options.map(({ value, label, icon }) => ({ value, label, icon })),
    [
      { value: "live", label: "Live", icon: undefined },
      { value: "paper", label: "Paper", icon: TRADING_MODE_ICONS.paper },
    ]
  );
  views.onChange("paper");
  assert.deepEqual(changes, ["paper"]);
});

test("the empty paper book says where paper rounds come from", () => {
  const open = paperEmptyState("open");
  assert.equal(open.title, "No paper positions");
  assert.equal(
    plain(open.message),
    "Set a strategy to Paper on Strategy Control to test it without spending SOL."
  );
  assert.deepEqual(paperEmptyState("closed"), open);
  assert.equal(paperEmptyState("archived").title, "No archived paper positions");
});
