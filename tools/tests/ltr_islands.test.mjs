/**
 * Guard: shared helpers that emit an address, mint or signature as an element mark it
 * `dir="ltr"`, so the value keeps its left-to-right order and isolates from
 * surrounding right-to-left text.
 *
 * `token_identity.js` and `core/utils.js` depend on browser globals (`I18n`, `window`)
 * and cannot be imported in node, so the assertions read the helper source text.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";

const SCRIPTS = new URL("../../src/webserver/templates/scripts/", import.meta.url);
const read = (rel) => fs.readFileSync(new URL(rel, SCRIPTS), "utf8");

/** Source of the function that starts at `signature`, up to its closing brace at the same indent. */
function functionSource(source, signature) {
  const start = source.indexOf(signature);
  assert.notEqual(start, -1, `${signature} not found`);
  const lineStart = source.lastIndexOf("\n", start) + 1;
  const indent = /^\s*/.exec(source.slice(lineStart))[0];
  const end = source.indexOf(`\n${indent}}\n`, start);
  assert.notEqual(end, -1, `end of ${signature} not found`);
  return source.slice(start, end);
}

/** The opening tag of the element that renders `content` (a template expression). */
function openingTagBefore(body, content) {
  const at = body.indexOf(content);
  assert.notEqual(at, -1, `${content} not found`);
  return body.slice(body.lastIndexOf("<", at), at);
}

test("renderAddress marks the address link dir=ltr", () => {
  const body = functionSource(read("ui/token_identity.js"), "export function renderAddress(");
  assert.match(openingTagBefore(body, "${safe}</a>"), /class="ti-address-value"[^>]*dir="ltr"/);
});

test("renderAddressChip marks the address link dir=ltr", () => {
  const body = functionSource(read("core/utils.js"), "function renderAddressChip(");
  assert.match(openingTagBefore(body, "${display}</a>"), /class="addr-chip-link mono"[^>]*dir="ltr"/);
});
