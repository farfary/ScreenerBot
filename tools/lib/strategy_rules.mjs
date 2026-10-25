// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategy test fixtures - the recorded condition schemas and builders for saved rule trees.

import { readFileSync } from "node:fs";

import { FIXTURES_ROOT } from "./dashboard_fixtures.mjs";

const recorded = JSON.parse(
  readFileSync(`${FIXTURES_ROOT}/trader/strategies_condition_schemas.json`, "utf8")
);

export const schemas = recorded.schemas;
export const defaultTimeframe = recorded.default_timeframe;

/** Saved parameters of a condition: every schema parameter at its default, then `values`. */
export function savedParameters(type, values = {}) {
  const specs = schemas[type].parameters;
  return Object.fromEntries(
    Object.entries(specs).map(([key, spec]) => [
      key,
      { value: key in values ? values[key] : (spec.default ?? null), default: spec.default ?? null },
    ])
  );
}

/** A saved leaf node. */
export function leaf(type, values = {}) {
  return { condition: { type, parameters: savedParameters(type, values) } };
}

/** A saved branch node. */
export function branch(operator, ...conditions) {
  return { operator, conditions };
}
