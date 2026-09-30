'use strict';

// Localized text for the Electron shell (tray, menus, native dialogs, splash).
//
// The catalogs are the same Fluent files the backend and the dashboard use:
// `locales/<code>/desktop.ftl` plus that locale's `terms.ftl`, with `en` as the
// per-message fallback. This module never imports `electron`, so it can be
// exercised by plain node tests; the caller supplies the resource locations.

const fs = require('fs');
const path = require('path');

const SOURCE_LOCALE = 'en';
const DOMAIN_FILES = ['terms.ftl', 'desktop.ftl'];

/**
 * Registered locales from `registry.toml`: `[{ code, dir }]`.
 * @param {string} toml
 */
function parseRegistry(toml) {
  const entries = [];
  for (const block of toml.split(/^\[\[locale\]\]\s*$/m).slice(1)) {
    const code = block.match(/^\s*code\s*=\s*"([^"]+)"/m);
    const dir = block.match(/^\s*dir\s*=\s*"([^"]+)"/m);
    if (code) entries.push({ code: code[1], dir: dir && dir[1] === 'rtl' ? 'rtl' : 'ltr' });
  }
  return entries;
}

/**
 * Resolve an ordered list of preferred language tags to a registered code:
 * exact match, then the language subtag alone, else the source locale.
 * @param {string[]} preferred
 * @param {string[]} codes registered codes
 */
function negotiateLocale(preferred, codes) {
  const byLower = new Map(codes.map((code) => [code.toLowerCase(), code]));
  for (const tag of Array.isArray(preferred) ? preferred : []) {
    if (typeof tag !== 'string') continue;
    const normalized = tag.trim().replace(/_/g, '-').toLowerCase();
    if (!normalized) continue;
    if (byLower.has(normalized)) return byLower.get(normalized);
    const language = normalized.split('-')[0];
    if (byLower.has(language)) return byLower.get(language);
    // Region or script variants of the same language ("pt" -> "pt-BR",
    // "zh-CN" -> "zh-Hans"); a registered script must match the requested one.
    const script = scriptOf(normalized);
    const variant = codes.find((code) => {
      const [codeLanguage, ...rest] = code.toLowerCase().split('-');
      if (codeLanguage !== language) return false;
      const codeScript = rest.find((part) => part.length === 4);
      return !codeScript || !script || codeScript === script;
    });
    if (variant) return variant;
  }
  return SOURCE_LOCALE;
}

// Chinese system tags usually carry a region, not a script.
const CHINESE_REGION_SCRIPTS = { cn: 'hans', sg: 'hans', my: 'hans', tw: 'hant', hk: 'hant', mo: 'hant' };

/** Lower-case script subtag of a normalized tag, inferred from the region for Chinese. */
function scriptOf(normalized) {
  const [language, ...rest] = normalized.split('-');
  const explicit = rest.find((part) => part.length === 4);
  if (explicit) return explicit;
  if (language !== 'zh') return null;
  const region = rest.find((part) => part.length === 2);
  return CHINESE_REGION_SCRIPTS[region] || null;
}

/**
 * Evaluate the vendored UMD bundle as a CommonJS module. `require` is avoided
 * because in a development checkout the file sits under the repository's
 * `"type": "module"` package scope, which would load it as an ES module with no
 * exports.
 */
function loadFluent(bundlePath) {
  const module = { exports: {} };
  new Function('module', 'exports', fs.readFileSync(bundlePath, 'utf8')).call(
    module.exports,
    module,
    module.exports
  );
  return module.exports;
}

function readCatalog(localesDir, code, file) {
  try {
    return fs.readFileSync(path.join(localesDir, code, file), 'utf8');
  } catch (_) {
    return null;
  }
}

/**
 * @param {{ localesDir: string, bundlePath: string, locale?: string|null }} options
 *   `locale` is a registered code; anything else resolves to the source locale.
 */
function createLocalizer({ localesDir, bundlePath, locale = null }) {
  const { FluentBundle, FluentResource } = loadFluent(bundlePath);
  const registry = parseRegistry(fs.readFileSync(path.join(localesDir, 'registry.toml'), 'utf8'));
  const codes = registry.map((entry) => entry.code);
  const resolved = codes.includes(locale) ? locale : SOURCE_LOCALE;
  const entry = registry.find((item) => item.code === resolved);

  // Isolation marks are for mixed-direction layout inside a page; native menus
  // and dialogs would show them as stray glyphs.
  function bundleFor(code) {
    const bundle = new FluentBundle(code, { useIsolating: false });
    for (const file of DOMAIN_FILES) {
      const source = readCatalog(localesDir, code, file);
      if (source !== null) bundle.addResource(new FluentResource(source));
    }
    return bundle;
  }

  const primary = bundleFor(resolved);
  const fallback = resolved === SOURCE_LOCALE ? primary : bundleFor(SOURCE_LOCALE);

  function render(bundle, id, args) {
    const message = bundle.getMessage(id);
    if (!message || !message.value) return null;
    return bundle.formatPattern(message.value, args || undefined, []);
  }

  const sourceIds = (readCatalog(localesDir, SOURCE_LOCALE, 'desktop.ftl') || '')
    .split('\n')
    .map((line) => line.match(/^([a-z][a-z0-9-]*)\s*=/))
    .filter(Boolean)
    .map((match) => match[1]);

  return {
    locale: resolved,
    dir: entry ? entry.dir : 'ltr',
    codes,

    /** Message text for `id`; the source locale fills a gap, the id marks a missing message. */
    t(id, args) {
      const text = render(primary, id, args);
      if (text !== null) return text;
      const source = render(fallback, id, args);
      return source !== null ? source : id;
    },

    /** Every source `desktop` message id starting with one of `prefixes`, rendered. */
    stringsWithPrefix(prefixes) {
      const strings = {};
      for (const id of sourceIds) {
        if (prefixes.some((prefix) => id.startsWith(prefix))) strings[id] = this.t(id);
      }
      return strings;
    },

    negotiate(preferred) {
      return negotiateLocale(preferred, codes);
    },
  };
}

module.exports = { createLocalizer, negotiateLocale, parseRegistry, SOURCE_LOCALE };
