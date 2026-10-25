// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the strategy rule model keeps a saved rule tree intact and checks it against
 * the condition schemas.
 *
 * The editor once flattened every tree into one AND list and dropped switched-off rows,
 * so opening a strategy with an OR or a NOT and saving it changed what it traded.
 *
 * - `serializeRules(parseRules(x))` equals `x` for AND, OR, NOT over a leaf and over a
 *   group, a switched-off leaf and NOT wrapper, three group levels, a parameter with
 *   constraints, an extra parameter key and a condition type the schemas do not list.
 * - A bare leaf is the only child of an AND root; no rules is an empty root.
 * - Each edit leaves the serialization it names, and a group refuses a child it could
 *   not hold within four levels, an inverted group included.
 * - Each problem code fires for the state it names and stops when that state is fixed;
 *   switched-off rows are checked too.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { branch, leaf, savedParameters, schemas } from "../lib/strategy_rules.mjs";
import * as tree from "../../src/webserver/templates/scripts/pages/strategies/rule_tree.js";

const roundTrip = (rules) => tree.serializeRules(tree.parseRules(rules, schemas), schemas);
const codes = (root, type = "ENTRY") => tree.problemsOf(root, schemas, type).map((p) => p.code);

test("a saved tree round-trips through the model unchanged", () => {
  const withConstraints = leaf("Rsi", { period: 21 });
  withConstraints.condition.parameters.period.constraints = { min: 5, max: 50 };
  const withExtra = leaf("VolumeSpike");
  withExtra.condition.parameters.legacy_key = { value: 3, default: 3 };
  const mystery = {
    condition: { type: "Mystery", parameters: { depth: { value: 1, default: 0 }, bare: 7 } },
  };
  const rules = branch(
    "AND",
    leaf("PriceChangePercent"),
    branch(
      "OR",
      leaf("Rsi", { comparison: "ABOVE" }),
      branch("NOT", leaf("CandleSize")),
      branch(
        "NOT",
        branch("AND", leaf("PriceToMA"), branch("OR", leaf("LiquidityLevel"), withExtra))
      )
    ),
    { ...leaf("PositionProfit"), disabled: true },
    { ...branch("NOT", leaf("DrawdownFromPeak")), disabled: true },
    withConstraints,
    mystery,
    leaf("PriceChangePercent", { timeframe: "1h", time_unit: "HOURS" })
  );
  assert.deepEqual(roundTrip(rules), rules);
});

test("a bare leaf is the only child of an AND root and no rules is an empty root", () => {
  const bare = leaf("Rsi");
  const root = tree.parseRules(bare, schemas);
  assert.equal(root.children.length, 1);
  assert.deepEqual(tree.serializeRules(root, schemas), branch("AND", bare));

  const empty = tree.parseRules(null, schemas);
  assert.equal(empty.match, "all");
  assert.deepEqual(empty.children, []);
  assert.equal(tree.serializeRules(empty, schemas), null);
});

test("a NOT over a switched-off child is one switched-off inverted row", () => {
  const rules = branch("AND", branch("NOT", { ...leaf("Rsi"), disabled: true }));
  const [row] = tree.parseRules(rules, schemas).children;
  assert.deepEqual([row.inverted, row.disabled], [true, true]);
  assert.deepEqual(roundTrip(rules), branch("AND", { ...branch("NOT", leaf("Rsi")), disabled: true }));
});

test("a missing parameter takes the schema default and an unknown type stays unknown", () => {
  const rules = branch("AND", { condition: { type: "Rsi", parameters: { period: { value: 7, default: 14 } } } });
  const [row] = tree.parseRules(rules, schemas).children;
  assert.equal(row.params.period, 7);
  assert.equal(row.params.threshold, 30);
  assert.equal(row.params.timeframe, null);
  assert.equal(row.known, true);

  const [unknown] = tree.parseRules(branch("AND", { condition: { type: "Nope", parameters: {} } }), schemas).children;
  assert.equal(unknown.known, false);
});

test("edits leave the serialization they name", () => {
  const root = tree.parseRules(null, schemas);
  const first = tree.addCondition(root, root.id, "Rsi", schemas);
  const second = tree.addCondition(root, root.id, "VolumeSpike", schemas);
  assert.ok(first && second && first !== second);
  assert.equal(tree.addCondition(root, root.id, "Nope", schemas), false);
  assert.equal(tree.addCondition(root, 9999, "Rsi", schemas), false);
  assert.deepEqual(tree.serializeRules(root, schemas), branch("AND", leaf("Rsi"), leaf("VolumeSpike")));

  tree.setParam(root, first, "period", 21);
  assert.equal(tree.moveNode(root, first, 1), first);
  assert.equal(tree.moveNode(root, first, 1), false, "the last row cannot move down");
  const copy = tree.duplicateNode(root, first);
  assert.notEqual(copy, first);
  assert.deepEqual(
    tree.serializeRules(root, schemas),
    branch("AND", leaf("VolumeSpike"), leaf("Rsi", { period: 21 }), leaf("Rsi", { period: 21 }))
  );

  tree.setParam(root, copy, "period", 30);
  tree.setInverted(root, copy, true);
  tree.setDisabled(root, second, true);
  const group = tree.addGroup(root, root.id);
  const inner = tree.addCondition(root, group, "CandleSize", schemas);
  tree.setMatch(root, group, "any");
  assert.deepEqual(
    tree.serializeRules(root, schemas),
    branch(
      "AND",
      { ...leaf("VolumeSpike"), disabled: true },
      leaf("Rsi", { period: 21 }),
      branch("NOT", leaf("Rsi", { period: 30 })),
      branch("OR", leaf("CandleSize"))
    )
  );

  assert.equal(tree.removeNode(root, inner), inner);
  assert.equal(tree.removeNode(root, root.id), false, "the root cannot be removed");
  assert.equal(tree.setDisabled(root, root.id, true), false, "the root cannot be switched off");
  assert.equal(tree.depthOf(root, group), 2);
  assert.equal(tree.findNode(root, inner), null);

  tree.setInverted(root, root.id, true);
  assert.equal(tree.serializeRules(root, schemas).operator, "NOT");
  assert.equal(tree.depthOf(root, group), 3, "the root's NOT wrapper takes a level");
});

