#!/usr/bin/env node
// Identity of the Electron shell, so the updater can tell whether a release
// actually changes it.
//
// The core binary carries the whole dashboard, so most releases change nothing
// about Electron. When the revision a release was built with equals the revision
// of the running shell, the update is core-only: tens of megabytes and a restart
// instead of a full Chromium bundle and an operating-system installer.
//
// The hash therefore has to cover exactly what ends up in the packaged shell and
// nothing that changes on every release:
//   - every file under src/ (except the generated revision file itself),
//   - every packaged asset,
//   - forge.config.js — it decides what is packaged and how,
//   - package.json WITHOUT its version field,
//   - package-lock.json, which pins the complete packaged dependency graph,
//     excluding the root application's release version.
//
// Deterministic across machines: paths are POSIX-normalised and sorted, text is
// hashed with LF line endings, and binaries are hashed byte for byte.
//
// Usage:  node tools/shell-revision.js [--write]
//   --write  also refresh src/shell_revision.json (the copy the app reads)

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const SHELL_ROOT = path.join(__dirname, '..');
const REVISION_FILE = path.join(SHELL_ROOT, 'src', 'shell_revision.json');
const REVISION_LENGTH = 12;

const INPUT_DIRECTORIES = ['src', 'assets'];
const INPUT_FILES = ['forge.config.js'];
/** Never part of the identity: generated, or noise that is not shipped. */
const EXCLUDED = new Set(['shell_revision.json', '.DS_Store']);

function walk(directory, base = directory) {
  if (!fs.existsSync(directory)) return [];
  const found = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    if (EXCLUDED.has(entry.name)) continue;
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      found.push(...walk(absolute, base));
    } else if (entry.isFile()) {
      found.push(absolute);
    }
  }
  return found;
}

/**
 * package.json minus the fields that move on every release. The version is
 * deliberately excluded: a release that only rebuilds the core still bumps it,
 * and treating that as a shell change would defeat the whole mechanism.
 */
function packageIdentity() {
  const manifest = JSON.parse(fs.readFileSync(path.join(SHELL_ROOT, 'package.json'), 'utf8'));
  delete manifest.version;
  return stableJson(manifest);
}

/**
 * package-lock.json minus the two root-version fields npm rewrites when the app
 * version changes. Dependency versions and integrity hashes remain covered,
 * while a core-only release bump does not falsely look like a shell rebuild.
 */
function packageLockIdentity() {
  const lock = JSON.parse(fs.readFileSync(path.join(SHELL_ROOT, 'package-lock.json'), 'utf8'));
  delete lock.version;
  if (lock.packages?.['']) delete lock.packages[''].version;
  return stableJson(lock);
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

/**
 * The bytes of one input, as the revision must see them.
 *
 * The revision identifies the shell *source*, and it is computed once on the
 * Linux runner but re-asserted on every packaging runner. A Windows checkout can
 * legitimately materialise text files with CRLF, which changes nothing about the
 * shell yet would move the hash and fail that assertion — taking the whole
 * release down with it. So text is hashed with LF line endings regardless of how
 * it landed on disk. Binaries (icons, images) are hashed exactly as they are,
 * and are recognised the way git recognises them: a NUL byte near the start.
 */
function contentForHash(file) {
  const bytes = fs.readFileSync(file);
  if (bytes.subarray(0, 8000).includes(0)) return bytes;
  // latin1 round-trips every byte value, so this rewrites line endings without
  // reinterpreting the content.
  return Buffer.from(bytes.toString('latin1').replace(/\r\n/g, '\n'), 'latin1');
}

/** Compute the revision of the shell sources in this checkout. */
function computeShellRevision() {
  const files = [
    ...INPUT_DIRECTORIES.flatMap((directory) => walk(path.join(SHELL_ROOT, directory))),
    ...INPUT_FILES.map((file) => path.join(SHELL_ROOT, file)).filter((file) => fs.existsSync(file)),
  ];

  const entries = files
    .map((file) => ({ key: path.relative(SHELL_ROOT, file).split(path.sep).join('/'), file }))
    .sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));

  const hash = crypto.createHash('sha256');
  hash.update('screenerbot-shell\0');
  hash.update(packageIdentity());
  hash.update('\0package-lock.json\0');
  hash.update(packageLockIdentity());
  for (const entry of entries) {
    hash.update(`\0${entry.key}\0`);
    hash.update(contentForHash(entry.file));
  }
  return hash.digest('hex').slice(0, REVISION_LENGTH);
}

/** Read the revision baked into this build, or null when it was never generated. */
function readShellRevision() {
  try {
    const value = JSON.parse(fs.readFileSync(REVISION_FILE, 'utf8')).revision;
    return typeof value === 'string' && /^[0-9a-f]{8,64}$/.test(value) ? value : null;
  } catch (_) {
    return null;
  }
}

function writeShellRevision(revision) {
  fs.writeFileSync(REVISION_FILE, `${JSON.stringify({ revision }, null, 2)}\n`);
  return revision;
}

if (require.main === module) {
  const revision = computeShellRevision();
  if (process.argv.includes('--write')) writeShellRevision(revision);
  process.stdout.write(`${revision}\n`);
}

module.exports = {
  REVISION_FILE,
  computeShellRevision,
  readShellRevision,
  writeShellRevision,
};
