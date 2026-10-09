// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Versioning of a DataTable's saved column layout against its column definitions.
 *
 * Saved table state outlives the code that wrote it. Without a version, a saved
 * column order and saved widths keep overriding new column defaults forever, so a
 * layout change never reaches an existing installation. The table stores a
 * signature of the column definitions it saved under; when the definitions change,
 * the saved layout is dropped and the new defaults apply, while the viewer's
 * sort, filters, search, visibility and page size are kept.
 */

/** Saved state keys that describe the column layout and are dropped on a mismatch. */
export const LAYOUT_STATE_KEYS = Object.freeze([
  "columnOrder",
  "columnWidths",
  "tableWidth",
  "userResizedColumns",
  "floatingColumns",
]);

const finiteOrNull = (value) => (Number.isFinite(value) ? value : null);

/**
 * Signature of the layout-bearing parts of column definitions: id order, the
 * default pin, and the declared widths. Labels, renderers and types are left out,
 * so text and formatting changes keep a viewer's layout.
 * @param {Array<object>} columns
 * @returns {string}
 */
export function columnLayoutSignature(columns) {
  const shape = (columns || [])
    .filter((column) => column && column.id)
    .map((column) => [
      column.id,
      column.floating === true ? 1 : 0,
      finiteOrNull(column.width),
      finiteOrNull(column.minWidth),
      finiteOrNull(column.maxWidth),
    ]);
  const text = JSON.stringify(shape);
  // FNV-1a, 32-bit: a short stable token for the saved state, not a security hash.
  let hash = 0x811c9dc5;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

/**
 * Saved state ready to merge into the table state: the signature itself removed, and
 * the layout keys removed too when the state was written under other column
 * definitions (or before signatures existed).
 * @param {object|null} saved
 * @param {string} signature - the current `columnLayoutSignature`
 * @returns {{state: object|null, reset: boolean}}
 */
export function reconcileSavedLayout(saved, signature) {
  if (!saved) {
    return { state: saved, reset: false };
  }
  const { layoutSignature, ...state } = saved;
  const reset = layoutSignature !== signature;
  if (reset) {
    for (const key of LAYOUT_STATE_KEYS) {
      delete state[key];
    }
  }
  return { state, reset };
}
