// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1
//

/**
 * Hardcoded user-visible strings in dashboard markup and scripts.
 *
 * Every candidate is an audit error. `// l10n-ignore: <reason>` on the same or
 * the previous line skips a candidate; the reason is mandatory.
 */

import {
  isStringLiteral,
  literalText,
  memberName,
  parseJs,
  propertyName,
  staticString,
  walkAst,
} from "./ast.mjs";
import { hasMarkup, lineIndex, scanMarkup, stripPlaceholders } from "./markup.mjs";

const HTML_ATTRIBUTES = new Set(["title", "placeholder", "aria-label", "alt", "aria-description"]);
const MARKUP_ATTRIBUTES = new Set(["title", "placeholder", "aria-label", "alt"]);
const SET_ATTRIBUTES = new Set(["title", "aria-label", "placeholder", "alt"]);
const TEXT_PROPERTIES = new Set(["textContent", "innerText", "title", "placeholder", "ariaLabel"]);
const LABEL_KEYS = new Set([
  "label", "title", "message", "placeholder", "tooltip", "description", "hint",
  "emptyText", "emptyMessage", "text", "header", "subtitle", "confirmText", "cancelText",
]);
const TOAST_CALLS = new Set(["showToast", "toast"]);
const COMPARISONS = new Set(["===", "!==", "==", "!="]);
const IGNORE_MARK = /l10n-ignore\b:?(.*)$/s;

