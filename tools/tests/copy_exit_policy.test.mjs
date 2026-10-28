// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a Copy Trading rule card and the exit warnings name one stop-loss value.
 *
 * The backend merges a task's exit overrides per field, so a stored threshold under a
 * switch that follows the Trader still applies. The card note and the warnings both
 * read the resolved policy; neither may fall back to the Trader default alone.
 * An inherited take-profit ladder reads as its steps on the Rules tab, never as the target
 * of its first step alone, and the editor says that a target override is one full exit.
 * The break-even stop and the flat position exit, which a task cannot override, read as
 * the Trader's values on the Rules tab and in the editor's review, and never enter a
 * task's overrides.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import "./fixtures/document_shim.mjs";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  INHERITED_RULES,
  RULES,
  blankOverrides,
  effectivePolicy,
  exitWarnings,
  inheritNote,
} from "../../src/webserver/templates/scripts/pages/copy/policy.js";
import { rulesHtml } from "../../src/webserver/templates/scripts/pages/copy/rules.js";

const plain = (text) => text.replace(/[⁨⁩]/g, "");
const workspace = JSON.parse(
  readFileSync(new URL("./fixtures/dashboard/copy/copy_workspace.json", import.meta.url), "utf8")
);
const STOP = RULES.find((rule) => rule.group === "stop_loss");

/** Every percentage a text names, unsigned, as numbers. */
const percents = (text) => [...plain(text).matchAll(/(\d+(?:\.\d+)?)%/g)].map((m) => Number(m[1]));

