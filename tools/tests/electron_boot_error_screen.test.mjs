// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the desktop boot-error screen (`electron/src/boot.js`), the screen a
 * fatal startup failure ends on. Covered here:
 *   - every startup error code the core can send has its own subtitle, and that
 *     subtitle exists in every shell catalog, so no failure falls back to a generic
 *     heading or shows a raw message id;
 *   - Copy details puts the title, detail, remedy and log path on the clipboard.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const BOOT_JS = fs.readFileSync(path.join(ROOT, "electron/src/boot.js"), "utf8");
const STARTUP_RS = fs.readFileSync(path.join(ROOT, "src/errors/startup.rs"), "utf8");

function startupCodes() {
  const body = STARTUP_RS.slice(STARTUP_RS.indexOf("pub fn as_str(self)"));
  return [...body.slice(0, body.indexOf("\n    }\n")).matchAll(/=> "([a-z_]+)"/g)].map((m) => m[1]);
}

function subtitleIds() {
  const block = BOOT_JS.slice(BOOT_JS.indexOf("const SUBTITLE_IDS = {"));
  const body = block.slice(0, block.indexOf("};"));
  return Object.fromEntries([...body.matchAll(/([a-z_]+): '([a-z-]+)'/g)].map((m) => [m[1], m[2]]));
}

test("every startup error code has a subtitle in every shell catalog", () => {
  const codes = startupCodes();
  assert.ok(codes.includes("storage_upgrade"), `codes read from startup.rs: ${codes}`);
  const ids = subtitleIds();
  const locales = fs
    .readdirSync(path.join(ROOT, "locales"), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name);
  for (const code of codes) {
    assert.ok(ids[code], `no subtitle for startup code ${code}`);
    for (const locale of locales) {
      const catalog = fs.readFileSync(path.join(ROOT, "locales", locale, "desktop.ftl"), "utf8");
      assert.match(catalog, new RegExp(`^${ids[code]} = \\S`, "m"), `${locale} lacks ${ids[code]}`);
    }
  }
});

/** Run boot.js against a minimal page and return the boot-error callback and clipboard. */
function loadBootScreen() {
  const elements = new Map();
  const element = (id) => {
    if (!elements.has(id)) {
      elements.set(id, {
        id,
        textContent: "",
        hidden: false,
        innerHTML: "",
        children: [],
        classList: { add() {}, remove() {} },
        append(...parts) {
          this.textContent += parts.map((part) => (typeof part === "string" ? part : part.textContent)).join("");
        },
        appendChild(child) {
          this.children.push(child);
        },
      });
    }
    return elements.get(id);
  };
  const copied = [];
  let onBootError = null;
  const document = {
    documentElement: { setAttribute() {} },
    getElementById: element,
    querySelectorAll: () => [],
    createElement: () => ({
      textContent: "",
      listeners: [],
      addEventListener(type, listener) {
        this.listeners.push(listener);
      },
    }),
  };
  const window = {
    location: { search: "" },
    electronAPI: {
      getShellStrings: () => ({ strings: { "desktop-boot-action-copy": "Copy details" } }),
      getVersion: () => Promise.resolve("0.0.0"),
      onLoadingStatus() {},
      onBootError(callback) {
        onBootError = callback;
      },
    },
  };
  const navigator = {
    clipboard: {
      writeText(value) {
        copied.push(value);
        return Promise.resolve();
      },
    },
  };
  vm.runInNewContext(BOOT_JS, { window, document, navigator, URLSearchParams, setTimeout, console });
  return { render: onBootError, actions: element("bootErrorActions"), copied };
}

test("Copy details copies the title, detail, remedy and log path", async () => {
  const { render, actions, copied } = loadBootScreen();
  render({
    code: "storage_upgrade",
    title: "Your data could not be upgraded",
    detail: "positions.db was not changed",
    remedy: "Send the details to support",
    log_path: "/data/logs/latest.log",
  });
  const copy = actions.children.find((button) => button.textContent === "Copy details");
  assert.ok(copy, "the Copy details button is rendered");
  copy.listeners[0]();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(copied.length, 1);
  for (const part of [
    "Your data could not be upgraded",
    "positions.db was not changed",
    "Send the details to support",
    "/data/logs/latest.log",
  ]) {
    assert.ok(copied[0].includes(part), `${part} missing from ${copied[0]}`);
  }
});
