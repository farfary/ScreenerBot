// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Guard: every dashboard comparison against a transaction's direction uses an id the backend emits.

import assert from "node:assert/strict";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { test } from "node:test";

const ROOT = new URL("../../", import.meta.url).pathname;
const SCRIPTS = join(ROOT, "src/webserver/templates/scripts");

/** The persisted ids of `TransactionDirection`, read from its `as_str`. */
function emittedDirectionIds() {
  const source = readFileSync(join(ROOT, "src/transactions/types.rs"), "utf8");
  const body = source.match(
    /impl TransactionDirection \{[\s\S]*?pub fn as_str\(&self\)[^{]*\{([\s\S]*?)\n {4}\}/
  );
  assert.ok(body, "TransactionDirection::as_str not found");
  return new Set([...body[1].matchAll(/=> "([A-Za-z]+)"/g)].map((match) => match[1]));
}

function scripts(dir) {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return scripts(path);
    return name.endsWith(".js") ? [path] : [];
  });
}

test("transaction direction comparisons name an emitted id", () => {
  const ids = emittedDirectionIds();
  assert.ok(ids.has("TokensIn") && ids.has("SolOut"), `unexpected ids: ${[...ids]}`);
  const offenders = [];
  for (const path of scripts(SCRIPTS)) {
    const source = readFileSync(path, "utf8");
    for (const match of source.matchAll(/\.direction\s*[!=]==?\s*["']([^"']*)["']/g)) {
      if (!ids.has(match[1])) offenders.push(`${relative(ROOT, path)}: "${match[1]}"`);
    }
    if (/\.direction\s*\|\|\s*""\)\.toLowerCase\(\)/.test(source)) {
      offenders.push(`${relative(ROOT, path)}: lower-cased direction id`);
    }
  }
  assert.deepEqual(offenders, []);
});
