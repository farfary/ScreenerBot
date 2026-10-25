// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Client-side search of a DataTable: which rows a query keeps.
 *
 * A column is searched by the text it shows. A column whose cell displays a value
 * other than the raw field (a localized name for an id, a label for a code) declares
 * `searchValue: (row) => string`; every other column is searched by its raw field.
 */

/** The text a column contributes to the search of `row`. */
export function columnSearchText(column, row) {
  const value = typeof column.searchValue === "function" ? column.searchValue(row) : row[column.id];
  return String(value || "").toLowerCase();
}

/** True when any column of `row` contains the lower-cased `query`. */
export function rowMatchesSearch(row, columns, query) {
  return columns.some((column) => columnSearchText(column, row).includes(query));
}
