// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: no planning or tracker id in the repository. Defect, drift, decision,
 * question and plan-unit ids name records kept outside this repository; in code,
 * comments, tests, logs or UI text they point nowhere and age into noise. Text
 * states the technical fact instead. ALLOWED holds exact tokens that match the
 * shape but are not tracker ids (a standard's name, a protocol field); it only
 * shrinks.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const TRACKER_ID = /(?<![\w-])(?:BUG|DEF|ORG|DEC|Q|[A-F])-\d+[a-z]?(?![\w-])/g;
const ALLOWED = new Set([]);
const TEXT = new Set([
  ".rs",
  ".js",
  ".mjs",
  ".css",
  ".html",
  ".ftl",
  ".json",
  ".toml",
  ".yml",
  ".yaml",
  ".md",
  ".sh",
  ".py",
  ".txt",
]);

function repositoryTextFiles() {
  return execFileSync(
    "git",
    ["-C", REPO, "ls-files", "--cached", "--others", "--exclude-standard"],
    {
      encoding: "utf8",
    }
  )
    .split("\n")
    .filter((path) => path && TEXT.has(extname(path)));
}

test("no tracker id appears in any repository file", () => {
  const found = [];
  for (const path of repositoryTextFiles()) {
    let source;
    try {
      source = readFileSync(join(REPO, path), "utf8");
    } catch {
      continue;
    }
    source.split("\n").forEach((line, index) => {
      for (const [token] of line.matchAll(TRACKER_ID)) {
        if (!ALLOWED.has(token)) found.push(`${path}:${index + 1} ${token}`);
      }
    });
  }
  assert.deepEqual(found, []);
});
