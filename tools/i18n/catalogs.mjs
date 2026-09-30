/**
 * Catalog parity: every locale is checked against the source locale (`en`).
 *
 * Parse validity and duplicate ids are already enforced by `build/i18n.rs`;
 * this module adds the cross-locale guarantees the build cannot see:
 * matching shape, variables, plural coverage and untranslated terms.
 */

import { readFileSync } from "node:fs";
import { readFile, readdir } from "node:fs/promises";
import { resolve } from "node:path";
import { Resource, Term, parse, serialize } from "@fluent/syntax";

import { REPO_ROOT } from "../lib/dashboard_ui.mjs";

export const LOCALES_ROOT = resolve(REPO_ROOT, "locales");
export const SOURCE_LOCALE = "en";
const TERMS_FILE = "terms.ftl";
const CLDR_CATEGORIES = new Set(["zero", "one", "two", "few", "many", "other"]);

/** Every AST node reachable from `node`, depth first. */
function* descendants(node) {
  if (Array.isArray(node)) {
    for (const child of node) yield* descendants(child);
  } else if (node && typeof node === "object") {
    if (typeof node.type === "string") yield node;
    for (const value of Object.values(node)) yield* descendants(value);
  }
}

/** `{ messages, terms, junk }` for one locale: Maps of id to `{ node, file }`. */
export function parseLocale(files) {
  const messages = new Map();
  const terms = new Map();
  const junk = [];
  for (const [file, text] of Object.entries(files)) {
    const resource = parse(text, { withSpans: false });
    for (const entry of resource.body) {
      if (entry.type === "Message") messages.set(entry.id.name, { node: entry, file });
      else if (entry.type === "Term") terms.set(entry.id.name, { node: entry, file });
      else if (entry.type === "Junk") {
        junk.push({ file, message: entry.annotations.map((a) => a.message).join("; ") });
      }
    }
  }
  return { messages, terms, junk };
}

function variablesOf(node) {
  return new Set(
    [...descendants(node)]
      .filter((child) => child.type === "VariableReference")
      .map((child) => child.id.name)
  );
}

function termRefsOf(node) {
  return new Set(
    [...descendants(node)]
      .filter((child) => child.type === "TermReference")
      .map((child) => child.id.name)
  );
}

/** Inline tags a message value may use, without attributes. Mirrors ALLOWED_TAGS in src/i18n/markup.rs. */
export const MARKUP_TAGS = ["strong", "em", "b", "i", "code", "br"];
/** Tags Telegram HTML messages may use, without attributes. Links are built in Rust. */
export const TELEGRAM_MARKUP_TAGS = ["b", "i", "u", "s", "code", "pre"];
/** Catalog domain rendered as Telegram HTML; it has its own tag allowlist. */
export const TELEGRAM_DOMAIN = "telegram";

const tagPatterns = new Map();

/** Anchored matcher for one allowlisted tag written without attributes. */
function markupTagPattern(tags) {
  if (!tagPatterns.has(tags)) {
    const paired = tags.filter((t) => t !== "br").join("|");
    const void_ = tags.includes("br") ? "|br/?" : "";
    tagPatterns.set(tags, new RegExp(`^<(?:/?(?:${paired})${void_})>`, "i"));
  }
  return tagPatterns.get(tags);
}

/** Tag allowlist of the catalog file `file` (`telegram.ftl` is Telegram HTML). */
function tagsForFile(file) {
  return file === `${TELEGRAM_DOMAIN}.ftl` ? TELEGRAM_MARKUP_TAGS : MARKUP_TAGS;
}

/**
 * Markup used by a message value: `tags` are the allowlisted tags in order
 * (lowercase, closing tags as `/name`), `invalid` the `<` sequences outside the
 * allowlist. Attribute text is plain and not inspected.
 */
function markupOf(node, allowed = MARKUP_TAGS) {
  const pattern = markupTagPattern(allowed);
  const tags = [];
  const invalid = [];
  if (!node.value) return { tags, invalid, hasSelect: false };
  const texts = [...descendants(node.value)].filter((child) => child.type === "TextElement");
  for (const text of texts) {
    let from = text.value.indexOf("<");
    while (from >= 0) {
      const match = pattern.exec(text.value.slice(from));
      if (match) {
        tags.push(match[0].slice(1, -1).replace(/\/$/, "").toLowerCase());
        from = text.value.indexOf("<", from + match[0].length);
      } else {
        invalid.push(text.value.slice(from, from + 12));
        from = text.value.indexOf("<", from + 1);
      }
    }
  }
  const hasSelect = [...descendants(node.value)].some((child) => child.type === "SelectExpression");
  return { tags, invalid, hasSelect };
}

