// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the ScreenerBot logo mark is drawn only through `.brand-mark`, which
 * masks the single-colour logo and fills it with the theme's `--brand-mark`.
 *
 * The logo file is one solid colour. Drawn as an `<img>` it keeps that colour in
 * every theme and disappears against the matching surface, and the workaround
 * (a dark tile or a glow behind it) puts a background behind the mark.
 *
 * - No template or script draws the logo file as an `<img>`.
 * - Both theme palettes define `--brand-mark`, and `.brand-mark` paints it.
 * - A class that sizes a mark paints nothing behind it: no background, border,
 *   padding, shadow or glow.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  STYLES_ROOT,
  TEMPLATES_ROOT,
  findOpenTags,
  classTokens,
  repoPath,
  rulesIn,
  subjectCompound,
  walk,
} from "../lib/dashboard_ui.mjs";

const markup = (await walk(TEMPLATES_ROOT))
  .filter((path) => /\.(?:html|js)$/.test(path))
  .map((path) => ({ path, source: readFileSync(path, "utf8") }));
const stylesheets = (await walk(STYLES_ROOT))
  .filter((path) => path.endsWith(".css"))
  .map((path) => ({ path, rules: rulesIn(readFileSync(path, "utf8")) }));

test("no surface draws the logo file as an image", () => {
  const found = [];
  for (const { path, source } of markup) {
    for (const tag of findOpenTags(source, "img")) {
      if (/logo\.svg/.test(tag.tag)) found.push(repoPath(path));
    }
  }
  assert.deepEqual(found, []);
});

test("the mark takes its colour from the theme", () => {
  const foundation = readFileSync(`${STYLES_ROOT}/foundation.css`, "utf8");
  const palettes = rulesIn(foundation).filter(({ body }) => /--bg-primary\s*:/.test(body));
  assert.ok(palettes.length >= 2, "foundation.css defines both theme palettes");
  for (const { selector, body } of palettes)
    assert.match(body, /--brand-mark\s*:/, `${selector} defines no --brand-mark`);
  const mark = rulesIn(foundation).find(({ selector }) => selector === ".brand-mark");
  assert.ok(mark, "no .brand-mark rule in foundation.css");
  assert.match(mark.body, /background-color\s*:\s*var\(--brand-mark\)/);
  assert.match(mark.body, /mask\s*:\s*url\("\/assets\/logo\.svg"\)/);
});

test("nothing is painted behind a logo mark", () => {
  const sizing = new Set();
  for (const { source } of markup) {
    for (const tag of findOpenTags(source, "[a-z]+")) {
      const { statics } = classTokens(tag.tag);
      if (statics.includes("brand-mark"))
        statics.filter((name) => name !== "brand-mark").forEach((name) => sizing.add(name));
    }
  }
  assert.ok(sizing.size >= 5, `only ${sizing.size} logo surfaces found`);
  const found = [];
  for (const { path, rules } of stylesheets) {
    for (const { selector, body } of rules) {
      const subject = subjectCompound(selector);
      if (![...sizing].some((name) => new RegExp(`\\.${name}(?![\\w-])`).test(subject))) continue;
      if (
        /(?:^|;|\s)(?:background(?:-color|-image)?|border(?!-radius)[\w-]*|padding[\w-]*|box-shadow|text-shadow)\s*:(?!\s*none)/.test(
          body
        ) ||
        /drop-shadow/.test(body)
      )
        found.push(`${repoPath(path)}: ${selector}`);
    }
  }
  assert.deepEqual(found, []);
});
