/**
 * Contract tests for the Assistant "Settings" tab wire mapping
 * (`pages/assistant/config_contract.js`).
 *
 * `GET`/`PATCH /api/llm-analysis/config` speak a FLAT body: snake_case scalar
 * keys, the confidence fields a Rust `u8` percentage (0-100). The dashboard
 * once read `config.filtering?.min_confidence` and PATCHed a nested
 * `{ filtering: { ... } }` object that serde drops on the floor, so every save
 * silently did nothing. These assertions pin the mapping to the real Rust
 * struct so that mismatch cannot come back unnoticed.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import {
  ANALYSIS_CONFIG_FIELDS,
  SLIDER_SUFFIX,
  readAnalysisConfigForm,
  applyAnalysisConfigForm,
  sliderLabelId,
} from "../../src/webserver/templates/scripts/pages/assistant/config_contract.js";

const repoFile = (rel) => fileURLToPath(new URL(`../../${rel}`, import.meta.url));

/** Field names inside a `pub struct <name> { ... }` block in a Rust source file. */
function rustStructFields(source, structName) {
  const start = source.indexOf(`pub struct ${structName} {`);
  assert.notEqual(start, -1, `struct ${structName} not found`);
  const body = source.slice(start, source.indexOf("\n}", start));
  return new Set([...body.matchAll(/^\s*pub\s+([a-z0-9_]+):/gm)].map((m) => m[1]));
}

const TYPES_RS = readFileSync(repoFile("src/webserver/routes/llm_analysis/types.rs"), "utf8");
const REQUEST_FIELDS = rustStructFields(TYPES_RS, "UpdateAnalysisConfigRequest");
const RESPONSE_FIELDS = rustStructFields(TYPES_RS, "AnalysisConfigResponse");

/** A DOM stand-in: `getControl(id)` returns a checkbox- or range-like element. */
function form(values) {
  const els = new Map();
  for (const [, id, kind] of ANALYSIS_CONFIG_FIELDS) {
    if (!(id in values)) continue;
    els.set(id, kind === "bool" ? { checked: values[id] } : { value: String(values[id]) });
  }
  return (id) => els.get(id) || null;
}

test("every mapped key is flat and exists on the Rust request + response structs", () => {
  for (const [key] of ANALYSIS_CONFIG_FIELDS) {
    assert.doesNotMatch(key, /\./, `${key} must be a flat key, not nested`);
    assert.ok(REQUEST_FIELDS.has(key), `${key} missing from UpdateAnalysisConfigRequest`);
    assert.ok(RESPONSE_FIELDS.has(key), `${key} missing from AnalysisConfigResponse`);
  }
});

test("the nested shapes serde silently drops are never produced", () => {
  const patch = readAnalysisConfigForm(
    form({
      "setting-filtering-enabled": true,
      "setting-min-confidence": 65,
      "setting-entry-analysis": true,
    })
  );
  for (const dead of ["filtering", "entry_analysis", "exit_analysis"]) {
    assert.equal(patch[dead], undefined, `${dead} object must not be sent`);
  }
  assert.equal(patch.filtering_enabled, true);
  assert.equal(patch.entry_analysis_enabled, true);
});

test("confidence is an integer percentage (u8), not a 0-1 float", () => {
  const patch = readAnalysisConfigForm(
    form({ "setting-min-confidence": 70, "setting-blacklist-min-confidence": 30 })
  );
  assert.equal(patch.filtering_min_confidence, 70);
  assert.equal(patch.auto_blacklist_min_confidence, 30);
  assert.ok(Number.isInteger(patch.filtering_min_confidence));
});

test("only controls present in the DOM are sent", () => {
  const patch = readAnalysisConfigForm(form({ "setting-cache-ttl": 600 }));
  assert.deepEqual(Object.keys(patch), ["cache_ttl_seconds"]);
  assert.equal(patch.cache_ttl_seconds, 600);
});

test("booleans coerce and never leak a truthy string", () => {
  const patch = readAnalysisConfigForm(form({ "setting-fallback-pass": false }));
  assert.strictEqual(patch.filtering_fallback_pass, false);
});

test("applyAnalysisConfigForm round-trips a flat response onto the controls", () => {
  const state = {
    "setting-filtering-enabled": { checked: false },
    "setting-min-confidence": { value: "0" },
    "setting-cache-ttl": { value: "0" },
  };
  const labels = {};
  applyAnalysisConfigForm(
    {
      filtering_enabled: true,
      filtering_min_confidence: 80,
      cache_ttl_seconds: 900,
    },
    (id) => state[id] || null,
    (id, text) => {
      labels[id] = text;
    }
  );
  assert.equal(state["setting-filtering-enabled"].checked, true);
  assert.equal(state["setting-min-confidence"].value, "80");
  assert.equal(state["setting-cache-ttl"].value, "900");
  assert.equal(labels["setting-min-confidence"], "80%");
  assert.equal(labels["setting-cache-ttl"], "900");
});

test("a missing key leaves its control untouched", () => {
  const state = { "setting-min-confidence": { value: "55" } };
  applyAnalysisConfigForm(
    {},
    (id) => state[id] || null,
    () => {}
  );
  assert.equal(state["setting-min-confidence"].value, "55");
});

test("slider label id and suffix table stay in step with the field kinds", () => {
  assert.equal(sliderLabelId("setting-min-confidence"), "slider-value-min-confidence");
  for (const [, , kind] of ANALYSIS_CONFIG_FIELDS) {
    if (kind !== "bool") assert.ok(kind in SLIDER_SUFFIX, `no suffix for kind ${kind}`);
  }
});
