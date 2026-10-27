// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a strategy tested against a token marks the editor's own rules with their
 * verdicts, and the whole strategy reads as one line.
 *
 * - The outcome tree maps onto the editor's rules by position: the root to the root,
 *   children by index; a NOT of the saved tree marks the inverted rule it wraps, and a
 *   saved tree whose top is one condition marks that condition.
 * - A subtree whose shape differs from the editor's is left unmarked, never guessed.
 * - Each condition row and nested group head carries its verdict; an unknown condition
 *   shows its own reason under its head, not as an alert, and a group repeats no reason.
 * - The overall line reads the token, the root's verdict and an unknown root's reason.
 * - Marks survive the editor's redraws until cleared.
 * - Each verdict carries a bare glyph of its own.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import "./fixtures/document_shim.mjs";
import test from "node:test";
import assert from "node:assert/strict";

import { branch, leaf, schemas } from "../lib/strategy_rules.mjs";
import { createRuleEditor } from "../../src/webserver/templates/scripts/pages/strategies/rule_editor.js";
import * as tree from "../../src/webserver/templates/scripts/pages/strategies/rule_tree.js";
import {
  renderNodeVerdict,
  renderTestSummary,
  verdictsByNode,
} from "../../src/webserver/templates/scripts/pages/strategies/test_result.js";

const MISSING_COUNTS = {
  id: "strategies-error-missing-data",
  args: {
    data: { type: "nested", value: { id: "strategies-error-data-transaction-counts" } },
  },
};

const RULES = branch(
  "AND",
  leaf("Rsi"),
  { ...leaf("VolumeSpike"), disabled: true },
  branch("NOT", branch("OR", leaf("CandleSize"), leaf("PriceToMA")))
);

const OUTCOME = {
  verdict: "unknown",
  operator: "AND",
  reason: MISSING_COUNTS,
  children: [
    { verdict: "pass", condition_type: "Rsi" },
    { verdict: "off", condition_type: "VolumeSpike" },
    {
      verdict: "unknown",
      operator: "NOT",
      reason: MISSING_COUNTS,
      children: [
        {
          verdict: "unknown",
          operator: "OR",
          reason: MISSING_COUNTS,
          children: [
            { verdict: "unknown", condition_type: "CandleSize", reason: MISSING_COUNTS },
            { verdict: "fail", condition_type: "PriceToMA" },
          ],
        },
      ],
    },
  ],
};

const text = (fragment) =>
  fragment
    .replace(/<[^>]+>/g, " ")
    .replace(/[⁨⁩]/g, "")
    .replace(/\s+/g, " ")
    .trim();

function editorFor(rules) {
  const editor = createRuleEditor({
    state: { currentStrategy: { type: "ENTRY", timeframe: "5m" } },
    conditionSchemas: schemas,
    $: () => null,
    Utils: { escapeHtml: (value) => String(value).replace(/&/g, "&amp;").replace(/</g, "&lt;") },
    renderStateView: () => "",
    renderVerdict: renderNodeVerdict,
    announce: () => {},
    confirm: async () => ({ confirmed: false }),
    enhanceAllSelects: () => {},
    addTrackedListener: () => {},
    onChange: () => {},
    openCatalog: () => {},
  });
  editor.load(rules);
  return editor;
}

/** Each marked head in document order: `[kind, verdict]`, plus the reason lines. */
function marks(html) {
  const heads = [
    ...html.matchAll(
      /data-kind="(condition|group)"|<span class="strategy-test-verdict" data-verdict="(\w+)"/g
    ),
  ];
  const found = [];
  let kind = null;
  for (const [, nodeKind, verdict] of heads) {
    if (nodeKind) kind = nodeKind;
    else found.push([kind, verdict]);
  }
  return found;
}

test("the outcome tree marks the editor's rules by position", () => {
  const editor = editorFor(RULES);
  const root = editor.model();
  const map = verdictsByNode(root, OUTCOME);
  const [rsi, volume, inverted] = root.children;
  const [candle, price] = inverted.children;

  assert.equal(inverted.inverted, true, "the NOT opens as an inverted group");
  assert.deepEqual(
    [root, rsi, volume, inverted, candle, price].map((node) => map.get(node.id)?.verdict),
    ["unknown", "pass", "off", "unknown", "unknown", "fail"]
  );
  assert.deepEqual(map.get(candle.id).reason, MISSING_COUNTS);
});

