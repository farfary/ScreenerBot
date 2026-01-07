const path = require('path');

// Platform-specific binary name
const isWindows = process.platform === 'win32';
const isMacOS = process.platform === 'darwin';
const binaryName = isWindows ? 'screenerbot.exe' : 'screenerbot';

// Detect target architecture for conditional makers
// ELECTRON_FORGE_ARCH is set by electron-forge during make, fallback to process.arch
const targetArch = process.env.ELECTRON_FORGE_ARCH || process.arch;
const isArm64 = targetArch === 'arm64';

// RPM maker has issues on macOS:
// - rpmbuild on macOS doesn't support aarch64 cross-compilation
// - rpmbuild on macOS has path issues with BSD cp command
// Skip RPM maker entirely when building on macOS host
const skipRpm = isMacOS;

module.exports = {
  packagerConfig: {
    asar: true,
    name: 'ScreenerBot',
    executableName: 'ScreenerBot',
    appBundleId: 'io.screenerbot.app',
    appCategoryType: 'public.app-category.finance',
    icon: path.join(__dirname, 'assets', 'icon'),
    extraResource: [
      path.join(__dirname, '..', 'target', 'release', binaryName)
    ],
    // macOS code signing (disabled by default - enable for distribution)
    // osxSign: {},
    // osxNotarize: {
    //   appleId: process.env.APPLE_ID,
    //   appleIdPassword: process.env.APPLE_PASSWORD,
    //   teamId: process.env.APPLE_TEAM_ID,
    // },
    darwinDarkModeSupport: true,
  },
  rebuildConfig: {},
  makers: [
    // =========================================================================
    // macOS Makers
    // =========================================================================
    {
      name: '@electron-forge/maker-zip',
      platforms: ['darwin'],
      config: {}
    },
    {
      name: '@electron-forge/maker-dmg',
      platforms: ['darwin'],
      config: {
        name: 'ScreenerBot',
        icon: path.join(__dirname, 'assets', 'icon.icns'),
        // DMG background is optional - comment out if not present
        background: path.join(__dirname, 'assets', 'dmg-background.png'),
        format: 'UDZO',
        additionalDMGOptions: {
          window: {
            size: {
              width: 600,
              height: 400
            }
          }
        },
        contents: (opts) => [
          { x: 150, y: 200, type: 'file', path: opts.appPath },
          { x: 450, y: 200, type: 'link', path: '/Applications' }
        ]
      }
    },
    // =========================================================================
    // Linux Makers
    // =========================================================================
    {
      name: '@electron-forge/maker-deb',
      platforms: ['linux'],
      config: {
        options: {
          name: 'screenerbot',
          productName: 'ScreenerBot',
          genericName: 'Solana Trading Bot',
          description: 'Automated Solana DeFi trading bot with wallet management',
          categories: ['Finance', 'Utility'],
          icon: path.join(__dirname, 'assets', 'icon.png'),
          maintainer: 'ScreenerBot <support@screenerbot.io>',
          homepage: 'https://screenerbot.io'
        }
      }
    },
    // RPM maker - skip on macOS (rpmbuild has path issues and doesn't support aarch64 cross-compilation)
    // RPM packages will only be built when running on a native Linux host
    ...(skipRpm ? [] : [{
      name: '@electron-forge/maker-rpm',
      platforms: ['linux'],
      config: {
        options: {
          name: 'screenerbot',
          productName: 'ScreenerBot',
          genericName: 'Solana Trading Bot',
          description: 'Automated Solana DeFi trading bot with wallet management',
          categories: ['Finance', 'Utility'],
          icon: path.join(__dirname, 'assets', 'icon.png'),
          license: 'Proprietary',
          homepage: 'https://screenerbot.io',
          vendor: 'unknown',
          platform: 'linux'
        }
      }
    }]),
    {
      name: '@electron-forge/maker-zip',
      platforms: ['linux'],
      config: {}
    },
    // =========================================================================
    // Windows Makers
    // =========================================================================
    {
      name: '@electron-forge/maker-wix',
      platforms: ['win32'],
      config: {
        name: 'ScreenerBot',
        manufacturer: 'ScreenerBot',
        description: 'Automated Solana DeFi trading bot with wallet management',
        language: 1033, // English (United States)
        icon: path.join(__dirname, 'assets', 'icon.ico'),
        // CRITICAL: Set arch to match build target - defaults to x86 if not specified!
        // This determines whether the MSI installs to "Program Files" (x64) or "Program Files (x86)"
        arch: targetArch === 'arm64' ? 'x64' : targetArch, // WIX doesn't support arm64 yet, use x64 for arm64 builds
        ui: {
          chooseDirectory: true, // Allow user to choose install directory
        },
        // Optional: Code signing for Windows
        // certificateFile: process.env.WINDOWS_CERT_FILE,
        // certificatePassword: process.env.WINDOWS_CERT_PASSWORD,
      }
    },
    {
      name: '@electron-forge/maker-zip',
      platforms: ['win32'],
      config: {}
    }
  ],
  plugins: [
    {
      name: '@electron-forge/plugin-auto-unpack-natives',
      config: {}
    }
  ]
};
