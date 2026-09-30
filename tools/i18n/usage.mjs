/**
 * Key usage: which catalog ids the dashboard and the Rust sources reference.
 *
 * Errors: unknown ids, call sites whose id cannot be checked statically,
 * source-locale ids nothing references, and localized elements that still
 * carry their own text.
 */

import {
  INTERPOLATION,
  commentsByLine,
  literalText,
  memberName,
  parseJs,
  staticString,
  walkAst,
} from "./ast.mjs";
import { hasMarkup, lineIndex, scanMarkup, stripPlaceholders } from "./markup.mjs";

/**
 * Namespace prefix (`config-`, `nav-`) to the Rust test that guarantees every id in
 * it is reachable. A call site may build ids dynamically only inside a
 * declared namespace, and ids in a namespace are exempt from the unused check.
 */
export const DYNAMIC_NAMESPACES = {
  "config-": "config_catalog_covers_fields",
  "nav-": "nav_catalog_covers_tabs_and_pages",
};

const CALLS = new Set(["t", "attr", "has"]);
const DYNAMIC_MARK = /l10n-dynamic:\s*(\S+)/;

function namespaceDeclared(commentTexts, namespaces) {
  for (const text of commentTexts) {
    const match = DYNAMIC_MARK.exec(text);
    if (match && Object.hasOwn(namespaces, match[1])) return true;
  }
  return false;
}

function commentsNear(byLine, line) {
  return [...(byLine.get(line) ?? []), ...(byLine.get(line - 1) ?? [])];
}

/** Check the `data-l10n-id` elements of one markup string. `lineOf` maps offsets to lines. */
function scanMarkupString({ text, path, ids, used, errors, lineOf, dynamicOk }) {
  const { elements } = scanMarkup(text);
  for (const element of elements) {
    const attr = element.attrs.find((entry) => entry.name === "data-l10n-id");
    if (!attr) continue;
    const line = lineOf(attr.index);
    if (attr.value.includes(INTERPOLATION)) {
      if (!dynamicOk(line)) {
        errors.push({ file: path, line, message: "dynamic data-l10n-id needs a declared // l10n-dynamic namespace" });
      }
    } else if (!ids.has(attr.value)) {
      errors.push({ file: path, line, message: `data-l10n-id "${attr.value}" is not in the source catalog` });
    } else {
      used.add(attr.value);
    }
    if (stripPlaceholders(element.ownText).trim() !== "") {
      errors.push({
        file: path,
        line: lineOf(element.index),
        message: `element with data-l10n-id "${attr.value}" carries its own text; the catalog is the only source`,
      });
    }
  }
}

export function scanHtmlUsage({ source, path, ids, namespaces = DYNAMIC_NAMESPACES }) {
  const used = new Set();
  const errors = [];
  const lineOf = lineIndex(source);
  const comments = new Map();
  for (const match of source.matchAll(/<!--([\s\S]*?)-->/g)) {
    const line = lineOf(match.index + match[0].length - 1);
    comments.set(line, [...(comments.get(line) ?? []), match[1]]);
  }
  scanMarkupString({
    text: source,
    path,
    ids,
    used,
    errors,
    lineOf,
    dynamicOk: (line) => namespaceDeclared(commentsNear(comments, line), namespaces),
  });
  return { used, errors };
}

export function scanJsUsage({ source, path, ids, namespaces = DYNAMIC_NAMESPACES }) {
  const used = new Set();
  const errors = [];
  const parsed = parseJs(source);
  if (parsed.error) {
    return { used, errors: [{ file: path, line: parsed.error.lineNumber, message: `cannot parse: ${parsed.error.message}` }] };
  }
  const byLine = commentsByLine(parsed.comments);
  const declared = (line) => namespaceDeclared(commentsNear(byLine, line), namespaces);

  walkAst(parsed.ast, (node, ancestors) => {
    const value = staticString(node);
    if (value !== null && ids.has(value)) used.add(value);

    if (node.type === "CallExpression" && node.callee.type === "MemberExpression") {
      const target = node.callee.object;
      if (target.type === "Identifier" && target.name === "I18n" && CALLS.has(memberName(node.callee))) {
        /* The runtime object forwards caller-supplied ids; it is not a call site. */
        const implementsRuntime = ancestors.some(
          (ancestor) => ancestor.type === "VariableDeclarator" && ancestor.id.name === "I18n"
        );
        if (!implementsRuntime) checkCall(node);
      }
    }

    const text = literalText(node);
    if (text !== null && hasMarkup(text)) {
      const start = node.loc.start.line;
      const offsetLine = (offset) => start + (text.slice(0, offset).match(/\n/g)?.length ?? 0);
      scanMarkupString({
        text,
        path,
        ids,
        used,
        errors,
        lineOf: offsetLine,
        dynamicOk: (line) => declared(line) || declared(start),
      });
    }
  });

  function checkCall(call) {
    const [first] = call.arguments;
    const line = call.loc.start.line;
    const id = first ? staticString(first) : null;
    if (id === null) {
      if (!declared(line)) {
        errors.push({
          file: path,
          line,
          message: `I18n.${memberName(call.callee)} id is not a static string; declare // l10n-dynamic: <namespace>`,
        });
      }
    } else if (!ids.has(id)) {
      errors.push({ file: path, line, message: `I18n.${memberName(call.callee)} id "${id}" is not in the source catalog` });
    } else {
      used.add(id);
    }
  }

  return { used, errors };
}

/** Ids referenced from Rust: generated `ids::SCREAMING_NAME` constants and quoted literals. */
export function scanRustUsage({ source, ids }) {
  const used = new Set();
  for (const match of source.matchAll(/\bids::([A-Z][A-Z0-9_]*)/g)) {
    const id = match[1].toLowerCase().replaceAll("_", "-");
    if (ids.has(id)) used.add(id);
  }
  for (const match of source.matchAll(/"((?:[^"\\\n]|\\.)*)"/g)) {
    if (ids.has(match[1])) used.add(match[1]);
  }
  return used;
}

export function unusedErrors({ ids, used, namespaces = DYNAMIC_NAMESPACES }) {
  const prefixes = Object.keys(namespaces);
  return [...ids]
    .filter((id) => !used.has(id) && !prefixes.some((prefix) => id.startsWith(prefix)))
    .sort()
    .map((id) => ({ file: "locales/en", message: `message "${id}" is not used anywhere` }));
}

/**
 * Aggregate over `{ js, html, rust }` arrays of `{ path, source }`. `dashboardUsed` maps each id
 * referenced by dashboard JS or HTML to the first file that references it.
 */
export function scanUsage({ ids, js, html, rust, namespaces = DYNAMIC_NAMESPACES }) {
  const used = new Set();
  const dashboardUsed = new Map();
  const errors = [];
  const record = (file, result) => {
    result.used.forEach((id) => {
      used.add(id);
      if (!dashboardUsed.has(id)) dashboardUsed.set(id, file.path);
    });
    errors.push(...result.errors);
  };
  for (const file of js) record(file, scanJsUsage({ ...file, ids, namespaces }));
  for (const file of html) record(file, scanHtmlUsage({ ...file, ids, namespaces }));
  for (const file of rust) scanRustUsage({ source: file.source, ids }).forEach((id) => used.add(id));
  errors.push(...unusedErrors({ ids, used, namespaces }));
  return { used, dashboardUsed, errors };
}
