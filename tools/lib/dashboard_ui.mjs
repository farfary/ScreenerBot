/**
 * Shared parsing for the dashboard's own CSS and markup.
 *
 * The dashboard ships as `include_str!` assets, so nothing in the Rust build
 * ever looks at a selector or a tag. Every static guarantee about the UI is
 * therefore made here, by reading `src/webserver/templates/` the same way the
 * browser eventually will: all stylesheets concatenate into ONE document, and
 * markup lives in `pages/*.html` as well as inside JS template literals.
 *
 * Consumers: `tools/dashboard-ui-audit.mjs`, `tools/lib/button_contract.mjs`.
 */

import { readFile, readdir } from "node:fs/promises";
import { relative, resolve } from "node:path";
import { fileURLToPath, URL } from "node:url";

export const REPO_ROOT = fileURLToPath(new URL("../..", import.meta.url));
export const TEMPLATES_ROOT = resolve(REPO_ROOT, "src/webserver/templates");
export const STYLES_ROOT = resolve(TEMPLATES_ROOT, "styles");
export const SCRIPTS_ROOT = resolve(TEMPLATES_ROOT, "scripts");
export const PAGES_ROOT = resolve(TEMPLATES_ROOT, "pages");
export const BASE_HTML = resolve(TEMPLATES_ROOT, "base.html");

export async function walk(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(
    entries.map((entry) => {
      const path = resolve(directory, entry.name);
      return entry.isDirectory() ? walk(path) : [path];
    })
  );
  return files.flat();
}

export function repoPath(file) {
  return relative(REPO_ROOT, file);
}

/** Split a selector list on top-level commas only (`:is(a, b)` stays whole). */
export function splitSelectorList(value) {
  const selectors = [];
  let start = 0;
  let depth = 0;

  for (let index = 0; index < value.length; index += 1) {
    const char = value[index];
    if (char === "(" || char === "[") depth += 1;
    else if (char === ")" || char === "]") depth = Math.max(0, depth - 1);
    else if (char === "," && depth === 0) {
      selectors.push(value.slice(start, index).trim());
      start = index + 1;
    }
  }

  selectors.push(value.slice(start).trim());
  return selectors.filter(Boolean);
}

/**
 * Flat list of `{ selector, body, line }`, one entry per selector in a list.
 * Rules nested in `@media`/`@scope` are returned too — they still apply.
 */
export function rulesIn(css) {
  const blanked = blankComments(css);
  const rules = [];
  const pattern = /([^{}]+)\{([^{}]*)\}/g;
  let match;
  while ((match = pattern.exec(blanked))) {
    const raw = match[1];
    const selector = raw.slice(raw.lastIndexOf(";") + 1).trim();
    if (!selector || selector.startsWith("@")) continue;
    const line = lineOf(blanked, match.index + raw.length - raw.trimStart().length);
    for (const part of splitSelectorList(selector)) {
      rules.push({ selector: part, body: match[2], line });
    }
  }
  return rules;
}

