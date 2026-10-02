// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: `core/format.js` is the only place that attaches a unit or symbol to a
 * number in dashboard scripts. A hand-built "<n> SOL", "$<n>", "≈ <n>" or "<n>%"
 * fixes the unit position, spacing and symbol for every locale.
 *
 * A line that must keep such text (a ticker, a CSS length, a machine value) carries
 * `// format-ok: <reason>`; the reason is required.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const SCRIPTS = new URL("../../src/webserver/templates/scripts/", import.meta.url);
const OWNER = "core/format.js";

const ESCAPE = /\/\/\s*format-ok\b(.*)$/;

// CSS lengths and geometry: a `%` there is a style value, not a displayed number.
const CSS_CONTEXT =
  /\bstyle\b|\.style\.|setProperty|--[a-z]|(?:width|height|top|left|right|bottom|inset[\w-]*|translate\w*|flex[\w-]*|background[\w-]*|margin[\w-]*|padding[\w-]*|font-size|opacity|calc)\s*:|\b(?:width|height|top|left)\s*=/;

/** Rules a line of code is checked against. Each returns true when the line builds a unit by hand. */
export const RULES = [
  {
    name: "SOL unit beside an interpolation or concatenation",
    test: (line) => /\}\s?SOL\b|\bSOL\s?\$\{|["'`]\s+SOL["'`]|\+\s*["'`]\s*SOL\b/.test(line),
  },
  { name: "approximation sign", test: (line) => line.includes("≈") },
  {
    name: "dollar symbol beside an interpolation or concatenation",
    test: (line) => /\$\$\{|(["'`])\$\1/.test(line),
  },
  {
    name: "percent sign after a number expression",
    test: (line) =>
      !CSS_CONTEXT.test(line) && /(?<![\d,])\}%|\+\s*["'`]%["'`]|["'`]%["'`]\s*\+/.test(line),
  },
];

/** Findings for one source text: `[{ line, rule }]`, `format-ok` lines and comments skipped. */
export function scan(source) {
  const found = [];
  const lines = source.split("\n");
  let inBlock = false;
  lines.forEach((text, index) => {
    const trimmed = text.trim();
    if (inBlock) {
      if (trimmed.includes("*/")) inBlock = false;
      return;
    }
    if (trimmed.startsWith("/*")) {
      if (!trimmed.includes("*/")) inBlock = true;
      return;
    }
    if (trimmed.startsWith("//") || trimmed.startsWith("*")) return;
    const escape = ESCAPE.exec(text);
    if (escape) {
      if (!/^\s*:\s*\S/.test(escape[1])) {
        found.push({ line: index + 1, rule: "format-ok needs a reason (`// format-ok: <reason>`)" });
      }
      return;
    }
    for (const rule of RULES) {
      if (rule.test(text)) found.push({ line: index + 1, rule: rule.name });
    }
  });
  return found;
}

function walk(dir, out = []) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (entry.name.endsWith(".js")) out.push(full);
  }
  return out;
}

test("no dashboard script builds a unit or symbol by hand outside core/format.js", () => {
  const root = new URL(".", SCRIPTS).pathname;
  const failures = [];
  for (const file of walk(root)) {
    const rel = path.relative(root, file);
    if (rel === OWNER) continue;
    for (const { line, rule } of scan(fs.readFileSync(file, "utf8"))) {
      failures.push(`${rel}:${line}: ${rule}`);
    }
  }
  assert.equal(
    failures.length,
    0,
    `Route units through core/format.js (formatSol, withSolUnit, formatCurrencyUSD, withUsdSymbol, withApprox, formatPercentValue, withPercentUnit):\n${failures.join("\n")}`
  );
});

test("the scanner flags hand-built units and spares CSS, ids and regexes", () => {
  const flagged = [
    "const a = `${x} SOL`;",
    'const b = value + " SOL";',
    'const c = "≈ " + text;',
    "const d = `$${amount}`;",
    'const e = "$" + amount;',
    "const f = `${pct.toFixed(1)}%`;",
    'const g = n.toFixed(1) + "%";',
    'formatCompactNumber(v, { prefix: "$" });',
  ];
  for (const line of flagged) assert.ok(scan(line).length > 0, line);
  const spared = [
    "bar.style.width = `${pct}%`;",
    "el.style.setProperty('--fill', `${pct}%`);",
    '<div style="width: ${pct}%"></div>',
    "const re = /^\\d{2}%$/;",
    'I18n.t("format-sol-amount", { amount });',
    "// note: `${x} SOL` is built by the owner",
    "const price = `${x}`; // format-ok: machine value",
  ];
  for (const line of spared) assert.equal(scan(line).length, 0, line);
  assert.equal(scan("const x = `${a} SOL`; // format-ok").length, 1);
  assert.equal(scan("const x = `${a} SOL`; // format-ok:  ").length, 1);
});
