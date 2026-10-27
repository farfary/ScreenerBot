// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: page scripts draw a token's logo and identity only through
 * `ui/token_identity.js`, so every table and list shows one avatar, one letter
 * fallback and one row-cell layout.
 *
 * - No page or shared UI script builds its own `<img>` for a token logo (a provider
 *   logo URL); the token details, position details and token analyzer headers once
 *   drew their own avatar with their own letter fallback. The allowlist holds the
 *   surfaces not yet moved to the owner; it only shrinks.
 * - Every DataTable consumer that shows a token renders it with `renderTokenRowCell`
 *   or `renderTokenCell`, directly or through a helper that does: a top-level function,
 *   or an arrow helper declared in the same file.
 * - The row cell's hidden actions sit inside the cell, so the token column never
 *   reports a scroll width wider than itself.
 * - The row cell's action buttons are the shared bare icon button (`.btn-icon`), not a
 *   `.btn` reskinned inside the cell.
 * - A token without a symbol is labelled by its short mint (`tokenLabel`), never by a
 *   question-mark placeholder.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, STYLES_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const RAW_LOGO_IMG = /<img\b[^>]*\$\{[^}]*\b(?:logoUrl|logo_url|image_url)\b/;
const RAW_LOGO_ALLOWLIST = new Set();
const IDENTITY_OWNER = "src/webserver/templates/scripts/ui/token_identity.js";

const scripts = (await walk(SCRIPTS_ROOT)).filter((file) => file.endsWith(".js"));

test("page and UI scripts take token logos from the identity owner", () => {
  const found = [];
  for (const file of scripts.filter((path) => /\/scripts\/(?:pages|ui)\//.test(path))) {
    if (RAW_LOGO_ALLOWLIST.has(repoPath(file)) || repoPath(file) === IDENTITY_OWNER) continue;
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

/**
 * Helpers local to one source, `const name = (...) =>` at any depth: name to body, the
 * declaration line and every following line indented deeper than it.
 */
function localArrowHelpers(source) {
  const lines = source.split("\n");
  const helpers = new Map();
  lines.forEach((line, index) => {
    const head = /^(\s*)const (\w+) = (?:\([^)]*\)|\w+) =>/.exec(line);
    if (!head) return;
    let end = index + 1;
    while (
      end < lines.length &&
      (lines[end].trim() === "" || /^\s*/.exec(lines[end])[0].length > head[1].length)
    )
      end += 1;
    helpers.set(head[2], lines.slice(index, end).join("\n"));
  });
  return helpers;
}

/**
 * Names of the local helpers in `source` that reach the shared cell, directly or through
 * another helper that does (`sharedHelpers` or a local one), resolved to a fixed point.
 */
function localSharedHelpers(source, sharedHelpers) {
  const helpers = localArrowHelpers(source);
  const reaching = new Set();
  let grew = true;
  while (grew) {
    grew = false;
    for (const [name, body] of helpers) {
      if (reaching.has(name)) continue;
      const calls = [...body.matchAll(/\b(\w+)\(/g)].map((call) => call[1]);
      const reaches =
        SHARED_CELL.test(body) ||
        calls.some((call) => call !== name && (sharedHelpers.has(call) || reaching.has(call)));
      if (reaches) {
        reaching.add(name);
        grew = true;
      }
    }
  }
  return reaching;
}

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
    const localHelpers = localSharedHelpers(source, sharedHelpers);
    const lines = source.split("\n");
    lines.forEach((line, index) => {
      const head = /^(\s*)id: "(token|token_mint|mint)",$/.exec(line);
      if (!head || !/^\s*label:/.test(lines[index + 1] ?? "")) return;
      const close = `${head[1].slice(2)}}`;
      let end = index + 1;
      while (end < lines.length && !lines[end].startsWith(close)) end += 1;
      const block = lines.slice(index, end).join("\n");
      const calls = [...block.matchAll(/\b(\w+)\(/g)].map((call) => call[1]);
      const viaHelper = calls.some((call) => sharedHelpers.has(call) || localHelpers.has(call));
      if (!SHARED_CELL.test(block) && !viaHelper)
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

test("row-cell actions are the shared icon button", () => {
  const found = [];
  for (const file of scripts) {
    readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        const classes = /class="([^"]*\brow-action\b[^"]*)"/.exec(line)?.[1];
        if (classes && !/\bbtn-icon\b/.test(classes)) found.push(`${repoPath(file)}:${index + 1}`);
      });
  }
  assert.deepEqual(found, []);
  const css = readFileSync(`${STYLES_ROOT}/ui/token_identity.css`, "utf8");
  assert.doesNotMatch(
    css,
    /\.ti-row-cell__actions\s+\.btn\b(?!-)/,
    "a reskinned .btn in the row cell"
  );
});

test("a token without a symbol is never labelled with a placeholder", () => {
  const found = [];
  for (const file of scripts) {
    readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        const code = /^\s*(?:\*|\/\/)/.test(line) ? "" : line;
        if (/["'`]\?{2,}["'`]/.test(code)) found.push(`${repoPath(file)}:${index + 1}`);
      });
  }
  assert.deepEqual(found, [], "use tokenLabel(symbol, mint) from ui/token_identity.js");
});
