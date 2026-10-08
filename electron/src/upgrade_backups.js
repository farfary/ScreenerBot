// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Restores the stores a staged core upgraded before it failed its first start, from the backups that core took in `<data>/backups` (written by `src/database/backup.rs`), so the bundled core finds the storage it can read.

'use strict';

const fs = require('fs');
const fsp = fs.promises;
const path = require('path');

const BACKUPS_DIRECTORY = 'backups';
const MANIFEST_PATTERN = /^upgrade-([a-z0-9_-]+)\.json$/;

async function readManifest(file) {
  try {
    return JSON.parse(await fsp.readFile(file, 'utf8'));
  } catch (_) {
    return null;
  }
}

/**
 * Whether a manifest describes a backup taken by `failedVersion` during the launch
 * that started at `since` (Unix milliseconds) for the store named by its file stem.
 */
function belongsToFailedLaunch(manifest, stem, failedVersion, since) {
  return Boolean(manifest)
    && manifest.store === `${stem}.db`
    && manifest.coreVersion === failedVersion
    && Number.isFinite(manifest.createdAtMs)
    && Number.isFinite(since)
    && manifest.createdAtMs >= since;
}

/**
 * Put each matching backup back in place of its store. The failed backend must have
 * exited: its WAL and shared-memory files describe the upgraded file and are removed
 * before the backup takes the store's name, so they are never replayed onto it.
 * Returns the store names restored.
 */
async function restoreUpgradeBackups(dataDir, failedVersion, since) {
  const backupsDir = path.join(dataDir, BACKUPS_DIRECTORY);
  let entries;
  try {
    entries = await fsp.readdir(backupsDir);
  } catch (_) {
    return [];
  }
  const restored = [];
  for (const entry of entries.sort()) {
    const match = MANIFEST_PATTERN.exec(entry);
    if (!match) continue;
    const stem = match[1];
    const manifest = await readManifest(path.join(backupsDir, entry));
    if (!belongsToFailedLaunch(manifest, stem, failedVersion, since)) continue;
    const backup = path.join(backupsDir, `upgrade-${stem}.db`);
    const store = path.join(dataDir, manifest.store);
    const partial = `${store}.restore-partial`;
    await fsp.copyFile(backup, partial);
    await Promise.all(['-wal', '-shm'].map((suffix) => fsp.rm(`${store}${suffix}`, { force: true })));
    await fsp.rename(partial, store);
    restored.push(manifest.store);
  }
  return restored;
}

module.exports = { belongsToFailedLaunch, restoreUpgradeBackups };
