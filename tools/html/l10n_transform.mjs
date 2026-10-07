// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * html-validate transformer: validates dashboard templates as they are served
 * in English.
 *
 * Templates keep localized text and attributes empty and the server fills them
 * (`localize_html` in src/i18n/html.rs) and substitutes `{{NAME}}` placeholders.
 * This transformer applies the same steps with the `en` catalogs:
 *   - `{{LANG}}` becomes `en`, `{{DIR}}` becomes `ltr`, other `{{UPPER_CASE}}`
 *     placeholders become `0`;
 *   - every element with `data-l10n-id` gets the message value as its text
 *     content (as `localize_html`, existing content is replaced; not applied
 *     with `data-l10n-markup`, whose value is sanitized markup) and each message
 *     attribute on the allowlist as an element attribute.
 * Messages are formatted without arguments; Fluent fallback placeables are kept.
 */
import { readFileSync, readdirSync } from "node:fs";
import vm from "node:vm";
import { readL10nAttributes } from "../i18n/catalogs.mjs";

const ROOT = new URL("../../", import.meta.url);
// The dashboard build of @fluent/bundle is a browser UMD script; run it in a
// sandbox exactly as the fixture in tools/tests/fixtures/i18n_en.mjs does.
const sandbox = {};
sandbox.globalThis = sandbox;
vm.createContext(sandbox);
vm.runInContext(
  readFileSync(new URL("src/webserver/assets/fluent-bundle.js", ROOT), "utf8"),
  sandbox
);
const { FluentBundle, FluentResource } = sandbox.FluentBundle;

const L10N_ATTRIBUTES = readL10nAttributes();
const VOID_TAGS = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"]);
const RAW_TEXT_TAGS = new Set(["script", "style"]);
const TAG = /<!--[\s\S]*?-->|<(\/?)([a-zA-Z][\w:-]*)((?:"[^"]*"|'[^']*'|[^'">])*)>/g;

const SOURCE_LOCALE = "en";
const bundles = new Map();

/** Source-locale catalogs with the requested locale's messages layered over them, as the server resolves them. */
function localeBundle(locale) {
  if (bundles.has(locale)) return bundles.get(locale);
  const bundle = new FluentBundle(locale, { useIsolating: false });
  for (const code of new Set([SOURCE_LOCALE, locale])) {
    const dir = new URL(`locales/${code}/`, ROOT);
    for (const name of readdirSync(dir).filter((file) => file.endsWith(".ftl")).sort()) {
      bundle.addResource(new FluentResource(readFileSync(new URL(name, dir), "utf8") + "\n"), {
        allowOverrides: true,
      });
    }
  }
  bundles.set(locale, bundle);
  return bundle;
}

/** Text direction of a registered locale, from `locales/registry.toml`. */
export function localeDirection(locale) {
  const registry = readFileSync(new URL("locales/registry.toml", ROOT), "utf8");
  const entry = registry.split("[[locale]]").find((block) => block.includes(`code = "${locale}"`));
  return /dir\s*=\s*"(rtl|ltr)"/.exec(entry ?? "")?.[1] ?? "ltr";
}

/** Format one message value of `locale` with `args`; null when the id is unknown. */
export function formatMessage(id, args, locale = SOURCE_LOCALE) {
  const bundle = localeBundle(locale);
  const found = bundle.getMessage(id);
  return found?.value ? bundle.formatPattern(found.value, args, []) : null;
}

function message(id, locale) {
  const bundle = localeBundle(locale);
  const found = bundle.getMessage(id);
  if (!found) return null;
  const errors = [];
  const format = (pattern) => (pattern ? bundle.formatPattern(pattern, undefined, errors) : null);
  const attributes = {};
  for (const [name, pattern] of Object.entries(found.attributes)) attributes[name] = format(pattern);
  return { value: format(found.value), attributes };
}

const escapeText = (text) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
const escapeAttr = (text) => text.replace(/&/g, "&amp;").replace(/"/g, "&quot;");

function attrValue(attrs, name) {
  const match = new RegExp(`(?:^|\\s)${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)')`).exec(attrs);
  return match ? (match[1] ?? match[2]) : null;
}

function withAttribute(attrs, name, value) {
  const existing = new RegExp(`\\s${name}\\s*=\\s*(?:"[^"]*"|'[^']*'|[^\\s"'>]+)`);
  const trimmed = attrs.replace(existing, "").replace(/\s*\/$/, "");
  const selfClosing = /\/\s*$/.test(attrs) ? " /" : "";
  return `${trimmed} ${name}="${escapeAttr(value)}"${selfClosing}`;
}

/** Replace placeholders, then localize every `data-l10n-id` element. */
export function localizeTemplate(html, locale = SOURCE_LOCALE) {
  const source = html.replace(/\{\{([A-Z][A-Z0-9_]*)\}\}/g, (_, name) =>
    name === "LANG" ? locale : name === "DIR" ? localeDirection(locale) : "0"
  );
  const tags = [...source.matchAll(TAG)].filter((m) => m[2]);
  // Replacements are collected as [start, end, text] over the source and applied last to first.
  const edits = [];
  let rawUntil = -1;
  for (let i = 0; i < tags.length; i += 1) {
    const tag = tags[i];
    if (tag.index < rawUntil) continue;
    const name = tag[2].toLowerCase();
    if (tag[1]) continue;
    if (RAW_TEXT_TAGS.has(name)) {
      const close = tags.findIndex((t, j) => j > i && t[1] && t[2].toLowerCase() === name);
      rawUntil = close < 0 ? Infinity : tags[close].index;
      continue;
    }
    const id = attrValue(tag[3], "data-l10n-id");
    if (id === null) continue;
    const found = message(id, locale);
    if (!found) continue;
    let attrs = tag[3];
    for (const [attr, value] of Object.entries(found.attributes)) {
      if (L10N_ATTRIBUTES.includes(attr) && value !== null) attrs = withAttribute(attrs, attr, value);
    }
    edits.push([tag.index, tag.index + tag[0].length, `<${tag[2]}${attrs}>`]);
    const isVoid = VOID_TAGS.has(name) || /\/\s*$/.test(tag[3]);
    if (found.value === null || isVoid || / data-l10n-markup(?=[\s=/]|$)/.test(` ${tag[3]}`)) continue;
    let depth = 1;
    for (let j = i + 1; j < tags.length; j += 1) {
      const next = tags[j];
      if (next[2].toLowerCase() !== name) continue;
      depth += next[1] ? -1 : 1;
      if (depth === 0) {
        edits.push([tag.index + tag[0].length, next.index, escapeText(found.value)]);
        break;
      }
    }
  }
  let out = source;
  for (const [start, end, text] of edits.sort((a, b) => b[0] - a[0])) {
    out = out.slice(0, start) + text + out.slice(end);
  }
  return out;
}

/** html-validate transformer entry point (default export must be the function). */
export default function transform(source) {
  return [{ ...source, data: localizeTemplate(source.data) }];
}

transform.api = 1;
