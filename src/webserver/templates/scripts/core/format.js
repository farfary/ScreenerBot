// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Dashboard value formatters: the single owner of number, money, percent, date,
 * relative time, size and duration display.
 *
 * Locale rules:
 * - Every Intl instance uses `I18n.intlLocale`, which pins Latin digits. The
 *   locale is never a caller's choice.
 * - Precision, rounding and decimal counts are locale independent. Values that
 *   are rounded with `toFixed` keep that digit string and only swap the decimal
 *   separator for the locale's own. Intl rounds shortest-representation
 *   decimals (1.005 -> 1.01) where `toFixed` rounds the exact binary value
 *   (1.005 -> 1.00), so Intl is used for rounding only where it always was.
 * - Words and unit names come from the `format-*` catalog messages. Symbols
 *   (`K`/`M`/`B`, the fallback dashes) are code constants. The plus and
 *   minus signs come from Intl (`signPrefix`) so a locale's direction marks are kept.
 *
 * Loaded after `core/i18n.js`, which provides the `I18n` global.
 */

const DASH = "—";
const HYPHEN = "-";
const SUBSCRIPT_DIGITS = "₀₁₂₃₄₅₆₇₈₉";

// Fluent wraps placeables in bidi isolates. Formatter output is a compact
// display string compared, sliced and diffed by callers, so the marks are removed.
const ISOLATES = /[\u2066-\u2069]/g;

/**
 * Text without bidi isolation marks (U+2066-U+2069), for strings that are compared or
 * sliced and for surfaces that show the controls literally, such as the document title.
 */
export function stripIsolates(text) {
  return String(text).replace(ISOLATES, "");
}

const plain = stripIsolates;

const instances = new Map();

/** Memoized Intl instance for the active locale, keyed by constructor and options. */
function intl(Ctor, options) {
  const key = `${Ctor.name}:${JSON.stringify(options)}`;
  let instance = instances.get(key);
  if (!instance) {
    instance = new Ctor(I18n.intlLocale, options);
    instances.set(key, instance);
  }
  return instance;
}

let decimalMark = null;

function decimalSeparator() {
  if (decimalMark === null) {
    const parts = intl(Intl.NumberFormat, {
      minimumFractionDigits: 1,
      useGrouping: false,
    }).formatToParts(1.1);
    decimalMark = parts.find((part) => part.type === "decimal")?.value ?? ".";
  }
  return decimalMark;
}

const signPrefixes = new Map();

/**
 * The active locale's plus or minus sign, including any direction marks the locale
 * places around it, so a sign concatenated before a number stays bidi-safe.
 * Digits are never produced here; rounding stays with the caller.
 */
function signPrefix(negative) {
  const key = negative ? "-" : "+";
  let prefix = signPrefixes.get(key);
  if (prefix === undefined) {
    const parts = intl(Intl.NumberFormat, {
      signDisplay: "always",
      useGrouping: false,
    }).formatToParts(negative ? -1 : 1);
    const integerAt = parts.findIndex((part) => part.type === "integer");
    prefix = parts
      .slice(0, integerAt)
      .map((part) => part.value)
      .join("");
    signPrefixes.set(key, prefix);
  }
  return prefix;
}

/** Swap the ASCII decimal point of a digit string for the locale's separator. */
function localizeDecimal(text) {
  return text.replace(".", () => decimalSeparator());
}

const counted = (count, amount = String(count)) => ({ count, amount });

const notAvailable = () => plain(I18n.t("format-not-available"));

/** A caller-supplied value, or the catalog default built only when none was given. */
const orElse = (value, make) => (value === undefined ? make() : value);

const ago = {
  second: (n) => plain(I18n.t("format-ago-second", counted(n))),
  minute: (n) => plain(I18n.t("format-ago-minute", counted(n))),
  hour: (n) => plain(I18n.t("format-ago-hour", counted(n))),
  day: (n) => plain(I18n.t("format-ago-day", counted(n))),
};

const until = {
  second: (n) => plain(I18n.t("format-in-second", counted(n))),
  minute: (n) => plain(I18n.t("format-in-minute", counted(n))),
  hour: (n) => plain(I18n.t("format-in-hour", counted(n))),
  day: (n) => plain(I18n.t("format-in-day", counted(n))),
};

const unit = {
  day: (n, amount) => plain(I18n.t("format-unit-day", counted(n, amount))),
  hour: (n, amount) => plain(I18n.t("format-unit-hour", counted(n, amount))),
  minute: (n, amount) => plain(I18n.t("format-unit-minute", counted(n, amount))),
  second: (n, amount) => plain(I18n.t("format-unit-second", counted(n, amount))),
  millisecond: (n, amount) => plain(I18n.t("format-unit-millisecond", counted(n, amount))),
  microsecond: (n, amount) => plain(I18n.t("format-unit-microsecond", counted(n, amount))),
  nanosecond: (n, amount) => plain(I18n.t("format-unit-nanosecond", counted(n, amount))),
};

