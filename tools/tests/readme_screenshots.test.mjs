// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guard: the README shows product screenshots committed in this repository under
 * `assets/screenshots/`, never images served by a website. A remote image renders
 * only while that site serves it and changes without a commit; a repository copy
 * renders anywhere the repository is read and changes only with the code.
 *
 * - The only remote README images are generated status badges (`BADGE_HOSTS`).
 * - Every other image and image link is a repository path to a file that exists.
 * - Every file under `assets/screenshots/` is shown by the README.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { REPO_ROOT } from "../lib/dashboard_ui.mjs";

const README = readFileSync(resolve(REPO_ROOT, "README.md"), "utf8");
const SCREENSHOTS_DIR = "assets/screenshots";
// Badges render live repository and build status, which no committed file can.
const BADGE_HOSTS = new Set(["img.shields.io"]);

const imageSources = [
  ...[...README.matchAll(/<img\b[^>]*\bsrc="([^"]+)"/g)].map((match) => match[1]),
  ...[...README.matchAll(/!\[[^\]]*\]\(([^)\s]+)/g)].map((match) => match[1]),
];
const imageLinks = [...README.matchAll(/<a\b[^>]*\bhref="([^"]+\.(?:png|jpe?g|webp|gif))"/g)].map(
  (match) => match[1]
);

const isRemote = (source) => /^[a-z]+:\/\//i.test(source);

test("README images are files in this repository, except status badges", () => {
  assert.ok(imageSources.length > 0, "the README shows images");
  const remote = [...imageSources, ...imageLinks].filter(
    (source) => isRemote(source) && !BADGE_HOSTS.has(new URL(source).hostname)
  );
  assert.deepEqual(remote, [], "README images and image links must be repository paths");
  const absent = [...imageSources, ...imageLinks].filter(
    (source) => !isRemote(source) && !existsSync(resolve(REPO_ROOT, source))
  );
  assert.deepEqual(absent, [], "every README image path exists");
});

test("every committed screenshot is shown by the README", () => {
  const shown = new Set(imageSources);
  const unused = readdirSync(resolve(REPO_ROOT, SCREENSHOTS_DIR))
    .map((name) => `${SCREENSHOTS_DIR}/${name}`)
    .filter((path) => !shown.has(path));
  assert.deepEqual(unused, [], "no screenshot without a README reference");
});
