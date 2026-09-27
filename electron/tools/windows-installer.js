#!/usr/bin/env node

const fs = require('fs');
const { APP_ID } = require('../src/app_identity');

// These identities shipped in v0.2.8. Keeping them stable lets every later
// MSI replace that release instead of registering an unrelated installation.
const UPGRADE_CODES = Object.freeze({
  x64: '9E5F6612-4722-4F9B-A0DA-927F09608DA4',
  arm64: '7C52B2B5-EF09-45D9-A1BE-19A4B5BDBD90',
});

function assertSupportedArchitecture(architecture) {
  if (!Object.hasOwn(UPGRADE_CODES, architecture)) {
    throw new Error(`Unsupported Windows installer architecture: ${architecture}`);
  }
  return architecture;
}

function replaceOnce(source, search, replacement, invariant) {
  const occurrences = source.split(search).length - 1;
  if (occurrences !== 1) {
    throw new Error(
      `Windows installer template changed: expected one ${invariant}, found ${occurrences}`,
    );
  }
  return source.replace(search, replacement);
}

/** Apply the product invariants that electron-wix-msi does not expose as options. */
function customizeWindowsInstaller(creator) {
  let template = creator.wixTemplate.replace(/[ \t]+(?=\r?$)/gm, '').replace(/\r\n/g, '\n');
  template = replaceOnce(
    template,
    `    <!-- Lets cleanup any files that are were not part of the initial install
    via this MSI. Such as newer versions installed by the auto-updater. -->
    <DirectoryRef Id="APPLICATIONROOTDIRECTORY">
      <Component Id="PurgeOnUninstall" Guid="{{RandomGuid}}" Win64="{{Win64YesNo}}">
        <CreateFolder/>
        <util:RemoveFolderEx On="uninstall" Property="INSTALLPATH" />
      </Component>
    </DirectoryRef>

`,
    '',
    'recursive uninstall purge',
  );
  template = replaceOnce(
    template,
    '        <ComponentRef Id="PurgeOnUninstall" />\n',
    '',
    'recursive purge component reference',
  );
  template = replaceOnce(
    template,
    `    <!-- Necessary registry search to find the install path which is used by the
    PurgeOnUninstall action. Since this package can be installed perUser or perMachine,
    we have to look in both places. First successful search wins. -->
    <Property Id="INSTALLPATH">
      <RegistrySearch Key="SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{{{ProductCode}}}.msq"
                      Root="HKCU"
                      Type="raw"
                      Id="INSTALLPATH_REGSEARCH_HKCU"
                      Name="InstallPath"
                      Win64="{{Win64YesNo}}"/>
      <RegistrySearch Key="SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{{{ProductCode}}}.msq"
                      Root="HKLM"
                      Type="raw"
                      Id="INSTALLPATH_REGSEARCH_HKLM"
                      Name="InstallPath"
                      Win64="{{Win64YesNo}}"/>
    </Property>
`,
    '',
    'recursive purge install path lookup',
  );
  template = replaceOnce(
    template,
    'InstallerVersion="405"',
    'InstallerVersion="500"',
    'InstallerVersion attribute',
  );
  template = replaceOnce(
    template,
    '<Property Id="VisibleProductName" Value="{{ApplicationName}} (Machine)" />',
    '<Property Id="VisibleProductName" Value="{{ApplicationName}}" />',
    'per-machine display name',
  );
  template = replaceOnce(
    template,
    'Value="{{ApplicationName}} (User)">',
    'Value="{{ApplicationName}}">',
    'per-user display name',
  );
  creator.wixTemplate = template;
}

function windowsInstallerIdentity(architecture) {
  const supportedArchitecture = assertSupportedArchitecture(architecture);
  return {
    appUserModelId: APP_ID,
    arch: supportedArchitecture,
    defaultInstallMode: 'perMachine',
    upgradeCode: UPGRADE_CODES[supportedArchitecture],
    beforeCreate: customizeWindowsInstaller,
  };
}

function requireMatch(source, pattern, description) {
  if (!pattern.test(source)) {
    throw new Error(`Invalid Windows installer source: ${description}`);
  }
}

/** Verify the fully rendered WiX source immediately before release packaging. */
function verifyWindowsInstallerSource(source, architecture) {
  const supportedArchitecture = assertSupportedArchitecture(architecture);
  const upgradeCode = UPGRADE_CODES[supportedArchitecture];

  requireMatch(
    source,
    /<Product\b[\s\S]*?Name\s*=\s*"ScreenerBot \(Machine - MSI\)"/,
    'hidden MSI identity is missing',
  );
  requireMatch(
    source,
    new RegExp(`UpgradeCode="\\{?${upgradeCode}\\}?"`, 'i'),
    'upgrade identity changed',
  );
  requireMatch(
    source,
    new RegExp(`Platform="${supportedArchitecture}"`, 'i'),
    'package architecture is incorrect',
  );
  requireMatch(source, /InstallerVersion="500"/, 'Windows Installer schema must be version 500');
  requireMatch(source, /InstallScope="perMachine"/, 'installation scope changed');
  requireMatch(source, /<MajorUpgrade\b/, 'major-upgrade handling is missing');
  if (/<(?:util:)?RemoveFolderEx\b|\bPurgeOnUninstall\b|<Property Id="INSTALLPATH"/.test(source)) {
    throw new Error('Invalid Windows installer source: recursive uninstall purge is present');
  }
  if (/\bConfigurableDirectory=/.test(source)) {
    throw new Error('Invalid Windows installer source: custom install directory is enabled');
  }
  requireMatch(
    source,
    /<Property Id="VisibleProductName" Value="ScreenerBot" \/>/,
    'Apps display name must be ScreenerBot',
  );
  requireMatch(
    source,
    /<SetProperty\b[^>]*Id="VisibleProductName"[^>]*Value="ScreenerBot">/,
    'per-user fallback display name must be ScreenerBot',
  );

  if (/VisibleProductName[^\n]*(?:\(Machine\)|\(User\))/.test(source)) {
    throw new Error('Invalid Windows installer source: scope leaked into the Apps display name');
  }
}

if (require.main === module) {
  const [command, sourcePath, architecture] = process.argv.slice(2);
  if (command !== '--verify' || !sourcePath || !architecture) {
    process.stderr.write(
      'Usage: node tools/windows-installer.js --verify <installer.wxs> <x64|arm64>\n',
    );
    process.exitCode = 2;
  } else {
    verifyWindowsInstallerSource(fs.readFileSync(sourcePath, 'utf8'), architecture);
    process.stdout.write(`Verified ${architecture} Windows installer metadata\n`);
  }
}

module.exports = {
  APP_ID,
  UPGRADE_CODES,
  customizeWindowsInstaller,
  verifyWindowsInstallerSource,
  windowsInstallerIdentity,
};
