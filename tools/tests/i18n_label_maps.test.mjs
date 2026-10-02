// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Every frozen `<NAME>_LABELS` map in the dashboard scripts (`I18n.label`
 * tables) must point at message ids that exist in the source catalog.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const ROOT = new URL("../../", import.meta.url).pathname;
const SCRIPTS = path.join(ROOT, "src/webserver/templates/scripts");
const CATALOG = path.join(ROOT, "locales/en");

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return walk(full);
    return entry.name.endsWith(".js") ? [full] : [];
  });
}

const ids = new Set();
for (const file of fs.readdirSync(CATALOG).filter((name) => name.endsWith(".ftl"))) {
  const text = fs.readFileSync(path.join(CATALOG, file), "utf8");
  for (const match of text.matchAll(/^([a-z][a-z0-9-]*)\s*=/gm)) ids.add(match[1]);
}

const MAP = /const (\w+_LABELS) = Object\.freeze\(\{([\s\S]*?)\}\);/g;

test("label maps reference existing catalog messages", () => {
  let maps = 0;
  for (const file of walk(SCRIPTS)) {
    const source = fs.readFileSync(file, "utf8");
    for (const match of source.matchAll(MAP)) {
      maps += 1;
      const values = [...match[2].matchAll(/:\s*"([^"]+)"/g)].map((entry) => entry[1]);
      assert.ok(values.length > 0, `${match[1]} in ${file} has no entries`);
      for (const value of values) {
        assert.ok(ids.has(value), `${match[1]} in ${path.relative(ROOT, file)}: "${value}" is not in locales/en`);
      }
      assert.equal(new Set(values).size, values.length, `${match[1]} repeats a message key`);
    }
  }
  assert.ok(maps >= 7, `expected the label maps to be found, saw ${maps}`);
});
