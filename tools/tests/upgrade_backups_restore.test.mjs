// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the desktop shell's restore of upgrade backups
 * (`electron/src/upgrade_backups.js`). A rollback relaunches the bundled core, which
 * cannot read storage the failed staged core upgraded, so the backups that core took
 * during the failed launch replace the stores, and nothing else is touched:
 *   - only a manifest naming the failed version and taken during that launch counts,
 *   - the store's WAL and shared-memory files never survive into the restored store,
 *   - a manifest whose store does not match its own file name is ignored.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const require = createRequire(import.meta.url);
const { restoreUpgradeBackups, belongsToFailedLaunch } = require("../../electron/src/upgrade_backups.js");

async function dataDir() {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-upgrade-backups-"));
  await fs.mkdir(path.join(dir, "backups"));
  return dir;
}

async function seed(dir, stem, manifest) {
  await fs.writeFile(path.join(dir, `${stem}.db`), "upgraded");
  await fs.writeFile(path.join(dir, `${stem}.db-wal`), "upgraded wal");
  await fs.writeFile(path.join(dir, `${stem}.db-shm`), "upgraded shm");
  await fs.writeFile(path.join(dir, "backups", `upgrade-${stem}.db`), "before upgrade");
  await fs.writeFile(path.join(dir, "backups", `upgrade-${stem}.json`), JSON.stringify(manifest));
}

const exists = (file) => fs.access(file).then(() => true, () => false);

test("a backup taken by the failed launch replaces its store and drops the sidecars", async () => {
  const dir = await dataDir();
  await seed(dir, "positions", { store: "positions.db", coreVersion: "0.2.14", fromSchemaVersion: "5", createdAtMs: 2000 });

  const restored = await restoreUpgradeBackups(dir, "0.2.14", 1000);

  assert.deepEqual(restored, ["positions.db"]);
  assert.equal(await fs.readFile(path.join(dir, "positions.db"), "utf8"), "before upgrade");
  assert.equal(await exists(path.join(dir, "positions.db-wal")), false);
  assert.equal(await exists(path.join(dir, "positions.db-shm")), false);
  assert.equal(await exists(path.join(dir, "positions.db.restore-partial")), false);
  await fs.rm(dir, { recursive: true, force: true });
});

test("backups of another version or an earlier launch are left alone", async () => {
  const dir = await dataDir();
  await seed(dir, "positions", { store: "positions.db", coreVersion: "0.2.13", createdAtMs: 2000 });
  await seed(dir, "wallets", { store: "wallets.db", coreVersion: "0.2.14", createdAtMs: 500 });

  assert.deepEqual(await restoreUpgradeBackups(dir, "0.2.14", 1000), []);
  assert.equal(await fs.readFile(path.join(dir, "positions.db"), "utf8"), "upgraded");
  assert.equal(await fs.readFile(path.join(dir, "wallets.db-wal"), "utf8"), "upgraded wal");
  await fs.rm(dir, { recursive: true, force: true });
});

test("a manifest must name the store its own file name belongs to", () => {
  const manifest = { store: "../config.db", coreVersion: "0.2.14", createdAtMs: 2000 };
  assert.equal(belongsToFailedLaunch(manifest, "positions", "0.2.14", 1000), false);
  assert.equal(belongsToFailedLaunch(null, "positions", "0.2.14", 1000), false);
  assert.equal(belongsToFailedLaunch({ ...manifest, store: "positions.db" }, "positions", "0.2.14", undefined), false);
  assert.equal(belongsToFailedLaunch({ ...manifest, store: "positions.db" }, "positions", "0.2.14", 1000), true);
});

test("a missing backups directory restores nothing", async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "sb-upgrade-backups-"));
  assert.deepEqual(await restoreUpgradeBackups(dir, "0.2.14", 0), []);
  await fs.rm(dir, { recursive: true, force: true });
});