function markupAllowlistErrors(id, node, locale, file) {
  const allowed = tagsForFile(file.split("/").pop());
  const { invalid } = markupOf(node, allowed);
  if (invalid.length === 0) return [];
  const list = allowed.map((tag) => `<${tag}>`).join(", ");
  return [
    {
      file,
      message: `${locale}: message "${id}" uses "<" outside the markup allowlist (${list}, no attributes): ${invalid.map((text) => JSON.stringify(text)).join(", ")}`,
    },
  ];
}

/**
 * A translation uses the tags of the source message. Messages with plural or
 * other selects are compared by distinct tag names, because locales have
 * different variant counts; every other message by tag multiset.
 */
function markupParityErrors(id, source, target, locale, file) {
  const allowed = tagsForFile(file.split("/").pop());
  const want = markupOf(source, allowed);
  const have = markupOf(target, allowed);
  const key = (info) =>
    (want.hasSelect || have.hasSelect ? [...new Set(info.tags)] : [...info.tags]).sort().join(" ");
  if (key(want) === key(have)) return [];
  return [
    {
      file,
      message: `${locale}: message "${id}" markup tags [${have.tags.join(" ")}] differ from the source [${want.tags.join(" ")}]`,
    },
  ];
}

function attributeNames(node) {
  return new Set(node.attributes.map((attribute) => attribute.id.name));
}

function termText(node) {
  return serialize(new Resource([new Term(node.id, node.value, node.attributes)]), {}).trim();
}

/** Plural selects whose variant keys use CLDR categories, checked against the locale. */
function pluralErrors(id, node, locale, file) {
  const errors = [];
  let required;
  try {
    required = new Intl.PluralRules(locale).resolvedOptions().pluralCategories;
  } catch {
    return [{ file, message: `${locale}: "${locale}" is not a valid locale for plural rules` }];
  }
  for (const select of descendants(node)) {
    if (select.type !== "SelectExpression") continue;
    const { selector } = select;
    const numeric =
      selector.type === "VariableReference" ||
      (selector.type === "FunctionReference" && selector.id.name === "NUMBER");
    if (!numeric) continue;
    const keys = select.variants
      .filter((variant) => variant.key.type === "Identifier")
      .map((variant) => variant.key.name);
    if (!keys.some((key) => CLDR_CATEGORIES.has(key))) continue;
    const missing = required.filter((category) => !keys.includes(category));
    if (missing.length > 0) {
      errors.push({
        file,
        message: `${locale}: message "${id}" plural select lacks categories [${missing.join(", ")}]`,
      });
    }
  }
  return errors;
}

function compareMessage(id, source, target, locale, termIds) {
  const errors = [];
  const file = target.file;
  const fail = (message) => errors.push({ file, message: `${locale}: message "${id}" ${message}` });

  if (Boolean(source.node.value) !== Boolean(target.node.value)) {
    fail(source.node.value ? "is missing its value" : "has a value the source lacks");
  }
  const want = attributeNames(source.node);
  const have = attributeNames(target.node);
  const missing = [...want].filter((name) => !have.has(name));
  const extra = [...have].filter((name) => !want.has(name));
  if (missing.length) fail(`is missing attributes [${missing.join(", ")}]`);
  if (extra.length) fail(`has attributes not in the source [${extra.join(", ")}]`);

  const allowed = variablesOf(source.node);
  const unknown = [...variablesOf(target.node)].filter((name) => !allowed.has(name));
  if (unknown.length) fail(`references variables not in the source [${unknown.join(", ")}]`);

  const unknownTerms = [...termRefsOf(target.node)].filter((name) => !termIds.has(name));
  if (unknownTerms.length) fail(`references unknown terms [${unknownTerms.map((n) => `-${n}`).join(", ")}]`);

  errors.push(...markupParityErrors(id, source.node, target.node, locale, file));
  errors.push(...pluralErrors(id, target.node, locale, file));
  return errors;
}

/**
 * `catalogs`: `{ code: { "file.ftl": text } }`, `registered`: Set of registered codes.
 * Returns `{ errors, info, completeness, sourceIds }`; each error is `{ file, message }`.
 */