const size = {
  b: (n, amount) => plain(I18n.t("format-bytes-b", counted(n, amount))),
  kb: (n, amount) => plain(I18n.t("format-bytes-kb", counted(n, amount))),
  mb: (n, amount) => plain(I18n.t("format-bytes-mb", counted(n, amount))),
  gb: (n, amount) => plain(I18n.t("format-bytes-gb", counted(n, amount))),
};

function coerceNumber(value) {
  if (value === null || value === undefined || value === "") {
    return Number.NaN;
  }
  const num = Number(value);
  return Number.isFinite(num) ? num : Number.NaN;
}

export function formatNumber(value, decimalsOrOptions = 2, maybeOptions = {}) {
  let decimals = decimalsOrOptions;
  let options = maybeOptions;

  if (typeof decimalsOrOptions === "object" && decimalsOrOptions !== null) {
    options = decimalsOrOptions;
    decimals = options.decimals ?? 2;
  }

  const { fallback = DASH, useGrouping = true, maxDecimals = decimals } = options || {};

  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  return intl(Intl.NumberFormat, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: Math.max(decimals, maxDecimals),
    useGrouping,
  }).format(num);
}

/** Drop trailing fraction zeros of a `toFixed` digit string ("0.500" -> "0.5", "-0.00" -> "0"). */
function trimZeros(text) {
  if (!text.includes(".")) return text;
  const trimmed = text.replace(/\.?0+$/, "");
  return trimmed === "-0" ? "0" : trimmed;
}

/**
 * A number at a fixed count of decimals, without grouping. Rounds the exact
 * binary value like `toFixed`, so it matches the money formatters digit for digit.
 * `trim` drops trailing fraction zeros.
 */
export function formatFixed(value, { decimals = 2, fallback = DASH, trim = false } = {}) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const text = num.toFixed(decimals);
  return localizeDecimal(trim ? trimZeros(text) : text);
}

/**
 * Compact notation ("1.50K", "81.24万", "1,23 Mio.") at the given fraction digits. The
 * locale owns the scale steps and their symbols, so every compact figure on a page
 * agrees on one system.
 */
function compactText(num, minimumFractionDigits, maximumFractionDigits = minimumFractionDigits) {
  return plain(
    intl(Intl.NumberFormat, { notation: "compact", minimumFractionDigits, maximumFractionDigits }).format(num)
  );
}

let kmbSteps = null;

/** Whether the locale's compact steps are the `K`/`M`/`B` thousands steps. */
function usesKmbSteps() {
  if (kmbSteps === null) {
    kmbSteps = compactText(1e3, 0) === "1K" && compactText(1e6, 0) === "1M" && compactText(1e9, 0) === "1B";
  }
  return kmbSteps;
}

/**
 * A compact figure (magnitude of at least one thousand) at exactly `decimals` fraction
 * digits. Under `K`/`M`/`B` steps a figure keeps its own step where Intl would change it:
 * a value whose rounding reaches 1000 of its step ("1000.00K"), a magnitude past the
 * billions ("1000.00B"), and past the millions when `billions` is off ("2500.0M").
 */
function compactFixed(num, decimals, billions = true) {
  if (usesKmbSteps()) {
    const abs = Math.abs(num);
    const [divisor, suffix] = billions && abs >= 1e9 ? [1e9, "B"] : abs >= 1e6 ? [1e6, "M"] : [1e3, "K"];
    const digits = (num / divisor).toFixed(decimals);
    if (Math.abs(Number(digits)) >= 1000) {
      return `${localizeDecimal(digits)}${suffix}`;
    }
  }
  return compactText(num, decimals);
}

/**
 * A magnitude of at least one thousand in the locale's compact notation at fixed
 * decimals ("1.50K"); values below one thousand use `belowDecimals` (optionally
 * trimmed). `billions: false` keeps `K`/`M`/`B` steps at `M` for surfaces that never show `B`.
 */
export function formatCompactFixed(
  value,
  { decimals = 2, belowDecimals = decimals, trimBelow = false, billions = true, fallback = DASH } = {}
) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const abs = Math.abs(num);
  if (abs >= 1e3) return compactFixed(num, decimals, billions);
  return formatFixed(num, { decimals: belowDecimals, trim: trimBelow });
}