/** A string a person would read, as opposed to an identifier, token, URL or path. */
export function isWordy(text, ids = new Set()) {
  if (ids.has(text.trim())) return false;
  const value = stripPlaceholders(text).trim();
  if (!/\p{L}/u.test(value)) return false;
  // Compound identifiers (css classes, keys, enum values) and constants are not
  // prose; a plain word such as "loading" is.
  if (/^[a-z0-9]+(?:[_-][a-z0-9]+)+$/.test(value) || /^[A-Z0-9_]+$/.test(value)) return false;
  if (/^[a-z]*\d[a-z0-9]*$/.test(value)) return false;
  if (/^#[0-9a-f]+$/i.test(value)) return false;
  if (/^[a-z][a-z0-9+.-]*:\/\/\S*$/i.test(value) || /^(mailto:|data:|tel:)\S*$/i.test(value)) return false;
  if (/^(\/|\.\/|\.\.\/|~\/)\S*$/.test(value)) return false;
  if (/^[\w.-]+(\/[\w.-]+)+\/?$/.test(value) && (value.includes(".") || value === value.toLowerCase())) return false;
  return true;
}

/** Ignore marks from comment texts keyed by line: `{ lines, count, errors }`. */
function collectIgnores(entries, path) {
  const lines = new Set();
  const errors = [];
  let count = 0;
  for (const { line, text } of entries) {
    const match = IGNORE_MARK.exec(text);
    if (!match) continue;
    if (match[1].trim() === "") {
      errors.push({ file: path, line, message: "l10n-ignore requires a reason: // l10n-ignore: <reason>" });
    } else {
      lines.add(line);
      count += 1;
    }
  }
  return { lines, count, errors };
}

/** Wordy text segments and attribute values of one markup string. */
function markupCandidates(text, ids, attributes) {
  const found = [];
  const { elements, texts } = scanMarkup(text);
  for (const node of texts) {
    if (node.localized || node.noTranslate || !isWordy(node.text, ids)) continue;
    found.push({ index: node.index, text: stripPlaceholders(node.text).trim(), kind: "markup-text" });
  }
  for (const element of elements) {
    if (element.localized || element.noTranslate) continue;
    for (const attr of element.attrs) {
      if (attributes.has(attr.name) && isWordy(attr.value, ids)) {
        found.push({ index: attr.index, text: stripPlaceholders(attr.value).trim(), kind: `markup-${attr.name}` });
      }
    }
  }
  return found;
}

export function scanHtmlHardcoded({ source, path, ids = new Set() }) {
  const lineOf = lineIndex(source);
  const comments = [...source.matchAll(/<!--([\s\S]*?)-->/g)].map((match) => ({
    line: lineOf(match.index),
    text: match[1],
  }));
  const ignores = collectIgnores(comments, path);
  const items = markupCandidates(source, ids, HTML_ATTRIBUTES)
    .map((found) => ({ line: lineOf(found.index), text: found.text, kind: found.kind }))
    .filter((item) => !ignores.lines.has(item.line) && !ignores.lines.has(item.line - 1));
  return { items, ignores: ignores.count, errors: ignores.errors };
}

/** The literal nodes an expression can display: literals and both arms of ?:, ||, ??, +. */
function textLiterals(node) {
  if (!node) return [];
  if (isStringLiteral(node) || node.type === "TemplateLiteral") return [node];
  if (node.type === "ConditionalExpression") return [...textLiterals(node.consequent), ...textLiterals(node.alternate)];
  if (node.type === "LogicalExpression" || (node.type === "BinaryExpression" && node.operator === "+")) {
    return [...textLiterals(node.left), ...textLiterals(node.right)];
  }
  return [];
}

function isTossed(node, parent, ancestors) {
  if (parent) {
    if (/^(Import|Export)/.test(parent.type) && parent.source === node) return true;
    if (parent.type === "ImportSpecifier" || parent.type === "ExportSpecifier") return true;
    if (["Property", "PropertyDefinition", "MethodDefinition"].includes(parent.type) && parent.key === node && !parent.computed) return true;
    if (parent.type === "BinaryExpression" && COMPARISONS.has(parent.operator)) return true;
    if (parent.type === "SwitchCase" && parent.test === node) return true;
  }
  return ancestors.some(
    (ancestor) =>
      ancestor.type === "ThrowStatement" ||
      (ancestor.type === "NewExpression" && ancestor.callee.type === "Identifier" && ancestor.callee.name.endsWith("Error")) ||
      (ancestor.type === "CallExpression" &&
        ancestor.callee.type === "MemberExpression" &&
        ancestor.callee.object.type === "Identifier" &&
        ancestor.callee.object.name === "console")
  );
}

export function scanJsHardcoded({ source, path, ids = new Set() }) {
  const parsed = parseJs(source);
  if (parsed.error) {
    return { items: [], ignores: 0, errors: [{ file: path, line: parsed.error.lineNumber, message: `cannot parse: ${parsed.error.message}` }] };
  }
  const ignores = collectIgnores(
    parsed.comments.map((comment) => ({ line: comment.loc.start.line, text: comment.value })),
    path
  );
  const claimed = new Map();
  const claim = (nodes, kind) => nodes.forEach((node) => claimed.has(node) || claimed.set(node, kind));
  const items = [];

  walkAst(parsed.ast, (node, ancestors) => {
    const parent = ancestors[ancestors.length - 1];

    if (node.type === "AssignmentExpression" && TEXT_PROPERTIES.has(memberName(node.left))) {
      claim(textLiterals(node.right), `assign-${memberName(node.left)}`);
    } else if (node.type === "CallExpression") {
      const name = node.callee.type === "Identifier" ? node.callee.name : memberName(node.callee);
      if (name === "setAttribute" && SET_ATTRIBUTES.has(staticString(node.arguments[0]))) {
        claim(textLiterals(node.arguments[1]), "set-attribute");
      } else if (TOAST_CALLS.has(name) && (node.callee.type === "Identifier" || name === "showToast")) {
        // A string first argument is the message; object titles and messages are
        // claimed by the property rule, and later arguments are the toast type.
        const [first] = node.arguments;
        if (first && first.type !== "ObjectExpression") claim(textLiterals(first), "toast");
      }
    } else if (node.type === "Property" && parent?.type === "ObjectExpression" && LABEL_KEYS.has(propertyName(node))) {
      claim(textLiterals(node.value), `property-${propertyName(node)}`);
    }

    const text = literalText(node);
    if (text === null || isTossed(node, parent, ancestors)) return;
    const start = node.loc.start.line;
    const offsetLine = (index) => start + (text.slice(0, index).match(/\n/g)?.length ?? 0);
    const skip = (line) =>
      [line, line - 1, start, start - 1].some((candidate) => ignores.lines.has(candidate));

    if (hasMarkup(text)) {
      for (const found of markupCandidates(text, ids, MARKUP_ATTRIBUTES)) {
        const line = offsetLine(found.index);
        if (!skip(line)) items.push({ line, text: found.text, kind: found.kind });
      }
    } else if (claimed.has(node) && isWordy(text, ids) && !skip(start)) {
      items.push({ line: start, text: stripPlaceholders(text).trim(), kind: claimed.get(node) });
    }
  });

  return { items, ignores: ignores.count, errors: ignores.errors };
}
