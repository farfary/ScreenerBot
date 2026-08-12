/**
 * Structural tests for the shared DataTable toolbar renderer.
 *
 * Dropdown grouping belongs to the renderer so every table receives the same
 * outer-shell and divider behavior without page-owned first/middle/last classes.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { TableToolbarView } from "../../src/webserver/templates/scripts/ui/table_toolbar.js";

const select = (id) => ({
  id,
  type: "select",
  options: [{ value: "all", label: "All" }],
});

test("groups each consecutive run of toolbar dropdowns", () => {
  const html = new TableToolbarView({
    settings: false,
    controls: [select("type"), select("status"), { id: "reset", type: "button" }],
  }).render();

  assert.match(html, /class="table-toolbar-select-group"/);
  assert.match(
    html,
    /table-toolbar-select-group[\s\S]*data-filter-id="type"[\s\S]*data-filter-id="status"[\s\S]*<\/div>/
  );
});

test("leaves a standalone toolbar dropdown unchanged", () => {
  const html = new TableToolbarView({ settings: false, controls: [select("status")] }).render();

  assert.doesNotMatch(html, /table-toolbar-select-group/);
  assert.match(html, /data-filter-id="status"/);
});

test("does not group dropdowns separated by another control", () => {
  const html = new TableToolbarView({
    settings: false,
    controls: [select("type"), { id: "reset", type: "button" }, select("status")],
  }).render();

  assert.doesNotMatch(html, /table-toolbar-select-group/);
});

test("keeps hidden dropdowns inside the run so visibility can change without rerendering", () => {
  const hiddenSelect = { ...select("status"), hidden: true };
  const html = new TableToolbarView({
    settings: false,
    controls: [select("type"), hiddenSelect, select("wallet")],
  }).render();

  assert.match(html, /class="table-toolbar-select-group"/);
  assert.match(html, /data-filter-id="status"[^>]*hidden/);
  assert.match(html, /data-filter-id="wallet"/);
});

test("stores a requested standalone width without hard-wiring grouped field geometry", () => {
  const html = new TableToolbarView({
    settings: false,
    controls: [{ ...select("wallet"), minWidth: "170px" }],
  }).render();

  assert.match(html, /--table-toolbar-select-label-width:3.5rem/);
  assert.match(html, /--table-toolbar-field-min-width:170px/);
  assert.doesNotMatch(html, /style="min-width:170px;"/);
});

test("sizes grouped dropdowns from their longest option instead of one equal track", () => {
  const html = new TableToolbarView({
    settings: false,
    controls: [
      { ...select("type"), options: [{ value: "all", label: "All Types" }] },
      {
        ...select("direction"),
        options: [{ value: "all", label: "All Directions" }],
      },
    ],
  }).render();

  assert.match(html, /--table-toolbar-select-label-width:4.5rem[^>]*data-filter-id="type"/);
  assert.match(
    html,
    /--table-toolbar-select-label-width:7rem[^>]*data-filter-id="direction"/
  );
});

test("keeps typed search controls addressable in the flat toolbar index", () => {
  const view = new TableToolbarView({
    settings: false,
    controls: [{ id: "search", type: "search", placeholder: "Signature…" }],
  });

  assert.equal(view.getItem("search")?.placeholder, "Signature…");
});

test("renders an explicit query row without changing control semantics", () => {
  const html = new TableToolbarView({
    layout: "query-row",
    settings: false,
    controls: [
      { id: "search", type: "search", placeholder: "Search signatures…" },
      select("status"),
    ],
  }).render();

  assert.match(html, /data-layout="query-row"/);
  assert.match(html, /placeholder="Search signatures…"/);
  assert.doesNotMatch(html, /data-collapsible/);
});
