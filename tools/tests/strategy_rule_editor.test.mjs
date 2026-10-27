// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the rule editor's rows, their summaries, the saved tree and the Save check all
 * read one value per parameter.
 *
 * The summary once listed only the parameters stored on the condition, capped at
 * three, while the editor fell back to schema defaults and a select without an unset
 * option showed its first choice. A Price Change card then read "Timeframe: —" with
 * no lookback while its editor showed "1 Minute" and 5 minutes.
 *
 * - Every schema parameter appears in the summary (a lookback as one period entry),
 *   with the option the editor selects.
 * - An unset optional parameter is its own editor choice and summary value, naming
 *   the strategy's value it falls back to; the saved tree keeps it unset.
 * - A lookback stored as `time_value` + `time_unit` is one duration field: the amount
 *   input with its unit select as the field's `.input-unit`, under one label. Its
 *   summary is the amount with the unit, days included, under the amount's label.
 * - An hour of the UTC day reads "UTC" beside the input and in the summary, with the
 *   local clock time as a hint; a pool-to-size multiple shows its price impact.
 * - A switched-off row is rendered, dimmed, and saved with `disabled`; an inverted row
 *   is saved under a NOT; a nested group has its own match select.
 * - An empty number field is a `required` error at the field and blocks Save.
 *
 * Move up and Move down are disabled on the first and last row, and Remove removes a
 * row only after it is confirmed.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";

import { defaultTimeframe, schemas } from "../lib/strategy_rules.mjs";
import { createRuleEditor } from "../../src/webserver/templates/scripts/pages/strategies/rule_editor.js";
import * as tree from "../../src/webserver/templates/scripts/pages/strategies/rule_tree.js";
import { formatTimestamp } from "../../src/webserver/templates/scripts/core/format.js";
import { paramHint } from "../../src/webserver/templates/scripts/pages/strategies/condition_text.js";

const escapeHtml = (text) =>
  String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
/** Text without the Unicode isolates Fluent places around arguments. */
const plain = (text) => text.replace(/[⁨⁩]/g, "");
const unescape = (text) =>
  text
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, "<")
    .replace(/&amp;/g, "&");

function editorFor({ type = "ENTRY", confirm = async () => ({ confirmed: false }) } = {}) {
  const state = { currentStrategy: { type, timeframe: defaultTimeframe } };
  const editor = createRuleEditor({
    state,
    conditionSchemas: schemas,
    $: () => null,
    Utils: { escapeHtml },
    renderStateView: () => "",
    announce: () => {},
    confirm,
    enhanceAllSelects: () => {},
    addTrackedListener: () => {},
    onChange: () => {},
    openCatalog: () => {},
  });
  editor.load(null);
  return editor;
}

/** Add a condition to the root and return its model node. */
function add(editor, type) {
  editor.addConditionTo(editor.model().id, type);
  return editor.model().children.at(-1);
}

/** Label of the option each select of a rendered parameter editor shows. */
function selectedOptions(html) {
  const shown = {};
  for (const select of html.matchAll(/<select[^>]*data-key="([^"]+)"[^>]*>([\s\S]*?)<\/select>/g)) {
    const options = [
      ...select[2].matchAll(/<option value="[^"]*" ?(selected)?>([^<]*)<\/option>/g),
    ];
    const chosen = options.find((option) => option[1]) ?? options[0];
    shown[select[1]] = plain(unescape(chosen[2]));
  }
  return shown;
}

