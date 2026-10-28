// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Structural tests for the shared DataTable toolbar renderer.
 *
 * Dropdown grouping belongs to the renderer so every table receives the same
 * outer-shell and divider behavior without page-owned first/middle/last classes.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import "./fixtures/i18n_en.mjs";

import { STYLES_ROOT, rulesIn } from "../lib/dashboard_ui.mjs";

import {
  TableToolbarView,
  renderSummaryChip,
} from "../../src/webserver/templates/scripts/ui/table_toolbar.js";

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

  assert.match(html, /--table-toolbar-field-min-width:170px/);
  assert.doesNotMatch(html, /style="min-width:170px;"/);
});

test("sizes every toolbar dropdown from its rendered options, not a character estimate", () => {
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

  assert.equal(html.match(/data-custom-select data-cs-fit-options/g)?.length, 2);
  assert.doesNotMatch(html, /--table-toolbar-select-label-width/);
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

test("only a button with an icon sheds its label in a narrow bar", () => {
  const html = new TableToolbarView({
    settings: false,
    buttons: [
      { id: "with-icon", label: "New Strategy", icon: "icon-plus" },
      { id: "text-only", label: "Start Auto Trader" },
    ],
  }).render();
  const classes = (id) =>
    new RegExp(`<button[^>]*class="([^"]*)"[^>]*data-btn-id="${id}"`).exec(html)?.[1] ??
    new RegExp(`<button[^>]*data-btn-id="${id}"[^>]*class="([^"]*)"`).exec(html)?.[1];

  assert.match(classes("with-icon") ?? "", /table-toolbar-btn--collapsible/);
  assert.ok(classes("text-only"), "the text-only button renders");
  assert.doesNotMatch(classes("text-only"), /table-toolbar-btn--collapsible/);
});

test("a neutral toolbar button shares the surface of the toolbar's select triggers", () => {
  const background = (path, subject) =>
    rulesIn(readFileSync(`${STYLES_ROOT}/${path}`, "utf8"))
      .filter(({ selector }) => selector === subject)
      .map(({ body }) => /(?:^|;)\s*background(?:-color)?\s*:\s*([^;]+)/.exec(body)?.[1]?.trim())
      .find(Boolean);

  const trigger = background("ui/custom_select.css", ".cs-trigger");
  assert.ok(trigger, "no .cs-trigger background in custom_select.css");
  assert.equal(background("ui/table_toolbar.css", ".table-toolbar-btn"), trigger);
});

test("an absent summary figure renders as a neutral dash", () => {
  const html = new TableToolbarView({
    settings: false,
    summary: [
      { id: "failed", label: "Failed", value: "—", variant: "success" },
      { id: "missing", label: "Missing", value: null, variant: "warning" },
      { id: "count", label: "Count", value: "3", variant: "warning" },
    ],
  }).render();

  const chip = (id) =>
    html.match(new RegExp(`<div class="table-toolbar-chip" data-summary-id="${id}"[^>]*>`))[0];
  assert.doesNotMatch(chip("failed"), /data-variant/);
  assert.doesNotMatch(chip("missing"), /data-variant/);
  assert.match(chip("count"), /data-variant="warning"/);
  assert.match(html, /data-summary-id="missing"[\s\S]*?table-toolbar-chip__value">—</);
});

test("an in-place summary update that loses its figure drops the colour", () => {
  const attributes = { "data-variant": "success" };
  const value = { textContent: "4" };
  const chip = {
    setAttribute: (name, next) => (attributes[name] = next),
    removeAttribute: (name) => delete attributes[name],
    querySelector: (selector) => (selector === ".table-toolbar-chip__value" ? value : null),
  };
  const root = { querySelector: () => chip };

  TableToolbarView.updateSummary(root, [{ id: "failed", value: null, variant: "success" }]);
  assert.equal(value.textContent, "—");
  assert.equal(attributes["data-variant"], undefined);

  TableToolbarView.updateSummary(root, [{ id: "failed", value: "2", variant: "warning" }]);
  assert.equal(value.textContent, "2");
  assert.equal(attributes["data-variant"], "warning");
});

test("no chip variant uses a raw colour", () => {
  const rules = rulesIn(readFileSync(`${STYLES_ROOT}/ui/table_toolbar.css`, "utf8")).filter(
    ({ selector }) => /^\.table-toolbar-chip\[data-variant="[a-z]+"\]/.test(selector)
  );
  assert.ok(rules.length >= 4, "the chip variants are styled in table_toolbar.css");
  for (const { selector, body } of rules) {
    assert.doesNotMatch(body, /#[0-9a-f]{3,8}\b|rgba?\(/i, `${selector} reads a token`);
  }
});

test("the summary chip renderer is the toolbar's own", () => {
  const chip = renderSummaryChip({ id: "state", label: "Status", value: "Paused", variant: "warning" });
  assert.equal(
    chip,
    '<div class="table-toolbar-chip" data-summary-id="state" data-variant="warning"><span class="table-toolbar-chip__label">Status</span><span class="table-toolbar-chip__value">Paused</span></div>'
  );
  const html = new TableToolbarView({
    settings: false,
    summary: [{ id: "state", label: "Status", value: "Paused", variant: "warning" }],
  }).render();
  assert.ok(html.includes(chip), "the toolbar paints the same chip");
});
