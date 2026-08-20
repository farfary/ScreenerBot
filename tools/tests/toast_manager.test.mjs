/**
 * Tests for the toast manager's identity rules (`core/toast.js`).
 *
 * The whole point of the layer is that ONE subject owns ONE toast: a swap used
 * to raise three separate notices that stacked on screen. These assertions pin
 * the two rules that guarantee it — an explicit key updates the toast already
 * showing, and a repeated unkeyed notice collapses into a counter — plus the
 * visible/queued caps that bound a burst.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

installDomStub();

const { toastManager } = await import("../../src/webserver/templates/scripts/core/toast.js");

/**
 * The toast view touches only a handful of DOM APIs; stubbing them keeps this a
 * plain node test instead of a browser one.
 */
function installDomStub() {
  const makeElement = () => {
    const element = {
      children: [],
      dataset: {},
      style: {},
      hidden: false,
      textContent: "",
      className: "",
      innerHTML: "",
      attributes: {},
      classList: {
        classes: new Set(),
        add(...names) {
          names.forEach((name) => this.classes.add(name));
        },
        remove(...names) {
          names.forEach((name) => this.classes.delete(name));
        },
        contains(name) {
          return this.classes.has(name);
        },
        toggle(name, on) {
          if (on) this.add(name);
          else this.remove(name);
        },
      },
      setAttribute(name, value) {
        this.attributes[name] = value;
      },
      addEventListener() {},
      appendChild(child) {
        this.children.push(child);
        return child;
      },
      remove() {},
      querySelector() {
        return makeElement();
      },
    };
    return element;
  };

  globalThis.document = {
    body: makeElement(),
    createElement: () => makeElement(),
  };
  globalThis.requestAnimationFrame = (fn) => fn();
}

/** Toasts are global state; each test starts from an empty screen. */
function reset() {
  [...toastManager.entries.keys()].forEach((key) => toastManager.dismiss(key));
  toastManager.entries.clear();
  toastManager.visible.length = 0;
  toastManager.queued.length = 0;
}

test("an explicit key updates the toast already showing instead of stacking", () => {
  reset();
  toastManager.show({ key: "trade:BONK", type: "progress", title: "Buying BONK" });
  toastManager.show({ key: "trade:BONK", type: "success", title: "Bought BONK" });

  assert.equal(toastManager.entries.size, 1);
  const entry = toastManager.entries.get("trade:BONK");
  assert.equal(entry.config.type, "success");
  assert.equal(entry.config.title, "Bought BONK");
  // An update is not a repeat: no "x2" counter on a notice that moved on.
  assert.equal(entry.repeat, 1);
});

test("the same unkeyed notice fired again is counted, not repeated", () => {
  reset();
  toastManager.show({ type: "success", title: "Address copied" });
  toastManager.show({ type: "success", title: "Address copied" });
  toastManager.show({ type: "success", title: "Address copied" });

  assert.equal(toastManager.entries.size, 1);
  assert.equal(toastManager.entries.get("success:Address copied").repeat, 3);
});

test("different notices still get their own toast", () => {
  reset();
  toastManager.show({ type: "success", title: "Address copied" });
  toastManager.show({ type: "error", title: "Sell failed" });

  assert.equal(toastManager.entries.size, 2);
});

test("only three toasts are visible at once; the rest queue", () => {
  reset();
  for (let i = 0; i < 6; i += 1) {
    toastManager.show({ type: "info", title: `Notice ${i}` });
  }

  assert.equal(toastManager.visible.length, 3);
  assert.equal(toastManager.queued.length, 3);
});

test("a burst cannot build an unbounded backlog", () => {
  reset();
  for (let i = 0; i < 40; i += 1) {
    toastManager.show({ type: "info", title: `Notice ${i}` });
  }

  assert.equal(toastManager.visible.length, 3);
  assert.equal(toastManager.queued.length, 8);
  assert.equal(toastManager.entries.size, 11);
});

test("dismissing a visible toast promotes a queued one once it has animated out", async () => {
  reset();
  for (let i = 0; i < 5; i += 1) {
    toastManager.show({ type: "info", title: `Notice ${i}` });
  }
  const first = toastManager.visible[0];
  toastManager.dismiss(first);

  assert.equal(toastManager.entries.has(first), false);
  // The slot opens when the leaving toast is gone, not while it is still there.
  assert.equal(toastManager.queued.length, 2);

  await new Promise((resolve) => setTimeout(resolve, 300));
  assert.equal(toastManager.queued.length, 1);
  assert.equal(toastManager.visible.length, 3);
});

test("a progress toast never auto-dismisses on a timer", () => {
  reset();
  toastManager.show({ key: "trade:WIF", type: "progress", title: "Selling WIF" });
  const entry = toastManager.entries.get("trade:WIF");

  assert.equal(entry.timer, null);
  assert.notEqual(entry.stallTimer, null);
  clearTimeout(entry.stallTimer);
});
