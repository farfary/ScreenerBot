// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: in the frameless macOS window the dashboard header is the title bar and
 * drags the window, its controls and scrolling rows stay out of the drag region,
 * and open overlays mounted after the header cut its drag area. Electron builds the
 * drag shape in document order and ignores z-index, so a separate drag strip placed
 * before the header is erased by it.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { STYLES_ROOT, TEMPLATES_ROOT } from "../lib/dashboard_ui.mjs";

const HEADER_STYLES = readFileSync(`${STYLES_ROOT}/header.css`, "utf8");
const BASE_HTML = readFileSync(`${TEMPLATES_ROOT}/base.html`, "utf8");

function ruleBody(selectorPattern) {
  return HEADER_STYLES.match(new RegExp(`${selectorPattern}\\s*\\{([^}]+)\\}`))?.[1];
}

test("the macOS header drags the window", () => {
  const rule = ruleBody("\\.electron-macos \\.modern-header");
  assert.ok(rule, ".electron-macos .modern-header has a rule");
  assert.match(rule, /-webkit-app-region:\s*drag\s*;/);
  assert.match(rule, /(^|[^-])app-region:\s*drag\s*;/);
});

test("header controls and scrolling rows stay out of the drag region", () => {
  const match = HEADER_STYLES.match(
    /\.electron-macos \.modern-header :is\(([\s\S]*?)\)\s*\{([^}]+)\}/
  );
  assert.ok(match, "the header has one interactive no-drag rule");
  const selectors = match[1].split(",").map((selector) => selector.trim());
  for (const selector of [
    "button",
    "a",
    "input",
    "select",
    "textarea",
    '[role="button"]',
    '[role="tab"]',
    "[tabindex]",
    "[title]",
    ".tab-scroll-area",
  ]) {
    assert.ok(selectors.includes(selector), `${selector} stays outside the drag region`);
  }
  assert.match(match[2], /-webkit-app-region:\s*no-drag\s*;/);
});

test("open overlays after the header cut its drag area; closed ones do not", () => {
  const rule = ruleBody(
    '\\.electron-macos \\.modern-header ~ :not\\(\\[aria-hidden="true"\\], \\[hidden\\], \\[inert\\]\\)'
  );
  assert.ok(rule, "later siblings of the header that are not hidden are no-drag");
  assert.match(rule, /-webkit-app-region:\s*no-drag\s*;/);
});

test("no separate drag strip precedes the header", () => {
  assert.doesNotMatch(BASE_HTML, /electron-titlebar-bg/);
  assert.doesNotMatch(HEADER_STYLES, /electron-titlebar-bg/);
});