export function formatCompactNumber(value, digitsOrOptions = 2, maybeFallback = DASH) {
  // Support both (value, digits, fallback) and (value, { digits, fallback, usd })
  let digits = digitsOrOptions;
  let fallback = maybeFallback;
  let usd = false;

  if (typeof digitsOrOptions === "object" && digitsOrOptions !== null) {
    digits = digitsOrOptions.digits ?? 2;
    fallback = digitsOrOptions.fallback ?? DASH;
    usd = digitsOrOptions.usd ?? false;
  }

  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  const formatted = compactText(num, 0, digits);

  return usd ? withUsdSymbol(formatted) : formatted;
}

export function formatBooleanFlag(value, unknownLabel) {
  if (value === true) return plain(I18n.t("format-yes"));
  if (value === false) return plain(I18n.t("format-no"));
  return orElse(unknownLabel, () => plain(I18n.t("format-unknown")));
}

/** An already formatted US dollar amount with the dollar symbol ("1.23K" -> "$1.23K"). */
export function withUsdSymbol(amount) {
  return plain(I18n.t("format-usd-amount", { amount: String(amount) }));
}

/** An already formatted percentage number with the percent sign ("12.5" -> "12.5%"). */
export function withPercentUnit(amount) {
  return plain(I18n.t("format-percent-amount", { amount: String(amount) }));
}

/** An already formatted amount marked approximate ("$1.23K" -> "≈ $1.23K"). */
export function withApprox(text) {
  return plain(I18n.t("format-approx", { value: String(text) }));
}

export function formatCurrencyUSD(value, { fallback = DASH, approx = false } = {}) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const text = usdText(num);
  return approx ? withApprox(text) : text;
}

function usdText(num) {
  const abs = Math.abs(num);
  if (abs >= 1_000) {
    return withUsdSymbol(compactFixed(num, 2));
  }
  if (abs > 0 && abs < 0.01) {
    // Sub-cent prices round to $0.00 with toFixed(2); render the real value in
    // subscript notation (e.g. $0.0₅8142) so tiny token prices stay visible.
    return withUsdSymbol(formatPriceSubscript(num, { precision: 4 }));
  }

  return withUsdSymbol(localizeDecimal(num.toFixed(2)));
}

/**
 * Format price with subscript notation for very small numbers
 * Uses subscript notation: 0.0₉12345 means 0.000000000012345 (9 zeros after decimal)
 * @param {number} price - The price to format
 * @param {Object} options - Formatting options
 * @param {string} options.fallback - Value to return if price is invalid
 * @param {number} options.precision - Number of significant digits
 * @param {string} options.sign - "negative" signs only negatives; "always" also adds the
 *   locale's plus to a positive (a price change). Zero stays unsigned.
 * @returns {string} Formatted price string
 */
export function formatPriceSubscript(price, { fallback = DASH, precision = 5, sign: signMode = "negative" } = {}) {
  const num = coerceNumber(price);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  if (num === 0) return "0";

  const absPrice = Math.abs(num);
  const sign = num < 0 ? signPrefix(true) : signMode === "always" ? signPrefix(false) : "";

  // Normal-sized numbers (>= 0.0001) get the SAME significant-digit budget as
  // the subscript branch below: decimals are derived from the magnitude, never
  // a flat toFixed(). A flat toFixed printed 0.000105191666 for a price whose
  // meaningful part is 0.00010519, which overflowed every column and tooltip
  // it landed in. Prices >= 1 keep at least 2 decimals so they still read as
  // amounts rather than rounded integers.
  if (absPrice >= 0.0001) {
    const magnitude = Math.floor(Math.log10(absPrice));
    const minDecimals = absPrice >= 1 ? 2 : 0;
    const decimals = Math.min(12, Math.max(minDecimals, precision - 1 - magnitude));
    const formatted = absPrice.toFixed(decimals);
    return (
      sign + localizeDecimal(formatted.includes(".") ? formatted.replace(/\.?0+$/, "") : formatted)
    );
  }

  // Count leading zeros after decimal
  const str = absPrice.toFixed(20);
  const match = str.match(/^0\.0*/);
  if (!match) return sign + localizeDecimal(absPrice.toPrecision(precision));

  const leadingZeros = match[0].length - 2; // Subtract "0."

  // Get significant digits after zeros. Trailing zeros are padding from the
  // fixed-20 expansion, not precision — 0.0₇50000 is the same number as
  // 0.0₇5 and only makes the column wider.
  const significantPart = str.substring(match[0].length);
  const significant = significantPart
    .substring(0, Math.min(precision, significantPart.length))
    .replace(/0+$/, "");

  // Use subscript for zero count
  let subscript = "";
  const zeroStr = leadingZeros.toString();
  for (const char of zeroStr) {
    subscript += SUBSCRIPT_DIGITS[parseInt(char, 10)];
  }

  return `${sign}0${decimalSeparator()}0${subscript}${significant}`;
}

