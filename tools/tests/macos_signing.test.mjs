// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//
// Regression coverage for ad-hoc Electron library loading and packaged signature policy.

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const require = createRequire(path.join(ROOT, "electron", "package.json"));
const plist = require("plist");
const {
  macosSigningOptions,
  verifyProcessSignature,
  probeElectronLoader,
} = require("./tools/macos-signing.js");
const LIBRARY_VALIDATION = "com.apple.security.cs.disable-library-validation";
const ALLOW_JIT = "com.apple.security.cs.allow-jit";

test("Forge applies the macOS signing owner and fails signing errors", () => {
  const config = require("./forge.config.js");
  assert.equal(config.packagerConfig.osxSign.continueOnError, false);
  assert.equal(macosSigningOptions().identity, "-");
  assert.equal(macosSigningOptions().identityValidation, false);
  assert.equal(macosSigningOptions("-").identityValidation, false);
});

test("ad-hoc process entitlements preserve each upstream permission and permit framework loading", () => {
  const options = macosSigningOptions();
  const upstream = path.join(ROOT, "electron/node_modules/@electron/osx-sign/entitlements");
  const cases = [
    ["ScreenerBot.app", "default.darwin.plist"],
    ["ScreenerBot Helper.app", "default.darwin.plist"],
    ["ScreenerBot Helper (GPU).app", "default.darwin.gpu.plist"],
    ["ScreenerBot Helper (Renderer).app", "default.darwin.renderer.plist"],
    ["ScreenerBot Helper (Plugin).app", "default.darwin.plugin.plist"],
  ];
  for (const [name, template] of cases) {
    const perFile = options.optionsForFile(
      `/bundle/Contents/Frameworks/${name}/Contents/MacOS/helper`
    );
    const actual = plist.parse(fs.readFileSync(perFile.entitlements, "utf8"));
    const expected = plist.parse(fs.readFileSync(path.join(upstream, template), "utf8"));
    assert.equal(perFile.hardenedRuntime, true, name);
    assert.deepEqual(actual, { ...expected, [LIBRARY_VALIDATION]: true }, name);
    verifyProcessSignature(
      { adhoc: true, runtime: true, entitlements: actual, plugin: name.includes("(Plugin)") },
      null
    );
  }
});

test("Developer ID signing retains upstream policy and identity validation", () => {
  const options = macosSigningOptions("Developer ID Application: Example (ABCDEFGHIJ)");
  assert.equal(options.identityValidation, true);
  assert.equal(options.continueOnError, false);
  assert.equal(options.optionsForFile, undefined);
});

test("valid ad-hoc seals without the library exception are refused", () => {
  const signature = { adhoc: true, runtime: true, entitlements: { [ALLOW_JIT]: true } };
  assert.throws(() => verifyProcessSignature(signature, null), /library-validation exception/);
  assert.throws(
    () =>
      verifyProcessSignature(
        { ...signature, entitlements: { ...signature.entitlements, [LIBRARY_VALIDATION]: false } },
        null
      ),
    /library-validation exception/
  );
});

test("verification requires hardened runtime and the JIT permission", () => {
  const signature = {
    adhoc: true,
    runtime: true,
    entitlements: { [ALLOW_JIT]: true, [LIBRARY_VALIDATION]: true },
  };
  assert.throws(
    () => verifyProcessSignature({ ...signature, runtime: false }, null),
    /Hardened runtime/
  );
  assert.throws(
    () =>
      verifyProcessSignature({ ...signature, entitlements: { [LIBRARY_VALIDATION]: true } }, null),
    /JIT entitlement/
  );
});

test("Developer ID process signatures must match the framework team", () => {
  const signature = {
    adhoc: false,
    runtime: true,
    team: "ABCDEFGHIJ",
    entitlements: { [ALLOW_JIT]: true },
  };
  verifyProcessSignature(signature, "ABCDEFGHIJ");
  assert.throws(() => verifyProcessSignature(signature, "OTHERTEAM1"), /share a signing Team ID/);
  assert.throws(
    () => verifyProcessSignature({ ...signature, team: null }, null),
    /share a signing Team ID/
  );
});

test("plugin verification preserves the executable-memory permission", () => {
  const signature = {
    adhoc: true,
    runtime: true,
    plugin: true,
    entitlements: { [LIBRARY_VALIDATION]: true },
  };
  assert.throws(() => verifyProcessSignature(signature, null), /executable-memory entitlement/);
  verifyProcessSignature(
    {
      ...signature,
      entitlements: {
        ...signature.entitlements,
        "com.apple.security.cs.allow-unsigned-executable-memory": true,
      },
    },
    null
  );
});

test("the macOS release gate verifies the actual package and architecture", () => {
  const workflow = fs.readFileSync(path.join(ROOT, ".github/workflows/build.yml"), "utf8");
  assert.ok(
    workflow.includes(
      'node electron/tools/macos-signing.js --verify "$APP" "${{ matrix.electron-arch }}"'
    )
  );
  const packaging = workflow.slice(
    workflow.indexOf("  package-macos:"),
    workflow.indexOf("  package-linux:")
  );
  assert.match(packaging, /os: macos-\d+-intel\s+arch: x64/);
});

test("the packaged loader refuses a non-native runner before spawning Electron", () => {
  const foreignArchitecture = process.arch === "x64" ? "arm64" : "x64";
  assert.throws(
    () => probeElectronLoader("/unused-package", foreignArchitecture),
    /requires a native .* runner/
  );
});
