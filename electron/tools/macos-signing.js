#!/usr/bin/env node
// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// macOS signing policy and packaged loader verification for ad-hoc and Developer ID distributions.

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const { APP_ID } = require('../src/app_identity');

const LIBRARY_VALIDATION = 'com.apple.security.cs.disable-library-validation';
const ALLOW_JIT = 'com.apple.security.cs.allow-jit';
const ENTITLEMENTS = path.join(__dirname, '..', 'assets', 'entitlements');

/** Ad-hoc code has no Team ID, so every Electron process must opt out of library validation. */
function macosSigningOptions(identity) {
  const signingIdentity = identity || '-';
  const adhoc = signingIdentity === '-';
  return {
    identity: signingIdentity,
    identityValidation: !adhoc,
    continueOnError: false,
    ...(adhoc ? {
      optionsForFile: (filePath) => {
        const normalized = filePath.split(path.sep).join('/');
        const template = normalized.includes('(Plugin).app') ? 'plugin'
          : /\((GPU|Renderer)\)\.app/.test(normalized) ? 'jit' : 'default';
        return { hardenedRuntime: true, entitlements: path.join(ENTITLEMENTS, `${template}.plist`) };
      },
    } : {}),
  };
}

/** Validate runtime policy separately from the on-disk signature seal. */
function verifyProcessSignature(signature, frameworkTeam) {
  if (!signature.runtime) throw new Error('Hardened runtime is missing');
  if (signature.adhoc) {
    if (signature.entitlements[LIBRARY_VALIDATION] !== true) {
      throw new Error('Ad-hoc Electron process requires the library-validation exception');
    }
  } else if (!signature.team || signature.team !== frameworkTeam) {
    throw new Error('Electron process and framework must share a signing Team ID');
  }
  if (!signature.plugin && signature.entitlements[ALLOW_JIT] !== true) {
    throw new Error('Electron process requires the JIT entitlement');
  }
  if (signature.plugin && signature.entitlements['com.apple.security.cs.allow-unsigned-executable-memory'] !== true) {
    throw new Error('Electron plugin process requires its executable-memory entitlement');
  }
}

function readSignature(target) {
  // codesign writes signature metadata to stderr even on success.
  const { spawnSync } = require('child_process');
  const details = spawnSync('codesign', ['-d', '--verbose=4', target], { encoding: 'utf8' });
  if (details.error || details.status !== 0) {
    throw new Error(`Cannot inspect signature of ${target}: ${details.error || details.stderr}`);
  }
  const metadata = details.stderr;
  const flags = metadata.match(/\bflags=0x([0-9a-f]+)/i);
  if (!flags) throw new Error(`Missing CodeDirectory flags for ${target}`);
  const team = metadata.match(/^TeamIdentifier=(.+)$/m)?.[1];
  return {
    identifier: metadata.match(/^Identifier=(.+)$/m)?.[1],
    adhoc: (parseInt(flags[1], 16) & 0x2) !== 0,
    runtime: (parseInt(flags[1], 16) & 0x10000) !== 0,
    team: team && team !== 'not set' ? team : null,
  };
}

function readEntitlements(target) {
  const xml = execFileSync('codesign', ['-d', '--entitlements', '-', '--xml', target], { encoding: 'utf8' });
  return JSON.parse(execFileSync('plutil', ['-convert', 'json', '-o', '-', '--', '-'], {
    input: xml,
    encoding: 'utf8',
  }));
}

/** Exercise dyld and the shipped Electron runtime without running app code or starting the core. */
function probeElectronLoader(appPath, architecture) {
  if (process.arch !== architecture) {
    throw new Error(`Packaged Electron loader requires a native ${architecture} runner; current architecture is ${process.arch}`);
  }
  const executable = path.join(appPath, 'Contents', 'MacOS', 'ScreenerBot');
  const env = Object.fromEntries(Object.entries(process.env).filter(([key]) =>
    !key.startsWith('DYLD_') && key !== 'NODE_OPTIONS'));
  const output = execFileSync(executable, [
    '-e', 'process.stdout.write(JSON.stringify({arch:process.arch,electron:process.versions.electron}))',
  ], {
    env: { ...env, ELECTRON_RUN_AS_NODE: '1' },
    encoding: 'utf8',
    timeout: 30000,
  });
  const runtime = JSON.parse(output);
  if (runtime.arch !== architecture || !runtime.electron) {
    throw new Error(`Unexpected packaged Electron runtime: ${output}`);
  }
  return runtime;
}

function verifyMacosApp(appPath, architecture) {
  if (!['x64', 'arm64'].includes(architecture)) throw new Error('Expected x64 or arm64 architecture');
  execFileSync('codesign', ['--verify', '--deep', '--strict', appPath]);
  execFileSync('codesign', ['--verify', '--strict', path.join(appPath, 'Contents', 'Resources', 'screenerbot')]);
  const frameworks = path.join(appPath, 'Contents', 'Frameworks');
  const framework = readSignature(path.join(frameworks, 'Electron Framework.framework'));
  const helperNames = fs.readdirSync(frameworks).filter((name) => name.endsWith('.app'));
  for (const suffix of ['', ' (GPU)', ' (Renderer)', ' (Plugin)']) {
    if (!helperNames.includes(`ScreenerBot Helper${suffix}.app`)) {
      throw new Error(`Missing Electron helper: ${suffix || 'standard'}`);
    }
  }
  for (const target of [appPath, ...helperNames.map((name) => path.join(frameworks, name))]) {
    const signature = { ...readSignature(target), entitlements: readEntitlements(target), plugin: target.includes('(Plugin).app') };
    if (target === appPath && signature.identifier !== APP_ID) {
      throw new Error(`Unexpected app identifier: ${signature.identifier}`);
    }
    if (signature.adhoc !== framework.adhoc) throw new Error('Mixed Electron signing identities');
    verifyProcessSignature(signature, framework.team);
  }
  return probeElectronLoader(appPath, architecture);
}

if (require.main === module) {
  const [command, appPath, architecture] = process.argv.slice(2);
  if (command !== '--verify' || !appPath || !architecture) {
    process.stderr.write('Usage: node tools/macos-signing.js --verify <app> <x64|arm64>\n');
    process.exitCode = 2;
  } else {
    const runtime = verifyMacosApp(path.resolve(appPath), architecture);
    process.stdout.write(`Verified macOS signatures, process entitlements and ${runtime.arch} Electron ${runtime.electron} loader\n`);
  }
}

module.exports = { macosSigningOptions, verifyProcessSignature, verifyMacosApp, probeElectronLoader };
