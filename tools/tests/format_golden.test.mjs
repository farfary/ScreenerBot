// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

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
import { readServerOnlyDomains } from "../i18n/catalogs.mjs";

const WEBSERVER = new URL("../../src/webserver/", import.meta.url);
const read = (rel) => fs.readFileSync(new URL(rel, WEBSERVER), "utf8");
const FIXTURE = JSON.parse(
  fs.readFileSync(new URL("./fixtures/format_golden.json", import.meta.url), "utf8")
);
const BUNDLE_JS = read("assets/fluent-bundle.js");
const RUNTIME_JS = read("templates/scripts/core/i18n.js");
const FORMAT_JS = read("templates/scripts/core/format.js");

// Domains the server keeps out of the dashboard payload.
const SERVER_ONLY = readServerOnlyDomains();

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
function load(intlLocale, dir = "ltr") {
  const context = {
    console: { warn() {}, error() {}, debug() {}, log() {} },
    document: { addEventListener() {}, getElementById: () => null },
  };
  context.window = context;
  context.__SCREENERBOT_L10N__ = {
    locale: intlLocale.split("-")[0],
    intlLocale,
    dir,
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
  // Compact figures follow the locale's compact system: de has no short thousands step.
  assert.equal(fmt.formatCurrencyUSD(1234.5678), "$1234,57");
  assert.equal(fmt.formatCurrencyUSD(1234567.891), "$1,23\u00a0Mio.");
  assert.equal(fmt.formatPriceSol(0.000123456789, { decimals: 9 }), "0,000123457");
  assert.equal(fmt.formatPercentValue(12.345), "+12,35\u00a0%");
  assert.equal(fmt.formatPercent(-3.14159, { style: "plain", decimals: 3 }), "-3,142\u00a0%");
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
  assert.equal(de.formatPriceSubscript(0.0000000012345), "0,0₈1235");
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
  // format-native-amount is the only catalog message carrying an $amount placeable.
  const render = (type, value) =>
    I18n.text({ id: "format-native-amount", args: { amount: { type, value } } }).replace(
      /[\u2068\u2069]/g,
      ""
    );
  assert.equal(render("sol", 1.5), "1.5 SOL SOL");
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
  // Detailed spans carry seconds only below an hour and never a zero part.
  assert.equal(fmt.formatUptime(11520), "3h 12m");
  assert.equal(fmt.formatUptime(20160), "5h 36m");
  assert.equal(fmt.formatUptime(86460), "1d 1m");
  assert.equal(fmt.formatUptime(303), "5m 3s");
  assert.equal(fmt.formatUptime(120), "2m");
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

test("surface formatter options: trimmed fixed text, extra decimals, zero sign and address marks", () => {
  const { fmt } = load("en-u-nu-latn");
  assert.equal(fmt.formatNumber(1234.56789, { decimals: 0, maxDecimals: 3 }), "1,234.568");
  assert.equal(fmt.formatNumber(5, { decimals: 0, maxDecimals: 3 }), "5");
  assert.equal(fmt.formatNumber(1234.5, 2), "1,234.50");
  assert.equal(fmt.formatFixed(0.5, { decimals: 4, trim: true }), "0.5");
  assert.equal(fmt.formatFixed(10, { decimals: 2, trim: true }), "10");
  assert.equal(fmt.formatFixed(-0.00001, { decimals: 4, trim: true }), "0");
  assert.equal(fmt.formatFixed(0.0000005, { decimals: 9, trim: true }), "0.0000005");
  assert.equal(fmt.formatFixed(0.5, { decimals: 4 }), "0.5000");
  assert.equal(fmt.formatTimeSpan(3, { decimals: 1, trim: true }), "3s");
  assert.equal(fmt.formatTimeSpan(3.4, { decimals: 1, trim: true }), "3.4s");
  assert.equal(fmt.formatTimeSpan(820, { unit: "millisecond" }), "820ms");
  assert.equal(fmt.formatPercentValue(0, { signZero: true }), "+0.00%");
  assert.equal(fmt.formatPercentValue(0), "0.00%");
  assert.equal(fmt.formatPercentValue(-1.5, { signZero: true }), "-1.50%");
  assert.equal(fmt.formatPercentValue(-1.5, { includeSign: false }), "1.50%");
  assert.equal(fmt.formatPercentValue(2, { plus: "", trim: true }), "2%");
  assert.equal(fmt.formatPercentValue(2.5, { plus: "", trim: true }), "2.5%");
  assert.equal(fmt.formatPercentValue(-0.001, { trim: true }), "0%");
  assert.equal(fmt.formatSol(0.05, { decimals: 4, trim: true }), "0.05 SOL");
  assert.equal(fmt.formatSol(2, { decimals: 4, trim: true }), "2 SOL");
  assert.equal(fmt.formatSol(0.00004, { decimals: 4, trim: true }), "0 SOL");
  // A free-standing amount drops padded zeros; a table cell keeps its column's fixed count.
  assert.equal(fmt.formatSol(0.05), "0.05 SOL");
  assert.equal(fmt.formatSol(0.05, { decimals: 4, trim: false }), "0.0500 SOL");
  assert.equal(fmt.formatSignedSol(0.25), "+0.25 SOL");
  assert.equal(fmt.formatSignedSol(0.25, { minDecimals: 4, unit: false }), "+0.2500");
  assert.match(fmt.formatPnL(-1.5), />-1\.5 SOL</);
  assert.match(fmt.formatPnL(-1.5, { trim: false, unit: false }), />-1\.5000</);
  assert.equal(
    fmt.formatPercentValue(2, { decimals: 0, signZero: true, includeSign: true }),
    "+2%"
  );
  assert.equal(fmt.formatUptime(30, { style: "hm" }), "<1m");
  assert.equal(fmt.formatUptime(300, { style: "hm" }), "5m");
  assert.equal(fmt.formatUptime(3660, { style: "hm" }), "1h 1m");
  assert.equal(fmt.formatUptime(108300, { style: "hm" }), "30h 5m");
  assert.equal(fmt.formatUptime(7200, { style: "hm" }), "2h 0m");
});

test("a month title starts with a capital where the locale writes month names lowercase", () => {
  assert.equal(load("ru-u-nu-latn").fmt.formatMonthYear(2026, 9), "Сентябрь 2026 г.");
  assert.equal(load("en-u-nu-latn").fmt.formatMonthYear(2026, 9), "September 2026");
});

test("the calendar week starts on the locale's first day", () => {
  assert.equal(load("en-u-nu-latn").fmt.firstDayOfWeek(), 0);
  assert.equal(load("de-u-nu-latn").fmt.firstDayOfWeek(), 1);
  assert.equal(load("fa-u-nu-latn").fmt.firstDayOfWeek(), 6);
});

test("every compact figure uses one compact system per locale", () => {
  const zh = load("zh-Hans-u-nu-latn").fmt;
  assert.equal(zh.formatCurrencyUSD(812350), "$81.24万");
  assert.equal(zh.formatCompactFixed(812350), "81.24万");
  assert.equal(zh.formatCompactNumber(812350, { usd: true }), "$81.24万");
  const ar = load("ar-u-nu-latn").fmt;
  assert.equal(ar.formatCurrencyUSD(810060), ar.formatCompactNumber(810060, { usd: true }));
});

test("a compact dollar amount trims free-standing and keeps both digits in a column", () => {
  const { fmt } = load("en-u-nu-latn");
  assert.equal(fmt.formatCurrencyUSD(9_900_000), "$9.9M");
  assert.equal(fmt.formatCurrencyUSD(462_000_000), "$462M");
  const column = [17_480_000, 12_400_000, 9_900_000, 887_980, 1_000_000_000].map((value) =>
    fmt.formatCurrencyUSD(value, { trim: false })
  );
  assert.deepEqual(column, ["$17.48M", "$12.40M", "$9.90M", "$887.98K", "$1.00B"]);
  for (const text of column)
    assert.match(text, /\.\d{2}[KMB]$/, `${text} keeps two fraction digits`);
  assert.equal(
    load("de-u-nu-latn").fmt.formatCurrencyUSD(9_900_000, { trim: false }),
    "$9,90\u00a0Mio."
  );
});

test("de keeps digits for the surface formatters; only separators change", () => {
  const { fmt } = load("de-u-nu-latn");
  assert.equal(fmt.formatCompactFixed(1234567), "1,23\u00a0Mio.");
  assert.equal(fmt.formatFixed(0.5, { decimals: 4, trim: true }), "0,5");
  assert.equal(fmt.formatLatencyMs(1500), "1,50s");
  assert.equal(fmt.formatMemoryMb(1536), "1,5GB");
  assert.equal(fmt.formatSizeAt(1.5, { unit: "mb", decimals: 1 }), "1,5 MB");
  assert.equal(fmt.formatNumber(1234.56789, { decimals: 0, maxDecimals: 3 }), "1.234,568");
});

test("signed plain numbers group per locale and take the locale's sign", () => {
  const de = load("de-u-nu-latn").fmt;
  assert.equal(de.formatSignedNumber(1234567.891, { decimals: 2 }), "+1.234.567,89");
  assert.equal(de.formatSignedNumber(-0.5, { decimals: 1 }), "-0,5");
  const ar = load("ar-u-nu-latn").fmt;
  assert.equal(ar.formatSignedNumber(1234.5, { decimals: 2 }), "‎+1,234.50");
  assert.equal(ar.formatSignedNumber(-1234.5, { decimals: 2 }), "‎-1,234.50");
  assert.equal(ar.formatSignedNumber(-0.004, { decimals: 2 }), "0.00");
  assert.equal(ar.signedTone(-0.00004, 4), "neutral");
  const hi = load("hi-u-nu-latn").fmt;
  assert.equal(hi.formatSignedNumber(1234567.5, { decimals: 1 }), "+12,34,567.5");
});

test("a price change signs both directions and leaves zero unsigned", () => {
  const en = load("en-u-nu-latn").fmt;
  assert.equal(en.formatPriceSubscript(0.00000123, { sign: "always" }), "+0.0₅123");
  assert.equal(en.formatPriceSubscript(-0.00000123, { sign: "always" }), "-0.0₅123");
  assert.equal(en.formatPriceSubscript(12.5, { sign: "always" }), "+12.5");
  assert.equal(en.formatPriceSubscript(0, { sign: "always" }), "0");
  assert.equal(load("ar-u-nu-latn").fmt.formatPriceSubscript(12.5, { sign: "always" }), "‎+12.5");
});

test("a percentage that rounds to zero is shown unsigned and neutral", () => {
  for (const locale of ["en-u-nu-latn", "ar-u-nu-latn", "de-u-nu-latn"]) {
    const { fmt } = load(locale);
    for (const decimals of [0, 1, 2]) {
      const step = 10 ** -decimals;
      for (const value of [step * 0.49, -step * 0.49, step * 0.01, -step * 0.01, 1e-9, -1e-9]) {
        const zero = fmt.formatPercentValue(0, { decimals });
        assert.equal(fmt.formatPercentValue(value, { decimals }), zero, `${locale} ${value}`);
        assert.equal(fmt.signedTone(value, decimals), "neutral", `${locale} ${value}`);
        assert.match(fmt.formatPercent(value, { style: "pnl", decimals }), /pnl-neutral/);
      }
    }
    assert.notEqual(
      fmt.formatPercentValue(-0.6, { decimals: 0 }),
      fmt.formatPercentValue(0, { decimals: 0 })
    );
  }
});

test("a Gregorian calendar grid names Gregorian months in fa, with Latin digits", () => {
  const fa = load("fa-u-nu-latn").fmt;
  const day = Date.UTC(2026, 8, 19);
  assert.equal(fa.formatMonthYear(2026, 9, { calendar: "gregory" }), "سپتامبر 2026");
  assert.equal(
    fa.formatDate(day, { includeYear: false, weekday: true, utc: true, calendar: "gregory" }),
    "شنبه 19 سپتامبر"
  );
  assert.equal(fa.formatDate(day, { utc: true, calendar: "gregory" }), "19 سپتامبر 2026");
  // Other surfaces keep the locale's own calendar.
  assert.equal(fa.formatMonthYear(2026, 9), "1405 شهریور");
  const en = load("en-u-nu-latn").fmt;
  assert.equal(en.formatMonthYear(2026, 9, { calendar: "gregory" }), "September 2026");
});

test("a SOL price shows significant digits without trailing zeros", () => {
  const { fmt } = load("en-u-nu-latn");
  const shown = {
    0.0000092: "0.0₅92",
    22.6796: "22.68",
    0.00000012: "0.0₆12",
    0.004494: "0.004494",
    1.5: "1.5",
  };
  for (const [price, text] of Object.entries(shown)) {
    assert.equal(fmt.formatPriceSubscript(Number(price)), text, `price ${price}`);
  }
});

test("dashboard pages show prices through the significant-digit formatter", () => {
  // `formatPriceSol` prints a fixed decimal count; it stays for plain-text reports in
  // core/utils.js and never reaches a page, a table cell or a dialog. No other
  // fixed-decimal formatter takes a price either: a fixed count prints a run of zeros
  // for a small price, where `formatPriceSubscript` prints the zero count (0.0₅92).
  // `formatCurrencyUSD` is the USD owner and uses the same zero count below a cent.
  // The one exception is the exact value (`decimals: 12, trim: true`) kept for a
  // title or a copied report.
  const FIXED_PRICE =
    /\b(?:formatSol|formatFixed|formatCompactFixed|formatNumber)\(\s*([^,)]*price[^,)]*)|\b\w*price\w*\.toFixed\(/gi;
  const NOT_A_PRICE = /change|percent|pct|priced|count|summary|with_pool_price/i;
  const scripts = new URL("templates/scripts/", WEBSERVER);
  const offenders = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const path = new URL(entry.name + (entry.isDirectory() ? "/" : ""), dir);
      if (entry.isDirectory()) walk(path);
      else if (entry.name.endsWith(".js")) {
        const rel = path.pathname.slice(scripts.pathname.length);
        if (rel === "core/format.js" || rel === "core/utils.js") continue;
        const source = fs.readFileSync(path, "utf8");
        if (source.includes("formatPriceSol")) offenders.push(rel);
        source.split("\n").forEach((line, index) => {
          for (const match of line.matchAll(FIXED_PRICE)) {
            if (NOT_A_PRICE.test(match[1] ?? match[0])) continue;
            if (/decimals: 12, trim: true/.test(line)) continue;
            offenders.push(`${rel}:${index + 1}`);
          }
        });
      }
    }
  };
  walk(scripts);
  assert.deepEqual(offenders, []);
});

