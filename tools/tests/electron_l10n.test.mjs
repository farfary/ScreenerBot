// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Guards for the Electron shell's localization (`electron/src/l10n.js`,
 * `locales/en/desktop.ftl`).
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const { createLocalizer, negotiateLocale, parseRegistry } = require("../../electron/src/l10n.js");
const { l10nResources } = require("../../electron/src/paths.js");

const resources = l10nResources({ isPackaged: false });
const catalog = fs.readFileSync(path.join(ROOT, "locales/en/desktop.ftl"), "utf8");
const catalogIds = new Set([...catalog.matchAll(/^([a-z][a-z0-9-]*)\s*=/gm)].map((match) => match[1]));

const shellSources = ["main.js", "boot.js", "preload.js", "index.html"].map((name) => ({
  name,
  text: fs.readFileSync(path.join(ROOT, "electron/src", name), "utf8"),
}));

test("negotiation prefers an exact registered code", () => {
  assert.equal(negotiateLocale(["fr", "en"], ["en", "fr"]), "fr");
  assert.equal(negotiateLocale(["pt-BR"], ["en", "pt-BR", "pt"]), "pt-BR");
  assert.equal(negotiateLocale(["PT_br"], ["en", "pt-BR"]), "pt-BR");
});

test("negotiation falls back to the language subtag", () => {
  assert.equal(negotiateLocale(["fr-CA", "en-US"], ["en", "fr"]), "fr");
  assert.equal(negotiateLocale(["de-AT"], ["en", "de"]), "de");
  assert.equal(negotiateLocale(["pt"], ["en", "pt-BR"]), "pt-BR");
  assert.equal(negotiateLocale(["pt-PT"], ["en", "pt-BR"]), "pt-BR");
  assert.equal(negotiateLocale(["zh-CN"], ["en", "zh-Hans"]), "zh-Hans");
  assert.equal(negotiateLocale(["zh-Hans-SG"], ["en", "zh-Hans"]), "zh-Hans");
  assert.equal(negotiateLocale(["zh-TW"], ["en", "zh-Hans"]), "en");
});

test("negotiation resolves unknown or empty preferences to the source locale", () => {
  assert.equal(negotiateLocale(["zz-ZZ"], ["en", "fr"]), "en");
  assert.equal(negotiateLocale([], ["en"]), "en");
  assert.equal(negotiateLocale(undefined, ["en"]), "en");
});

test("the registry parser reads codes and directions", () => {
  const entries = parseRegistry('source = "en"\n\n[[locale]]\ncode = "en"\nname = "English"\ndir = "ltr"\n\n[[locale]]\ncode = "ar"\nname = "x"\ndir = "rtl"\n');
  assert.deepEqual(entries, [{ code: "en", dir: "ltr" }, { code: "ar", dir: "rtl" }]);
});

test("the localizer renders source messages without isolation marks", () => {
  const localizer = createLocalizer({ ...resources, locale: "en" });
  assert.equal(localizer.locale, "en");
  assert.equal(localizer.dir, "ltr");
  assert.equal(localizer.t("desktop-tray-quit"), "Quit ScreenerBot");
  assert.equal(localizer.t("desktop-splash-updating", { version: "1.2.3" }), "Updating to v1.2.3");
  assert.doesNotMatch(localizer.t("desktop-about-detail", { version: "1.2.3" }), /[⁨⁩]/);
  assert.equal(localizer.t("desktop-not-a-message"), "desktop-not-a-message");
});

test("an unregistered locale resolves to the source locale", () => {
  assert.equal(createLocalizer({ ...resources, locale: "zz" }).locale, "en");
  assert.equal(createLocalizer({ ...resources, locale: null }).locale, "en");
});

test("splash strings cover the splash and boot-error prefixes", () => {
  const strings = createLocalizer({ ...resources, locale: "en" }).stringsWithPrefix(["desktop-splash-", "desktop-boot-"]);
  assert.ok(strings["desktop-splash-starting"]);
  assert.ok(strings["desktop-boot-action-quit"]);
  assert.ok(Object.keys(strings).every((id) => /^desktop-(splash|boot)-/.test(id)));
});

test("every desktop id referenced by the shell exists in desktop.ftl", () => {
  for (const { name, text } of shellSources) {
    for (const { 0: id, index } of text.matchAll(/desktop-[a-z0-9]+(?:-[a-z0-9]+)*/g)) {
      // A trailing hyphen marks a prefix such as `desktop-splash-`, not an id.
      if (text[index + id.length] === "-") continue;
      assert.ok(catalogIds.has(id), `${name} references unknown id ${id}`);
    }
  }
});

test("every desktop.ftl message is referenced by the shell", () => {
  const source = shellSources.map(({ text }) => text).join("\n");
  for (const id of catalogIds) {
    assert.ok(source.includes(id), `${id} is not referenced by electron/src`);
  }
});

test("main.js has no English literal in label, title, message, detail or buttons", () => {
  const main = shellSources.find((entry) => entry.name === "main.js").text;
  const offenders = [];
  for (const [line, text] of main.split("\n").entries()) {
    const literal = text.match(/\b(label|title|message|detail|buttons)\s*:\s*(\[?\s*['"`])/);
    if (literal) offenders.push(`main.js:${line + 1}: ${text.trim()}`);
  }
  // Menu labels that name the application come from the platform, not a catalog.
  const allowed = offenders.filter((entry) => !/label: app\.name/.test(entry));
  assert.deepEqual(allowed, []);
});
