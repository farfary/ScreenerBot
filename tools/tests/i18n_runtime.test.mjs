// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Tests for the dashboard localization runtime (`core/i18n.js`).
 *
 * Runs the vendored Fluent bundle and the runtime in an isolated context with a
 * catalog payload shaped like `/i18n/<locale>/catalog.js`.
 *
 * Run with `npm run test:js`.
 */

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const read = (rel) =>
  fs.readFileSync(new URL(`../../src/webserver/${rel}`, import.meta.url), "utf8");
const BUNDLE_JS = read("assets/fluent-bundle.js");
const RUNTIME_JS = read("templates/scripts/core/i18n.js");

const EN = `
hello = Hello
greeting = Hi { $name }
items = { $count ->
    [one] One item
   *[other] { $count } items
}
wrapped = Result: { $inner }
when = Seen { $at }
tip = Tip
    .title = Hover
    .onclick = bad
    .aria-label = Label
attrs-only =
    .placeholder = Type here
hinted = Headline
    .hint = Try { $name }
`;

const FA = `
hello = Salam
`;

function load(payload) {
  const logs = { warn: [], error: [], debug: [] };
  const context = {
    console: {
      warn: (...a) => logs.warn.push(a.join(" ")),
      error: (...a) => logs.error.push(a.join(" ")),
      debug: (...a) => logs.debug.push(a.join(" ")),
      log() {},
    },
    Intl,
    Date,
    JSON,
    fetch: async () => ({ ok: true }),
    // Records what the markup path builds: the sanitized string handed to an inert template.
    document: {
      createElement: (tag) => ({
        tag,
        innerHTML: "",
        get content() {
          return { template: this.tag, html: this.innerHTML };
        },
      }),
    },
  };
  context.window = context;
  if (payload) context.__SCREENERBOT_L10N__ = payload;
  vm.createContext(context);
  vm.runInContext(BUNDLE_JS, context);
  vm.runInContext(RUNTIME_JS, context);
  return { I18n: context.I18n, logs };
}

const payload = {
  locale: "fa",
  intlLocale: "fa-u-nu-latn",
  dir: "rtl",
  source: "en",
  catalogs: [
    { locale: "en", ftl: EN },
    { locale: "fa", ftl: FA },
  ],
};

function fakeElement(attrs) {
  const store = { ...attrs };
  return {
    textContent: "old",
    getAttribute: (name) => (name in store ? store[name] : null),
    setAttribute: (name, value) => {
      store[name] = value;
    },
    hasAttribute: (name) => name in store,
    replaceChildren(content) {
      this.children = content;
    },
    querySelectorAll: () => [],
    store,
  };
}

test("exposes locale metadata with Latin digits", () => {
  const { I18n } = load(payload);
  assert.equal(I18n.locale, "fa");
  assert.equal(I18n.dir, "rtl");
  assert.equal(I18n.source, "en");
  assert.match(I18n.intlLocale, /-u-nu-latn$/);
});

test("locale overrides per key and falls back to the source", () => {
  const { I18n } = load(payload);
  assert.equal(I18n.t("hello"), "Salam");
  assert.equal(I18n.t("tip"), "Tip");
  assert.equal(I18n.has("hello"), true);
  assert.equal(I18n.has("nope"), false);
});

test("missing id returns the id and warns once", () => {
  const { I18n, logs } = load(payload);
  assert.equal(I18n.t("nope"), "nope");
  assert.equal(I18n.t("nope"), "nope");
  assert.equal(logs.warn.length, 1);
});

test("plural selection follows $count", () => {
  const { I18n } = load(payload);
  assert.equal(I18n.t("items", { count: 1 }), "One item");
  assert.match(I18n.t("items", { count: 3 }), /3.* items$/);
});

test("label resolves mapped values and returns unmapped values unchanged", () => {
  const { I18n, logs } = load(payload);
  const map = Object.freeze({ a: "hello", b: "greeting", c: "absent-key" });
  assert.equal(I18n.label(map, "a"), "Salam");
  assert.equal(I18n.label(map, "zzz"), "zzz");
  assert.equal(I18n.label(map, "zzz"), "zzz");
  assert.equal(I18n.label(map, "c"), "c");
  assert.equal(I18n.label(map, "constructor"), "constructor");
  assert.equal(I18n.label(map, null), "");
  assert.equal(I18n.label(map, undefined), "");
  assert.equal(logs.warn.filter((line) => line.includes("zzz")).length, 1);
  assert.ok(logs.warn.some((line) => line.includes("constructor")));
});

test("attr returns the formatted attribute or null", () => {
  const { I18n } = load(payload);
  assert.equal(I18n.attr("tip", "title"), "Hover");
  assert.equal(I18n.attr("tip", "missing"), null);
  assert.equal(I18n.attr("nope", "title"), null);
});