test("a stored threshold under an inherited switch shows in the card and the warning alike", () => {
  const { trader_defaults: traderDefaults, exit_policy_overrides: overrides } = workspace;
  assert.equal(
    workspace.effective_policy.stop_loss.threshold_pct,
    overrides.stop_loss.threshold_pct,
    "the backend resolves the stored threshold"
  );
  assert.equal(overrides.stop_loss.enabled, null, "the switch follows the Trader");
  assert.notEqual(overrides.stop_loss.threshold_pct, traderDefaults.stop_loss.threshold_pct);

  const note = plain(inheritNote(STOP, traderDefaults, overrides));
  const resolved = Number(overrides.stop_loss.threshold_pct);
  assert.ok(percents(note).includes(resolved), `the card names ${resolved}%: ${note}`);
  assert.ok(
    !percents(note).includes(Number(traderDefaults.stop_loss.threshold_pct)),
    `the card does not name the Trader default: ${note}`
  );
  assert.match(note, /this task's values/);

  const warnings = exitWarnings(effectivePolicy(traderDefaults, overrides), workspace.exit_mode, {})
    .map(plain)
    .filter((text) => /stop/i.test(text));
  for (const text of warnings) {
    assert.ok(
      !percents(text).includes(Number(traderDefaults.stop_loss.threshold_pct)),
      `the warning does not name the Trader default: ${text}`
    );
  }
});

test("a rule with no stored values follows the Trader's own summary", () => {
  const traderDefaults = { stop_loss: { enabled: true, threshold_pct: 40, min_hold_seconds: 120 } };
  const note = plain(inheritNote(STOP, traderDefaults, { stop_loss: { enabled: null } }));
  assert.match(note, /^Follows the Trader: /);
  assert.ok(percents(note).includes(40), note);
});

const esc = (text) => String(text).replace(/[&<>"']/g, (char) => `&#${char.charCodeAt(0)};`);
const LADDER = [
  { profit_pct: 20, sell_pct: 30 },
  { profit_pct: 40, sell_pct: 30 },
  { profit_pct: 80, sell_pct: 40 },
];
const TAKE_PROFIT = RULES.find((rule) => rule.group === "roi");

/** The text of the take-profit target row of a rendered Rules table. */
function targetRow(html) {
  const label = TAKE_PROFIT.fields.find((field) => field.key === "target_profit_pct").label;
  const row = plain(html).match(new RegExp(`<tr[^>]*><td>${label}</td><td>([^<]*)</td>`));
  assert.ok(row, "the Rules table has a take-profit target row");
  return row[1];
}

test("an inherited take-profit ladder reads as its steps on the Rules tab", () => {
  const roi = { enabled: true, target_profit_pct: 20, levels: LADDER };
  const traderDefaults = { ...workspace.trader_defaults, roi };
  const overrides = {
    ...workspace.exit_policy_overrides,
    roi: { enabled: null, target_profit_pct: null },
  };
  const task = { ...workspace, exit_policy_overrides: overrides };
  const context = { task, traderDefaults, managesExits: true };

  const steps = "At +20%, sell 30%, At +40%, sell 30%, At +80%, sell 40%";
  const backend = { ...workspace.effective_policy, roi };
  assert.equal(targetRow(rulesHtml({ ...context, effective: backend }, esc)), steps);
  assert.equal(
    targetRow(
      rulesHtml({ ...context, effective: effectivePolicy(traderDefaults, overrides) }, esc)
    ),
    steps,
    "the editor's review reads the same steps"
  );

  // A target override is one full exit, named by its target alone.
  const own = { ...overrides, roi: { enabled: true, target_profit_pct: 50 } };
  assert.equal(
    targetRow(
      rulesHtml(
        {
          ...context,
          task: { ...task, exit_policy_overrides: own },
          effective: effectivePolicy(traderDefaults, own),
        },
        esc
      )
    ),
    "+50%"
  );
});

test("the take-profit target field says an override replaces the ladder with one full exit", () => {
  const target = TAKE_PROFIT.fields.find((field) => field.key === "target_profit_pct");
  assert.equal(
    plain(target.hint),
    "A target here replaces the Trader's take-profit steps with one step that sells the whole position"
  );
});

test("the rules a task cannot override read as the Trader's values", () => {
  const traderDefaults = {
    ...workspace.trader_defaults,
    break_even: { enabled: true, trigger_pct: 15, offset_pct: 1 },
    time_stagnation: { enabled: true, duration_seconds: 14400, band_pct: 5 },
  };
  const overrides = workspace.exit_policy_overrides;
  const context = { task: workspace, traderDefaults, managesExits: true };
  // A label can repeat across groups (the trailing stop and the break-even stop both arm at
  // a gain), so a row is looked up inside its own group's section of the table. Titles and
  // labels are matched as the table escapes them.
  const literal = (text) => esc(text).replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const section = (html, title) => {
    const text = plain(html);
    const start = text.indexOf(`>${esc(title)}</th>`);
    assert.ok(start >= 0, `${title} has its group row`);
    const next = text.indexOf('<tr class="copy-rules-group">', start);
    return text.slice(start, next < 0 ? undefined : next);
  };
  const row = (html, title, label) =>
    section(html, title).match(
      new RegExp(
        `<tr[^>]*><td>${literal(label)}</td><td>([^<]*)</td><td class="copy-rules-source">([^<]*)</td>`
      )
    );
  const expected = {
    trigger_pct: "+15%",
    offset_pct: "+1%",
    band_pct: "±5%",
  };
  for (const effective of [
    {
      ...workspace.effective_policy,
      break_even: traderDefaults.break_even,
      time_stagnation: traderDefaults.time_stagnation,
    },
    effectivePolicy(traderDefaults, overrides),
  ]) {
    const html = rulesHtml({ ...context, effective }, esc);
    for (const rule of INHERITED_RULES) {
      for (const field of rule.fields) {
        const found = row(html, rule.title, field.label);
        assert.ok(found, `${rule.group}.${field.key} has a row`);
        assert.equal(found[2], "Trader default", `${rule.group}.${field.key} is inherited`);
        assert.notEqual(found[1], "—", `${rule.group}.${field.key} shows a value`);
        if (expected[field.key]) assert.equal(found[1], expected[field.key], field.key);
      }
    }
  }
  assert.deepEqual(Object.keys(blankOverrides()), ["stop_loss", "trailing", "roi", "time"]);
});

test("the break-even stop or the flat position exit alone counts as an exit rule", () => {
  const off = { enabled: false };
  const policy = { stop_loss: off, trailing: off, roi: off, time: off };
  const none = exitWarnings({ ...policy, break_even: off, time_stagnation: off }, "buy_only", {});
  assert.ok(
    none.map(plain).some((text) => /no exit rule/i.test(text)),
    none.join(" | ")
  );
  for (const group of ["break_even", "time_stagnation"]) {
    const warnings = exitWarnings(
      { ...policy, break_even: off, time_stagnation: off, [group]: { enabled: true } },
      "buy_only",
      {}
    ).map(plain);
    assert.ok(
      !warnings.some((text) => /no exit rule/i.test(text)),
      `${group}: ${warnings.join(" | ")}`
    );
  }
});
