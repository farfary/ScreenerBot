// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every page DataTable mount fills its panel. The table wrapper is
 * `height: 100%`, so its mount must be a flex item that takes the panel's height
 * (`flex: 1; min-height: 0; overflow: hidden`). A mount left at its content height
 * hands the table an indefinite height: rows past the panel are clipped with no
 * scrollbar, and some engines shrink the table to a single row.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, STYLES_ROOT, repoPath, rulesIn, walk } from "../lib/dashboard_ui.mjs";

const scripts = (await walk(`${SCRIPTS_ROOT}/pages`)).filter((file) => file.endsWith(".js"));
const stylesheets = (await walk(STYLES_ROOT)).filter((file) => file.endsWith(".css"));
const rules = stylesheets.flatMap((file) => rulesIn(readFileSync(file, "utf8")));

/** Every `#id` a page passes as a DataTable container, with where it is declared. */
function pageMounts() {
  const mounts = [];
  for (const file of scripts) {
    const source = readFileSync(file, "utf8");
    for (const call of source.matchAll(/\bnew DataTable\(\{\s*container:\s*"#([\w-]+)"/g))
      mounts.push({ id: call[1], where: `${repoPath(file)} #${call[1]}` });
  }
  return mounts;
}

test("every page DataTable mount fills its panel", () => {
  const mounts = pageMounts();
  assert.ok(mounts.length >= 10, "page DataTable mounts not found");
  const loose = mounts.filter(({ id }) => {
    const body = rules
      .filter((rule) => rule.selector === `#${id}` || rule.selector === `.${id}`)
      .map((rule) => rule.body)
      .join("\n");
    return !(
      /\bflex:\s*1\b/.test(body) &&
      /\bmin-height:\s*0\b/.test(body) &&
      /\boverflow:\s*hidden\b/.test(body)
    );
  });
  assert.deepEqual(
    loose.map(({ where }) => where),
    [],
    "mounts without flex: 1, min-height: 0 and overflow: hidden"
  );
});
