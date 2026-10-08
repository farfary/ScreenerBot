// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Tests for the data table's pagination mixins - cursor and has-more preservation across metadata updates, and the range text.

import assert from "node:assert/strict";
import test from "node:test";
import "./fixtures/i18n_en.mjs";

globalThis.window = { addEventListener() {} };
// core/utils.js registers document listeners when it is imported.
globalThis.document = { addEventListener() {}, readyState: "complete" };

const { applyServerPaginationMixin } =
  await import("../../src/webserver/templates/scripts/ui/data_table/server_pagination.js");

class TestTable {}

applyServerPaginationMixin(TestTable);

function tableWithCursors() {
  const table = new TestTable();
  table._pagination = {
    enabled: true,
    cursorNext: { signature: "older" },
    cursorPrev: { signature: "newer" },
    hasMoreNext: true,
    hasMorePrev: true,
    meta: {},
  };
  return table;
}

test("undefined pagination metadata preserves cursors owned by another direction", () => {
  const table = tableWithCursors();

  table._updatePaginationMeta({
    cursorNext: undefined,
    cursorPrev: undefined,
    hasMoreNext: undefined,
    hasMorePrev: undefined,
  });

  assert.deepEqual(table._pagination.cursorNext, { signature: "older" });
  assert.deepEqual(table._pagination.cursorPrev, { signature: "newer" });
  assert.equal(table._pagination.hasMoreNext, true);
  assert.equal(table._pagination.hasMorePrev, true);
});

test("explicit null pagination cursor still clears that direction", () => {
  const table = tableWithCursors();

  table._updatePaginationMeta({ cursorNext: null });

  assert.equal(table._pagination.cursorNext, null);
  assert.equal(table._pagination.hasMoreNext, false);
  assert.deepEqual(table._pagination.cursorPrev, { signature: "newer" });
});

const { applyClientPaginationMixin } =
  await import("../../src/webserver/templates/scripts/ui/data_table/client_pagination.js");

class ClientTable {}

applyClientPaginationMixin(ClientTable);

test("both pagers group the range and total like every other count", (t) => {
  // escapeHtml goes through a detached element; text escaping is all the pager needs.
  const createElement = document.createElement;
  document.createElement = () => {
    const node = { textContent: "" };
    Object.defineProperty(node, "innerHTML", {
      get: () =>
        node.textContent.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;"),
    });
    return node;
  };
  t.after(() => {
    document.createElement = createElement;
  });

  const server = new TestTable();
  server._pagination = { enabled: true, pageSizes: [50] };
  server._serverPaginationMode = "pages";
  server._generateServerPaginationButtons = () => "";
  server.state = {
    serverPaginationState: { currentPage: 21, pageSize: 50, totalPages: 64, totalItems: 3193 },
  };

  const client = new ClientTable();
  client.options = { clientPagination: { enabled: true, pageSizes: [50] } };
  client._clientPaginationActive = true;
  client._generateClientPaginationButtons = () => "";
  client.state = {
    clientPaginationState: { currentPage: 21, pageSize: 50, totalPages: 64 },
    filteredData: { length: 3193 },
  };

  for (const html of [server._renderServerPaginationBar(), client._renderClientPaginationBar()]) {
    const range = html.match(/pagination-range">([\s\S]*?)<\/span>/)[1].replace(/<[^>]+>/g, "");
    assert.equal(range.replace(/[⁨⁩]/g, "").trim(), "Showing 1,001–1,050 of 3,193");
  }
});