test("textAttr renders an attribute of a backend text with its arguments", () => {
  const { I18n } = load(payload);
  const text = { id: "hinted", args: { name: { type: "text", value: "again" } } };
  assert.match(I18n.textAttr(text, "hint"), /Try .*again/);
  assert.equal(I18n.textAttr(text, "missing"), null);
  assert.equal(I18n.textAttr({ id: "hello" }, "hint"), null);
  assert.equal(I18n.textAttr({ id: "nope" }, "hint"), null);
  assert.equal(I18n.textAttr(null, "hint"), null);
});

test("text renders count, nested and time arguments", () => {
  const { I18n } = load(payload);
  assert.match(I18n.text({ id: "items", args: { count: { type: "count", value: 3 } } }), /3/);
  const nested = I18n.text({
    id: "wrapped",
    args: { inner: { type: "nested", value: { id: "hello" } } },
  });
  assert.match(nested, /Result: .*Salam/);
  const ms = Date.UTC(2026, 0, 15, 12, 0);
  const expected = new Intl.DateTimeFormat("fa-u-nu-latn", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(ms));
  const when = I18n.text({ id: "when", args: { at: { type: "time", value: ms } } });
  assert.ok(when.includes(expected), when);
});

test("registerArgFormatter replaces a converter", () => {
  const { I18n } = load(payload);
  I18n.registerArgFormatter("sol", (v) => `${v} SOL`);
  const out = I18n.text({ id: "greeting", args: { name: { type: "sol", value: "1.5" } } });
  assert.ok(out.includes("1.5 SOL"), out);
});

test("localizeTree sets text and allowlisted attributes only", () => {
  const { I18n } = load(payload);
  const el = fakeElement({ "data-l10n-id": "tip" });
  I18n.localizeTree(el);
  assert.equal(el.textContent, "Tip");
  assert.equal(el.store.title, "Hover");
  assert.equal(el.store["aria-label"], "Label");
  assert.equal("onclick" in el.store, false);
});

test("localizeTree keeps children for attribute-only messages and unknown ids", () => {
  const { I18n } = load(payload);
  const attrsOnly = fakeElement({ "data-l10n-id": "attrs-only" });
  I18n.localizeTree(attrsOnly);
  assert.equal(attrsOnly.textContent, "old");
  assert.equal(attrsOnly.store.placeholder, "Type here");
  const unknown = fakeElement({ "data-l10n-id": "nope" });
  I18n.localizeTree(unknown);
  assert.equal(unknown.textContent, "old");
});

test("localizeTree interpolates data-l10n-args and walks descendants", () => {
  const { I18n } = load(payload);
  const child = fakeElement({
    "data-l10n-id": "greeting",
    "data-l10n-args": '{"name":"Ada"}',
  });
  const root = { ...fakeElement({}), querySelectorAll: () => [child] };
  I18n.localizeTree(root);
  assert.ok(child.textContent.includes("Ada"));
});

test("runtime survives a missing catalog", () => {
  const { I18n, logs } = load(null);
  assert.equal(I18n.t("hello"), "hello");
  assert.equal(I18n.has("hello"), false);
  assert.equal(I18n.attr("tip", "title"), null);
  assert.equal(I18n.text({ id: "hello" }), "hello");
  assert.equal(logs.error.length, 1);
});

const PSEUDO_FIXTURE = JSON.parse(
  fs.readFileSync(new URL("./fixtures/i18n_pseudo.json", import.meta.url), "utf8")
);

function pseudoPayload(kind, ftl) {
  return {
    locale: kind === "bidi" ? "ar-XB" : "en-XA",
    intlLocale: kind === "bidi" ? "ar-XB-u-nu-latn" : "en-XA-u-nu-latn",
    dir: kind === "bidi" ? "rtl" : "ltr",
    source: "en",
    pseudo: kind,
    catalogs: [{ locale: "en", ftl }],
  };
}

// The empty input cannot be written as a Fluent message value; it is covered by
// the Rust tests, and every other fixture input is valid inline text.
const FIXTURE_CASES = PSEUDO_FIXTURE.map((c, i) => ({ ...c, id: `case-${i}` })).filter(
  (c) => c.input !== ""
);
const FIXTURE_FTL = FIXTURE_CASES.map((c) => `${c.id} = ${c.input}`).join("\n") + "\n";