export function formatPriceSol(price, { fallback, decimals = 12 } = {}) {
  const num = coerceNumber(price);
  if (!Number.isFinite(num)) {
    return orElse(fallback, notAvailable);
  }

  const desired = Math.floor(Number(decimals));
  const precision = Number.isFinite(desired) && desired >= 0 ? desired : 12;
  const boundedPrecision = precision > 12 ? 12 : precision;
  const formatted = num.toFixed(boundedPrecision);
  if (Object.is(num, -0)) {
    return localizeDecimal(`0.${"0".repeat(boundedPrecision)}`);
  }
  return localizeDecimal(formatted);
}

/**
 * Move the decimal point of a non-negative `toFixed` digit string two places left
 * ("12.35" -> "0.1235"), the exact decimal form of value / 100. Intl percent
 * formatting scales by 100, so feeding it this string reproduces the `toFixed`
 * digits with no binary rounding in between.
 */
function shiftPercent(digits) {
  const [whole, fraction = ""] = digits.split(".");
  const all = whole + fraction;
  const point = whole.length - 2;
  return point > 0 ? `${all.slice(0, point)}.${all.slice(point) || "0"}` : `0.${"0".repeat(-point)}${all}`;
}

/**
 * A percentage at fixed decimals through Intl percent style, so the symbol, its
 * spacing and the sign placement follow the locale. The digits are the `toFixed`
 * digits of |value|. `sign`: "auto" shows a minus for negatives and a plus for
 * positives, "negative" only the minus, "none" no sign, "always" plus for zero too.
 */
function percentText(num, decimals, sign) {
  const digits = Math.abs(num).toFixed(decimals);
  const negative = num < 0;
  if (!/^\d+(\.\d+)?$/.test(digits)) {
    // Magnitudes from 1e21 stringify in exponent form, which has no decimal
    // shift; they keep their exponent text and are not locale placed.
    const shown = negative ? signPrefix(true) : sign === "auto" || sign === "always" ? signPrefix(false) : "";
    return `${shown}${localizeDecimal(digits)}%`;
  }
  const magnitude = shiftPercent(digits);
  let signDisplay = "never";
  if (negative && sign !== "none") signDisplay = "always";
  else if (!negative && (sign === "always" || (sign === "auto" && num > 0))) signDisplay = "always";
  return plain(
    intl(Intl.NumberFormat, {
      style: "percent",
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
      useGrouping: false,
      signDisplay,
    }).format(negative ? `-${magnitude}` : magnitude)
  );
}

export function formatPercentValue(
  value,
  { fallback = DASH, decimals = 2, includeSign = true, plus = "+", signZero = false } = {}
) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  if (!includeSign) {
    return percentText(Math.abs(num), decimals, "none");
  }
  if (num === 0) {
    return percentText(0, decimals, signZero && plus === "+" ? "always" : "none");
  }
  return percentText(num, decimals, plus === "+" ? "auto" : "negative");
}

export function formatPercent(value, { style = "plain", decimals = 2, fallback = HYPHEN } = {}) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    if (style === "token") {
      return `<span>${fallback}</span>`;
    }
    return fallback;
  }

  const text = percentText(num === 0 ? 0 : num, decimals, "auto");

  if (style === "token") {
    const color = num > 0 ? "#16a34a" : num < 0 ? "#ef4444" : "inherit";
    return `<span style="color:${color};">${text}</span>`;
  }

  if (style === "pnl") {
    const tone = num > 0 ? "positive" : num < 0 ? "negative" : "neutral";
    return `<span class="pnl-${tone}">${text}</span>`;
  }

  return text;
}

export function formatSol(amount, { decimals = 4, fallback = HYPHEN, suffix } = {}) {
  const num = coerceNumber(amount);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const formatted = localizeDecimal(num.toFixed(decimals));
  if (suffix === undefined) {
    return plain(I18n.t("format-native-amount", { amount: formatted }));
  }
  return `${formatted}${suffix}`;
}

/** An already formatted elapsed span with the "ago" wording ("3h 5m" -> "3h 5m ago"); `count` selects the plural form. */
export function withAgo(span, count = 0) {
  return plain(I18n.t("format-ago-span", counted(count, span)));
}

/** An already formatted SOL amount with the SOL term ("0.1500" -> "0.1500 SOL"). */
export function withSolUnit(amount) {
  return plain(I18n.t("format-native-amount", { amount }));
}

