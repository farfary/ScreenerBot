// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: dashboard scripts take the tone of a signed figure from `signedTone`
 * (`core/format.js`) at the decimals the figure is printed with, never from a
 * comparison of the raw value against zero.
 *
 * A raw comparison colours a change that prints as "0.00%" green or red and points an
 * arrow at a change that is not shown. The patterns refused here are the shapes that
 * did: a sign ternary yielding a tone class, a `classList.toggle` of a tone class on a
 * sign test, and an `if` on the sign appending a tone class.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

// Installs the global I18n the dashboard formatters read.
import "./fixtures/i18n_en.mjs";
import { signedSol, toneClass } from "../../src/webserver/templates/scripts/pages/copy/format.js";

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const SCRIPTS = path.join(ROOT, "src/webserver/templates/scripts");
const OWNER = path.join(SCRIPTS, "core", "format.js");

const TONE = String.raw`(?:positive|negative|profit|loss|gain|pos|neg|up|down|bull|bear)`;
const SIGN_TEST = String.raw`[<>]=?\s*0(?:\.0+)?\b`;
const RAW_TONE = [
  new RegExp(String.raw`${SIGN_TEST}\s*\?\s*["'\`][\w -]*\b${TONE}\b[\w -]*["'\`]`, "g"),
  new RegExp(String.raw`classList\.toggle\(\s*["'][\w-]*\b${TONE}\b["'][^)]*${SIGN_TEST}`, "g"),
  new RegExp(String.raw`if\s*\([^)]*${SIGN_TEST}\s*\)\s*\w+\s*\+=\s*["'][\w -]*\b${TONE}\b`, "g"),
];

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(full) : [full];
  });
}

function lineOf(source, index) {
  return source.slice(0, index).split("\n").length;
}

test("the patterns catch the raw-sign tone shapes", () => {
  const offending = [
    'const cls = num > 0 ? "value-positive" : num < 0 ? "value-negative" : "";',
    "const tone =\n  pnl > 0 ? 'positive' : '';",
    'const tone = change >= 0 ? "pos" : "neg";',
    'el.classList.toggle("negative", change !== null && change < 0);',
    'if (pnl > 0) cls += " profit";',
    'const tone = last >= 0 ? "is-positive" : "is-negative";',
  ];
  for (const sample of offending) {
    assert.ok(
      RAW_TONE.some((pattern) => sample.match(pattern)),
      `not caught: ${sample}`
    );
  }
  const allowed = [
    "const y = net >= 0 ? mid - h : mid;",
    'variant: alerts > 0 ? "warning" : "success",',
    "const cls = Utils.signedTone(change, 2);",
  ];
  for (const sample of allowed) {
    assert.ok(!RAW_TONE.some((pattern) => sample.match(pattern)), `caught: ${sample}`);
  }
});

test("no dashboard script picks a tone from the raw sign", () => {
  const offenders = [];
  for (const file of walk(SCRIPTS)) {
    if (!file.endsWith(".js") || file === OWNER) continue;
    const source = fs.readFileSync(file, "utf8");
    for (const pattern of RAW_TONE) {
      for (const match of source.matchAll(pattern)) {
        offenders.push(`${path.relative(ROOT, file)}:${lineOf(source, match.index)}`);
      }
    }
  }
  assert.deepEqual(offenders, [], "take the tone from signedTone at the printed decimals");
});

test("a P&L printed as zero carries no tone", () => {
  for (const value of [0, 0.00004, -0.00004, null]) {
    assert.equal(toneClass(value), "", `${value} prints ${signedSol(value)}`);
  }
  assert.equal(toneClass(0.0001), "is-positive");
  assert.equal(toneClass(-0.00006), "is-negative");
});
