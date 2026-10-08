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

const { hasOpenOverlay, pushEscapeHandler } = await import(
  "../../src/webserver/templates/scripts/core/escape_stack.js"
);

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
