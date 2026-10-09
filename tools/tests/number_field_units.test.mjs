// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: every number-field unit is written in its short form, in every locale.
 *
 * A unit sits inside its field after the value (`ui/number_field.js`), so its text has
 * to be short: "%", "SOL", "s", "min", "h" or a short count label. A word-length unit
 * ("seconds", "closed rounds") leaves the value no room in a compact field.
 *
 * - Every config field's `.unit` attribute, and every message a page writes as a field
 *   unit, is at most `MAX_COLUMNS` display columns in every locale.
 * - A config field label never carries a unit in parentheses ("Timeout (seconds)").
 * - `unitColumns` counts an East Asian wide character as two columns.
 *
 * The rendered placement is checked on every page by `dashboard_stability.test.mjs`.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";

import { REPO_ROOT, loadMarkupSources } from "../lib/dashboard_ui.mjs";
import { unitColumns } from "../../src/webserver/templates/scripts/ui/number_field.js";

const MAX_COLUMNS = 8;
const LOCALES_ROOT = resolve(REPO_ROOT, "locales");

/** Message id -> value per locale, with each attribute as `id.attribute`. */
async function catalogs() {
  const result = new Map();
  const locales = (await readdir(LOCALES_ROOT, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name);
  for (const locale of locales) {
    const entries = new Map();
    for (const file of (await readdir(resolve(LOCALES_ROOT, locale))).filter((name) =>
      name.endsWith(".ftl")
    )) {
      let id = null;
      for (const line of (await readFile(resolve(LOCALES_ROOT, locale, file), "utf8")).split(
        "\n"
      )) {
        const message = /^([a-z][a-z0-9-]*) = (.*)$/.exec(line);
        if (message) {
          [, id] = message;
          entries.set(id, message[2]);
          continue;
        }
        const attribute = /^\s+\.([a-z-]+) = (.*)$/.exec(line);
        if (attribute && id) entries.set(`${id}.${attribute[1]}`, attribute[2]);
      }
    }
    result.set(locale, entries);
  }
  return result;
}

/** Ids a page writes as a field unit: `.input-unit` spans and unit getters. */
async function fieldUnitIds() {
  const ids = new Set();
  for (const { source } of await loadMarkupSources()) {
    for (const match of source.matchAll(/class="input-unit"\s+data-l10n-id="([a-z0-9-]+)"/g)) {
      ids.add(match[1]);
    }
    for (const match of source.matchAll(/["']([a-z]+(?:-[a-z]+)*-unit-[a-z]+(?:-[a-z]+)*)["']/g)) {
      ids.add(match[1]);
    }
  }
  return ids;
}

const [all, unitIds] = await Promise.all([catalogs(), fieldUnitIds()]);
const english = all.get("en");

test("every field unit is short in every locale", () => {
  const units = [...english.keys()].filter((key) => key.endsWith(".unit"));
  const messages = [...unitIds].filter((id) => english.has(id));
  assert.ok(units.length > 150 && messages.length >= 10, "the unit inventory was found");
  const long = [];
  for (const [locale, entries] of all) {
    for (const key of [...units, ...messages]) {
      const value = entries.get(key)?.replace(/\{ -sol \}/g, "SOL");
      if (value !== undefined && unitColumns(value) > MAX_COLUMNS) {
        long.push(`${locale} ${key}: ${value}`);
      }
    }
  }
  assert.deepEqual(long, []);
});

test("a config field label never carries a unit in parentheses", () => {
  const repeated = [];
  for (const [locale, entries] of all) {
    const units = new Set(
      [...entries]
        .filter(([key]) => key.endsWith(".unit"))
        .map(([, value]) => value.trim())
        .concat(["%"])
    );
    for (const [key, label] of entries) {
      if (!key.startsWith("config-") || key.includes(".")) continue;
      const note = /[（(]([^()（）]*)[)）]\s*$/.exec(label)?.[1].trim();
      // A parenthesised window ("Min TX (1h)") qualifies the label; a unit belongs in the field.
      if (note && (units.has(note) || note.includes("-sol"))) {
        repeated.push(`${locale} ${key}: ${label}`);
      }
    }
  }
  assert.deepEqual(repeated, []);
});

test("an East Asian wide character takes two columns", () => {
  assert.equal(unitColumns("s"), 1);
  assert.equal(unitColumns("min"), 3);
  assert.equal(unitColumns("秒"), 2);
  assert.equal(unitColumns("時間"), 4);
  assert.equal(unitColumns("시간"), 4);
  assert.equal(unitColumns(" µlam/CU "), 7);
});
