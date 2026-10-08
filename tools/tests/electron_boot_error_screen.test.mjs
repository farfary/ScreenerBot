// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the desktop boot-error screen (`electron/src/boot.js`), the screen a
 * fatal startup failure ends on. Covered here:
 *   - every startup error code the core can send has its own subtitle, and that
 *     subtitle exists in every shell catalog, so no failure falls back to a generic
 *     heading or shows a raw message id.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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