/**
 * The `toFixed` magnitude digits of a signed amount as it will be shown, and whether
 * they read as zero. `minDecimals` drops trailing fraction zeros down to that count.
 * Every signed formatter and `signedTone` take the sign from this rounding, so a value
 * that rounds to zero is unsigned and neutral everywhere.
 */
function roundedMagnitude(num, decimals, minDecimals = decimals) {
  let digits = Math.abs(num).toFixed(decimals);
  if (minDecimals < decimals) {
    digits = digits.replace(new RegExp(`(\\.\\d{${minDecimals}}\\d*?)0+$`), "$1").replace(/\.$/, "");
  }
  return { digits, zero: Number(digits) === 0 };
}

/** A `toFixed` digit string with the locale's grouping and decimal separator, digits unchanged. */
function groupedDecimal(digits) {
  if (!/^\d+(\.\d+)?$/.test(digits)) return localizeDecimal(digits);
  const fraction = digits.split(".")[1]?.length ?? 0;
  // A decimal string is formatted exactly, with no binary rounding in between.
  return plain(
    intl(Intl.NumberFormat, {
      minimumFractionDigits: fraction,
      maximumFractionDigits: fraction,
      useGrouping: true,
    }).format(digits)
  );
}

/** Signed digits of a finite number: the locale's sign (per `sign`) before the rounded magnitude. */
function signedDigits(num, { decimals, minDecimals = decimals, sign = "always", grouping = false }) {
  const { digits, zero } = roundedMagnitude(num, decimals, minDecimals);
  const shown = zero || (num > 0 && sign !== "always") ? "" : signPrefix(num < 0);
  return `${shown}${grouping ? groupedDecimal(digits) : localizeDecimal(digits)}`;
}

/**
 * Tone of a signed amount as shown at `decimals`: "positive", "negative", or "neutral"
 * for a value that rounds to zero (or is not a number). Colour classes take this, never
 * the raw value, so a displayed "0.0000" is never green or red.
 */
export function signedTone(value, decimals = 4) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num) || roundedMagnitude(num, decimals).zero) {
    return "neutral";
  }
  return num > 0 ? "positive" : "negative";
}

/**
 * A signed SOL amount ("+0.1500 SOL", "-0.0077 SOL"). The sign is the locale's own
 * (`signPrefix`), attached to the digits before the unit, so it stays at the number's
 * start in right-to-left text. The sign follows the rounded digits: a value that rounds
 * to zero shows none. `sign`: "always" signs both directions, "negative" only losses.
 * `minDecimals` drops trailing fraction zeros down to that count ("0.005000" -> "0.005").
 * `unit: false` returns the signed digits alone.
 */
export function formatSignedSol(
  amount,
  { decimals = 4, minDecimals = decimals, fallback = HYPHEN, unit: withUnit = true, sign = "always" } = {}
) {
  const num = coerceNumber(amount);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const text = signedDigits(num, { decimals, minDecimals, sign });
  return withUnit ? withSolUnit(text) : text;
}

/**
 * A signed plain number with the locale's grouping ("+1,234.50", "-0.25"), such as a
 * token amount change. Sign and rounding follow `formatSignedSol`: the sign is the
 * locale's own and comes from the `toFixed` digits, so a value that rounds to zero
 * shows none. `sign`: "always" signs both directions, "negative" only decreases.
 */
export function formatSignedNumber(
  value,
  { decimals = 2, minDecimals = decimals, sign = "always", fallback = HYPHEN } = {}
) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  return signedDigits(num, { decimals, minDecimals, sign, grouping: true });
}

export function formatPnL(value, { decimals = 4, fallback = HYPHEN } = {}) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  const formatted = formatSol(Math.abs(num), {
    decimals,
    fallback: fallback === HYPHEN ? HYPHEN : fallback,
  });
  if (formatted === fallback) {
    return fallback;
  }

  // The sign follows the value as shown: an amount that rounds to zero is neutral.
  const tone = signedTone(num, decimals);
  if (tone === "positive") {
    return `<span class="pnl-positive">${signPrefix(false)}${formatted}</span>`;
  }
  if (tone === "negative") {
    return `<span class="pnl-negative">${signPrefix(true)}${formatted}</span>`;
  }
  return `<span class="pnl-neutral">${formatted}</span>`;
}

export function formatTimeFromSeconds(
  timestamp,
  { fallback = HYPHEN, includeSeconds = false } = {}
) {
  const num = coerceNumber(timestamp);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const date = new Date(num * 1000);
  if (Number.isNaN(date.getTime())) {
    return fallback;
  }
  const options = {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  };
  if (includeSeconds) {
    options.second = "2-digit";
  }
  return intl(Intl.DateTimeFormat, options).format(date);
}

