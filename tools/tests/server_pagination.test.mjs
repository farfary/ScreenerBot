// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Tests for the data table's pagination mixins - cursor and has-more preservation across metadata updates, and the range text.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
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

/** A table whose collaborators are stubbed, so only the pagination calls are observed. */
function loadingTable(loadPage) {
  const table = new TestTable();
  table._pagination = {
    enabled: true,
    loadPage,
    threshold: 160,
    cursorNext: null,
    cursorPrev: null,
    hasMoreNext: true,
    hasMorePrev: true,
    loadingNext: false,
    loadingPrev: false,
    loadingInitial: false,
    pendingRequest: null,
    abortController: null,
  };
  table.elements = { scrollContainer: { scrollTop: 0, scrollHeight: 100, clientHeight: 400 } };
  table.loadNext = (options) => table._loadPage("next", options);
  table.loadPrevious = (options) => table._loadPage("prev", options);
  table._createAbortController = () => new globalThis.AbortController();
  table._setLoadingState = () => {};
  table._updateScrollLoadingIndicator = () => {};
  table._notifyPaginationStateChange = () => {};
  table._applyPageResult = (direction, result) => table._updatePaginationMeta(result);
  table._log = () => {};
  return table;
}

test("no next or previous page loads while the first page is loading", async () => {
  const calls = [];
  let finishFirst;
  const table = loadingTable((args) => {
    calls.push(args.direction);
    if (args.direction === "next") assert.equal(args.cursor, 2);
    if (args.direction === "initial") {
      return new Promise((resolve) => {
        finishFirst = () =>
          resolve({ rows: [], cursorNext: 2, hasMoreNext: true, hasMorePrev: false });
      });
    }
    return Promise.resolve({ rows: [], cursorNext: 3, hasMoreNext: false });
  });

  const first = table._loadPage("initial", { reason: "initial" });

  const duringNext = table.loadNext({ reason: "scroll" });
  const duringPrev = table.loadPrevious({ reason: "scroll" });
  table._autoLoadIfViewportShort();
  table._maybeTriggerPaginationLoad();
  assert.deepEqual(calls, ["initial"]);

  finishFirst();
  await Promise.all([first, duringNext, duringPrev]);
  assert.equal(table._pagination.cursorNext, 2);

  await table.loadNext({ reason: "scroll" });
  assert.deepEqual(calls, ["initial", "next"]);
});

test("an initial page result re-renders unchanged rows only when it asks to", () => {
  const table = new TestTable();
  table._pagination = { enabled: true, abortController: null };
  table.state = { data: [], hasAutoFitted: false };
  table._lastFullRenderTime = Date.now();
  let renders = 0;
  table._applyFilters = () => {
    renders += 1;
  };
  table._setLoadingState = () => {};
  table._log = () => {};

  table._applyPageResult("initial", { rows: [] });
  assert.equal(renders, 0, "unchanged rows rendered moments ago are not drawn again");

  table._applyPageResult("initial", { rows: [], forceRender: true });
  assert.equal(renders, 1, "a result that asks for a render is drawn");
});

/** The `reload` method of `ui/data_table.js`, lifted from its source: the class needs a DOM. */
function reloadOfDataTable() {
  const source = readFileSync(
    new URL("../../src/webserver/templates/scripts/ui/data_table.js", import.meta.url),
    "utf8"
  );
  const start = source.indexOf("\n  reload(options = {}) {") + 1;
  let depth = 0;
  let end = source.indexOf(") {", start) + 2;
  for (; end < source.length; end += 1) {
    if (source[end] === "{") depth += 1;
    else if (source[end] === "}" && --depth === 0) break;
  }
  return new Function(`return ({ ${source.slice(start, end + 1)} }).reload;`)();
}

test("a first load starts before the render it triggers can start a next load", async () => {
  const calls = [];
  const table = loadingTable((args) => {
    calls.push(args.direction);
    return Promise.resolve({ rows: [], hasMoreNext: false });
  });
  Object.assign(table._pagination, { initialCursor: null });
  table._loadToken = 0;
  table.state = { filteredData: [], sortColumn: null };
  table._cancelPaginationRequest = () => {};
  table._isSupersededByInFlightLoad = () => false;
  table._serverPaginationMode = "scroll";
  table._hasHybridPaginationModes = () => false;
  // The empty render finds a short viewport and asks for more rows at once.
  table._renderTable = () => table._autoLoadIfViewportShort();
  table.reload = reloadOfDataTable();

  await table.reload({ reason: "initial", silent: false });

  assert.deepEqual(calls, ["initial"]);
});
