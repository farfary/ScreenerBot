// Which core binary this launch should run.
//
// A core-only update never rewrites the installed application. The backend
// stages a verified binary under <data>/core/<version>/ and publishes a pointer;
// this module is what actually adopts it, immediately before every backend
// spawn. That makes the restart the activation step, so an update that cannot be
// trusted simply never takes effect and the shipped binary stays in charge.
//
// Three things have to hold before a staged core is used:
//   1. the pointer parses and names a version newer than the bundled one,
//   2. the file it names exists, with the recorded size,
//   3. its SHA-256 equals the digest recorded when it was staged.
//
// If the adopted core then fails to come up, `quarantineStagedCore` records the
// version so neither this launch nor the updater will try it again, and the
// caller relaunches with the bundled binary.
//
// A staged core stays staged for good — the pointer is what makes it the version
// the machine runs, so it is still there on every later launch. `adopted.json`
// records the versions that have actually come up, which is the only way the
// shell can tell "this update is being applied right now" from "this has been
// the installed version for weeks". Without it every launch looks like an
// install.

const fs = require('fs');
const fsp = require('fs/promises');
const path = require('path');
const crypto = require('crypto');

const POINTER_FILE = 'current.json';
const QUARANTINE_FILE = 'quarantine.json';
const ADOPTED_FILE = 'adopted.json';

/** Plain `MAJOR.MINOR.PATCH` with an optional pre-release/build suffix. */
const VERSION_RE = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

function coreBinaryName(platform = process.platform) {
  return platform === 'win32' ? 'screenerbot.exe' : 'screenerbot';
}

/** Numeric comparison of two release versions; pre-release suffixes are ignored. */
function compareVersions(left, right) {
  const parts = (value) => String(value).split(/[-+]/)[0].split('.').map((n) => Number(n) || 0);
  const a = parts(left);
  const b = parts(right);
  for (let i = 0; i < 3; i += 1) {
    if ((a[i] || 0) > (b[i] || 0)) return 1;
    if ((a[i] || 0) < (b[i] || 0)) return -1;
  }
  return 0;
}

function isValidVersion(value) {
  return typeof value === 'string' && VERSION_RE.test(value);
}

/** The pointer path is only ever `<version>/<binary>` — never anything traversable. */
function isValidPointerPath(value, platform = process.platform) {
  if (typeof value !== 'string') return false;
  const segments = value.split('/');
  return segments.length === 2
    && isValidVersion(segments[0])
    && segments[1] === coreBinaryName(platform);
}

/**
 * The pure decision: given a parsed pointer and the bundled version, which core
 * should run? Separated from the filesystem so every branch is directly testable.
 *
 * @returns {{use:'bundled'|'staged', prune:boolean, reason:string}}
 */
function chooseCore({ staged, bundledVersion, quarantined = [], platform = process.platform }) {
  if (!staged || typeof staged !== 'object') {
    return { use: 'bundled', prune: false, reason: 'no staged core' };
  }
  if (!isValidVersion(staged.version) || !isValidPointerPath(staged.path, platform)) {
    return { use: 'bundled', prune: true, reason: 'staged pointer is malformed' };
  }
  if (typeof staged.sha256 !== 'string' || !/^[0-9a-f]{64}$/.test(staged.sha256)) {
    return { use: 'bundled', prune: true, reason: 'staged pointer has no usable digest' };
  }
  if (quarantined.includes(staged.version)) {
    return { use: 'bundled', prune: true, reason: `v${staged.version} previously failed to start` };
  }
  if (!isValidVersion(bundledVersion) || compareVersions(bundledVersion, staged.version) >= 0) {
    // A full installer overtook the staged core: the bundle is authoritative.
    return { use: 'bundled', prune: true, reason: 'the installed version is not older' };
  }
  return { use: 'staged', prune: false, reason: `staged core v${staged.version}` };
}

async function readJson(filePath) {
  try {
    return JSON.parse(await fsp.readFile(filePath, 'utf8'));
  } catch (_) {
    return null;
  }
}

async function readVersionList(coreDir, file) {
  const data = await readJson(path.join(coreDir, file));
  return Array.isArray(data?.versions) ? data.versions.filter(isValidVersion) : [];
}

async function writeVersionList(coreDir, file, versions) {
  await fsp.mkdir(coreDir, { recursive: true });
  await fsp.writeFile(
    path.join(coreDir, file),
    JSON.stringify({ versions: versions.slice(-8) }, null, 2)
  );
}

async function readQuarantine(coreDir) {
  return readVersionList(coreDir, QUARANTINE_FILE);
}

/** Staged versions that have already started successfully at least once. */
async function readAdoptedCores(coreDir) {
  return readVersionList(coreDir, ADOPTED_FILE);
}

