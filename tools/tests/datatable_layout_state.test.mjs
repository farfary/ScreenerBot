// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a DataTable's saved column layout is versioned against its column
 * definitions, so a changed default order, pin or width reaches an installation
 * that already saved a layout, while the viewer's sort, filters, search,
 * visibility and page size survive.
 *
 * - The signature follows id order, the default pin and declared widths, and
 *   ignores labels and renderers.
 * - A saved state written under other definitions, or before signatures existed,
 *   loses exactly the layout keys.
 * - The table saves the signature it loaded under and reconciles every load
 *   through the one helper; it is the only place saved table state is read.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { SCRIPTS_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";
import {
  LAYOUT_STATE_KEYS,
  columnLayoutSignature,
  reconcileSavedLayout,
} from "../../src/webserver/templates/scripts/ui/data_table/layout_state.js";

const COLUMNS = [
  { id: "time", label: "Time", width: 140, floating: true },
  { id: "type", label: "Type", minWidth: 120 },
  { id: "signature", label: "Signature", width: 200 },
];

const SAVED = {
  sortColumn: "time",
  sortDirection: "desc",
  searchQuery: "abc",
  filters: { status: "failed" },
  visibleColumns: { type: false },
  serverPageSize: 100,
  columnOrder: ["signature", "time", "type"],
  columnWidths: { signature: 425 },
  tableWidth: 1200,
  userResizedColumns: { signature: true },
  floatingColumns: ["time"],
};

test("signature follows order, pin and declared widths only", () => {
  const base = columnLayoutSignature(COLUMNS);
  assert.equal(base, columnLayoutSignature(COLUMNS.map((c) => ({ ...c }))));
  assert.equal(
    base,
    columnLayoutSignature(COLUMNS.map((c) => ({ ...c, label: `${c.label}!`, render: () => "" })))
  );
  assert.notEqual(base, columnLayoutSignature([COLUMNS[2], COLUMNS[0], COLUMNS[1]]));
  assert.notEqual(
    base,
    columnLayoutSignature([{ ...COLUMNS[0], floating: false }, ...COLUMNS.slice(1)])
  );
  assert.notEqual(
    base,
    columnLayoutSignature([...COLUMNS.slice(0, 2), { ...COLUMNS[2], width: 180 }])
  );
  assert.notEqual(base, columnLayoutSignature(COLUMNS.slice(0, 2)));
});

test("a stale or unsigned layout is dropped and viewer choices are kept", () => {
  const signature = columnLayoutSignature(COLUMNS);
  for (const saved of [SAVED, { ...SAVED, layoutSignature: "00000000" }]) {
    const { state, reset } = reconcileSavedLayout(saved, signature);
    assert.equal(reset, true);
    for (const key of LAYOUT_STATE_KEYS) {
      assert.equal(key in state, false, `${key} survived a layout change`);
    }
    assert.equal("layoutSignature" in state, false);
    for (const key of [
      "sortColumn",
      "sortDirection",
      "searchQuery",
      "filters",
      "visibleColumns",
      "serverPageSize",
    ]) {
      assert.deepEqual(state[key], SAVED[key], `${key} was lost`);
    }
  }
});

test("a layout saved under the current definitions is kept whole", () => {
  const signature = columnLayoutSignature(COLUMNS);
  const { state, reset } = reconcileSavedLayout(
    { ...SAVED, layoutSignature: signature },
    signature
  );
  assert.equal(reset, false);
  assert.deepEqual(state, SAVED);
  assert.deepEqual(reconcileSavedLayout(null, signature), { state: null, reset: false });
});

test("the table saves its signature and reconciles every load", async () => {
  const source = readFileSync(resolve(SCRIPTS_ROOT, "ui/data_table.js"), "utf8");
  const load = source.slice(
    source.indexOf("  _loadState() {"),
    source.indexOf("  _seedFloatingColumns(hasSaved")
  );
  const save = source.slice(source.indexOf("  _saveState() {"));
  assert.match(load, /reconcileSavedLayout\(\s*AppState\.load\(this\.options\.stateKey\)/);
  assert.match(
    save.slice(0, save.indexOf("AppState.save")),
    /layoutSignature: this\._layoutSignature/
  );

  const readers = (await walk(SCRIPTS_ROOT))
    .filter((file) => file.endsWith(".js"))
    .filter((file) => /AppState\.load\(this\.options\.stateKey\)/.test(readFileSync(file, "utf8")));
  assert.deepEqual(
    readers.map(repoPath),
    ["src/webserver/templates/scripts/ui/data_table.js"],
    "saved DataTable state is read outside the reconciled load"
  );
});
