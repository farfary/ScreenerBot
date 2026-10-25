// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the wallet bulk import speaks the contract of `routes/wallets/import.rs`.
 *
 * The preview response is `{ session_id, preview }` where `preview.rows` are the
 * validated rows (`RowValidationResult`, tagged by `status` as "Valid" / "Invalid") and
 * the counts are `valid_count`, `invalid_count` (duplicates included) and
 * `duplicate_count`. The execute request is `{ session_id, mapping, options }` with the
 * `*_col` mapping fields of `ColumnMappingRequest`. The recorded preview below is the
 * shape `ImportPreviewResponse` serializes to.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";

globalThis.window = globalThis.window || { addEventListener() {} };
globalThis.document = { addEventListener() {}, readyState: "complete" };

const { buildImportRequest, previewRowView, selectionFromPreview, summarizePreview } =
  await import("../../src/webserver/templates/scripts/pages/wallets/bulk_operations.js");

const RECORDED = {
  session_id: "7b0e57a2-2c63-4d4b-8e0f-0f2d1c6a9a11",
  preview: {
    headers: ["Label", "Secret", "Memo"],
    column_mapping: {
      name_col: 0,
      private_key_col: 1,
      notes_col: null,
      address_col: null,
      is_valid: true,
    },
    total_rows: 3,
    valid_count: 1,
    invalid_count: 2,
    duplicate_count: 1,
    rows: [
      {
        status: "Valid",
        row_num: 2,
        name: "Alpha",
        address: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
      },
      { status: "Invalid", row_num: 3, errors: ["Wallet already exists"] },
      { status: "Invalid", row_num: 4, errors: ["Invalid private key", "Name is required"] },
    ],
  },
};

test("the detected mapping seeds the column selection", () => {
  assert.deepEqual(selectionFromPreview(RECORDED), { name_col: 0, private_key_col: 1 });
});

test("the execute request carries the session, the *_col mapping and the options", () => {
  const selection = { ...selectionFromPreview(RECORDED), notes_col: 2 };
  assert.deepEqual(buildImportRequest(RECORDED, selection), {
    session_id: RECORDED.session_id,
    mapping: { name_col: 0, private_key_col: 1, notes_col: 2, address_col: null },
    options: { skip_invalid: true, skip_duplicates: true, set_first_as_main: false },
  });
});

test("the summary splits the server's duplicates out of its invalid count", () => {
  assert.deepEqual(summarizePreview(RECORDED.preview), { valid: 1, duplicate: 1, invalid: 1 });
});

test("preview rows read the serde-tagged status", () => {
  const [valid, duplicate, invalid] = RECORDED.preview.rows.map(previewRowView);
  assert.equal(valid.valid, true);
  assert.equal(valid.name, "Alpha");
  assert.equal(duplicate.valid, false);
  assert.deepEqual(duplicate.errors, ["Wallet already exists"]);
  assert.deepEqual(invalid.errors, ["Invalid private key", "Name is required"]);
});