/**
 * Record that a staged core actually came up. The caller does this once the
 * backend reports ready, so the launch that adopts an update is the only one
 * that presents itself as an update.
 */
async function markCoreAdopted(coreDir, version) {
  if (!isValidVersion(version)) return;
  const existing = await readAdoptedCores(coreDir);
  if (existing.includes(version)) return;
  existing.push(version);
  try {
    await writeVersionList(coreDir, ADOPTED_FILE, existing);
  } catch (err) {
    console.error('[Electron] Could not record the adopted core:', err.message);
  }
}

async function sha256File(filePath) {
  return new Promise((resolve, reject) => {
    const hash = crypto.createHash('sha256');
    const stream = fs.createReadStream(filePath);
    stream.on('data', (chunk) => hash.update(chunk));
    stream.on('error', reject);
    stream.on('end', () => resolve(hash.digest('hex')));
  });
}

/**
 * Resolve the binary to launch.
 *
 * `firstRun` is true only when the staged core has never reported ready before,
 * which is what makes this launch an actual update rather than an ordinary start
 * of the version already in use.
 *
 * @param {{coreDir:string, bundledPath:string, bundledVersion:string}} options
 * @returns {Promise<{path:string, version:string|null, staged:boolean, firstRun:boolean, reason:string}>}
 */
async function resolveCore({ coreDir, bundledPath, bundledVersion }) {
  const bundled = {
    path: bundledPath,
    version: bundledVersion,
    staged: false,
    firstRun: false,
    reason: ''
  };

  const staged = await readJson(path.join(coreDir, POINTER_FILE));
  const quarantined = await readQuarantine(coreDir);
  const decision = chooseCore({ staged, bundledVersion, quarantined });

  if (decision.prune) {
    await pruneStagedCores(coreDir, null);
  }
  if (decision.use === 'bundled') {
    return { ...bundled, reason: decision.reason };
  }

  const stagedPath = path.join(coreDir, staged.path);
  try {
    const stat = await fsp.stat(stagedPath);
    if (!stat.isFile() || (typeof staged.size === 'number' && stat.size !== staged.size)) {
      return { ...bundled, reason: 'staged core has the wrong size' };
    }
    if (await sha256File(stagedPath) !== staged.sha256) {
      await quarantineStagedCore(coreDir, staged.version);
      return { ...bundled, reason: 'staged core failed its digest check' };
    }
  } catch (err) {
    return { ...bundled, reason: `staged core unreadable (${err.message})` };
  }

  const adopted = await readAdoptedCores(coreDir);
  return {
    path: stagedPath,
    version: staged.version,
    staged: true,
    firstRun: !adopted.includes(staged.version),
    reason: decision.reason
  };
}

/**
 * Record a staged version as unusable and drop the pointer, so the next launch
 * falls back to the bundled binary and the updater will not re-stage it.
 */
async function quarantineStagedCore(coreDir, version) {
  if (!isValidVersion(version)) return;
  const existing = await readQuarantine(coreDir);
  if (!existing.includes(version)) existing.push(version);
  try {
    await writeVersionList(coreDir, QUARANTINE_FILE, existing);
    await fsp.rm(path.join(coreDir, POINTER_FILE), { force: true });
  } catch (err) {
    console.error('[Electron] Could not quarantine staged core:', err.message);
  }
}

/**
 * Remove staged core trees other than `keep`. Only version-shaped directory
 * names are ever touched, so nothing else under the data directory is at risk.
 */
async function pruneStagedCores(coreDir, keep) {
  let entries;
  try {
    entries = await fsp.readdir(coreDir, { withFileTypes: true });
  } catch (_) {
    return;
  }
  await Promise.all(entries.map(async (entry) => {
    if (!entry.isDirectory()) return;
    const stale = entry.name.startsWith('.staging-');
    if (entry.name === keep || (!isValidVersion(entry.name) && !stale)) return;
    try {
      await fsp.rm(path.join(coreDir, entry.name), { recursive: true, force: true });
    } catch (_) { /* another launch may already have removed it */ }
  }));
  if (!keep) {
    // The adoption record only describes the stage that is going away; leaving
    // it behind would suppress the announcement of a future staged core that
    // happens to reuse one of those version numbers.
    await Promise.all([POINTER_FILE, ADOPTED_FILE].map((file) =>
      fsp.rm(path.join(coreDir, file), { force: true }).catch(() => {})
    ));
  }
}

module.exports = {
  chooseCore,
  compareVersions,
  coreBinaryName,
  isValidPointerPath,
  isValidVersion,
  markCoreAdopted,
  pruneStagedCores,
  quarantineStagedCore,
  readAdoptedCores,
  resolveCore,
};
