// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: a selector with a `:has()` ancestor names the element it styles.
 *
 * Chromium cannot scope the invalidation of a rule such as
 * `.section:has(+ .section) .grid > :nth-child(3n + 1) ~ *`: its subject is any element,
 * so every text or child change anywhere in the document restyles the whole document.
 * Two such rules in the Token Details sheets, with the dialog closed, made each live
 * table cell update cost about 8 ms of style recalculation instead of 1 ms. Naming the
 * subject (`> .position-fact:nth-child(…)`) keeps the rule's meaning and its cost local.
 *
 * A subject is named when its compound carries a type, class, id or attribute, or when
 * every argument of an `:is()`/`:where()` in it is named. `:not()`, `:nth-*()` and other
 * pseudo-classes never name it: `:not(.x)` matches almost every element.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { loadStylesheets, splitSelectorList, subjectCompound } from "../lib/dashboard_ui.mjs";

/** Does `compound` name the elements it can match, so its invalidation stays local? */
function namesSubject(compound) {
  let index = 0;
  let named = false;
  if (/^[a-zA-Z]/.test(compound)) named = true;
  while (index < compound.length) {
    const char = compound[index];
    if (char === "." || char === "#" || char === "[") named = true;
    if (char === ":") {
      const match = /^::?([\w-]+)/.exec(compound.slice(index));
      const name = match ? match[1] : "";
      index += match ? match[0].length : 1;
      if (compound[index] === "(") {
        const end = closingParen(compound, index);
        const argument = compound.slice(index + 1, end);
        if (
          (name === "is" || name === "where") &&
          splitSelectorList(argument).every((part) => namesSubject(subjectCompound(part)))
        ) {
          named = true;
        }
        index = end + 1;
      }
      continue;
    }
    if (char === "[") index = closingBracket(compound, index);
    index += 1;
  }
  return named;
}

function closingParen(source, open) {
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === "(") depth += 1;
    else if (source[index] === ")") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return source.length - 1;
}

function closingBracket(source, open) {
  const end = source.indexOf("]", open);
  return end < 0 ? source.length - 1 : end;
}

test("namesSubject tells a named subject from a bare one", () => {
  assert.equal(namesSubject(".position-fact:nth-child(3n + 1)"), true);
  assert.equal(namesSubject("td:last-child"), true);
  assert.equal(namesSubject('[data-kind="x"]'), true);
  assert.equal(namesSubject(':where(input[type="checkbox"], input[type="radio"])'), true);
  assert.equal(namesSubject(":nth-child(3n + 1):nth-last-child(-n + 3)"), false);
  assert.equal(namesSubject("*"), false);
  assert.equal(namesSubject(":not(.wide)"), false);
  assert.equal(namesSubject(":is(.a, :last-child)"), false);
});

test("every selector with a :has() ancestor names its subject", async () => {
  const offenders = [];
  for (const sheet of await loadStylesheets()) {
    for (const rule of sheet.rules) {
      const selector = rule.selector.replace(/\s+/g, " ");
      const subject = subjectCompound(selector);
      const ancestors = selector.slice(0, selector.length - subject.length);
      if (!ancestors.includes(":has(")) continue;
      if (!namesSubject(subject)) offenders.push(`${sheet.path}:${rule.line} ${selector}`);
    }
  }
  assert.deepEqual(offenders, [], "name the subject: `> .item:nth-child(…)`, `~ .item`");
});
