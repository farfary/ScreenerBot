/**
 * Golden tests for the dashboard formatters (`core/format.js`).
 *
 * `fixtures/format_golden.json` holds the output of the formatters as they behaved
 * before they moved to `format.js` and became catalog and Intl driven: inputs,
 * option sets and the exact strings they produced under `en-US` and UTC with the
 * clock pinned. The `en` catalogs must reproduce every entry byte for byte.
 *
 * A second group runs the same formatters under other locales to pin the money
 * invariant: only separators change, never digits, decimals or the subscript price
 * rule, and digits stay Latin.
 *
 * Run with `npm run test:js`.
 */

process.env.TZ = "UTC";

import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const WEBSERVER = new URL("../../src/webserver/", import.meta.url);
const read = (rel) => fs.readFileSync(new URL(rel, WEBSERVER), "utf8");
const FIXTURE = JSON.parse(
  fs.readFileSync(new URL("./fixtures/format_golden.json", import.meta.url), "utf8")
);
const BUNDLE_JS = read("assets/fluent-bundle.js");
const RUNTIME_JS = read("templates/scripts/core/i18n.js");
const FORMAT_JS = read("templates/scripts/core/format.js");

// Domains the server keeps out of the dashboard payload (SERVER_ONLY_DOMAINS).
const SERVER_ONLY = new Set(["telegram", "shell"]);

/** The source catalogs as `dashboard_catalog` concatenates them: terms first. */
function enCatalog() {
  const dir = new URL("../../locales/en/", import.meta.url);
  const domains = fs
    .readdirSync(dir)
    .filter((name) => name.endsWith(".ftl"))
    .map((name) => name.slice(0, -4))
    .filter((domain) => !SERVER_ONLY.has(domain))
    .sort();
  const ordered = ["terms", ...domains.filter((domain) => domain !== "terms")];
  return ordered
    .map((domain) => fs.readFileSync(new URL(`${domain}.ftl`, dir), "utf8") + "\n")
    .join("");
}

// The fixture encodes values JSON cannot carry (undefined, NaN, -0, Date).
const RUNNER = `(function (cases, fmt) {
  const dec = (a) => {
    if (a && typeof a === "object" && typeof a.$ === "string") {
      const k = a.$;
      if (k === "undefined") return undefined;
      if (k === "NaN") return NaN;
      if (k === "-0") return -0;
      if (k === "Infinity") return Infinity;
      if (k === "-Infinity") return -Infinity;
      if (k.startsWith("date:")) return new Date(k.slice(5) === "NaN" ? NaN : k.slice(5));
    }
    return a;
  };
  return cases.map((c) => {
    try {
      return { out: fmt[c.fn](...c.args.map(dec)) };
    } catch (e) {
      return { throws: e.name };
    }
  });
})`;

/** Fresh browser-like context with the runtime and `format.js` loaded for `intlLocale`. */
function load(intlLocale) {
  const context = {
    console: { warn() {}, error() {}, debug() {}, log() {} },
    document: { addEventListener() {}, getElementById: () => null },
  };
  context.window = context;
  context.__SCREENERBOT_L10N__ = {
    locale: intlLocale.split("-")[0],
    intlLocale,
    dir: "ltr",
    source: "en",
    catalogs: [{ locale: "en", ftl: enCatalog() }],
  };
  vm.createContext(context);
  vm.runInContext(
    `(function () {
      const RealDate = Date;
      globalThis.Date = class extends RealDate {
        constructor(...args) {
          if (args.length === 0) super(${FIXTURE.now});
          else super(...args);
        }
        static now() {
          return ${FIXTURE.now};
        }
      };
    })();`,
    context
  );
  vm.runInContext(BUNDLE_JS, context);
  vm.runInContext(RUNTIME_JS, context);

  // format.js is an ES module without imports; run it as a script and collect
  // its named exports.
  const names = [...FORMAT_JS.matchAll(/^export (?:function|const) (\w+)/gm)].map((m) => m[1]);
  const script = `(function () {\n${FORMAT_JS.replace(/^export /gm, "")}\nreturn { ${names.join(", ")} };\n})()`;
  const fmt = vm.runInContext(script, context);
  return { context, fmt };
}

function run(fmt, context, cases) {
  return vm.runInContext(RUNNER, context)(JSON.parse(JSON.stringify(cases)), fmt);
}

test("every formatter reproduces the recorded pre-migration output", () => {
  const { context, fmt } = load("en-u-nu-latn");
  const results = run(fmt, context, FIXTURE.cases);
  const failures = [];
  FIXTURE.cases.forEach((expected, index) => {
    const actual = results[index];
    if (actual.out !== expected.out || actual.throws !== expected.throws) {
      failures.push(
        `${expected.fn}(${JSON.stringify(expected.args)}): expected ${JSON.stringify(expected.out ?? expected.throws)}, got ${JSON.stringify(actual.out ?? actual.throws)}`
      );
    }
  });
  assert.equal(
    failures.length,
    0,
    `${failures.length} mismatches:\n${failures.slice(0, 15).join("\n")}`
  );
});

test("the fixture covers every formatter", () => {
  const { fmt } = load("en-u-nu-latn");
  const covered = new Set(FIXTURE.cases.map((c) => c.fn));
  for (const name of Object.keys(fmt)) {
    assert.ok(covered.has(name), `${name} has golden cases`);
  }
  assert.ok(FIXTURE.cases.length > 3000);
});

