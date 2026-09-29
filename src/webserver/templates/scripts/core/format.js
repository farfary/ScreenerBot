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
 *   (`%`, `+`, `$`, `K`/`M`/`B`, the fallback dashes) are code constants.
 *
 * Loaded after `core/i18n.js`, which provides the `I18n` global.
 */

const DASH = "—";
const HYPHEN = "-";
const SUBSCRIPT_DIGITS = "₀₁₂₃₄₅₆₇₈₉";

// Fluent wraps placeables in bidi isolates. Formatter output is a compact
// display string compared, sliced and diffed by callers, so the marks are removed.
const ISOLATES = /[⁨⁩]/g;
const plain = (text) => text.replace(ISOLATES, "");

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

  const { fallback = DASH, useGrouping = true } = options || {};

  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  return intl(Intl.NumberFormat, {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
    useGrouping,
  }).format(num);
}

export function formatCompactNumber(value, digitsOrOptions = 2, maybeFallback = DASH) {
  // Support both (value, digits, fallback) and (value, { digits, fallback, prefix })
  let digits = digitsOrOptions;
  let fallback = maybeFallback;
  let prefix = "";

  if (typeof digitsOrOptions === "object" && digitsOrOptions !== null) {
    digits = digitsOrOptions.digits ?? 2;
    fallback = digitsOrOptions.fallback ?? DASH;
    prefix = digitsOrOptions.prefix ?? "";
  }

  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  const formatted = intl(Intl.NumberFormat, {
    notation: "compact",
    maximumFractionDigits: digits,
  }).format(num);

  return prefix + formatted;
}

export function formatBooleanFlag(value, unknownLabel) {
  if (value === true) return plain(I18n.t("format-yes"));
  if (value === false) return plain(I18n.t("format-no"));
  return orElse(unknownLabel, () => plain(I18n.t("format-unknown")));
}

export function formatCurrencyUSD(value, { fallback = DASH } = {}) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  const abs = Math.abs(num);
  let scaled = num;
  let suffix = "";

  if (abs >= 1_000_000_000) {
    scaled = num / 1_000_000_000;
    suffix = "B";
  } else if (abs >= 1_000_000) {
    scaled = num / 1_000_000;
    suffix = "M";
  } else if (abs >= 1_000) {
    scaled = num / 1_000;
    suffix = "K";
  } else if (abs > 0 && abs < 0.01) {
    // Sub-cent prices round to $0.00 with toFixed(2); render the real value in
    // subscript notation (e.g. $0.0₅8142) so tiny token prices stay visible.
    return `$${formatPriceSubscript(num, { fallback, precision: 4 })}`;
  }

  return `$${localizeDecimal(scaled.toFixed(2))}${suffix}`;
}

/**
 * Format price with subscript notation for very small numbers
 * Uses subscript notation: 0.0₉12345 means 0.000000000012345 (9 zeros after decimal)
 * @param {number} price - The price to format
 * @param {Object} options - Formatting options
 * @param {string} options.fallback - Value to return if price is invalid
 * @param {number} options.precision - Number of significant digits
 * @returns {string} Formatted price string
 */
export function formatPriceSubscript(price, { fallback = DASH, precision = 5 } = {}) {
  const num = coerceNumber(price);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  if (num === 0) return "0";

  const absPrice = Math.abs(num);
  const sign = num < 0 ? "-" : "";

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

export function formatPercentValue(
  value,
  { fallback = DASH, decimals = 2, includeSign = true } = {}
) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    return fallback;
  }

  const magnitude = localizeDecimal(Math.abs(num).toFixed(decimals));
  if (!includeSign) {
    return `${magnitude}%`;
  }

  if (num > 0) return `+${magnitude}%`;
  if (num < 0) return `-${magnitude}%`;
  return `${magnitude}%`;
}

export function formatPercent(value, { style = "plain", decimals = 2, fallback = HYPHEN } = {}) {
  const num = coerceNumber(value);
  if (!Number.isFinite(num)) {
    if (style === "token") {
      return `<span>${fallback}</span>`;
    }
    return fallback;
  }

  if (style === "token") {
    const color = num > 0 ? "#16a34a" : num < 0 ? "#ef4444" : "inherit";
    const sign = num > 0 ? "+" : "";
    return `<span style="color:${color};">${sign}${localizeDecimal(num.toFixed(decimals))}%</span>`;
  }

  if (style === "pnl") {
    const magnitude = localizeDecimal(Math.abs(num).toFixed(decimals));
    if (num > 0) {
      return `<span class="pnl-positive">+${magnitude}%</span>`;
    }
    if (num < 0) {
      return `<span class="pnl-negative">-${magnitude}%</span>`;
    }
    return `<span class="pnl-neutral">${magnitude}%</span>`;
  }

  const sign = num > 0 ? "+" : num < 0 ? "-" : "";
  return `${sign}${localizeDecimal(Math.abs(num).toFixed(decimals))}%`;
}

export function formatSol(amount, { decimals = 4, fallback = HYPHEN, suffix } = {}) {
  const num = coerceNumber(amount);
  if (!Number.isFinite(num)) {
    return fallback;
  }
  const formatted = localizeDecimal(num.toFixed(decimals));
  if (suffix === undefined) {
    return plain(I18n.t("format-sol-amount", { amount: formatted }));
  }
  return `${formatted}${suffix}`;
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

  if (num > 0) {
    return `<span class="pnl-positive">+${formatted}</span>`;
  }
  if (num < 0) {
    return `<span class="pnl-negative">-${formatted}</span>`;
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

export function formatTimestamp(value, { fallback, includeSeconds = true } = {}) {
  const date = toDate(value);
  if (!date) {
    return orElse(fallback, notAvailable);
  }
  const options = {
    year: "numeric",
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

/**
 * Calendar day only. A release date, a report day or an expiry is about the
 * day itself, and formatTimestamp's time-of-day is noise there.
 */
export function formatDate(value, { fallback } = {}) {
  const date = toDate(value);
  if (!date) {
    return orElse(fallback, notAvailable);
  }
  return intl(Intl.DateTimeFormat, {
    year: "numeric",
    month: "short",
    day: "numeric",
  }).format(date);
}

export function formatTimeAgo(value, { fallback = HYPHEN } = {}) {
  const date = toDate(value);
  if (!date) {
    return fallback;
  }
  const seconds = Math.floor((Date.now() - date.getTime()) / 1000);
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

  if (style === "compact") {
    if (total < 60) return unit.second(total);
    if (total < 3600) return unit.minute(Math.floor(total / 60));
    if (total < 86400) {
      return `${unit.hour(Math.floor(total / 3600))} ${unit.minute(Math.floor((total % 3600) / 60))}`;
    }
    return `${unit.day(days)} ${unit.hour(hours)}`;
  }

  if (days > 0) return `${unit.day(days)} ${unit.hour(hours)} ${unit.minute(minutes)}`;
  if (hours > 0)
    return `${unit.hour(hours)} ${unit.minute(minutes)} ${unit.second(remainingSeconds)}`;
  if (minutes > 0) return `${unit.minute(minutes)} ${unit.second(remainingSeconds)}`;
  return unit.second(remainingSeconds);
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
  if (address.length <= start + end + 1) {
    return address;
  }
  return `${address.slice(0, start)}…${address.slice(-end)}`;
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