/**
 * In a right-to-left page a formatter's unit or word value is wrapped in a first-strong
 * isolate, so "134 MB" cannot split into "MB 134" inside RTL text. Apart from the isolation
 * marks the output is exactly the left-to-right output: no step, rounding or text changes.
 */
test("right-to-left formatter output differs from left-to-right only by isolation marks", () => {
  const ltr = load("en-u-nu-latn");
  const rtl = load("en-u-nu-latn", "rtl");
  const expected = run(ltr.fmt, ltr.context, FIXTURE.cases);
  const actual = run(rtl.fmt, rtl.context, FIXTURE.cases);
  const failures = [];
  FIXTURE.cases.forEach((testCase, index) => {
    const before = expected[index].out;
    const after = actual[index].out;
    if (typeof before !== "string") return;
    if (after.replace(/[\u2068\u2069]/g, "") !== before)
      failures.push(`${testCase.fn}: ${JSON.stringify(before)} -> ${JSON.stringify(after)}`);
  });
  assert.deepEqual(failures, []);
  for (const [fn, value] of [
    ["formatMemoryMb", 134],
    ["formatLatencyMs", 382],
    ["formatSol", 0.5],
  ])
    assert.equal(rtl.fmt[fn](value), `\u2068${ltr.fmt[fn](value)}\u2069`, fn);
});

