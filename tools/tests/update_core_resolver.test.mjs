/**
 * Tests for the desktop shell's core resolver (`electron/src/core_resolver.js`)
 * and the data paths it shares with the Rust core (`electron/src/paths.js`).
 *
 * This module is the activation step of every silent update: what it returns is
 * literally the binary the machine runs next. So the properties under test are
 * the ones that decide whether a bad update can take effect at all —
 *   - a pointer that is malformed, traversable or undigested is refused,
 *   - a version that already failed to start is never adopted again,
 *   - an installer that overtook the staged core wins,
 *   - a file whose bytes do not match the recorded digest is quarantined,
 * and that Electron resolves the same data directory as `paths::get_base_directory()`.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";

const require = createRequire(import.meta.url);
const resolver = require("../../electron/src/core_resolver.js");
const paths = require("../../electron/src/paths.js");

const BINARY = resolver.coreBinaryName();

function pointer(overrides = {}) {
  return {
    version: "0.2.2",
    path: `0.2.2/${BINARY}`,
    sha256: "a".repeat(64),
    size: 10,
    staged_at: new Date().toISOString(),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// The decision
// ---------------------------------------------------------------------------

test("a newer staged core is adopted over the bundled binary", () => {
  const decision = resolver.chooseCore({ staged: pointer(), bundledVersion: "0.2.1" });
  assert.equal(decision.use, "staged");
  assert.equal(decision.prune, false);
});

test("an installer that overtook the staged core wins and the stage is pruned", () => {
  for (const bundled of ["0.2.2", "0.2.3"]) {
    const decision = resolver.chooseCore({ staged: pointer(), bundledVersion: bundled });
    assert.equal(decision.use, "bundled", `bundled ${bundled}`);
    assert.equal(decision.prune, true);
  }
});

test("a pointer that could escape the core directory is refused", () => {
  for (const bad of [
    pointer({ path: `../../${BINARY}` }),
    pointer({ path: `0.2.2/nested/${BINARY}` }),
    pointer({ path: "0.2.2/other" }),
    pointer({ version: "../etc" }),
    pointer({ version: "0.2" }),
  ]) {
    const decision = resolver.chooseCore({ staged: bad, bundledVersion: "0.2.1" });
    assert.equal(decision.use, "bundled", JSON.stringify(bad));
    assert.equal(decision.prune, true);
  }
});

test("a pointer without a usable digest is refused", () => {
  for (const digest of [undefined, "", "not-hex", "A".repeat(64), "a".repeat(63)]) {
    const decision = resolver.chooseCore({
      staged: pointer({ sha256: digest }),
      bundledVersion: "0.2.1",
    });
    assert.equal(decision.use, "bundled", String(digest));
  }
});

test("a version that already failed to start is never adopted again", () => {
  const decision = resolver.chooseCore({
    staged: pointer(),
    bundledVersion: "0.2.1",
    quarantined: ["0.2.2"],
  });
  assert.equal(decision.use, "bundled");
  assert.match(decision.reason, /failed to start/);
});

test("no pointer at all is the ordinary case, and prunes nothing", () => {
  const decision = resolver.chooseCore({ staged: null, bundledVersion: "0.2.1" });
  assert.equal(decision.use, "bundled");
  assert.equal(decision.prune, false);
});

test("versions compare numerically, not lexically", () => {
  assert.equal(resolver.compareVersions("0.2.10", "0.2.9"), 1);
  assert.equal(resolver.compareVersions("0.2.9", "0.2.10"), -1);
  assert.equal(resolver.compareVersions("1.0.0", "1.0.0"), 0);
  assert.equal(resolver.compareVersions("0.2.2-rc.1", "0.2.2"), 0);
});

// ---------------------------------------------------------------------------
// Against a real staging directory
// ---------------------------------------------------------------------------

async function stage(coreDir, { version = "0.2.2", contents = "core-bytes", digest } = {}) {
  await fs.mkdir(path.join(coreDir, version), { recursive: true });
  const binaryPath = path.join(coreDir, version, BINARY);
  await fs.writeFile(binaryPath, contents);
  await fs.writeFile(
    path.join(coreDir, "current.json"),
    JSON.stringify({
      version,
      path: `${version}/${BINARY}`,
      sha256: digest ?? crypto.createHash("sha256").update(contents).digest("hex"),
      size: Buffer.byteLength(contents),
      staged_at: new Date().toISOString(),
    })
  );
  return binaryPath;
}

test("a verified staged core is what gets launched", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  const binaryPath = await stage(dir);
  const resolved = await resolver.resolveCore({
    coreDir: dir,
    bundledPath: "/bundled/screenerbot",
    bundledVersion: "0.2.1",
  });
  assert.equal(resolved.staged, true);
  assert.equal(resolved.path, binaryPath);
  assert.equal(resolved.version, "0.2.2");
  assert.equal(resolved.firstRun, true);
});

// ---------------------------------------------------------------------------
// Adoption — what separates "applying an update" from "this is the version I run"
// ---------------------------------------------------------------------------

test("a staged core is a first run only until it has actually come up", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  await stage(dir);
  const options = {
    coreDir: dir,
    bundledPath: "/bundled/screenerbot",
    bundledVersion: "0.2.1",
  };

  const first = await resolver.resolveCore(options);
  assert.equal(first.firstRun, true, "the launch that adopts the update announces it");

  await resolver.markCoreAdopted(dir, "0.2.2");

  const second = await resolver.resolveCore(options);
  assert.equal(second.staged, true, "the staged core still runs");
  assert.equal(second.firstRun, false, "but it is no longer news");
});

test("the bundled binary is never reported as a first run", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  const resolved = await resolver.resolveCore({
    coreDir: dir,
    bundledPath: "/bundled/screenerbot",
    bundledVersion: "0.2.1",
  });
  assert.equal(resolved.staged, false);
  assert.equal(resolved.firstRun, false);
});

test("only well-formed versions are ever recorded as adopted", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  await resolver.markCoreAdopted(dir, "../../etc/passwd");
  await resolver.markCoreAdopted(dir, "0.2.2");
  assert.deepEqual(await resolver.readAdoptedCores(dir), ["0.2.2"]);
});

test("dropping the stage drops the adoption record with it", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  await stage(dir);
  await resolver.markCoreAdopted(dir, "0.2.2");

  // A full installer overtook the staged core: the stage is pruned, and the
  // record of what was adopted from it must not outlive it.
  const resolved = await resolver.resolveCore({
    coreDir: dir,
    bundledPath: "/bundled/screenerbot",
    bundledVersion: "0.2.2",
  });
  assert.equal(resolved.staged, false);
  assert.deepEqual(await resolver.readAdoptedCores(dir), []);
});

test("a staged core whose bytes changed is quarantined, not launched", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  await stage(dir, { digest: "b".repeat(64) });
  const resolved = await resolver.resolveCore({
    coreDir: dir,
    bundledPath: "/bundled/screenerbot",
    bundledVersion: "0.2.1",
  });
  assert.equal(resolved.staged, false);
  assert.equal(resolved.path, "/bundled/screenerbot");

  const quarantine = JSON.parse(await fs.readFile(path.join(dir, "quarantine.json"), "utf8"));
  assert.deepEqual(quarantine.versions, ["0.2.2"]);
  // The pointer is dropped so the next launch does not repeat the work.
  await assert.rejects(fs.access(path.join(dir, "current.json")));
});

test("a quarantined version stays quarantined across relaunches", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  await stage(dir);
  await resolver.quarantineStagedCore(dir, "0.2.2");
  await stage(dir); // the updater re-staged it

  const resolved = await resolver.resolveCore({
    coreDir: dir,
    bundledPath: "/bundled/screenerbot",
    bundledVersion: "0.2.1",
  });
  assert.equal(resolved.staged, false);
});

test("pruning removes version trees and leaves anything else alone", async (t) => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-core-"));
  t.after(() => fs.rm(dir, { recursive: true, force: true }));

  for (const name of ["0.2.1", "0.2.2", ".staging-0.2.3", "notes"]) {
    await fs.mkdir(path.join(dir, name), { recursive: true });
  }
  await resolver.pruneStagedCores(dir, "0.2.2");

  await fs.access(path.join(dir, "0.2.2"));
  await fs.access(path.join(dir, "notes"));
  await assert.rejects(fs.access(path.join(dir, "0.2.1")));
  await assert.rejects(fs.access(path.join(dir, ".staging-0.2.3")));
});

// ---------------------------------------------------------------------------
// Data paths — must agree with src/paths/mod.rs
// ---------------------------------------------------------------------------

test("the base directory matches what the Rust core resolves", () => {
  assert.equal(
    paths.resolveBaseDirectory("/home/u", "darwin", {}),
    "/home/u/Library/Application Support/ScreenerBot"
  );
  assert.equal(
    paths.resolveBaseDirectory("/home/u", "win32", { LOCALAPPDATA: "C:\\Users\\u\\AppData\\Local" }),
    path.join("C:\\Users\\u\\AppData\\Local", "ScreenerBot")
  );
  assert.equal(
    paths.resolveBaseDirectory("/home/u", "linux", {}),
    "/home/u/.local/share/ScreenerBot"
  );
  assert.equal(
    paths.resolveBaseDirectory("/home/u", "linux", { XDG_DATA_HOME: "/data" }),
    "/data/ScreenerBot"
  );
});

test("the test-and-development data override wins on every platform", () => {
  for (const platform of ["darwin", "win32", "linux"]) {
    assert.equal(
      paths.resolveBaseDirectory("/home/u", platform, { SCREENERBOT_DATA_DIR: "/tmp/pinned" }),
      "/tmp/pinned"
    );
  }
});

test("the core directory hangs off the data directory, not the base", () => {
  assert.equal(paths.coreDirectory("/base"), path.join("/base", "data", "core"));
  assert.equal(paths.logsDirectory("/base"), path.join("/base", "logs"));
});