/** Comments become spaces so byte offsets - and therefore line numbers - hold. */
function blankComments(css) {
  return css.replace(/\/\*[\s\S]*?\*\//g, (comment) => comment.replace(/[^\n]/g, " "));
}

export function lineOf(source, index) {
  let line = 1;
  for (let i = 0; i < index && i < source.length; i += 1) {
    if (source[i] === "\n") line += 1;
  }
  return line;
}

/** The rightmost compound of a selector - the element the rule actually draws. */
export function subjectCompound(selector) {
  let depth = 0;
  let start = 0;
  for (let index = 0; index < selector.length; index += 1) {
    const char = selector[index];
    if (char === "(" || char === "[") depth += 1;
    else if (char === ")" || char === "]") depth = Math.max(0, depth - 1);
    else if (depth === 0 && /[\s>+~]/.test(char)) start = index + 1;
  }
  return selector.slice(start).trim();
}

/** Class names a selector mentions anywhere, including inside `:is()`/`:not()`. */
export function classesIn(selector) {
  return [...selector.matchAll(/\.([\w-]+)/g)].map((match) => match[1]);
}

export async function loadStylesheets() {
  const files = (await walk(STYLES_ROOT)).filter((file) => file.endsWith(".css"));
  return Promise.all(
    files.map(async (file) => {
      const css = await readFile(file, "utf8");
      return { file, path: relative(STYLES_ROOT, file), css, rules: rulesIn(css) };
    })
  );
}

/** Every file that can emit dashboard DOM: page templates, base, and JS. */
export async function loadMarkupSources() {
  const files = [
    ...(await walk(SCRIPTS_ROOT)).filter((file) => file.endsWith(".js")),
    ...(await walk(PAGES_ROOT)).filter((file) => file.endsWith(".html")),
    BASE_HTML,
  ];
  return Promise.all(
    files.map(async (file) => ({
      file,
      path: repoPath(file),
      source: await readFile(file, "utf8"),
    }))
  );
}

/**
 * Opening tags of `tagName`, scanned rather than regexed: a template literal's
 * `${...}` routinely contains `>` and quotes (`${a > b ? "x" : "y"}`), so a
 * `<button[^>]*>` pattern truncates the tag and silently loses its classes.
 */
export function findOpenTags(source, tagName = "[a-z][\\w-]*") {
  const tags = [];
  const opening = new RegExp(`<${tagName}\\b`, "gi");
  let match;
  while ((match = opening.exec(source))) {
    let index = match.index + match[0].length;
    let quote = null;
    let depth = 0;
    for (; index < source.length; index += 1) {
      const char = source[index];
      if (depth > 0) {
        if (char === "{") depth += 1;
        else if (char === "}") depth -= 1;
        continue;
      }
      if (char === "$" && source[index + 1] === "{") {
        depth = 1;
        index += 1;
        continue;
      }
      if (quote) {
        if (char === "\\") index += 1;
        else if (char === quote) quote = null;
        continue;
      }
      if (char === '"' || char === "'") quote = char;
      else if (char === ">") break;
    }
    tags.push({ tag: source.slice(match.index, index + 1), index: match.index });
    opening.lastIndex = index + 1;
  }
  return tags;
}

/**
 * An attribute split into what is always there and what an expression decides.
 * `class="btn ${busy ? "is-busy" : ""}"` gives statics `["btn"]` and one
 * expression, whose own string literals are the conditional class names.
 */
export const EXPRESSION_MARK = "\u0001";

export function attributeParts(tagText, name) {
  const opening = new RegExp(`\\b${name}\\s*=\\s*(["'\`])`, "i").exec(tagText);
  if (!opening) return null;

  const quote = opening[1];
  const expressions = [];
  let statics = "";
  let current = "";
  let depth = 0;

  for (let index = opening.index + opening[0].length; index < tagText.length; index += 1) {
    const char = tagText[index];
    if (depth > 0) {
      if (char === "{") depth += 1;
      else if (char === "}") {
        depth -= 1;
        if (depth === 0) {
          expressions.push(current);
          current = "";
          /* The mark keeps adjacency: `action-bar-btn--${kind}` must not read
             as the complete class name `action-bar-btn--`. */
          statics += EXPRESSION_MARK;
          continue;
        }
      }
      current += char;
      continue;
    }
    if (char === "$" && tagText[index + 1] === "{") {
      depth = 1;
      index += 1;
      continue;
    }
    if (char === quote) break;
    statics += char;
  }

  return { statics, expressions };
}

const CLASS_NAME = /^[a-z][\w-]*$/i;

/**
 * Class names an expression can ADD, which is only ever what it evaluates to:
 * the string literals in its result positions. `x === "candlestick" ? " active"
 * : ""` contributes `active` — `candlestick` is a comparison, not a class.
 */
function conditionalClassNames(expression) {
  const names = [];
  let depth = 0;
  let index = 0;

  const readLiteral = (start) => {
    const quote = expression[start];
    let value = "";
    for (let i = start + 1; i < expression.length; i += 1) {
      if (expression[i] === "\\") {
        i += 1;
        continue;
      }
      if (expression[i] === quote) return { value, end: i };
      value += expression[i];
    }
    return { value, end: expression.length };
  };

  /* Result positions: after a top-level `?` or `:`, and after `||`/`&&`. */
  let armed = expression.trim().startsWith("`") || /^\s*["']/.test(expression);
  for (; index < expression.length; index += 1) {
    const char = expression[index];
    if (char === "(" || char === "[" || char === "{") depth += 1;
    else if (char === ")" || char === "]" || char === "}") depth = Math.max(0, depth - 1);
    else if (depth === 0 && (char === "?" || char === ":" || char === "|" || char === "&")) {
      armed = true;
    } else if (char === '"' || char === "'" || char === "`") {
      const literal = readLiteral(index);
      if (armed) {
        for (const token of literal.value.split(/\s+/)) {
          if (CLASS_NAME.test(token)) names.push(token);
        }
      }
      index = literal.end;
    }
  }

  return names;
}

/**
 * `{ statics, conditional }` for an element's class attribute. `conditional`
 * holds the class names an expression may add (`" active"`, `"is-busy"`) — a
 * state class is still a class, and a renamed one is invisible to every other
 * check. A token glued to an expression is a prefix, not a name, and is dropped.
 */
export function classTokens(tagText) {
  const parts = attributeParts(tagText, "class");
  if (!parts) return { statics: [], conditional: [], dynamic: false, present: false };

  const statics = parts.statics.split(/\s+/).filter((token) => CLASS_NAME.test(token));
  const conditional = parts.expressions.flatMap(conditionalClassNames);

  return {
    statics,
    conditional: [...new Set(conditional)],
    dynamic: parts.expressions.length > 0,
    present: true,
  };
}

/** Does the tag carry a bare (non-expression) attribute such as `disabled`? */
export function hasBareAttribute(tagText, name) {
  return new RegExp(`\\b${name}\\b(?!\\s*=\\s*["'\`]?\\s*\\$)`, "i").test(tagText);
}