function toDate(value) {
  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value;
  }

  if (typeof value === "number") {
    const num = value > 1e12 ? value : value * 1000;
    const date = new Date(num);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  if (typeof value === "string" && value.trim() !== "") {
    const numeric = Number(value);
    if (Number.isFinite(numeric)) {
      const num = numeric > 1e12 ? numeric : numeric * 1000;
      const date = new Date(num);
      if (!Number.isNaN(date.getTime())) {
        return date;
      }
    }
    const parsed = new Date(value);
    return Number.isNaN(parsed.getTime()) ? null : parsed;
  }

  return null;
}

export function formatTimestamp(
  value,
  { fallback, includeSeconds = true, includeYear = true, includeDate = true } = {}
) {
  const date = toDate(value);
  if (!date) {
    return orElse(fallback, notAvailable);
  }
  const options = {};
  if (includeDate) {
    if (includeYear) {
      options.year = "numeric";
    }
    options.month = "short";
    options.day = "numeric";
  }
  options.hour = "2-digit";
  options.minute = "2-digit";
  if (includeSeconds) {
    options.second = "2-digit";
  }
  return intl(Intl.DateTimeFormat, options).format(date);
}

/**
 * Calendar day only. A release date, a report day or an expiry is about the
 * day itself, and formatTimestamp's time-of-day is noise there. `calendar`
 * ("gregory") overrides the locale's calendar for a surface laid out on a fixed
 * calendar grid; digits stay Latin.
 */
export function formatDate(
  value,
  { fallback, includeYear = true, weekday = false, utc = false, calendar } = {}
) {
  const date = toDate(value);
  if (!date) {
    return orElse(fallback, notAvailable);
  }
  const options = {};
  if (weekday) {
    options.weekday = "short";
  }
  if (includeYear) {
    options.year = "numeric";
  }
  options.month = "short";
  options.day = "numeric";
  if (utc) {
    options.timeZone = "UTC";
  }
  if (calendar) {
    options.calendar = calendar;
  }
  return intl(Intl.DateTimeFormat, options).format(date);
}

/** One calendar component of a moment (`year` or `month`), for chart axis ticks. */
export function formatDatePart(value, { part = "year", fallback } = {}) {
  const date = toDate(value);
  if (!date) {
    return orElse(fallback, notAvailable);
  }
  const options = part === "month" ? { month: "short" } : { year: "numeric" };
  return intl(Intl.DateTimeFormat, options).format(date);
}

/**
 * Month name and year of a UTC Gregorian calendar month (`month` is 1-12), for calendar
 * headings. `calendar` ("gregory") names it in that calendar instead of the locale's
 * (fa defaults to the Persian calendar); a Gregorian month grid needs it.
 */
export function formatMonthYear(year, month, { calendar } = {}) {
  const date = new Date(Date.UTC(year, month - 1, 1));
  const options = { month: "long", year: "numeric", timeZone: "UTC" };
  if (calendar) {
    options.calendar = calendar;
  }
  const text = intl(Intl.DateTimeFormat, options).format(date);
  // Some locales write month names lowercase (ru "сентябрь 2026 г."); this is a title.
  const [first] = intl(Intl.Segmenter, { granularity: "grapheme" }).segment(text);
  if (!first) return text;
  return first.segment.toLocaleUpperCase(I18n.intlLocale) + text.slice(first.segment.length);
}

let weekStart = null;

/**
 * The locale's first day of the week as a day index where 0 is Sunday, for calendar
 * grids. Intl numbers days 1-7 from Monday; runtimes without week data start on Sunday.
 */
export function firstDayOfWeek() {
  if (weekStart === null) {
    const locale = new Intl.Locale(I18n.intlLocale);
    const firstDay = (locale.getWeekInfo?.() ?? locale.weekInfo)?.firstDay;
    weekStart = Number.isInteger(firstDay) ? firstDay % 7 : 0;
  }
  return weekStart;
}

/** Short weekday name for a day index where 0 is Sunday, for calendar column headings. */
export function formatWeekday(index) {
  const date = new Date(Date.UTC(2023, 0, 1 + index));
  return intl(Intl.DateTimeFormat, { weekday: "short", timeZone: "UTC" }).format(date);
}

/**
 * Elapsed time since a moment. The default style shows the largest whole unit
 * ("3h ago"); `detailed` shows the trimmed two-unit span ("3h 5m ago") and
 * "just now" below five seconds.
 */
