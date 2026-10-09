// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every event subtype code the Rust producers record has a label.
 *
 * The Events table and the event details dialog showed the stored subtype id
 * ("service_started") wherever `EVENT_SUBTYPE_LABELS` (ui/event_labels.js) had no entry,
 * and that map only knew the scheduled-task outcomes. This guard reads the codes from
 * the Rust sources in every form a producer writes one:
 *
 * - the `subtype` argument of the recorders in src/events/recorders/, as a literal;
 * - `Event::info/warn/error/debug(EventCategory::…, Some("code"…))` rows;
 * - `record_force_stop_event("Code", …)` rows;
 * - the gap outcome table (`("code", Severity::…)` in ohlcvs/gaps.rs) and
 *   `subtype: "code"` announcement fields;
 * - the `ScheduledTaskOutcome` codes;
 * - a candle feed's `format!("{}_code", feed.label)`, labelled by its shared code.
 *
 * The map and the producers must name the same codes, so a new code fails until it is
 * labelled and a removed one fails until its label goes. A recorder call whose subtype
 * is not a literal is accepted only in the files listed in `COMPUTED`.
 *
 * Run with `npm run test:js`.
 */

import "./fixtures/i18n_en.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

globalThis.window = { addEventListener() {} };
// core/utils.js registers document listeners when it is imported.
globalThis.document = { addEventListener() {}, readyState: "complete" };

const { EVENT_FEED_SUBTYPE_CODES, EVENT_SUBTYPE_LABELS, eventSubtypeLabel } =
  await import("../../src/webserver/templates/scripts/ui/event_labels.js");

const SRC = new URL("../../src/", import.meta.url).pathname;
const LOCALES = new URL("../../locales/", import.meta.url).pathname;

// Argument positions that form the subtype of each recorder, joined with "_".
const RECORDERS = {
  record_system_event: [0, 1],
  record_token_event: [1],
  record_wallet_event: [0],
  record_security_event: [1],
  record_connectivity_event: [1],
  record_ohlcv_event: [0],
  record_filtering_event: [0],
  record_trader_event: [0],
  record_position_event_flexible: [0],
  record_rpc_event: [1],
  record_api_event: [1],
  record_transaction_event: [1],
  record_pool_event: [2, 4],
  record_position_event: [2],
};

// Recorder calls whose subtype is computed, and where its code comes from instead.
const COMPUTED = {
  "apis/stats.rs": "the API endpoint name, shown as stored",
  "ohlcvs/fetcher.rs": "a candle feed code",
  "ohlcvs/monitor.rs": "a candle feed code",
  "ohlcvs/gaps.rs": "the gap outcome table",
  "trader/copy/notify.rs": "the announcement's subtype field",
};

function rustFiles(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) rustFiles(full, out);
    else if (entry.name.endsWith(".rs")) out.push(full);
  }
  return out;
}

/** The top-level arguments of the call whose open parenthesis ends at `start`. */
function callArguments(source, start) {
  const args = [];
  let depth = 1;
  let current = "";
  let inString = false;
  for (let i = start; i < source.length; i += 1) {
    const char = source[i];
    if (inString) {
      current += char;
      if (char === "\\") current += source[++i];
      else if (char === '"') inString = false;
      continue;
    }
    if (char === '"') inString = true;
    else if ("([{".includes(char)) depth += 1;
    else if (")]}".includes(char)) {
      depth -= 1;
      if (depth === 0) break;
    } else if (char === "," && depth === 1) {
      args.push(current.trim());
      current = "";
      continue;
    }
    current += char;
  }
  args.push(current.trim());
  return args.filter(Boolean);
}

const literal = (text) => /^"([^"]*)"$/.exec(text ?? "")?.[1];

function producedCodes() {
  const codes = new Set();
  const computed = new Set();
  for (const file of rustFiles(SRC)) {
    const relative = path.relative(SRC, file);
    if (relative.startsWith("events/recorders/")) continue;
    const source = fs.readFileSync(file, "utf8").replace(/\/\/[^\n"]*$/gm, "");

    for (const [recorder, positions] of Object.entries(RECORDERS)) {
      for (const match of source.matchAll(new RegExp(`\\b${recorder}\\s*\\(`, "g"))) {
        const args = callArguments(source, match.index + match[0].length);
        const parts = positions.map((position) => literal(args[position]));
        if (parts.every((part) => part !== undefined)) codes.add(parts.join("_"));
        else computed.add(relative);
      }
    }
    for (const match of source.matchAll(
      /Event::(?:info|warn|error|debug)\(\s*EventCategory::\w+,\s*Some\("([^"]+)"/g
    )) {
      codes.add(match[1]);
    }
    for (const match of source.matchAll(/record_force_stop_event\(\s*"([^"]+)"/g)) {
      codes.add(match[1]);
    }
    if (relative === "ohlcvs/gaps.rs") {
      for (const match of source.matchAll(/=>\s*\("([a-z_]+)",\s*Severity::/g)) codes.add(match[1]);
    }
    for (const line of source.split("\n").filter((text) => /\bsubtype:\s*(?:if\b|")/.test(text))) {
      for (const match of line.matchAll(/"([a-z_]+)"/g)) codes.add(match[1]);
    }
    for (const match of source.matchAll(/format!\("\{\}_([a-z_]+)", feed\.label\)/g)) {
      codes.add(match[1]);
    }
    if (relative === "events/display_text.rs") {
      for (const match of source.matchAll(/Self::\w+ => "([a-z_]+)"/g)) codes.add(match[1]);
    }
  }
  return { codes, computed };
}

const { codes, computed } = producedCodes();

test("every produced subtype code has a label, and every label a producer", () => {
  assert.ok(codes.size > 100, `only ${codes.size} codes found; the source scan broke`);
  const labelled = new Set(Object.keys(EVENT_SUBTYPE_LABELS));
  assert.deepEqual([...codes].filter((code) => !labelled.has(code)).sort(), [], "unlabelled");
  assert.deepEqual([...labelled].filter((code) => !codes.has(code)).sort(), [], "not produced");
});

test("a computed subtype is recorded only where its code is accounted for", () => {
  assert.deepEqual([...computed].sort(), Object.keys(COMPUTED).sort());
});

test("candle feed codes share the labels of their codes", () => {
  for (const code of EVENT_FEED_SUBTYPE_CODES) {
    assert.ok(Object.hasOwn(EVENT_SUBTYPE_LABELS, code), code);
    assert.equal(eventSubtypeLabel(`somefeed_${code}`), eventSubtypeLabel(code));
  }
  assert.equal(eventSubtypeLabel("service_started"), "Service started");
  assert.equal(eventSubtypeLabel("getPools"), "getPools", "an endpoint name is shown as stored");
});

test("every subtype label is in every locale", () => {
  const keys = Object.values(EVENT_SUBTYPE_LABELS);
  const missing = [];
  for (const locale of fs.readdirSync(LOCALES)) {
    const file = path.join(LOCALES, locale, "events.ftl");
    if (!fs.existsSync(file)) continue;
    const defined = new Set(
      [...fs.readFileSync(file, "utf8").matchAll(/^([a-z0-9-]+) =/gm)].map((m) => m[1])
    );
    for (const key of keys) if (!defined.has(key)) missing.push(`${locale}: ${key}`);
  }
  assert.deepEqual(missing, []);
});
