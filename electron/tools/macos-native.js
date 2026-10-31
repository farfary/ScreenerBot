// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Compiles the macOS native window-button helper into one universal (x64 + arm64) Node-API module.

const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const { NATIVE_HELPER_PATH } = require('../src/window_controls');

const SOURCE = path.join(__dirname, '..', 'native', 'window_buttons.m');

/**
 * The Node-API headers of the node running this build. Official Node
 * distributions (and actions/setup-node) ship them under `<prefix>/include/node`.
 */
function nodeApiIncludeDirectory(execPath = process.execPath) {
  const directory = path.join(path.dirname(execPath), '..', 'include', 'node');
  if (!fs.existsSync(path.join(directory, 'node_api.h'))) {
    throw new Error(`Node-API headers not found at ${directory}; build with an official Node distribution`);
  }
  return directory;
}

/**
 * The clang invocation. Both architectures go into one file, so the helper is
 * identical for the x64 and arm64 packages; the deployment target is the
 * bundle's own minimum macOS version.
 */
function clangArguments({ minimumVersion, includeDirectory, output = NATIVE_HELPER_PATH }) {
  return [
    '-x', 'objective-c',
    '-fobjc-arc',
    '-O2',
    '-Wall',
    '-bundle',
    '-undefined', 'dynamic_lookup',
    '-arch', 'x86_64',
    '-arch', 'arm64',
    `-mmacosx-version-min=${minimumVersion}`,
    '-framework', 'AppKit',
    '-I', includeDirectory,
    '-o', output,
    SOURCE,
  ];
}

/**
 * Build the helper on macOS; elsewhere there is nothing to build.
 *
 * @param {{ minimumVersion: string }} options
 * @returns {string | null} the built module's path
 */
function buildMacosNative({ minimumVersion }) {
  if (process.platform !== 'darwin') return null;
  fs.mkdirSync(path.dirname(NATIVE_HELPER_PATH), { recursive: true });
  execFileSync('clang', clangArguments({ minimumVersion, includeDirectory: nodeApiIncludeDirectory() }), {
    stdio: 'inherit',
  });
  return NATIVE_HELPER_PATH;
}

module.exports = { buildMacosNative, clangArguments, nodeApiIncludeDirectory };
