const path = require('path');

// Platform-specific binary name
const isWindows = process.platform === 'win32';
const binaryName = isWindows ? 'screenerbot.exe' : 'screenerbot';

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
    {
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
          homepage: 'https://screenerbot.io'
        }
      }
    },
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
        ui: {
          chooseDirectory: true, // Allow user to choose install directory
          images: {
            background: path.join(__dirname, 'assets', 'icon.png'),
            banner: path.join(__dirname, 'assets', 'icon.png')
          }
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