test("every formatUptime caller names a style the formatter implements", () => {
  const STYLES = new Set(["detailed", "hm", "compact", "trimmed"]);
  const scripts = new URL("templates/scripts/", WEBSERVER);
  const files = fs
    .readdirSync(scripts, { recursive: true })
    .filter((name) => name.endsWith(".js") || name.endsWith(".mjs"));
  const unknown = [];
  for (const name of files) {
    const source = fs.readFileSync(new URL(name, scripts), "utf8");
    for (const match of source.matchAll(/formatUptime\([^;]*?style:\s*"([^"]+)"/g)) {
      if (!STYLES.has(match[1])) unknown.push(`${name}: ${match[1]}`);
    }
  }
  assert.deepEqual(unknown, []);
});

test("every formatTimestamp caller passes only options the formatter implements", () => {
  const OPTIONS = new Set(["fallback", "includeSeconds", "includeYear", "includeDate"]);
  const scripts = new URL("templates/scripts/", WEBSERVER);
  const files = fs
    .readdirSync(scripts, { recursive: true })
    .filter((name) => name.endsWith(".js") || name.endsWith(".mjs"));
  const unknown = [];
  for (const name of files) {
    const source = fs.readFileSync(new URL(name, scripts), "utf8");
    for (const match of source.matchAll(/formatTimestamp\([^;{)]*,\s*\{([^}]*)\}/g)) {
      for (const key of match[1].matchAll(/(\w+)\s*:/g)) {
        if (!OPTIONS.has(key[1])) unknown.push(`${name}: ${key[1]}`);
      }
    }
  }
  assert.deepEqual(unknown, []);
});

