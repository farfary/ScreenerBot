// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Dashboard scripts must read an API error response through the envelope
 * helpers in `core/request_manager.js` (`apiErrorMessage`, `apiErrorTitle`,
 * `apiErrorDetails`).
 *
 * Every error body is `{ error: { code, message, text, details, timestamp } }`,
 * so `body.message` / `body.error` never carry the server's explanation and a
 * string read of them silently falls back to a generic label. A line that
 * reads a SUCCESS body's own documented field may opt out with
 * `// api-body-ok: <reason>`.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..", "..");
const SCRIPTS = path.join(ROOT, "src/webserver/templates/scripts");
const OWNER = path.join(SCRIPTS, "core", "request_manager.js");

const BODY_READ =
  /\b(data|result|res|body|json|payload|err(or)?Body)\??\.(message|error)\b|\.error\??\.message\b/;
const ESCAPE = /\/\/\s*api-body-ok:\s*\S/;

function walk(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(full) : [full];
  });
}

test("API error bodies are read through the envelope helpers", () => {
  const offenders = [];
  for (const file of walk(SCRIPTS)) {
    if (!file.endsWith(".js") || file === OWNER) continue;
    fs.readFileSync(file, "utf8")
      .split("\n")
      .forEach((line, index) => {
        if (BODY_READ.test(line) && !ESCAPE.test(line)) {
          offenders.push(`${path.relative(ROOT, file)}:${index + 1}: ${line.trim()}`);
        }
      });
  }
  assert.deepEqual(
    offenders,
    [],
    `Read API errors with apiErrorMessage/apiErrorTitle/apiErrorDetails, or mark a success-body field with "// api-body-ok: <reason>":\n${offenders.join("\n")}`
  );
});
