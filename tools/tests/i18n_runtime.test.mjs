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

test("attr returns the formatted attribute or null", () => {
  const { I18n } = load(payload);
  assert.equal(I18n.attr("tip", "title"), "Hover");
  assert.equal(I18n.attr("tip", "missing"), null);
  assert.equal(I18n.attr("nope", "title"), null);
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
