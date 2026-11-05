// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: in the frameless macOS window the dashboard header is the title bar and
 * drags the window, its controls and scrolling rows stay out of the drag region,
 * and open overlays mounted after the header cut its drag area. Electron builds the
 * drag shape in document order and ignores z-index, so a separate drag strip placed
 * before the header is erased by it. The page area under the header carries no
 * app-region, because no-drag is inherited down to content scrolled out of view.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  STYLES_ROOT,
  TEMPLATES_ROOT,
  classesIn,
  loadStylesheets,
  subjectCompound,
} from "../lib/dashboard_ui.mjs";

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
  // Whitespace-insensitive: the formatter breaks a long selector across lines.
  const match = HEADER_STYLES.match(
    /\.electron-macos\s+\.modern-header\s+:is\(([\s\S]*?)\)\s*\{([^}]+)\}/
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

/** The `:not(...)` list and body of the rule that makes later siblings of the header no-drag. */
function overlayRule() {
  const match = HEADER_STYLES.match(
    /\.electron-macos\s+\.modern-header\s*~\s*:not\(([^)]*)\)\s*\{([^}]+)\}/
  );
  assert.ok(match, "later siblings of the header have one no-drag rule");
  return { excluded: match[1].split(",").map((selector) => selector.trim()), body: match[2] };
}

test("open overlays after the header cut its drag area; closed ones do not", () => {
  const { excluded, body } = overlayRule();
  for (const closed of ['[aria-hidden="true"]', "[hidden]", "[inert]"]) {
    assert.ok(excluded.includes(closed), `${closed} overlays do not cut the drag area`);
  }
  assert.match(body, /-webkit-app-region:\s*no-drag\s*;/);
  assert.match(body, /(^|[^-])app-region:\s*no-drag\s*;/);
});

// app-region is inherited: each descendant of a no-drag element is a no-drag box at its
// unclipped position. A page or table scrolled down leaves content above its scroll
// container, over the header, and that content erased the header's whole drag area.
const LAYOUT_REGIONS = [".app-main", ".status-bar"];

test("the page area and the status bar never carry no-drag", () => {
  const { excluded } = overlayRule();
  for (const region of LAYOUT_REGIONS) {
    assert.ok(excluded.includes(region), `${region} stays out of the overlay no-drag rule`);
  }

  const afterHeader = BASE_HTML.slice(BASE_HTML.indexOf("</header>"));
  for (const region of LAYOUT_REGIONS) {
    const className = region.slice(1);
    assert.match(
      afterHeader,
      new RegExp(`<(div|footer) class="${className}"`),
      `${region} is a body child after the header, so the exclusion names a real element`
    );
  }
});

// The title-bar surfaces that own a drag shape. Page and component stylesheets inside
// the page area set no app-region at all, so nothing under the header can inherit
// no-drag. Shrink only.
const APP_REGION_OWNERS = new Set([
  "header.css",
  "pages/onboarding.css",
  "pages/setup.css",
  "pages/splash.css",
  "token_details/base.css",
  "ui/position_details/base.css",
  "ui/position_details/header.css",
]);

test("only the title-bar surfaces declare app-region", async () => {
  const layoutClasses = [...LAYOUT_REGIONS, ".content"].map((region) => region.slice(1));
  for (const { path, rules } of await loadStylesheets()) {
    const regionRules = rules.filter(({ body }) => /app-region\s*:/.test(body));
    if (regionRules.length === 0) continue;
    assert.ok(APP_REGION_OWNERS.has(path), `${path} declares app-region outside the owners`);
    for (const { selector, line } of regionRules) {
      const subject = classesIn(subjectCompound(selector).replace(/:not\([^)]*\)/g, ""));
      for (const className of layoutClasses) {
        assert.ok(
          !subject.includes(className),
          `${path}:${line} gives .${className} an app-region (${selector})`
        );
      }
    }
  }
});

test("no separate drag strip precedes the header", () => {
  assert.doesNotMatch(BASE_HTML, /electron-titlebar-bg/);
  assert.doesNotMatch(HEADER_STYLES, /electron-titlebar-bg/);
});
