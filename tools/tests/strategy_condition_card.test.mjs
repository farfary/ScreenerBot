// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a strategy condition card's collapsed summary, its expanded editor and the
 * saved rule tree all read one resolved value per parameter.
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
 * - The saved tree carries the value the editor shows, defaults included.
 * - A lookback stored as `time_value` + `time_unit` is one duration field: the amount
 *   input with its unit select as the field's `.input-unit`, under one label, saved
 *   as the same two parameters.
 *
 * The card actions act only where they can: Move up and Move down are disabled on the
 * first and last card, and Delete removes a condition only after it is confirmed.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { createConditionEditor } from "../../src/webserver/templates/scripts/pages/strategies/condition_editor.js";

const { schemas, default_timeframe: defaultTimeframe } = JSON.parse(
  readFileSync(
    new URL("./fixtures/dashboard/trader/strategies_condition_schemas.json", import.meta.url),
    "utf8"
  )
);

const escapeHtml = (text) =>
  String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/"/g, "&quot;");
/** Text without the Unicode isolates Fluent places around arguments. */
const plain = (text) => text.replace(/[\u2068\u2069]/g, "");
const unescape = (text) =>
  text
    .replace(/&quot;/g, '"')
    .replace(/&lt;/g, "<")
    .replace(/&amp;/g, "&");

function editorFor(conditions, confirm = async () => ({ confirmed: false })) {
  const state = { currentStrategy: { timeframe: defaultTimeframe, rules: null } };
  const editor = createConditionEditor({
    state,
    conditions,
    conditionSchemas: schemas,
    $: () => null,
    $$: () => [],
    Utils: { escapeHtml },
    announce: () => {},
    confirm,
    enhanceAllSelects: () => {},
    addTrackedListener: () => {},
    clearScope: () => {},
    CleanupScope: {},
  });
  return { editor, state };
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

test("the summary names every parameter with the option the editor shows", () => {
  assert.equal(defaultTimeframe, "5m");
  for (const [type, schema] of Object.entries(schemas)) {
    const condition = { type, enabled: true, params: {} };
    const { editor } = editorFor([condition]);
    const summary = plain(editor.buildConditionSummary(condition));
    const shown = selectedOptions(editor.renderParamEditor(condition, schema, 0));
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
    if (period) assert.match(summary, /Lookback: 5 min/, `${type} lookback missing`);
  }
});

test("an unset timeframe names the strategy timeframe and stays unset when saved", () => {
  const condition = { type: "PriceChangePercent", enabled: true, params: { timeframe: null } };
  const { editor, state } = editorFor([condition]);
  const fallback = "Strategy setting (5 Minutes)";

  const shown = selectedOptions(editor.renderParamEditor(condition, schemas.PriceChangePercent, 0));
  assert.equal(shown.timeframe, fallback);
  assert.ok(plain(editor.buildConditionSummary(condition)).includes(`Timeframe: ${fallback}`));

  editor.updateRuleTreeFromEditor();
  const saved = state.currentStrategy.rules.condition.parameters;
  assert.equal(saved.timeframe.value, null);
  assert.equal(saved.time_value.value, 5);
  assert.equal(saved.time_unit.value, "MINUTES");
  assert.equal(saved.percentage.value, 10);
});

test("a lookback is one duration field with its unit picked inside it", () => {
  const condition = {
    type: "PriceChangePercent",
    enabled: true,
    params: { time_value: 2, time_unit: "HOURS" },
  };
  const { editor, state } = editorFor([condition]);
  const html = editor.renderParamEditor(condition, schemas.PriceChangePercent, 0);
  const lookback = html
    .split('<div class="param-field">')
    .slice(1)
    .filter((field) => /data-key="time_(value|unit)"/.test(field));

  assert.equal(lookback.length, 1, "amount and unit share one field");
  assert.match(lookback[0], /<label>Lookback<\/label>/);
  assert.match(
    lookback[0],
    /<input [^>]*data-key="time_value" type="number" value="2"[^>]*>\s*<span class="input-unit"><select [^>]*data-key="time_unit"[^>]*aria-label="Time Unit"[^>]*data-custom-select/,
    "the unit select is the amount's own unit"
  );
  assert.deepEqual(selectedOptions(lookback[0]), { time_unit: "h" });

  editor.updateRuleTreeFromEditor();
  const saved = state.currentStrategy.rules.condition.parameters;
  assert.equal(saved.time_value.value, 2);
  assert.equal(saved.time_unit.value, "HOURS");
});

test("move actions are disabled where the card cannot move", () => {
  const conditions = ["PriceChangePercent", "CandleSize", "VolumeSpike"].map((type) => ({
    type,
    enabled: true,
    params: {},
  }));
  const { editor } = editorFor(conditions);
  const disabled = (html, action) =>
    new RegExp(`<button[^>]*data-action="${action}"[^>]*\\sdisabled[^>]*>`).test(html);

  const cards = conditions.map((condition, index) => editor.renderConditionCard(condition, index));
  assert.deepEqual(
    cards.map((html) => [disabled(html, "move-up"), disabled(html, "move-down")]),
    [
      [true, false],
      [false, false],
      [false, true],
    ]
  );

  const single = [conditions[0]];
  const html = editorFor(single).editor.renderConditionCard(single[0], 0);
  assert.ok(disabled(html, "move-up") && disabled(html, "move-down"));
});

test("a condition is removed only once the removal is confirmed", async () => {
  const asked = [];
  let answer = false;
  const conditions = [{ type: "PriceChangePercent", enabled: true, params: {} }];
  const { editor } = editorFor(conditions, async (config) => {
    asked.push(config);
    return { confirmed: answer };
  });

  await editor.deleteCondition(0);
  assert.equal(conditions.length, 1, "a declined removal keeps the condition");
  assert.equal(asked[0].variant, "danger");
  assert.match(plain(asked[0].message), /Price Change/);

  answer = true;
  await editor.deleteCondition(0);
  assert.equal(conditions.length, 0);
});
