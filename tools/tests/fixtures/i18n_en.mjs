// `I18n` global backed by the real runtime and the source-locale catalogs, for
// tests that assert on rendered wording. Importing this file installs it.
import fs from "node:fs";
import vm from "node:vm";

const WEBSERVER = new URL("../../../src/webserver/", import.meta.url);
const CATALOGS = new URL("../../../locales/en/", import.meta.url);
// Domains the server keeps out of the dashboard payload (SERVER_ONLY_DOMAINS).
const SERVER_ONLY = new Set(["telegram", "shell"]);

function catalog() {
  const domains = fs
    .readdirSync(CATALOGS)
    .filter((name) => name.endsWith(".ftl"))
    .map((name) => name.slice(0, -4))
    .filter((domain) => !SERVER_ONLY.has(domain) && domain !== "terms")
    .sort();
  return ["terms", ...domains]
    .map((domain) => fs.readFileSync(new URL(`${domain}.ftl`, CATALOGS), "utf8") + "\n")
    .join("");
}

const context = {
  console: { warn() {}, error() {}, debug() {}, log() {} },
  Intl,
  Date,
  JSON,
  fetch: async () => ({ ok: true }),
};
context.window = context;
context.__SCREENERBOT_L10N__ = {
  locale: "en",
  intlLocale: "en-US",
  dir: "ltr",
  source: "en",
  catalogs: [{ locale: "en", ftl: catalog() }],
};
vm.createContext(context);
for (const file of ["assets/fluent-bundle.js", "templates/scripts/core/i18n.js"]) {
  vm.runInContext(fs.readFileSync(new URL(file, WEBSERVER), "utf8"), context);
}

export const englishI18n = context.I18n;
globalThis.I18n = englishI18n;
