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

import { analyze } from "../i18n/audit.mjs";
import { checkCatalogs, parseServerOnlyDomains } from "../i18n/catalogs.mjs";
import { scanCss } from "../i18n/css_direction.mjs";
import { scanFormatting } from "../i18n/formatting.mjs";
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

test("markup outside the allowlist is an error in every locale", () => {
  const bad = ["<script>x</script>", '<strong class="x">x</strong>', "<a href>x</a>", "a < b", "<strong >x</strong>"];
  for (const text of bad) {
    const files = { "common.ftl": `hello = ${text}\n`, "terms.ftl": "-brand = ScreenerBot\n" };
    const target = catalogErrors("de", files);
    assert.match(messagesOf(target), /message "hello" uses "<" outside the markup allowlist/, text);
    const source = checkCatalogs({ catalogs: { en: files }, registered: new Set(["en"]) });
    assert.match(messagesOf(source), /^en: message "hello" uses "<"/m, text);
  }
});

test("allowlisted markup passes and a string literal may carry a bare less-than", () => {
  const files = {
    "common.ftl": 'hello = <STRONG>Hallo</STRONG> <em>x</em><br/> { "<" }1\n',
    "terms.ftl": "-brand = ScreenerBot\n",
  };
  const source = { ...EN, "common.ftl": 'hello = <strong>Hello</strong> <em>x</em><br> { "<" }1\n' };
  const result = checkCatalogs({ catalogs: { en: source, de: files }, registered: new Set(["en"]) });
  assert.doesNotMatch(messagesOf(result), /markup/);
});

test("a translation must use the same markup tags as the source", () => {
  const en = { ...EN, "common.ftl": "hello = Hello <strong>{ $n }</strong> and <em>x</em>\nother = Other\n" };
  const run = (text) =>
    checkCatalogs({
      catalogs: { en, de: { "common.ftl": `hello = ${text}\nother = Anderes\n`, "terms.ftl": "-brand = ScreenerBot\n" } },
      registered: new Set(["en"]),
    });
  assert.match(messagesOf(run("Hallo <strong>{ $n }</strong> und x")), /message "hello" markup tags \[strong \/strong\] differ from the source \[strong \/strong em \/em\]/);
  assert.match(messagesOf(run("Hallo <strong>{ $n }</strong> <b>x</b> <em>y</em>")), /markup tags \[strong \/strong b \/b em \/em\] differ/);
  assert.match(messagesOf(run("Hallo <strong>{ $n }</strong> <em>x</em> <em>y</em>")), /differ from the source/);
  assert.doesNotMatch(messagesOf(run("<em>x</em> und <strong>{ $n }</strong>")), /markup tags/);
});

