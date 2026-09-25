import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const {
  APP_ID,
  UPGRADE_CODES,
  customizeWindowsInstaller,
  verifyWindowsInstallerSource,
  windowsInstallerIdentity,
} = require("../../electron/tools/windows-installer.js");

const BASE_TEMPLATE = `
<Wix>
  <Product UpgradeCode="{{UpgradeCode}}" Name = "{{ApplicationName}} (Machine - MSI)">
    <Package InstallerVersion="405" Platform="{{Platform}}" InstallScope="{{PackageScope}}"/>
    <MajorUpgrade AllowSameVersionUpgrades="yes" />
    <Property Id="VisibleProductName" Value="{{ApplicationName}} (Machine)" />
    <SetProperty Action="SetVisibleProductName" Id="VisibleProductName" Value="{{ApplicationName}} (User)">
      MSIINSTALLPERUSER = "1"
    </SetProperty>
  </Product>
</Wix>`;

function renderedSource(architecture) {
  const identity = windowsInstallerIdentity(architecture);
  const creator = { wixTemplate: BASE_TEMPLATE };
  identity.beforeCreate(creator);
  return creator.wixTemplate
    .replaceAll("{{ApplicationName}}", "ScreenerBot")
    .replaceAll("{{UpgradeCode}}", identity.upgradeCode)
    .replaceAll("{{Platform}}", identity.arch)
    .replaceAll("{{PackageScope}}", identity.defaultInstallMode);
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

test("the public Apps entry omits installer scope", () => {
  for (const architecture of ["x64", "arm64"]) {
    const source = renderedSource(architecture);
    verifyWindowsInstallerSource(source, architecture);
    assert.match(source, /VisibleProductName" Value="ScreenerBot"/);
    assert.doesNotMatch(source, /VisibleProductName[^\n]*(?:\(Machine\)|\(User\))/);
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
    () => customizeWindowsInstaller({ wixTemplate: BASE_TEMPLATE.replace(" (Machine)", "") }),
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
