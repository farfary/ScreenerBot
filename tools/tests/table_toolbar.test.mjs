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
