// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: page scripts draw a token's logo and identity only through
 * `ui/token_identity.js`, so every table and list shows one avatar, one letter
 * fallback and one row-cell layout.
 *
 * - No page script builds its own `<img>` for a token logo (a provider logo URL).
 *   The allowlist holds the page surfaces not yet moved to the owner; it only shrinks.
 * - Every DataTable consumer that shows a token renders it with `renderTokenRowCell`
 *   or `renderTokenCell`.
 * - The row cell's hidden actions sit inside the cell, so the token column never
 *   reports a scroll width wider than itself.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const RAW_LOGO_IMG = /<img\b[^>]*\$\{[^}]*\b(?:logoUrl|logo_url|image_url)\b/;
const RAW_LOGO_ALLOWLIST = new Set(["src/webserver/templates/scripts/pages/tools/token_tools.js"]);

const scripts = (await walk(SCRIPTS_ROOT)).filter((file) => file.endsWith(".js"));

test("page scripts take token logos from the identity owner", () => {
  const found = [];
  for (const file of scripts.filter((path) => path.includes("/scripts/pages/"))) {
    if (RAW_LOGO_ALLOWLIST.has(repoPath(file))) continue;
    readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        if (RAW_LOGO_IMG.test(line)) found.push(`${repoPath(file)}:${index + 1}`);
      });
  }
  assert.deepEqual(found, [], "token logo <img> built outside ui/token_identity.js");
});

test("the raw-logo allowlist names only files that still need it", () => {
  for (const path of RAW_LOGO_ALLOWLIST) {
    const file = scripts.find((candidate) => repoPath(candidate) === path);
    assert.ok(file, `${path} no longer exists`);
    assert.match(readFileSync(file, "utf8"), RAW_LOGO_IMG, `${path} no longer builds a raw logo`);
  }
});

/** Body of a top-level `function name(` in `source`, up to the next top-level declaration. */
function functionBody(source, name) {
  const start = new RegExp(`^(?:export )?function ${name}\\(`, "m").exec(source);
  if (!start) return null;
  const rest = source.slice(start.index);
  const next = /\n(?:export )?(?:function|const|class) /.exec(rest.slice(1));
  return next ? rest.slice(0, next.index + 1) : rest;
}

const SHARED_CELL = /\b(?:renderTokenRowCell|renderTokenCell)\(/;

test("DataTable token columns render through the shared token cell", () => {
  const sources = new Map(scripts.map((file) => [file, readFileSync(file, "utf8")]));
  const sharedHelpers = new Set();
  for (const source of sources.values()) {
    for (const match of source.matchAll(/^(?:export )?function (\w+)\(/gm)) {
      if (SHARED_CELL.test(functionBody(source, match[1]) ?? "")) sharedHelpers.add(match[1]);
    }
  }
  const missing = [];
  for (const [file, source] of sources) {
    if (!/\bnew DataTable\(|\bbuildColumns\b/.test(source) || file.endsWith("ui/data_table.js"))
      continue;
    const lines = source.split("\n");
    lines.forEach((line, index) => {
      const head = /^(\s*)id: "(token|token_mint|mint)",$/.exec(line);
      if (!head || !/^\s*label:/.test(lines[index + 1] ?? "")) return;
      const close = `${head[1].slice(2)}}`;
      let end = index + 1;
      while (end < lines.length && !lines[end].startsWith(close)) end += 1;
      const block = lines.slice(index, end).join("\n");
      const calls = [...block.matchAll(/\b(\w+)\(/g)].map((call) => call[1]);
      if (!SHARED_CELL.test(block) && !calls.some((call) => sharedHelpers.has(call)))
        missing.push(`${repoPath(file)}:${index + 1} ${head[2]}`);
    });
  }
  assert.deepEqual(missing, []);
});

test("hidden row-cell actions stay inside the cell's box", () => {
  const css = readFileSync(`${STYLES_ROOT}/ui/token_identity.css`, "utf8").replace(
    /\/\*[\s\S]*?\*\//g,
    ""
  );
  const rest = /(?:^|\})\s*\.ti-row-cell__actions\s*\{([^}]*)\}/.exec(css)?.[1];
  assert.ok(rest, "no .ti-row-cell__actions rule in token_identity.css");
  const transform = /\btransform\s*:\s*([^;]+)/.exec(rest)?.[1] ?? "none";
  assert.match(
    transform.trim(),
    /^(?:none|translateY\([^)]*\))$/,
    "an inline offset at rest widens the cell's scroll width"
  );
});
