// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * The dashboard localization runtime over the English source catalogs, for tests that
 * import dashboard modules directly in Node.
 *
 * The vendored Fluent bundle and `core/i18n.js` run in an isolated vm context fed the
 * English catalogs the way `dashboard_catalog` concatenates them (terms first, server-only
 * domains left out); the resulting `I18n` is installed as the global the modules read.
 */

import fs from "node:fs";
import vm from "node:vm";

import { readServerOnlyDomains } from "../i18n/catalogs.mjs";

const WEBSERVER = new URL("../../src/webserver/", import.meta.url);
const LOCALES = new URL("../../locales/en/", import.meta.url);

function englishCatalog() {
  const serverOnly = readServerOnlyDomains();
  const domains = fs
    .readdirSync(LOCALES)
    .filter((name) => name.endsWith(".ftl"))
    .map((name) => name.slice(0, -4))
    .filter((domain) => !serverOnly.has(domain))
    .sort();
  return ["terms", ...domains.filter((domain) => domain !== "terms")]
    .map((domain) => fs.readFileSync(new URL(`${domain}.ftl`, LOCALES), "utf8") + "\n")
    .join("");
}

/** Installs the English `I18n` runtime as `globalThis.I18n` and returns it. */
export function installEnglishI18n() {
  const context = { console, Intl, Date, JSON, fetch: async () => ({ ok: true }) };
  context.window = context;
  context.__SCREENERBOT_L10N__ = {
    locale: "en",
    intlLocale: "en-u-nu-latn",
    dir: "ltr",
    source: "en",
    catalogs: [{ locale: "en", ftl: englishCatalog() }],
  };
  vm.createContext(context);
  for (const file of ["assets/fluent-bundle.js", "templates/scripts/core/i18n.js"]) {
    vm.runInContext(fs.readFileSync(new URL(file, WEBSERVER), "utf8"), context);
  }
  globalThis.I18n = context.I18n;
  return context.I18n;
}