export function formatTimeAgo(value, { fallback = HYPHEN, style = "compact" } = {}) {
  const date = toDate(value);
  if (!date) {
    return fallback;
  }
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
  if (style === "detailed") {
    const elapsed = (Date.now() - date.getTime()) / 1000;
    if (elapsed < 5) return plain(I18n.t("format-just-now"));
    const span = formatUptime(Math.round(elapsed), { style: "trimmed" });
    return plain(I18n.t("format-ago-span", counted(Math.round(elapsed), span)));
  }
  if (seconds < 0) {
    return ago.second(0);
  }
  if (seconds < 60) return ago.second(seconds);
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return ago.minute(minutes);
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return ago.hour(hours);
  const days = Math.floor(hours / 24);
  return ago.day(days);
}

/**
 * The forward counterpart of formatTimeAgo, for a moment that has not happened
 * yet — a scheduled run, an unlock, a cooldown. formatTimeAgo floors a future
 * date at "0s ago", which reads as "it just ran" about something that has not
 * run at all, so a future time must never be sent through it.
 */
export function formatTimeUntil(value, { fallback = HYPHEN, past } = {}) {
  const date = toDate(value);
  if (!date) {
    return fallback;
  }
  const seconds = Math.floor((date.getTime() - Date.now()) / 1000);
  if (seconds <= 0) return orElse(past, () => plain(I18n.t("format-due")));
  if (seconds < 60) return until.second(seconds);
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return until.minute(minutes);
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return until.hour(hours);
  const days = Math.floor(hours / 24);
  return until.day(days);
}

