// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: an address or mint is shown in full on one line everywhere in the
 * dashboard. `renderAddress` (ui/token_identity.js) sizes the value to its line;
 * DataTable column minimums are computed from the same geometry, so the stylesheet
 * and the script must agree on it, and no script may crop an address to
 * "abcd…wxyz" instead. A transaction signature is the one exception: it renders
 * compact through `renderSignature`, whose crop is `formatSignatureCompact`
 * (core/format.js).
 *
 * The backend is held to the same rule: no recorded API response (the dashboard
 * fixtures) carries an address already cropped to "head…tail", because a value
 * cropped before it reaches the dashboard bypasses `renderAddress` entirely.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const TEMPLATES = new URL("../../src/webserver/templates/", import.meta.url);
const read = (rel) => fs.readFileSync(new URL(rel, TEMPLATES), "utf8");

/** The number a JS `export const NAME = <number>;` declares. */
function exportedNumber(source, name) {
  const match = new RegExp(`export const ${name} = ([0-9.]+);`).exec(source);
  assert.ok(match, `${name} not exported`);
  return Number(match[1]);
}

/** The body of the first CSS rule whose selector is exactly `selector`. */
function ruleBody(css, selector) {
  const at = css.indexOf(`\n${selector} {`);
  assert.notEqual(at, -1, `${selector} rule not found`);
  return css.slice(at, css.indexOf("\n}", at));
}

function scriptFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) return scriptFiles(full);
    return entry.name.endsWith(".js") ? [full] : [];
  });
}

test("the address geometry in token_identity.css matches the column-width constants", () => {
  const js = read("scripts/ui/token_identity.js");
  const address = ruleBody(read("styles/ui/token_identity.css"), ".ti-address");
  const value = ruleBody(read("styles/ui/token_identity.css"), ".ti-address-value");

  const advance = exportedNumber(js, "ADDRESS_GLYPH_ADVANCE");
  const floor = exportedNumber(js, "ADDRESS_FLOOR_PX");
  const actions = exportedNumber(js, "ADDRESS_ACTIONS_PX");

  assert.match(address, new RegExp(`--ti-address-actions: ${actions}px;`));
  assert.match(
    address,
    new RegExp(`--ti-address-glyphs: calc\\(var\\(--ti-address-chars, 44\\) \\* ${advance}\\);`)
  );
  assert.match(
    address,
    new RegExp(`var\\(--ti-address-glyphs\\) \\* ${floor}px \\+ var\\(--ti-address-actions\\)`)
  );
  assert.match(value, new RegExp(`clamp\\(\\s*${floor}px,`));
  assert.match(value, /white-space: nowrap;/);
  assert.match(value, /text-transform: none;/);
  assert.match(value, /letter-spacing: 0;/);
  assert.doesNotMatch(value, /text-overflow|overflow-wrap|word-break/);
});

test("no dashboard script crops an address or mint", () => {
  const root = new URL("scripts/", TEMPLATES).pathname;
  const cropped = [
    // `${value.slice(0, 6)}...${value.slice(-4)}` and its `…` form.
    /\.(?:slice|substring|substr)\(0,\s*\d+\)\)?\}?\s*(?:\.\.\.|…)/,
    // A head-of-value cut on anything named like an address.
    /(?:mint|address|pubkey|wallet)\w*\??\.(?:slice|substring|substr)\(0,\s*\d+\)/i,
    /\bformatAddressCompact\b/,
  ];
  // Free-text previews (event messages, payload summaries, config values) that are
  // not addresses.
  const allowed = [
    ["pages/events.js", "text.slice(0, 160)"],
    ["pages/events.js", "val.slice(0, 32)"],
    ["ui/events_dialog.js", "message.slice(0, 140)"],
    ["ui/config_import_export_dialog.js", "value.substring(0, 40)"],
  ];
  const isAllowed = (rel, line) =>
    allowed.some(([file, snippet]) => rel === file && line.includes(snippet));

  const offenders = [];
  for (const file of scriptFiles(root)) {
    const rel = path.relative(root, file);
    fs.readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        const where = `${rel}:${index + 1}`;
        if (cropped.some((pattern) => pattern.test(line)) && !isAllowed(rel, line)) {
          offenders.push(`${where}: ${line.trim()}`);
        }
      });
  }
  assert.deepEqual(offenders, [], "render the value with renderAddress (ui/token_identity.js)");
});

test("no API response carries a cropped address", () => {
  const fixtures = new URL("./fixtures/dashboard/", import.meta.url).pathname;
  const base58 = "[1-9A-HJ-NP-Za-km-z]";
  const cropped = new RegExp(`^${base58}{3,8}(?:…|\\.\\.\\.)${base58}{3,8}$`);
  const offenders = [];
  const visit = (value, where) => {
    if (typeof value === "string") {
      if (cropped.test(value)) offenders.push(`${where}: ${value}`);
    } else if (value && typeof value === "object") {
      for (const [key, child] of Object.entries(value)) visit(child, `${where}.${key}`);
    }
  };
  const jsonFiles = (dir) =>
    fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) return jsonFiles(full);
      return entry.name.endsWith(".json") ? [full] : [];
    });
  for (const file of jsonFiles(fixtures)) {
    visit(JSON.parse(fs.readFileSync(file, "utf8")), path.relative(fixtures, file));
  }
  assert.deepEqual(
    offenders,
    [],
    "send the full address; the dashboard sizes it with renderAddress"
  );
});