/** The markup of each condition row of a rendered tree. */
const rowsOf = (html) => html.split(/(?=<div class="rule-row)/).slice(1);

test("the summary names every parameter with the option the editor shows", () => {
  assert.equal(defaultTimeframe, "5m");
  for (const [type, schema] of Object.entries(schemas)) {
    const editor = editorFor();
    const node = add(editor, type);
    const summary = plain(editor.buildConditionSummary(node));
    const shown = selectedOptions(editor.renderParamEditor(node));
    const params = schema.parameters || {};
    const period = "time_value" in params && "time_unit" in params;

    for (const [key, spec] of Object.entries(params)) {
      if (period && (key === "time_value" || key === "time_unit")) continue;
      const label = plain(I18n.t(spec.key));
      assert.ok(summary.includes(`${label}:`), `${type}.${key} missing from "${summary}"`);
      if (key in shown) {
        assert.ok(
          summary.includes(`${label}: ${shown[key]}`),
          `${type}.${key}: editor shows "${shown[key]}", summary "${summary}"`
        );
      }
    }
    if (period) {
      // One entry under the amount's own label, in the unit picked inside the field.
      const label = plain(I18n.t(params.time_value.key));
      const expected = `${label}: ${params.time_value.default} min`;
      assert.ok(summary.includes(expected), `${type}: "${expected}" missing from "${summary}"`);
    }
  }
});

test("an unset timeframe names the strategy timeframe and stays unset when saved", () => {
  const editor = editorFor();
  const node = add(editor, "PriceChangePercent");
  const fallback = "Strategy setting (5 Minutes)";

  assert.equal(selectedOptions(editor.renderParamEditor(node)).timeframe, fallback);
  assert.ok(plain(editor.buildConditionSummary(node)).includes(`Timeframe: ${fallback}`));

  const [saved] = editor.serialize().conditions;
  assert.equal(saved.condition.parameters.timeframe.value, null);
  assert.equal(saved.condition.parameters.time_value.value, 5);
  assert.equal(saved.condition.parameters.time_unit.value, "MINUTES");
  assert.equal(saved.condition.parameters.percentage.value, 10);
});

test("a lookback is one duration field with its unit picked inside it", () => {
  const editor = editorFor();
  const node = add(editor, "PriceChangePercent");
  tree.setParam(editor.model(), node.id, "time_value", 2);
  tree.setParam(editor.model(), node.id, "time_unit", "HOURS");
  const html = editor.renderParamEditor(node);
  const lookback = html
    .split('<div class="param-field"')
    .slice(1)
    .filter((field) => /data-key="time_(value|unit)"/.test(field));

  assert.equal(lookback.length, 1, "amount and unit share one field");
  assert.match(lookback[0], /<label for="param-\d+-time_value">Lookback<\/label>/);
  assert.match(
    lookback[0],
    /<input [^>]*data-key="time_value" type="number" value="2"[^>]*>\s*<span class="input-unit"><select [^>]*data-key="time_unit"[^>]*aria-label="Time Unit"[^>]*data-custom-select/,
    "the unit select is the amount's own unit"
  );
  assert.deepEqual(selectedOptions(lookback[0]), { time_unit: "h" });

  const [saved] = editor.serialize().conditions;
  assert.equal(saved.condition.parameters.time_value.value, 2);
  assert.equal(saved.condition.parameters.time_unit.value, "HOURS");
});

test("a token age in days reads as days in the summary and the duration field", () => {
  const editor = editorFor();
  const node = add(editor, "TokenAge");
  tree.setParam(editor.model(), node.id, "time_value", 3);
  tree.setParam(editor.model(), node.id, "time_unit", "DAYS");

  assert.ok(plain(editor.buildConditionSummary(node)).includes("Age: 3 d"));
  const html = editor.renderParamEditor(node);
  assert.match(html, /<input [^>]*data-key="time_value" type="number" value="3"[^>]*>\s*<span class="input-unit"><select [^>]*data-key="time_unit"/);
  assert.deepEqual(selectedOptions(html), { comparison: "Older Than (≥)", time_unit: "d" });
});

test("a UTC hour shows UTC beside the input and the local time as a hint", () => {
  const editor = editorFor();
  const node = add(editor, "TimeOfDay");
  const html = editor.renderParamEditor(node);
  const start = html.split('<div class="param-field"').find((part) => /data-key="start_hour"/.test(part));

  assert.match(start, /<input [^>]*data-key="start_hour"[^>]*>\s*<span class="input-unit">UTC<\/span>/);
  const expected = paramHint(schemas.TimeOfDay.parameters.start_hour, 13);
  assert.match(plain(expected), /^Your local time: /);
  assert.ok(plain(unescape(start)).includes(plain(expected)), "the local-time hint is shown");
  assert.ok(plain(editor.buildConditionSummary(node)).includes("Start Hour (UTC): 13:00 UTC"));
});

test("the local-time hint is the same instant of the UTC day on the viewer's clock", () => {
  const spec = schemas.TimeOfDay.parameters.start_hour;
  const now = new Date(Date.UTC(2026, 0, 15, 9, 30));
  const time = formatTimestamp(Date.UTC(2026, 0, 15, 22), {
    includeDate: false,
    includeSeconds: false,
  });
  assert.equal(plain(paramHint(spec, 22, now)), `Your local time: ${plain(time)}`);
  assert.equal(paramHint(spec, null, now), "");
  assert.equal(paramHint(schemas.TimeOfDay.parameters.end_hour, ""), "");
});

test("a pool-to-size multiple shows the price impact a trade of that size has", () => {
  const spec = schemas.LiquidityToSize.parameters.multiple;
  assert.equal(spec.hint, "price_impact");
  assert.equal(
    plain(paramHint(spec, 99)),
    "A trade of this size moves the price by about 1%"
  );
  assert.equal(plain(paramHint(spec, 3)), "A trade of this size moves the price by about 25%");
  assert.equal(paramHint(spec, 0), "");

  const editor = editorFor();
  const node = add(editor, "LiquidityToSize");
  tree.setParam(editor.model(), node.id, "multiple", 19);
  assert.ok(
    plain(unescape(editor.renderParamEditor(node))).includes("moves the price by about 5%"),
    "the impact hint follows the value"
  );
});

test("a parameter without a hint renders no hint line", () => {
  const editor = editorFor();
  const node = add(editor, "PriceChangePercent");
  assert.doesNotMatch(editor.renderParamEditor(node), /data-hint/);
});

test("move actions are disabled where the row cannot move", () => {
  const editor = editorFor();
  for (const type of ["PriceChangePercent", "CandleSize", "VolumeSpike"]) add(editor, type);
  const disabled = (html, action) =>
    new RegExp(`<button[^>]*data-action="${action}"[^>]*\\sdisabled[^>]*>`).test(html);

  assert.deepEqual(
    rowsOf(editor.renderRules()).map((html) => [disabled(html, "move-up"), disabled(html, "move-down")]),
    [
      [true, false],
      [false, false],
      [false, true],
    ]
  );

  const single = editorFor();
  add(single, "Rsi");
  const [only] = rowsOf(single.renderRules());
  assert.ok(disabled(only, "move-up") && disabled(only, "move-down"));
});

test("a row is removed only once the removal is confirmed", async () => {
  const asked = [];
  let answer = false;
  const editor = editorFor({
    confirm: async (config) => {
      asked.push(config);
      return { confirmed: answer };
    },
  });
  const node = add(editor, "PriceChangePercent");

  await editor.removeNode(node.id);
  assert.equal(editor.model().children.length, 1, "a declined removal keeps the condition");
  assert.equal(asked[0].variant, "danger");
  assert.match(plain(asked[0].message), /Price Change/);

  answer = true;
  await editor.removeNode(node.id);
  assert.equal(editor.model().children.length, 0);
});

test("a switched-off row stays in place, dimmed, and is saved with disabled", () => {
  const editor = editorFor();
  const off = add(editor, "Rsi");
  add(editor, "VolumeSpike");
  tree.setDisabled(editor.model(), off.id, true);

  const [first, second] = rowsOf(editor.renderRules());
  assert.match(first, /class="rule-row is-off/);
  assert.match(first, /<span class="rule-chip">Off<\/span>/);
  assert.doesNotMatch(first, /data-action="switch"[^>]*checked/);
  assert.doesNotMatch(second, /is-off/);

  const [saved] = editor.serialize().conditions;
  assert.equal(saved.condition.type, "Rsi");
  assert.equal(saved.disabled, true);
});

test("an inverted row is saved under a NOT", () => {
  const editor = editorFor();
  const node = add(editor, "Rsi");
  tree.setInverted(editor.model(), node.id, true);

  const [row] = rowsOf(editor.renderRules());
  assert.match(row, /<span class="rule-chip">Inverted<\/span>/);
  assert.match(row, /data-action="invert"[^>]*aria-pressed="true"/);

  const [saved] = editor.serialize().conditions;
  assert.equal(saved.operator, "NOT");
  assert.equal(saved.conditions[0].condition.type, "Rsi");
});

test("a nested group has its own match select", () => {
  const editor = editorFor();
  add(editor, "Rsi");
  const group = tree.addGroup(editor.model(), editor.model().id);
  tree.addCondition(editor.model(), group, "CandleSize", schemas);

  const html = editor.renderRules();
  assert.equal(html.match(/<select class="rule-match-select"/g).length, 2);
  assert.match(html, /Match [\s\S]*of these conditions/);
  const nested = html.slice(html.indexOf('class="rule-group is-nested'));
  assert.match(plain(nested), /Match <select class="rule-match-select"[\s\S]*<\/select> of these/);
  assert.match(nested, /data-action="add-condition"/);
  assert.match(nested, /data-action="add-group"/);
});

test("an empty number field is a required error at the field and blocks Save", () => {
  const editor = editorFor();
  const node = add(editor, "Rsi");
  assert.equal(editor.checkBeforeSave(), true);

  tree.setParam(editor.model(), node.id, "period", null);
  editor.setOpen(node.id, true);
  const [row] = rowsOf(editor.renderRules());
  const field = row.split('<div class="param-field"').find((part) => /data-key="period"/.test(part));
  assert.match(field, /<input [^>]*data-key="period"[^>]*aria-invalid="true"/);
  assert.match(field, /class="param-field-error"[^>]*>Enter a number</);
  assert.match(row, /class="rule-row[^"]*has-problem/);
  assert.equal(editor.checkBeforeSave(), false);
  assert.match(plain(editor.describeProblem(editor.problems()[0])), /^RSI \(Relative Strength Index\), RSI Period: Enter a number$/);

  tree.setParam(editor.model(), node.id, "period", 14);
  assert.equal(editor.checkBeforeSave(), true);
  assert.doesNotMatch(editor.renderRules(), /aria-invalid/);
});

test("a condition the strategy type cannot use is shown with its problem and blocks Save", () => {
  const editor = editorFor({ type: "ENTRY" });
  add(editor, "PositionHoldingTime");
  const [row] = rowsOf(editor.renderRules());
  assert.match(row, /class="rule-node-problem"[^>]*>This condition cannot be used in this type of strategy</);
  assert.equal(editor.checkBeforeSave(), false);

  const exit = editorFor({ type: "EXIT" });
  add(exit, "PositionHoldingTime");
  assert.equal(exit.checkBeforeSave(), true);
});
