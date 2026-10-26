// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: no dashboard script grades a score by comparing it to a number.
 *
 * RugCheck's normalised score is a risk score (higher is riskier). Its bands are drawn
 * once, in Rust (`tokens::security::SecurityRiskBand`), and every response that grades a
 * token sends the band beside the score. A threshold written in a script drifts from
 * that owner, and one such threshold read the scale backwards and labelled the riskiest
 * tokens safe. A script maps the band to its label; a score is only ever displayed.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import { SCRIPTS_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const SCORE_OPERAND = String.raw`[\w$.?]*score[\w$]*\)?`;
const NUMBER = String.raw`-?\d`;
const COMPARISONS = [
  new RegExp(String.raw`${SCORE_OPERAND}\s*[<>]=?\s*${NUMBER}`, "i"),
  new RegExp(String.raw`${NUMBER}[\d.]*\s*[<>]=?\s*${SCORE_OPERAND}`, "i"),
];

/** Lines of `source` that compare a score to a numeric literal: `[{ line, text }]`. */
export function scoreComparisons(source) {
  const found = [];
  source.split("\n").forEach((text, index) => {
    const code = text.replace(/\/\/.*$/, "");
    if (COMPARISONS.some((pattern) => pattern.test(code))) {
      found.push({ line: index + 1, text: text.trim() });
    }
  });
  return found;
}

test("the scanner catches a score banded in either operand order", () => {
  assert.equal(scoreComparisons("const level = score >= 70 ? 1 : 2;").length, 1);
  assert.equal(scoreComparisons("if (token.security_score < 40) {").length, 1);
  assert.equal(scoreComparisons("if (30 >= safetyScore) {").length, 1);
  assert.equal(scoreComparisons("const label = I18n.label(LABELS, token.security_band);").length, 0);
});

test("no dashboard script compares a security score to a number", async () => {
  const offenders = [];
  for (const file of await walk(SCRIPTS_ROOT)) {
    if (!file.endsWith(".js")) continue;
    for (const hit of scoreComparisons(await readFile(file, "utf8"))) {
      offenders.push(`${repoPath(file)}:${hit.line} ${hit.text}`);
    }
  }
  assert.deepEqual(offenders, [], "map the backend's band to a label instead");
});
