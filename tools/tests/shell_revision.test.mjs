/**
 * Tests for the shell revision (`electron/tools/shell-revision.js`).
 *
 * The revision is the whole basis of a core-only update: if it is unstable, or
 * if it moves for a reason that is not a shell change, every release looks like
 * a shell release and silent updates never happen. If it fails to move when the
 * shell DOES change, a release would install a core against an Electron build it
 * was not made for. So the properties tested are exactly those two.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";

const require = createRequire(import.meta.url);
const { computeShellRevision, readShellRevision, REVISION_FILE } = require("../../electron/tools/shell-revision.js");

const MAIN = path.join(path.dirname(REVISION_FILE), "main.js");

test("the revision is a short lowercase hex digest", () => {
  const revision = computeShellRevision();
  assert.match(revision, /^[0-9a-f]{12}$/);
});

test("the revision is stable across repeated computation", () => {
  assert.equal(computeShellRevision(), computeShellRevision());
});

test("changing a shell source changes the revision", () => {
  const before = computeShellRevision();
  const original = fs.readFileSync(MAIN);
  try {
    fs.appendFileSync(MAIN, "\n// revision probe\n");
    assert.notEqual(computeShellRevision(), before);
  } finally {
    fs.writeFileSync(MAIN, original);
  }
  assert.equal(computeShellRevision(), before);
});

test("line endings are not part of the revision", () => {
  // The revision is computed on the Linux runner and re-asserted on the macOS
  // and Windows packaging runners. A Windows checkout may materialise CRLF,
  // which changes nothing about the shell — but before the revision normalised
  // line endings it changed the hash, failing the assertion and taking the whole
  // release build down with it.
  const before = computeShellRevision();
  const original = fs.readFileSync(MAIN);
  try {
    fs.writeFileSync(MAIN, original.toString("latin1").replace(/\n/g, "\r\n"), "latin1");
    assert.equal(computeShellRevision(), before);
  } finally {
    fs.writeFileSync(MAIN, original);
  }
  assert.equal(computeShellRevision(), before);
});

test("the release version is not part of the revision", () => {
  const manifestPath = path.join(path.dirname(REVISION_FILE), "..", "package.json");
  const before = computeShellRevision();
  const original = fs.readFileSync(manifestPath, "utf8");
  try {
    const manifest = JSON.parse(original);
    manifest.version = "99.99.99";
    fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    // A core-only release still bumps the version; treating that as a shell
    // change would disable the silent path for every release.
    assert.equal(computeShellRevision(), before);
  } finally {
    fs.writeFileSync(manifestPath, original);
  }
});

test("the generated revision file is not part of its own input", () => {
  const before = computeShellRevision();
  const existed = fs.existsSync(REVISION_FILE);
  const original = existed ? fs.readFileSync(REVISION_FILE) : null;
  try {
    fs.writeFileSync(REVISION_FILE, JSON.stringify({ revision: "deadbeefcafe" }));
    assert.equal(computeShellRevision(), before);
    assert.equal(readShellRevision(), "deadbeefcafe");
  } finally {
    if (existed) fs.writeFileSync(REVISION_FILE, original);
    else fs.rmSync(REVISION_FILE, { force: true });
  }
});

test("an unreadable or malformed revision file reads as absent", () => {
  const existed = fs.existsSync(REVISION_FILE);
  const original = existed ? fs.readFileSync(REVISION_FILE) : null;
  try {
    fs.writeFileSync(REVISION_FILE, JSON.stringify({ revision: "not hex" }));
    assert.equal(readShellRevision(), null);
    fs.writeFileSync(REVISION_FILE, "{ broken");
    assert.equal(readShellRevision(), null);
  } finally {
    if (existed) fs.writeFileSync(REVISION_FILE, original);
    else fs.rmSync(REVISION_FILE, { force: true });
  }
});