test("each row and nested group head shows its verdict; an unknown condition its reason", () => {
  const editor = editorFor(RULES);
  editor.setVerdicts(verdictsByNode(editor.model(), OUTCOME));
  const html = editor.renderRules();

  assert.deepEqual(marks(html), [
    ["condition", "pass"],
    ["condition", "off"],
    ["group", "unknown"],
    ["condition", "unknown"],
    ["condition", "fail"],
  ]);
  const reasons = [...html.matchAll(/<p class="strategy-test-reason"([^>]*)>([^<]*)<\/p>/g)];
  assert.equal(reasons.length, 1, "only the unknown condition repeats the reason");
  assert.match(text(reasons[0][2]), /Transaction counts is not available/);
  assert.doesNotMatch(reasons[0][1], /role=/, "a reason is a note, not an alert");
  assert.equal(
    (html.match(/data-test-mark/g) ?? []).length,
    6,
    "every mark and reason can be taken off in place"
  );
});

test("marks survive a redraw until another strategy loads", () => {
  const editor = editorFor(RULES);
  editor.setVerdicts(verdictsByNode(editor.model(), OUTCOME));
  editor.render();
  assert.equal(marks(editor.renderRules()).length, 5);
  editor.clearVerdicts();
  assert.equal(marks(editor.renderRules()).length, 0);
  editor.setVerdicts(verdictsByNode(editor.model(), OUTCOME));
  editor.load(RULES);
  assert.equal(marks(editor.renderRules()).length, 0);
});

test("a shape that differs from the editor's is left unmarked", () => {
  const editor = editorFor(RULES);
  const root = editor.model();
  const changed = {
    ...OUTCOME,
    children: [OUTCOME.children[0], OUTCOME.children[2]],
  };
  const map = verdictsByNode(root, changed);
  assert.deepEqual([...map.keys()], [root.id], "children of another count are not guessed");

  const swapped = {
    ...OUTCOME,
    children: [OUTCOME.children[1], OUTCOME.children[0], OUTCOME.children[2]],
  };
  const partial = verdictsByNode(root, swapped);
  assert.equal(partial.has(root.children[0].id), false);
  assert.equal(partial.has(root.children[1].id), false);
});

test("a saved tree that is one condition, or a NOT of one, marks that condition", () => {
  const bare = editorFor(leaf("Rsi")).model();
  assert.equal(
    verdictsByNode(bare, { verdict: "fail", condition_type: "Rsi" }).get(bare.children[0].id)
      ?.verdict,
    "fail"
  );

  const negated = editorFor(branch("NOT", leaf("Rsi"))).model();
  const [only] = negated.children;
  assert.equal(tree.findNode(negated, only.id).inverted, true);
  const map = verdictsByNode(negated, {
    verdict: "pass",
    operator: "NOT",
    children: [{ verdict: "fail", condition_type: "Rsi" }],
  });
  assert.equal(map.get(only.id)?.verdict, "pass", "an inverted rule reads as its NOT");
});

test("the overall line reads the token, the root verdict and an unknown root's reason", () => {
  assert.match(
    text(renderTestSummary(OUTCOME, { token: "BONK" })),
    /^Test against BONK Unknown Transaction counts is not available/
  );
  const decided = renderTestSummary({ verdict: "pass", operator: "AND" }, { token: "BONK" });
  assert.equal(text(decided), "Test against BONK Passes");
  assert.doesNotMatch(decided, /strategy-test-summary-reason/);
});

test("each verdict carries its own bare glyph", () => {
  const glyphs = {};
  for (const verdict of ["pass", "fail", "unknown", "off"]) {
    const { verdict: html } = renderNodeVerdict({ verdict, reason: null }, { condition: true });
    const icons = [...html.matchAll(/<i class="([^"]+)" aria-hidden="true"><\/i>/g)].map(
      ([, name]) => name
    );
    assert.equal(icons.length, 1, `${verdict}: one glyph`);
    assert.match(icons[0], /^icon-[a-z-]+$/, `${verdict}: a bare glyph class`);
    glyphs[verdict] = icons[0];
  }
  assert.equal(new Set(Object.values(glyphs)).size, 4, "verdicts are told apart");
});
