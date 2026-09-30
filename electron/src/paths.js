// Filesystem locations shared with the Rust core.
//
// The backend resolves its own base directory in src/paths/mod.rs. Electron has
// to agree with it exactly — the staged-core pointer, the logs and the data tree
// are all written by one process and read by the other — and `app.getPath('userData')`
// does NOT agree outside macOS: Electron uses %APPDATA% (Roaming) and ~/.config,
// while the core uses %LOCALAPPDATA% and ~/.local/share. So the rule is mirrored
// here rather than approximated.
//
// Electron's own userData path is still the right home for window-state.json:
// that file belongs to the shell, not to the trading data.

const path = require('path');
const os = require('os');

const APP_DIR = 'ScreenerBot';

/**
 * Base directory for all ScreenerBot data, matching `paths::get_base_directory()`.
 * @param {string=} homeDir override, for tests
 * @param {NodeJS.Platform=} platform override, for tests
 * @param {NodeJS.ProcessEnv=} env override, for tests
 */
function resolveBaseDirectory(homeDir = os.homedir(), platform = process.platform, env = process.env) {
  const override = (env.SCREENERBOT_DATA_DIR || '').trim();
  if (override) return override;

  if (platform === 'darwin') {
    return path.join(homeDir, 'Library', 'Application Support', APP_DIR);
  }
  if (platform === 'win32') {
    const localAppData = (env.LOCALAPPDATA || '').trim() || path.join(homeDir, 'AppData', 'Local');
    return path.join(localAppData, APP_DIR);
  }
  const xdgDataHome = (env.XDG_DATA_HOME || '').trim() || path.join(homeDir, '.local', 'share');
  return path.join(xdgDataHome, APP_DIR);
}

/** Databases, config.toml, the update state and the staged cores. */
function dataDirectory(base = resolveBaseDirectory()) {
  return path.join(base, 'data');
}

/** Where the updater stages verified core binaries. */
function coreDirectory(base = resolveBaseDirectory()) {
  return path.join(dataDirectory(base), 'core');
}

/** Rotating log files written by the core. */
function logsDirectory(base = resolveBaseDirectory()) {
  return path.join(base, 'logs');
}

/**
 * Localization resources shared with the backend and the dashboard.
 *
 * Packaged builds receive the catalog directory and the vendored Fluent bundle
 * through `extraResource` (forge.config.js), which places each under Resources
 * by its own name. A development run reads them from the repository.
 * @param {{ isPackaged: boolean, resourcesPath?: string }} runtime
 */
function l10nResources({ isPackaged, resourcesPath }) {
  if (isPackaged) {
    return {
      localesDir: path.join(resourcesPath, 'locales'),
      bundlePath: path.join(resourcesPath, 'fluent-bundle.js'),
    };
  }
  const repoRoot = path.join(__dirname, '..', '..');
  return {
    localesDir: path.join(repoRoot, 'locales'),
    bundlePath: path.join(repoRoot, 'src', 'webserver', 'assets', 'fluent-bundle.js'),
  };
}

module.exports = {
  APP_DIR,
  l10nResources,
  resolveBaseDirectory,
  dataDirectory,
  coreDirectory,
  logsDirectory,
};
