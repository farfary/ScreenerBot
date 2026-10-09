// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every English field label is in Title Case.
 *
 * One dialog said "Require Filter Pass" and the next "Wallet address", so the
 * dashboard read as two products. A field label capitalises every word except the
 * short function words (articles, conjunctions, short prepositions) after the first;
 * each part of a hyphenated word counts as a word ("Per-Trade Cap").
 *
 * A field label is the text naming a form control:
 * - a `<label>` element's message, on the label or on its first span;
 * - a Copy Trading field (`copy-field` labels, `numberInput` labels, rule fields);
 * - a config field's catalog message (one that carries a hint, unit or placeholder).
 *
 * Title Case is an English convention; the other catalogs follow their own language.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";

import { REPO_ROOT, loadMarkupSources } from "../lib/dashboard_ui.mjs";

const SMALL_WORDS = new Set(
  "a an the and or nor but of per to in on for at by with from vs via as into".split(" ")
);
const ENGLISH = resolve(REPO_ROOT, "locales/en");

/** English messages, with each message's attribute names. */
async function english() {
  const messages = new Map();
  for (const file of (await readdir(ENGLISH)).filter((name) => name.endsWith(".ftl"))) {
    let id = null;
    for (const line of (await readFile(resolve(ENGLISH, file), "utf8")).split("\n")) {
      const message = /^([a-z][a-z0-9-]*) = (.*)$/.exec(line);
      if (message) {
        [, id] = message;
        messages.set(id, { file, value: message[2], attributes: new Set() });
        continue;
      }
      const attribute = /^\s+\.([a-z-]+) = /.exec(line);
      if (attribute && id) messages.get(id).attributes.add(attribute[1]);
    }
  }
  return messages;
}

/** Message ids the dashboard renders as the label of a form control. */
async function fieldLabelIds(messages) {
  const ids = new Set();
  const add = (pattern, source, group = 1) => {
    for (const match of source.matchAll(pattern)) ids.add(match[group]);
  };
  for (const { path, source } of await loadMarkupSources()) {
    add(/<label\b[^>]*\bdata-l10n-id="([a-z0-9-]+)"/g, source);
    add(
      /<label\b[^>]*>\s*(?:<i\b[^>]*><\/i>\s*)?<span\b[^>]*\bdata-l10n-id="([a-z0-9-]+)"/g,
      source
    );
    add(/<label\b[^>]*>\s*<span>\$\{esc\(I18n\.t\("([a-z0-9-]+)"/g, source);
    add(/numberInput\(esc, \{[^}]*?\blabel:[^,}]*?I18n\.t\("([a-z0-9-]+)"/g, source);
    add(
      /numberInput\(esc, \{[^}]*?\blabel:[^,}]*?I18n\.t\("[a-z0-9-]+"\)\s*:\s*I18n\.t\("([a-z0-9-]+)"/g,
      source
    );
    if (path.endsWith("pages/copy/policy.js")) {
      add(/get label\(\)\s*\{\s*return I18n\.t\("([a-z0-9-]+)"/g, source);
    }
  }
  for (const [id, { file, attributes }] of messages) {
    if (file !== "config.ftl") continue;
    if (["hint", "unit", "placeholder"].some((name) => attributes.has(name))) ids.add(id);
  }
  return [...ids].filter((id) => messages.has(id));
}

/** Whether a label capitalises every word but the short function words after the first. */
export function isTitleCase(label) {
  const words = label
    .replace(/\{[^}]*\}/g, "X")
    .split(/[\s()—:/]+/)
    .filter(Boolean);
  return words.every((word, index) =>
    word.split("-").every((part, partIndex) => {
      if (!/^\p{Ll}/u.test(part)) return true;
      return (index > 0 || partIndex > 0) && SMALL_WORDS.has(part);
    })
  );
}

test("the Title Case check reads labels as intended", () => {
  for (const label of [
    "Require Filter Pass",
    "Per-Trade Cap",
    "Share of Each Trade",
    "Min TX (1h)",
    "Trigger Amount",
    "Total { -sol } Limit",
    "Sells at a Loss of",
  ]) {
    assert.ok(isTitleCase(label), label);
  }
  for (const label of ["Wallet address", "Per-trade cap", "Auto-expand Categories", "of Wallets"]) {
    assert.ok(!isTitleCase(label), label);
  }
});

test("every English field label is in Title Case", async () => {
  const messages = await english();
  const ids = await fieldLabelIds(messages);
  assert.ok(ids.length > 400, `the field label inventory was found (${ids.length})`);
  const offenders = ids
    .map((id) => ({ id, ...messages.get(id) }))
    .filter(({ value }) => !isTitleCase(value))
    .map(({ file, id, value }) => `${file} ${id} = ${value}`);
  assert.deepEqual(offenders, []);
});
