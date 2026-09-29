/**
 * Tests for the localization audit (`tools/i18n/`).
 *
 * Every check is exercised on in-memory fixtures through the modules' pure
 * functions, so no files are read.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";

import { compareBaseline, initialBaseline, lowerBaseline } from "../i18n/audit.mjs";
import { checkCatalogs } from "../i18n/catalogs.mjs";
import { scanCss } from "../i18n/css_direction.mjs";
import { scanHtmlHardcoded, scanJsHardcoded } from "../i18n/hardcoded.mjs";
import { scanHtmlUsage, scanJsUsage, unusedErrors } from "../i18n/usage.mjs";

const EN = {
  "common.ftl": `
hello = Hello
tip = Tip
    .title = Hover
greeting = Hi { $name }
items = { $count ->
    [one] One item
   *[other] { $count } items
}
`,
  "terms.ftl": `-brand = ScreenerBot\n`,
};

function catalogErrors(locale, files, { registered = [] } = {}) {
  return checkCatalogs({
    catalogs: { en: EN, [locale]: files },
    registered: new Set(["en", ...registered]),
  });
}

const messagesOf = (result) => result.errors.map((error) => error.message).join("\n");

test("catalog parity flags a missing attribute", () => {
  const result = catalogErrors("de", { "common.ftl": "tip = Tipp\n", "terms.ftl": "-brand = ScreenerBot\n" });
  assert.match(messagesOf(result), /message "tip" is missing attributes \[title\]/);
});

test("catalog parity flags a variable the source does not use", () => {
  const result = catalogErrors("de", {
    "common.ftl": "greeting = Hallo { $name } { $other }\n",
    "terms.ftl": "-brand = ScreenerBot\n",
  });
  assert.match(messagesOf(result), /references variables not in the source \[other\]/);
});

test("catalog parity requires the plural categories of the locale", () => {
  const ru = (variants) => ({
    "common.ftl": `items = { $count ->\n${variants}\n}\n`,
    "terms.ftl": "-brand = ScreenerBot\n",
  });
  const partial = catalogErrors("ru", ru("    [one] a\n   *[other] b"));
  assert.match(messagesOf(partial), /lacks categories \[few, many\]/);
  const full = catalogErrors("ru", ru("    [one] a\n    [few] b\n    [many] c\n   *[other] d"));
  assert.doesNotMatch(messagesOf(full), /lacks categories/);
});

test("catalog parity rejects a translated term and a missing term", () => {
  const translated = catalogErrors("de", { "common.ftl": "", "terms.ftl": "-brand = BildschirmBot\n" });
  assert.match(messagesOf(translated), /term "-brand" must be identical/);
  const missing = catalogErrors("de", { "common.ftl": "", "terms.ftl": "" });
  assert.match(messagesOf(missing), /term "-brand" is missing/);
});

test("catalog parity reports ids absent from the source and junk", () => {
  const result = catalogErrors("de", { "common.ftl": "unknown = x\nbroken = { \n", "terms.ftl": "-brand = ScreenerBot\n" });
  assert.match(messagesOf(result), /message "unknown" does not exist in en/);
  assert.match(messagesOf(result), /junk entry/);
});

test("an unregistered partial locale is info, a registered one must be complete", () => {
  const files = { "common.ftl": "hello = Hallo\n", "terms.ftl": "-brand = ScreenerBot\n" };
  const unregistered = catalogErrors("de", files);
  assert.equal(unregistered.errors.length, 0);
  assert.match(unregistered.info.join("\n"), /locale de is not registered: \d+(\.\d+)?% complete/);
  const registered = catalogErrors("de", files, { registered: ["de"] });
  assert.match(messagesOf(registered), /registered locale is incomplete; missing \[tip, greeting, items\]/);
});

const IDS = new Set(["hello", "tip", "config-x"]);
const NAMESPACES = { "config-": "config_catalog_covers_fields" };
const usageOf = (source, namespaces = {}) => scanJsUsage({ source, path: "a.js", ids: IDS, namespaces });

test("usage rejects an unknown id in data-l10n-id and I18n calls", () => {
  const html = scanHtmlUsage({ source: `<span data-l10n-id="nope"></span>`, path: "a.html", ids: IDS });
  assert.match(html.errors[0].message, /"nope" is not in the source catalog/);
  const js = usageOf(`I18n.t("nope");`);
  assert.match(js.errors[0].message, /id "nope" is not in the source catalog/);
});

test("usage rejects dynamic I18n calls unless a declared namespace annotates them", () => {
  assert.equal(usageOf("I18n.t(`config-${key}`);").errors.length, 1);
  assert.equal(usageOf(`I18n.attr("a" + b, "title");`).errors.length, 1);
  assert.equal(usageOf(`I18n.has(name);`).errors.length, 1);
  const inline = usageOf("I18n.t(`config-${key}`); // l10n-dynamic: config-", NAMESPACES);
  assert.deepEqual(inline.errors, []);
  const above = usageOf("// l10n-dynamic: config-\nI18n.t(`config-${key}`);", NAMESPACES);
  assert.deepEqual(above.errors, []);
  const undeclared = usageOf("I18n.t(`x-${key}`); // l10n-dynamic: x-", NAMESPACES);
  assert.equal(undeclared.errors.length, 1);
});

test("usage collects literal and data-l10n-id references and flags unused ids", () => {
  const js = usageOf(`const a = "hello"; el.innerHTML = '<b data-l10n-id="tip"></b>';`);
  assert.deepEqual([...js.used].sort(), ["hello", "tip"]);
  const unused = unusedErrors({ ids: IDS, used: js.used });
  assert.match(unused[0].message, /"config-x" is not used/);
  assert.deepEqual(unusedErrors({ ids: IDS, used: js.used, namespaces: NAMESPACES }), []);
});

test("usage rejects text inside a data-l10n-id element", () => {
  const bad = scanHtmlUsage({ source: `<p data-l10n-id="hello">Hello <b>x</b></p>`, path: "a.html", ids: IDS });
  assert.match(bad.errors[0].message, /carries its own text/);
  const ok = scanHtmlUsage({ source: `<p data-l10n-id="hello"> <b>x</b>\n</p>`, path: "a.html", ids: IDS });
  assert.deepEqual(ok.errors, []);
});

const hardcodedOf = (source, ids = new Set()) => scanJsHardcoded({ source, path: "a.js", ids });

test("hardcoded counts visible text sinks", () => {
  assert.equal(hardcodedOf(`el.textContent = "Save changes";`).items.length, 1);
  assert.equal(hardcodedOf(`el.setAttribute("aria-label", "Close panel");`).items.length, 1);
  assert.equal(hardcodedOf(`const x = { label: "Open wallet" };`).items.length, 1);
  assert.equal(hardcodedOf(`Utils.showToast({ type: "error", title: "Trade failed" });`).items.length, 1);
  assert.equal(hardcodedOf(`showToast("Copied to clipboard", "success");`).items.length, 1);
  assert.equal(hardcodedOf(`ConfirmationDialog.show({ title: "Delete", confirmText: "Remove it" });`).items.length, 2);
});

test("hardcoded does not count logs, errors, comparisons or keys", () => {
  assert.equal(hardcodedOf(`console.log("x y");`).items.length, 0);
  assert.equal(hardcodedOf(`throw new Error("Something went wrong");`).items.length, 0);
  assert.equal(hardcodedOf(`switch (side) { case "Buy": break; }`).items.length, 0);
  assert.equal(hardcodedOf(`if (side === "Buy Now") {}`).items.length, 0);
  assert.equal(hardcodedOf(`const o = { "Some Key": 1 };`).items.length, 0);
  assert.equal(hardcodedOf(`el.textContent = "btn-primary";`).items.length, 0);
  assert.equal(hardcodedOf(`el.textContent = "hello";`, new Set(["hello"])).items.length, 0);
});

test("hardcoded counts markup text segments and ignores interpolations", () => {
  const source = "el.innerHTML = `<div title=\"Hint text\"><b>Total</b> ${count} <i>${name}</i> Items</div>`;";
  const kinds = hardcodedOf(source).items.map((item) => `${item.kind}:${item.text}`);
  assert.deepEqual(kinds.sort(), ["markup-text:Total", "markup-text:Items", "markup-title:Hint text"].sort());
  const localized = "el.innerHTML = `<b data-l10n-id=\"x\" title=\"Tip\">Text</b><i translate=\"no\">Brand Name</i>`;";
  assert.equal(hardcodedOf(localized).items.length, 0);
});

test("hardcoded honours l10n-ignore and rejects an empty reason", () => {
  const same = hardcodedOf(`el.textContent = "Brand Name"; // l10n-ignore: brand`);
  assert.equal(same.items.length, 0);
  assert.equal(same.ignores, 1);
  const above = hardcodedOf(`// l10n-ignore: brand\nel.textContent = "Brand Name";`);
  assert.equal(above.items.length, 0);
  const empty = hardcodedOf(`el.textContent = "Brand Name"; // l10n-ignore:`);
  assert.equal(empty.items.length, 1);
  assert.match(empty.errors[0].message, /requires a reason/);
});

test("hardcoded html counts text and attributes, skipping localized and no-translate regions", () => {
  const source = [
    `<h1>Dashboard</h1>`,
    `<input placeholder="Search tokens" title="Find">`,
    `<p data-l10n-id="hello"></p>`,
    `<span translate="no">Solana <b>Labs</b></span>`,
    `<script>const a = "Not text";</script>`,
    `<div>{{PLACEHOLDER}}</div>`,
    `<!-- l10n-ignore: legal -->`,
    `<p>Kept as is</p>`,
  ].join("\n");
  const { items } = scanHtmlHardcoded({ source, path: "a.html" });
  assert.deepEqual(items.map((item) => item.text).sort(), ["Dashboard", "Find", "Search tokens"].sort());
});

test("css counts physical properties and asymmetric shorthands only", () => {
  const count = (css) => scanCss({ source: css, path: "a.css" }).items.length;
  assert.equal(count(".a { margin-left: 4px; }"), 1);
  assert.equal(count(".a { margin-inline-start: 4px; text-align: start; }"), 0);
  assert.equal(count(".a { text-align: right; float: left; border-right-color: red; left: 0; }"), 4);
  assert.equal(count(".a { margin: 0 4px 0 8px; }"), 1);
  assert.equal(count(".a { margin: 0 4px 0 4px; padding: 1px 2px; }"), 0);
  assert.equal(count(".a { border-radius: 4px 4px 0 0; }"), 0);
  assert.equal(count(".a { border-radius: 4px 0 0 4px; }"), 1);
  assert.equal(count(".a { transform: translateX(4px); }"), 0);
});

test("css honours rtl-ok and rejects an empty reason", () => {
  const same = scanCss({ source: ".a { left: 0; /* rtl-ok: chart axis */ }", path: "a.css" });
  assert.equal(same.items.length, 0);
  const above = scanCss({ source: ".a {\n  /* rtl-ok: chart axis */\n  left: 0;\n}", path: "a.css" });
  assert.equal(above.items.length, 0);
  assert.equal(above.ignores, 1);
  const empty = scanCss({ source: ".a { left: 0; /* rtl-ok: */ }", path: "a.css" });
  assert.equal(empty.items.length, 1);
  assert.match(empty.errors[0].message, /requires a reason/);
});

