// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: catalog text addresses the person using the app, never the developer.
 *
 * Field hints read "SOL per position (0.005-0.01 for testing)", "Configured in code (not
 * editable via UI)" and repeated their default list in brackets. A hint says what the
 * value does for the user; defaults belong in the field itself.
 *
 * - No message in any locale repeats a list of numbers in square brackets.
 * - No English message suggests testing values or refers to the code or "the UI".
 * - German addresses the user formally: no "du" pronoun and no "du" imperative of the
 *   verbs the catalogs use for instructions or for the Assistant's suggested prompts.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";

import { REPO_ROOT } from "../lib/dashboard_ui.mjs";

const LOCALES_ROOT = resolve(REPO_ROOT, "locales");
const BRACKETED_LIST = /\[\s*-?\d[\d.]*(\s*,\s*-?\d[\d.]*)+\s*\]/;
const DEVELOPER_NOTE = /\([^)]*\bfor testing\)|\bin code\b|\bvia (the )?UI\b|\bnot editable\b/i;

async function lines(locale) {
  const result = [];
  const dir = resolve(LOCALES_ROOT, locale);
  for (const file of (await readdir(dir)).filter((name) => name.endsWith(".ftl"))) {
    const text = await readFile(resolve(dir, file), "utf8");
    text.split("\n").forEach((line, index) => {
      if (!line.startsWith("#") && /=/.test(line)) result.push(`${file}:${index + 1} ${line}`);
    });
  }
  return result;
}

const locales = (await readdir(LOCALES_ROOT, { withFileTypes: true }))
  .filter((entry) => entry.isDirectory())
  .map((entry) => entry.name);

test("no message repeats a bracketed list of numbers", async () => {
  const found = [];
  for (const locale of locales) {
    for (const line of await lines(locale)) {
      if (BRACKETED_LIST.test(line)) found.push(`${locale}/${line}`);
    }
  }
  assert.deepEqual(found, []);
});

test("no English message speaks to the developer", async () => {
  const found = (await lines("en")).filter((line) => DEVELOPER_NOTE.test(line.split("=")[1]));
  assert.deepEqual(found, []);
});

// The informal pronouns, and the informal imperatives that instructions and the
// Assistant's suggested prompts once used.
const GERMAN_INFORMAL =
  /\b(?:du|dich|dir|dein(?:e|en|em|er|es)?|sieh|lege|aktiviere|prüfe|markiere|füge|kopiere|bearbeite|installiere|warte|führe|versuche es|erhöhe|analysiere|erkläre)\b/iu;

test("German messages address the user formally", async () => {
  const found = [];
  const dir = resolve(LOCALES_ROOT, "de");
  for (const file of (await readdir(dir)).filter((name) => name.endsWith(".ftl"))) {
    const text = await readFile(resolve(dir, file), "utf8");
    text.split("\n").forEach((line, index) => {
      if (line.trimStart().startsWith("#")) return;
      const value = /^[a-z0-9-]+\s*=/.test(line) ? line.slice(line.indexOf("=") + 1) : line;
      if (GERMAN_INFORMAL.test(value)) found.push(`${file}:${index + 1} ${line.trim()}`);
    });
  }
  assert.deepEqual(found, []);
});