test("formatters take no locale option", () => {
  assert.doesNotMatch(FORMAT_JS, /locale\s*=\s*"en-US"|toLocale(?:String|DateString|TimeString)/);
});

test("de keeps decimals and rounding; only the separators change", () => {
  const { fmt } = load("de-u-nu-latn");
  assert.equal(fmt.formatNumber(1234.5678), "1.234,57");
  assert.equal(fmt.formatNumber(1234.5678, 4), "1.234,5678");
  assert.equal(fmt.formatNumber(1234.5678, { useGrouping: false }), "1234,57");
  assert.equal(fmt.formatCurrencyUSD(1.005), "$1,00");
  assert.equal(fmt.formatCurrencyUSD(1234.5678), "$1,23K");
  assert.equal(fmt.formatPriceSol(0.000123456789, { decimals: 9 }), "0,000123457");
  assert.equal(fmt.formatPercentValue(12.345), "+12,35%");
  assert.equal(fmt.formatPercent(-3.14159, { style: "plain", decimals: 3 }), "-3,142%");
  assert.equal(fmt.formatSol(1.23456789), "1,2346 SOL");
  assert.equal(fmt.formatBytes(1536), "1,5 KB");
  assert.equal(fmt.formatDuration(1500), "1,5µs");
  assert.equal(fmt.formatSecondsToTime(90), "1,5m");
});

test("de subscript price keeps its subscript digits and significant digits", () => {
  const en = load("en-u-nu-latn").fmt;
  const de = load("de-u-nu-latn").fmt;
  for (const price of [0.0000000012345, 0.000012345, 5e-15, 1.5e-7, 0.00010519166, 123.456]) {
    for (const options of [{}, { precision: 3 }, { precision: 8 }]) {
      assert.equal(
        de.formatPriceSubscript(price, options),
        en.formatPriceSubscript(price, options).replace(".", ","),
        `price ${price} ${JSON.stringify(options)}`
      );
    }
  }
  assert.equal(de.formatPriceSubscript(0.0000000012345), "0,0₈12345");
});

test("fa keeps Latin digits in numbers, percents and dates", () => {
  const { fmt } = load("fa-u-nu-latn");
  const samples = [
    fmt.formatNumber(1234.5678),
    fmt.formatCompactNumber(1234567),
    fmt.formatPercentValue(12.5),
    fmt.formatSol(3.14159),
    fmt.formatTimestamp(FIXTURE.now),
    fmt.formatDate(FIXTURE.now),
    fmt.formatTimeAgo(FIXTURE.now - 300000),
    fmt.formatUptime(90061),
  ];
  for (const sample of samples) {
    assert.doesNotMatch(sample, /[٠-٩۰-۹]/, sample);
    assert.match(sample, /[0-9]/, sample);
  }
});

test("UiText arguments use the formatters", () => {
  const { context } = load("en-u-nu-latn");
  const { I18n } = context;
  vm.runInContext(FORMAT_JS.replace(/^export /gm, ""), context);
  // format-sol-amount is the only catalog message carrying an $amount placeable.
  const render = (type, value) =>
    I18n.text({ id: "format-sol-amount", args: { amount: { type, value } } }).replace(
      /[\u2068\u2069]/g,
      ""
    );
  assert.equal(render("sol", 1.5), "1.5000 SOL SOL");
  assert.equal(render("usd", 1234.5), "$1.23K SOL");
  assert.equal(render("percent", 12.345), "12.35% SOL");
  assert.equal(render("time", FIXTURE.now), "Jun 15, 2025, 03:06:40 PM SOL");
  assert.equal(render("duration", 90061000), "1d 1h 1m SOL");
});

test("page-level formatter options: date parts, trimmed spans and detailed elapsed time", () => {
  const { context, fmt } = load("en-u-nu-latn");
  const now = FIXTURE.now;
  assert.equal(fmt.formatTimestamp(now, { includeYear: false }), "Jun 15, 03:06:40 PM");
  assert.equal(fmt.formatTimestamp(now, { includeDate: false, includeSeconds: false }), "03:06 PM");
  assert.equal(fmt.formatDate(now, { includeYear: false }), "Jun 15");
  assert.equal(
    fmt.formatDate(now, { includeYear: false, weekday: true, utc: true }),
    "Sun, Jun 15"
  );
  assert.equal(fmt.formatUptime(10800, { style: "trimmed" }), "3h");
  assert.equal(fmt.formatUptime(11100, { style: "trimmed" }), "3h 5m");
  assert.equal(fmt.formatUptime(90000, { style: "trimmed" }), "1d 1h");
  assert.equal(fmt.formatUptime(86400, { style: "trimmed" }), "1d");
  assert.equal(fmt.formatUptime(45, { style: "trimmed" }), "45s");
  assert.equal(fmt.formatTimeAgo(now - 2000, { style: "detailed" }), "just now");
  assert.equal(fmt.formatTimeAgo(now - 45000, { style: "detailed" }), "45s ago");
  assert.equal(fmt.formatTimeAgo(now - 11100000, { style: "detailed" }), "3h 5m ago");
  assert.equal(fmt.formatTimeAgo(now - 11100000), "3h ago");
  assert.equal(fmt.formatTimeAgo(null, { style: "detailed", fallback: "—" }), "—");
  assert.equal(fmt.formatPercentValue(1.5, { plus: "" }), "1.50%");
  assert.equal(fmt.formatPercentValue(-1.5, { plus: "" }), "-1.50%");
  assert.equal(fmt.formatPercentValue(1.5), "+1.50%");
  assert.ok(context.I18n);
});
