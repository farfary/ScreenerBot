// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for overlay detection in the escape stack (`core/escape_stack.js`).
 *
 * `hasOpenOverlay()` gates every global shortcut (search, quick trade). A
 * dialog that stays in the document while closed, such as the notification
 * drawer's panel inside its `aria-hidden` container, must not read as open, or
 * every shortcut it gates stops working for the whole session.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import { SCRIPTS_ROOT, repoPath, walk } from "../lib/dashboard_ui.mjs";

const HIDING_SELECTOR = '[hidden], [aria-hidden="true"], [inert]';

/** An element that knows its own and its ancestors' hiding attributes. */
function element({ parent = null, attributes = {}, style = {} } = {}) {
  return {
    parent,
    attributes,
    style: { display: "block", visibility: "visible", opacity: "1", ...style },
    get hidden() {
      return "hidden" in this.attributes;
    },
    getAttribute(name) {
      return this.attributes[name] ?? null;
    },
    closest(selector) {
      assert.equal(selector, HIDING_SELECTOR);
      for (let node = this; node; node = node.parent) {
        const { attributes: own } = node;
        if ("hidden" in own || own["aria-hidden"] === "true" || "inert" in own) return node;
      }
      return null;
    },
  };
}

let candidates = [];
globalThis.document = {
  querySelectorAll: () => candidates,
  addEventListener() {},
  removeEventListener() {},
};
globalThis.window = { getComputedStyle: (node) => node.style };

const { hasOpenOverlay, pushEscapeHandler } =
  await import("../../src/webserver/templates/scripts/core/escape_stack.js");

test("a visible dialog is an open overlay", () => {
  candidates = [element({ attributes: { role: "dialog" } })];
  assert.equal(hasOpenOverlay(), true);
});

test("a dialog inside a closed container is not an open overlay", () => {
  for (const hiding of [{ "aria-hidden": "true" }, { hidden: "" }, { inert: "" }]) {
    const drawer = element({ attributes: hiding });
    candidates = [element({ parent: drawer, attributes: { role: "dialog" } })];
    assert.equal(hasOpenOverlay(), false, JSON.stringify(hiding));
  }
});

test("a dialog hidden by its own style is not an open overlay", () => {
  candidates = [
    element({ style: { display: "none" } }),
    element({ style: { visibility: "hidden" } }),
    element({ style: { opacity: "0" } }),
  ];
  assert.equal(hasOpenOverlay(), false);
});

test("a registered escape handler counts as an open overlay until released", () => {
  candidates = [];
  const release = pushEscapeHandler(() => {});
  assert.equal(hasOpenOverlay(), true);
  release();
  assert.equal(hasOpenOverlay(), false);
});

/**
 * Files under `scripts/core`, `scripts/ui` and `scripts/pages` that compare a key to "Escape" without
 * dismissing an overlay. The list only shrinks: an overlay takes its Escape from
 * `pushEscapeHandler`, never from its own listener, because the stack's capture
 * listener hands Escape to the topmost overlay and an overlay still on its own
 * listener loses it to any overlay stacked beneath.
 */
const ESCAPE_COMPARISON_ALLOWED = {
  "src/webserver/templates/scripts/core/escape_stack.js":
    "the stack itself reads Escape for every overlay",
  "src/webserver/templates/scripts/core/menu_manager.js":
    "dropdown menus close through the menu manager, which the stack defers to while a menu is open",
  "src/webserver/templates/scripts/core/tooltip.js": "a non-modal hover tooltip hides on Escape",
  "src/webserver/templates/scripts/core/chat_widget.js":
    "the composer field's own key handling: close the sessions list, decline a pending tool, cancel a request",
  "src/webserver/templates/scripts/core/onboarding.js":
    "the full-screen first-run tour; nothing stacks above it",
  "src/webserver/templates/scripts/ui/data_table/event_handlers.js":
    "the toolbar search field clears or blurs itself",
  "src/webserver/templates/scripts/pages/home/portfolio_calendar.js":
    "the day-detail tooltip of the calendar grid hides while the grid has focus",
};

test("no overlay reads the Escape key outside the escape stack", async () => {
  const found = new Set();
  const files = (await walk(SCRIPTS_ROOT)).filter(
    (path) => path.endsWith(".js") && /\/scripts\/(?:core|ui|pages)\//.test(path)
  );
  for (const path of files) {
    const code = readFileSync(path, "utf8")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/^\s*\/\/.*$/gm, "");
    if (/["'`]Escape["'`]/.test(code)) found.add(repoPath(path));
  }
  assert.deepEqual(
    [...found].sort(),
    Object.keys(ESCAPE_COMPARISON_ALLOWED).sort(),
    "an overlay closes through pushEscapeHandler (core/escape_stack.js); a file that is not an overlay joins the allowlist with its reason"
  );
});