export function formatUptime(seconds, { fallback, style = "detailed" } = {}) {
  const num = coerceNumber(seconds);
  if (!Number.isFinite(num) || num < 0) {
    return orElse(fallback, () => unit.second(0));
  }

  const total = Math.floor(num);
  const days = Math.floor(total / 86400);
  const hours = Math.floor((total % 86400) / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const remainingSeconds = total % 60;

  // `hm` counts hours without rolling over to days and shows "<1m" below a minute.
  if (style === "hm") {
    const wholeHours = Math.floor(total / 3600);
    if (wholeHours > 0) return `${unit.hour(wholeHours)} ${unit.minute(minutes)}`;
    if (minutes > 0) return unit.minute(minutes);
    return plain(I18n.t("format-under-minute"));
  }

  // `trimmed` is `compact` without a trailing zero part ("3h", not "3h 0m").
  if (style === "compact" || style === "trimmed") {
    const trim = style === "trimmed";
    if (total < 60) return unit.second(total);
    if (total < 3600) return unit.minute(Math.floor(total / 60));
    if (total < 86400) {
      if (trim && minutes === 0) return unit.hour(hours);
      return `${unit.hour(Math.floor(total / 3600))} ${unit.minute(minutes)}`;
    }
    if (trim && hours === 0) return unit.day(days);
    return `${unit.day(days)} ${unit.hour(hours)}`;
  }

  if (days > 0) return `${unit.day(days)} ${unit.hour(hours)} ${unit.minute(minutes)}`;
  if (hours > 0)
    return `${unit.hour(hours)} ${unit.minute(minutes)} ${unit.second(remainingSeconds)}`;
  if (minutes > 0) return `${unit.minute(minutes)} ${unit.second(remainingSeconds)}`;
  return unit.second(remainingSeconds);
}

/**
 * A quantity of one time unit ("1.5s", "2.0h") at a fixed number of decimals.
 * `unit` is one of `millisecond`, `second`, `minute`, `hour`, `day`. `trim` drops
 * trailing fraction zeros.
 */
export function formatTimeSpan(
  value,
  { unit: unitName = "second", decimals = 0, fallback = DASH, trim = false } = {}
) {
  const num = coerceNumber(value);
  const words = unit[unitName];
  if (!Number.isFinite(num) || !words) {
    return fallback;
  }
  const text = num.toFixed(decimals);
  return words(num, localizeDecimal(trim ? trimZeros(text) : text));
}

export function formatBytes(bytes, fallback) {
  const num = coerceNumber(bytes);
  if (!Number.isFinite(num) || num < 0) {
    return orElse(fallback, () => size.b(0));
  }
  if (num < 1024) return size.b(num, localizeDecimal(String(num)));
  if (num < 1_048_576) return size.kb(num / 1024, localizeDecimal((num / 1024).toFixed(1)));
  if (num < 1_073_741_824) {
    return size.mb(num / 1_048_576, localizeDecimal((num / 1_048_576).toFixed(1)));
  }
  return size.gb(num / 1_073_741_824, localizeDecimal((num / 1_073_741_824).toFixed(2)));
}

/**
 * Process memory in megabytes, one decimal in gigabytes from 1024 MB, with the
 * unit attached to the number ("512MB", "1.5GB").
 */
export function formatMemoryMb(megabytes, { fallback = DASH } = {}) {
  const num = coerceNumber(megabytes);
  if (!Number.isFinite(num) || num < 0) {
    return fallback;
  }
  if (num >= 1024) {
    const gigabytes = num / 1024;
    return plain(I18n.t("format-memory-gb", counted(gigabytes, localizeDecimal(gigabytes.toFixed(1)))));
  }
  return plain(I18n.t("format-memory-mb", counted(Math.round(num))));
}

/** Round-trip latency in milliseconds; two decimals in seconds from 1000 ms. */
export function formatLatencyMs(milliseconds, { fallback = DASH } = {}) {
  const num = coerceNumber(milliseconds);
  if (!Number.isFinite(num) || num < 0) {
    return fallback;
  }
  if (num >= 1000) {
    return unit.second(num / 1000, localizeDecimal((num / 1000).toFixed(2)));
  }
  return unit.millisecond(Math.round(num));
}

/**
 * A data size in one unit (`b`, `kb`, `mb`, `gb`) at a fixed number of decimals
 * ("1.5 MB"), for sources that already report the size in that unit.
 */
export function formatSizeAt(value, { unit: unitName = "mb", decimals = 1, fallback = DASH } = {}) {
  const num = coerceNumber(value);
  const words = size[unitName];
  if (!Number.isFinite(num) || !words) {
    return fallback;
  }
  return words(num, localizeDecimal(num.toFixed(decimals)));
}

export function formatDuration(nanos, fallback) {
  const num = coerceNumber(nanos);
  if (!Number.isFinite(num) || num < 0) {
    return orElse(fallback, () => unit.nanosecond(0));
  }
  if (num < 1_000) return unit.nanosecond(num, localizeDecimal(String(num)));
  if (num < 1_000_000) {
    return unit.microsecond(num / 1_000, localizeDecimal((num / 1_000).toFixed(1)));
  }
  if (num < 1_000_000_000) {
    return unit.millisecond(num / 1_000_000, localizeDecimal((num / 1_000_000).toFixed(1)));
  }
  return unit.second(num / 1_000_000_000, localizeDecimal((num / 1_000_000_000).toFixed(2)));
}

export function formatSignatureCompact(signature, options = {}) {
  if (!signature) return DASH;
  const start = options.start ?? 6;
  const end = options.end ?? 6;
  if (signature.length <= start + end + 1) {
    return signature;
  }
  return `${signature.slice(0, start)}…${signature.slice(-end)}`;
}

export function formatAddressCompact(address, options = {}) {
  if (!address) return DASH;
  const start = options.start ?? 4;
  const end = options.end ?? 4;
  const ellipsis = options.ellipsis ?? "…";
  if (address.length <= start + end + 1) {
    return address;
  }
  return `${address.slice(0, start)}${ellipsis}${address.slice(-end)}`;
}

/**
 * Locale-aware list of already-formatted strings ("a, b, and c"). `type` is
 * "conjunction" (and), "disjunction" (or) or "unit" (bare separators).
 */
export function formatList(items, { type = "conjunction" } = {}) {
  return intl(Intl.ListFormat, { type, style: "long" }).format((items ?? []).map(String));
}

export function formatSecondsToTime(seconds, fallback = HYPHEN) {
  if (typeof seconds !== "number" || !Number.isFinite(seconds) || seconds < 0) {
    return fallback;
  }
  const num = Math.round(seconds);
  if (num < 60) {
    return unit.second(num);
  }
  const minutes = num / 60;
  if (Number.isInteger(minutes)) {
    return unit.minute(minutes);
  }
  if (minutes < 120) {
    return unit.minute(minutes, localizeDecimal(minutes.toFixed(1)));
  }
  const hours = minutes / 60;
  if (Number.isInteger(hours)) {
    return unit.hour(hours);
  }
  return unit.hour(hours, localizeDecimal(hours.toFixed(1)));
}

// Backend UiText arguments. `sol`, `usd` and `percent` use the dashboard
// defaults of their formatters; `time` receives epoch milliseconds and `duration`
// milliseconds, shown with the detailed uptime style.
I18n.registerArgFormatter("sol", (value) => formatSol(value));
I18n.registerArgFormatter("usd", (value) => formatCurrencyUSD(value));
I18n.registerArgFormatter("percent", (value) => formatPercentValue(value, { includeSign: false }));
I18n.registerArgFormatter("time", (value) => formatTimestamp(new Date(value)));
I18n.registerArgFormatter("duration", (value) => formatUptime(value / 1000));
