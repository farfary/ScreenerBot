// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every `Utils.<name>` read through `import * as Utils from ".../core/utils.js"`
 * names an export of `core/utils.js`.
 *
 * A formatter that lives in `core/format.js` but is not re-exported by `core/utils.js`
 * is `undefined` on the namespace; the call throws only when that code path runs, so
 * a dialog or cell renderer fails at runtime while every static check passes.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

/** Names `core/utils.js` exports, from its export lists, destructures and declarations. */
function utilsExports() {
  const source = readFileSync(`${SCRIPTS_ROOT}/core/utils.js`, "utf8");
  const names = new Set();
  for (const block of source.matchAll(/^export (?:const )?\{([^}]*)\}/gm)) {
    for (const entry of block[1].split(",")) {
      const name = entry
        .trim()
        .split(/\s+as\s+/)
        .pop()
        .split(":")[0]
        .trim();
      if (/^\w+$/.test(name)) names.add(name);
    }
  }
  for (const match of source.matchAll(/^export (?:async )?(?:function|const|let|class) (\w+)/gm)) {
    names.add(match[1]);
  }
  return names;
}

test("Utils namespace reads name exports of core/utils.js", async () => {
  const exported = utilsExports();
  const missing = [];
  for (const file of (await walk(SCRIPTS_ROOT)).filter((path) => path.endsWith(".js"))) {
    const source = readFileSync(file, "utf8");
    if (!/^import \* as Utils from "[./]+core\/utils\.js";$/m.test(source)) continue;
    source.split("\n").forEach((line, index) => {
      for (const match of line.matchAll(/(?<![\w.])Utils\.(\w+)/g)) {
        if (!exported.has(match[1]))
          missing.push(`${repoPath(file)}:${index + 1} Utils.${match[1]}`);
      }
    });
  }
  assert.deepEqual(missing, []);
});