test("markup parity of select messages compares distinct tag names", () => {
  const en = {
    ...EN,
    "common.ftl": "items = { $count ->\n    [one] <b>{ $count }</b> item\n   *[other] <b>{ $count }</b> items\n}\n",
  };
  const ru = {
    "common.ftl": "items = { $count ->\n    [one] <b>{ $count }</b> a\n    [few] <b>{ $count }</b> b\n    [many] <b>{ $count }</b> c\n   *[other] <b>{ $count }</b> d\n}\n",
    "terms.ftl": "-brand = ScreenerBot\n",
  };
  const result = checkCatalogs({ catalogs: { en, ru }, registered: new Set(["en"]) });
  assert.doesNotMatch(messagesOf(result), /markup tags/);
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
  const unused = unusedErrors({ ids: IDS, used: js.used, namespaces: {} });
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
  assert.equal(count(".a { border-radius: 0 4px 4px; }"), 1);
  assert.equal(count(".a { border-radius: 0 4px; }"), 1);
  assert.equal(count(".a { border-radius: 4px 4px 2px; }"), 1);
  assert.equal(count(".a { border-radius: 4px; }"), 0);
  assert.equal(count(".a { margin: 0 4px 2px; }"), 0);
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

function analyzeSources({ js = [], css = [] }) {
  return analyze({
    sources: { js, html: [], css, rust: [] },
    catalogInput: { catalogs: { en: EN }, registered: new Set(["en"]) },
  });
}

const cssErrors = (result) => result.errors.filter((error) => error.file === "a.css");
const analyzeScripts = (js) => analyzeSources({ js });
const cssMessages = (result) => cssErrors(result).map((error) => `${error.file}:${error.line} ${error.message}`).join("\n");

test("a hardcoded string is an error", () => {
  const result = analyzeScripts([{ path: "a.js", source: `el.textContent = "Save changes";` }]);
  const messages = result.errors.map((error) => `${error.file}:${error.line} ${error.message}`).join("\n");
  assert.match(messages, /a\.js:1 hardcoded user-visible string \(assign-textContent\): "Save changes"/);
});

test("a physical-direction declaration is an error", () => {
  const result = analyzeSources({ css: [{ path: "a.css", source: ".a {\n  margin-left: 4px;\n}" }] });
  assert.match(
    cssMessages(result),
    /a\.css:2 physical-direction CSS \(margin-left\): use the logical property or annotate \/\* rtl-ok: <reason> \*\//
  );
  assert.deepEqual(result.current.cssDirection, { "a.css": 1 });
  const logical = analyzeSources({ css: [{ path: "a.css", source: ".a { margin-inline-start: 4px; }" }] });
  assert.deepEqual(cssErrors(logical), []);
});

test("rtl-ok exempts a physical declaration and requires a reason", () => {
  const exempt = analyzeSources({ css: [{ path: "a.css", source: ".a {\n  /* rtl-ok: chart axis */\n  left: 0;\n}" }] });
  assert.deepEqual(cssErrors(exempt), []);
  assert.equal(exempt.rtlOk, 1);
  const empty = analyzeSources({ css: [{ path: "a.css", source: ".a {\n  /* rtl-ok: */\n  left: 0;\n}" }] });
  assert.match(cssMessages(empty), /rtl-ok requires a reason/);
  assert.match(cssMessages(empty), /physical-direction CSS \(left\)/);
});

test("l10n-ignore still exempts a hardcoded string from the error", () => {
  const result = analyzeScripts([
    { path: "a.js", source: `el.textContent = "Brand Name"; // l10n-ignore: brand` },
  ]);
  assert.deepEqual(result.errors.filter((error) => /hardcoded/.test(error.message)), []);
  assert.equal(result.ignores, 1);
});

const SCRIPTS = "src/webserver/templates/scripts";
const formattingOf = (source, path) => scanFormatting({ source, path: `${SCRIPTS}/${path}` });

test("formatting flags toLocale*String calls in a page", () => {
  const result = formattingOf("const a = x.toLocaleString();\nconst b = d.toLocaleDateString('de');", "pages/home.js");
  assert.equal(result.errors.length, 2);
  assert.match(result.errors[0].message, /toLocaleString\(\) formats a value outside core\/format\.js/);
  assert.equal(result.errors[1].line, 2);
  assert.equal(formattingOf("t.toLocaleTimeString();", "pages/home.js").errors.length, 1);
  assert.equal(formattingOf("x?.['toLocaleString']();", "pages/home.js").errors.length, 1);
});

test("formatting flags Intl members and the en-US literal in ui scripts", () => {
  assert.equal(formattingOf("const f = new Intl.NumberFormat('de');", "ui/panel.js").errors.length, 1);
  assert.equal(formattingOf("const f = window.Intl.DateTimeFormat();", "ui/panel.js").errors.length, 1);
  assert.equal(formattingOf("const l = 'en-US';", "ui/panel.js").errors.length, 1);
  assert.equal(formattingOf("const l = `en-US`;", "ui/panel.js").errors.length, 1);
  assert.equal(formattingOf("const label = 'International';", "ui/panel.js").errors.length, 0);
});

test("formatting allows core/format.js and core/i18n.js only", () => {
  const source = "const f = new Intl.NumberFormat('en-US'); x.toLocaleString();";
  assert.equal(formattingOf(source, "core/format.js").errors.length, 0);
  assert.equal(formattingOf(source, "core/i18n.js").errors.length, 0);
  assert.ok(formattingOf(source, "core/utils.js").errors.length > 0);
  assert.ok(formattingOf(source, "pages/format.js").errors.length > 0);
});

test("formatting honours l10n-format-ok with a reason and rejects it without one", () => {
  const same = formattingOf("input.value = d.toLocaleDateString('en-CA'); // l10n-format-ok: machine input value", "pages/a.js");
  assert.equal(same.errors.length, 0);
  assert.equal(same.escapes, 1);
  const above = formattingOf("// l10n-format-ok: machine input value\ninput.value = d.toLocaleDateString('en-CA');", "pages/a.js");
  assert.equal(above.errors.length, 0);
  const empty = formattingOf("input.value = d.toLocaleDateString('en-CA'); // l10n-format-ok:", "pages/a.js");
  assert.equal(empty.errors.length, 2);
  assert.ok(empty.errors.some((error) => /requires a reason/.test(error.message)));
  const far = formattingOf("// l10n-format-ok: machine input value\n\ninput.value = d.toLocaleDateString('en-CA');", "pages/a.js");
  assert.equal(far.errors.length, 1);
});

test("formatting reports a script that cannot be parsed", () => {
  const result = formattingOf("const = ;", "pages/a.js");
  assert.match(result.errors[0].message, /cannot parse/);
});

test("the server-only domain list is read from its Rust declaration", () => {
  const rust = `pub const SERVER_ONLY_DOMAINS: &[&str] = &["telegram", "desktop"];`;
  assert.deepEqual([...parseServerOnlyDomains(rust)], ["telegram", "desktop"]);
  assert.throws(() => parseServerOnlyDomains("pub const OTHER: u8 = 1;"), /SERVER_ONLY_DOMAINS not found/);
});

test("a dashboard reference to a server-only domain is an error", () => {
  const result = analyze({
    sources: {
      js: [{ path: "a.js", source: `I18n.t("bot-hello");` }],
      html: [],
      css: [],
      rust: [{ path: "b.rs", source: `ids::BOT_OTHER` }],
    },
    catalogInput: {
      catalogs: { en: { "bot.ftl": "bot-hello = Hello\nbot-other = Other\n", "terms.ftl": "-brand = B\n" } },
      registered: new Set(["en"]),
      serverOnly: new Set(["bot"]),
    },
  });
  const messages = result.errors.map((error) => error.message).join("\n");
  assert.match(messages, /"bot-hello" is used by the dashboard but bot\.ftl is server-only/);
  assert.doesNotMatch(messages, /bot-other/);
});
