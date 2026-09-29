/**
 * Dashboard localization runtime.
 *
 * Loads synchronously after `fluent-bundle.js` and the server-rendered
 * `/i18n/<locale>/catalog.js`, so every later script can call `I18n.t()`.
 * All catalogs of the fallback chain are merged into one bundle, least specific
 * first, so a key missing from the active locale resolves from the source
 * locale. Static HTML is localized by the server; this module covers text built
 * in JavaScript and DOM inserted after load.
 */
(function () {
  "use strict";

  // Message attributes that may be written to an element. Mirrors
  // L10N_ATTRIBUTES in src/i18n/html.rs.
  const ATTRIBUTE_ALLOWLIST = [
    "title",
    "aria-label",
    "placeholder",
    "alt",
    "aria-description",
    "label",
  ];

  const data = window.__SCREENERBOT_L10N__ || null;
  const locale = (data && data.locale) || "en";
  const intlLocale = (data && data.intlLocale) || "en-u-nu-latn";
  const source = (data && data.source) || "en";
  const dir = (data && data.dir) || "ltr";

  const warned = new Set();
  function warnOnce(key, message) {
    if (warned.has(key)) return;
    warned.add(key);
    console.warn("[I18n] " + message);
  }

  function buildBundle() {
    if (!data || !Array.isArray(data.catalogs) || typeof FluentBundle === "undefined") {
      console.error("[I18n] Catalog runtime is unavailable; messages resolve to their ids");
      return null;
    }
    const bundle = new FluentBundle.FluentBundle([intlLocale], { useIsolating: true });
    for (const catalog of data.catalogs) {
      const errors = bundle.addResource(new FluentBundle.FluentResource(catalog.ftl), {
        allowOverrides: true,
      });
      if (errors.length > 0) {
        console.error("[I18n] Catalog " + catalog.locale + " has errors:", errors);
      }
    }
    return bundle;
  }

  const bundle = buildBundle();

  // Converters from backend UiText argument types to Fluent argument values.
  // `sol` and `usd` are passed through until core/format.js takes them over via
  // registerArgFormatter.
  const percentFormat = new Intl.NumberFormat(intlLocale, {
    style: "percent",
    maximumFractionDigits: 2,
  });
  const timeFormat = new Intl.DateTimeFormat(intlLocale, {
    dateStyle: "medium",
    timeStyle: "short",
  });
  const ARG_FORMATTERS = {
    count: (value) => value,
    number: (value) => value,
    percent: (value) => percentFormat.format(value / 100),
    sol: (value) => value,
    usd: (value) => value,
    time: (value) => timeFormat.format(new Date(value)),
    duration: (value) => value,
    text: (value) => String(value),
    nested: (value) => I18n.text(value),
  };

  function format(pattern, args, id) {
    const errors = [];
    const out = bundle.formatPattern(pattern, args, errors);
    if (errors.length > 0) warnOnce("format:" + id, "Formatting " + id + " failed: " + errors[0]);
    return out;
  }

  function message(id) {
    return bundle ? bundle.getMessage(id) : undefined;
  }

  function parseArgs(raw) {
    if (!raw) return undefined;
    let parsed;
    try {
      parsed = JSON.parse(raw);
    } catch {
      return undefined;
    }
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return undefined;
    const args = {};
    for (const [name, value] of Object.entries(parsed)) {
      if (typeof value === "string" || typeof value === "number") args[name] = value;
    }
    return args;
  }

  function localizeElement(el) {
    const id = el.getAttribute("data-l10n-id");
    const msg = id ? message(id) : undefined;
    if (!msg) {
      console.debug("[I18n] Unknown l10n id", id);
      return;
    }
    const args = parseArgs(el.getAttribute("data-l10n-args"));
    if (msg.value) el.textContent = format(msg.value, args, id);
    for (const [name, pattern] of Object.entries(msg.attributes || {})) {
      if (ATTRIBUTE_ALLOWLIST.includes(name)) {
        el.setAttribute(name, format(pattern, args, id + "." + name));
      } else {
        console.debug("[I18n] Skipping non-allowlisted attribute", name, "on", id);
      }
    }
  }

  const I18n = {
    locale,
    intlLocale,
    dir,
    source,

    /** Formatted message value, or the id when the catalog has no value for it. */
    t(id, args) {
      const msg = message(id);
      if (!msg || !msg.value) {
        warnOnce("missing:" + id, "Missing message " + id);
        return id;
      }
      return format(msg.value, args, id);
    },

    /** Formatted message attribute, or null. */
    attr(id, name, args) {
      const msg = message(id);
      const pattern = msg && msg.attributes ? msg.attributes[name] : undefined;
      return pattern ? format(pattern, args, id + "." + name) : null;
    },

    has(id) {
      return Boolean(message(id));
    },

    /** Render a backend UiText: `{ id, args: { name: { type, value } } }`. */
    text(uiText) {
      if (!uiText || typeof uiText.id !== "string") return "";
      const args = {};
      for (const [name, arg] of Object.entries(uiText.args || {})) {
        const convert = ARG_FORMATTERS[arg.type];
        if (!convert) warnOnce("arg:" + arg.type, "Unknown argument type " + arg.type);
        args[name] = convert ? convert(arg.value) : arg.value;
      }
      return I18n.t(uiText.id, args);
    },

    registerArgFormatter(type, fn) {
      ARG_FORMATTERS[type] = fn;
    },

    /** Apply `data-l10n-id` elements in `root` (and `root` itself). */
    localizeTree(root) {
      if (!root) return;
      if (typeof root.getAttribute === "function" && root.getAttribute("data-l10n-id")) {
        localizeElement(root);
      }
      if (typeof root.querySelectorAll === "function") {
        root.querySelectorAll("[data-l10n-id]").forEach(localizeElement);
      }
    },
  };

  window.I18n = I18n;
})();
