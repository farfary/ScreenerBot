// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a hint's first line reads as a whole sentence in every locale.
 *
 * The Hints settings list previews each hint by the first line of its content, and
 * several hints opened with a lead-in to a list ("Tokens shown here have:"), so their
 * preview stopped at a colon. The first line of every hint's content ends a sentence,
 * never at a colon, in every shipped locale.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";

import { REPO_ROOT } from "../lib/dashboard_ui.mjs";

/** First non-empty line of every `hints-*-content` message in a Fluent file. */
function contentLeads(source) {
  const leads = new Map();
  let current = null;
  for (const line of source.split("\n")) {
    const message = /^([a-z][\w-]*) =(.*)$/.exec(line);
    if (message) {
      current = message[1].endsWith("-content") ? message[1] : null;
      if (current && message[2].trim()) leads.set(current, message[2].trim());
      continue;
    }
    if (!current || leads.has(current)) continue;
    if (/^\s/.test(line) && line.trim()) leads.set(current, line.trim());
    else if (line.trim()) current = null;
  }
  return leads;
}

test("every hint's first line ends a sentence, not at a colon", () => {
  const locales = readdirSync(`${REPO_ROOT}/locales`, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name);
  assert.ok(locales.length >= 16);
  const found = [];
  for (const locale of locales) {
    const leads = contentLeads(readFileSync(`${REPO_ROOT}/locales/${locale}/hints.ftl`, "utf8"));
    assert.ok(leads.size > 0, `${locale}: hints found`);
    for (const [id, lead] of leads) {
      if (/[:：]$/.test(lead)) found.push(`${locale} ${id}: ${lead}`);
    }
  }
  assert.deepEqual(found, []);
});