for (const kind of ["accented", "bidi"]) {
  test(`pseudo ${kind} runtime matches the shared fixture`, () => {
    const { I18n, logs } = load(pseudoPayload(kind, FIXTURE_FTL));
    assert.equal(I18n.pseudo, kind);
    for (const c of FIXTURE_CASES) {
      assert.equal(I18n.t(c.id), c[kind], c.input);
    }
    assert.deepEqual(logs.error, []);
  });

  test(`pseudo ${kind} leaves placeables untransformed`, () => {
    const { I18n } = load(pseudoPayload(kind, "who = Hi { $name }!\n"));
    // "Hi " and "!" are text elements; the variable value is passed through as is.
    const prefix = kind === "bidi" ? "\u202EHı\u202C " : "Ħīī ";
    assert.equal(I18n.t("who", { name: "Word" }), `${prefix}\u2068Word\u2069!`);
  });
}

test("pseudo is null for a real locale", () => {
  assert.equal(load(payload).I18n.pseudo, null);
});

const MARKUP_FIXTURE = JSON.parse(
  fs.readFileSync(new URL("./fixtures/i18n_markup.json", import.meta.url), "utf8")
);

function markupRuntime(ftl, extra = {}) {
  return load({
    locale: "en",
    intlLocale: "en-u-nu-latn",
    dir: "ltr",
    source: "en",
    catalogs: [{ locale: "en", ftl }],
    ...extra,
  });
}

// Each sanitizer input is fed through a Fluent string-literal placeable, which
// formats verbatim and, as the whole pattern, without isolation marks. The
// Rust test runs the same cases against `sanitize` directly.
test("markup sanitizer matches the shared fixture", () => {
  for (const c of MARKUP_FIXTURE.sanitize) {
    const { I18n } = markupRuntime(`m = { ${JSON.stringify(c.input)} }\n`);
    assert.equal(I18n.markup("m"), c.expected, c.name);
  }
});

test("markup render pipeline matches the shared fixture", () => {
  for (const c of MARKUP_FIXTURE.render) {
    const { I18n } = markupRuntime(`${c.ftl}\n`);
    assert.equal(I18n.markup("m", c.args), c.expected, c.name);
  }
});

test("plain t() output of a markup message is not sanitized", () => {
  const { I18n } = markupRuntime("m = Use <strong>{ $x }</strong>\n");
  assert.equal(I18n.t("m", { x: "<b>" }), "Use <strong>\u2068<b>\u2069</strong>");
});

test("markup of a missing id is the escaped id and warns once", () => {
  const { I18n, logs } = markupRuntime("m = x\n");
  assert.equal(I18n.markup("<nope>"), "&lt;nope&gt;");
  assert.equal(I18n.markup("<nope>"), "&lt;nope&gt;");
  assert.equal(logs.warn.length, 1);
});

test("markup drops arguments that are neither strings nor numbers", () => {
  const { I18n } = markupRuntime("m = <b>{ $x }</b>\n");
  const out = I18n.markup("m", { x: { toString: () => "<script>" } });
  assert.ok(!out.includes("<script>"), out);
});

test("localizeTree applies markup only when data-l10n-markup is present", () => {
  const { I18n } = markupRuntime("m = Total <strong>{ $n }</strong>\n");
  const args = JSON.stringify({ n: "<i>1</i>" });
  const withMarkup = fakeElement({
    "data-l10n-id": "m",
    "data-l10n-args": args,
    "data-l10n-markup": "",
  });
  I18n.localizeTree(withMarkup);
  assert.deepEqual(withMarkup.children, {
    template: "template",
    html: "Total <strong>\u2068&lt;i&gt;1&lt;/i&gt;\u2069</strong>",
  });
  assert.equal(withMarkup.textContent, "old");

  const plain = fakeElement({ "data-l10n-id": "m", "data-l10n-args": args });
  I18n.localizeTree(plain);
  assert.equal(plain.textContent, "Total <strong>\u2068<i>1</i>\u2069</strong>");
  assert.equal(plain.children, undefined);
});

test("attributes of a markup element are plain text with unescaped arguments", () => {
  const { I18n } = markupRuntime("m = <b>{ $n }</b>\n    .title = Cost { $n }\n");
  const el = fakeElement({
    "data-l10n-id": "m",
    "data-l10n-args": JSON.stringify({ n: "a & b" }),
    "data-l10n-markup": "",
  });
  I18n.localizeTree(el);
  assert.equal(el.store.title, "Cost \u2068a & b\u2069");
  assert.equal(el.children.html, "<b>\u2068a &amp; b\u2069</b>");
});

for (const kind of ["accented", "bidi"]) {
  test(`pseudo ${kind} keeps markup tag names intact`, () => {
    const { I18n } = markupRuntime("m = Save <strong>all</strong><br/> now\n", {
      pseudo: kind,
      locale: kind === "bidi" ? "ar-XB" : "en-XA",
    });
    const out = I18n.markup("m");
    assert.ok(out.includes("<strong>") && out.includes("</strong><br>"), out);
  });
}
