import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const forgeConfigPath = fileURLToPath(new URL("../../electron/forge.config.js", import.meta.url));
const requireFromElectron = createRequire(forgeConfigPath);
const MSI_TEMPLATE = readFileSync(
  join(dirname(requireFromElectron.resolve("electron-wix-msi/lib/creator")), "../static/wix.xml"),
  "utf8",
);
const {
  APP_ID,
  UPGRADE_CODES,
  customizeWindowsInstaller,
  verifyWindowsInstallerSource,
  windowsInstallerIdentity,
} = require("../../electron/tools/windows-installer.js");

function renderedSource(architecture) {
  const identity = windowsInstallerIdentity(architecture);
  const creator = { wixTemplate: MSI_TEMPLATE };
  identity.beforeCreate(creator);
  return creator.wixTemplate
    .replaceAll("{{ApplicationName}}", "ScreenerBot")
    .replaceAll("{{UpgradeCode}}", identity.upgradeCode)
    .replaceAll("{{Platform}}", identity.arch)
    .replaceAll("{{PackageScope}}", identity.defaultInstallMode)
    .replaceAll("{{ConfigurableDirectory}}", "");
}

test("Windows installer identities remain stable and architecture-specific", () => {
  assert.deepEqual(windowsInstallerIdentity("x64"), {
    appUserModelId: APP_ID,
    arch: "x64",
    defaultInstallMode: "perMachine",
    upgradeCode: UPGRADE_CODES.x64,
    beforeCreate: customizeWindowsInstaller,
  });
  assert.deepEqual(windowsInstallerIdentity("arm64"), {
    appUserModelId: APP_ID,
    arch: "arm64",
    defaultInstallMode: "perMachine",
    upgradeCode: UPGRADE_CODES.arm64,
    beforeCreate: customizeWindowsInstaller,
  });
  assert.notEqual(UPGRADE_CODES.x64, UPGRADE_CODES.arm64);
});

test("Forge uses the requested Windows architecture for MSI and bundled runtime", () => {
  for (const architecture of ["x64", "arm64"]) {
    const inspectConfig = `
      Object.defineProperty(process, 'platform', { value: 'win32' });
      const config = require(process.argv[1]);
      const wix = config.makers.find((maker) => maker.name === '@electron-forge/maker-wix');
      process.stdout.write(JSON.stringify({
        arch: wix.config.arch,
        upgradeCode: wix.config.upgradeCode,
        chooseDirectory: wix.config.ui.chooseDirectory,
        runtime: config.packagerConfig.extraResource.find((resource) => resource.includes('vc_redist.')),
      }));
    `;
    const config = JSON.parse(execFileSync(process.execPath, ["-e", inspectConfig, forgeConfigPath], {
      env: { ...process.env, ELECTRON_FORGE_ARCH: architecture },
      encoding: "utf8",
    }));
    assert.equal(config.arch, architecture);
    assert.equal(config.upgradeCode, UPGRADE_CODES[architecture]);
    assert.ok(config.runtime.endsWith(`vc_redist.${architecture}.exe`));
    assert.equal(config.chooseDirectory, false);
  }
});

test("the public Apps entry omits installer scope", () => {
  for (const architecture of ["x64", "arm64"]) {
    const source = renderedSource(architecture);
    verifyWindowsInstallerSource(source, architecture);
    assert.match(source, /VisibleProductName" Value="ScreenerBot"/);
    assert.doesNotMatch(source, /VisibleProductName[^\n]*(?:\(Machine\)|\(User\))/);
  }
});

test("installer omits recursive uninstall cleanup", () => {
  for (const architecture of ["x64", "arm64"]) {
    const source = renderedSource(architecture);
    assert.doesNotMatch(source, /RemoveFolderEx|PurgeOnUninstall|<Property Id="INSTALLPATH"/);
    assert.doesNotMatch(source, /ConfigurableDirectory=/);
    assert.throws(
      () => verifyWindowsInstallerSource(`${source}<util:RemoveFolderEx On="uninstall" />`, architecture),
      /recursive uninstall purge/,
    );
  }
});

test("ARM64 packages are authored as ARM64 with schema 500", () => {
  const source = renderedSource("arm64");
  assert.match(source, /Platform="arm64"/);
  assert.match(source, /InstallerVersion="500"/);
  assert.doesNotMatch(source, /Platform="x64"/);
});

test("template drift fails before an installer can silently regress", () => {
  assert.throws(
    () => customizeWindowsInstaller({ wixTemplate: MSI_TEMPLATE.replace('<util:RemoveFolderEx On="uninstall"', '<util:RemoveFolderEx On="both"') }),
    /expected one recursive uninstall purge, found 0/,
  );
  assert.throws(
    () => customizeWindowsInstaller({ wixTemplate: MSI_TEMPLATE.replace(" (Machine)", "") }),
    /expected one per-machine display name, found 0/,
  );
});

test("release verification rejects the wrong architecture or upgrade identity", () => {
  const source = renderedSource("arm64");
  assert.throws(() => verifyWindowsInstallerSource(source, "x64"), /upgrade identity changed/);
  assert.throws(
    () =>
      verifyWindowsInstallerSource(
        source.replace(UPGRADE_CODES.arm64, "00000000-0000-0000-0000-000000000000"),
        "arm64",
      ),
    /upgrade identity changed/,
  );
  assert.throws(
    () => windowsInstallerIdentity("ia32"),
    /Unsupported Windows installer architecture/,
  );
});