test("a price column keeps one significant-digit count across both notations", () => {
  const { fmt } = load("en-u-nu-latn");
  const column = { trim: false };
  const shown = {
    0.0004249: "0.0004249",
    0.00046152: "0.0004615",
    0.0223: "0.02230",
    0.00005101: "0.0₄5101",
    0.00005: "0.0₄5000",
    22.6796: "22.68",
  };
  for (const [price, text] of Object.entries(shown)) {
    assert.equal(fmt.formatPriceSubscript(Number(price), column), text, `price ${price}`);
  }
  // A free-standing price still drops the padding.
  assert.equal(fmt.formatPriceSubscript(0.0223), "0.0223");
});

test("the subscript starts at four leading zeros and both notations round", () => {
  const { fmt } = load("en-u-nu-latn");
  assert.equal(fmt.formatPriceSubscript(0.0001), "0.0001");
  assert.equal(fmt.formatPriceSubscript(0.000099), "0.0₄99");
  // Rounded, not truncated, in both notations.
  assert.equal(fmt.formatPriceSubscript(0.0000123456), "0.0₄1235");
  assert.equal(fmt.formatPriceSubscript(0.000123456), "0.0001235");
  // A price that rounds up to the threshold leaves the subscript.
  assert.equal(fmt.formatPriceSubscript(0.0000999999, { trim: false }), "0.0001000");
  assert.equal(fmt.formatPriceSubscript(1e-25), "0.0₂₄1");
});
