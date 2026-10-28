// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Strategy transfer shapes - API detail to editor strategy, request body, export file and import parsing.

import { parseRules, problemsOf } from "./rule_tree.js";

export const EXPORT_FORMAT = "screenerbot.strategy";
export const EXPORT_VERSION = 1;

/** Priority the API takes for a strategy saved without one. */
export const DEFAULT_PRIORITY = 10;
export const MIN_PRIORITY = 1;
export const MAX_PRIORITY = 100;

const STRATEGY_TYPES = new Set(["ENTRY", "EXIT"]);

/** Editor strategy of a `GET /api/strategies/{id}` answer. */
export function strategyFromDetail(data) {
  return {
    id: data.id,
    name: data.name,
    description: data.description || null,
    type: data.strategy_type,
    enabled: Boolean(data.enabled),
    priority: data.priority,
    timeframe: data.timeframe,
    rules: data.rules || null,
    parameters: data.parameters || {},
    created_at: data.created_at,
    updated_at: data.updated_at,
    author: data.author || null,
    version: data.version,
    mode: data.mode,
    entry_strategy_ids: data.entry_strategy_ids,
  };
}

/** Create, update and validate request body of an editor strategy. */
export function requestBodyOf(strategy) {
  return {
    name: strategy.name,
    description: strategy.description || null,
    strategy_type: strategy.type,
    enabled: Boolean(strategy.enabled),
    priority: strategy.priority ?? DEFAULT_PRIORITY,
    timeframe: strategy.timeframe || undefined,
    rules: strategy.rules || null,
    parameters: strategy.parameters || {},
    author: strategy.author || null,
    // Only an exit strategy manages positions; the ids belong to this install.
    entry_strategy_ids: strategy.type === "EXIT" ? strategy.entry_strategy_ids : [],
  };
}

/** Whether `priority` is an integer the priority field accepts. */
export function isValidPriority(priority) {
  return Number.isInteger(priority) && priority >= MIN_PRIORITY && priority <= MAX_PRIORITY;
}

/** The timeframe options every candle condition offers, which are also the strategy's own. */
export function timeframeOptions(schemas) {
  for (const schema of Object.values(schemas ?? {})) {
    const options = schema.parameters?.timeframe?.options;
    if (Array.isArray(options)) return options;
  }
  return [];
}

/** The file an exported strategy is saved as. */
export function exportEnvelope(strategy) {
  return {
    format: EXPORT_FORMAT,
    version: EXPORT_VERSION,
    strategy: {
      name: strategy.name,
      description: strategy.description || null,
      strategy_type: strategy.type,
      timeframe: strategy.timeframe,
      priority: strategy.priority ?? DEFAULT_PRIORITY,
      rules: strategy.rules || null,
      parameters: strategy.parameters || {},
    },
  };
}

/** File name of an exported strategy: the name as a slug, or `strategy` when it has none. */
export function exportFileName(name) {
  const slug = String(name ?? "")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, "-")
    .replace(/^-+|-+$/g, "");
  return `${slug || "strategy"}.json`;
}

/**
 * Read an import file: an export, or a bare strategy object with `strategy_type` or `type`.
 * Returns `{ strategy }` for the editor (no id, rules as saved) or `{ error }` with a
 * `code` of `json`, `shape`, `priority`, `unknown-condition` or `not-applicable` and its
 * `args`. A file without a priority takes the default; one outside the range is refused.
 * @param {string} text File content.
 * @param {object} schemas Condition schemas by type.
 * @param {string|null} defaultTimeframe Timeframe of a file that names none.
 */
export function parseImport(text, schemas, defaultTimeframe) {
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    return { error: { code: "json", args: {} } };
  }
  const isObject = (value) => value !== null && typeof value === "object" && !Array.isArray(value);
  if (!isObject(data) || ("format" in data && data.format !== EXPORT_FORMAT)) {
    return { error: { code: "shape", args: {} } };
  }
  const body = data.format === EXPORT_FORMAT ? data.strategy : data;
  const type = String(body?.strategy_type ?? body?.type ?? "").toUpperCase();
  if (!isObject(body) || !STRATEGY_TYPES.has(type) || typeof body.name !== "string") {
    return { error: { code: "shape", args: {} } };
  }
  const rules = body.rules ?? null;
  if (rules !== null && !isObject(rules)) return { error: { code: "shape", args: {} } };
  const priority = body.priority ?? DEFAULT_PRIORITY;
  if (!isValidPriority(priority)) {
    return { error: { code: "priority", args: { min: MIN_PRIORITY, max: MAX_PRIORITY } } };
  }

  const problems = problemsOf(parseRules(rules, schemas), schemas, type);
  const refused = problems.find(
    ({ code }) => code === "unknown-condition" || code === "not-applicable"
  );
  if (refused) return { error: { code: refused.code, args: refused.args } };

  // An imported strategy arrives switched off: rules from a file are reviewed before they
  // trade.
  return {
    strategy: {
      id: null,
      name: body.name,
      description: body.description || null,
      type,
      enabled: false,
      priority,
      timeframe: typeof body.timeframe === "string" ? body.timeframe : defaultTimeframe,
      rules,
      parameters: isObject(body.parameters) ? body.parameters : {},
      // Strategy ids belong to the install that exported the file: an import manages
      // every position until its entry strategies are picked here.
      entry_strategy_ids: [],
    },
  };
}