const BASELINE = { hardcoded: { "a.js": 3, "b.js": 2 }, cssDirection: { "c.css": 1 } };

test("baseline fails on an increase and on a new file", () => {
  const raised = compareBaseline({ hardcoded: { "a.js": 4, "b.js": 2 }, cssDirection: { "c.css": 1 } }, BASELINE);
  assert.deepEqual(raised.exceeded.map((item) => item.path), ["a.js"]);
  const added = compareBaseline({ hardcoded: { "a.js": 3, "b.js": 2, "new.js": 1 }, cssDirection: { "c.css": 1 } }, BASELINE);
  assert.deepEqual(added.exceeded.map((item) => item.path), ["new.js"]);
});

test("baseline reports a decrease so the update command can record it", () => {
  const { exceeded, lowered } = compareBaseline({ hardcoded: { "a.js": 1 }, cssDirection: { "c.css": 1 } }, BASELINE);
  assert.equal(exceeded.length, 0);
  assert.deepEqual(lowered.map((item) => `${item.path}:${item.count}`), ["a.js:1", "b.js:0"]);
});

test("baseline update lowers or drops entries and refuses to raise", () => {
  const lowered = lowerBaseline({ hardcoded: { "a.js": 1 }, cssDirection: {} }, BASELINE);
  assert.deepEqual(lowered.refused, []);
  assert.deepEqual(lowered.baseline, { hardcoded: { "a.js": 1 }, cssDirection: {} });
  const refused = lowerBaseline({ hardcoded: { "a.js": 9 }, cssDirection: { "c.css": 1 } }, BASELINE);
  assert.deepEqual(refused.refused.map((item) => item.path), ["a.js"]);
  assert.equal(refused.baseline, BASELINE);
  const fresh = lowerBaseline({ hardcoded: { "z.js": 1 }, cssDirection: {} }, BASELINE);
  assert.deepEqual(fresh.refused.map((item) => item.path), ["z.js"]);
});

test("initial baseline sorts keys", () => {
  const baseline = initialBaseline({ hardcoded: { "b.js": 1, "a.js": 2 }, cssDirection: {} });
  assert.deepEqual(Object.keys(baseline.hardcoded), ["a.js", "b.js"]);
});