export function checkCatalogs({ catalogs, registered, source = SOURCE_LOCALE }) {
  const errors = [];
  const info = [];
  const completeness = new Map();
  const parsed = new Map();

  for (const [code, files] of Object.entries(catalogs)) {
    const locale = parseLocale(files);
    parsed.set(code, locale);
    for (const item of locale.junk) {
      errors.push({ file: `locales/${code}/${item.file}`, message: `${code}: junk entry: ${item.message}` });
    }
  }

  const en = parsed.get(source) ?? { messages: new Map(), terms: new Map(), junk: [] };
  const total = en.messages.size + en.terms.size;
  completeness.set(source, 100);
  for (const [id, entry] of en.messages) {
    errors.push(...markupAllowlistErrors(id, entry.node, source, `locales/${source}/${entry.file}`));
  }
  const sourceTerms = [...en.terms].filter(([, entry]) => entry.file === TERMS_FILE);

  for (const [code, locale] of parsed) {
    if (code === source) continue;
    const where = (entry) => `locales/${code}/${entry.file}`;

    for (const [id, entry] of locale.messages) {
      errors.push(...markupAllowlistErrors(id, entry.node, code, where(entry)));
      const original = en.messages.get(id);
      if (!original) {
        errors.push({ file: where(entry), message: `${code}: message "${id}" does not exist in ${source}` });
      } else {
        for (const error of compareMessage(id, original, entry, code, new Set(en.terms.keys()))) {
          errors.push({ ...error, file: where(entry) });
        }
      }
    }
    for (const [id, entry] of locale.terms) {
      if (!en.terms.has(id)) {
        errors.push({ file: where(entry), message: `${code}: term "-${id}" does not exist in ${source}` });
      }
    }
    for (const [id, entry] of sourceTerms) {
      const translated = locale.terms.get(id);
      if (translated && termText(translated.node) !== termText(entry.node)) {
        errors.push({
          file: where(translated),
          message: `${code}: term "-${id}" must be identical to ${source} (terms are never translated)`,
        });
      } else if (!translated) {
        errors.push({ file: `locales/${code}/${TERMS_FILE}`, message: `${code}: term "-${id}" is missing from ${TERMS_FILE}` });
      }
    }

    const missingMessages = [...en.messages.keys()].filter((id) => !locale.messages.has(id));
    const missingTerms = [...en.terms.keys()].filter((id) => !locale.terms.has(id));
    const percent = total === 0 ? 100 : Math.floor(((total - missingMessages.length - missingTerms.length) / total) * 1000) / 10;
    /* Terms defined in terms.ftl are reported individually above. */
    const missing = [
      ...missingMessages,
      ...missingTerms.filter((id) => en.terms.get(id).file !== TERMS_FILE).map((id) => `-${id}`),
    ];
    completeness.set(code, percent);
    if (registered.has(code)) {
      if (missing.length) {
        errors.push({
          file: `locales/${code}`,
          message: `${code}: registered locale is incomplete; missing [${missing.join(", ")}]`,
        });
      }
    } else {
      info.push(`locale ${code} is not registered: ${percent}% complete`);
    }
  }

  const sourceDomains = new Map([...en.messages].map(([id, entry]) => [id, entry.file.replace(/\.ftl$/, "")]));
  return { errors, info, completeness, sourceIds: new Set(en.messages.keys()), sourceDomains };
}

/** Domains listed in `SERVER_ONLY_DOMAINS` (src/i18n/mod.rs): never sent to the dashboard. */
export function parseServerOnlyDomains(rustSource) {
  const list = rustSource.match(/SERVER_ONLY_DOMAINS:\s*&\[&str\]\s*=\s*&\[([^\]]*)\]/);
  if (!list) throw new Error("SERVER_ONLY_DOMAINS not found in src/i18n/mod.rs");
  return new Set([...list[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]));
}

/** The server-only domains, read from the Rust source that owns them. */
export function readServerOnlyDomains() {
  return parseServerOnlyDomains(readFileSync(resolve(REPO_ROOT, "src/i18n/mod.rs"), "utf8"));
}

/** Attributes a message may write to an element: `L10N_ATTRIBUTES` in src/i18n/html.rs. */
export function readL10nAttributes() {
  const source = readFileSync(resolve(REPO_ROOT, "src/i18n/html.rs"), "utf8");
  const list = source.match(/L10N_ATTRIBUTES:\s*&\[&str\]\s*=\s*&\[([^\]]*)\]/);
  if (!list) throw new Error("L10N_ATTRIBUTES not found in src/i18n/html.rs");
  return [...list[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

/** Codes listed in `registry.toml` (`code = "xx"` entries). */
export function registeredCodes(toml) {
  return new Set([...toml.matchAll(/^\s*code\s*=\s*"([^"]+)"/gm)].map((match) => match[1]));
}

export async function loadCatalogs() {
  const catalogs = {};
  for (const entry of await readdir(LOCALES_ROOT, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    const dir = resolve(LOCALES_ROOT, entry.name);
    const files = {};
    for (const name of (await readdir(dir)).filter((file) => file.endsWith(".ftl")).sort()) {
      files[name] = await readFile(resolve(dir, name), "utf8");
    }
    catalogs[entry.name] = files;
  }
  const registry = await readFile(resolve(LOCALES_ROOT, "registry.toml"), "utf8");
  return { catalogs, registered: registeredCodes(registry), serverOnly: readServerOnlyDomains() };
}
