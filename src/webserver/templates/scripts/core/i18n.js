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

  // Pseudo-locale transforms. Mirror of src/i18n/pseudo.rs; both are pinned by
  // tools/tests/fixtures/i18n_pseudo.json. Each table holds 26 scalars (A-Z, a-z).
  const ACCENT_UPPER = "ȦƁƇḒḖƑƓĦĪĴĶĿḾȠǾƤɊŘŞŦŬṼẆẊẎẐ";
  const ACCENT_LOWER = "ȧƀƈḓḗƒɠħīĵķŀḿƞǿƥɋřşŧŭṽẇẋẏẑ";
  const BIDI_UPPER = "∀ԐↃᗡƎℲ⅁HIſӼ⅂WNOԀÒᴚS⊥∩ɅMX⅄Z";
  const BIDI_LOWER = "ɐqɔpǝɟƃɥıɾʞʅɯuodbɹsʇnʌʍxʎz";
  const RLO = "\u202E";
  const PDF = "\u202C";
  const ACCENT_TABLES = [Array.from(ACCENT_UPPER), Array.from(ACCENT_LOWER)];
  const BIDI_TABLES = [Array.from(BIDI_UPPER), Array.from(BIDI_LOWER)];

  function mapLetter(ch, tables) {
    const code = ch.charCodeAt(0);
    return code < 97 ? tables[0][code - 65] : tables[1][code - 97];
  }

  function isAsciiLetter(ch) {
    return (ch >= "A" && ch <= "Z") || (ch >= "a" && ch <= "z");
  }

  function transformAccented(text) {
    let out = "";
    for (const ch of text) {
      if (!isAsciiLetter(ch)) {
        out += ch;
        continue;
      }
      const mapped = mapLetter(ch, ACCENT_TABLES);
      out += "aeiouAEIOU".includes(ch) ? mapped + mapped : mapped;
    }
    return out;
  }

  function transformBidi(text) {
    let out = "";
    let inWord = false;
    for (const ch of text) {
      const letter = isAsciiLetter(ch);
      if (letter && !inWord) out += RLO;
      else if (!letter && inWord) out += PDF;
      inWord = letter;
      out += letter ? mapLetter(ch, BIDI_TABLES) : ch;
    }
    return inWord ? out + PDF : out;
  }

  const PSEUDO_TRANSFORMS = { accented: transformAccented, bidi: transformBidi };

  const data = window.__SCREENERBOT_L10N__ || null;
  const locale = (data && data.locale) || "en";
  const intlLocale = (data && data.intlLocale) || "en-u-nu-latn";
  const source = (data && data.source) || "en";
  const dir = (data && data.dir) || "ltr";
  const pseudo = (data && data.pseudo) || null;

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
    const options = { useIsolating: true };
    if (pseudo && PSEUDO_TRANSFORMS[pseudo]) options.transform = PSEUDO_TRANSFORMS[pseudo];
    const bundle = new FluentBundle.FluentBundle([intlLocale], options);
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

  function uiTextArgs(uiText) {
    const args = {};
    for (const [name, arg] of Object.entries(uiText.args || {})) {
      const convert = ARG_FORMATTERS[arg.type];
      if (!convert) warnOnce("arg:" + arg.type, "Unknown argument type " + arg.type);
      args[name] = convert ? convert(arg.value) : arg.value;
    }
    return args;
  }

  const I18n = {
    locale,
    intlLocale,
    dir,
    source,
    pseudo,

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

    /**
     * Label for an enum-like value. `map` is a frozen `{ <id>: <message key> }`
     * table owned by the calling module. A value the map does not list, or whose
     * key the bundle lacks, renders as the raw value so it is never presented
     * as a wording the catalog did not provide.
     */
    label(map, value) {
      if (map && Object.hasOwn(map, value) && I18n.has(map[value])) return I18n.t(map[value]);
      warnOnce("label:" + String(value), "No label for value " + String(value));
      return String(value ?? "");
    },

    /** Render a backend UiText: `{ id, args: { name: { type, value } } }`. */
    text(uiText) {
      if (!uiText || typeof uiText.id !== "string") return "";
      return I18n.t(uiText.id, uiTextArgs(uiText));
    },

    /** Formatted attribute of a backend UiText's message, with its args, or null. */
    textAttr(uiText, name) {
      if (!uiText || typeof uiText.id !== "string") return null;
      return I18n.attr(uiText.id, name, uiTextArgs(uiText));
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