test("a group refuses a child it could not hold within four levels", () => {
  const root = tree.parseRules(null, schemas);
  const second = tree.addGroup(root, root.id);
  const third = tree.addGroup(root, second);
  assert.ok(second && third);
  assert.equal(tree.canAddGroup(root, root.id), true);
  assert.equal(tree.canAddGroup(root, second), true);
  assert.equal(tree.canAddGroup(root, third), false);
  assert.equal(tree.addGroup(root, third), false);
  assert.equal(tree.canAddCondition(root, third), true);

  const deepest = tree.addCondition(root, third, "Rsi", schemas);
  assert.equal(tree.depthOf(root, deepest), 4);
  assert.equal(tree.canInvert(root, second), false, "inverting would push the condition to level five");
  assert.equal(tree.setInverted(root, second, true), false);
  assert.equal(tree.canInvert(root, third), false);
  assert.equal(tree.canInvert(root, deepest), false);
  assert.deepEqual(codes(root), []);

  tree.removeNode(root, deepest);
  assert.equal(tree.setInverted(root, third, true), third);
  assert.equal(tree.canAddCondition(root, third), false, "an inverted group at level four holds nothing");
  assert.equal(tree.MAX_DEPTH, 4);
});

test("each problem code fires for its state and stops when it is fixed", () => {
  const root = tree.parseRules(null, schemas);
  assert.deepEqual(codes(root), ["rules-empty"]);

  const rsi = tree.addCondition(root, root.id, "Rsi", schemas);
  assert.deepEqual(codes(root), []);

  tree.setDisabled(root, rsi, true);
  assert.deepEqual(codes(root), ["rules-empty"], "a root of switched-off rows decides nothing");
  tree.setDisabled(root, rsi, false);
  const keep = tree.addCondition(root, root.id, "VolumeSpike", schemas);

  const group = tree.addGroup(root, root.id);
  assert.deepEqual(codes(root), ["group-empty"]);
  const inner = tree.addCondition(root, group, "CandleSize", schemas);
  tree.setDisabled(root, inner, true);
  assert.deepEqual(codes(root), ["group-empty"], "a group of switched-off rows decides nothing");
  tree.setDisabled(root, inner, false);
  assert.deepEqual(codes(root), []);

  const holding = tree.addCondition(root, root.id, "PositionHoldingTime", schemas);
  assert.deepEqual(codes(root, "ENTRY"), ["not-applicable"]);
  assert.deepEqual(codes(root, "EXIT"), []);
  tree.removeNode(root, holding);
  assert.deepEqual(codes(root, "ENTRY"), []);

  tree.setParam(root, rsi, "period", null);
  assert.deepEqual(tree.problemsOf(root, schemas, "ENTRY"), [
    { nodeId: rsi, key: "period", code: "required", args: {} },
  ]);
  tree.setParam(root, rsi, "period", Number.NaN);
  assert.deepEqual(codes(root), ["required"]);
  tree.setParam(root, rsi, "period", 1000);
  assert.deepEqual(tree.problemsOf(root, schemas, "ENTRY"), [
    { nodeId: rsi, key: "period", code: "range", args: { min: 2, max: 100 } },
  ]);
  tree.setParam(root, rsi, "period", 14);
  assert.deepEqual(codes(root), []);

  tree.setParam(root, rsi, "timeframe", null);
  assert.deepEqual(codes(root), [], "an optional enum may stay unset");
  tree.setParam(root, rsi, "timeframe", "2m");
  assert.deepEqual(tree.problemsOf(root, schemas, "ENTRY"), [
    { nodeId: rsi, key: "timeframe", code: "invalid-option", args: {} },
  ]);
  tree.setParam(root, rsi, "timeframe", "5m");
  tree.setParam(root, rsi, "comparison", null);
  assert.deepEqual(codes(root), ["invalid-option"], "a required enum must be set");
  tree.setParam(root, rsi, "comparison", "BELOW");
  assert.deepEqual(codes(root), []);

  tree.setDisabled(root, rsi, true);
  tree.setParam(root, rsi, "threshold", 500);
  assert.deepEqual(codes(root), ["range"], "a switched-off row is still checked");
  tree.setParam(root, rsi, "threshold", 30);

  const unknown = tree.parseRules(branch("AND", { condition: { type: "Nope", parameters: {} } }), schemas);
  assert.deepEqual(tree.problemsOf(unknown, schemas, "ENTRY"), [
    { nodeId: unknown.children[0].id, code: "unknown-condition", args: { type: "Nope" } },
  ]);
  assert.ok(keep);
});

test("an inverted group counts as a level of its own", () => {
  const deep = tree.parseRules(
    branch("AND", branch("AND", branch("NOT", branch("AND", leaf("Rsi"))))),
    schemas
  );
  assert.deepEqual(codes(deep), ["too-deep"]);

  const fits = tree.parseRules(branch("AND", branch("AND", branch("AND", leaf("Rsi")))), schemas);
  assert.deepEqual(codes(fits), []);
});

test("savedParameters matches the schema defaults", () => {
  assert.equal(savedParameters("Rsi").period.value, 14);
});
